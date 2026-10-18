// Collage pieces: looks, style merge, sizes, weight, names, problems (agents/research/COLLAGE.md §2).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { BLEND_MODES, COLLAGE_LOOKS, collageLook, collageProblems, heavyWeight, pieceHeight, pieceKind, pieceName, pieceStyle, type CollagePiece } from '../src/index.ts';

const piece = (extra: Partial<CollagePiece> = {}): CollagePiece => ({ id: 'piece-abcdefg', src: 'vectors/wreath.svg', ratio: 0.5, rect: { x: 10, y: 10, width: 30 }, ...extra });
const problems = (pieces: unknown, host: 'slice' | 'score' = 'slice') => {
	const out: string[] = [];
	collageProblems(pieces, host, (level, path, message) => out.push(`${level} ${path.join('.')}: ${message}`));
	return out;
};
const where = (pieces: unknown, host: 'slice' | 'score' = 'slice') => problems(pieces, host).map((p) => p.split(':')[0]);

test('the 16 blend modes and the ten code looks (each valid as a piece style)', () => {
	assert.equal(BLEND_MODES.length, 16);
	assert.deepEqual(COLLAGE_LOOKS.map((l) => l.id), ['ink', 'red', 'cyan', 'negative', 'burn', 'acid', 'sticker', 'glow', 'ghost', 'fade']);
	for (const { id, ...style } of COLLAGE_LOOKS) assert.deepEqual(problems([piece({ style })]), [], id);
	assert.equal(collageLook('negative')?.blend, 'difference');
	assert.equal(collageLook('nope'), undefined);
	assert.equal(collageLook(3), undefined);
});

test('pieceStyle: the look under the own keys, a key replaced whole, null = off', () => {
	assert.deepEqual(pieceStyle(piece()), {});
	assert.deepEqual(pieceStyle(piece({ preset: 'negative' })), { fill: '#ffffff', blend: 'difference' });
	// a whole shadow replaces the look's, null switches the outline off; the id never leaks
	assert.deepEqual(pieceStyle(piece({ preset: 'sticker', style: { outline: null, shadow: { color: '#ff0000', x: 0, y: 4, blur: 1 } } })), {
		shadow: { color: '#ff0000', x: 0, y: 4, blur: 1 }
	});
	// an own value over a look, undefined = the look's
	assert.deepEqual(pieceStyle(piece({ preset: 'negative', style: { blend: 'normal', fill: undefined } })), { fill: '#ffffff', blend: 'normal' });
	// an unknown look = none
	assert.deepEqual(pieceStyle(piece({ preset: 'gone', style: { opacity: 0.4 } })), { opacity: 0.4 });
	assert.deepEqual(pieceStyle(null), {});
});

test('pieceStyle: only the six style keys, own ones – a stored __proto__ key sets no prototype', () => {
	const stored = JSON.parse('{"id":"piece-abcdefg","src":"vectors/wreath.svg","ratio":0.5,"rect":{"x":10,"y":10,"width":30},"style":{"__proto__":{"fill":"url(x)","blend":"plus-lighter"},"id":"x"}}');
	const style = pieceStyle(stored);
	assert.deepEqual(style, {});
	assert.equal(Object.getPrototypeOf(style), Object.prototype);
	assert.equal((style as Record<string, unknown>).fill, undefined);
	assert.deepEqual(problems([stored]), []);
	// unknown keys are dropped, the known ones kept
	assert.deepEqual(pieceStyle(piece({ style: { opacity: 0.5, id: 'x' } as never })), { opacity: 0.5 });
});

test('pieceHeight: the width in % of the box, a file of ratio w ÷ h, a box of h ÷ w', () => {
	assert.equal(pieceHeight(30, 0.5, 1.6), 37.5);
	// a square file on the 390 × 625 slice
	assert.equal(Math.round(pieceHeight(50, 1, 625 / 390) * 100) / 100, 31.2);
});

test('heavyWeight: outline 1, shadow 1, blur 1, a progressive blur 2', () => {
	assert.equal(heavyWeight(undefined), 0);
	assert.equal(heavyWeight({ fill: '#111111', blend: 'multiply' }), 0);
	assert.equal(heavyWeight(pieceStyle({ preset: 'sticker' })), 2);
	assert.equal(heavyWeight(pieceStyle({ preset: 'ghost' })), 1);
	assert.equal(heavyWeight(pieceStyle({ preset: 'fade' })), 2);
	assert.equal(heavyWeight({ outline: { color: '#fff', width: 0 }, blur: { amount: 0 } }), 0);
});

test('kind and name are derived from the file / the figure', () => {
	assert.equal(pieceKind(piece()), 'shape');
	assert.equal(pieceKind(piece({ src: '../characters/catpope/regular/default_portrait-happy.webp' })), 'image');
	assert.equal(pieceKind(piece({ src: 'x.webp', figure: { id: 'catpope', mood: 'happy', view: 'portrait' } })), 'character');
	assert.equal(pieceName(piece()), 'wreath');
	assert.equal(pieceName(piece({ src: 'images/1x/bg.2x.webp' })), 'bg');
	assert.equal(pieceName(piece({ figure: { id: 'catpope', mood: 'happy', view: 'full' } })), 'catpope · happy');
	assert.equal(pieceName(piece({ figure: { id: 'catpope', view: 'avatar' } })), 'catpope');
});

