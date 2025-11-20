import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';

export const load: PageLoad = async ({ fetch, params }) => {
	let response = await fetch(`${API_BASE}/${params.toonId}/de/credits.json`);
	const credits = await response.json();
	if (credits) return { credits };
	error(404, 'Not found');
};
