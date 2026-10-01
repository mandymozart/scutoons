# @scutoons/auth

Single sign-on for every SCUTOONS app (viewer, creator, admin, shop, embassy): one session, one sign-in flow, one look.
The API keeps the session in the HttpOnly cookie `scutoons_sid` (server/docs/API.md "Single sign-on"); this package is
the browser side. Svelte 5, styled with `@scutoons/tokens` (IBM Plex Mono, ink/paper, 2px rules) and `@scutoons/ui`.

## Public API (`import … from '@scutoons/auth'`)

| Export | What |
|---|---|
| `createAuthSession({ api, locale?, onChange?, watch? })` | the session (runes): `me`, `user`, `roles`, `hasRole()`, `isLoggedIn`, `ready`, `loading`, `pending`, `error`, `mode` (`cookie` / `bearer`), `init(magicToken?)`, `loadMe()`, `refresh()`, `requestLogin()`, `verifyOtp()`, `verifyToken()`, `logout(all?)`, `patchMe()`. `api` = `createQrApi(client)` of a client with `credentials: 'include'`. |
| `takeMagicToken(url)` | `?login=<token>` of the mail's magic link → `{ token, cleaned }` (the app replaces its history entry, then `init(token)`) |
| `LoginForm` | email → 6-digit code (+ resend, change email, errors); props `session`, `locale`, `code?`, `itemId?`, `variant?`, `signup?` (news opt-in), `hint?`, `sentHint?`, `onlogin?`, `labels?` |
| `AuthGate` | the door of a role app: loading → sign-in screen → "Your account (x@y) does not have access to this app." + **Switch account** (signs out everywhere → this app's sign-in) → `children`; props `session`, `app`, `appName`, `locale`, `readerHref?`, `logo?` |
| `SignedInAs` | "Signed in as x@y" + Sign out (everywhere); `compact` for a top bar |
| `authLabels(locale, overrides?)`, `AUTH_LABELS` | the de/en texts of all of the above |
| `APP_ROLES`, `canUseApp(app, roles)` | who may use which app (below) |

## How the session works

- **Sign-in**: `/auth/verify/` sets the cookie. The session checks `GET /auth/session/` (cookie only): works → the
  token from the answer is dropped; refused (cookies blocked in the browser) → the token goes to localStorage
  `scutoons_session` and is sent as Bearer (that app keeps working, without single sign-on).
- **Start** (`init`): the cookie (`GET /auth/session/`) → signed in. Else a token stored by an older build (localStorage
  `scutoons_session`) is moved into the cookie once (`POST /auth/session/` with Bearer), checked, then deleted.
- **Sign out** (`logout`): `POST /auth/logout/` ends the shared session and clears the cookie → every app is signed out.
  Other open apps notice when their tab becomes visible again (`refresh()` on `visibilitychange` / `focus`); an account
  switched in another app is picked up the same way.
- `pending`: still checking and this browser was signed in before (`localStorage.scutoons_signed_in` – a hint, no
  credential), so the app shows "loading" instead of flashing the sign-in.

## Role matrix (`APP_ROLES`)

| App | Who |
|---|---|
| viewer (scutoons.com) | anyone; member features need membership as before |
| shop | anyone (shows "signed in as" only) |
| creator | `creator` (the content API needs it; `admin` alone is not enough) |
| admin | `admin` |
| embassy | `liaison`, `ambassador` or `admin` |

The server checks the roles again on every request; the matrix only decides what an app shows.

## Example

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AuthGate, createAuthSession, takeMagicToken } from '@scutoons/auth';
	import { createApiClient, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';

	const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });
	const session = createAuthSession({ api: createQrApi(api), locale: getLocale });
	onMount(() => {
		const magic = takeMagicToken(page.url);
		if (magic) goto(magic.cleaned, { replaceState: true });
		void session.init(magic?.token);
	});
</script>

<AuthGate {session} app="creator" appName="Creator" locale={getLocale()} readerHref={VIEWER_URL}>
	…the app…
</AuthGate>
```

## Check

`npm run check -w @scutoons/auth` = `svelte-check --fail-on-warnings`. Browser: `node scripts/check-sso.mjs`.
