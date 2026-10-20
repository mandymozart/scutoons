// The group's ONE layer list (owner 2026-10-20, agents/DECISIONS.md; research/TEXT-ACROSS-SLICES.md §4.1 E2): every
// overlay of the group, front first, flat – the paint order across slices: every slice's stack lies over every picture,
// a later slice's stack over an earlier one's; the collage pieces (part of their slice's picture) under all stacks. A row
// dragged among another slice's rows re-anchors its box there (strip-anchor.ts: id and place on screen kept) at the
// place it was dropped; pieces and click / hover areas stay with their slice (refused across).
import { setSliceLayers, type SliceLayerRef } from '@scutoons/content';
import { canMoveLayerRow, layerRows, moveLayerRows } from './collage-edit.ts';
import { moveAcrossSeam, type DrawnBox } from './strip-anchor.ts';

/** a row: the slice it is anchored on + its place in that slice's stack */
export interface GroupLayerRow {
	slice: number;
	ref: SliceLayerRef;
}

/** the group's rows front first: the stacks of the last slice to the first, then the pieces likewise */
export function groupLayerRows(slices: readonly unknown[]): GroupLayerRow[] {
	const stacks: GroupLayerRow[] = [];
	const pieces: GroupLayerRow[] = [];
	for (let s = slices.length - 1; s >= 0; s--) {
		for (const ref of layerRows(slices[s])) (ref.kind === 'piece' ? pieces : stacks).push({ slice: s, ref });
	}
	return [...stacks, ...pieces];
}

/** a row's place in its own slice's list (`layerRows`) */
function localIndex(slice: unknown, ref: SliceLayerRef): number {
	return layerRows(slice).findIndex((r) => r.kind === ref.kind && r.index === ref.index);
}

/** what a move did: where the moved entry is now (its slice; `moved` = the open slice's pieces' new indices) */
export type GroupMove =
	| { ok: true; slice: number; kind: SliceLayerRef['kind']; index: number; moved?: number[] }
	| { ok: false; why: 'pieces' | 'area' | 'none' };

/**
 * Moves row `from` of the group's list to `to` (the sortable's indices – the row ends up where row `to` was): within a
 * slice a reorder (`moveLayerRows`); a text box / placed item among another slice's rows is re-anchored there and put
 * in front of (dragged up) or behind (dragged down) the row it landed on. `boxOf(slice)` = the slice's drawn box.
 */
export function moveGroupLayerRow(slices: any[], from: number, to: number, boxOf: (slice: number) => DrawnBox | null): GroupMove {
	const rows = groupLayerRows(slices);
	const row = rows[from];
	const target = rows[to];
	if (!row || !target || from === to) return { ok: false, why: 'none' };
	if ((row.ref.kind === 'piece') !== (target.ref.kind === 'piece')) return { ok: false, why: 'pieces' };
	const t = target.slice;
	if (t === row.slice) {
		const slice = slices[t];
		const a = localIndex(slice, row.ref);
		const b = localIndex(slice, target.ref);
		if (!canMoveLayerRow(layerRows(slice), a, b)) return { ok: false, why: 'pieces' };
		const moved = moveLayerRows(slice, a, b);
		if (!moved) return { ok: false, why: 'none' };
		const index = row.ref.kind === 'piece' ? (moved[row.ref.index] ?? row.ref.index) : row.ref.index;
		return { ok: true, slice: t, kind: row.ref.kind, index, moved };
	}
	if (row.ref.kind === 'piece') return { ok: false, why: 'pieces' };
	if (row.ref.kind !== 'dialog' && row.ref.kind !== 'item') return { ok: false, why: 'area' };
	const fromBox = boxOf(row.slice);
	const toBox = boxOf(t);
	if (!fromBox || !toBox) return { ok: false, why: 'none' };
	const list = row.ref.kind === 'dialog' ? 'dialog' : 'items';
	const index = moveAcrossSeam(slices, list, row.slice, row.ref.index, t, fromBox, toBox);
	if (index < 0) return { ok: false, why: 'none' };
	// the target slice's stack, front first, with the moved one next to the row it landed on
	const stack = layerRows(slices[t]).filter((r) => r.kind !== 'piece');
	const mine = stack.findIndex((r) => r.kind === row.ref.kind && r.index === index);
	const [entry] = stack.splice(mine, 1);
	const at = stack.findIndex((r) => r.kind === target.ref.kind && r.index === target.ref.index);
	stack.splice(from < to ? at + 1 : at, 0, entry);
	setSliceLayers(slices[t], [...stack].reverse());
	return { ok: true, slice: t, kind: row.ref.kind, index };
}
