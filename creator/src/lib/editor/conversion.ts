// The Convert page (agents/NEUTRAL.md "Migration"): a toon's chapters, scenes and seasons that are still stored per
// language (`chapter:<toon>:<n>:<l>` …) → one language-neutral document each, with the owner's session. The plan is
// @scutoons/content `planConversion` – the same as scripts/dev/neutralize.mjs on the local database: the published
// per-locale versions → a published neutral version, a draft after it where a language has newer drafts. Entities that
// already have a neutral document are left alone (idempotent; the creator may have edited it since). The per-locale
// documents are never changed (rollback = delete the neutral entity).
// Step 4: the per-language toon settings (`toon-settings:<toon>:<l>`) are folded into the toon entry `toon:<toon>`
// (`planToonConversion`) – new entry versions after the newest one; listed while the entry has no texts, or still stores
// the old name `i18n` (folded into `title`, decision 4).
import { contentIndex, getVersion, publishVersion, putDraft } from '$lib/api.ts';
import {
	entities,
	NEUTRAL_DOC_KINDS,
	parseEntityKey,
	planConversion,
	planToonConversion,
	type ConversionStep,
	type LocaleVersion,
	type NeutralDifference,
	type NeutralKind,
	type ToonConversionStep,
	type ToonLanguages
} from '@scutoons/content';

/** One entity to convert and what its conversion stores */
export interface EntityConversion {
	entity: string;
	kind: NeutralKind | 'toon';
	/** the per-locale documents it is made of (source first) */
	locales: string[];
	steps: (ConversionStep | (ToonConversionStep & { unchanged: false }))[];
	/** the toon entry: its newest version (the new versions are saved after it); neutral documents have none */
	base?: number;
}

/** The toon settings of a toon folded into its entry (step 4), or null when there is nothing to fold */
async function planToonEntry(toon: string, index: { key: string; latest: number; published: number | null }[], languages: ToonLanguages): Promise<EntityConversion | null> {
	const entity = entities.toon(toon);
	const entryRow = index.find((row) => row.key === entity);
	const settings = index.filter((row) => parseEntityKey(row.key)?.kind === 'toon-settings' && parseEntityKey(row.key)?.toon === toon);
	// no entry: nothing to fold into (without toon settings the plan only folds the old name `i18n` of an entry with texts)
	if (!entryRow) return null;
	const load = async (key: string, id: number): Promise<LocaleVersion> => ({ id, content: (await getVersion(key, id)).content });
	const entryNewest = await load(entity, entryRow.latest);
	const entryPublished = entryRow.published === null ? undefined : entryRow.published === entryRow.latest ? entryNewest : await load(entity, entryRow.published);
	const published: Record<string, LocaleVersion> = {};
	const newest: Record<string, LocaleVersion> = {};
	await Promise.all(
		settings.map(async (row) => {
			const locale = parseEntityKey(row.key)!.locale!;
			newest[locale] = await load(row.key, row.latest);
			if (row.published !== null) published[locale] = row.published === row.latest ? newest[locale] : await load(row.key, row.published);
		})
	);
	const steps = planToonConversion({ source: languages.source, languages: languages.languages, entryPublished, entryNewest, published, newest });
	if (!steps.length) return null;
	const known: readonly string[] = languages.languages;
	const order = (l: string) => (known.includes(l) ? known.indexOf(l) : known.length);
	const locales = Object.keys(newest).sort((a, b) => order(a) - order(b) || a.localeCompare(b));
	return { entity, kind: 'toon', locales, steps: steps.map((step) => ({ ...step, unchanged: false as const })), base: entryRow.latest };
}

/**
 * The dry run: the toon settings while the toon entry has no texts, every chapter / scene / season of the toon without
 * a neutral document – planned (nothing is stored)
 */
