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
	useItem,
	endEffect,
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
