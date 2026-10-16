// The session of an app, the same in every app (single sign-on, server/docs/API.md "Single sign-on").
//
// The API keeps the session in the HttpOnly cookie `scutoons_sid` (every app on *.scutoons.com, locally every port of
// localhost); JavaScript never stores the token. Apps call the API with `credentials: 'include'`.
// Fallback: when the browser refuses the cookie (blocked cookies, privacy settings), the token from `/auth/verify/` goes
// to localStorage `scutoons_session` as before and the client sends it as Bearer – the app keeps working, per origin.
// Migration: a token an app stored before single sign-on is moved into the cookie once (`POST /auth/session/`) and then
// deleted from localStorage.
//
// No app state is bound here: the app creates one session (`createAuthSession`) and keeps it in its own context.
import { ApiError, getSessionToken, setSessionToken, type SessionResult, type VerifyResult } from '@scutoons/api';
import type { Me, User } from '@scutoons/content';

/** The API calls the session needs (`createQrApi(client)` of a client with `credentials: 'include'`) */
export interface AuthApi {
	authRequest(
		email: string,
		code?: string | null,
		locale?: string,
		marketingOptIn?: boolean,
		choice?: { item_id?: string | null; variant?: string | null; dispute?: boolean }
	): Promise<unknown>;
	authVerifyOtp(email: string, otp: string, marketingOptIn?: boolean): Promise<VerifyResult>;
	authVerifyToken(token: string): Promise<VerifyResult>;
	authLogout(all?: boolean): Promise<unknown>;
	authSession(): Promise<SessionResult>;
	authExchange(): Promise<SessionResult>;
	fetchMe(): Promise<Me>;
}

export type AuthEvent = 'login' | 'logout' | 'me';

export interface AuthSessionOptions {
	api: AuthApi;
	/** the app's locale (login mail language) */
	locale?: () => string;
	/** after a sign-in, a sign-out (also one noticed from another app) and every loaded `/me/` */
	onChange?: (event: AuthEvent) => void;
	/**
	 * Re-check the shared session when the tab becomes visible again (default true): a sign-out or an account switch in
	 * another app shows up here without a reload.
	 */
	watch?: boolean;
}

/** A hint only (no credential): "this browser was signed in" – avoids a loading flash for guests before the check */
export const SIGNED_IN_HINT_KEY = 'scutoons_signed_in';

function hint(value?: boolean): boolean {
	try {
		if (value === true) localStorage.setItem(SIGNED_IN_HINT_KEY, '1');
		else if (value === false) localStorage.removeItem(SIGNED_IN_HINT_KEY);
		return localStorage.getItem(SIGNED_IN_HINT_KEY) === '1';
	} catch {
		return false;
	}
}

const unauthorized = (e: unknown) => e instanceof ApiError && (e.status === 401 || e.status === 403);

export type AuthSession = ReturnType<typeof createAuthSession>;

