// score-collage.ts + screens.ts (agents/research/COLLAGE.md §4.4): the score stage's edits on the item document – the
// part's collage bound to its file, new / moved / scaled / removed pieces, the Bar / Page switch, the list's sides, the
// stage's problems – and the simulated screens
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { CollagePiece } from '@scutoons/content';
import { placeScorePiece, type PageGeometry } from '@scutoons/reader/score-anchors';
import {
	addScorePiece,
	canMoveCollageRow,
	centrePlace,
	collagePath,
	collageRows,
	cornerWidth,
	ensureSheet,
	fileChanged,
	missingBars,
	moveCollageRows,
	movePiece,
	newScorePiece,
	partFile,
	pieceHandle,
	removeScorePiece,
	setPiecePage,
	sheetPieces,
	stagePart,
	stageProblems
} from '../src/lib/editor/score-collage.ts';
import { SCREENS, readerSizes, screenScale, screenZoom } from '../src/lib/editor/screens.ts';

const FILE = 'scores/cloud9_en/4971f460e393.musicxml';
const CHOIR = 'scores/cloud9_en/choir.musicxml';
const item = (score: Record<string, unknown> = { musicxml: FILE }) => ({ id: 'cloud9_en', category: 'score', score });

/** one page, 1000 × 1500 units, sp 10: bars 1 and 2 on one system of two staves, bar 3 on the next */
const STAVES = [
	{ n: 1, top: 200, bottom: 240 },
	{ n: 2, top: 300, bottom: 340 }
];
const geo: PageGeometry = {
	width: 1000,
	height: 1500,
	sp: 10,
	frame: { x: 0, y: 0, width: 100 },
	bars: [
		{ id: 'm1', n: '1', nth: 1, left: 100, right: 500, staves: STAVES, beats: [{ t: 0, x: 150 }, { t: 1, x: 500 }] },
		{ id: 'm2', n: '2', nth: 1, left: 500, right: 900, staves: STAVES, beats: [{ t: 0, x: 520 }, { t: 1, x: 900 }] },
		{ id: 'm3', n: '3', nth: 1, left: 100, right: 900, staves: STAVES.map((s) => ({ ...s, top: s.top + 400, bottom: s.bottom + 400 })), beats: [{ t: 0, x: 150 }, { t: 1, x: 900 }] }
	]
};
const shape = { src: 'catpope/vectors/wreath.svg', ratio: 2, name: 'wreath' };
const piece = (id: string, extra: Partial<CollagePiece> = {}): CollagePiece => ({ id, src: shape.src, ratio: 2, rect: { x: 50, y: 2, width: 12 }, anchor: 'c', at: { bar: '1' }, ...extra });

test('the part readers open: the choir when there is one, else the full score – the collage and its file there', () => {
	assert.equal(stagePart(item()), null);
	assert.equal(stagePart(item({ musicxml: FILE, parts: { choir: { musicxml: CHOIR } } })), 'choir');
	assert.equal(stagePart(null), null);
	assert.deepEqual(collagePath(null), ['score', 'collage']);
	assert.deepEqual(collagePath('choir'), ['score', 'parts', 'choir', 'collage']);
	assert.equal(partFile(item(), null), FILE);
	assert.equal(partFile(item({ musicxml: FILE, parts: { choir: { musicxml: CHOIR } } }), 'choir'), CHOIR);
	assert.equal(partFile(item({ pdf: 'x.pdf' }), null), null);
});

test('ensureSheet: the first piece binds the collage to the part\'s file; an item without one gets none', () => {
	const full = item();
	const sheet = ensureSheet(full, null);
	assert.deepEqual(full.score.collage, { file: FILE, pieces: [] });
	assert.equal(ensureSheet(full, null), sheet, 'made once');
	const choir = item({ musicxml: FILE, parts: { choir: { musicxml: CHOIR } } });
	ensureSheet(choir, 'choir');
	assert.deepEqual((choir.score.parts as any).choir.collage, { file: CHOIR, pieces: [] });
	assert.equal((choir.score as any).collage, undefined, 'the full score untouched');
	const none = item({ pdf: 'x.pdf' });
	assert.equal(ensureSheet(none, null), null);
	assert.equal(addScorePiece(none, null, piece('piece-a')), null);
	assert.deepEqual(none.score, { pdf: 'x.pdf' }, 'nothing written without a file');
});

