import { setContext, getContext } from 'svelte';
import { createAuthSession, type AuthEvent } from '@scutoons/auth';
import { authApi, updateMe } from '$lib/api.ts';
import type { ServerMembership, User } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';

const SESSION_KEY = Symbol('session');

type Listener = (event: AuthEvent, session: SessionContext) => void;

export type SessionContext = ReturnType<typeof createSessionContext>;

let _current: SessionContext | undefined;

/**
 * Email login – the single sign-on session of every SCUTOONS app (@scutoons/auth): the API keeps it in an HttpOnly
 * cookie, so an account signed in on the creator, admin, shop or embassy is signed in here too, and signing out here
 * signs out everywhere. `/me/` (user, memberships, inventory, avatar) is loaded on init and after login.
 * Signup rule (ITEMS.md 2026-09-30): a new account needs a code from a SCUTOONS product or a promo code; without
 * one the server still answers "sent" but mails only an explanation. Roles (`member`, `creator`, `admin`) come
 * with `/me/ user.roles`.
 */
export function createSessionContext() {
	const listeners = new Set<Listener>();
	const auth = createAuthSession({
		api: authApi,
		locale: getLocale,
		onChange: (event) => {
			for (const l of listeners) {
				try {
					l(event, context);
				} catch (e) {
					console.error(e);
				}
			}
		}
	});

	const memberships = $derived<ServerMembership[]>(auth.me?.memberships ?? []);
	/**
	 * Access model 2026-09-30: which code type grants what is decided by the server's permissions map;
	 * the viewer only checks the `catalog` permission (all chapters of all toons).
	 */
	const permissions = $derived<string[]>(auth.me?.access?.permissions ?? []);
	/** account access: the logged-in account owns a code granting `catalog` (device access: membership context) */
	const hasAccess = $derived(permissions.includes('catalog'));

	/** Account page toggle: `PUT /me/ { marketing_opt_in }` */
	async function setMarketingOptIn(value: boolean) {
		const result = await updateMe({ marketing_opt_in: value });
		if (auth.me) auth.patchMe({ user: { ...auth.me.user, ...result.user } });
		return result.user;
	}

	/**
	 * Account page toggle "Hide sensitive content": `PUT /me/ { hide_sensitive }` (on by default; it blurs and mutes
	 * slices marked sensitive)
	 */
	async function setHideSensitive(value: boolean) {
		const result = await updateMe({ hide_sensitive: value });
		if (auth.me) auth.patchMe({ user: { ...auth.me.user, ...result.user }, ...(result.avatar ? { avatar: result.avatar } : {}) });
		return result.user;
	}

	/** Account page "Media quality": `PUT /me/ { media_quality }` (auto by default; lib/media-quality.svelte.ts) */
	async function setMediaQuality(value: NonNullable<User['media_quality']>) {
		const result = await updateMe({ media_quality: value });
		if (auth.me) auth.patchMe({ user: { ...auth.me.user, ...result.user } });
		return result.user;
	}

	function subscribe(listener: Listener) {
		listeners.add(listener);
		return () => listeners.delete(listener);
	}

	const context = {
		/** the shared session (for the @scutoons/auth components) */
		auth,
		get isLoggedIn() {
			return auth.isLoggedIn;
		},
		/** still checking, and this browser was signed in before: show "loading" instead of "sign in" */
		get pending() {
			return auth.pending;
		},
		get me() {
			return auth.me;
		},
		get user() {
			return auth.user;
		},
		/** `/me/ user.roles`, e.g. `['member', 'creator']` */
		get roles() {
			return auth.roles;
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
			return auth.loading;
		},
		get ready() {
			return auth.ready;
		},
		get error() {
			return auth.error;
		},
		init: auth.init,
		loadMe: auth.loadMe,
		requestLogin: auth.requestLogin,
		verifyOtp: auth.verifyOtp,
		verifyToken: auth.verifyToken,
		setMarketingOptIn,
		setHideSensitive,
		hasRole: auth.hasRole,
		logout: auth.logout,
		/** Replace parts of `me` after inventory/avatar mutations without refetching. */
		patchMe: auth.patchMe,
		setMediaQuality,
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
