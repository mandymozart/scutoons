// The variants board's layout (src/lib/board-layout.js, agents/research/VARIANTS-CANVAS.md §3.2 + §3.5): sources in
// column 0, a parent's results as one row, a generation's results one row down and one column right, the main input,
// the rightmost parent's rows first, records without a parent (the render by its inputs, the take, the earlier row),
// a loop in the parents, the lines (comb, ticks, drops), the card sizes, the keys across rows and bands, X's ids, the
// head's figures.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bandFacts, bandLayout, branchOf, CAPTION_W, cardSize, COMB, GAP_X, GAP_Y, LABELS, META, neighbour, PAD, PAD_T, PIC_H, ratioOf, rejectIds, shortId, statusOf, TOP, unrejectIds } from '../src/lib/board-layout.js';

const S = 'sb-25-2';
const CLEAN = { role: 'still', key: `out:s1c3/clean/${S}.png` };
/** a generated file as an input (its main input when it comes first – jobs.mjs inputsOf puts the main input first) */
const GEN = (id, role = 'still') => ({ role, key: `out:s1c3/generated/${id}.png` });
/** a record as GET /api/generated returns it (only what the layout reads); with parents, its inputs are those files */
const rec = (n, f = {}) => ({ id: `${S}--g${n}`, file: `${S}--g${n}.${f.video ? 'mp4' : 'png'}`, parents: [], inputs: f.parents ? f.parents.map((p) => GEN(p)) : [CLEAN], take: null, generator: 'runway', model: 'gen4_turbo', approved: false, mark: null, marked: null, before: null, earlier: false, ...f });
const at = (lay, key) => lay.nodes.find((n) => n.key === key);
const id = (n) => `${S}--g${n}`;

test('sizes: one picture height, the aspect, a caption wide at least', () => {
	assert.equal(ratioOf('9:16'), 9 / 16);
	assert.equal(ratioOf('1:3'), 1 / 3);
	assert.equal(ratioOf('nonsense'), 9 / 16);
	assert.deepEqual(cardSize('9:16'), { h: PIC_H, pw: PIC_H * 9 / 16, w: PIC_H * 9 / 16 });
	const narrow = cardSize('1:3');
	assert.equal(narrow.h, PIC_H);
	assert.equal(narrow.pw, 3);
	assert.equal(narrow.w, CAPTION_W);
	assert.equal(cardSize('2:1').w, 18);
	assert.equal(cardSize('9:16', 0.5).h, PIC_H / 2);
	assert.equal(shortId(id(7)), 'g7');
});

test('the status: chosen before the mark, review, rejected, none', () => {
	assert.equal(statusOf(rec(1, { approved: true, mark: 'review' })), 'chosen');
	assert.equal(statusOf(rec(1, { mark: 'review' })), 'review');
	assert.equal(statusOf(rec(1, { mark: 'rejected' })), 'rejected');
	assert.equal(statusOf(rec(1)), null);
	assert.equal(statusOf(null), null);
});

test('a source row: the render at column 0, its results oldest left, one row', () => {
	const lay = bandLayout({ shot: S, records: [rec(3), rec(1), rec(2)], aspect: '9:16' });
	const w = cardSize('9:16').w;
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3)]]);
	assert.equal(lay.rows[0].label, LABELS.source);
	assert.equal(at(lay, `render:${S}`).x, PAD);
	assert.equal(at(lay, id(1)).x, PAD + w + GAP_X);
	assert.equal(at(lay, id(3)).x, PAD + 3 * (w + GAP_X));
	assert.equal(at(lay, id(2)).parent, `render:${S}`);
	assert.equal(lay.width, at(lay, id(3)).x + w + PAD);
	assert.equal(lay.rows[0].y, PAD_T);
});

