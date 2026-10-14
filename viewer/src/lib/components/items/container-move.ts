// Moving an item into a container by hand (agents/research/SCORES.md §2): past a soft limit (the DIY song book) the
// reader is warned first, in-page; used by the item detail's buttons and the inventory's drag and drop.
import { confirmDialog } from '@scutoons/ui';
import type { Entry } from '@scutoons/content';
import type { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
import { m } from '$lib/paraglide/messages.js';

type Inventory = ReturnType<typeof getInventoryContext>;

/** null = done (or the reader said no), else the message to show */
export async function moveWithWarning(inventory: Inventory, entry: Entry, into: Entry | null): Promise<string | null> {
	let result = await inventory.move(entry, into);
	if (!result.ok && result.error === 'overfill_warning' && into) {
		const ok = await confirmDialog({
			title: m['items.overfill_title'](),
			message: m['items.overfill_warning']({ max: result.max ?? '', days: result.days ?? '' }),
			confirmLabel: m['items.overfill_anyway'](),
			cancelLabel: m['items.overfill_cancel']()
		});
		result = ok ? await inventory.move(entry, into, true) : { ok: true };
	}
	if (result.ok) return null;
	return result.error === 'container_full' ? m['items.container_full']({ max: result.max ?? '' }) : m['items.move_failed']();
}
