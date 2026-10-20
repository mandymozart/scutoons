import { test } from 'node:test';
import assert from 'node:assert/strict';
import { convertPoint, convertRect, moveAcrossSeam, sliceAt } from '../src/lib/editor/strip-anchor.ts';
import { nextDialogId } from '../src/lib/editor/defaults.ts';

// two slices stacked (vertical): 390 × 625 at y 0 and 390 × 500 at y 625
const a = { left: 0, top: 0, width: 390, height: 625 };
const b = { left: 0, top: 625, width: 390, height: 500 };

test('convertPoint / convertRect: the same spot on screen, the size in the other slice', () => {
	assert.deepEqual(convertPoint({ x: 10, y: 110 }, a, b), { x: 10, y: 12.5 });
	assert.deepEqual(convertPoint({ x: 10, y: -10 }, b, a), { x: 10, y: 92 });
	assert.deepEqual(convertRect({ x: 10, y: 110, width: 50, height: 20 }, a, b), { x: 10, y: 12.5, width: 50, height: 25 });
	// no height (auto height): stays without one
	assert.deepEqual(convertRect({ x: 10, y: 110, width: 50 }, a, b), { x: 10, y: 12.5, width: 50 });
	// horizontal: side by side at the same height, different widths
	const h1 = { left: 0, top: 0, width: 400, height: 800 };
	const h2 = { left: 400, top: 0, width: 200, height: 800 };
	assert.deepEqual(convertRect({ x: 110, y: 30, width: 20, height: 10 }, h1, h2), { x: 20, y: 30, width: 40, height: 10 });
});

test('moveAcrossSeam: the entry moves with its id, its rect and its tail tip converted, appended in front', () => {
	const slices: any[] = [
		{ image: 'a', dialog: [{ id: 'x', rect: { x: 10, y: 110, width: 50, height: 20 }, tail: { kind: 'speech', tip: { x: 20, y: 130 }, base: { side: 'bottom', at: 0.5 } } }] },
		{ image: 'b', dialog: [{ id: 'y', rect: { x: 0, y: 0, width: 10 } }] }
	];
	assert.equal(moveAcrossSeam(slices, 'dialog', 0, 0, 1, a, b), 1);
	assert.equal(slices[0].dialog, undefined, 'an emptied list goes');
	assert.deepEqual(slices[1].dialog[1], { id: 'x', rect: { x: 10, y: 12.5, width: 50, height: 25 }, tail: { kind: 'speech', tip: { x: 20, y: 37.5 }, base: { side: 'bottom', at: 0.5 } } });
	// items: the list is created; nothing moves onto its own slice or from a missing entry
	const withItem: any[] = [{ items: [{ id: 'tuna-1', item: 'tuna', rect: { x: 50, y: 105, width: 10 } }] }, {}];
	assert.equal(moveAcrossSeam(withItem, 'items', 0, 0, 1, a, b), 0);
	assert.deepEqual(withItem[1].items[0], { id: 'tuna-1', item: 'tuna', rect: { x: 50, y: 6.25, width: 10 } });
	assert.equal(moveAcrossSeam(withItem, 'items', 1, 0, 1, b, b), -1);
	assert.equal(moveAcrossSeam(withItem, 'items', 0, 3, 1, a, b), -1);
});

test('sliceAt: the slice under a point along the axis', () => {
	assert.equal(sliceAt([a, b], { x: 999, y: 700 }, 'vertical'), 1);
	assert.equal(sliceAt([a, null, b], { x: 0, y: 10 }, 'vertical'), 0);
	assert.equal(sliceAt([a, b], { x: 0, y: 2000 }, 'vertical'), -1);
	assert.equal(sliceAt([{ left: 0, top: 0, width: 400, height: 800 }, { left: 400, top: 0, width: 200, height: 800 }], { x: 450, y: 9999 }, 'horizontal'), 1);
});

test('nextDialogId: a chapter counter (a box may move to another slice – its id names none)', () => {
	assert.equal(nextDialogId(2), 's1c2-t1');
	// the next after the highest counter; older slice-numbered ids don't count, nor other chapters'
	assert.equal(nextDialogId(2, ['s1c2-5-1', 's1c2-t3', 's1c2-t1', 's1c3-t9', 's1c2-tx']), 's1c2-t4');
	assert.equal(nextDialogId(12, ['s1c12-t7'], 1), 's1c12-t8');
});
