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
	fetchCatalog,
	fetchEditions,
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

/** Shop data (homepage slides): API /products/ and /products/featured/. Public: a client without session / device id
 *  headers, so the browser sends a simple request (no CORS preflight). */
export const { fetchProducts, fetchFeaturedProducts } = createProductsApi(createApiClient({ baseUrl: API_URL }));

/** Toon / season / chapter configs and VN scene files: content store (published) → static `/api` files. */
export const contentApi = createContentApi({ client: api, staticBase: API_BASE });
export const { fetchContent, fetchCharacters, fetchRegistry } = contentApi;
