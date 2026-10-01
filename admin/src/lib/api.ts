// The admin's API wiring: @scutoons/api with this app's API base and session token (localStorage of this origin).
import { createAdminApi, createApiClient, createProductsApi, createQrApi, getDeviceId, getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export { ApiError, getSessionToken, setSessionToken } from '@scutoons/api';

export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId });

export const { authRequest, authVerifyOtp, authVerifyToken, authLogout, fetchMe } = createQrApi(api);

export const admin = createAdminApi(api);

/** The shop catalog (public), for choosing the homepage's editions */
export const { fetchProducts } = createProductsApi(api);
