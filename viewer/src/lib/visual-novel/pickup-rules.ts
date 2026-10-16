import { maxOf, type ItemDef, type RecurrenceInterval } from '@scutoons/content';

/**
 * Whether a placed pickup can be taken, and what a grant's answer means for it (agents/ITEMS.md "Placed items",
 * "Can't take it"; owner 2026-10-16: "I am not supposed to be picking up items that I already have or are limited").
 * Pure – slice-items.ts feeds it the catalog, the inventory and the reader's memory. Test: viewer/test/pickup-rules.test.ts.
 *
 * A placement's memory (`picked:<slice image>#<id>` in the game state; in memory for `visit` respawns):
 * - taken – the reader took it (once: `1`; respawning: the period) → it is gone from the slice;
 * - held – the server had granted this ref before (taken on another device, or before the save game was reset) →
 *   it stays on the slice as art: no glow, no nudge, a tap shakes and says so (once: `held`; respawning: `held@<period>`).
 */

/** why the reader can't take a placed item now (null = they can) – the notice's reason (items-host `refusedText`) */
export type PickupBlock = 'not_grantable' | 'already_taken' | 'max_reached' | 'taken';

export type PickState = 'taken' | 'held' | null;

type Stored = string | number | boolean | null | undefined;

/** the value remembered for a taken placement */
export function takenValue(interval: RecurrenceInterval | null, period: string): string | number {
	return interval === null ? 1 : period;
}

/** the value remembered for a placement the server had granted before (granted 0) */
export function heldValue(interval: RecurrenceInterval | null, period: string): string {
	return interval === null ? 'held' : `held@${period}`;
}

/** a remembered value read back: taken, held, or nothing (another period's value = nothing: it is back) */
export function pickState(stored: Stored, interval: RecurrenceInterval | null, period: string): PickState {
	if (stored === undefined || stored === null || stored === '' || stored === false || stored === 0) return null;
	if (stored === heldValue(interval, period)) return 'held';
	if (interval === null) return stored === 'held' ? 'held' : 'taken';
	return stored === period ? 'taken' : null;
}

/**
 * Can the reader take it? `carried` = their pieces outside homes (the server's count: placed furniture doesn't count),
 * `refused` = a refusal the server gave on this page view that won't change in it (the world limit).
 */
export function pickupBlock(o: { def: Pick<ItemDef, 'grantable' | 'max'> | null | undefined; carried: number; state: PickState; refused?: PickupBlock | null }): PickupBlock | null {
	if (!o.def || !o.def.grantable?.includes('story')) return 'not_grantable';
	if (o.state === 'held') return 'already_taken';
	if (o.refused) return o.refused;
	if (o.carried >= maxOf(o.def)) return 'max_reached';
	return null;
}

export interface PickupAnswer {
	/** what to remember for the placement */
	state: PickState;
	/** the reason it stayed (none = taken) */
	error?: string;
	/** remember for this page view: it can't be taken here again until the next one */
	block?: PickupBlock;
}

/** errors that stay true for the page view (the world limit, a catalog / content problem); the rest may pass (offline, rate limits) */
const LASTING: Record<string, PickupBlock> = { taken: 'taken', not_grantable: 'not_grantable', unknown_item: 'not_grantable', invalid_respawn: 'not_grantable' };

/** a grant's answer (`inventory.grant`; null = the item isn't in the catalog) → what happens to the placement */
export function pickupAnswer(result: { ok: boolean; error?: string; granted?: number } | null): PickupAnswer {
	if (!result) return { state: null, error: 'not_grantable', block: 'not_grantable' };
	if (!result.ok) {
		const error = result.error ?? 'failed';
		const block = LASTING[error] ?? (error.startsWith('respawn_') ? 'not_grantable' : undefined);
		return { state: null, error, ...(block ? { block } : {}) };
	}
	// the server had this ref already: nothing new came into the bag – it stays, as the reader's already
	if (result.granted === 0) return { state: 'held', error: 'already_taken' };
	return { state: 'taken' };
}
