// The toon's languages (source + translations) are set per toon, in its index entry (`ToonIndex.sourceLanguage` /
// `.languages`), the entity `toon:<toon>` (draft/publish like every other entity). Every editor takes its locales and
// its source from here.
import { entities, NEUTRAL_DOC_KINDS, NEUTRAL_LOCALE, parseEntityKey, toonLanguages, type ToonIndex, type ToonLanguages } from '@scutoons/content';
import { ApiError, getDraft } from '$lib/api.ts';
import { ContentDoc } from '$lib/editor/doc.svelte.ts';

/** The newest index entry of a toon, or null when it has none yet. */
export async function loadToonEntry(toon: string): Promise<ToonIndex | null> {
	try {
		return (await getDraft<ToonIndex>(entities.toon(toon))).content;
	} catch (e) {
		if (!(e instanceof ApiError) || e.status !== 404) throw e;
	}
	return null;
}

/** Source + languages of a toon (newest draft of its index entry; defaults for older entries: `title` keys, the old `i18n` keys / `de`). */
export async function loadToonLanguages(toon: string): Promise<ToonLanguages> {
	return toonLanguages(await loadToonEntry(toon));
}

/**
 * The toon's index entry as an editable doc (language-neutral), saved/published like the other entities. With the toon's
 * `languages` its texts (title, description, release, status per language – agents/NEUTRAL.md step 4) are edited like a
 * neutral document's (`text` / `setText`). Without: the entry as stored (the toon page's languages).
 */
export function toonIndexDoc(toon: string, languages?: ToonLanguages): ContentDoc {
	return new ContentDoc(entities.toon(toon), [NEUTRAL_LOCALE], NEUTRAL_LOCALE, languages ? { source: languages.source, languages: languages.languages } : null);
}

/**
 * A localized entity of the toon in all its languages, the source first: a chapter / season is ONE
 * language-neutral document with its texts per language (agents/NEUTRAL.md); the script is one document per language
 * (a translation by design). The toon settings are the toon entry's (`toonIndexDoc` with the languages).
 */
export function toonDoc(entity: string, languages: ToonLanguages): ContentDoc {
	const kind = parseEntityKey(entity)?.kind;
	if (kind && NEUTRAL_DOC_KINDS.includes(kind)) return new ContentDoc(entity, [NEUTRAL_LOCALE], NEUTRAL_LOCALE, { source: languages.source, languages: languages.languages });
	return new ContentDoc(entity, languages.languages, languages.source);
}

/** A language-neutral entity (cast, credits, characters …) as an editable doc. */
export function neutralDoc(entity: string): ContentDoc {
	return new ContentDoc(entity, [NEUTRAL_LOCALE], NEUTRAL_LOCALE);
}
