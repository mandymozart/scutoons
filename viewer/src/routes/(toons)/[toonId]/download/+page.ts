import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Toon } from '$lib/models/content.ts';
import { fetchContent } from '$lib/api/content.service.ts';

export const load: PageLoad = async ({ fetch, params }) => {
	const response = await fetchContent(fetch, params.toonId, getLocale(), 'toon.json');
	if (!response.ok) throw error(404, `Failed to load toon ${params.toonId}`);
	const toon: Toon = await response.json();

	if (toon) return { toon };
	error(404, 'Not found');
};
