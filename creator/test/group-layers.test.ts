import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupLayerRows, moveGroupLayerRow } from '../src/lib/editor/group-layers.ts';

// two stacked slices, 390 × 625 each
const boxes = [
	{ left: 0, top: 0, width: 390, height: 625 },
	{ left: 0, top: 625, width: 390, height: 625 }
];
const boxOf = (i: number) => boxes[i] ?? null;
const rect = { x: 10, y: 10, width: 30, height: 10 };
const group = () => [
	{ dialog: [{ id: 'a1', rect }, { id: 'a2', rect }], pieces: [{ id: 'p1', src: 'x.png', rect }] },
	{ dialog: [{ id: 'b1', rect }], items: [{ id: 'gurke-1', item: 'gurke', rect }], events: [{ type: 'fx', trigger: { type: 'click', rect } }] }
];
const ids = (slices: any[]) => groupLayerRows(slices).map(({ slice, ref }) => `${slice}:${ref.kind === 'dialog' ? slices[slice].dialog[ref.index].id : ref.kind === 'item' ? slices[slice].items[ref.index].id : ref.kind === 'piece' ? slices[slice].pieces[ref.index].id : ref.kind}`);

test("groupLayerRows: front first – the later slice's stack, the earlier one's, then the pieces", () => {
	assert.deepEqual(ids(group()), ['1:event', '1:b1', '1:gurke-1', '0:a2', '0:a1', '0:p1']);
});

test('moveGroupLayerRow: within a slice a reorder, across slices the box is re-anchored at the drop', () => {
	// within slice 0: a2 behind a1
	const g1 = group();
	assert.deepEqual(moveGroupLayerRow(g1, 3, 4, boxOf), { ok: true, slice: 0, kind: 'dialog', index: 1, moved: [0] });
	assert.deepEqual(ids(g1), ['1:event', '1:b1', '1:gurke-1', '0:a1', '0:a2', '0:p1']);
	// a2 (slice 0) dragged up onto b1 (slice 1): it moves to slice 1, in front of b1 – the same place on screen
	const g2 = group();
	const r = moveGroupLayerRow(g2, 3, 1, boxOf);
	assert.deepEqual(r, { ok: true, slice: 1, kind: 'dialog', index: 1 });
	assert.deepEqual(ids(g2), ['1:event', '1:a2', '1:b1', '1:gurke-1', '0:a1', '0:p1']);
	assert.deepEqual(g2[1].dialog[1].rect, { x: 10, y: -90, width: 30, height: 10 });
	// the item (slice 1) dragged down onto a1 (slice 0): to slice 0, behind a1
	const g3 = group();
	assert.equal(moveGroupLayerRow(g3, 2, 4, boxOf).ok, true);
	assert.deepEqual(ids(g3), ['1:event', '1:b1', '0:a2', '0:a1', '0:gurke-1', '0:p1']);
	assert.equal(g3[1].items, undefined);
	// refused: an area across slices, a stack row among the pieces
	assert.deepEqual(moveGroupLayerRow(group(), 0, 3, boxOf), { ok: false, why: 'area' });
	assert.deepEqual(moveGroupLayerRow(group(), 1, 5, boxOf), { ok: false, why: 'pieces' });
});
