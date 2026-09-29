import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { fetchProducts } from '$lib/api.ts';

export const load: PageLoad = async () => {
	try {
		const catalog = await fetchProducts();
		if (catalog?.products) return { catalog };
	} catch (e) {
		console.error(e);
	}
	error(404, 'Error fetching products');
};
