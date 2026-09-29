// The viewer's API wiring: @scutoons/api configured with this app's API base, static files and session token.
import { createApiClient, createContentApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
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
	fetchProgress,
	putProgress
} = createQrApi(api);

/** Toon / season / chapter configs and VN scene files: content store (published) → static `/api` files. */
export const contentApi = createContentApi({ client: api, staticBase: API_BASE });
export const { fetchContent } = contentApi;
// editor endpoints for lib/creator until the creator moves to its own app
export const { getDraft, getVersion, putDraft, publishVersion, unpublishFile, restoreVersion, listVersions, contentIndex, getStatic } =
	contentApi;
export { CONTENT_PATH, type ContentVersion, type WrappedContent, type ContentIndexFile } from '@scutoons/api';
