import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { CollagePiece } from '@scutoons/content';
import { anchorScorePiece, pageFrame, pageGeometry, placeScorePiece, type PageGeometry } from '../src/score/score-anchors.ts';
import { barIndex, barTime, barX, findBar, type Beat } from '../src/score/score-bars.ts';
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
	assert.deepEqual(
		bars.map(({ id, n, nth }) => ({ id, n, nth })),
		[
			{ id: 'm1', n: '4', nth: 1 },
			{ id: 'm2', n: '5', nth: 1 },
			{ id: 'm3', n: '5', nth: 2 },
			{ id: 'm4', n: '6a', nth: 1 }
		]
	);
	assert.equal(findBar(bars, { bar: '5', nth: 2 })?.id, 'm3');
	assert.equal(findBar(bars, { bar: ' 5 ' })?.id, 'm2', 'nth defaults to 1, the number trimmed');
	assert.equal(findBar(bars, { bar: '5', nth: 3 }), undefined);
	assert.equal(findBar(bars, null), undefined);
});

test('barIndex: when each note and rest starts, in parts of the bar (Verovio’s MusicXML import: dur.ppq)', () => {
	// staff 1 counts 2 pulses a quarter, staff 2 four; a chord's notes start with it, grace notes take no time, a
	// space counts but is no event, beams / tuplets only group; the bar lasts as long as its longest layer
	const mei = `<mei><music><body><mdiv><score>
		<scoreDef><staffGrp><staffDef n="1" ppq="2"/><staffDef n="2" ppq="4"/></staffGrp></scoreDef>
		<section><measure xml:id="b1" n="1">
			<staff n="1"><layer n="1">
				<note xml:id="a" dur.ppq="2" dur="4"/>
				<beam><note xml:id="b" dur.ppq="1" dur="8"/><note xml:id="c" dur.ppq="1" dur="8"/></beam>
				<graceGrp><note xml:id="g1" grace="unacc" dur="16"/></graceGrp>
				<note xml:id="g2" grace="acc" dur="8"/>
				<chord xml:id="ch" dur.ppq="2" dur="4"><note xml:id="c1"/><note xml:id="c2"/></chord>
				<space dur.ppq="1" dur="8"/>
				<rest xml:id="r" dur.ppq="1" dur="8"/>
			</layer><layer n="2"><note xml:id="v2" dur.ppq="4" dur="2"/></layer></staff>
			<staff n="2"><layer n="1">
				<tuplet num="3" numbase="2"><note xml:id="t1" dur.ppq="2" dur="8"/><note xml:id="t2" dur.ppq="2" dur="8"/><note xml:id="t3" dur.ppq="2" dur="8"/></tuplet>
				<note xml:id="h" dur.ppq="10" dur="2"/>
			</layer></staff>
		</measure>
		<scoreDef><staffGrp><staffDef n="1" ppq="4"/></staffGrp></scoreDef>
		<measure xml:id="b2" n="2">
			<staff n="1"><layer n="1"><note xml:id="q1" dur.ppq="4" dur="4"/><note xml:id="lost" dur="4"/><note xml:id="gone" dur.ppq="4" dur="4"/></layer>
			<layer n="2"><note xml:id="w" dur.ppq="16" dur="1"/></layer></staff>
			<staff n="2"><layer n="1"><mRest xml:id="mr"/></layer></staff>
		</measure>
		<measure xml:id="b3" n="3"><staff n="1"><layer n="1"><note xml:id="z" dur="4"/></layer></staff></measure>
	</section></score></mdiv></body></music></mei>`;
	const [one, two, three] = barIndex(mei);
	// bar 1 lasts four quarters: a quarter is 0.25 of it
	const q = (quarters: number) => quarters / 4;
	assert.deepEqual(Object.fromEntries(one.onsets!), { a: 0, b: q(1), c: q(1.5), ch: q(2), c1: q(2), c2: q(2), r: q(3.5), v2: 0, t1: 0, t2: q(0.5), t3: q(1), h: q(1.5) });
	assert.equal(one.rests!.size, 0);
	// a staffDef between the bars changes staff 1's pulse; an event without a length stops its layer's count there
	assert.deepEqual(Object.fromEntries(two.onsets!), { q1: 0, lost: 0.25, w: 0 });
	assert.deepEqual([...two.rests!], ['mr']);
	assert.equal(three.onsets!.size, 0, 'a bar of no length has no time');
});

