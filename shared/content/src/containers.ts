// Containers, scores and ageing – items that hold items (agents/research/SCORES.md §2, §2a, §2b; FEATURES 10.97)
import type { Entry, ItemCategory, ItemDef, Recipe } from './items.ts';
import { MEDIA_FORMATS, type MediaFormat } from './albums.ts';
import { collageProblems, type CollageSheet } from './collage.ts';

/**
 * What a container takes (owner 2026-10-14: "a simple rule we can edit"): an item fits when its category is listed OR
 * its id is listed OR its sound-carrier `format` is listed (owner 2026-10-16: CDs into a CD case / tower, LPs into a DJ
 * trolley / record bag / shelf, tapes into the cassette rack – research/PACKING.md §3); `max` = how many pieces fit
 * (none = no limit). A container never holds a container.
 */
export interface ItemHolds {
	categories?: ItemCategory[];
	items?: string[];
	formats?: MediaFormat[];
	max?: number;
	/**
	 * A soft limit (the DIY song book, owner 2026-10-14): past `max` up to `overfill.max` after a warning; overfilled
	 * for `days` it breaks into `breaks_into` (its contents fall out). Taking entries out back to `max` in time saves it.
	 */
	overfill?: { days: number; max: number; breaks_into: string };
}

/** An item that ages from its creation (the DIY song book): a tint from `tint_days`, spots from `spots_days`, fully aged at `full_days`. */
export interface ItemAges {
	tint_days: number;
	spots_days: number;
	full_days: number;
}

/** A score's files (one language per score – an edition per language, owner 2026-10-14): CDN paths, served signed. */
export interface ScoreFiles {
	language: string;
	/** the full score */
	musicxml: string;
	pdf?: string | null;
	/**
	 * Named parts of the same score (owner 2026-10-14: one item per song, "different parts" – no second item):
	 * `choir` = the choir arrangement. Switched on per reader later with live mode's performance groups; for now the
	 * reader sees `READER_PART` when the score has it (`readerPart`).
	 */
	parts?: Record<string, ScorePart> | null;
	/**
	 * collage pieces on the full score (collage.ts, agents/research/COLLAGE.md §4): edited in the creator's score stage,
	 * never in the public catalog – readers get the pieces with the signed file
	 */
	collage?: CollageSheet | null;
}

export interface ScorePart {
	musicxml: string;
	pdf?: string | null;
	/** collage pieces on this part (`ScoreFiles.collage`) */
	collage?: CollageSheet | null;
}

/** the part readers see for now (owner 2026-10-14: "you always choose the choir parts in the viewer") */
export const READER_PART = 'choir';

/** which part of a score a reader opens: the choir when there is one, else the full score (null) */
export function readerPart(score: Pick<ScoreFiles, 'parts'> | { parts?: Record<string, unknown> | null } | null | undefined): string | null {
	return score?.parts && READER_PART in score.parts ? READER_PART : null;
}

const DAY_MS = 86_400_000;

export const isContainer = (item: Pick<ItemDef, 'holds'> | null | undefined): boolean => !!item?.holds;

/**
 * Can this container entry be filled now? Furniture (a shelf, a CD tower) only where it is set up – today a home
 * (owner 2026-10-16: "furniture can only be stocked when setup in a place … in the inventory you can only carry it");
 * a container that is no furniture (a vinyl bag, a DJ trolley) is carried and filled anywhere.
 */
export function canFill(container: Pick<ItemDef, 'holds' | 'furniture'> | null | undefined, entry: { home?: unknown } | null | undefined): boolean {
	return !!container?.holds && (!container.furniture || !!entry?.home);
}

/** Does `item` fit into `container` by its rule (not counting the capacity)? */
export function fitsIn(container: Pick<ItemDef, 'holds'> | null | undefined, item: (Pick<ItemDef, 'id' | 'category' | 'holds'> & { format?: MediaFormat | null }) | null | undefined): boolean {
	const holds = container?.holds;
	if (!holds || !item || item.holds) return false;
	return (holds.categories ?? []).includes(item.category) || (holds.items ?? []).includes(item.id) || (!!item.format && (holds.formats ?? []).includes(item.format));
}

/**
 * Can two entries lie on one field (owner 2026-10-17: two cucumber crosses "auf einem Feld", like Stardew – put
 * together by hand: the combine mode, a drag)? The same item and variant at the same place (loose or in the same
 * container), neither a container, standing furniture or an ageing item, and no piece of its own (a code, an edition
 * number). Worn pieces are the caller's check. The server's Containers::stack is the same rule.
 */
export function canStack(a: Pick<Entry, 'id' | 'item_id' | 'variant' | 'code' | 'edition_number' | 'in' | 'home'>, b: typeof a, def: Pick<ItemDef, 'holds' | 'ages' | 'furniture'> | null | undefined): boolean {
	if (String(a.id) === String(b.id) || a.item_id !== b.item_id || (a.variant ?? null) !== (b.variant ?? null)) return false;
	if (!def || def.holds || def.ages) return false;
	if (a.code || b.code || a.edition_number != null || b.edition_number != null || a.home || b.home) return false;
	return String(a.in ?? '') === String(b.in ?? '');
}

