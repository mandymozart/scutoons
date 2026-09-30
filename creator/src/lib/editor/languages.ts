// The toon's languages (source + translations) are set per toon, in its index entry (`ToonIndex.sourceLanguage` /
// `.languages`), the entity `toon:<toon>` (draft/publish like every other entity). Every editor takes its locales and
// its source from here.
import { entities, NEUTRAL_LOCALE, sceneEntityOfRef, toonLanguages, type ToonIndex, type ToonLanguages } from '@scutoons/content';
import { ApiError, getDraft } from '$lib/api.ts';
import { ContentDoc } from './doc.svelte.ts';

/** The newest index entry of a toon, or null when it has none yet. */
export async function loadToonEntry(toon: string): Promise<ToonIndex | null> {
	try {
		return (await getDraft<ToonIndex>(entities.toon(toon))).content;
	} catch (e) {
		if (!(e instanceof ApiError) || e.status !== 404) throw e;
	}
	return null;
}

/** Source + languages of a toon (newest draft of its index entry; defaults for older entries: i18n keys / `de`). */
export async function loadToonLanguages(toon: string): Promise<ToonLanguages> {
	return toonLanguages(await loadToonEntry(toon));
}

/** The toon's index entry as an editable doc (language-neutral), saved/published like the other entities. */
export function toonIndexDoc(toon: string): ContentDoc {
	return new ContentDoc(entities.toon(toon), [NEUTRAL_LOCALE], NEUTRAL_LOCALE);
}

/** A localized entity of the toon (`entities.chapter(toon, 1)` …) in all its languages, the source first. */
export function toonDoc(entity: string, languages: ToonLanguages): ContentDoc {
	return new ContentDoc(entity, languages.languages, languages.source);
}

/**
 * A chapter's scene (a group stored on its own) by the group's reference `ref.src` (content format `chapter<N>/<name>.json`);
 * an invalid reference throws (the validation reports it).
 */
export function sceneDoc(toon: string, src: string, languages: ToonLanguages): ContentDoc {
	const entity = sceneEntityOfRef(toon, src);
	if (!entity) throw new Error(`Invalid scene reference ${src}`);
	return toonDoc(entity, languages);
}

/** A language-neutral entity (cast, credits, characters …) as an editable doc. */
export function neutralDoc(entity: string): ContentDoc {
	return new ContentDoc(entity, [NEUTRAL_LOCALE], NEUTRAL_LOCALE);
}
