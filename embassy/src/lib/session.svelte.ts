import { getContext, setContext } from 'svelte';
import { createAuthSession, type AuthSession } from '@scutoons/auth';
import { authApi } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

const SESSION_KEY = Symbol('embassy-session');

export type EmbassySession = AuthSession;

/**
 * The Embassy's session: single sign-on (@scutoons/auth) – an account signed in on any SCUTOONS app is signed in here.
 * Only existing accounts (the Embassy never sends a signup code). Access = role `liaison`, `ambassador` or `admin`
 * (`APP_ROLES`).
 */
export function setEmbassySession() {
	return setContext(SESSION_KEY, createAuthSession({ api: authApi, locale: getLocale }));
}

export function getEmbassySession() {
	return getContext<EmbassySession>(SESSION_KEY);
}
