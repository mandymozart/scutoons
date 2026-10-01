// @scutoons/auth: single sign-on for every SCUTOONS app (server/docs/API.md "Single sign-on", agents/PLATFORM.md "SSO").
// The app imports '@scutoons/tokens' and creates its API client with `credentials: 'include'`.
export {
	createAuthSession,
	takeMagicToken,
	SIGNED_IN_HINT_KEY,
	type AuthApi,
	type AuthEvent,
	type AuthSession,
	type AuthSessionOptions
} from './session.svelte.ts';
export { default as LoginForm, type LoginSession } from './login-form.svelte';
export { default as AuthGate } from './auth-gate.svelte';
export { default as SignedInAs } from './signed-in-as.svelte';
export { AUTH_LABELS, authLabels, type AuthLabels } from './labels.ts';
export { APP_ROLES, canUseApp, type AppId } from './roles.ts';
