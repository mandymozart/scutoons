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
import { isItemListed, isItemPublic, Status, type Chapter, type ToonIndex } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fetchCharacters, fetchContent } from '$lib/api.ts';

export const load: LayoutLoad = async ({ fetch, params, parent }) => {
	const lang = getLocale();
	// Draft toons (the sandbox, unreleased toons) exist in dev only, like draft chapters (isItemPublic): in a production
	// build the toon and every page below it (chapters, credits, download) is the normal 404. Published = its toons.json
	// entry (what the home page and menu list; the root layout keeps only published ones) and its own toon.json.
	const gate = import.meta.env.DEV
		? Promise.resolve()
		: parent().then(({ toons }) => {
				if (!(toons as ToonIndex[] | undefined)?.some((t) => t.id === params.toonId || t.uri === params.toonId)) error(404, 'Not found');
			});
	const base = `${API_BASE}/${params.toonId}/${lang}`;
	// toon / season / chapter configs: content store first (published versions), static files as fallback
	const content = (path: string) => fetchContent(fetch, params.toonId, lang, path);

	// The chapter files depend on the season only: they start as soon as season1.json is there, not after the slowest of
	// the other files (agents/PERFORMANCE.md).
	const seasonPromise = content('season1.json').then(async (seasonRes) => {
		if (!seasonRes.ok) throw error(404, `Failed to load season 1 of toon ${params.toonId}`);
		const season: Season = await seasonRes.json();
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
		return { season, sequence };
	});

	// characters: registry + the toon's cast from the content store, resolved to the old characters.json shape;
	// fallback: the static characters.json of the locale; a toon without any → []
	const [, charactersResult, { season, sequence }, locationsRes, routesRes, toonRes, collectablesRes] =
		await Promise.all([
			gate,
			fetchCharacters(fetch, params.toonId, lang),
			seasonPromise,
			fetch(`${base}/locations.json`),
			fetch(`${base}/routes.json`),
			content('toon.json'),
			fetch(`${base}/collectables.json`)
		]);

	if (!locationsRes.ok) throw error(404, `Failed to load locations of toon ${params.toonId}`);
	if (!routesRes.ok) throw error(404, `Failed to load routes of toon ${params.toonId}`);
	if (!toonRes.ok) throw error(404, `Failed to load toon ${params.toonId}`);
	if (!collectablesRes.ok) throw error(404, `Failed to load toon ${params.toonId}`);

	const characters: Character[] = charactersResult.characters;
	const locations: MapLocation[] = await locationsRes.json();
	const routes: MapRoute[] = await routesRes.json();
	const toon: Toon = await toonRes.json();
	if (!isItemPublic(toon, import.meta.env.DEV)) error(404, 'Not found');
	const collectables: Collectable[] = await collectablesRes.json();

	return { sequence, characters, locations, routes, toon, collectables, season };
};