test('a music piece in a bar without time (no bar index): % of its staff lines, staff spaces below the staff top line, width in staff spaces', () => {
	assert.deepEqual(geo.bars[1].beats, [{ t: 0, x: geo.bars[1].left }, { t: 1, x: geo.bars[1].right }]);
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

test('the bar’s time on the page: a beat per moment in the middle of its notehead, the barline at 1 – a line’s clef, key and time left out', () => {
	// bar 3 starts the second system: its staff lines begin at x 1142, its notes at 2531 / 3567 / 4316 / 5385 (+ the
	// margin) – a dotted quarter, an eighth, a dotted quarter, an eighth; staff 1 and 2 named, 3 and 4 left out (any staff
	// gives the moment)
	const three = geo.bars[2];
	const onsets = new Map<string, number>([
		['b1pw4nk8', 0], ['jmw21eo', 0.375], ['qlpjd5i', 0.5], ['y1b05l5o', 0.875],
		['m1hlh9nc', 0], ['tw6yp5a', 0.375], ['aczanpf', 0.5], ['ha7dwpd', 0.875]
	]);
	const timed = pageGeometry(FIXTURE, [{ id: three.id, n: '3', nth: 1, onsets }])!;
	const bar = timed.bars[2];
	const mid = 0.6 * 180;
	assert.deepEqual(bar.beats, [
		{ t: 0, x: 2531 + mid },
		{ t: 0.375, x: 3567 + mid },
		{ t: 0.5, x: 4316 + mid },
		{ t: 0.875, x: 5385 + mid },
		{ t: 1, x: three.right }
	]);
	assert.equal(three.left, 1142);
	assert.ok((bar.beats[0].x - bar.left) / 180 > 7, 'the clef, key and time signature lie before the first beat');
	// x = 0 is the first note, 37.5 the second, halfway in time between two notes = halfway between them
	const at = (x: number) => (placeScorePiece(piece({ bar: '3' }, { x, y: 0, width: 4 }), timed)!.left / 100) * 13000;
	near(at(0), 2531 + mid, 1e-6, 'the first note');
	near(at(37.5), 3567 + mid, 1e-6, 'the second note');
	near(at(43.75), (3567 + 4316) / 2 + mid, 1e-6, 'between the second and the third');
	// before the first note (the clef) and after the barline: the bar's mean pace
	const pace = (three.right - (2531 + mid)) / 1;
	near(at(-10), 2531 + mid - 0.1 * pace, 1e-6, 'over the clef');
	// the stage's pointer: anchored by the bar's time, placed back where it was
	for (const x of [1300, 2700, 4000, 5600, 5900]) {
		const anchored = anchorScorePiece({ x: (x / 13000) * 100, y: 65 }, timed)!;
		assert.equal(anchored.at.bar, '3');
		near(at(anchored.x), x, 13000 * 1e-4, `x ${x}`);
	}
	near(anchorScorePiece({ x: ((3567 + mid) / 13000) * 100, y: 65 }, timed)!.x, 37.5, 1e-9, 'on the second note: its moment');
});

test('barX / barTime: linear between the beats, the mean pace outside them, each the other’s inverse', () => {
	const beats: Beat[] = [{ t: 0, x: 100 }, { t: 0.5, x: 300 }, { t: 0.75, x: 500 }, { t: 1, x: 600 }];
	const bar = { beats };
	assert.equal(barX(bar, 0.25), 200);
	assert.equal(barX(bar, 0.625), 400);
	assert.equal(barX(bar, 1), 600);
	// mean pace: 500 per bar
	assert.equal(barX(bar, -0.1), 50);
	assert.equal(barX(bar, 1.2), 700);
	for (const t of [-0.3, 0, 0.1, 0.5, 0.7, 0.99, 1, 1.4]) near(barTime(bar, barX(bar, t)), t, 1e-12, `t ${t}`);
	// a single moment and the barline: one pace for all
	assert.equal(barX({ beats: [{ t: 0.5, x: 1000 }, { t: 1, x: 2000 }] }, 0), 0);
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
