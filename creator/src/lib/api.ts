// The creator's API wiring: @scutoons/api with this app's API base and session token. Content is read and written by
// entity key (`/api/entities`, the creator's write path) and previewed through the library (`draft`); agents/DATA.md.
import { createApiClient, createContentApi, createLibraryApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export {
	ApiError,
	getSessionToken,
	setSessionToken,
	type ContentIndexEntity,
	type ContentVersion,
	type WrappedContent
} from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId });

export const { authRequest, authVerifyOtp, authVerifyToken, authLogout, fetchMe } = createQrApi(api);

export const contentApi = createContentApi(api);
export const { getDraft, getPublished, getVersion, putDraft, publishVersion, unpublish, restoreVersion, listVersions, contentIndex } = contentApi;

/** Reader views of the content (drafts with `{ draft: true }`), e.g. the preview and the toons list */
export const library = createLibraryApi({ publicClient: api, sessionClient: api });
