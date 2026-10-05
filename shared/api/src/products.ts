// Shop data: GET /api/products/ (catalog) and /api/products/featured/ (homepage). Public, no session needed.
import type { FeaturedProduct, ProductCatalog } from '@scutoons/content';
import type { ApiClient } from './client.ts';

/** One store of the public "Where to buy" list (GET /api/stockists/, server docs/API.md "Stores") – public fields only */
export interface Stockist {
	id: string;
	name: string;
	type: 'store' | 'record_shop';
	/** a small / independent vendor (listed first) */
	independent: boolean;
	city: string;
	/** ISO 3166-1 alpha-2, '' = unknown */
	country: string;
	address: string;
	/** http(s) link or '' */
	website: string;
	opening: string;
	/** product ids (`ProductCatalog.products[].id`) */
	carries: string[];
}

export interface StockistList {
	/** filtered by the params; independent first, then country, city, name */
	stores: Stockist[];
	/** product id → number of listed stores carrying it (all stores, unfiltered) */
	products: Record<string, number>;
	/** countries with a listed store (unfiltered) */
	countries: string[];
}

export function createProductsApi(api: ApiClient) {
	return {
		/** Stores that carry SCUTOONS products (the Embassy's listed stores), optionally for one product / country */
		fetchStockists: (params: { product?: string; country?: string } = {}) => {
			const query = new URLSearchParams(Object.entries(params).filter((e): e is [string, string] => !!e[1])).toString();
			return api.get<StockistList>(`/stockists/${query ? `?${query}` : ''}`, { auth: false });
		},

		/** The shop catalog (version 2; the catalog tables, agents/CATALOG.md) */
		fetchProducts: () => api.get<ProductCatalog>('/products/', { auth: false }),

		/** The products featured on the homepage (`[{ product, variant? }]`, resolved against the catalog) */
		fetchFeaturedProducts: () => api.get<FeaturedProduct[]>('/products/featured/', { auth: false })
	};
}

export type ProductsApi = ReturnType<typeof createProductsApi>;
