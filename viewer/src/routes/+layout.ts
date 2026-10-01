export const csr = true;
export const prerender = false;
export const ssr = false;

import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.ts';
import { isToonListed } from '@scutoons/content';
import { fetchToons } from '$lib/api.ts';

/**
 * The toons of the home page and the menu from the API's library (agents/DATA.md): the published ones and the announced
 * ones (`announced`: listed as "soon", never a link; owner 2026-10-01). Drafts stay out, also in dev.
 */
export const load: LayoutLoad = async () => {
	try {
		const toons = (await fetchToons()).filter(isToonListed);
		return { toons };
	} catch (e) {
		error(503, 'Not available');
	}
};
