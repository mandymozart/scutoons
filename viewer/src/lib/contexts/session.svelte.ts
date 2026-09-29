import { setContext, getContext } from 'svelte';
import { ApiError, getSessionToken, setSessionToken } from '$lib/api/client.ts';
import {
	authLogout,
	authRequest,
	authVerifyOtp,
	authVerifyToken,
	fetchMe,
	updateMe,
	type VerifyResult
} from '$lib/api/qr.service.ts';
import type { Me, ServerMembership, User } from '$lib/models/items.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

const SESSION_KEY = Symbol('session');

type Listener = (event: 'login' | 'logout' | 'me', session: SessionContext) => void;

export type SessionContext = ReturnType<typeof createSessionContext>;

let _current: SessionContext | undefined;

/**
 * Email login against qr.scutoons.com. Token lives in localStorage `scutoons_session`,
 * `/me/` (user, memberships, inventory, avatar) is loaded on init and after login.
 * Signup rule (ITEMS.md 2026-09-30): a new account needs a code from a SCUTOONS product or a promo code; without
 * one the server still answers "sent" but mails only an explanation. Roles (`member`, `creator`, `admin`) come
 * with `/me/ user.roles`.
 */
export function createSessionContext() {
	let token = $state<string | null>(null);
	let me = $state<Me | null>(null);
	let loading = $state(false);
	let ready = $state(false);
	let error = $state<string | null>(null);

	const listeners = new Set<Listener>();
	const emit = (event: 'login' | 'logout' | 'me') => {
		for (const l of listeners) {
			try {
				l(event, context);
			} catch (e) {
				console.error(e);
			}
		}
	};

	const user = $derived<User | null>(me?.user ?? null);
	const roles = $derived<string[]>(me?.user?.roles ?? []);
	const memberships = $derived<ServerMembership[]>(me?.memberships ?? []);
	/**
	 * Access model 2026-09-30: which code type grants what is decided by the server's permissions map;
	 * the viewer only checks the `catalog` permission (all chapters of all toons).
	 */
	const permissions = $derived<string[]>(me?.access?.permissions ?? []);
	/** account access: the logged-in account owns a code granting `catalog` (device access: membership context) */
	const hasAccess = $derived(permissions.includes('catalog'));

	async function loadMe(): Promise<Me | null> {
		if (!token) return null;
		loading = true;
		try {
			me = await fetchMe();
			error = null;
			emit('me');
			return me;
		} catch (e) {
			if (e instanceof ApiError && (e.status === 401 || e.status === 403)) {
				// expired / revoked session
				dropSession();
			} else {
				error = e instanceof Error ? e.message : String(e);
			}
			return null;
		} finally {
			loading = false;
		}
	}

	async function init() {
		token = getSessionToken();
		if (token && !me) await loadMe();
		ready = true;
	}

	/** `marketingOptIn` = "Send me news about new releases" (only used when the account is created) */
	function requestLogin(email: string, code?: string | null, marketingOptIn = false) {
		return authRequest(email.trim(), code, getLocale(), marketingOptIn);
	}

	async function applyVerify(result: VerifyResult) {
		token = result.session_token;
		setSessionToken(token);
		await loadMe();
		emit('login');
		return result;
	}

	const verifyOtp = async (email: string, otp: string, marketingOptIn?: boolean) =>
		applyVerify(await authVerifyOtp(email.trim(), otp.trim(), marketingOptIn));

	/** Account page toggle: `PUT /me/ { marketing_opt_in }` */
	async function setMarketingOptIn(value: boolean) {
		const result = await updateMe({ marketing_opt_in: value });
		if (me) me = { ...me, user: { ...me.user, ...result.user } };
		return result.user;
	}

	const hasRole = (role: string) => roles.includes(role);

	const verifyToken = async (magicToken: string) => applyVerify(await authVerifyToken(magicToken));

	function dropSession() {
		const wasLoggedIn = !!token;
		token = null;
		me = null;
		setSessionToken(null);
		if (wasLoggedIn) emit('logout');
	}

	async function logout() {
		try {
			if (token) await authLogout();
		} catch {
			/* session is dropped locally anyway */
		}
		dropSession();
	}

	/** Replace parts of `me` after inventory/avatar mutations without refetching. */
	function patchMe(patch: Partial<Me>) {
		if (me) me = { ...me, ...patch };
	}

	function subscribe(listener: Listener) {
		listeners.add(listener);
		return () => listeners.delete(listener);
	}

	const context = {
		get token() {
			return token;
		},
		get isLoggedIn() {
			return !!token && !!me;
		},
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
		get memberships() {
			return memberships;
		},
		get permissions() {
			return permissions;
		},
		get hasAccess() {
			return hasAccess;
		},

		get loading() {
			return loading;
		},
		get ready() {
			return ready;
		},
		get error() {
			return error;
		},
		init,
		loadMe,
		requestLogin,
		verifyOtp,
		verifyToken,
		setMarketingOptIn,
		hasRole,
		logout,
		patchMe,
		subscribe
	};
	return context;
}

export function setSessionContext() {
	const context = createSessionContext();
	setContext(SESSION_KEY, context);
	_current = context;
	return context;
}

export function getSessionContext() {
	return getContext<SessionContext>(SESSION_KEY);
}

/** For code outside the component tree (game progress sync, VN actions). */
export function getCurrentSession() {
	return _current;
}
