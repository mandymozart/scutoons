// The creator's API wiring: @scutoons/api with this app's API base and session token. Content is read and written by
// entity key (`/api/entities`, the creator's write path) and previewed through the library (`draft`); agents/DATA.md.
import { createApiClient, createAssetsApi, createContentApi, createLibraryApi, createQrApi, createScenesApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export {
	ApiError,
	type ContentIndexEntity,
	type ContentVersion,
	type WrappedContent
} from '@scutoons/api';

// credentials: the single sign-on cookie (@scutoons/auth); a token stored by older builds still goes as Bearer until the
// session has moved it into the cookie
export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });

/** the account calls of the shared session (@scutoons/auth `createAuthSession`) */
export const authApi = createQrApi(api);

export const contentApi = createContentApi(api);
export const { getDraft, getPublished, getVersion, putDraft, publishVersion, unpublish, restoreVersion, listVersions, contentIndex, withContent, saveBatch, publishMany } = contentApi;

/** Reader views of the content (drafts with `{ draft: true }`), e.g. the preview and the toons list */
export const library = createLibraryApi({ publicClient: api, sessionClient: api });

/** The asset registry (agents/ASSETS.md): the asset picker browses, looks up and uploads media */
export const assetsApi = createAssetsApi(api);

/** The deck scenes `scene:<id>` (#490): their own route `/api/scenes` (the entity routes refuse `scene:` keys) */
export const scenesApi = createScenesApi(api);
