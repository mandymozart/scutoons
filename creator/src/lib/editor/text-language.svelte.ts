// The creator's content language, the "i18n" switch in the top bar (owner 2026-10-02; agents/DESIGN.md "Creator: content
// language (i18n) + translate mode"): which content language the translatable fields show – "source" (each field in its
// original language, the default since the owner's "less UI is better UI", 2026-10-02), "all" (every toon language side
// by side) or one of them. Not the UI language (paraglide). One value per browser (localStorage),
// shared by every page and toon; a language the current toon doesn't have shows all. When the chosen language is not
// the field's source, the field is in translate mode: the source text as a muted line under it, read-only, with
// "Copy original".

const KEY = 'scutoons_creator_text_language';
export const ALL_TEXT_LANGUAGES = 'all';
/** each field in its source language – the default */
export const SOURCE_TEXT_LANGUAGE = 'source';

function read(): string {
	try {
		return localStorage.getItem(KEY) || SOURCE_TEXT_LANGUAGE;
	} catch {
		return SOURCE_TEXT_LANGUAGE;
	}
}

class TextLanguage {
	/** `source`, `all` or a language code */
	value = $state<string>(read());

	set(next: string) {
		this.value = next || SOURCE_TEXT_LANGUAGE;
		try {
			if (this.value === SOURCE_TEXT_LANGUAGE) localStorage.removeItem(KEY);
			else localStorage.setItem(KEY, this.value);
		} catch {
			/* storage blocked: the choice lasts for this page load */
		}
	}

	/** the chosen language when `languages` has it, else null (source or all) */
	chosen(languages: readonly string[]): string | null {
		return this.value !== ALL_TEXT_LANGUAGES && languages.includes(this.value) ? this.value : null;
	}

	/** the languages a translatable field shows: the chosen one, the field's source (default), or all of them */
	visible(languages: readonly string[], source?: string | null): string[] {
		const one = this.chosen(languages);
		if (one) return [one];
		if (this.value !== ALL_TEXT_LANGUAGES && source && languages.includes(source)) return [source];
		return [...languages];
	}

	/** what the switch shows: a language code, `source` or `all` */
	mode(languages: readonly string[]): string {
		return this.chosen(languages) ?? (this.value === ALL_TEXT_LANGUAGES ? ALL_TEXT_LANGUAGES : SOURCE_TEXT_LANGUAGE);
	}

	/** translate mode: the source to show above the field (read-only), or null (all / the source itself is chosen) */
	original(languages: readonly string[], source: string): string | null {
		const one = this.chosen(languages);
		return one && one !== source && languages.includes(source) ? source : null;
	}
}

export const textLanguage = new TextLanguage();

/** The language a list shows titles in: the chosen one (when the toon has it), else the source. */
export function listLanguage(languages: readonly string[], source: string): string {
	return textLanguage.chosen(languages) ?? source;
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
