// collage-edit.ts + collage-pick.ts (agents/research/COLLAGE.md §5, §7): copy-on-write effects over a look, the picked
// file in the host's path form, the shapes' filters, titles and pages, the layer list with pieces
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { collageLook, pieceStyle, type CollagePiece } from '@scutoons/content';
import {
	canMoveLayerRow,
	editEffect,
	figurePath,
	followPieces,
	layerRows,
	moveLayerRows,
	piecesMoved,
	setBlurRamp,
	setEffect,
	setEffectColor,
	setFigure,
	setFillKind,
	setPlain,
	setPreset,
	type ClearedEffects
} from '../src/lib/editor/collage-edit.ts';
import { allPages, assetRatio, figureImage, figureViews, filterShapes, imagePick, pickedToPiece, shapeTags, shapeTitle } from '../src/lib/editor/collage-pick.ts';

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

/** a colour field's keystrokes: select all + delete, then the new colour typed character by character */
function retype(p: CollagePiece, key: 'outline' | 'shadow', color: string, cleared: ClearedEffects) {
	setEffectColor(p, key, undefined, cleared);
	for (let i = 1; i <= color.length; i++) setEffectColor(p, key, color.slice(0, i), cleared);
}

test("a colour retyped over a look keeps the look's width, offsets, blur and opacity", () => {
	const p = piece({ preset: 'sticker' });
	const cleared: ClearedEffects = new Map();
	retype(p, 'outline', '#000', cleared);
	retype(p, 'shadow', '#333', cleared);
	assert.deepEqual(pieceStyle(p).outline, { color: '#000', width: 3 });
	assert.deepEqual(pieceStyle(p).shadow, { color: '#333', x: 2, y: 2, blur: 0, opacity: 0.8 });
	const fresh = piece({ preset: 'sticker', style: { outline: null } });
	editEffect(fresh, 'outline', { width: 5 });
	assert.deepEqual(fresh.style?.outline, { color: '#ffffff', width: 5 }, "an outline switched off over the look comes back as the look's");
});

test("a colour retyped over the piece's own effect brings it back as it was; under another look it doesn't", () => {
	const own = piece({ style: { shadow: { color: '#ff0000', x: 5, y: 1, blur: 7, opacity: 0.3 } } });
	const cleared: ClearedEffects = new Map();
	setEffectColor(own, 'shadow', undefined, cleared);
	assert.equal(own.style, undefined, 'emptied: the shadow is off');
	retype(own, 'shadow', '#0f0', cleared);
	assert.deepEqual(pieceStyle(own).shadow, { color: '#0f0', x: 5, y: 1, blur: 7, opacity: 0.3 });
	const over = piece({ preset: 'sticker', style: { outline: { color: '#ff0000', width: 6 } } });
	retype(over, 'outline', '#00f', cleared);
	assert.deepEqual(over.style?.outline, { color: '#00f', width: 6 }, "the own width, not the look's 3");
	setEffectColor(over, 'outline', undefined, cleared);
	assert.equal(over.style?.outline, null);
	setPreset(over, 'glow');
	setEffectColor(over, 'outline', '#abc', cleared);
	assert.deepEqual(over.style?.outline, { color: '#abc', width: 2 }, 'a look picked since: the defaults, not the old width');
});

test("a fill switched off over the look comes back as the look's", () => {
	const ink = piece({ preset: 'ink' });
	setFillKind(ink, 'none');
	assert.equal(ink.style?.fill, null);
	setFillKind(ink, 'color');
	assert.equal(ink.style, undefined, "the look's colour again, not an own copy");
	assert.equal(pieceStyle(ink).fill, collageLook('ink')?.fill);
	const burn = piece({ preset: 'burn' });
	setFillKind(burn, 'none');
	setFillKind(burn, 'color');
	const burnFill = collageLook('burn')?.fill;
	assert.equal(burn.style?.fill, typeof burnFill === 'object' && burnFill ? burnFill.stops[0].color : undefined, "another kind starts from the look's colour");
});

test('a character picture never resolves to the missing-image substitute', () => {
	const look = (images: object) => ({ id: 'default', default: true, images });
	const character = (images: object): any => ({ id: 'x', name: { en: 'X' }, color: '#000', personas: [{ id: 'regular', looks: [look(images)] }] });
	assert.equal(figureImage(character({}), { mood: 'neutral', view: 'portrait' }), '', 'no picture at all');
	assert.equal(figureImage(character({}), { view: 'full' }), '');
	const drawn = character({ neutral: { portrait: 'characters/x/neutral.webp' } });
	assert.equal(figureImage(drawn, { mood: 'happy', view: 'full' }), 'characters/x/neutral.webp', 'the fallback rule still applies');
	assert.equal(figureImage(drawn, { view: 'full' }, (path) => path === 'characters/x/neutral.webp'), '', 'its only file is known to be missing');
	assert.equal(figureImage(null, { view: 'portrait' }), '');
});