test('a generation\'s results: a row under it, one column right; the rightmost parent\'s rows first', () => {
	// g1, g2, g5 from the render; g7 + g12 from g5; g9 from g2; g13 from g7 (a grandchild); g10 from g5 AND g2 – its main input g5
	const records = [rec(1), rec(2), rec(5), rec(7, { parents: [id(5)] }), rec(9, { parents: [id(2)] }), rec(10, { parents: [id(5), id(2)] }), rec(12, { parents: [id(5)] }), rec(13, { parents: [id(7)] })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [
		[`render:${S}`, id(1), id(2), id(5)],
		[id(7), id(10), id(12)],
		[id(13)],
		[id(9)]
	]);
	const g5 = at(lay, id(5)), g7 = at(lay, id(7)), g2 = at(lay, id(2));
	assert.equal(g7.x, g5.x + g5.w + GAP_X);
	assert.equal(at(lay, id(13)).x, g7.x + g7.w + GAP_X);
	assert.equal(at(lay, id(9)).x, g2.x + g2.w + GAP_X);
	assert.equal(at(lay, id(10)).parent, id(5));
	const rowH = TOP + PIC_H + META;
	assert.deepEqual(lay.rows.map((r) => r.y), [0, 1, 2, 3].map((i) => PAD_T + i * (rowH + GAP_Y)));
	assert.equal(lay.height, PAD_T + 4 * rowH + 3 * GAP_Y + 3.5);
	// no line crosses a card: every vertical segment's x is outside the cards of the rows it passes
	for (const [x1, y1, x2, y2] of lay.lines) {
		if (x1 !== x2) continue;
		for (const n of lay.nodes) {
			const inside = x1 > n.x && x1 < n.x + n.w && Math.max(y1, y2) > n.y + TOP && Math.min(y1, y2) < n.y + TOP + PIC_H + META;
			assert.equal(inside, false, `${[x1, y1, x2, y2]} crosses ${n.key}`);
		}
	}
});

test('the lines: a source\'s comb out of its side, a drop + comb from a parent, a tick into each result', () => {
	const lay = bandLayout({ shot: S, records: [rec(1), rec(2), rec(4, { parents: [id(2)] })], aspect: '9:16' });
	const { pw, w } = cardSize('9:16');
	const r0 = lay.rows[0].y, r1 = lay.rows[1].y;
	const src = at(lay, `render:${S}`), g1 = at(lay, id(1)), g2 = at(lay, id(2)), g4 = at(lay, id(4));
	const mid = r0 + TOP + PIC_H / 2, xc = src.x + w + GAP_X / 2, c = (n) => n.x + pw / 2;
	assert.deepEqual(lay.lines, [
		[src.x + pw, mid, xc, mid], [xc, mid, xc, r0 + COMB], [xc, r0 + COMB, c(g2), r0 + COMB],
		[c(g1), r0 + COMB, c(g1), r0 + TOP], [c(g2), r0 + COMB, c(g2), r0 + TOP],
		[c(g2), r0 + TOP + PIC_H + META, c(g2), r1 + COMB], [c(g2), r1 + COMB, c(g4), r1 + COMB],
		[c(g4), r1 + COMB, c(g4), r1 + TOP]
	]);
	// a source without results: no line
	assert.deepEqual(bandLayout({ shot: S, records: [], aspect: '9:16' }).lines, []);
});

