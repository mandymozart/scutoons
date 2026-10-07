// A slice's one layer stack of text boxes, placed items and click / hover areas (BACKLOG 10.43 + 10.54).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { hasSliceLayers, setSliceLayers, sliceEventAreas, sliceLayers, validateFile } from '../src/index.ts';

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

const rect = { x: 10, y: 10, width: 20, height: 20 };
const code = (refs: { kind: string; index: number }[]) => refs.map((r) => `${r.kind[0]}${r.index}`);

test('areas: click / hover events with a rect (one per trigger) and samples with an area join the stack', () => {
	const slice = {
		dialog: [{}],
		items: [{}],
		events: [
			{ type: 'showMessage', trigger: { type: 'click', rect } },
			{ type: 'showChoices', trigger: { type: 'click', rect } },
			{ type: 'showMessage', trigger: { type: 'click' } },
			{ type: 'showMessage', trigger: { type: 'intersect', rect } },
			{ type: 'showMessage', trigger: { type: 'hover', rect: { ...rect, x: 50 } } }
		],
		audio: { audio: { samples: [{ id: 'a', trigger: { type: 'click' } }, { id: 'b', trigger: { type: 'hover', rect } }] } }
	};
	// no layer: the reader's old z-order – sample areas, items, text boxes, event areas
	assert.deepEqual(code(sliceLayers(slice)), ['s1', 'i0', 'd0', 'e0', 'e4']);
	assert.deepEqual(sliceLayers(slice).find((r) => r.kind === 'event')?.events, [0, 1]);
	assert.deepEqual(sliceEventAreas(slice), [[0, 1], [4]]);
	assert.equal(hasSliceLayers(slice), false);
	// the repeat rule's spelling doesn't split an area (triggerKey)
	assert.deepEqual(sliceEventAreas({ events: [{ trigger: { type: 'click', rect } }, { trigger: { type: 'click', rect, repeat: { mode: 'every' } } }] }), [[0, 1]]);
});

test('setSliceLayers numbers areas too; the events of an area share the number', () => {
	const slice: any = {
		dialog: [{}],
		events: [
			{ type: 'showMessage', trigger: { type: 'click', rect } },
			{ type: 'showChoices', trigger: { type: 'click', rect } }
		],
		audio: { audio: { samples: [{ id: 'b', trigger: { type: 'click', rect } }] } }
	};
	const order = sliceLayers(slice);
	assert.deepEqual(code(order), ['s0', 'd0', 'e0']);
	// the event area to the back, the sample to the front
	setSliceLayers(slice, [order[2], order[1], order[0]]);
	assert.deepEqual([slice.events[0].layer, slice.events[1].layer, slice.dialog[0].layer, slice.audio.audio.samples[0].layer], [0, 0, 1, 2]);
	assert.deepEqual(code(sliceLayers(slice)), ['e0', 'd0', 's0']);
	assert.equal(hasSliceLayers(slice), true);
	// an event added to the area later (no layer) stays in it, at the area's place
	slice.events.push({ type: 'addChoice', trigger: { type: 'click', rect } });
	assert.deepEqual(sliceLayers(slice)[0].events, [0, 1, 2]);
	assert.deepEqual(code(sliceLayers(slice)), ['e0', 'd0', 's0']);
});

test('validation: layer is a whole number ≥ 0 on slice events and samples', () => {
	const chapter = (slice: object) => ({
		title: 'P',
		number: 1,
		audio: { sampleLibrary: [{ id: 'b', src: 'b.mp3' }] },
		groups: [{ slices: [{ image: 'images/p.webp', width: 390, height: 625, ...slice }] }]
	});
	const layerProblems = (slice: object) =>
		validateFile('chapter', chapter(slice), 'de', { file: 'x' } as never)
			.filter((p) => p.path.includes('layer'))
			.map((p) => `${p.level} ${p.path.join('.')}`);
	const event = (layer: number) => ({ type: 'showMessage', params: { text: 'Hi' }, trigger: { type: 'click', rect }, layer });
	const audio = (layer: number) => ({ audio: { samples: [{ id: 'b', trigger: { type: 'click', mode: 'poly', rect }, layer }] } });
	assert.deepEqual(layerProblems({ events: [event(2)], audio: audio(0) }), []);
	assert.deepEqual(layerProblems({ events: [event(-1)], audio: audio(0.5) }), [
		'error groups.0.slices.0.audio.audio.samples.0.layer',
		'error groups.0.slices.0.events.0.layer'
	]);
});