test('a new piece: the root path, a fresh id, its centre at the place, staff spaces wide; added at the front', () => {
	const place = centrePlace(geo)!;
	assert.ok(place && place.at.bar, 'the middle of the page names a bar');
	const fresh = newScorePiece(shape, { at: { bar: '2', staff: 2 }, x: 25, y: 1.5 }, 12, new Set(['piece-aaaaaaa']));
	assert.match(fresh.id, /^piece-[a-z0-9]{7}$/);
	assert.equal(fresh.src, 'catpope/vectors/wreath.svg', 'a catalog row stores the CDN-root path');
	assert.deepEqual([fresh.anchor, fresh.rect, fresh.at, fresh.ratio], ['c', { x: 25, y: 1.5, width: 12 }, { bar: '2', staff: 2 }, 2]);
	const doc = item();
	assert.equal(addScorePiece(doc, null, piece('piece-a')), 0);
	assert.equal(addScorePiece(doc, null, fresh), 1);
	assert.deepEqual(sheetPieces(doc, null).map((p) => p.id), ['piece-a', fresh.id]);
});

test('move = a new bar / staff and offsets (at replaced whole); the corner\'s distance from the centre is the half diagonal', () => {
	const p = piece('piece-a', { at: { bar: '1', staff: 2, nth: 2 } });
	movePiece(p, { at: { bar: '2' }, x: 10, y: -1 });
	assert.deepEqual([p.at, p.rect], [{ bar: '2' }, { x: 10, y: -1, width: 12 }]);
	// ratio 2: width 40 units → height 20; the half diagonal √(20² + 10²) – a pointer there gives 40 units = 4 sp
	const centre = { x: 50, y: 50 };
	const corner = { x: 50 + (20 / geo.width) * 100, y: 50 + (10 / geo.height) * 100 };
	assert.equal(cornerWidth(p, centre, corner, geo), 4);
	assert.equal(cornerWidth({ ...p, at: { bar: '2', page: true } }, centre, corner, geo), 4, 'a page piece: % of the page (40 of 1000)');
	assert.equal(cornerWidth(p, centre, centre, geo), 0.5, 'never below the smallest width');
});

test("the scale handle: the box's centre and its own bottom-right corner (anchor, rotation) in % of the page", () => {
	// 4 % of 1000 = 40 units wide, ratio 2 → 20 high, its anchor at (500, 750)
	const place = { left: 50, top: 50, width: 4 };
	const near = (a: { x: number; y: number }, b: { x: number; y: number }) => assert.ok(Math.hypot(a.x - b.x, a.y - b.y) < 1e-9, JSON.stringify({ a, b }));
	const c = pieceHandle({ ratio: 2, anchor: 'c' }, place, geo);
	near(c.centre, { x: 50, y: 50 });
	near(c.corner, { x: 52, y: (760 / 1500) * 100 });
	const tl = pieceHandle({ ratio: 2, anchor: 'tl' }, place, geo);
	near(tl.centre, { x: 52, y: (760 / 1500) * 100 });
	near(tl.corner, { x: 54, y: (770 / 1500) * 100 });
	// turned 90° clockwise: the corner (20, 10) from the centre goes to (−10, 20)
	const turned = pieceHandle({ ratio: 2, anchor: 'c', rotation: 90 }, place, geo);
	near(turned.corner, { x: 49, y: (770 / 1500) * 100 });
	// the corner is where the corner drag reads the width back: 40 units = 4 sp
	assert.equal(cornerWidth(piece('piece-a'), turned.centre, turned.corner, geo), 4);
});

test('removing a piece; the last one takes the collage away', () => {
	const doc = item();
	addScorePiece(doc, null, piece('piece-a'));
	addScorePiece(doc, null, piece('piece-b'));
	assert.equal(removeScorePiece(doc, null, 'piece-x'), false);
	assert.equal(removeScorePiece(doc, null, 'piece-a'), true);
	assert.deepEqual(sheetPieces(doc, null).map((p) => p.id), ['piece-b']);
	assert.equal(removeScorePiece(doc, null, 'piece-b'), true);
	assert.equal('collage' in doc.score, false, 'no empty collage stays');
	assert.equal(removeScorePiece(doc, null, 'piece-b'), false);
});

test('Bar / Page keeps the piece where it is on its page', () => {
	const p = piece('piece-a', { at: { bar: '2', staff: 2 }, rect: { x: 50, y: 2, width: 12 } });
	const before = placeScorePiece(p, geo)!;
	setPiecePage(p, true, geo);
	assert.equal(p.at?.page, true);
	assert.equal(p.at?.staff, undefined, 'a page piece has no staff');
	const onPage = placeScorePiece(p, geo)!;
	for (const k of ['left', 'top', 'width'] as const) assert.ok(Math.abs(onPage[k] - before[k]) < 0.01, `${k}: ${onPage[k]} ≠ ${before[k]}`);
	setPiecePage(p, false, geo);
	assert.equal(p.at?.page, undefined);
	assert.deepEqual([p.at?.bar, p.at?.staff], ['2', 2], 'back on the bar and staff under it');
	const back = placeScorePiece(p, geo)!;
	for (const k of ['left', 'top', 'width'] as const) assert.ok(Math.abs(back[k] - before[k]) < 0.01, `${k}: ${back[k]} ≠ ${before[k]}`);
	// without the page at hand only the flag changes
	const q = piece('piece-b');
	setPiecePage(q, true, null);
	assert.deepEqual([q.at, q.rect], [{ bar: '1', page: true }, { x: 50, y: 2, width: 12 }]);
	setPiecePage(q, false, null);
	assert.deepEqual(q.at, { bar: '1' });
});

