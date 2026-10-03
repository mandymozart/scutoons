import type { Slice, SliceItem, SliceItemRespawn, VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { EventProcessor } from './event-processor.ts';
import { grantStoryItem } from './variable-actions.ts';

/**
 * Taps on items placed on a slice (`Slice.items`, agents/ITEMS.md "Placed items"). What a tap does is the creator's
 * choice per placement (`action`):
 * - `pickup`: the VN item bridge, like a choice's `add <item> <qty>` – the story grant (source story, a ref per place,
 *   so the catalog's rules apply: grantable, max count, the server's once-per-ref) and the game variable.
 *   No `respawn`: the ref is the place; a taken item is remembered in the game state (`picked:<slice image>#<id>` = 1,
 *   synced with a member's progress) and is gone for that reader for good.
 *   `respawn` (`visit` / `day` / `{ hours }`): the ref is the place + the current period (`@v:<visit id>`,
 *   `@d:<YYYY-MM-DD>`, `@h<n>:<period no.>`), so the server grants it once per period; it is back in the next period
 *   and can be taken again. Taken in a visit is remembered in memory, in a day / n hours in the game state (the period
 *   key). Farming is bounded by the server: the item's max count, 120 grants / hour, and a repeated ref grants nothing.
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

/** the reader's local calendar date, YYYY-MM-DD */
function today(now = new Date()): string {
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** the current period of a respawning item (null = once): part of the grant ref and of the "taken" memory */
export function periodOf(respawn: SliceItemRespawn | undefined, now = Date.now()): string | null {
	if (respawn === 'visit') return `v:${visitId()}`;
	if (respawn === 'day') return `d:${today(new Date(now))}`;
	const hours = typeof respawn === 'object' && respawn ? Math.floor(Number(respawn.hours)) : 0;
	if (hours >= 1) return `h${hours}:${Math.floor(now / (hours * 3_600_000))}`;
	return null;
}

/** respawning placements taken on the current visit (`<period>:<slice image>#<id>`) */
const takenThisVisit = new Set<string>();

/** taken by this reader: for good (no respawn) or in the current period */
export function isTaken(slice: Pick<Slice, 'image'>, item: SliceItem, game: GameContext | undefined): boolean {
	const period = periodOf(item.respawn);
	const stored = game?.getVariable(pickedKey(slice, item));
	if (period === null) return !!stored;
	if (item.respawn === 'visit') return takenThisVisit.has(`${period}:${slice.image}#${item.id}`);
	return stored === period;
}

function remember(slice: Pick<Slice, 'image'>, item: SliceItem, period: string | null, game: GameContext) {
	if (item.respawn === 'visit') takenThisVisit.add(`${period}:${slice.image}#${item.id}`);
	else game.setVariable(pickedKey(slice, item), period ?? 1);
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
	const period = periodOf(item.respawn);
	const place = `${slice.image}#${item.id}${period ? `@${period}` : ''}`;
	if (isTaken(slice, item, game) || pending.has(place)) return { ok: false };
	pending.add(place);
	try {
		const qty = Math.max(1, Math.floor(Number(item.qty) || 1));
		const result = await grantStoryItem(item.item, qty, place, { quietWhenNone: true });
		if (!result) return { ok: false, error: 'not_grantable' }; // not in the catalog (yet)
		if (!result.ok) return { ok: false, error: result.error ?? 'failed' };
		if (result.granted === 0) {
			// the server had this ref already (taken on another device in this period): gone here too, nothing new
			remember(slice, item, period, game);
			return { ok: false, error: 'already_taken' };
		}
		game.addVariable(item.item, result.granted ?? qty);
		remember(slice, item, period, game);
		return { ok: true };
	} finally {
		pending.delete(place);
	}
}

/** Runs a placement's tap events; resolves to a URL when one of them jumps. */
export function runSliceItemEvents(item: SliceItem, game: GameContext, vn: VisualNovelContext): Promise<string | null> {
	return new EventProcessor((item.events ?? []) as VNEvent[], game, vn).run();
}
