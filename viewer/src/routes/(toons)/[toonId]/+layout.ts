import { API_BASE } from '$lib/config.ts';
import type {
	LoadedSequenceItem,
	MapLocation,
	MapRoute,
	Season,
	Toon
} from '@scutoons/content';
import type { Character, Collectable } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.js';
import { isItemListed, Status, type Chapter } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fetchContent } from '$lib/api.ts';

export const load: LayoutLoad = async ({ fetch, params }) => {
	const lang = getLocale();
	const base = `${API_BASE}/${params.toonId}/${lang}`;
	// toon / season / chapter configs: content store first (published versions), static files as fallback
	const content = (path: string) => fetchContent(fetch, params.toonId, lang, path);

	const [charactersRes, seasonRes, locationsRes, routesRes, toonRes, collectablesRes] =
		await Promise.all([
			fetch(`${base}/characters.json`),
			content('season1.json'),
			fetch(`${base}/locations.json`),
			fetch(`${base}/routes.json`),
			content('toon.json'),
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

	// visibility = the season's status per locale (the chapter files' own `status` is not read): readable chapters
	// (drafts only in dev) and coming-soon ones, which are listed but not clickable
	const listedItems = season.sequence.filter((item) => isItemListed(item, import.meta.env.DEV));

	const sequence: LoadedSequenceItem[] = await Promise.all(
		listedItems.map(async (item) => {
			const response = await content(`${item.type}${item.number}.json`);
			if (!response.ok) {
				// a coming-soon entry doesn't need its file (title / thumbnail / release stay empty)
				if (item.status === Status.COMING_SOON)
					return { title: '', thumbnail: '', release: '', type: item.type, number: item.number, status: item.status } as LoadedSequenceItem;
				throw error(
					404,
					`Failed to load sequence item ${m['common.chapter']({ number: item.number })} (${lang}). Please use the German default for now until we were able to catch up.`
				);
			}
			const { status: _deprecated, ...loadedItem }: Chapter = await response.json();
			return { ...loadedItem, type: item.type, number: item.number, status: item.status };
		})
	);

	return { sequence, characters, locations, routes, toon, collectables, season };
};
