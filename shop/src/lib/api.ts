// The shop's API wiring. Product data: public, a client without session / device id headers (a simple GET, no CORS
// preflight). The account: the single sign-on session of every SCUTOONS app (@scutoons/auth, cookie → credentials).
import { createApiClient, createProductsApi, createQrApi, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export const { fetchProducts } = createProductsApi(createApiClient({ baseUrl: API_URL }));

/** the account calls of the shared session (signed in on any SCUTOONS app = signed in here) */
export const authApi = createQrApi(createApiClient({ baseUrl: API_URL, getToken: getSessionToken, credentials: 'include' }));
