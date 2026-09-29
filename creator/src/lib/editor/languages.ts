// The toon's languages (source + translations) are set per toon, in its index entry (`ToonIndex.sourceLanguage` /
// `.languages`). The creator keeps the newest version in the content store as the language-neutral file
// `<toon>/_/index.json` (draft/publish like every other file); before the first save it comes from the viewer's
// static `toons.json`. Every editor takes its locales and its source from here.
import {
	NEUTRAL_LOCALE,
	TOON_INDEX_FILE,
	toonLanguages,
	type ToonIndex,
	type ToonLanguages
} from '@scutoons/content';
import { ApiError, getDraft } from '$lib/api.ts';
import { STATIC_API } from '$lib/config.ts';
import { ContentDoc } from './doc.svelte.ts';

/** The toon's entry of the viewer's static `toons.json` (all toons incl. drafts), or null. */
export async function staticToonEntry(toon: string): Promise<ToonIndex | null> {
	try {
		const response = await fetch(`${STATIC_API}/toons.json`, { cache: 'no-cache' });
		if (!response.ok) return null;
		const list = (await response.json()) as ToonIndex[];
		return (Array.isArray(list) ? list.find((t) => t?.id === toon) : null) ?? null;
	} catch {
		return null;
	}
}

/** The newest index entry of a toon (store draft → static toons.json), or null. */
export async function loadToonEntry(toon: string): Promise<ToonIndex | null> {
	try {
		return (await getDraft<ToonIndex>(toon, NEUTRAL_LOCALE, TOON_INDEX_FILE)).content;
	} catch (e) {
		if (!(e instanceof ApiError) || e.status !== 404) throw e;
	}
	return staticToonEntry(toon);
}

/** Source + languages of a toon (newest draft of its index entry; defaults for older entries: i18n keys / `de`). */
export async function loadToonLanguages(toon: string): Promise<ToonLanguages> {
	return toonLanguages(await loadToonEntry(toon));
}

/** The toon's index entry as an editable doc (one language-neutral file), saved/published like the other files. */
export function toonIndexDoc(toon: string): ContentDoc {
	return new ContentDoc(toon, TOON_INDEX_FILE, [NEUTRAL_LOCALE], NEUTRAL_LOCALE, async (locale) =>
		locale === NEUTRAL_LOCALE ? staticToonEntry(toon) : null
	);
}

/** A config file of the toon in all its languages, the source first. */
export function toonDoc(toon: string, path: string, languages: ToonLanguages): ContentDoc {
	return new ContentDoc(toon, path, languages.languages, languages.source);
}