/** How many more pieces go into a container holding `count` without overfilling it (Infinity = no limit). */
export function roomIn(container: Pick<ItemDef, 'holds'> | null | undefined, count: number): number {
	const max = container?.holds?.max;
	return max == null ? Infinity : Math.max(0, max - count);
}

/**
 * Why `item` can't go into this container entry now (research/PACKING.md §2a – the pack mode's dimmed targets):
 * `null` = it can; `kind` = the rule doesn't take it (another format / category); `at_home` = furniture, filled only at
 * home in the home view (owner 2026-10-16); `full` = no room left (an overfill container still takes it after a warning,
 * so it never counts as full before its ceiling). Order: the rule first, then where, then the room.
 */
export function packBlock(
	container: Pick<ItemDef, 'holds' | 'furniture'> | null | undefined,
	item: Parameters<typeof fitsIn>[1],
	count: number
): null | 'kind' | 'at_home' | 'full' {
	if (!container?.holds || !fitsIn(container, item)) return 'kind';
	if (container.furniture) return 'at_home';
	return moveInCheck(container, count) === 'full' ? 'full' : null;
}

/** The most a container can take: the overfill ceiling when it has one, else `max` (Infinity = no limit). */
export function hardCapacity(container: Pick<ItemDef, 'holds'> | null | undefined): number {
	const holds = container?.holds;
	return holds?.overfill?.max ?? holds?.max ?? Infinity;
}

/**
 * Moving one more entry into a container holding `count`: `ok`, `overfill` (allowed after a warning – it will break),
 * or `full` (refused).
 */
export function moveInCheck(container: Pick<ItemDef, 'holds'>, count: number): 'ok' | 'overfill' | 'full' {
	const holds = container.holds;
	if (!holds) return 'full';
	if (holds.max == null || count + 1 <= holds.max) return 'ok';
	return holds.overfill && count + 1 <= holds.overfill.max ? 'overfill' : 'full';
}

/** When an overfilled container breaks (ms since 1970), or null when it isn't overfilled / can't break. */
export function breaksAt(container: Pick<ItemDef, 'holds'>, count: number, overfilledAt: number | null | undefined): number | null {
	const holds = container.holds;
	if (!holds?.overfill || holds.max == null || count <= holds.max || overfilledAt == null) return null;
	return overfilledAt + holds.overfill.days * DAY_MS;
}

/** The look of an ageing item at `now`: tint and spots 0 … 1 (0 = new). */
export function ageLook(ages: ItemAges | null | undefined, createdAt: number, now: number): { tint: number; spots: number } {
	if (!ages) return { tint: 0, spots: 0 };
	const days = Math.max(0, (now - createdAt) / DAY_MS);
	const ramp = (from: number) => (days <= from ? 0 : Math.min(1, (days - from) / Math.max(1, ages.full_days - from)));
	return { tint: ramp(ages.tint_days), spots: ramp(ages.spots_days) };
}

type Add = (level: 'error' | 'warning', path: (string | number)[], message: string) => void;
const isObj = (v: unknown): v is Record<string, any> => typeof v === 'object' && v !== null && !Array.isArray(v);
const whole = (v: unknown, min: number) => typeof v === 'number' && Number.isInteger(v) && v >= min;

/** a score's / part's collage (collage.ts): the pieces on the score host, bound to the file they were placed on */
function checkCollage(collage: unknown, musicxml: unknown, path: (string | number)[], who: string, add: Add): void {
	if (collage == null) return;
	if (!isObj(collage)) return add('error', path, `${who}: collage must be { file, pieces }`);
	if (typeof collage.file !== 'string' || !/\.(musicxml|xml|mxl)$/i.test(collage.file)) add('error', [...path, 'file'], `${who}: collage.file must be the .musicxml / .mxl path the pieces were placed on`);
	else if (typeof musicxml === 'string' && collage.file !== musicxml) add('warning', [...path, 'file'], `${who}: the collage was made on another file – check the bars`);
	if (!Array.isArray(collage.pieces)) return add('error', [...path, 'pieces'], `${who}: collage.pieces must be a list`);
	collageProblems(collage.pieces, 'score', (level, at, message) => add(level, [...path, 'pieces', ...at], `${who}: ${message}`));
}

