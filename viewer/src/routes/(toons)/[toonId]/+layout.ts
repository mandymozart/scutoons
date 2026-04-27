import { API_BASE } from '$lib/config.ts';
import type {
	LoadedSequenceItem,
	MapLocation,
	MapRoute,
	Season,
	Toon
} from '$lib/models/content.ts';
import type { Character, Collectable } from '$lib/models/visual-novel.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.js';
import { isItemPublic } from '$lib/utils/template.ts';
import { m } from '$lib/paraglide/messages.js';

export const load: LayoutLoad = async ({ fetch, params }) => {
	const lang = getLocale();
	const base = `${API_BASE}/${params.toonId}/${lang}`;

	const [charactersRes, seasonRes, locationsRes, routesRes, toonRes, collectablesRes] =
		await Promise.all([
			fetch(`${base}/characters.json`),
			fetch(`${base}/season1.json`),
			fetch(`${base}/locations.json`),
			fetch(`${base}/routes.json`),
			fetch(`${base}/toon.json`),
			fetch(`${base}/collectables.json`)
		]);

	if (!charactersRes.ok) throw error(404, `Failed to load characters of toon ${params.toonId}`);
	if (!seasonRes.ok) throw error(404, `Failed to load season 1 of toon ${params.toonId}`);
	if (!locationsRes.ok) throw error(404, `Failed to load locations of toon ${params.toonId}`);
	if (!routesRes.ok) throw error(404, `Failed to load routes of toon ${params.toonId}`);
	if (!toonRes.ok) throw error(404, `Failed to load toon ${params.toonId}`);
	if (!collectablesRes.ok) throw error(404, `Failed to load toon ${params.toonId}`);

	const characters: Character[] = await charactersRes.json();
	const season: Season = await seasonRes.json();
	const locations: MapLocation[] = await locationsRes.json();
	const routes: MapRoute[] = await routesRes.json();
	const toon: Toon = await toonRes.json();
	const collectables: Collectable[] = await collectablesRes.json();

	const publicItems = season.sequence.filter(isItemPublic);

	const sequence: LoadedSequenceItem[] = await Promise.all(
		publicItems.map(async (item) => {
			const response = await fetch(`${base}/${item.type}${item.number}.json`);
			if (!response.ok)
				throw error(
					404,
					`Failed to load sequence item ${m['common.chapter']({ number: item.number })} (${lang}). Please use the German default for now until we were able to catch up.`
				);
			const loadedItem = await response.json();
			return { ...loadedItem, type: item.type, number: item.number };
		})
	);

	return { sequence, characters, locations, routes, toon, collectables, season };
};
