import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';
import type { ProductCatalog } from '$lib/components/shop/products/shop.ts';

export const load: PageLoad = async ({ fetch }) => {
	try {
		const response = await fetch(`${API_BASE}/products.json`);
		const catalog: ProductCatalog = await response.json();
		if (catalog?.products) return { catalog };
	} catch (e) {
		console.error(e);
	}
	error(404, 'Error fetching products');
};
