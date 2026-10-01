// The admin's API wiring: @scutoons/api with this app's API base; the session is the single sign-on cookie
// (@scutoons/auth), a token stored by older builds still goes as Bearer until the session has moved it into the cookie.
import { createAdminApi, createApiClient, createProductsApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export { ApiError } from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });

/** the account calls of the shared session (@scutoons/auth `createAuthSession`) */
export const authApi = createQrApi(api);

export const admin = createAdminApi(api);

/** The shop catalog (public), for choosing the homepage's editions */
export const { fetchProducts } = createProductsApi(api);
