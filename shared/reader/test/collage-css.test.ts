import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { CollagePiece } from '@scutoons/content';
import { blurBands, cssUrl, drawablePiece, fillCss, filterCss, gradientLine, pieceBox, pieceCss, spill, styleText, withAlpha } from '../src/collage/collage-css.ts';

const piece = (over: Partial<CollagePiece> = {}): CollagePiece => ({ id: 'piece-abcdefg', src: 'vectors/wreath.svg', ratio: 2, rect: { x: 10, y: 20, width: 30 }, ...over });

test('colours: the hex as written when opaque, rgb(r g b / a) under 1', () => {
	assert.equal(withAlpha('#ff1900'), '#ff1900');
	assert.equal(withAlpha('#ff1900', 1), '#ff1900');
	assert.equal(withAlpha('#ff1900', 0.8), 'rgb(255 25 0 / 0.8)');
	assert.equal(withAlpha('#0af', 0.5), 'rgb(0 170 255 / 0.5)');
	assert.equal(withAlpha('#111111', 0), 'rgb(17 17 17 / 0)');
	// not a hex (the validator names it): as written
	assert.equal(withAlpha('red', 0.5), 'red');
});

test('fill: a colour, one linear gradient (default 180deg, stop opacity), none = as drawn', () => {
	assert.equal(fillCss('#111111'), '#111111');
	assert.equal(fillCss(undefined), null);
	assert.equal(fillCss({ stops: [{ at: 0, color: '#ff1900' }, { at: 100, color: '#b300ff' }] }), 'linear-gradient(180deg, #ff1900 0%, #b300ff 100%)');
	assert.equal(
		fillCss({ angle: 45, stops: [{ at: 0, color: '#00d1ff', opacity: 0.25 }, { at: 62.5, color: '#37ff00' }] }),
		'linear-gradient(45deg, rgb(0 209 255 / 0.25) 0%, #37ff00 62.5%)'
	);
	// a broken gradient draws what it can
	assert.equal(fillCss({ stops: [{ at: 0, color: '#ff1900' }] }), '#ff1900');
	assert.equal(fillCss({ stops: [] }), null);
});

test('filter order: outline ×4 → shadow → blur, lengths in cqw', () => {
	const css = filterCss({ outline: { color: '#ffffff', width: 3 }, shadow: { color: '#111111', x: 2, y: 2, blur: 0, opacity: 0.8 }, blur: { amount: 5 } })!;
	assert.equal(
		css,
		'drop-shadow(3cqw 0 0 #ffffff) drop-shadow(-3cqw 0 0 #ffffff) drop-shadow(0 3cqw 0 #ffffff) drop-shadow(0 -3cqw 0 #ffffff) ' +
			'drop-shadow(2cqw 2cqw 0 rgb(17 17 17 / 0.8)) blur(5cqw)'
	);
	const parts = css.match(/(drop-shadow|blur)\(/g)!;
	assert.deepEqual(parts, ['drop-shadow(', 'drop-shadow(', 'drop-shadow(', 'drop-shadow(', 'drop-shadow(', 'blur(']);
	assert.equal(filterCss({ shadow: { color: '#00d1ff', x: 0, y: 0, blur: 6 } }), 'drop-shadow(0 0 6cqw #00d1ff)');
	assert.equal(filterCss({ blur: { amount: 3 } }), 'blur(3cqw)');
	// nothing (or zero) = no filter at all: no group of its own
	assert.equal(filterCss({}), null);
	assert.equal(filterCss({ outline: { color: '#fff', width: 0 }, blur: { amount: 0 } }), null);
});

test('gradient line: top → bottom = 180deg 0 % / 100 % on any box', () => {
	for (const ratio of [0.5, 1, 2.4]) assert.deepEqual(gradientLine({ x: 50, y: 0 }, { x: 50, y: 100 }, ratio), { angle: 180, start: 0, end: 100 });
	assert.deepEqual(gradientLine({ x: 50, y: 25 }, { x: 50, y: 75 }, 1.5), { angle: 180, start: 25, end: 75 });
	assert.deepEqual(gradientLine({ x: 50, y: 100 }, { x: 50, y: 0 }, 1), { angle: 0, start: 0, end: 100 });
	assert.deepEqual(gradientLine({ x: 0, y: 50 }, { x: 100, y: 50 }, 0.5), { angle: 90, start: 0, end: 100 });
});

test('gradient line: a diagonal follows the box (ratio 0.5 = twice as tall as wide)', () => {
	// corner to corner: CSS angle atan2(w, h) from the top, the corners at 0 % / 100 % (CSS's own line length)
	const line = gradientLine({ x: 0, y: 0 }, { x: 100, y: 100 }, 0.5);
	assert.equal(line.angle, 153.435); // 180 − atan(0.5) in degrees
	assert.equal(line.start, 0);
	assert.equal(line.end, 100);
	// on a square box the same points are 135deg
	assert.equal(gradientLine({ x: 0, y: 0 }, { x: 100, y: 100 }, 1).angle, 135);
	// a short diagonal in the middle: symmetric around 50 %
	const mid = gradientLine({ x: 25, y: 25 }, { x: 75, y: 75 }, 0.5);
	assert.equal(mid.angle, 153.435);
	assert.equal(mid.start, 25);
	assert.equal(mid.end, 75);
	// one point twice: straight down, one position
	assert.deepEqual(gradientLine({ x: 50, y: 40 }, { x: 50, y: 40 }, 1), { angle: 180, start: 40, end: 40 });
});

test('blur bands: none without a ramp; the pad = 3 × the blur, the line shifted into the padded box', () => {
	assert.equal(blurBands({ blur: { amount: 3 } }, 1), null);
	assert.equal(blurBands({ blur: { amount: 0, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } } }, 1), null);
	assert.equal(blurBands({}, 1), null);

	const bands = blurBands({ blur: { amount: 5, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } } }, 1)!;
	assert.equal(bands.pad, 15);
	// a square piece padded by 15 % of its width on every side: 130 × 130; the top 0 % lies at 15 / 130
	assert.equal(bands.angle, 180);
	assert.equal(bands.start, 11.538);
	assert.equal(bands.end, 88.462);
	assert.equal(bands.sharp, 'linear-gradient(180deg, #000 11.538%, transparent 88.462%)');
	assert.equal(bands.soft, 'linear-gradient(180deg, transparent 11.538%, #000 88.462%)');

	// a wide piece (ratio 2): the padded box is 1.3 × 0.8 piece widths – the stops move further in vertically
	const wide = blurBands({ blur: { amount: 5, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } } }, 2)!;
	assert.equal(wide.start, round((0.15 / 0.8) * 100));
	assert.equal(wide.end, round((0.65 / 0.8) * 100));

	// left → right on a tall piece: 90deg, the stops at the pad over the padded width
	const side = blurBands({ blur: { amount: 2, from: { x: 0, y: 50 }, to: { x: 100, y: 50 } } }, 0.5)!;
	assert.equal(side.pad, 6);
	assert.equal(side.angle, 90);
	assert.equal(side.start, round((0.06 / 1.12) * 100));
	assert.equal(side.end, round((1.06 / 1.12) * 100));
});

