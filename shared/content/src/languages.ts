// Toon languages: the source language and the translations are set per toon (ToonIndex.sourceLanguage / .languages),
// not per chapter and not globally. Older index entries without the fields default to their `title` keys (then the old
// `i18n` keys) / `de`.
import type { ToonIndex } from './content.ts';
import { Language } from './enums.ts';

/** Locale key of language-neutral files in the content store (`<toon>/_/<path>`). */
export const NEUTRAL_LOCALE = '_';
/** The toon's index entry (the `toons.json` entry) in the content store: `<toon>/_/index.json`. */
export const TOON_INDEX_FILE = 'index.json';
/** Used when an entry names no language at all. */
export const DEFAULT_SOURCE_LANGUAGE = Language.DE;

export interface ToonLanguages {
	/** the source language (always `languages[0]`) */
	source: Language;
	/** every language of the toon, the source first, no duplicates */
	languages: Language[];
}

const LANGUAGES = Object.values(Language) as string[];
export const isLanguage = (value: unknown): value is Language => typeof value === 'string' && LANGUAGES.includes(value);

/**
 * Source + languages of a toon index entry. Missing or invalid fields fall back to the first available locale
 * (`languages`, then the `title` keys, then the old `i18n` keys) and finally `de`, so entries written before the fields
 * existed keep working.
 */
export function toonLanguages(entry?: Partial<ToonIndex> | null): ToonLanguages {
	const keys = (value: unknown) => (value !== null && typeof value === 'object' && !Array.isArray(value) ? Object.keys(value).filter(isLanguage) : []);
	const listed = (Array.isArray(entry?.languages) ? entry.languages : []).filter(isLanguage);
	const titled = listed.length ? listed : keys(entry?.title);
	const available = titled.length ? titled : keys(entry?.i18n);
	const source = isLanguage(entry?.sourceLanguage) ? entry.sourceLanguage : (available[0] ?? DEFAULT_SOURCE_LANGUAGE);
	const languages = [source, ...available.filter((l) => l !== source)];
	return { source, languages: [...new Set(languages)] };
}

/** Writes source + languages into an index entry (a copy), keeping the source first. */
export function withToonLanguages<T extends Partial<ToonIndex>>(entry: T, source: Language, languages: Language[]): T {
	const ordered = [...new Set([source, ...languages.filter((l) => l !== source)])];
	return { ...entry, sourceLanguage: source, languages: ordered };
}
