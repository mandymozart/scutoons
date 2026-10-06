import { effectiveRespawn, periodOf, type RecurrenceInterval, type Slice, type SliceItem, type VNEvent } from '@scutoons/content';
import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { EventProcessor, type GroupScript } from './event-processor.ts';
import { grantStoryItem } from './variable-actions.ts';

/**
 * Taps on items placed on a slice (`Slice.items`, agents/ITEMS.md "Placed items"). What a tap does is the creator's
 * choice per placement (`action`):
 * - `pickup`: the VN item bridge, like a choice's `add <item> <qty>` – the story grant (source story, a ref per place,
 *   so the catalog's rules apply: grantable, max count, the server's once-per-ref) and the game variable.
 *   Whether it reappears (respawn.ts): the item's rule (catalog `respawn`), the placement may override within its range
 *   or say `never`. Once (no respawn): the ref is the place; a taken item is remembered in the game state
 *   (`picked:<slice image>#<id>` = 1, synced with a member's progress) and is gone for good.
 *   Respawning: the grant sends the interval (+ the visit id); the server checks it against the item's range and adds
 *   the period (its time bucket / the visit) to the ref, so it grants once per period. Here the period (the same
 *   bucket from the reader's clock, or the visit) is remembered – in memory for a visit, in the game state otherwise.
 * - `events`: the placement's VN events, run like a click trigger's (`EventProcessor`), once per page view by default.
 */

/** the game variable that remembers a taken placement */
export function pickedKey(slice: Pick<Slice, 'image'>, item: Pick<SliceItem, 'id'>): string {
	return `picked:${slice.image}#${item.id}`;
}

/** A visit = one stay on a page (path): a new path or a reload starts a new one. */
let visit = { path: '', id: '' };
export function visitId(): string {
	const path = typeof location === 'undefined' ? '' : location.pathname;
	if (visit.path !== path || !visit.id) visit = { path, id: Math.random().toString(36).slice(2, 10) };
	return visit.id;
}

/** the interval this placement reappears with (null = once), from the catalog's rule and the placement's choice */
export function placementRespawn(item: SliceItem): RecurrenceInterval | null {
	const rule = getCurrentInventory()?.item(item.item)?.respawn;
	return effectiveRespawn(item.respawn, rule);
}

/** respawning placements taken on the current visit (`<period>:<slice image>#<id>`) */
const takenThisVisit = new Set<string>();

/** taken by this reader: for good (once) or in the current period */
export function isTaken(slice: Pick<Slice, 'image'>, item: SliceItem, game: GameContext | undefined): boolean {
	const interval = placementRespawn(item);
	const stored = game?.getVariable(pickedKey(slice, item));
	if (interval === null) return !!stored;
	const period = periodOf(interval, Date.now(), visitId());
	if (interval === 'visit') return takenThisVisit.has(`${period}:${slice.image}#${item.id}`);
	return stored === period;
}

function remember(slice: Pick<Slice, 'image'>, item: SliceItem, interval: RecurrenceInterval | null, game: GameContext) {
	if (interval === null) return game.setVariable(pickedKey(slice, item), 1);
	const period = periodOf(interval, Date.now(), visitId());
	if (interval === 'visit') takenThisVisit.add(`${period}:${slice.image}#${item.id}`);
	else game.setVariable(pickedKey(slice, item), period);
}

const pending = new Set<string>();

export interface PickupResult {
	/** the item is the reader's now (hide it) */
	ok: boolean;
	/** why not: max_reached, not_grantable, already_taken, rate_limited, failed … (the API's code where it gave one) */
	error?: string;
}

/** Takes a placed item. A refusal leaves it on the slice (the caller tells the reader why). */
export async function pickUpSliceItem(slice: Pick<Slice, 'image'>, item: SliceItem, game: GameContext): Promise<PickupResult> {
	const interval = placementRespawn(item);
	const place = `${slice.image}#${item.id}`;
	const key = interval === null ? place : `${place}@${periodOf(interval, Date.now(), visitId())}`;
	if (isTaken(slice, item, game) || pending.has(key)) return { ok: false };
	pending.add(key);
	try {
		const qty = Math.max(1, Math.floor(Number(item.qty) || 1));
		const respawn = interval === null ? undefined : { respawn: interval, ...(interval === 'visit' ? { visit: visitId() } : {}) };
		// guests have no server: their period is the local key (the grant ref isn't checked anywhere)
		const result = await grantStoryItem(item.item, qty, place, { quietWhenNone: true, respawn });
		if (!result) return { ok: false, error: 'not_grantable' }; // not in the catalog (yet)
		if (!result.ok) return { ok: false, error: result.error ?? 'failed' };
		if (result.granted === 0) {
			// the server had this ref already (taken on another device in this period): gone here too, nothing new
			remember(slice, item, interval, game);
			return { ok: false, error: 'already_taken' };
		}
		game.addVariable(item.item, result.granted ?? qty);
		remember(slice, item, interval, game);
		return { ok: true };
	} finally {
		pending.delete(key);
	}
}

/** Runs a placement's tap events; resolves to a URL when one of them jumps. */
export function runSliceItemEvents(item: SliceItem, game: GameContext, vn: VisualNovelContext, group?: GroupScript): Promise<string | null> {
	return new EventProcessor((item.events ?? []) as VNEvent[], game, vn, group, `item/${item.id}`).run();
}
