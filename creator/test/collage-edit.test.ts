// collage-edit.ts + collage-pick.ts (agents/research/COLLAGE.md §5, §7): copy-on-write effects over a look, the picked
// file in the host's path form, the shapes' filters, the layer list with pieces
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { collageLook, pieceStyle, type CollagePiece } from '@scutoons/content';
import { canMoveLayerRow, editEffect, layerRows, moveLayerRows, setBlurRamp, setEffect, setFigure, setPlain, setPreset } from '../src/lib/editor/collage-edit.ts';
import { assetRatio, filterShapes, imagePick, pickedToPiece, shapeTags } from '../src/lib/editor/collage-pick.ts';

const piece = (more: Partial<CollagePiece> = {}): CollagePiece => ({ id: 'piece-a', src: 'vectors/x.svg', ratio: 1, rect: { x: 10, y: 10, width: 30 }, ...more });

test("editEffect copies the look's shadow before changing x – the look itself stays", () => {
	const p = piece({ preset: 'sticker' });
	editEffect(p, 'shadow', { x: 5 });
	assert.deepEqual(p.style?.shadow, { color: '#111111', x: 5, y: 2, blur: 0, opacity: 0.8 });
	assert.equal(collageLook('sticker')?.shadow?.x, 2, 'the code look is untouched');
	assert.equal(pieceStyle(p).outline?.width, 3, "the look's outline still applies");
	editEffect(p, 'shadow', { y: 7 });
	assert.deepEqual(p.style?.shadow, { color: '#111111', x: 5, y: 7, blur: 0, opacity: 0.8 }, 'the second change edits the own copy');
});

test('editEffect without a look starts from the defaults; a field set to undefined goes', () => {
	const p = piece();
	editEffect(p, 'shadow', { color: '#ff0000' });
	assert.deepEqual(p.style?.shadow, { color: '#ff0000', x: 2, y: 2, blur: 4 });
	editEffect(p, 'shadow', { opacity: 0.5 });
	editEffect(p, 'shadow', { opacity: undefined });
	assert.equal('opacity' in (p.style?.shadow ?? {}), false);
});

test('clearing an effect the look has writes null, one it lacks deletes the key; an emptied style goes', () => {
	const sticker = piece({ preset: 'sticker' });
	setEffect(sticker, 'outline', undefined);
	assert.equal(sticker.style?.outline, null, 'off over the look');
	assert.equal(pieceStyle(sticker).outline, undefined);
	const glow = piece({ preset: 'glow', style: { outline: { color: '#000000', width: 1 } } });
	setEffect(glow, 'outline', undefined);
	assert.equal(glow.style, undefined, 'the look has no outline: the key goes, then the empty style');
	const plain = piece({ style: { blend: 'multiply' } });
	setPlain(plain, 'blend', undefined);
	assert.equal(plain.style, undefined);
});

test('setPreset writes the look and drops the own values in one go; none = as drawn', () => {
	const p = piece({ preset: 'ink', style: { opacity: 0.4, shadow: null } });
	setPreset(p, 'negative');
	assert.equal(p.preset, 'negative');
	assert.equal(p.style, undefined);
	setPreset(p, undefined);
	assert.equal('preset' in p, false);
});

test('setBlurRamp keeps the amount and adds / drops the points', () => {
	const p = piece({ preset: 'ghost' });
	setBlurRamp(p, true);
	assert.deepEqual(p.style?.blur, { amount: 3, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } });
	setBlurRamp(p, false);
	assert.deepEqual(p.style?.blur, { amount: 3 });
});

test('setFigure re-resolves the picture: figure, src and ratio in one change; a ratio it could not read is kept', () => {
	const p = piece({ src: '../characters/catpope/a.webp', ratio: 0.8, figure: { id: 'catpope', mood: 'neutral', view: 'portrait' } });
	setFigure(p, { id: 'catpope', mood: 'happy', view: 'full' }, '../characters/catpope/b.webp', 0.5);
	assert.deepEqual([p.figure?.mood, p.figure?.view, p.src, p.ratio], ['happy', 'full', '../characters/catpope/b.webp', 0.5]);
	setFigure(p, { id: 'catpope', mood: 'sad', view: 'full' }, '../characters/catpope/c.webp', null);
	assert.equal(p.ratio, 0.5);
});

