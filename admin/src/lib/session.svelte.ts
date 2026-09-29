import { getContext, setContext } from 'svelte';
import type { Me, User } from '@scutoons/content';
import { ApiError, authLogout, authRequest, authVerifyOtp, authVerifyToken, fetchMe, getSessionToken, setSessionToken } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

const SESSION_KEY = Symbol('admin-session');

export type AdminSession = ReturnType<typeof createSession>;

/**
 * Email + one-time-code login for this origin (localStorage `scutoons_session`, per origin until the shared
 * session cookie exists). Only existing accounts: the admin never sends a signup code. Access = role `admin`.
 */
export function createSession() {
	let token = $state<string | null>(null);
	let me = $state<Me | null>(null);
	let ready = $state(false);
	let loading = $state(false);

	const user = $derived<User | null>(me?.user ?? null);
	const roles = $derived<string[]>(me?.user?.roles ?? []);

	async function loadMe() {
		if (!token) return null;
		loading = true;
		try {
			me = await fetchMe();
			return me;
		} catch (e) {
			// expired / revoked session
			if (e instanceof ApiError && (e.status === 401 || e.status === 403)) drop();
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

	async function apply(sessionToken: string) {
		token = sessionToken;
		setSessionToken(token);
		await loadMe();
	}

	function drop() {
		token = null;
		me = null;
		setSessionToken(null);
	}

	return {
		get token() {
			return token;
		},
		get me() {
			return me;
		},
		get user() {
			return user;
		},
		get roles() {
			return roles;
		},
		get ready() {
			return ready;
		},
		get loading() {
			return loading;
		},
		get isLoggedIn() {
			return !!token && !!me;
		},
		hasRole: (role: string) => roles.includes(role),
		init,
		requestLogin: (email: string) => authRequest(email.trim(), null, getLocale()),
		verifyOtp: async (email: string, otp: string) => {
			const result = await authVerifyOtp(email.trim(), otp.trim());
			await apply(result.session_token);
			return result;
		},
		/** magic link from the login mail: `<admin origin>/?login=<token>` */
		verifyToken: async (magicToken: string) => {
			const result = await authVerifyToken(magicToken);
			await apply(result.session_token);
			return result;
		},
		logout: async () => {
			try {
				if (token) await authLogout();
			} catch {
				/* dropped locally anyway */
			}
			drop();
		}
	};
}

export function setAdminSession() {
	return setContext(SESSION_KEY, createSession());
}

export function getAdminSession() {
	return getContext<AdminSession>(SESSION_KEY);
}
