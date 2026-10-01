import { getContext, setContext } from 'svelte';
import { createAuthSession, type AuthSession } from '@scutoons/auth';
import { authApi } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

const SESSION_KEY = Symbol('admin-session');

export type AdminSession = AuthSession;

/**
 * The admin's session: single sign-on (@scutoons/auth) – an account signed in on any SCUTOONS app is signed in here.
 * Only existing accounts (the admin never sends a signup code). Access = role `admin` (`APP_ROLES`).
 */
export function setAdminSession() {
	return setContext(SESSION_KEY, createAuthSession({ api: authApi, locale: getLocale }));
}

export function getAdminSession() {
	return getContext<AdminSession>(SESSION_KEY);
}
