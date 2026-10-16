// Moving an item into a container by hand (agents/research/SCORES.md §2, research/PACKING.md): past a soft limit (the
// DIY song book) the reader is warned first, in-page; used by the pack mode, the container view, the item detail's
// take-out and the inventory's drag and drop.
import { confirmDialog } from '@scutoons/ui';
import type { Entry } from '@scutoons/content';
import type { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
import { m } from '$lib/paraglide/messages.js';

type Inventory = ReturnType<typeof getInventoryContext>;

/**
 * Moves `qty` of an entry (none = all that fits) into a container or out (`into` null). `error` null = done (or the
 * reader said no to the overfill warning – `moved` 0), else the message to show.
 */
export async function moveWithWarning(inventory: Inventory, entry: Entry, into: Entry | null, qty?: number): Promise<{ error: string | null; moved: number }> {
	let result = await inventory.move(entry, into, false, qty);
	if (!result.ok && result.error === 'overfill_warning' && into) {
		const ok = await confirmDialog({
			title: m['items.overfill_title'](),
			message: m['items.overfill_warning']({ max: result.max ?? '', days: result.days ?? '' }),
			confirmLabel: m['items.overfill_anyway'](),
			cancelLabel: m['items.overfill_cancel']()
		});
		result = ok ? await inventory.move(entry, into, true, qty) : { ok: true, moved: 0 };
	}
	if (result.ok) return { error: null, moved: result.moved ?? 0 };
	if (result.error === 'not_set_up') return { error: m['items.container_not_set_up'](), moved: 0 };
	return { error: result.error === 'container_full' ? m['items.container_full']({ max: result.max ?? '' }) : m['items.move_failed'](), moved: 0 };
}