export function createAuthSession(options: AuthSessionOptions) {
	const { api } = options;
	let me = $state<Me | null>(null);
	let ready = $state(false);
	let loading = $state(false);
	let error = $state<string | null>(null);
	/** how this app is signed in: the shared cookie, or (fallback) its own stored Bearer token */
	let mode = $state<'cookie' | 'bearer' | null>(null);
	let initializing: Promise<void> | null = null;

	const user = $derived<User | null>(me?.user ?? null);
	const roles = $derived<string[]>(me?.user?.roles ?? []);
	const emit = (event: AuthEvent) => {
		try {
			options.onChange?.(event);
		} catch (e) {
			console.error(e);
		}
	};

	async function loadMe(): Promise<Me | null> {
		if (!mode) return null;
		loading = true;
		try {
			me = await api.fetchMe();
			error = null;
			hint(true);
			emit('me');
			return me;
		} catch (e) {
			if (unauthorized(e)) drop();
			else error = e instanceof Error ? e.message : String(e);
			return null;
		} finally {
			loading = false;
		}
	}

	/** Does the cookie carry a session right now? (without the stored token) */
	async function cookieSession(): Promise<(SessionResult & { user: User }) | null> {
		try {
			const result = await api.authSession();
			return result.user ? (result as SessionResult & { user: User }) : null;
		} catch (e) {
			if (unauthorized(e)) return null;
			throw e;
		}
	}

	/** After `/auth/verify/`: the cookie is the session; the token is kept only if the browser refused the cookie */
	async function adopt(result: VerifyResult): Promise<VerifyResult> {
		let viaCookie = false;
		try {
			viaCookie = (await cookieSession()) !== null;
		} catch {
			viaCookie = false;
		}
		setSessionToken(viaCookie ? null : result.session_token);
		mode = viaCookie ? 'cookie' : 'bearer';
		await loadMe();
		emit('login');
		return result;
	}

	/** On start: the shared cookie, else a token this app stored before (moved into the cookie once) */
	async function restore() {
		try {
			if (await cookieSession()) {
				setSessionToken(null); // a leftover token would win over the cookie (Bearer first)
				mode = 'cookie';
				await loadMe();
				return;
			}
		} catch {
			// API unreachable: try the stored token below (the requests will tell)
		}
		if (!getSessionToken()) {
			hint(false);
			return;
		}
		try {
			await api.authExchange();
			const viaCookie = (await cookieSession()) !== null;
			if (viaCookie) setSessionToken(null);
			mode = viaCookie ? 'cookie' : 'bearer';
		} catch (e) {
			if (unauthorized(e) && e instanceof ApiError && e.code !== 'csrf_rejected') {
				setSessionToken(null); // expired / revoked
				hint(false);
				return;
			}
			mode = 'bearer'; // network trouble or a refused move: keep using the token
		}
		await loadMe();
	}

	function drop() {
		const was = !!me;
		me = null;
		mode = null;
		setSessionToken(null);
		hint(false);
		if (was) emit('logout');
	}

	// ── watch: another app (same cookie) signed out or switched the account ──────────────────────────────────
	let lastCheck = 0;
	async function refresh() {
		if (!ready || (mode === 'bearer' && me)) return; // a Bearer session is this app's own
		const now = Date.now();
		if (now - lastCheck < 2000) return;
		lastCheck = now;
		let current: (SessionResult & { user: User }) | null;
		try {
			current = await cookieSession();
		} catch {
			return; // offline: keep what we have
		}
		if (!current) {
			if (me) drop();
			return;
		}
		if (current.user.id !== me?.user.id) {
			setSessionToken(null);
			mode = 'cookie';
			await loadMe();
			emit('login');
		}
	}

	function startWatching() {
		if (options.watch === false || typeof document === 'undefined') return;
		document.addEventListener('visibilitychange', () => {
			if (document.visibilityState === 'visible') void refresh();
		});
		window.addEventListener('focus', () => void refresh());
		// back online (an app started offline has no session yet – the reader's offline progress waits for it)
		window.addEventListener('online', () => void refresh());
	}

	/** Once per page: a magic-link token (`?login=`) first, then the shared cookie / a stored token */
	function init(magicToken?: string | null): Promise<void> {
		initializing ??= (async () => {
			if (magicToken) {
				try {
					await adopt(await api.authVerifyToken(magicToken));
				} catch (e) {
					console.warn('Magic link login failed', e);
				}
			}
			if (!me) await restore();
			ready = true;
			startWatching();
		})();
		return initializing;
	}

	return {
		get me() {
			return me;
		},
		get user() {
			return user;
		},
		/** `/me/ user.roles`, e.g. `['member', 'creator']` */
		get roles() {
			return roles;
		},
		get ready() {
			return ready;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		get mode() {
			return mode;
		},
		get isLoggedIn() {
			return !!mode && !!me;
		},
		/** the session is still being checked and this browser was signed in before (show "loading", not "sign in") */
		get pending() {
			return !ready && (hint() || !!getSessionToken());
		},
		hasRole: (role: string) => roles.includes(role),
		init,
		loadMe,
		refresh,
		/** `marketingOptIn` = "Send me news about new releases" (stored when the account is created) */
		requestLogin: (
			email: string,
			code?: string | null,
			marketingOptIn = false,
			choice?: { item_id?: string | null; variant?: string | null; dispute?: boolean }
		) => api.authRequest(email.trim(), code, options.locale?.(), marketingOptIn, choice),
		verifyOtp: async (email: string, otp: string, marketingOptIn?: boolean) =>
			adopt(await api.authVerifyOtp(email.trim(), otp.trim(), marketingOptIn)),
		/** magic link from the login mail: `<app origin>/?login=<token>` */
		verifyToken: async (magicToken: string) => adopt(await api.authVerifyToken(magicToken)),
		/** Sign out everywhere: the shared session ends, every app shows its sign-in (`all` = also other devices) */
		logout: async (all = false) => {
			try {
				if (mode) await api.authLogout(all);
			} catch {
				/* dropped locally anyway */
			}
			drop();
		},
		/** Replace parts of `/me/` after a change the app made (inventory, avatar, settings) */
		patchMe: (patch: Partial<Me>) => {
			if (me) me = { ...me, ...patch };
		}
	};
}

/** `?login=<token>` of a magic link: the token and the URL without it (the app replaces its history entry) */
export function takeMagicToken(url: URL, params: string[] = ['login']): { token: string; cleaned: string } | null {
	const name = params.find((p) => url.searchParams.get(p));
	if (!name) return null;
	const token = url.searchParams.get(name) as string;
	const next = new URL(url);
	for (const p of params) next.searchParams.delete(p);
	return { token, cleaned: next.pathname + next.search + next.hash };
}
