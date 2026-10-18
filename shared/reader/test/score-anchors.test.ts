import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { CollagePiece } from '@scutoons/content';
import { anchorScorePiece, barIndex, findBar, pageFrame, pageGeometry, placeScorePiece, type PageGeometry } from '../src/score/score-anchors.ts';
import { ANCHOR_ATTRIBUTES, margin, scoreOptions, stepZoom, ZOOMS } from '../src/score/score-layout.ts';
import type { ScoreLook } from '../src/score/score-looks.ts';

// cloud9_en page 1 from Verovio 6.3.0, laid out as the score view does (scoreOptions: a 390 × 700 phone sheet at
// zoom 30, no look): two systems (bars 1–2, 3–5) of four staves, the page margin translate(400, 400)
const FIXTURE = readFileSync(new URL('./fixtures/score-page.svg', import.meta.url), 'utf8');
const geo = pageGeometry(FIXTURE, [])!;

const near = (actual: number, expected: number, within: number, what: string) =>
	assert.ok(Math.abs(actual - expected) <= within, `${what}: ${actual} ≠ ${expected} (± ${within})`);

const piece = (at: CollagePiece['at'], rect: CollagePiece['rect']): Pick<CollagePiece, 'rect' | 'at'> => ({ at, rect });

test('pageGeometry reads a real Verovio page: viewBox, the margin added, bars and staves, sp from the lines', () => {
	assert.equal(geo.width, 13000);
	assert.equal(geo.height, 23330);
	assert.equal(geo.sp, 180);
	assert.deepEqual(geo.bars.map((b) => b.n), ['1', '2', '3', '4', '5']);
	const [one, two, three] = geo.bars;
	// bar 1's lines run from x 1713 at y 2400 in the page-margin group: + 400 / + 400
	assert.deepEqual([one.left, one.staves[0].top, one.staves[0].bottom], [2113, 2800, 3520]);
	assert.deepEqual(one.staves.map((s) => s.n), [1, 2, 3, 4]);
	// bars of one system share their staves; the next bar starts where the last ends
	assert.deepEqual(two.staves, one.staves);
	assert.equal(two.left, one.right);
	assert.ok(three.staves[0].top > one.staves[3].bottom, 'the second system lies below the first');
	// an index from the toolkit's MEI names the bars (here: bar 1 as the 2nd bar numbered "1"); unknown ids count by data-n
	const indexed = pageGeometry(FIXTURE, [{ id: one.id, n: '1', nth: 2 }])!;
	assert.deepEqual([indexed.bars[0].n, indexed.bars[0].nth, indexed.bars[1].nth], ['1', 2, 1]);
	assert.equal(pageGeometry('<svg><g/></svg>', []), null, 'not a Verovio page');
});

test('the frame: Verovio rounds the element up to whole pixels and centres the viewBox in it – the layer takes the viewBox', () => {
	// 390 × 700 px for a 13000 × 23330 viewBox: 0.03 px per unit, 699.9 px of music – 0.05 px above it (% of the width)
	near(geo.frame.x, 0, 1e-9, 'x');
	near(geo.frame.width, 100, 1e-9, 'width');
	near(geo.frame.y, (0.05 / 390) * 100, 1e-9, 'y');
	assert.deepEqual(pageFrame(geo), { width: '100%', 'aspect-ratio': '13000 / 23330', 'margin-left': '0%', 'margin-top': '0.0128%' });
	// a viewBox narrower than its element: centred across, in % of the element's width
	const narrow = pageGeometry('<svg width="400px" height="100px"><svg class="definition-scale" viewBox="0 0 2000 1000"></svg></svg>', [])!;
	assert.deepEqual(narrow.frame, { x: 25, y: 0, width: 50 });
	// no element size: the viewBox is the box
	assert.deepEqual(pageGeometry('<svg><svg class="definition-scale" viewBox="0 0 100 100"></svg></svg>', [])!.frame, { x: 0, y: 0, width: 100 });
});