export async function planToon(toon: string, languages: ToonLanguages): Promise<EntityConversion[]> {
	const index = (await contentIndex(toon)).entities;
	const neutral = new Set<string>();
	const perLocale = new Map<string, { kind: NeutralKind; rows: { locale: string; key: string; latest: number; published: number | null }[] }>();
	for (const row of index) {
		const ref = parseEntityKey(row.key);
		if (!ref || ref.toon !== toon || !NEUTRAL_DOC_KINDS.includes(ref.kind)) continue;
		if (!ref.locale) {
			neutral.add(row.key);
			continue;
		}
		const entity = row.key.slice(0, -(ref.locale.length + 1));
		if (!perLocale.has(entity)) perLocale.set(entity, { kind: ref.kind as NeutralKind, rows: [] });
		perLocale.get(entity)!.rows.push({ locale: ref.locale, key: row.key, latest: row.latest, published: row.published });
	}
	const todo = [...perLocale].filter(([entity]) => !neutral.has(entity)).sort(([a], [b]) => a.localeCompare(b, 'en', { numeric: true }));
	const entry = await planToonEntry(toon, index, languages);
	const documents = await Promise.all(
		todo.map(async ([entity, { kind, rows }]) => {
			const published: Record<string, LocaleVersion> = {};
			const newest: Record<string, LocaleVersion> = {};
			await Promise.all(
				rows.map(async (row) => {
					const load = async (id: number): Promise<LocaleVersion> => ({ id, content: (await getVersion(row.key, id)).content });
					const latest = await load(row.latest);
					newest[row.locale] = latest;
					if (row.published !== null) published[row.locale] = row.published === row.latest ? latest : await load(row.published);
				})
			);
			const known: readonly string[] = languages.languages;
			const order = (l: string) => (known.includes(l) ? known.indexOf(l) : known.length);
			const locales = rows.map((r) => r.locale).sort((a, b) => order(a) - order(b) || a.localeCompare(b));
			return { entity, kind, locales, steps: planConversion({ kind, source: languages.source, languages: languages.languages, published, newest }) } as EntityConversion;
		})
	);
	return entry ? [entry, ...documents] : documents;
}

/** the neutral versions a plan stores */
export const versionCount = (plan: readonly EntityConversion[]) => plan.reduce((n, e) => n + e.steps.filter((s) => !s.unchanged).length, 0);

/**
 * Stores the plan: per entity the published neutral version (saved, then published) and the draft after it. Returns how
 * many versions were stored; `progress` after each entity. Stops at the first error (what is stored stays; a second run
 * skips the entities that have a neutral document now).
 */
export async function convertToon(plan: readonly EntityConversion[], progress?: (done: number) => void): Promise<number> {
	let stored = 0;
	let done = 0;
	for (const entry of plan) {
		let base: number | null = entry.base ?? null;
		for (const step of entry.steps) {
			if (step.unchanged) continue;
			const message =
				entry.kind === 'toon'
					? 'nameOnly' in step && step.nameOnly
						? `the old name (i18n) folded into the title (${step.status}) from ${step.versions}`
						: `toon settings folded into the entry (${step.status}) from ${step.versions}`
					: `neutral conversion (${step.status}) from the ${entry.locales.join(', ')} documents`;
			const saved = await putDraft(entry.entity, step.doc, base, message);
			base = saved.version.id;
			if (step.status === 'published') await publishVersion(entry.entity, base);
			stored++;
		}
		progress?.(++done);
	}
	return stored;
}

const short = (value: unknown) => {
	const text = JSON.stringify(value) ?? '–';
	return text.length > 80 ? `${text.slice(0, 79)}…` : text;
};

/** one reported difference as a line: kind · path: the values per locale */
export const differenceLine = (r: NeutralDifference) =>
	`${r.kind} · ${r.path || '–'}: ${Object.entries(r.values)
		.map(([l, v]) => `${l} ${short(v)}`)
		.join(' · ')}${r.missing ? ` (– ${r.missing.join(', ')})` : ''}`;
