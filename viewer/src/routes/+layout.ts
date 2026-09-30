export const csr = true;
export const prerender = false;
export const ssr = false;

import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.ts';
import { Status } from '@scutoons/content';
import { fetchToons } from '$lib/api.ts';

/** The published toons (home page, menu) from the API's library (agents/DATA.md). */
export const load: LayoutLoad = async () => {
	try {
		const toons = (await fetchToons()).filter((toon) => toon.status === Status.PUBLISHED);
		return { toons };
	} catch (e) {
		error(503, 'Not available');
	}
};