test('barIndex: every measure in file order, the n-th bar of a number', () => {
	const mei = `<mei><music><body><mdiv><score><section>
		<measure xml:id="m1" n="4"><staff n="1"/></measure>
		<measure n="5" xml:id="m2" right="rptend"></measure>
		<measure xml:id="m3" n="5"/>
		<measure xml:id="m4" n="6a"></measure>
		<measure n="7"></measure>
		<measureSpace xml:id="x"/>
	</section></score></mdiv></body></music></mei>`;
	const bars = barIndex(mei);
	assert.deepEqual(bars, [
		{ id: 'm1', n: '4', nth: 1 },
		{ id: 'm2', n: '5', nth: 1 },
		{ id: 'm3', n: '5', nth: 2 },
		{ id: 'm4', n: '6a', nth: 1 }
	]);
	assert.equal(findBar(bars, { bar: '5', nth: 2 })?.id, 'm3');
	assert.equal(findBar(bars, { bar: ' 5 ' })?.id, 'm2', 'nth defaults to 1, the number trimmed');
	assert.equal(findBar(bars, { bar: '5', nth: 3 }), undefined);
	assert.equal(findBar(bars, null), undefined);
});

test('a music piece: % of its bar, staff spaces below the staff top line, width in staff spaces', () => {
	const two = geo.bars[1];
	const place = placeScorePiece(piece({ bar: '2' }, { x: 50, y: 2, width: 6 }), geo)!;
	near(place.left, ((two.left + 0.5 * (two.right - two.left)) / 13000) * 100, 1e-9, 'left');
	near(place.top, ((2800 + 2 * 180) / 23330) * 100, 1e-9, 'top');
	near(place.width, ((6 * 180) / 13000) * 100, 1e-9, 'width');
	// staff 3 of bar 4 (second system)
	const four = geo.bars[3];
	const low = placeScorePiece(piece({ bar: '4', staff: 3 }, { x: 0, y: -1, width: 4 }), geo)!;
	near(low.left, (four.left / 13000) * 100, 1e-9, 'left of bar 4');
	near(low.top, ((four.staves[2].top - 180) / 23330) * 100, 1e-9, 'one space above staff 3');
	// a page piece: % of the page that holds the bar
	assert.deepEqual(placeScorePiece(piece({ bar: '3', page: true }, { x: 10, y: 20, width: 80 }), geo), { left: 10, top: 20, width: 80 });
});

test('a missing staff falls back to the bar’s top staff; a bar not on the page or not in the file → null', () => {
	const top = placeScorePiece(piece({ bar: '1' }, { x: 0, y: 0, width: 1 }), geo)!;
	assert.deepEqual(placeScorePiece(piece({ bar: '1', staff: 7 }, { x: 0, y: 0, width: 1 }), geo), top);
	assert.equal(placeScorePiece(piece({ bar: '99' }, { x: 0, y: 0, width: 1 }), geo), null);
	assert.equal(placeScorePiece(piece({ bar: '1', nth: 2 }, { x: 0, y: 0, width: 1 }), geo), null);
	assert.equal(placeScorePiece(piece(undefined, { x: 0, y: 0, width: 1 }), geo), null);
	// a condensed system leaves staff 1 out: the bar's top staff is staff 2
	const condensed = `<svg width="100px" height="100px"><svg class="definition-scale" viewBox="0 0 1000 1000"><g class="page-margin" transform="translate(10, 20)">
		<g class="system"><g id="b9" class="measure" data-n="9"><g id="s2" class="staff" data-n="2">
		<path d="M100 300 L900 300" /><path d="M100 318 L900 318" /><path d="M100 336 L900 336" /><path d="M100 354 L900 354" /><path d="M100 372 L900 372" />
		<g class="layer"><path d="M120 200 L140 200" /></g></g></g></g></g></svg></svg>`;
	const g = pageGeometry(condensed, [])!;
	assert.deepEqual(g.bars[0].staves, [{ n: 2, top: 320, bottom: 392 }]);
	assert.equal(g.sp, 18, 'sp from this page’s lines; the ledger-like path in the layer is no staff line');
	const place = placeScorePiece(piece({ bar: '9' }, { x: 0, y: 1, width: 10 }), g)!;
	near(place.left, 11, 1e-9, 'left');
	near(place.top, 33.8, 1e-9, 'top: one space below the top line of staff 2');
	near(place.width, 18, 1e-9, 'width');
});

