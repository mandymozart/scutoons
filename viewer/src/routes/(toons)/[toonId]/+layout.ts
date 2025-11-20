import { API_BASE } from '$lib/config.ts';
import type { LoadedSequenceItem, Season } from '$lib/models/content.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.js';
import { isItemPublic } from '$lib/utils/template.ts';
import { m } from '$lib/paraglide/messages.js';

export const load: LayoutLoad = async ({ fetch, params }) => {
	const lang = getLocale();
	// sequence should move to toon and not season, season on has meta data
	const response1 = await fetch(`${API_BASE}/${params.toonId}/${lang}/season1.json`);
	if (!response1.ok) throw error(404, `Failed to load season 1 of toon ${params.toonId}`);
	const season: Season = await response1.json();

	const sequence: LoadedSequenceItem[] = [];

	for (const item of season.sequence) {
		const response2 = await fetch(
			`${API_BASE}/${params.toonId}/${lang}/${item.type}${item.number}.json`
		);
		if (!response2.ok)
			throw error(
				404,
				`Failed to load song ${m[item.type]({ number: item.number })} (${lang}). Please use the German default for now until we were able to catch up.`
			);
		const loadedItem = await response2.json();
		if (isItemPublic(loadedItem))
			sequence.push({
				...loadedItem,
				type: item.type,
				number: item.number
			});
	}
	if (season && sequence) return { sequence };
	error(404, 'Not found');
};