test('problems: a good slice piece has none; nothing / an empty list is fine, null is an error', () => {
	assert.deepEqual(problems(undefined), []);
	assert.deepEqual(problems([]), []);
	assert.deepEqual(problems([piece({ anchor: 'c', rotation: -12, preset: 'burn', style: { opacity: 0.7, blur: { amount: 4, from: { x: 0, y: 0 }, to: { x: 100, y: 100 } } } })]), []);
	assert.deepEqual(where({}), ['error ']);
	// null is no list (the server's schema refuses `pieces: null`)
	assert.deepEqual(problems(null), ['error : pieces must be a list']);
	assert.deepEqual(where([3]), ['error 0']);
});

test('problems: ids, file, ratio, rect, anchor, rotation, figure', () => {
	assert.deepEqual(where([piece({ id: '' }), piece(), piece()]), ['error 0.id', 'error 2.id']);
	assert.deepEqual(where([piece({ src: 'vectors/wreath' }), piece({ id: 'b', src: 'a.mp3' })]), ['error 0.src', 'error 1.src']);
	assert.deepEqual(where([piece({ ratio: 0 })]), ['error 0.ratio']);
	assert.deepEqual(where([piece({ rect: { x: 1, y: 2, width: 0 } })]), ['error 0.rect.width']);
	assert.deepEqual(where([piece({ rect: { x: 1, y: 2, width: 10, height: 10 } as never })]), ['error 0.rect.height']);
	assert.deepEqual(where([piece({ rect: { x: '1', y: 2 } as never })]), ['error 0.rect.x', 'error 0.rect.width']);
	assert.deepEqual(where([piece({ anchor: 'middle' as never, rotation: '9' as never })]), ['error 0.anchor', 'error 0.rotation']);
	assert.deepEqual(where([piece({ figure: { id: 'catpope', view: 'bust' as never } })]), ['error 0.figure.view']);
});

test('problems: style values', () => {
	const style = (s: object) => where([piece({ style: s as never })]);
	assert.deepEqual(style({ blend: 'plus-lighter' }), ['error 0.style.blend']);
	assert.deepEqual(style({ opacity: 1.2 }), ['error 0.style.opacity']);
	assert.deepEqual(style({ fill: 'red' }), ['error 0.style.fill']);
	assert.deepEqual(style({ fill: null, outline: null, shadow: null, blur: null }), []);
	assert.deepEqual(style({ fill: { stops: [{ at: 0, color: '#fff' }] } }), ['error 0.style.fill.stops']);
	assert.deepEqual(style({ fill: { angle: 90, stops: [{ at: 50, color: '#fff' }, { at: 120, color: '#000' }] } }), ['error 0.style.fill.stops.1.at']);
	assert.deepEqual(style({ fill: { stops: [{ at: 60, color: '#fff' }, { at: 20, color: '#000', opacity: 2 }] } }), ['error 0.style.fill.stops.1.at', 'error 0.style.fill.stops.1.opacity']);
	assert.deepEqual(style({ outline: { color: '#ffffff', width: -1 } }), ['error 0.style.outline.width']);
	assert.deepEqual(style({ shadow: { color: '#000', x: 0, y: 'a', blur: 2, opacity: 3 } }), ['error 0.style.shadow.y', 'error 0.style.shadow.opacity']);
	assert.deepEqual(style({ blur: { amount: 3, from: { x: 0, y: 0 } } }), ['error 0.style.blur']);
	assert.deepEqual(style({ blur: { amount: 3, from: { x: 0 }, to: { x: 0, y: 1 } } }), ['error 0.style.blur.from']);
	assert.deepEqual(where([piece({ style: 'ink' as never })]), ['error 0.style']);
	// effect sizes past the limits warn (% of the piece)
	assert.deepEqual(style({ outline: { color: '#fff', width: 9 }, shadow: { color: '#000', x: 0, y: 0, blur: 26 }, blur: { amount: 21 } }), [
		'warning 0.style.outline.width',
		'warning 0.style.shadow.blur',
		'warning 0.style.blur.amount'
	]);
});

test('problems: an unknown look, too many pieces, too heavy for phones', () => {
	assert.deepEqual(where([piece({ preset: 'mystery' })]), ['warning 0.preset']);
	const many = Array.from({ length: 13 }, (_, i) => piece({ id: `p${i}` }));
	assert.deepEqual(where(many), ['warning ']);
	assert.deepEqual(where(many.slice(0, 12)), []);
	// sticker (2) × 3 = 6 > 4 on a slice, fine on a score (20)
	const heavy = Array.from({ length: 3 }, (_, i) => piece({ id: `h${i}`, preset: 'sticker', at: { bar: '1' } }));
	assert.match(problems(heavy.map(({ at: _at, ...p }) => p))[0], /^warning : outlines, shadows and blurs weigh 6/);
	assert.deepEqual(problems(heavy, 'score'), []);
});

test('problems: hosts – at / behind only on scores, at { bar } required there', () => {
	assert.deepEqual(where([piece({ at: { bar: '3' }, behind: true })]), ['error 0.at', 'error 0.behind']);
	const score = (extra: Partial<CollagePiece>) => where([piece({ anchor: 'c', ...extra })], 'score');
	assert.deepEqual(score({ at: { bar: '6a', nth: 2, staff: 1, page: false }, behind: true }), []);
	assert.deepEqual(score({}), ['error 0.at']);
	assert.deepEqual(score({ at: { bar: '' } }), ['error 0.at.bar']);
	assert.deepEqual(score({ at: { bar: 12 as never } }), ['error 0.at.bar']);
	assert.deepEqual(score({ at: { bar: '1', nth: 0, staff: 1.5, page: 'yes' as never }, behind: 'no' as never }), ['error 0.at.nth', 'error 0.at.staff', 'error 0.at.page', 'error 0.behind']);
});
