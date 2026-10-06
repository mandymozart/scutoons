// Per-locale documents → one language-neutral document (agents/NEUTRAL.md "Migration"): the conversion of the stored
// `chapter:<toon>:<n>:<l>`, `scene:<toon>:<chapter>:<name>:<l>`, `season:<toon>:<n>:<l>` documents into
// `chapter:<toon>:<n>` … (neutral.ts). Pure; used by the creator's Convert page and scripts/dev/neutralize.mjs.
//
// - The source locale's document is the structure. Every text leaf (neutral.ts header) becomes a locale map with the
//   value of each locale (blank ones too – they resolve to the source like missing ones); `release` and season
//   `sequence[].status` likewise. `language` and the chapter's deprecated `status` are dropped; `accepted_warnings` of
//   every locale are kept (union).
// - Items of arrays (groups, slices, text boxes, VN events, items, sequence) are matched by index. A locale whose array
//   has another length, or whose VN event at an index has another type, does not match: its texts below that point are
//   not taken (never guessed) and are listed (`text-lost`), the mismatch too (`structure`).
// - Every other field must be the same in every locale; where a locale differs the source wins (`differs`).
// - A locale without a text the source has (or with a blank one) resolves to the source's text (`text-missing`).
import { deepEqual, type Path } from './paths.ts';
import type { Chapter, Group, Season } from './content.ts';
import type { ChapterDoc, GroupDoc, NeutralKind, SeasonDoc } from './neutral.ts';
import { isLocaleValue } from './neutral.ts';

export type NeutralDifferenceKind = 'differs' | 'structure' | 'text-lost' | 'text-missing' | 'source-missing';

/** One thing the conversion could not keep (or a text that will show the source's) */
export interface NeutralDifference {
	kind: NeutralDifferenceKind;
	/** dotted path in the document, e.g. `groups.2.slices.0.image` ('' = the document) */
	path: string;
	/**
	 * differs: the value per locale (the source's is kept); structure: the array length (or VN event type) per locale;
	 * text-lost: the text of the locale that is not taken; text-missing: the locales' blank / missing value (the source's
	 * text is shown there); source-missing: the locales there are
	 */
	values: Record<string, unknown>;
	/** differs: the locales without the field */
	missing?: string[];
}

export interface NeutralizeResult<T> {
	doc: T;
	report: NeutralDifference[];
	/** the locales that were converted, the source (base) first */
	locales: string[];
	/** the locale whose document is the structure (the source, else the first there is) */
	base: string;
}

type Ctx = 'chapter' | 'season' | 'group' | 'slice' | 'dialog' | 'event' | 'item' | 'sequence' | 'params-showMessage' | 'params-addChoice';

const TEXT: Partial<Record<Ctx, readonly string[]>> = {
	chapter: ['title', 'description', 'release'],
	season: ['title', 'description', 'release'],
	dialog: ['text'],
	sequence: ['status'],
	'params-showMessage': ['text'],
	'params-addChoice': ['label']
};
const DROP: Partial<Record<Ctx, readonly string[]>> = { chapter: ['language', 'status'], season: ['language'] };
const CHILD: Partial<Record<Ctx, Record<string, Ctx>>> = {
	chapter: { groups: 'group' },
	group: { slices: 'slice', events: 'event' },
	slice: { dialog: 'dialog', events: 'event', items: 'item' },
	item: { events: 'event' },
	season: { sequence: 'sequence' }
};
const ACCEPTED = 'accepted_warnings';

type Obj = Record<string, unknown>;
const isObject = (value: unknown): value is Obj => value !== null && typeof value === 'object' && !Array.isArray(value);
const join = (path: string, key: string | number) => (path ? `${path}.${key}` : String(key));
const clone = <T>(value: T): T => (value === undefined ? value : (JSON.parse(JSON.stringify(value)) as T));

function childCtx(ctx: Ctx, node: Obj, key: string): Ctx | null {
	if (ctx === 'event' && key === 'params') {
		return node.type === 'showMessage' ? 'params-showMessage' : node.type === 'addChoice' ? 'params-addChoice' : null;
	}
	return CHILD[ctx]?.[key] ?? null;
}