test('anchorScorePiece → placeScorePiece round-trips within 0.01 % of the page', () => {
	const points = [
		{ x: 40, y: 13 }, // bar 1, staff 1
		{ x: 80, y: 47 }, // bar 2, around staff 4
		{ x: 55, y: 75 }, // bar 4, staff 3
		{ x: 3, y: 62 }, // in the left margin of the second system
		{ x: 98, y: 52 }, // between the systems, at the right edge
		{ x: 50, y: 99 } // below the last staff
	];
	for (const point of points) {
		for (const page of [false, true]) {
			const anchored = anchorScorePiece(point, geo, page)!;
			const place = placeScorePiece(piece(anchored.at, { x: anchored.x, y: anchored.y, width: 5 }), geo)!;
			near(place.left, point.x, 0.01, `left of ${JSON.stringify(point)} (page ${page})`);
			near(place.top, point.y, 0.01, `top of ${JSON.stringify(point)} (page ${page})`);
		}
	}
	// the bar whose system holds the point, the nearest staff; staff 1 / nth 1 are not written
	assert.deepEqual(anchorScorePiece({ x: 40, y: 13 }, geo)!.at, { bar: '1' });
	assert.deepEqual(anchorScorePiece({ x: 55, y: 79 }, geo)!.at, { bar: '4', staff: 3 });
	assert.deepEqual(anchorScorePiece({ x: 3, y: 62 }, geo)!.at, { bar: '3' });
	assert.deepEqual(anchorScorePiece({ x: 50, y: 99 }, geo, true)!.at, { bar: '4', page: true });
	const empty: PageGeometry = { width: 100, height: 100, sp: 18, bars: [], frame: { x: 0, y: 0, width: 100 } };
	assert.equal(anchorScorePiece({ x: 5, y: 5 }, empty), null);
});

test('scoreOptions: a page of the sheet’s size, a look’s attributes kept and the anchors’ added, margins in rem', () => {
	const plain = scoreOptions(390, 700, 30, null, 16);
	assert.equal(plain.pageWidth, 1300);
	assert.equal(plain.pageHeight, 2333);
	assert.equal(plain.scale, 30);
	assert.equal(plain.adjustPageHeight, false);
	assert.equal(plain.pageMarginTop, 40);
	assert.deepEqual(plain.svgAdditionalAttribute, ANCHOR_ATTRIBUTES);
	const look: ScoreLook = { id: 'colour', options: { font: 'Gootville', svgAdditionalAttribute: ['note@pname', 'staff@n'] }, margin: { top: 3, side: 1.25, bottom: 2 } };
	const styled = scoreOptions(1280, 720, 40, look, 16);
	assert.equal(styled.font, 'Gootville');
	assert.deepEqual(styled.svgAdditionalAttribute, ['note@pname', 'staff@n', 'measure@n']);
	assert.deepEqual([styled.pageMarginTop, styled.pageMarginLeft, styled.pageMarginRight, styled.pageMarginBottom], [120, 50, 50, 80]);
	assert.deepEqual(look.options.svgAdditionalAttribute, ['note@pname', 'staff@n'], 'the look itself is unchanged');
	assert.equal(margin(3, 30, 16), 160);
	assert.equal(margin(undefined, 30, 16), 40);
	assert.equal(stepZoom(40, 1), 45);
	assert.equal(stepZoom(ZOOMS[0], -1), ZOOMS[0]);
	assert.equal(stepZoom(ZOOMS[ZOOMS.length - 1], 1), ZOOMS[ZOOMS.length - 1]);
	assert.equal(stepZoom(33, -1), 35, 'an unknown zoom steps from the default');
});
