// The viewer's API wiring: @scutoons/api configured with this app's API base, static files and session token.
import {
	createApiClient,
	createContentApi,
	createProductsApi,
	createQrApi,
	getDeviceId,
	getSessionToken
} from '@scutoons/api';
import { API_BASE, API_URL } from '$lib/config.ts';

export {
	ApiError,
	SESSION_STORAGE_KEY,
	getDeviceId,
	getSessionToken,
	setSessionToken,
	type AccessResult,
	type InventoryResult,
	type ProgressResult,
	type VerifyResult
} from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId });

export const {
	authRequest,
	authVerifyOtp,
	authVerifyToken,
	checkAccess,
	authLogout,
	fetchMe,
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
 * Toon / season / chapter configs and VN scene files: content store (published) → static `/api` files. Published reads
 * are kept 60 s, so the chapter page reuses the `chapterN.json` the toon layout just loaded (agents/PERFORMANCE.md).
 */
export const contentApi = createContentApi({ client: api, staticBase: API_BASE, cacheMs: 60_000 });
export const { fetchContent, fetchCharacters, fetchRegistry } = contentApi;