/** Every text leaf below a node (path → value), by the same rules as the conversion */
function collectTexts(node: unknown, ctx: Ctx | null, path: string, out: [string, unknown][]) {
	if (ctx === null) return;
	if (Array.isArray(node)) {
		node.forEach((item, i) => collectTexts(item, ctx, join(path, i), out));
		return;
	}
	if (!isObject(node)) return;
	for (const [key, value] of Object.entries(node)) {
		if (TEXT[ctx]?.includes(key)) {
			if (isLocaleValue(value)) out.push([join(path, key), value]);
		} else {
			collectTexts(value, childCtx(ctx, node, key), join(path, key), out);
		}
	}
}

/** What a text leaf is (for messages): the chapter / season `title` …, a text box's `text`, a VN message `text`, a choice `label` */
export type TextLeafKind = 'title' | 'description' | 'release' | 'status' | 'dialog' | 'message' | 'choice';

/**
 * Every locale-map leaf of a neutral document (texts, `release`, season `sequence[].status`), by the same rules as the
 * conversion: `fn(path, value, kind)` with the path into the document (array indices as numbers) and the stored value
 * (a locale map – or an older plain value). Leaves that are missing are not visited.
 */
export function forEachText(kind: NeutralKind, doc: unknown, fn: (path: Path, value: unknown, leaf: TextLeafKind) => void) {
	const walk = (node: unknown, ctx: Ctx | null, path: Path) => {
		if (ctx === null) return;
		if (Array.isArray(node)) {
			node.forEach((item, i) => walk(item, ctx, [...path, i]));
			return;
		}
		if (!isObject(node)) return;
		for (const [key, value] of Object.entries(node)) {
			if (TEXT[ctx]?.includes(key)) {
				const leaf: TextLeafKind =
					ctx === 'dialog' ? 'dialog' : ctx === 'params-showMessage' ? 'message' : ctx === 'params-addChoice' ? 'choice' : (key as TextLeafKind);
				fn([...path, key], value, leaf);
			} else {
				walk(value, childCtx(ctx, node, key), [...path, key]);
			}
		}
	};
	walk(doc, kind === 'scene' ? 'group' : kind, []);
}

class Merger {
	readonly report: NeutralDifference[] = [];
	constructor(readonly base: string) {}

	private lost(locale: string, node: unknown, ctx: Ctx | null, path: string) {
		const texts: [string, unknown][] = [];
		collectTexts(node, ctx, path, texts);
		for (const [p, text] of texts) this.report.push({ kind: 'text-lost', path: p, values: { [locale]: text } });
	}

	/** a neutral (non-text) value: the base's; differing locales are reported */
	private plain(value: unknown, others: [string, unknown][], path: string): unknown {
		const differ = others.filter(([, v]) => !deepEqual(v, value));
		if (differ.length) this.report.push({ kind: 'differs', path, values: { [this.base]: value, ...Object.fromEntries(differ) } });
		return clone(value);
	}

	value(value: unknown, others: [string, unknown][], path: string, ctx: Ctx | null): unknown {
		if (ctx === null) {
			// a neutral object is compared field by field (finer paths in the report); anything else as a whole
			return isObject(value) && others.every(([, o]) => isObject(o)) ? this.object(value, others as [string, Obj][], path, null) : this.plain(value, others, path);
		}
		if (!Array.isArray(value) && !isObject(value)) return this.plain(value, others, path);
		if (Array.isArray(value)) {
			const aligned: [string, unknown[]][] = [];
			const counts: Record<string, unknown> = {};
			for (const [locale, other] of others) {
				if (Array.isArray(other) && other.length === value.length) {
					aligned.push([locale, other]);
				} else {
					counts[locale] = Array.isArray(other) ? other.length : other;
					this.lost(locale, other, ctx, path);
				}
			}
			if (Object.keys(counts).length) this.report.push({ kind: 'structure', path, values: { [this.base]: value.length, ...counts } });
			return value.map((item, i) => this.value(item, aligned.map(([l, o]) => [l, o[i]]), join(path, i), ctx));
		}
		const aligned: [string, Obj][] = [];
		for (const [locale, other] of others) {
			if (isObject(other) && (ctx !== 'event' || other.type === value.type)) {
				aligned.push([locale, other]);
			} else {
				this.report.push({ kind: 'structure', path, values: { [this.base]: value.type ?? 'object', [locale]: isObject(other) ? (other.type ?? 'object') : other } });
				this.lost(locale, other, ctx, path);
			}
		}
		return this.object(value, aligned, path, ctx);
	}

