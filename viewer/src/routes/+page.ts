import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';
import type { FeaturedProduct, ProductCatalog } from '$lib/components/shop/products/shop.ts';

export const load: PageLoad = async ({ fetch }) => {
	try {
		const [featuredRes, catalogRes] = await Promise.all([
			fetch(`${API_BASE}/featured-products.json`),
			fetch(`${API_BASE}/products.json`)
		]);
		const featured: FeaturedProduct[] = await featuredRes.json();
		const catalog: ProductCatalog = await catalogRes.json();
		if (featured && catalog?.products) return { featured, catalog };
	} catch (e) {
		console.error(e);
	}
	error(404, 'Error fetching products');
};
