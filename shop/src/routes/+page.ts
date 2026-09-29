import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { fetchProducts } from '$lib/api.ts';
import { m } from '$lib/paraglide/messages.js';

export const load: PageLoad = async () => {
	try {
		const catalog = await fetchProducts();
		if (catalog?.products) return { catalog };
	} catch (e) {
		console.error(e);
	}
	error(503, m['error.products']());
};
