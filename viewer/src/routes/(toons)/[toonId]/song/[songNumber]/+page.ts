import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';
import { API_BASE } from '$lib/config.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

export const load: PageLoad = async ({ fetch, params }) => {
	let response = await fetch(
		`${API_BASE}/${params.toonId}/${getLocale()}/song${params.songNumber}.json`
	);
	if(!response.ok) throw error(404, `Song not found for ${getLocale()}. If you think this is a mistake, try the German version.`)
	const song = await response.json();
	if (song) return { song, toonId: params.toonId };
	error(404, 'Not found');
};
