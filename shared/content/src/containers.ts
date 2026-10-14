// Containers, scores and ageing – items that hold items (agents/research/SCORES.md §2, §2a, §2b; FEATURES 10.97)
import type { ItemCategory, ItemDef, Recipe } from './items.ts';

/**
 * What a container takes (owner 2026-10-14: "a simple rule we can edit"): an item fits when its category is listed OR
 * its id is listed; `max` = how many entries fit (none = no limit). A container never holds a container.
 */
export interface ItemHolds {
	categories?: ItemCategory[];
	items?: string[];
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
	musicxml: string;
	pdf?: string | null;
}

const DAY_MS = 86_400_000;

export const isContainer = (item: Pick<ItemDef, 'holds'> | null | undefined): boolean => !!item?.holds;

/** Does `item` fit into `container` by its rule (not counting the capacity)? */
export function fitsIn(container: Pick<ItemDef, 'holds'> | null | undefined, item: Pick<ItemDef, 'id' | 'category' | 'holds'> | null | undefined): boolean {
	const holds = container?.holds;
	if (!holds || !item || item.holds) return false;
	return (holds.categories ?? []).includes(item.category) || (holds.items ?? []).includes(item.id);
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

/** The container / score / ageing / recipe rules of a catalog (inside `validateCatalog`; the server: CatalogValidator). */
export function validateContainers(catalog: { items: any[]; recipes?: any[] }, add: Add): void {
	const byId = new Map(catalog.items.filter(isObj).map((i) => [i.id, i]));
	catalog.items.forEach((item: any, i: number) => {
		if (!isObj(item)) return;
		const path = ['items', i];
		const h = item.holds;
		if (h != null) {
			if (!isObj(h)) add('error', [...path, 'holds'], `${item.id}: holds must be { categories?, items?, max?, overfill? }`);
			else {
				if (!(h.categories?.length || h.items?.length)) add('error', [...path, 'holds'], `${item.id}: holds needs categories or items`);
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