test('records without a parent: the take by its take, the render by its inputs, else the earlier row; uploads are sources', () => {
	const records = [
		rec(1),
		rec(2, { take: 't3', inputs: [{ role: 'control', key: `out:s1c3/handheld/${S}--t3--beauty.mp4` }] }),
		rec(3, { inputs: [{ role: 'still', key: `boards:dataset/files/s1c3/${S}/chosen.png` }] }),
		rec(4, { inputs: [{ role: 'still', key: `frame:out:s1c3/frames/${S}.png#first` }] }),
		rec(5, { inputs: [], generator: 'upload' }),
		rec(6, { parents: [id(5)] }),
		rec(7, { inputs: [{ role: 'reference', key: 'media:characters/plf.png' }] }),
		rec(8, { earlier: true, inputs: [], generator: null }),
		rec(9, { parents: [id(8)] }),
		rec(11, { take: 't9', inputs: [] }),
		{ ...rec(10), file: null, status: 'failed' }
	];
	const lay = bandLayout({ shot: S, takes: ['t3'], records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => [r.label, ...r.keys]), [
		[LABELS.source, `render:${S}`, id(1), id(3), id(4)],
		[LABELS.source, `take:${S}:t3`, id(2)],
		[LABELS.source, id(5), id(6)],
		[LABELS.earlier, id(7), id(8), id(11)],
		[null, id(9)]
	]);
	assert.equal(at(lay, id(5)).source, 'upload');
	assert.equal(at(lay, id(5)).kind, 'source');
	assert.deepEqual(rejectIds(lay, id(5)), [], 'a dropped file is a source – not rejectable (§3.5)');
	assert.deepEqual(rejectIds(lay, id(6)), [id(6)], 'what came from it is');
	assert.equal(at(lay, `take:${S}:t3`).take, 't3');
	assert.equal(lay.nodes.some((n) => n.key === id(10)), false, 'a failed job is no card');
	assert.equal(lay.rows[3].x, PAD);
	// the earlier row has no lines into it; its child row hangs under its parent
	const g8 = at(lay, id(8));
	assert.ok(lay.lines.some(([x1, , x2]) => x1 === x2 && x1 === g8.x + g8.pw / 2));
});

test('the MAIN input decides (inputs[0]): a take\'s video before a generated keyframe, a generated still before the render', () => {
	const take = (t) => ({ role: 'control', key: `out:s1c3/handheld/${S}--${t}--beauty.mp4` });
	const records = [
		rec(1),
		rec(2, { parents: [id(1)] }),
		// Aleph: take t3's video + g1 as the keyframe → take t3's row (the mockup's g10), not under g1
		rec(3, { take: 't3', parents: [id(1)], inputs: [take('t3'), { ...GEN(id(1), 'keyframe'), at: 0 }] }),
		// Aleph: g2's video + the render as the keyframe → under g2
		rec(4, { parents: [id(2)], inputs: [{ role: 'control', key: `out:s1c3/generated/${id(2)}.mp4` }, { role: 'keyframe', key: `out:s1c3/clean/${S}.png`, at: 0 }] }),
		// comfy: the render as image 1, g1 as a reference → under the render
		rec(5, { parents: [id(1)], inputs: [CLEAN, GEN(id(1), 'reference')] }),
		// the approved copy as the still: its `from` (g1) is the parent → under g1
		rec(6, { parents: [id(1)], inputs: [{ role: 'still', key: `boards:dataset/files/s1c3/${S}/chosen.png` }] }),
		// a take's handheld frame as the still, the take without a render on the board → the render it also names
		rec(7, { take: 't9', inputs: [{ role: 'still', key: `out:s1c3/handheld/${S}--t9/beauty/f0001.png` }, { role: 'keyframe', key: `out:s1c3/frames/${S}.png` }] })
	];
	const lay = bandLayout({ shot: S, takes: ['t3'], records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [
		[`render:${S}`, id(1), id(5), id(7)],
		[id(2), id(6)],
		[id(4)],
		[`take:${S}:t3`, id(3)]
	]);
	assert.equal(at(lay, id(3)).parent, `take:${S}:t3`);
	assert.equal(at(lay, id(4)).parent, id(2));
	assert.equal(at(lay, id(6)).parent, id(1));
});

test('a loop in the parents lands in the earlier row, nothing is lost', () => {
	const records = [rec(1, { parents: [id(2)], inputs: [] }), rec(2, { parents: [id(1)], inputs: [] }), rec(3, { parents: [id(3)], inputs: [CLEAN] })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.equal(lay.nodes.length, 4);
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(3)], [id(1)], [id(2)]]);
	assert.equal(lay.rows[1].label, LABELS.earlier);
});

