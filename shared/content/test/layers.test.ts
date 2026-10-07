// A slice's one layer stack of text boxes and placed items (BACKLOG 10.43).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { setSliceLayers, sliceLayers, validateFile } from '../src/index.ts';

test('no layer: items under the text boxes, each in array order', () => {
	assert.deepEqual(sliceLayers({ dialog: [{}, {}], items: [{}] }), [
		{ kind: 'item', index: 0 },
		{ kind: 'dialog', index: 0 },
		{ kind: 'dialog', index: 1 }
	]);
	assert.deepEqual(sliceLayers(null), []);
	assert.deepEqual(sliceLayers({}), []);
});

test('layers interleave both arrays; an entry without one is on top', () => {
	const slice = { dialog: [{ layer: 0 }, { layer: 2 }, {}], items: [{ layer: 1 }, { layer: 3 }] };
	assert.deepEqual(
		sliceLayers(slice).map((r) => `${r.kind[0]}${r.index}`),
		['d0', 'i0', 'd1', 'i1', 'd2']
	);
});

test('setSliceLayers numbers every entry back to front; the arrays keep their order', () => {
	const slice: { dialog: { id: string; layer?: number }[]; items: { id: string; layer?: number }[] } = { dialog: [{ id: 'a' }, { id: 'b' }], items: [{ id: 'x' }] };
	const order = sliceLayers(slice);
	// the item to the front
	setSliceLayers(slice, [...order.slice(1), order[0]]);
	assert.deepEqual(slice.dialog.map((d) => d.id), ['a', 'b']);
	assert.deepEqual(sliceLayers(slice).map((r) => `${r.kind[0]}${r.index}`), ['d0', 'd1', 'i0']);
	assert.deepEqual([slice.dialog[0].layer, slice.dialog[1].layer, slice.items[0].layer], [0, 1, 2]);
});

test('validation: layer is a whole number ≥ 0 on text boxes and placed items', () => {
	const rect = { x: 10, y: 10, width: 20 };
	const chapter = (slice: object) => ({ title: 'P', number: 1, groups: [{ slices: [{ image: 'images/p.webp', width: 390, height: 625, ...slice }] }] });
	const layerProblems = (slice: object) =>
		validateFile('chapter', chapter(slice), 'de', { file: 'x' } as never)
			.filter((p) => p.path.includes('layer'))
			.map((p) => `${p.level} ${p.path.join('.')}`);
	assert.deepEqual(layerProblems({ dialog: [{ rect, text: 'A', layer: 1 }], items: [{ id: 'x-1', item: 'x', rect, layer: 0 }] }), []);
	assert.deepEqual(layerProblems({ dialog: [{ rect, text: 'A', layer: -1 }], items: [{ id: 'x-1', item: 'x', rect, layer: 1.5 }] }), [
		'error groups.0.slices.0.dialog.0.layer',
		'error groups.0.slices.0.items.0.layer'
	]);
});
