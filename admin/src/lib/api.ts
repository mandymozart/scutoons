// The admin's API wiring: @scutoons/api with this app's API base; the session is the single sign-on cookie
// (@scutoons/auth), a token stored by older builds still goes as Bearer until the session has moved it into the cookie.
import { createAdminApi, createAgentChatsApi, createApiClient, createAssetsApi, createContentApi, createProductsApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export { ApiError } from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });

/** the account calls of the shared session (@scutoons/auth `createAuthSession`) */
export const authApi = createQrApi(api);

export const admin = createAdminApi(api);

/** The content store (role admin counts as content editor): the platform-wide `events` document */
export const content = createContentApi(api);

/** The asset manager (agents/ASSETS.md): registry, storage operations, jobs */
export const assets = createAssetsApi(api);

/** The shop catalog (public), for choosing the homepage's editions */
export const { fetchProducts } = createProductsApi(api);

/** the Ask panel's chats (`/api/agent-chats`, app admin) */
export const agentChats = createAgentChatsApi(api, 'admin');
