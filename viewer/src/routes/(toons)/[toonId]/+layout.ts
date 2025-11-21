import { API_BASE } from '$lib/config.ts';
import type { LoadedSequenceItem, Season } from '$lib/models/content.ts';
import type { Character } from '$lib/models/visual-novel.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.js';
import { isItemPublic } from '$lib/utils/template.ts';
import { m } from '$lib/paraglide/messages.js';

export const load: LayoutLoad = async ({ fetch, params }) => {
	const lang = getLocale();

	const response1 = await fetch(`${API_BASE}/${params.toonId}/${lang}/season1.json`);
	if (!response1.ok) throw error(404, `Failed to load season 1 of toon ${params.toonId}`);
	const season: Season = await response1.json();

	const response2 = await fetch(`${API_BASE}/${params.toonId}/${lang}/characters.json`);
	if (!response2.ok) throw error(404, `Failed to load characters of toon ${params.toonId}`);
	const characters: Character[] = await response2.json();

	const sequence: LoadedSequenceItem[] = [];

	for (const item of season.sequence) {
		const response3 = await fetch(
			`${API_BASE}/${params.toonId}/${lang}/${item.type}${item.number}.json`
		);
		if (!response3.ok)
			throw error(
				404,
				`Failed to load song ${m[item.type]({ number: item.number })} (${lang}). Please use the German default for now until we were able to catch up.`
			);
		const loadedItem = await response3.json();
		if (isItemPublic(loadedItem))
			sequence.push({
				...loadedItem,
				type: item.type,
				number: item.number
			});
	}
	if (season && sequence) return { sequence, characters };
	error(404, 'Not found');
};
