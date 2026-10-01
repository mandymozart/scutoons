import type { Character, LoadedSequenceItem, Season, Toon, ToonIndex } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.js';
import { isItemListed, isItemPublic, Status } from '@scutoons/content';
import { ApiError, fetchCharacters, fetchToon } from '$lib/api.ts';

export const load: LayoutLoad = async ({ params, parent }) => {
	const lang = getLocale();
	// Draft toons (the sandbox, unreleased toons) exist in dev only, like draft chapters (isItemPublic): in a production
	// build the toon and every page below it (chapters, credits, download) is the normal 404. Published = its index entry
	// (the root layout's list; announced toons are listed there too, as "soon", but never readable) and its settings. The
	// API applies the same rule (CONTENT_SHOW_DRAFTS locally).
	const gate = import.meta.env.DEV
		? Promise.resolve()
		: parent().then(({ toons }) => {
				const listed = (toons as ToonIndex[] | undefined)?.find((t) => t.id === params.toonId || t.uri === params.toonId);
				if (listed?.status !== Status.PUBLISHED) error(404, 'Not found');
			});

	// The toon page in one request (library: settings, every season with its chapter list) + the characters
	// (registry reduced to the toon's cast, resolved in the reader's locale); agents/DATA.md.
	const [, bundle, characters] = await Promise.all([
		gate,
		fetchToon(params.toonId, lang).catch((e) => {
			if (e instanceof ApiError && e.status === 404) error(404, 'Not found');
			throw error(e instanceof ApiError && e.status ? e.status : 503, `Failed to load toon ${params.toonId}`);
		}),
		fetchCharacters(params.toonId, lang).catch((): Character[] => [])
	]);

	const toon: Toon = bundle.toon;
	if (!isItemPublic(toon, import.meta.env.DEV)) error(404, 'Not found');
	// the reader shows season 1 (visibility = the season's status per locale; coming-soon chapters are listed, not clickable)
	const overview = bundle.seasons.find((s) => s.number === 1) ?? bundle.seasons[0];
	if (!overview) error(404, `Toon ${params.toonId} has no season`);
	const season: Season = overview.season;
	const sequence: LoadedSequenceItem[] = overview.chapters.filter((item) => isItemListed(item, import.meta.env.DEV));
	// the map loads its own data (`/library/map`, agents/MAP.md M1); the bundle's `locations` / `routes` are for older builds
	return { sequence, characters, toon, season };
};
