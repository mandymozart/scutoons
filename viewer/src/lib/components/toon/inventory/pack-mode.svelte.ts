// The pack mode and the container view's take-out (research/PACKING.md §2, owner 2026-10-16): "Pack" in an item's
// detail switches the inventory grid into a pick-the-target mode – containers that take the item now are lit, the rest
// dimmed with the reason; a tap packs all that fits, holding counts the amount (hold-count.ts). An open container's grid
// takes things out on a tap (Stardew's chest) – not furniture, which is filled and emptied at home (the home view).
import type { Entry, ItemDef } from '@scutoons/content';
import type { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
import { moveWithWarning } from '$lib/components/items/container-move.ts';
import { m } from '$lib/paraglide/messages.js';

type Inventory = ReturnType<typeof getInventoryContext>;

/** how the grid shows a tile in a mode: lit / dimmed / the item being packed, its accessible name, a reason, the hold limit */
export interface Tile {
	look: 'source' | 'lit' | 'dim' | null;
	label: string;
	reason?: string;
	/** the most a hold counts to (0 = no counting) */
	holdMax: number;
}

const same = (a: Entry | null | undefined, b: Entry | null | undefined) => !!a && !!b && String(a.id) === String(b.id);

export function createPackMode(inventory: Inventory, focusTile: (id: string) => void) {
	let packing = $state<Entry | null>(null);
	/** the visible line under the bar: done, a reason, an error */
	let note = $state('');
	/** the screen reader's polite line (entering the mode, counting) */
	let live = $state('');

	const targets = $derived(packing ? inventory.packTargets(packing) : []);

	function only(def: ItemDef | undefined): string {
		const formats = def?.holds?.formats ?? [];
		if (!formats.length) return m['items.pack_why_kind']();
		const word = { vinyl: m['items.pack_vinyl'], cd: m['items.pack_cd'], tape: m['items.pack_tape'] };
		return m['items.pack_only']({ what: formats.map((f) => word[f]()).join(', ') });
	}
	function reason(container: Entry, block: 'kind' | 'at_home' | 'full'): string {
		if (block === 'kind') return only(inventory.item(container.item_id));
		return block === 'at_home' ? m['items.pack_why_at_home']() : m['items.pack_why_full']();
	}
	const room = (container: Entry) => {
		const max = inventory.item(container.item_id)?.holds?.max;
		return max == null ? Infinity : Math.max(0, max - inventory.countIn(container));
	};

	/** a tile in the pack mode (null = not packing) */
	function packTile(entry: Entry): Tile | null {
		if (!packing) return null;
		const name = inventory.nameOf(entry);
		if (same(entry, packing)) return { look: 'source', label: name, holdMax: 0 };
		const target = targets.find((t) => same(t.container, entry));
		if (!target) return { look: 'dim', label: name, holdMax: 0 };
		if (target.block) {
			const why = reason(entry, target.block);
			return { look: 'dim', label: `${name} – ${why}`, reason: why, holdMax: 0 };
		}
		const max = inventory.item(entry.item_id)?.holds?.max;
		const label = max == null ? m['items.pack_target_open']({ name }) : m['items.pack_target']({ name, count: inventory.countIn(entry), max });
		return { look: 'lit', label, holdMax: packing.qty > 1 ? Math.min(packing.qty, room(entry)) : 0 };
	}

	/** a tile in an open container's grid: a tap takes it out (null = the normal tile – furniture is emptied at home) */
	function boxTile(entry: Entry, box: Entry): Tile | null {
		if (inventory.item(box.item_id)?.furniture) return null;
		return { look: null, label: `${inventory.nameOf(entry)} – ${m['items.container_take_out']({ name: inventory.nameOf(box) })}`, holdMax: entry.qty > 1 ? entry.qty : 0 };
	}

	async function packInto(container: Entry, qty?: number) {
		const entry = packing;
		if (!entry) return;
		const { error, moved } = await moveWithWarning(inventory, entry, container, qty);
		if (error) return void (note = error);
		if (!moved) return;
		const args = { name: inventory.nameOf(entry), container: inventory.nameOf(container), qty: moved };
		note = live = moved > 1 ? m['items.pack_done_qty'](args) : m['items.pack_done'](args);
		packing = null;
		focusTile(String(container.id));
	}

	async function takeOut(entry: Entry, qty?: number) {
		const { error, moved } = await moveWithWarning(inventory, entry, null, qty);
		if (error) return void (note = error);
		const args = { name: inventory.nameOf(entry), qty: moved };
		note = live = moved > 1 ? m['items.take_out_qty'](args) : m['items.take_out_done'](args);
	}

	return {
		get packing() {
			return packing;
		},
		get note() {
			return note;
		},
		get live() {
			return live;
		},
		set live(text: string) {
			live = text;
		},
		packTile,
		boxTile,
		/** "Pack" in the detail: the grid picks the target; focus goes to the first lit tile */
		start(entry: Entry) {
			packing = entry;
			note = '';
			const lit = inventory.packTargets(entry).filter((t) => !t.block);
			live = m['items.pack_live']({ count: lit.length });
			if (lit[0]) focusTile(String(lit[0].container.id));
		},
		cancel() {
			packing = null;
			note = '';
		},
		clearNote() {
			note = '';
		},
		/** a line for the status and the screen reader (stacking, research/PACKING.md §6) */
		say(text: string) {
			note = live = text;
		},
		/** a tap on a tile in the pack mode */
		tap(entry: Entry) {
			const tile = packTile(entry);
			if (!tile || tile.look === 'source') return;
			if (tile.look === 'lit') void packInto(entry);
			else note = tile.reason ?? m['items.pack_choose']();
		},
		/** a hold on a lit tile: pack this many */
		hold(entry: Entry, qty: number) {
			if (packTile(entry)?.look === 'lit') void packInto(entry, qty);
		},
		takeOut,
		/** desktop: dragged onto a container – the same move as a tap in the pack mode */
		drop(entry: Entry, container: Entry) {
			packing = entry;
			note = '';
			void packInto(container).finally(() => (packing = null));
		}
	};
}

export type PackMode = ReturnType<typeof createPackMode>;