	private object(node: Obj, others: [string, Obj][], path: string, ctx: Ctx | null): Obj {
		const out: Obj = {};
		const keys = [...Object.keys(node)];
		for (const [, other] of others) for (const key of Object.keys(other)) if (!keys.includes(key)) keys.push(key);
		for (const key of keys) {
			if (ctx !== null && DROP[ctx]?.includes(key)) continue;
			const here = Object.hasOwn(node, key);
			const with_ = others.filter(([, o]) => Object.hasOwn(o, key));
			const at = join(path, key);
			if (key === ACCEPTED) {
				const all = [node[key], ...with_.map(([, o]) => o[key])].flatMap((list) => (Array.isArray(list) ? list : []));
				const union = [...new Set(all.filter((k) => typeof k === 'string'))];
				if (union.length) out[key] = union;
				continue;
			}
			if (ctx !== null && TEXT[ctx]?.includes(key)) {
				const map: Obj = {};
				if (here) map[this.base] = clone(node[key]);
				for (const [locale, other] of with_) map[locale] = clone(other[key]);
				const source = isLocaleValue(node[key]) ? node[key] : undefined;
				const blank = others.filter(([, o]) => !isLocaleValue(o[key])).map(([l, o]) => [l, Object.hasOwn(o, key) ? o[key] : null] as const);
				if (source !== undefined && blank.length) this.report.push({ kind: 'text-missing', path: at, values: Object.fromEntries(blank) });
				out[key] = map;
				continue;
			}
			const sub = ctx === null ? null : childCtx(ctx, node, key);
			if (!here) {
				// only other locales have it: the base's structure wins
				this.report.push({ kind: 'differs', path: at, values: Object.fromEntries(with_.map(([l, o]) => [l, o[key]])), missing: [this.base] });
				for (const [locale, other] of with_) this.lost(locale, other[key], sub, at);
				continue;
			}
			const without = others.filter(([, o]) => !Object.hasOwn(o, key)).map(([l]) => l);
			if (without.length) this.report.push({ kind: 'differs', path: at, values: { [this.base]: node[key] }, missing: without });
			out[key] = this.value(node[key], with_.map(([l, o]) => [l, o[key]]), at, sub);
		}
		return out;
	}
}

function neutralize<T>(kind: NeutralKind, docs: Record<string, unknown>, source: string): NeutralizeResult<T> {
	const present = Object.keys(docs).filter((l) => docs[l] !== undefined && docs[l] !== null);
	if (!present.length) throw new Error(`neutralize ${kind}: no document`);
	const base = present.includes(source) ? source : present[0];
	const merger = new Merger(base);
	if (base !== source) merger.report.push({ kind: 'source-missing', path: '', values: { [source]: null, ...Object.fromEntries(present.map((l) => [l, true])) } });
	const others = present.filter((l) => l !== base).map((l) => [l, docs[l]] as [string, unknown]);
	const ctx: Ctx = kind === 'scene' ? 'group' : kind;
	const doc = merger.value(docs[base], others, '', ctx) as T;
	return { doc, report: merger.report, locales: [base, ...others.map(([l]) => l)], base };
}

/** Per-locale chapter documents (`{ de: Chapter, en: Chapter }`, any order; the source first is best) → one neutral chapter */
export function neutralizeChapter(docs: Record<string, Chapter | undefined>, source: string): NeutralizeResult<ChapterDoc> {
	return neutralize('chapter', docs, source);
}

/** Per-locale scene documents (groups stored on their own) → one neutral scene */
export function neutralizeGroup(docs: Record<string, Group | undefined>, source: string): NeutralizeResult<GroupDoc> {
	return neutralize('scene', docs, source);
}

/** Per-locale season documents → one neutral season */
export function neutralizeSeason(docs: Record<string, Season | undefined>, source: string): NeutralizeResult<SeasonDoc> {
	return neutralize('season', docs, source);
}