test('a view counts only with a file of its own – never the portrait the fallback rule draws under its name', () => {
	const biker = (images: object): any => ({ id: 'x', name: { en: 'X' }, color: '#000', personas: [{ id: 'regular', looks: [{ id: 'default', default: true, images }] }] });
	const two = biker({ neutral: { portrait: 'x_portrait-neutral.webp', full: 'x_full-neutral.webp' }, happy: { avatar: 'x_avatar-happy.webp', portrait: 'x_portrait-happy.webp' } });
	assert.deepEqual(figureViews(two, { mood: 'happy' }), ['avatar', 'portrait'], "happy's full would be its portrait");
	assert.equal(figureImage(two, { mood: 'happy', view: 'full' }), 'x_portrait-happy.webp', 'what the rule draws instead');
	assert.deepEqual(figureViews(two, {}), ['portrait', 'full'], 'neutral (no mood): no avatar');
	const fullOnly = biker({ neutral: { portrait: 'x_portrait-neutral.webp' }, sad: { full: 'x_full-sad.webp' } });
	assert.deepEqual(figureViews(fullOnly, { mood: 'sad' }), ['portrait', 'full'], "a full of its own counts though the portrait is neutral's");
	assert.deepEqual(figureViews(two, { mood: 'happy' }, (path) => path === 'x_avatar-happy.webp'), ['portrait'], 'a known-missing file is none');
	assert.deepEqual(figureViews(biker({}), { mood: 'happy' }), [], 'no picture at all');
	assert.deepEqual(figureViews(null, {}), []);
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

test('the kind a fill / blur already has changes nothing – a re-click keeps a gradient, the ramp points and the look', () => {
	const burn = piece({ preset: 'burn' });
	assert.equal(setFillKind(burn, 'gradient'), false);
	assert.equal(burn.style, undefined, "the look's gradient stays the look's");
	const own = { angle: 45, stops: [{ at: 0, color: '#ff0000' }, { at: 50, color: '#00ff00' }, { at: 100, color: '#0000ff' }] };
	const p = piece({ style: { fill: JSON.parse(JSON.stringify(own)), blur: { amount: 5, from: { x: 10, y: 20 }, to: { x: 80, y: 90 } } } });
	assert.equal(setFillKind(p, 'gradient'), false);
	assert.deepEqual(p.style?.fill, own, 'angle, end colour and the middle stop stay');
	assert.equal(setBlurRamp(p, true), false);
	assert.deepEqual(p.style?.blur, { amount: 5, from: { x: 10, y: 20 }, to: { x: 80, y: 90 } }, 'the points stay');
	const ink = piece({ preset: 'ink' });
	assert.equal(setFillKind(ink, 'color'), false);
	const ghost = piece({ preset: 'ghost' });
	assert.equal(setBlurRamp(ghost, false), false);
	assert.deepEqual([ink.style, ghost.style], [undefined, undefined], "a re-click of the look's colour / even blur makes no own copy");
	assert.equal(setFillKind(ink, 'gradient'), true);
	assert.deepEqual(ink.style?.fill, { stops: [{ at: 0, color: '#111111' }, { at: 100, color: '#ffffff' }] }, 'another kind starts from the colour');
	assert.equal(setFillKind(ink, 'none'), true);
	assert.equal(ink.style?.fill, null, "none over the look's fill = off");
});

test('figurePath finds the waiting re-pick its piece by id – never the piece now at the old path', () => {
	const slice: any = { pieces: [piece({ id: 'a', figure: { id: 'catpope', view: 'portrait' } }), piece({ id: 'shape' }), piece({ id: 'b', figure: { id: 'catpope', view: 'full' } })] };
	const file = { slices: [slice] };
	assert.deepEqual(figurePath(file, ['slices', 0, 'pieces', 2], 'b'), ['slices', 0, 'pieces', 2]);
	slice.pieces.splice(0, 1);
	assert.deepEqual(figurePath(file, ['slices', 0, 'pieces', 2], 'b'), ['slices', 0, 'pieces', 1], 'a Remove renumbered the list');
	assert.equal(figurePath(file, ['slices', 0, 'pieces', 0], 'a'), undefined, 'gone: dropped, the shape now at its index untouched');
	slice.pieces[1].figure = undefined;
	assert.equal(figurePath(file, ['slices', 0, 'pieces', 1], 'b'), undefined, 'no longer a character picture');
	assert.equal(figurePath(file, ['slices', 5, 'pieces', 0], 'b'), undefined);
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

test("a shape piece's library title by its file's base name, in a slice's and a score's path form", () => {
	const library = [
		{ path: 'catpope/vectors/disc-red.svg', title: 'Disc red' },
		{ path: 'catpope/vectors/moon.svg', title: '' }
	];
	assert.equal(shapeTitle('vectors/disc-red.svg', library), 'Disc red', 'a catpope slice');
	assert.equal(shapeTitle('../catpope/vectors/disc-red.svg', library), 'Disc red', "another toon's slice");
	assert.equal(shapeTitle('catpope/vectors/disc-red.svg', library), 'Disc red', 'a score');
	assert.equal(shapeTitle('vectors/moon.svg', library), undefined, 'no title: the caller falls back to the file');
	assert.equal(shapeTitle('vectors/gone.svg', library), undefined);
	assert.equal(shapeTitle('vectors/disc-red.svg', null), undefined, 'the library not loaded yet');
	assert.equal(shapeTitle(undefined, library), undefined);
});

test('allPages fetches the next pages until the total – a 501st shape stays in the picker', async () => {
	const rows = Array.from({ length: 1201 }, (_, i) => i);
	const asked: number[] = [];
	const page = async (n: number) => (asked.push(n), { items: rows.slice((n - 1) * 500, n * 500), pagination: { total: rows.length } });
	assert.deepEqual(await allPages(page), rows);
	assert.deepEqual(asked, [1, 2, 3]);
	asked.length = 0;
	assert.deepEqual(await allPages(async (n) => (asked.push(n), { items: n === 1 ? [1, 2] : [], pagination: { total: 5 } })), [1, 2], 'an empty page ends it (a total that shrank)');
	assert.deepEqual(asked, [1, 2]);
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
	assert.equal(moveLayerRows(slice, 2, 1), null, 'a piece never goes over a text box');
	assert.equal(moveLayerRows(slice, 0, 3), null, 'a text box never goes under a piece');
	assert.equal(slice.dialog[0].layer, undefined, 'a refused move changes nothing');
	assert.deepEqual(moveLayerRows(slice, 2, 3), [1, 0], 'where each piece went (old → new)');
	assert.deepEqual(slice.pieces.map((p: CollagePiece) => p.id), ['front', 'back'], 'the front piece goes to the back');
	assert.equal(slice.pieces[0].layer, undefined, 'pieces carry no layer');
	assert.deepEqual(moveLayerRows(slice, 1, 0), [0, 1], 'a stack move leaves the pieces where they are');
	assert.deepEqual([slice.dialog[0].layer, slice.dialog[1].layer], [1, 0], 'the stack is renumbered');
});

test('the selected piece follows its piece through a Layers reorder and an undo / redo', () => {
	const ids = (s: any) => s.pieces.map((p: CollagePiece) => p.id);
	const slice: any = { dialog: [{ id: 'd0' }], pieces: ['a', 'b', 'c'].map((id) => piece({ id })) };
	// rows front first: dialog:0, piece:2 (c), piece:1 (b), piece:0 (a) – the back piece a one row up (in front of b)
	const before = [...slice.pieces];
	const moved = moveLayerRows(slice, 3, 2)!;
	assert.deepEqual(ids(slice), ['b', 'a', 'c']);
	assert.deepEqual(moved, [1, 0, 2]);
	assert.deepEqual(followPieces({ kind: 'piece', index: 0 }, moved), { kind: 'piece', index: 1 }, 'the moved (selected) piece stays selected');
	assert.deepEqual(followPieces({ kind: 'piece', index: 1 }, moved), { kind: 'piece', index: 0 }, 'a selected piece the move pushed aside too');
	const still = { kind: 'piece', index: 2 };
	assert.equal(followPieces(still, moved), still, 'a piece that stayed: the same selection');
	const text = { kind: 'dialog', index: 0 };
	assert.equal(followPieces(text, moved), text, 'a text box: as it is');
	assert.equal(followPieces(null, moved), null);
	// a front piece moved to the back over two rows: c is now first
	const far = moveLayerRows(slice, 1, 3)!;
	assert.deepEqual(ids(slice), ['c', 'b', 'a']);
	assert.deepEqual(followPieces({ kind: 'piece', index: 2 }, far), { kind: 'piece', index: 0 });
	// an undo puts the list back (another array, by id); an undo of an add takes the selected piece away
	assert.deepEqual(piecesMoved(slice.pieces, before), [2, 1, 0]);
	assert.deepEqual(followPieces({ kind: 'piece', index: 0 }, piecesMoved(slice.pieces, before)), { kind: 'piece', index: 2 }, 'undo: c stays selected');
	const added = [...before, piece({ id: 'd' })];
	assert.equal(followPieces({ kind: 'piece', index: 3 }, piecesMoved(added, before)), null, 'undo of an add: the piece is gone, let go');
	assert.deepEqual(piecesMoved([{ src: 'x' }, piece({ id: 'b' })], [{ src: 'x' }]), [0, -1], 'no id: its index while there is one');
	assert.deepEqual(piecesMoved(undefined, before), []);
});
