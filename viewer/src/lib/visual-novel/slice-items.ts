import { effectiveRespawn, periodOf, type RecurrenceInterval, type SliceItem, type VNEvent } from '@scutoons/content';
import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { EventProcessor, type GroupScript } from './event-processor.ts';
import { giveStoryItem } from './variable-actions.ts';
import { pageView } from './page-view.ts';
import { heldValue, pickState, pickupAnswer, pickupBlock, takenValue, type PickState, type PickupBlock } from './pickup-rules.ts';

/**
 * Taps on items placed on a slice (`Slice.items`, agents/ITEMS.md "Placed items"). What a tap does is the creator's
 * choice per placement (`action`):
 * - `pickup`: like a choice's `give <item> <qty>` (BACKLOG 10.38: no story variable any more) – the story grant (source story, a ref per place,
 *   so the catalog's rules apply: grantable, max count, the server's once-per-ref) and the game variable.
 *   Whether it reappears (respawn.ts): the item's rule (catalog `respawn`), the placement may override within its range
 *   or say `never`. Once (no respawn): the ref is the place; a taken item is remembered in the game state
 *   (`picked:<placement id>` = 1, synced with a member's progress) and is gone for good. The placement id alone (unique:
 *   `<item>-<7 random>`, fresh on every paste) – so a slice's picture may be renamed or re-cut (2026-10-19, before:
 *   `picked:<slice image>#<id>`; the migration 2026-10-19-pickup-keys and save-game.ts `pickedKeyOf` renamed them).
 *   Respawning: the grant sends the interval (+ the visit id); the server checks it against the item's range and adds
 *   the period (its time bucket / the visit) to the ref, so it grants once per period. Here the period (the same
 *   bucket from the reader's clock, or the visit) is remembered – in memory for a visit, in the game state otherwise.
 *   The intervals and periods are the shared recurrence model (@scutoons/content recurrence.ts – the events' too).
 *   Can't take it (pickup-rules.ts – the limit reached, not grantable, the server had this ref already, the world
 *   limit): it stays on the slice, without the tap hint and the nudge; a tap shakes it and says why.
 * - `events`: the placement's VN events, run like a click trigger's (`EventProcessor`); how often: the placement's
 *   `repeat` (repeat.ts – every tap by default).
 */

/** the game variable that remembers a taken placement */
export function pickedKey(item: Pick<SliceItem, 'id'>): string {
	return `picked:${item.id}`;
}

/** A visit = the page view (page-view.ts: path + `?label=`; a reload starts a new one) – the same as the events' */
const visitId = () => pageView().id;

/** the interval this placement reappears with (null = once), from the catalog's rule and the placement's choice */
export function placementRespawn(item: SliceItem): RecurrenceInterval | null {
	const rule = getCurrentInventory()?.item(item.item)?.respawn;
	return effectiveRespawn(item.respawn, rule);
}

/** `visit` respawns: the placement's memory on the current visit (`<period>:<placement id>` → taken / held) */
const thisVisit = new Map<string, string | number>();
/** refusals that last for the page view (the world limit …): `<page view>:<placement id>` → why */
const refusedThisView = new Map<string, PickupBlock>();

/** the placement's memory: taken (gone), held (the server had granted it before: it stays, untakeable) or nothing */
function stateOf(item: SliceItem, game: GameContext | undefined): PickState {
	const interval = placementRespawn(item);
	const period = interval === null ? '' : periodOf(interval, Date.now(), visitId());
	const stored = interval === 'visit' ? thisVisit.get(`${period}:${item.id}`) : game?.getVariable(pickedKey(item));
	return pickState(stored as string | number | undefined, interval, period);
}

/** taken by this reader: for good (once) or in the current period – gone from the slice */
export function isTaken(item: SliceItem, game: GameContext | undefined): boolean {
	return stateOf(item, game) === 'taken';
}

/**
 * Why this reader can't take a placed pickup now (pickup-rules.ts), null = they can. A blocked one stays on the slice
 * as art: no glow, no nudge; a tap shakes and says why (owner 2026-10-16).
 */
export function pickupBlockOf(item: SliceItem, game: GameContext | undefined): PickupBlock | null {
	const inventory = getCurrentInventory();
	if (!inventory) return null; // no catalog yet: nothing is drawn anyway
	return pickupBlock({
		def: inventory.item(item.item),
		carried: inventory.carried(item.item),
		state: stateOf(item, game),
		refused: refusedThisView.get(`${visitId()}:${item.id}`)
	});
}

function remember(item: SliceItem, interval: RecurrenceInterval | null, state: 'taken' | 'held', game: GameContext) {
	const period = interval === null ? '' : periodOf(interval, Date.now(), visitId());
	const value = state === 'taken' ? takenValue(interval, period) : heldValue(interval, period);
	if (interval === 'visit') thisVisit.set(`${period}:${item.id}`, value);
	else game.setVariable(pickedKey(item), value);
}

/** slices whose pickup was nudged on this page view (agents/FX.md: once per visit): `<page view>:<slice image>` */
const nudged = new Set<string>();

/** true the first time on this page view – the caller nudges */
export function markNudged(place: string): boolean {
	const key = `${visitId()}:${place}`;
	if (nudged.has(key)) return false;
	nudged.add(key);
	return true;
}

const pending = new Set<string>();

export interface PickupResult {
	/** the item is the reader's now (hide it) */
	ok: boolean;
	/** why not: max_reached, not_grantable, already_taken, rate_limited, failed … (the API's code where it gave one) */
	error?: string;
}

/** Takes a placed item. A refusal leaves it on the slice (the caller shakes it and tells the reader why). */
export async function pickUpSliceItem(item: SliceItem, game: GameContext): Promise<PickupResult> {
	const interval = placementRespawn(item);
	const place = item.id;
	const key = interval === null ? place : `${place}@${periodOf(interval, Date.now(), visitId())}`;
	if (isTaken(item, game) || pending.has(key)) return { ok: false };
	const blocked = pickupBlockOf(item, game);
	if (blocked) return { ok: false, error: blocked };
	pending.add(key);
	try {
		const qty = Math.max(1, Math.floor(Number(item.qty) || 1));
		const respawn = interval === null ? undefined : { respawn: interval, ...(interval === 'visit' ? { visit: visitId() } : {}) };
		// guests have no server: their period is the local key (the grant ref isn't checked anywhere)
		const answer = pickupAnswer(await giveStoryItem(item.item, qty, place, { quietWhenNone: true, respawn, variant: item.variant }));
		// granted 0 = the server had this ref (taken on another device / before a reset): it stays, as the reader's already
		if (answer.state) remember(item, interval, answer.state, game);
		if (answer.block) refusedThisView.set(`${visitId()}:${place}`, answer.block);
		return answer.state === 'taken' ? { ok: true } : { ok: false, error: answer.error };
	} finally {
		pending.delete(key);
	}
}

/** Runs a placement's tap events; resolves to a URL when one of them jumps. */
export function runSliceItemEvents(item: SliceItem, game: GameContext, vn: VisualNovelContext, group?: GroupScript): Promise<string | null> {
	return new EventProcessor((item.events ?? []) as VNEvent[], game, vn, group, `item/${item.id}`).run();
}
