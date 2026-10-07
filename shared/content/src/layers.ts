// A slice's ONE layer stack (owner 2026-10-07, BACKLOG 10.43): its text boxes (`dialog[]`) and placed items (`items[]`)
// are drawn in one order, both in the creator's canvas and in the reader. Stored as a number `layer` on each entry
// (higher = on top) – the arrays keep their order and their ids (showTextBox events, pickups and problems point into
// them). No `layer` anywhere = the order from before: items under text boxes, each in array order.
//
// The paint order sorts by `layer`; an entry without one (a box added after a reorder, a pasted one) counts as above
// every numbered one, in the old order among themselves. The creator renumbers every entry 0…n-1 when the author
// reorders (`setSliceLayers`).

export type SliceLayerKind = 'dialog' | 'item';

/** one entry of the stack: a text box (`dialog[index]`) or a placed item (`items[index]`) */
export interface SliceLayerRef {
	kind: SliceLayerKind;
	index: number;
}

interface Layered {
	layer?: unknown;
}

interface LayeredSlice {
	dialog?: readonly (Layered | null | undefined)[] | null;
	items?: readonly (Layered | null | undefined)[] | null;
}

const layerOf = (entry: Layered | null | undefined): number => {
	const layer = entry?.layer;
	return typeof layer === 'number' && Number.isFinite(layer) ? layer : Infinity;
};

/** the slice's text boxes and placed items from the back to the front (the paint order) */
export function sliceLayers(slice: LayeredSlice | null | undefined): SliceLayerRef[] {
	const items = Array.isArray(slice?.items) ? slice.items : [];
	const dialogs = Array.isArray(slice?.dialog) ? slice.dialog : [];
	// the old order (items, then text boxes) is the tie-break: `pos`
	const all = [
		...items.map((entry, index) => ({ kind: 'item' as const, index, layer: layerOf(entry) })),
		...dialogs.map((entry, index) => ({ kind: 'dialog' as const, index, layer: layerOf(entry) }))
	].map((entry, pos) => ({ ...entry, pos }));
	all.sort((a, b) => (a.layer === b.layer ? 0 : a.layer < b.layer ? -1 : 1) || a.pos - b.pos);
	return all.map(({ kind, index }) => ({ kind, index }));
}

/**
 * Writes `layer` 0…n-1 on every text box and placed item of the slice in `order` (back to front, every entry once – as
 * `sliceLayers` answers it, reordered). Entries `order` leaves out keep what they have.
 */
export function setSliceLayers(slice: { dialog?: Layered[]; items?: Layered[] } | null | undefined, order: readonly SliceLayerRef[]): void {
	if (!slice) return;
	order.forEach((ref, layer) => {
		const entry = (ref.kind === 'dialog' ? slice.dialog : slice.items)?.[ref.index];
		if (entry && typeof entry === 'object') entry.layer = layer;
	});
}
