import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';
import type { LoadedSequenceItem, Chapter } from '$lib/models/content.ts';
import { isItemPublic } from '$lib/utils/template.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

export const load: PageLoad = async ({ fetch, params }) => {
	try {
		const lang = getLocale();
		const response1 = await fetch(`${API_BASE}/${params.toonId}/${lang}/toon.json`);
		if (!response1.ok) throw error(404, `Failed to load toon ${params.toonId} in your language (${lang}). Please use the German default for now until we were able to catch up.`);
		const toon = await response1.json();
		const response2 = await fetch(`${API_BASE}/${params.toonId}/${lang}/season1.json`);
		if (!response2.ok) throw error(404, `Failed to load season 1 of toon ${params.toonId} in your language (${lang}). Please use the German default for now until we were able to catch up.`);
		const season = await response2.json();

		const sequence: LoadedSequenceItem[] = [];

		for (const item of season.sequence) {
			try {
				const response = await fetch(
					`${API_BASE}/${params.toonId}/${lang}/${item.type}${item.number}.json`
				);
				if (!response.ok) throw error(404, `Failed to load ${item.type} ${item.number}`);
				const data = await response.json();
				sequence.push({
					...data,
					type: item.type,
					number: item.number
				});
			} catch (err) {
				throw error(404, `Failed to load ${item.type} from ${item.number}: ${err}`);
			}
		}

		// Filter to only show public items (published or draft in dev mode)
		const filteredSequence = sequence.filter((item) => isItemPublic(item as Chapter));

		if (toon && season) return { toon, season, sequence: filteredSequence };
	} catch (e) {
		error(404, 'Not found');
	}
};
