// Shop data: GET /api/products/ (catalog) and /api/products/featured/ (homepage). Public, no session needed.
import type { FeaturedProduct, ProductCatalog } from '@scutoons/content';
import type { ApiClient } from './client.ts';

export function createProductsApi(api: ApiClient) {
	return {
		/** The shop catalog (server `config/products.json`, version 2) */
		fetchProducts: () => api.get<ProductCatalog>('/products/', { auth: false }),

		/** The products featured on the homepage (`[{ product, variant? }]`, resolved against the catalog) */
		fetchFeaturedProducts: () => api.get<FeaturedProduct[]>('/products/featured/', { auth: false })
	};
}

export type ProductsApi = ReturnType<typeof createProductsApi>;
