// The viewer's API wiring: @scutoons/api configured with this app's API base, static files and session token.
import {
	createApiClient,
	createLibraryApi,
	createProductsApi,
	createQrApi,
	getDeviceId,
	getSessionToken
} from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export {
	ApiError,
	getDeviceId,
	type AccessResult,
	type InventoryResult,
	type ProgressResult,
	type VerifyResult
} from '@scutoons/api';

// credentials: the single sign-on cookie of every SCUTOONS app (@scutoons/auth); a token stored by older builds still
// goes as Bearer until the session has moved it into the cookie
export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });

/** the account calls of the shared session (@scutoons/auth `createAuthSession`, contexts/session.svelte.ts) */
export const authApi = createQrApi(api);

export const {
	checkAccess,
	updateMe,
	fetchCode,
	claimCode,
	craftRecipe,
	grantItem,
	importItems,
	putAvatar,
	putAvatarBody,
	fetchProgress,
	putProgress
} = createQrApi(api);

/** Public data: a client without session / device id headers, so the browser sends a simple request (no CORS preflight). */
const publicApi = createApiClient({ baseUrl: API_URL });

/** Shop data (homepage slides): API /products/ and /products/featured/. */
export const { fetchProducts, fetchFeaturedProducts } = createProductsApi(publicApi);

/** Item catalog + editions (public; the inventory loads them on every page). */
export const { fetchCatalog, fetchEditions } = createQrApi(publicApi);

/**
 * The reader's content from the API (library endpoints, agents/DATA.md): the toon page in one request, one request per
 * chapter (scenes inlined), cast, credits, events. Public answers are kept 60 s, so the chapter page and the layout share
 * requests (agents/PERFORMANCE.md).
 */
export const library = createLibraryApi({ publicClient: publicApi, sessionClient: api, cacheMs: 60_000 });
export const { fetchToons, fetchToon, fetchChapter, fetchCharacters, fetchRegistry, fetchCredits, fetchEvents } = library;
