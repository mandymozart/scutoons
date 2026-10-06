import { getContext, setContext } from 'svelte';
import { createAuthSession, type AuthSession } from '@scutoons/auth';
import { authApi } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { contextKey } from '$lib/utils/shared.ts';

const SESSION_KEY = contextKey('creator-session');

export type CreatorSession = AuthSession;

/**
 * The creator's session: single sign-on (@scutoons/auth) – an account signed in on any SCUTOONS app is signed in here.
 * Only existing accounts (the creator never sends a signup code). Access = role `creator` (`APP_ROLES`).
 */
export function setCreatorSession() {
	return setContext(SESSION_KEY, createAuthSession({ api: authApi, locale: getLocale }));
}

export function getCreatorSession() {
	return getContext<CreatorSession>(SESSION_KEY);
}
