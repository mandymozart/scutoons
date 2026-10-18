// canvas-pieces.ts (research/COLLAGE.md §3): collage pieces as slice canvas boxes, where a new piece goes
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { NEW_PIECE_WIDTH, pieceItems, piecePlace } from '../src/lib/editor/canvas-pieces.ts';

const RATIO = 625 / 390;

test('pieceItems: the height from the file\'s ratio and the slice\'s, never stored; path, rotation, anchor, label', () => {
	const pieces = [
		{ id: 'piece-a', src: 'vectors/disc.svg', ratio: 1, rect: { x: 10, y: 20, width: 30 } },
		{ id: 'piece-b', src: '../characters/catpope/catpope_portrait-neutral.webp', ratio: 0.5, rect: { x: 50, y: 50, width: 20 }, anchor: 'c', rotation: 15, figure: { id: 'catpope', mood: 'neutral', view: 'portrait' } }
	];
	const items = pieceItems(pieces, ['groups', 0, 'slices', 1], RATIO);
	assert.equal(items.length, 2);
	const [a, b] = items;
	// a square file 30 % wide on a 390 × 625 slice: 30 × 390 / 625 = 18.72 % of the slice's height
	assert.deepEqual(a.rect, { x: 10, y: 20, width: 30, height: 18.72 });
	assert.deepEqual(a.path, ['groups', 0, 'slices', 1, 'pieces', 0, 'rect']);
	assert.equal(a.label, '✂ disc');
	assert.equal(a.rotation, 0);
	assert.deepEqual(a.f, { ax: 0, ay: 0 });
	assert.equal(a.ramp, undefined);
	// a 1 : 2 portrait 20 % wide: 20 / (0.5 × 625 / 390) = 24.96 %
	assert.equal(b.rect.height, 24.96);
	assert.equal(b.rotation, 15);
	assert.deepEqual(b.f, { ax: 0.5, ay: 0.5 });
	assert.equal(b.label, '✂ catpope · neutral');
	assert.equal(b.kind, 'piece');
	assert.equal(b.index, 1);
});

test('pieceItems: a ramp\'s points (own or the look\'s) for the canvas dots; broken pieces have no box', () => {
	const pieces = [
		null,
		{ id: 'x', src: 'a.svg', ratio: 0, rect: { x: 0, y: 0, width: 10 } },
		{ id: 'y', src: 'a.svg', ratio: 1, rect: { x: 0, y: 0 } },
		{ id: 'fade', src: 'a.svg', ratio: 1, rect: { x: 0, y: 0, width: 10 }, preset: 'fade' },
		{ id: 'own', src: 'a.svg', ratio: 1, rect: { x: 0, y: 0, width: 10 }, style: { blur: { amount: 3, from: { x: 0, y: 50 }, to: { x: 100, y: 50 } } } },
		{ id: 'even', src: 'a.svg', ratio: 1, rect: { x: 0, y: 0, width: 10 }, preset: 'ghost' }
	];
	const items = pieceItems(pieces, [], RATIO);
	assert.deepEqual(items.map((i) => i.index), [3, 4, 5], 'the index stays the list\'s');
	assert.deepEqual(items[0].ramp, { from: { x: 50, y: 0 }, to: { x: 50, y: 100 } });
	assert.deepEqual(items[1].ramp, { from: { x: 0, y: 50 }, to: { x: 100, y: 50 } });
	assert.equal(items[2].ramp, undefined, 'an even blur has no dots');
	assert.deepEqual(pieceItems(undefined, [], RATIO), []);
});

test('piecePlace: a click centres the piece there, a drag keeps its corner and width, none = the middle', () => {
	assert.deepEqual(piecePlace({ x: 20.123, y: 70, width: 0 }), { rect: { x: 20.12, y: 70, width: NEW_PIECE_WIDTH }, anchor: 'c' });
	assert.deepEqual(piecePlace({ x: 10, y: 15 }), { rect: { x: 10, y: 15, width: NEW_PIECE_WIDTH }, anchor: 'c' });
	assert.deepEqual(piecePlace({ x: 10, y: 15, width: 42.5 }), { rect: { x: 10, y: 15, width: 42.5 } });
	assert.deepEqual(piecePlace(null), { rect: { x: 50, y: 50, width: NEW_PIECE_WIDTH }, anchor: 'c' });
	assert.deepEqual(piecePlace({ x: -5, y: 120 }).rect, { x: 0, y: 100, width: NEW_PIECE_WIDTH });
});
