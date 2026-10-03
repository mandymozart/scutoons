import type { Slice, SliceItem, VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { EventProcessor } from './event-processor.ts';
import { grantStoryItem } from './variable-actions.ts';

/**
 * Taps on items placed on a slice (`Slice.items`, agents/ITEMS.md "Placed items"). What a tap does is the creator's
 * choice per placement (`action`):
 * - `pickup`: the VN item bridge, like a choice's `add <item> <qty>` – the story grant (source story, a ref per place,
 *   so the catalog's rules apply: grantable, max count, the server's once-per-ref) and the game variable. A taken item
 *   is remembered in the game state (`picked:<slice image>#<placement id>`, synced with a member's progress) and is
 *   gone from the slice for that reader;
 * - `events`: the placement's VN events, run like a click trigger's (`EventProcessor`), once per page view by default.
 */

/** the game variable that remembers a taken placement */
export function pickedKey(slice: Pick<Slice, 'image'>, item: Pick<SliceItem, 'id'>): string {
	return `picked:${slice.image}#${item.id}`;
}

const pending = new Set<string>();

/** Takes a placed item; true when it is the reader's now (and is hidden), false when nothing was taken. */
export async function pickUpSliceItem(slice: Pick<Slice, 'image'>, item: SliceItem, game: GameContext): Promise<boolean> {
	const key = pickedKey(slice, item);
	if (game.getVariable(key) || pending.has(key)) return false;
	pending.add(key);
	try {
		const qty = Math.max(1, Math.floor(Number(item.qty) || 1));
		const result = await grantStoryItem(item.item, qty, `${slice.image}#${item.id}`);
		// unknown item, not grantable from the story, max count reached, offline …: it stays on the slice
		if (!result?.ok) return false;
		game.addVariable(item.item, qty);
		game.setVariable(key, 1);
		return true;
	} finally {
		pending.delete(key);
	}
}

/** Runs a placement's tap events; resolves to a URL when one of them jumps. */
export function runSliceItemEvents(item: SliceItem, game: GameContext, vn: VisualNovelContext): Promise<string | null> {
	return new EventProcessor((item.events ?? []) as VNEvent[], game, vn).run();
}