test('pickedToPiece writes vectors/x.svg in catpope, ../catpope/vectors/x.svg in toptoy and the root path on a score', () => {
	const picked = { src: 'catpope/vectors/x.svg', ratio: 2, name: 'x' };
	const place = { rect: { x: 50, y: 50, width: 30 }, anchor: 'c' as const };
	const a = pickedToPiece(picked, 'catpope', place);
	assert.equal(a.src, 'vectors/x.svg');
	assert.match(a.id, /^piece-[a-z0-9]{7}$/);
	assert.equal(a.anchor, 'c');
	assert.equal(a.ratio, 2);
	assert.equal(pickedToPiece(picked, 'toptoy', place).src, '../catpope/vectors/x.svg');
	assert.equal(pickedToPiece(picked, '', place).src, 'catpope/vectors/x.svg');
	const tl = pickedToPiece({ src: 'characters/catpope/p.webp', name: 'p', figure: { id: 'catpope', view: 'portrait' } }, 'catpope', { rect: { x: 1, y: 2, width: 3 }, anchor: 'tl' });
	assert.equal('anchor' in tl, false, "'tl' is no key");
	assert.equal(tl.ratio, 1, 'an unknown ratio is 1');
	assert.deepEqual(tl.figure, { id: 'catpope', view: 'portrait' });
	place.rect.x = 99;
	assert.equal(a.rect.x, 50, 'the rect is copied');
});

test('the shapes: tags by frequency, search over title / slug / tags, ratio from the sizes', () => {
	const shapes = [
		{ title: 'Arc blue', slug: 'catpope/vectors/arc-blue', tags: ['line', 'arc'] },
		{ title: 'Wreath', slug: 'catpope/vectors/wreath', tags: ['ornament', 'line'] },
		{ title: 'Moon', slug: 'catpope/vectors/moon', tags: ['crescent'] }
	];
	assert.deepEqual(shapeTags(shapes).slice(0, 2), [{ tag: 'line', count: 2 }, { tag: 'arc', count: 1 }]);
	assert.deepEqual(filterShapes(shapes, 'line').map((s) => s.title), ['Arc blue', 'Wreath']);
	assert.deepEqual(filterShapes(shapes, 'WREATH orn').map((s) => s.title), ['Wreath']);
	assert.deepEqual(filterShapes(shapes, '', 'crescent').map((s) => s.title), ['Moon']);
	assert.equal(assetRatio({ width: 178, height: 458 }), 0.3886);
	assert.equal(assetRatio({ width: null, height: 10 }), null);
});

test('an image pick prefers the WebP twin of a PNG', () => {
	const asset: any = {
		title: 'Halo',
		slug: 'catpope/halo',
		width: 400,
		height: 200,
		renditions: [
			{ path: 'catpope/halo.png', mime: 'image/png', width: 400, height: 200 },
			{ path: 'catpope/halo.webp', mime: 'image/webp', width: 400, height: 200 }
		]
	};
	assert.deepEqual(imagePick(asset, asset.renditions[0], 'catpope/halo.png'), { src: 'catpope/halo.webp', ratio: 2, name: 'Halo' });
	assert.equal(imagePick(asset, asset.renditions[1], 'catpope/halo.webp').src, 'catpope/halo.webp');
});

test('moveLayerRows refuses piece ↔ stack and reorders within the pieces and within the stack', () => {
	const slice: any = {
		dialog: [{ id: 'd0' }, { id: 'd1' }],
		pieces: [piece({ id: 'back' }), piece({ id: 'front' })]
	};
	const rows = layerRows(slice);
	assert.deepEqual(rows.map((r) => `${r.kind}:${r.index}`), ['dialog:1', 'dialog:0', 'piece:1', 'piece:0'], 'front first, the pieces at the bottom');
	assert.equal(canMoveLayerRow(rows, 1, 2), false);
	assert.equal(moveLayerRows(slice, 2, 1), false, 'a piece never goes over a text box');
	assert.equal(moveLayerRows(slice, 0, 3), false, 'a text box never goes under a piece');
	assert.equal(slice.dialog[0].layer, undefined, 'a refused move changes nothing');
	assert.equal(moveLayerRows(slice, 2, 3), true);
	assert.deepEqual(slice.pieces.map((p: CollagePiece) => p.id), ['front', 'back'], 'the front piece goes to the back');
	assert.equal(slice.pieces[0].layer, undefined, 'pieces carry no layer');
	assert.equal(moveLayerRows(slice, 1, 0), true);
	assert.deepEqual([slice.dialog[0].layer, slice.dialog[1].layer], [1, 0], 'the stack is renumbered');
});
