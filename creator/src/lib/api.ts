// The creator's API wiring: @scutoons/api with this app's API base, the viewer's static files and session token.
import { createApiClient, createContentApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL, STATIC_API } from '$lib/config.ts';

export {
	ApiError,
	CONTENT_PATH,
	getSessionToken,
	setSessionToken,
	type ContentIndexFile,
	type ContentVersion,
	type WrappedContent
} from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId });

export const { authRequest, authVerifyOtp, authVerifyToken, authLogout, fetchMe } = createQrApi(api);

export const contentApi = createContentApi({ client: api, staticBase: STATIC_API });
export const {
	fetchContent,
	getDraft,
	getVersion,
	putDraft,
	publishVersion,
	unpublishFile,
	restoreVersion,
	listVersions,
	contentIndex,
	getStatic
} = contentApi;
