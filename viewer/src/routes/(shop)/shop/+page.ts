import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';

export const load: PageLoad = async ({ fetch }) => {
	try {
		const response = await fetch(`${API_BASE}/products.json`);
		const products = await response.json();
		if (products) return { products };
	} catch (e) {
		error(404, 'Error fetching products');
	}
};