test('the list: over the notes front first, then behind them; a drag reorders within its side only', () => {
	// back → front: a (over), b (behind), c (over), d (behind)
	const pieces = [piece('a'), piece('b', { behind: true }), piece('c'), piece('d', { behind: true })];
	const rows = collageRows(pieces);
	assert.deepEqual(rows.map((r) => pieces[r.index].id), ['c', 'a', 'd', 'b']);
	assert.equal(canMoveCollageRow(rows, 0, 1), true);
	assert.equal(canMoveCollageRow(rows, 1, 2), false, 'across the sides');
	assert.equal(moveCollageRows(pieces, 1, 2), null);
	// a to the front of the pieces over the notes: c behind it now; the behind side keeps its places
	assert.deepEqual(moveCollageRows(pieces, 1, 0)!.map((p) => p.id), ['c', 'b', 'a', 'd']);
	assert.deepEqual(moveCollageRows(pieces, 2, 3)!.map((p) => p.id), ['a', 'd', 'c', 'b']);
});

test('the stage\'s problems: a bar the file lacks, a replaced file, a broken piece – at their paths in the item', () => {
	const doc = item();
	addScorePiece(doc, null, piece('piece-a'));
	addScorePiece(doc, null, piece('piece-b', { at: { bar: '99' } }));
	const bars = geo.bars.map(({ id, n, nth }) => ({ id, n, nth }));
	assert.deepEqual([...missingBars(sheetPieces(doc, null), bars)], ['piece-b']);
	assert.equal(missingBars(sheetPieces(doc, null), null).size, 0, 'the bars not read yet: none missing');
	const missing = (bar: string) => `bar ${bar} missing`;
	let problems = stageProblems(doc, null, bars, 'item:cloud9_en', missing);
	assert.deepEqual(problems.map((p) => [p.level, p.path.join('.'), p.message]), [['warning', 'score.collage.pieces.1.at.bar', 'bar 99 missing']]);
	assert.equal(fileChanged(doc, null), false);
	// the score file replaced: the collage was placed on the old one
	doc.score.musicxml = 'scores/cloud9_en/new.musicxml';
	assert.equal(fileChanged(doc, null), true);
	(doc.score as any).collage.pieces[0].ratio = 0;
	problems = stageProblems(doc, null, bars, 'item:cloud9_en', missing);
	assert.ok(problems.some((p) => p.level === 'warning' && p.path.join('.') === 'score.collage.file'), 'made on another file');
	assert.ok(problems.some((p) => p.level === 'error' && p.path.join('.').startsWith('score.collage.pieces.0')), 'a broken piece');
	assert.ok(problems.every((p) => p.file === 'item:cloud9_en'));
});

test('the screens: the device scaled to fit the room both ways, the reader\'s start zoom and sizes there', () => {
	assert.deepEqual(SCREENS.phone, { width: 390, height: 844, root: 14, touch: true });
	assert.equal(screenScale(SCREENS.phone, { width: 780, height: 844 }), 1);
	assert.equal(screenScale(SCREENS.desktop, { width: 720, height: 810 }), 0.5);
	assert.equal(screenScale(SCREENS.phone, { width: 0, height: 500 }), 0, 'the room unknown yet');
	assert.deepEqual([screenZoom('phone'), screenZoom('desktop')], [30, 40]);
	// the reader's root there, not the creator's: a phone 14px with the 44px touch floor, a desktop 24px
	const phone = readerSizes(SCREENS.phone);
	assert.equal(phone['--control-height'], 'max(44px, calc(28px + 2 * var(--border-width-regular)))');
	assert.deepEqual([phone['--space-075'], phone['--space-1'], phone['--type-body-size'], phone['--type-caption-size']], ['10.5px', '14px', '14px', '10.5px']);
	const desktop = readerSizes(SCREENS.desktop);
	assert.equal(desktop['--control-height'], 'calc(48px + 2 * var(--border-width-regular))');
	assert.deepEqual([desktop['--space-05'], desktop['--space-075'], desktop['--space-1'], desktop['--space-3']], ['12px', '18px', '24px', '72px']);
});