/** neutralizeChapter / neutralizeGroup / neutralizeSeason by kind */
export function neutralizeDocs(kind: NeutralKind, docs: Record<string, unknown>, source: string): NeutralizeResult<ChapterDoc | GroupDoc | SeasonDoc> {
	return neutralize(kind, docs, source);
}

// ── Conversion plan (the creator's Convert page and scripts/dev/neutralize.mjs) ──────────────────────────────────

/** One stored per-locale version */
export interface LocaleVersion {
	id: number;
	content: unknown;
}

export interface ConversionInput {
	kind: NeutralKind;
	/** the toon's source language */
	source: string;
	/** the toon's languages, source first (the order of the locale maps) */
	languages: readonly string[];
	/** the per-locale published versions by locale */
	published: Record<string, LocaleVersion>;
	/** the per-locale newest versions by locale */
	newest: Record<string, LocaleVersion>;
	/** the neutral entity's published / newest content when it has versions (unchanged steps are marked) */
	neutralPublished?: unknown;
	neutralNewest?: unknown;
}

/** One neutral version to store: published (from the published per-locale versions) or a draft after it */
export interface ConversionStep {
	status: 'published' | 'draft';
	doc: ChapterDoc | GroupDoc | SeasonDoc;
	report: NeutralDifference[];
	/** the converted versions, e.g. `de#12 en#13` (the version message) */
	versions: string;
	/** the neutral entity already has this content (nothing to store) */
	unchanged: boolean;
}

/** the versions in the toon's language order (source first, then the rest, then locales the toon doesn't list) */
function ordered(byLocale: Record<string, LocaleVersion>, languages: readonly string[]): [string, LocaleVersion][] {
	const locales = [...languages.filter((l) => byLocale[l]), ...Object.keys(byLocale).filter((l) => !languages.includes(l)).sort()];
	return locales.map((l) => [l, byLocale[l]]);
}

/**
 * What the conversion of one entity stores (agents/NEUTRAL.md decision 6): the published per-locale versions → a
 * published neutral version; when a locale has a newer draft (or the entity was never published) the newest versions →
 * a draft neutral version after it (creators' draft mode reads the newest). Steps whose content the neutral entity
 * already has are `unchanged`.
 */
export function planConversion(input: ConversionInput): ConversionStep[] {
	const variants: ['published' | 'draft', [string, LocaleVersion][]][] = [];
	if (Object.keys(input.published).length) variants.push(['published', ordered(input.published, input.languages)]);
	if (Object.entries(input.newest).some(([l, v]) => v.id !== input.published[l]?.id)) variants.push(['draft', ordered(input.newest, input.languages)]);
	let published = input.neutralPublished === undefined ? undefined : JSON.stringify(input.neutralPublished);
	let newest = input.neutralNewest === undefined ? undefined : JSON.stringify(input.neutralNewest);
	const steps: ConversionStep[] = [];
	for (const [status, versions] of variants) {
		const result = neutralize<ChapterDoc | GroupDoc | SeasonDoc>(input.kind, Object.fromEntries(versions.map(([l, v]) => [l, v.content])), input.source);
		const json = JSON.stringify(result.doc);
		const unchanged = (status === 'published' ? published : newest) === json;
		steps.push({ status, doc: result.doc, report: result.report, versions: versions.map(([l, v]) => `${l}#${v.id}`).join(' '), unchanged });
		if (!unchanged) {
			newest = json;
			if (status === 'published') published = json;
		}
	}
	return steps;
}

/**
 * The differences a reader would see (the creator's conversion notice): texts that are not taken (`text-lost`), and
 * neutral fields that differ where the reader shows them (not the deprecated / invisible ones: `interaction`, season
 * `credits`, `accepted_warnings`). `text-missing` (shows the source's text) and `structure` are listed by the caller.
 */
export function visibleDifferences(report: readonly NeutralDifference[]): NeutralDifference[] {
	const hidden = /(^|\.)(interaction|credits)(\.|$)/;
	return report.filter((r) => r.kind === 'text-lost' || r.kind === 'text-missing' || r.kind === 'structure' || (r.kind === 'differs' && !hidden.test(r.path)));
}
