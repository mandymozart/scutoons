# @scutoons/api

Typed client for the SCUTOONS API (agents/ITEMS.md → HTTP API contract): accounts / session, device id, access,
items, inventory, avatar, progress, and the content store (reader loader + creator editor endpoints).

Framework-agnostic TypeScript (no Svelte, no `import.meta.env`, no app state): the app injects the API base URL, the
static-file base and the session-token getter, and keeps its own state (session / access / inventory contexts) on top.
Depends on `@scutoons/content` (types) only.

## Public API (`import … from '@scutoons/api'`)

| Export | What |
|---|---|
| `createApiClient({ baseUrl, getToken?, getDeviceId?, fetch? })` | JSON client: `request`, `get`, `post`, `put`, `baseUrl`, `getToken()`. Sends `Accept`, `X-Device-Id`, `Authorization: Bearer`. Throws `ApiError` for non-2xx and `{status:"error"}` bodies. |
| `ApiError` | `{ status, code, message, body }`; network failure = `status 0`, `code 'network'`; 401 → `'unauthorized'`. |
| `createQrApi(client)` | `authRequest`, `authVerifyOtp`, `authVerifyToken`, `authLogout`, `checkAccess`, `fetchMe`, `updateMe`, `fetchCatalog`, `fetchEditions`, `fetchCode`, `claimCode`, `craftRecipe`, `grantItem`, `importItems`, `putAvatar`, `fetchProgress`, `putProgress` (+ types `VerifyResult`, `AccessResult`, `InventoryResult`, `ProgressResult`). |
| `createContentApi({ client, staticBase })` | `fetchContent(fetch, toon, locale, path, { draft? })` (store → static fallback, 4 s timeout, returns a `Response`), editor endpoints `getDraft`, `getVersion`, `putDraft`, `publishVersion`, `unpublishFile`, `restoreVersion`, `listVersions`, `contentIndex`, `getStatic`; `CONTENT_PATH`; types `ContentVersion`, `WrappedContent`, `ContentIndexFile`. |
| `createProductsApi(client)` | `fetchProducts()` (`GET /products/` → `ProductCatalog`), `fetchFeaturedProducts()` (`GET /products/featured/` → `FeaturedProduct[]`): the shop data, public. Give it a client without `getToken` / `getDeviceId` (`createApiClient({ baseUrl })`): then the browser sends a simple GET without a CORS preflight, which the API answers for any origin. |
| `getDeviceId()` | random per-browser id in localStorage `scutoons_device`. |
| `getSessionToken()` / `setSessionToken(token)` | localStorage `scutoons_session` of the current origin (`SESSION_STORAGE_KEY`). |

## Example

```ts
// app wiring, e.g. src/lib/api.ts
import { createApiClient, createQrApi, createContentApi, getDeviceId, getSessionToken, ApiError } from '@scutoons/api';

const API_URL = import.meta.env.VITE_API_URL || 'https://qr.scutoons.com/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId });
export const { fetchMe, authRequest, authVerifyOtp } = createQrApi(api);
export const content = createContentApi({ client: api, staticBase: '/api' });

try {
	const me = await fetchMe();
	console.log(me.user.roles);
} catch (e) {
	if (e instanceof ApiError && e.status === 401) {
		/* signed out */
	}
}

// in a SvelteKit load function
const response = await content.fetchContent(fetch, 'catpope', 'de', 'chapter1.json');
```

## Check

`npm run check -w @scutoons/api` = `tsc --noEmit`.
