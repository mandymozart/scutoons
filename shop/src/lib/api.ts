// The shop's API wiring: public product data only, so a client without session / device id headers (a simple GET,
// no CORS preflight).
import { createApiClient, createProductsApi } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';

export const { fetchProducts } = createProductsApi(createApiClient({ baseUrl: API_URL }));
