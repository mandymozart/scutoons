export const csr = true;
export const prerender = false;
export const ssr = false;

import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';
import type { ToonIndex } from '$lib/models/content.ts';
import { Status } from '$lib/models/enums.ts';

export const load: LayoutLoad = async ({ fetch }) => {
	try {
		const response = await fetch(`${API_BASE}/toons.json`);
		let toons = await response.json();
		toons = toons.filter((toon: ToonIndex) => toon.status === Status.PUBLISHED);

		if (toons) return { toons };
	} catch (e) {
		error(404, 'Not found');
	}
};
