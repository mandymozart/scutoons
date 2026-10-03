import type { Slice, SliceItem, VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { EventProcessor } from './event-processor.ts';
import { grantStoryItem } from './variable-actions.ts';

/**
 * Taps on items placed on a slice (`Slice.items`, agents/ITEMS.md "Placed items"). What a tap does is the creator's
 * choice per placement (`action`):
 * - `pickup`: the VN item bridge, like a choice's `add <item> <qty>` – the story grant (source story, a ref per place,
 *   so the catalog's rules apply: grantable, max count, the server's once-per-ref) and the game variable.
 *   `pickupMode` `once` (default): the ref is the place; a taken item is remembered in the game state
 *   (`picked:<slice image>#<placement id>`, synced with a member's progress) and is gone for that reader for good.
 *   `respawn`: the ref is the place + this visit, so the server grants it once per visit; it is gone until the reader
 *   comes to the page again (another page in between, or a reload) and can then be taken again. Farming is bounded by
 *   the server: the item's max count, 120 grants / hour, and a repeated ref within one visit grants nothing.
 * - `events`: the placement's VN events, run like a click trigger's (`EventProcessor`), once per page view by default.
 */

/** the game variable that remembers a taken placement (`once`) */
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

/** respawning placements taken on the current visit (`<visit>:<slice image>#<id>`) */
const takenThisVisit = new Set<string>();
export function respawnKey(slice: Pick<Slice, 'image'>, item: Pick<SliceItem, 'id'>): string {
	return `${visitId()}:${slice.image}#${item.id}`;
}

/** taken by this reader: for good (`once`) or on this visit (`respawn`) */
export function isTaken(slice: Pick<Slice, 'image'>, item: SliceItem, game: GameContext | undefined): boolean {
	if (item.pickupMode === 'respawn') return takenThisVisit.has(respawnKey(slice, item));
	return !!game?.getVariable(pickedKey(slice, item));
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
	const respawn = item.pickupMode === 'respawn';
	const key = respawn ? respawnKey(slice, item) : pickedKey(slice, item);
	if (isTaken(slice, item, game) || pending.has(key)) return { ok: false };
	pending.add(key);
	try {
		const qty = Math.max(1, Math.floor(Number(item.qty) || 1));
		const place = respawn ? `${slice.image}#${item.id}@${visitId()}` : `${slice.image}#${item.id}`;
		const result = await grantStoryItem(item.item, qty, place, { quietWhenNone: true });
		if (!result) return { ok: false, error: 'not_grantable' }; // not in the catalog (yet)
		if (!result.ok) return { ok: false, error: result.error ?? 'failed' };
		if (result.granted === 0) {
			// the server had this ref already (taken on another device): gone here too, nothing new
			if (respawn) takenThisVisit.add(key);
			else game.setVariable(key, 1);
			return { ok: false, error: 'already_taken' };
		}
		game.addVariable(item.item, result.granted ?? qty);
		if (respawn) takenThisVisit.add(key);
		else game.setVariable(key, 1);
		return { ok: true };
	} finally {
		pending.delete(key);
	}
}

/** Runs a placement's tap events; resolves to a URL when one of them jumps. */
export function runSliceItemEvents(item: SliceItem, game: GameContext, vn: VisualNovelContext): Promise<string | null> {
	return new EventProcessor((item.events ?? []) as VNEvent[], game, vn).run();
}
