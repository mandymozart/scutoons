// A slice's ONE layer stack (owner 2026-10-07, BACKLOG 10.43 + 10.54): what lies on the slice and takes the pointer is
// drawn and hit in one order, both in the creator's canvas and in the reader:
//   - text boxes (`dialog[]`) and placed items (`items[]`) – 10.43;
//   - areas (10.54): slice events started by click / hover inside a rect (`events[]` with `trigger.rect` – the events
//     sharing one trigger are ONE area, the reader runs them as one script) and samples with a click / hover area
//     (`audio.audio.samples[]` with `trigger.rect`). Whole-slice events, other starts (in view) and samples without an
//     area are not part of it.
// Stored as a number `layer` on each entry (higher = in front: drawn on top, gets the tap / hover first) – the arrays
// keep their order and their ids (showTextBox events, pickups, event ids and problems point into them). An area's
// events all carry the same number (the lowest one counts if they differ).
//
// No `layer` anywhere = the order from before (the reader's z-order until 10.54): sample areas, items, text boxes, event
// areas, each in array order; the reader then keeps its old hit testing too (every area under the pointer reacts –
// `hasSliceLayers`). An entry without `layer` (a box added after a reorder, a pasted one) counts as above every numbered
// one, in that old order among themselves. The creator renumbers every entry 0…n-1 when the author reorders
// (`setSliceLayers`).
//
// Collage pieces (`pieces[]`, collage.ts – agents/research/COLLAGE.md §3) are listed too, always at the back in their
// array order: art on the picture, under every text box, item and area. They carry no `layer` (`setSliceLayers` skips
// them) and never switch the reader's hit testing (`hasSliceLayers` ignores them); they never take the pointer.

import { Trigger } from './enums.ts';
import { triggerKey } from './reach.ts';

export type SliceLayerKind = 'dialog' | 'item' | 'event' | 'sample' | 'piece';

/**
 * one entry of the stack: a text box (`dialog[index]`), a placed item (`items[index]`), a sample area
 * (`audio.audio.samples[index]`), an event area (`events[index]` = its first event; `events` = all of its events) or a
 * collage piece (`pieces[index]`, always at the back)
 */
export interface SliceLayerRef {
	kind: SliceLayerKind;
	index: number;
	/** an event area: the indices of every event sharing its trigger (in `events[]` order; `index` is the first) */
	events?: number[];
}

interface Layered {
	layer?: unknown;
	trigger?: unknown;
}

interface LayeredSlice {
	dialog?: readonly (Layered | null | undefined)[] | null;
	items?: readonly (Layered | null | undefined)[] | null;
	pieces?: readonly unknown[] | null;
	events?: readonly (Layered | null | undefined)[] | null;
	audio?: { audio?: { samples?: readonly (Layered | null | undefined)[] | null } | null } | null;
}

const layerOf = (entry: Layered | null | undefined): number => {
	const layer = entry?.layer;
	return typeof layer === 'number' && Number.isFinite(layer) ? layer : Infinity;
};

const list = <T>(value: readonly T[] | null | undefined): readonly T[] => (Array.isArray(value) ? value : []);

/** a click / hover trigger with an area on the slice – the triggers that join the layer stack */
export function isAreaTrigger(trigger: unknown): boolean {
	const t = trigger as { type?: unknown; rect?: unknown } | null | undefined;
	return (t?.type === Trigger.CLICK || t?.type === Trigger.HOVER) && !!t.rect && typeof t.rect === 'object';
}

/** the slice's event areas: the click / hover events with a rect, those sharing a trigger (`triggerKey`) as one */
export function sliceEventAreas(slice: LayeredSlice | null | undefined): number[][] {
	const areas = new Map<string, number[]>();
	list(slice?.events).forEach((event, index) => {
		if (!isAreaTrigger(event?.trigger)) return;
		const key = triggerKey(event!.trigger);
		areas.set(key, [...(areas.get(key) ?? []), index]);
	});
	return [...areas.values()];
}

/**
 * the slice's stack entries in the old order (sample areas, items, text boxes, event areas) with their stored layers;
 * the pieces before them at -Infinity (the sort's tie-break keeps their array order)
 */
function entries(slice: LayeredSlice | null | undefined) {
	const events = list(slice?.events);
	return [
		...list(slice?.pieces).map((_, index) => ({ ref: { kind: 'piece' as const, index }, layer: -Infinity })),
		...list(slice?.audio?.audio?.samples).flatMap((entry, index) =>
			isAreaTrigger(entry?.trigger) ? [{ ref: { kind: 'sample' as const, index }, layer: layerOf(entry) }] : []
		),
		...list(slice?.items).map((entry, index) => ({ ref: { kind: 'item' as const, index }, layer: layerOf(entry) })),
		...list(slice?.dialog).map((entry, index) => ({ ref: { kind: 'dialog' as const, index }, layer: layerOf(entry) })),
		...sliceEventAreas(slice).map((indices) => ({
			ref: { kind: 'event' as const, index: indices[0], events: indices },
			layer: Math.min(...indices.map((i) => layerOf(events[i])))
		}))
	];
}

/** the slice's stack from the back to the front (the paint order; the last one gets the pointer first) */
export function sliceLayers(slice: LayeredSlice | null | undefined): SliceLayerRef[] {
	// the old order is the tie-break: `pos`
	const all = entries(slice).map((entry, pos) => ({ ...entry, pos }));
	all.sort((a, b) => (a.layer === b.layer ? 0 : a.layer < b.layer ? -1 : 1) || a.pos - b.pos);
	return all.map(({ ref }) => ref);
}

/**
 * Whether the author has ordered the slice's stack (any entry has a `layer`): the reader then hit-tests by it (the
 * front-most area / item under the pointer reacts, nothing below it); none = the old way (every area under the pointer).
 */
export function hasSliceLayers(slice: LayeredSlice | null | undefined): boolean {
	return entries(slice).some((entry) => entry.ref.kind !== 'piece' && entry.layer !== Infinity);
}

/** the entries a ref stands for (an event area: all of its events) */
function refEntries(slice: LayeredSlice, ref: SliceLayerRef): unknown[] {
	switch (ref.kind) {
		case 'dialog':
			return [list(slice.dialog)[ref.index]];
		case 'item':
			return [list(slice.items)[ref.index]];
		case 'sample':
			return [list(slice.audio?.audio?.samples)[ref.index]];
		case 'event':
			return (ref.events ?? [ref.index]).map((i) => list(slice.events)[i]);
		case 'piece':
			return [list(slice.pieces)[ref.index]];
	}
}

/**
 * Writes `layer` 0…n-1 on every entry of the slice's stack in `order` (back to front, every entry once – as
 * `sliceLayers` answers it, reordered); an event area's events all get its number. Entries `order` leaves out keep
 * what they have. Pieces are skipped (their order is `pieces[]`'s): the rest is numbered 0…n-1.
 */
export function setSliceLayers(slice: LayeredSlice | null | undefined, order: readonly SliceLayerRef[]): void {
	if (!slice) return;
	order.filter((ref) => ref.kind !== 'piece').forEach((ref, layer) => {
		for (const entry of refEntries(slice, ref)) if (entry && typeof entry === 'object') (entry as Layered).layer = layer;
	});
}
