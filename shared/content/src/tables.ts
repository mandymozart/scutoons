// GENERATED from server/schema/tables/*.json by server/schema/generate.ts – edit the source, run `npm run generate`.

/** Former characters that are personas now (owner 2026-09-30): an old VN / deck character id → the registry character and the persona its art went to (production data: catpope's `human` persona carries plf's art, `cat` cat's, `mountaintiger` mountain-tiger's). Generated into TS (shared/content/src/tables.ts) and PHP (server/src/Generated/ContentTables.php) by server/schema/generate.ts (#477). */
export const LEGACY_CHARACTER_IDS: Record<string, { character: string; persona: string }> = {
	plf: { character: 'catpope', persona: 'human' },
	cat: { character: 'catpope', persona: 'cat' },
	'mountain-tiger': { character: 'catpope', persona: 'mountaintiger' }
};

/** The one locale order for content text (#474, #477): the asked locale › the document's source language › English › any. `asked`, `source` and `any` are steps, anything else a locale code. A text counts when it is a string with more than white space; `any` = the first such text in the map's own order. Generated into Rust, TS and PHP with the one pick function each by server/schema/generate.ts; locale-order.vectors.json is the cases every language runs. */
export const LOCALE_ORDER = ['asked', 'source', 'en', 'any'] as const;

/** Content text in the one locale order (LOCALE_ORDER): the first step with a text, undefined when none has one. */
export function pickContentText(text: Readonly<Record<string, unknown>> | null | undefined, asked: string, source?: string | null): string | undefined {
	if (!text) return undefined;
	const present = (value: unknown): value is string => typeof value === 'string' && value.trim() !== '';
	for (const step of LOCALE_ORDER as readonly string[]) {
		if (step === 'any') {
			for (const value of Object.values(text)) if (present(value)) return value;
			continue;
		}
		const locale = step === 'asked' ? asked : step === 'source' ? source : step;
		const value = locale ? (Object.hasOwn(text, locale) ? text[locale] : undefined) : undefined;
		if (present(value)) return value;
	}
	return undefined;
}