test('keys: ← → in the row, ↑ ↓ the nearest column, over a band\'s edge into the next band', () => {
	const a = bandLayout({ shot: S, records: [rec(1), rec(2), rec(5), rec(7, { parents: [id(5)] })], aspect: '9:16' });
	const B = 'sb-25-3';
	const b = bandLayout({ shot: B, records: [{ ...rec(1), id: `${B}--g1`, file: `${B}--g1.png`, inputs: [{ role: 'still', key: `out:s1c3/clean/${B}.png` }] }], aspect: '1:3' });
	assert.deepEqual(b.rows.map((r) => r.keys), [[`render:${B}`, `${B}--g1`]]);
	const bands = [{ shot: S, layout: a }, { shot: B, layout: b }];
	const go = (key, dir, shot = S) => neighbour(bands, { shot, key }, dir);
	assert.deepEqual(go(id(1), 'right'), { shot: S, key: id(2) });
	assert.deepEqual(go(id(1), 'left'), { shot: S, key: `render:${S}` });
	assert.equal(go(`render:${S}`, 'left'), null);
	assert.equal(go(id(5), 'right'), null);
	assert.deepEqual(go(id(5), 'down'), { shot: S, key: id(7) });
	assert.deepEqual(go(id(7), 'up'), { shot: S, key: id(5) });
	assert.deepEqual(go(id(7), 'down'), { shot: B, key: `${B}--g1` }, 'the nearest column of the next band');
	assert.deepEqual(go(`render:${B}`, 'up', B), { shot: S, key: id(7) }, 'the previous band\'s last row (g7 alone in it)');
	assert.deepEqual(go(`render:${B}`, 'right', B), { shot: B, key: `${B}--g1` });
	assert.equal(go(`render:${S}`, 'up'), null);
	assert.equal(go(`${B}--g1`, 'down', B), null);
	assert.equal(go('nothing', 'down'), null);
});

test('X: a card and its branch (not the chosen, not what is rejected already); X again: what that X folded, to its marks before', () => {
	const T = '2026-10-20T15:22:00.000Z';
	const records = [rec(1), rec(3), rec(4, { parents: [id(3)] }), rec(6, { parents: [id(3)], approved: true }), rec(8, { parents: [id(4)], mark: 'rejected', marked: '2026-10-20T10:00:00.000Z' })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(branchOf(lay, id(3)), [id(3), id(4), id(8), id(6)]);
	assert.deepEqual(rejectIds(lay, id(3)), [id(3), id(4)]);
	assert.deepEqual(rejectIds(lay, id(6)), [], 'the chosen can\'t be rejected');
	assert.deepEqual(rejectIds(lay, `render:${S}`), [], 'the render has no status');
	assert.deepEqual(rejectIds(lay, id(8)), [], 'rejected already – X unrejects');

	const after = bandLayout({
		shot: S, aspect: '9:16', records: [
			rec(1), rec(3, { mark: 'rejected', marked: T, before: { mark: 'review', marked: '2026-10-20T09:00:00.000Z' } }),
			rec(4, { parents: [id(3)], mark: 'rejected', marked: T, before: null }), rec(6, { parents: [id(3)], approved: true }),
			rec(8, { parents: [id(4)], mark: 'rejected', marked: '2026-10-20T10:00:00.000Z', before: null })
		]
	});
	assert.deepEqual(unrejectIds(after, id(3)), [{ mark: 'review', marked: '2026-10-20T09:00:00.000Z', ids: [id(3)] }, { mark: null, marked: null, ids: [id(4)] }]);
	assert.deepEqual(unrejectIds(after, id(4)), [{ mark: null, marked: null, ids: [id(4)] }], 'inside its X: only the card');
	assert.deepEqual(unrejectIds(after, id(8)), [{ mark: null, marked: null, ids: [id(8)] }], 'its own X');
	assert.deepEqual(unrejectIds(after, id(1)), []);
});

test('the head\'s figures: results, rejected, the money of every job', () => {
	const records = [rec(1, { cost: { usd: 0.25 } }), rec(2, { mark: 'rejected', cost: { usd: 0.6 } }), { ...rec(3), file: null, cost: { usd: 0.25 } }, rec(4, { cost: null })];
	assert.deepEqual(bandFacts(records), { done: 3, rejected: 1, usd: 1.1 });
	assert.deepEqual(bandFacts(), { done: 0, rejected: 0, usd: 0 });
});