/** The container / score / ageing / recipe rules of a catalog (inside `validateCatalog`; the server: CatalogValidator). */
export function validateContainers(catalog: { items: any[]; recipes?: any[] }, add: Add): void {
	const byId = new Map(catalog.items.filter(isObj).map((i) => [i.id, i]));
	catalog.items.forEach((item: any, i: number) => {
		if (!isObj(item)) return;
		const path = ['items', i];
		const h = item.holds;
		if (h != null) {
			if (!isObj(h)) add('error', [...path, 'holds'], `${item.id}: holds must be { categories?, items?, formats?, max?, overfill? }`);
			else {
				if (!(h.categories?.length || h.items?.length || h.formats?.length)) add('error', [...path, 'holds'], `${item.id}: holds needs categories, items or formats`);
				if (h.formats != null && !(Array.isArray(h.formats) && h.formats.every((f: unknown) => (MEDIA_FORMATS as readonly unknown[]).includes(f)))) add('error', [...path, 'holds', 'formats'], `${item.id}: holds.formats must be a list of ${MEDIA_FORMATS.join(', ')}`);
				for (const id of h.items ?? []) {
					const target = byId.get(id);
					if (!target) add('warning', [...path, 'holds', 'items'], `${item.id}: holds an unknown item "${id}"`);
					else if (target.holds) add('error', [...path, 'holds', 'items'], `${item.id}: a container can't hold a container ("${id}")`);
				}
				if (h.max != null && !whole(h.max, 1)) add('error', [...path, 'holds', 'max'], `${item.id}: holds.max must be a whole number ≥ 1`);
				const o = h.overfill;
				if (o != null) {
					if (!isObj(o) || !whole(o.days, 1) || !whole(o.max, 1) || typeof o.breaks_into !== 'string') add('error', [...path, 'holds', 'overfill'], `${item.id}: overfill must be { days ≥ 1, max, breaks_into }`);
					else {
						if (h.max == null || o.max <= h.max) add('error', [...path, 'holds', 'overfill', 'max'], `${item.id}: overfill.max must be above holds.max`);
						if (!byId.has(o.breaks_into)) add('error', [...path, 'holds', 'overfill', 'breaks_into'], `${item.id}: breaks into an unknown item "${o.breaks_into}"`);
					}
				}
			}
		}
		const a = item.ages;
		if (a != null && !(isObj(a) && whole(a.tint_days, 0) && whole(a.spots_days, 0) && whole(a.full_days, 1) && a.tint_days < a.full_days && a.spots_days < a.full_days)) {
			add('error', [...path, 'ages'], `${item.id}: ages must be { tint_days, spots_days, full_days } with both starts before full_days`);
		}
		const s = item.score;
		if (s != null) {
			if (!isObj(s) || typeof s.language !== 'string' || !s.language || typeof s.musicxml !== 'string' || !/\.(musicxml|xml|mxl)$/i.test(s.musicxml)) {
				add('error', [...path, 'score'], `${item.id}: score must be { language, musicxml: a .musicxml / .mxl path, pdf? }`);
			} else if (s.pdf != null && !(typeof s.pdf === 'string' && /\.pdf$/i.test(s.pdf))) add('error', [...path, 'score', 'pdf'], `${item.id}: score.pdf must be a .pdf path`);
			if (isObj(s) && s.parts != null) {
				if (!isObj(s.parts)) add('error', [...path, 'score', 'parts'], `${item.id}: score.parts must be { <part>: { musicxml, pdf? } }`);
				else for (const [name, part] of Object.entries(s.parts as Record<string, any>)) {
					if (!/^[a-z0-9_]{1,40}$/.test(name)) add('error', [...path, 'score', 'parts', name], `${item.id}: part name "${name}" – lower-case letters, digits, _`);
					if (!isObj(part) || typeof part.musicxml !== 'string' || !/\.(musicxml|xml|mxl)$/i.test(part.musicxml)) add('error', [...path, 'score', 'parts', name], `${item.id}: part ${name} needs a .musicxml / .mxl path`);
					else if (part.pdf != null && !(typeof part.pdf === 'string' && /\.pdf$/i.test(part.pdf))) add('error', [...path, 'score', 'parts', name, 'pdf'], `${item.id}: part ${name}: pdf must be a .pdf path`);
					if (isObj(part)) checkCollage(part.collage, part.musicxml, [...path, 'score', 'parts', name, 'collage'], `${item.id} (${name})`, add);
				}
			}
			if (isObj(s)) checkCollage(s.collage, s.musicxml, [...path, 'score', 'collage'], item.id, add);
			if (item.category !== 'score') add('warning', [...path, 'category'], `${item.id}: an item with score files should have the category "score"`);
		}
	});
	(catalog.recipes ?? []).forEach((r: Recipe | any, i: number) => {
		if (!isObj(r)) return;
		const path = ['recipes', i];
		const inputs: any[] = Array.isArray(r.inputs) ? r.inputs : [];
		if (inputs.length && inputs.every((p) => p?.keep === true)) add('error', [...path, 'inputs'], `${r.id}: a recipe must use up at least one input`);
		const from = r.output?.variant_from;
		if (from != null) {
			const input = inputs.find((p) => p?.item === from);
			if (!input) add('error', [...path, 'output', 'variant_from'], `${r.id}: variant_from must name one of the inputs`);
			const outVariants: any[] = byId.get(r.output?.item)?.variants ?? [];
			const inVariants: any[] = byId.get(from)?.variants ?? [];
			const missing = inVariants.map((v) => v?.id).filter((v) => !outVariants.some((o) => o?.id === v));
			if (input && missing.length) add('warning', [...path, 'output', 'variant_from'], `${r.id}: ${r.output?.item} has no variant ${missing.join(', ')} of ${from}`);
		}
	});
}
