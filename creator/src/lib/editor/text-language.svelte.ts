// The creator's content language, the "i18n" switch in the top bar (owner 2026-10-02, 2026-10-06; agents/DESIGN.md
// "Creator: content language (i18n) + translate mode"): which ONE content language the translatable fields show. Not
// the UI language (paraglide). One language code per browser (localStorage), shared by every page and toon; nothing
// stored (or a language the current toon doesn't have) = the source language – the toon's, or the platform's `de` on
// pages without a toon. When the shown language is not the field's source, the field is in translate mode: the source
// text as a muted line under it, read-only, with "Copy original". (Owner 2026-10-06: no "all languages side by side"
// and no "each field in its source" mode any more – an old stored "all" / "source" reads as the source.)
import { characterName, type RegistryCharacter } from '@scutoons/content';
import { shared } from '$lib/utils/shared.ts';

const KEY = 'scutoons_creator_text_language';
/** the platform's source language: the default on pages without a toon (characters, moods, roles, map names) */
export const PLATFORM_SOURCE_LANGUAGE = 'de';
/** the source-language mark where only text fits (a native option, a dialog line); elsewhere `SourceMark` (Feather star) */
export const SOURCE_MARK = '★';

/** the stored choice: a language code, or '' (the source) – an old "all" / "source" counts as the source */
function read(): string {
	try {
		const value = localStorage.getItem(KEY) ?? '';
		return value === 'all' || value === 'source' ? '' : value;
	} catch {
		return '';
	}
}

class TextLanguage {
	/** the chosen language code, '' = the source language */
	value = $state<string>(read());
	/**
	 * the languages the header's switch offers on this page (the layout sets them): a toon page the toon's, the universe
	 * pages every toon's with the platform's `de` as source; empty while the toon is not loaded yet
	 */
	page = $state<{ languages: string[]; source: string }>({ languages: [], source: PLATFORM_SOURCE_LANGUAGE });

	set(next: string) {
		this.value = next === 'all' || next === 'source' ? '' : next || '';
		try {
			if (this.value) localStorage.setItem(KEY, this.value);
			else localStorage.removeItem(KEY);
		} catch {
			/* storage blocked: the choice lasts for this page load */
		}
	}

	/** the chosen language when `languages` has it, else null (= the source) */
	chosen(languages: readonly string[]): string | null {
		return this.value && languages.includes(this.value) ? this.value : null;
	}

	/** the one language shown: the chosen one when `languages` has it, else the source (else the first language) */
	current(languages: readonly string[], source?: string | null): string {
		return this.chosen(languages) ?? (source && languages.includes(source) ? source : (languages[0] ?? source ?? ''));
	}

	/**
	 * THE language every shown content name follows – characters, personas, looks, items, recipes, albums (owner
	 * 2026-10-07, BACKLOG 10.51): the header's switch as it stands on this page. A page without a language list yet
	 * (the toon still loading) takes the remembered choice, else the page's source.
	 */
	get names(): string {
		const { languages, source } = this.page;
		return languages.length ? this.current(languages, source) : this.value || source;
	}

	/** the names' fallback chain after `names`: the page's source, then en / de / id */
	get nameFallbacks(): string[] {
		return [...new Set([this.page.source, 'en', 'de', 'id'])].filter((l) => l !== this.names);
	}

	/** the languages a translatable field shows: exactly one – the chosen one, else the field's source */
	visible(languages: readonly string[], source?: string | null): string[] {
		const one = this.current(languages, source);
		return one ? [one] : [];
	}

	/** translate mode: the source to show under the field (read-only), or null when the source itself is shown */
	original(languages: readonly string[], source: string): string | null {
		const one = this.current(languages, source);
		return one && one !== source && languages.includes(source) ? source : null;
	}
}

export const textLanguage = shared('text-language', () => new TextLanguage());

/** The language a list shows titles in: the chosen one (when the toon has it), else the source. */
export function listLanguage(languages: readonly string[], source: string): string {
	return textLanguage.chosen(languages) ?? source;
}

/** a content name (`{ de: …, en: … }`) in the names' language (`textLanguage.names`), else its fallbacks, else `id` */
export function contentName(name: Record<string, string> | null | undefined, id: string): string {
	if (!name) return id;
	for (const l of [textLanguage.names, ...textLanguage.nameFallbacks]) if (name[l]) return name[l];
	return id;
}

/** a character's shown name ("Katzenpapst · Biker" with a persona) in the names' language – the creator's ONE way */
export function shownCharacterName(character: RegistryCharacter | null | undefined, persona?: string): string {
	return characterName(character, textLanguage.names, persona, textLanguage.nameFallbacks);
}

/** a language code for a text-only place, the source marked: "de ★" */
export function markedLanguage(locale: string, source?: string | null): string {
	return locale === source ? `${locale} ${SOURCE_MARK}` : locale;
}

/** The other languages' texts on one line for a tooltip: "en: Meeting Yellow Mouse · ko: 만남: …" */
export function otherTexts(languages: readonly string[], shown: string, text: (locale: string) => unknown): string {
	return languages
		.filter((l) => l !== shown)
		.map((l) => {
			const value = text(l);
			return `${l}: ${typeof value === 'string' && value ? value : '–'}`;
		})
		.join(' · ');
}