test('blur bands: the outline and the shadow widen the pad (a mask must not cut them)', () => {
	assert.equal(spill({ outline: { color: '#fff', width: 3 } }), 3);
	assert.equal(spill({ shadow: { color: '#111', x: 2, y: -4, blur: 2 } }), 4 + 3);
	assert.equal(spill({ blur: { amount: 6 } }), 18);
	const sticker = blurBands({ outline: { color: '#fff', width: 3 }, shadow: { color: '#111', x: 2, y: 2, blur: 0 }, blur: { amount: 6, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } } }, 1)!;
	assert.equal(sticker.pad, 3 + 2 + 18);
});

test('the piece box: % of the host, the height from the ratio, anchor translate, rotation', () => {
	assert.deepEqual(pieceBox(piece(), 10, 20, 30), { left: '10%', top: '20%', width: '30%', 'aspect-ratio': '2' });
	assert.deepEqual(pieceBox(piece({ anchor: 'c', rotation: -12.5 }), 50, 50, 30), { left: '50%', top: '50%', width: '30%', 'aspect-ratio': '2', translate: '-50% -50%', rotate: '-12.5deg' });
	assert.equal(pieceBox(piece({ ratio: 1 / 3 }), 0, 0, 10)['aspect-ratio'], '0.333');
	assert.equal(styleText({ left: '1%', translate: null, top: '', opacity: '0.5' }), 'left: 1%; opacity: 0.5');
});

test('pieceCss: the look under the own keys – blend, opacity, fill, filter; a ramp gets two bands', () => {
	const negative = pieceCss(piece({ preset: 'negative' }), 10, 20, 30);
	assert.equal(negative.box, 'left: 10%; top: 20%; width: 30%; aspect-ratio: 2; mix-blend-mode: difference');
	assert.equal(negative.fill, '#ffffff');
	assert.equal(negative.filter, null);
	assert.equal(negative.bands, null);

	// an own key replaces the look's (shadow off, another blend), opacity and z on the box
	const own = pieceCss(piece({ preset: 'sticker', style: { shadow: null, blend: 'multiply', opacity: 0.4 } }), 0, 0, 10, 2);
	assert.equal(own.box, 'left: 0%; top: 0%; width: 10%; aspect-ratio: 2; mix-blend-mode: multiply; opacity: 0.4; z-index: 2');
	assert.equal(own.fill, null);
	assert.equal(own.filter, 'drop-shadow(3cqw 0 0 #ffffff) drop-shadow(-3cqw 0 0 #ffffff) drop-shadow(0 3cqw 0 #ffffff) drop-shadow(0 -3cqw 0 #ffffff)');

	// fade: the sharp band without the blur, the soft band with it
	const fade = pieceCss(piece({ preset: 'fade' }), 0, 0, 10);
	assert.equal(fade.filter, null);
	assert.equal(fade.bands?.filter, 'blur(6cqw)');
	assert.equal(fade.bands?.pad, 18);
	// normal blend and full opacity write nothing
	assert.equal(pieceCss(piece({ style: { blend: 'normal', opacity: 1 } }), 0, 0, 10).box, 'left: 0%; top: 0%; width: 10%; aspect-ratio: 2');
});

test('drawable pieces and quoted URLs', () => {
	assert.ok(drawablePiece(piece()));
	assert.ok(!drawablePiece(null));
	assert.ok(!drawablePiece(piece({ ratio: 0 })));
	assert.ok(!drawablePiece(piece({ src: '' })));
	assert.ok(!drawablePiece({ ...piece(), rect: { x: 1, y: 2 } }));
	assert.ok(!drawablePiece(piece({ rect: { x: 1, y: 2, width: 0 } })));
	assert.equal(cssUrl('http://localhost:8001/catpope/vectors/wreath.svg'), 'url("http://localhost:8001/catpope/vectors/wreath.svg")');
	assert.equal(cssUrl('/@fs/C:/x/a"b.svg'), 'url("/@fs/C:/x/a\\"b.svg")');
});

function round(n: number) {
	return Math.round(n * 1000) / 1000;
}
