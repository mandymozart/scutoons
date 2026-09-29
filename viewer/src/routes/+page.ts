import type { PageLoad } from './$types.ts';
import type { FeaturedProduct, ProductCatalog } from '@scutoons/content';
import { fetchFeaturedProducts, fetchProducts } from '$lib/api.ts';

/** Featured products for the homepage slides (API /products/ + /products/featured/). The page still renders
 *  without them when the API is unreachable. */
export const load: PageLoad = async () => {
	try {
		const [featured, catalog] = await Promise.all([fetchFeaturedProducts(), fetchProducts()]);
		if (Array.isArray(featured) && catalog?.products) return { featured, catalog };
	} catch (e) {
		console.error('Error fetching products', e);
	}
	return { featured: [] as FeaturedProduct[], catalog: null as ProductCatalog | null };
};
