// The variants board's layout (src/lib/board-layout.js, agents/research/VARIANTS-CANVAS.md §3.2 + §3.5): sources in
// column 0, a parent's results as one row, a generation's results one row down and one column right, the main input,
// the rightmost parent's rows first, records without a parent (the render by its inputs, the take, the earlier row),
// a loop in the parents, the lines (comb, ticks, drops), the card sizes, the keys across rows and bands, X's ids, the
// head's figures; the fold (§3.5): strips, a branch in one strip, shared strips, a row lying down, the look, Show
// rejected, the keys skipping strips and folded bands; the captions below 60 %.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bandFacts, bandLayout, branchOf, CAPTION_W, cardSize, COMB, GAP_X, GAP_Y, LABELS, LIE_W, lookKeys, META, neighbour, nodeOf, PAD, PAD_B, PAD_T, PIC_H, ratioOf, rejectIds, shortId, statusOf, stripLabel, SW, TOP, unrejectIds } from '../src/lib/board-layout.js';

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
	assert.deepEqual(cardSize('9:16'), { h: PIC_H, pw: PIC_H * 9 / 16, w: PIC_H * 9 / 16, meta: META });
	const narrow = cardSize('1:3');
	assert.equal(narrow.h, PIC_H);
	assert.equal(narrow.pw, 3);
	assert.equal(narrow.w, CAPTION_W);
	assert.equal(cardSize('2:1').w, 18);
	assert.equal(cardSize('9:16', 0.5).h, PIC_H / 2);
	// below 60 %: the captions hide – no caption row, the column is the picture's width
	assert.deepEqual(cardSize('1:3', 0.5), { h: PIC_H / 2, pw: 1.5, w: 1.5, meta: 0 });
	assert.deepEqual(cardSize('1:3', 0.6), { h: PIC_H * 0.6, pw: PIC_H * 0.6 / 3, w: CAPTION_W * 0.6, meta: META });
	const small = bandLayout({ shot: S, records: [rec(1), rec(2, { parents: [id(1)] })], aspect: '9:16', zoom: 0.5 });
	assert.equal(small.rows[1].y, PAD_T + TOP + PIC_H / 2 + GAP_Y, 'a row is the picture high (no caption)');
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

// --- the fold (§3.5) ---------------------------------------------------------------------------------------------------
const T = '2026-10-20T15:22:00.000Z';
const R = (n, f = {}) => rec(n, { mark: 'rejected', marked: T, ...f });
/** render → g1 (rejected), g2 (chosen), g3 (rejected with g4 + g6), g5; g5 → g7; g2 → g9, g11 (both rejected) */
const FOLD = () => [R(1, { marked: '2026-10-20T15:20:00.000Z' }), rec(2, { approved: true }), R(3), R(4, { parents: [id(3)] }), rec(5), R(6, { parents: [id(3)] }), rec(7, { parents: [id(5)] }), R(9, { parents: [id(2)], marked: '2026-10-20T15:24:00.000Z' }), R(11, { parents: [id(2)], marked: '2026-10-20T15:24:00.000Z' })];

test('a rejected card is a strip in its place; its branch with nothing open under it is ONE strip; the row closes up', () => {
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const { w } = cardSize('9:16');
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3), id(5)], [id(7)], [id(9)]]);
	const g1 = at(lay, id(1)), g2 = at(lay, id(2)), g3 = at(lay, id(3)), g5 = at(lay, id(5));
	assert.equal(g1.kind, 'strip');
	assert.deepEqual([g1.w, g1.pw, g1.h, g1.lie], [SW, SW, PIC_H, false], 'the picture\'s height, SW wide');
	assert.equal(g2.x, g1.x + SW + GAP_X, 'the row closes up');
	assert.equal(g2.kind, 'result');
	assert.deepEqual(g3.keys, [id(3), id(4), id(6)], 'g3 with its branch');
	assert.equal(g3.whole, true);
	assert.equal(stripLabel(g3), 'g3 +2');
	assert.equal(stripLabel(g1), 'g1');
	assert.equal(g5.x, g3.x + SW + GAP_X);
	assert.equal(lay.nodes.some((n) => n.key === id(4) || n.key === id(6)), false, 'its row is gone');
	assert.equal(nodeOf(lay, id(4)), g3, 'a folded card is found in its strip');
	assert.ok(lay.tree.get(id(4)).record, 'the tree keeps every card');
	// the line to a strip stays: a tick into its middle
	const r0 = lay.rows[0].y;
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g1.x + SW / 2 && x2 === x1 && y1 === r0 + COMB && y2 === r0 + TOP));
	assert.ok(lay.lines.some(([x1, , x2, y2]) => x1 === g3.x + SW / 2 && x2 === x1 && y2 === r0 + TOP));
	assert.equal(at(lay, id(7)).x, g5.x + w + GAP_X);
});

test('a row of nothing but strips lies down as one line – the height comes back', () => {
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const { w } = cardSize('9:16');
	const g2 = at(lay, id(2)), g9 = at(lay, id(9));
	const row = lay.rows[2];
	assert.equal(row.lie, true);
	assert.deepEqual([g9.kind, g9.lie, g9.h, g9.w], ['strip', true, SW, Math.max(w, LIE_W)]);
	assert.deepEqual(g9.keys, [id(9), id(11)], 'rejected neighbours share one strip');
	assert.equal(stripLabel(g9), 'g9 +1');
	assert.equal(g9.x, g2.x + w + GAP_X, 'one column right of its parent');
	assert.equal(lay.height, row.y + TOP + SW + PAD_B, 'a lying row is TOP + SW high');
	// the drop from g2's caption into the lying row's comb, a tick into its middle
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g2.x + g2.pw / 2 && x2 === x1 && y1 === g2.y + TOP + PIC_H + META && y2 === row.y + COMB));
	assert.ok(lay.lines.some(([x1, , , y2]) => x1 === g9.x + g9.w / 2 && y2 === row.y + TOP));
	assert.equal(lay.rows[0].lie, false, 'a source row never lies down (its source is a card)');
});

test('folded neighbours share one strip; something open under a rejected card keeps its row', () => {
	// g1 + g2 rejected apart (two stamps) → one strip; g3 rejected, its g4 rejected with it, its g6 the chosen → g3 its own strip
	const records = [R(1), R(2, { marked: '2026-10-20T16:00:00.000Z' }), R(3), R(4, { parents: [id(3)] }), rec(6, { parents: [id(3)], approved: true }), rec(8)];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(3), id(8)], [id(4), id(6)]]);
	const g1 = at(lay, id(1)), g3 = at(lay, id(3)), g4 = at(lay, id(4));
	assert.deepEqual(g1.keys, [id(1), id(2)]);
	assert.equal(stripLabel(g1), 'g1 +1');
	assert.deepEqual([g3.kind, g3.whole, g3.keys], ['strip', false, [id(3)]], 'not merged: its line goes down');
	assert.equal(g3.x, g1.x + SW + GAP_X, 'next to the shared strip, not merged with it');
	assert.equal(lay.rows[1].lie, false, 'the chosen is a card – the row stands');
	assert.equal(lay.rows[1].x, g3.x + SW + GAP_X, 'one column right of the strip');
	assert.deepEqual([g4.kind, g4.w], ['strip', SW]);
	// the drop from the strip's foot (no caption under a strip)
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g3.x + SW / 2 && x2 === x1 && y1 === g3.y + TOP + PIC_H && y2 === lay.rows[1].y + COMB));
});

test('Show rejected folds nothing; a look unfolds a strip with its branch', () => {
	const all = bandLayout({ shot: S, records: FOLD(), aspect: '9:16', showRejected: true });
	assert.equal(all.nodes.some((n) => n.kind === 'strip'), false);
	assert.deepEqual(all.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3), id(5)], [id(7)], [id(4), id(6)], [id(9), id(11)]]);
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	assert.deepEqual(lookKeys(lay, id(3)), [id(3), id(4), id(6)]);
	assert.deepEqual(lookKeys(lay, id(4)), [id(3), id(4), id(6)], 'a card inside a strip opens the whole strip');
	assert.deepEqual(lookKeys(lay, id(9)), [id(9), id(11)]);
	const look = bandLayout({ shot: S, records: FOLD(), aspect: '9:16', open: lookKeys(lay, id(3)) });
	assert.deepEqual(look.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3), id(5)], [id(7)], [id(4), id(6)], [id(9)]]);
	assert.equal(at(look, id(3)).kind, 'result', 'unfolded: a card again (muted – its status stays rejected)');
	assert.equal(at(look, id(1)).kind, 'strip', 'the other strips stay folded');
	assert.equal(statusOf(at(look, id(4)).record), 'rejected');
});

test('the keys skip strips and folded bands; from a strip they go to the nearest card', () => {
	const a = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const B = 'sb-25-3', C = 'sb-25-4';
	const one = (s) => bandLayout({ shot: s, records: [{ ...rec(1), id: `${s}--g1`, file: `${s}--g1.png`, inputs: [{ role: 'still', key: `out:s1c3/clean/${s}.png` }] }], aspect: '9:16' });
	const bands = [{ shot: S, layout: a }, { shot: B, layout: one(B) }, { shot: C, layout: one(C) }];
	const go = (key, dir, shot = S, list = bands) => neighbour(list, { shot, key }, dir);
	assert.deepEqual(go(`render:${S}`, 'right'), { shot: S, key: id(2) }, 'over the strip g1');
	assert.deepEqual(go(id(2), 'right'), { shot: S, key: id(5) }, 'over the strip g3 +2');
	assert.deepEqual(go(id(5), 'left'), { shot: S, key: id(2) });
	assert.deepEqual(go(id(4), 'right'), { shot: S, key: id(5) }, 'from a strip (its folded card current)');
	assert.deepEqual(go(id(3), 'left'), { shot: S, key: id(2) });
	assert.deepEqual(go(id(2), 'down'), { shot: S, key: id(7) });
	assert.deepEqual(go(id(7), 'down'), { shot: B, key: `${B}--g1` }, 'the lying row is skipped – into the next band');
	assert.deepEqual(go(`render:${B}`, 'up', B), { shot: S, key: id(7) });
	assert.deepEqual(go(id(9), 'up'), { shot: S, key: id(7) }, 'from the lying strip up');
	// a folded band is skipped; from a folded band ← → stay, ↑ ↓ leave it
	const folded = bands.map((b) => ({ ...b, folded: b.shot === B }));
	assert.deepEqual(go(id(7), 'down', S, folded), { shot: C, key: `${C}--g1` });
	assert.deepEqual(go(`render:${C}`, 'up', C, folded), { shot: S, key: id(7) });
	assert.equal(go(`render:${B}`, 'right', B, folded), null);
	assert.deepEqual(go(`${B}--g1`, 'down', B, folded), { shot: C, key: `${C}--g1` });
	// a look: its cards are cards – the arrows walk them
	const look = [{ shot: S, layout: bandLayout({ shot: S, records: FOLD(), aspect: '9:16', open: lookKeys(a, id(3)) }) }];
	assert.deepEqual(go(id(3), 'down', S, look), { shot: S, key: id(7) });
	assert.deepEqual(go(id(7), 'down', S, look), { shot: S, key: id(6) }, 'g3\'s row: g6 under g7');
	assert.deepEqual(go(id(6), 'left', S, look), { shot: S, key: id(4) });
});

test('X on a folded layout: the tree answers – what a reject marks, what comes back', () => {
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	assert.deepEqual(rejectIds(lay, id(5)), [id(5), id(7)]);
	assert.deepEqual(rejectIds(lay, id(2)), [], 'the chosen');
	assert.deepEqual(unrejectIds(lay, id(3)), [{ mark: null, marked: null, ids: [id(3), id(4), id(6)] }], 'the root of its X: the whole branch');
	assert.deepEqual(unrejectIds(lay, id(4)), [{ mark: null, marked: null, ids: [id(4)] }], 'a folded card inside its X: only itself');
	assert.deepEqual(branchOf(lay, id(2)), [id(2), id(9), id(11)]);
});

test('layout variants are source cards after the render (Q5: A), a row each – no status; an input naming one hangs under it', () => {
	const variants = [{ id: 'v1', note: 'low angle' }, { id: 'v2', note: '' }, { note: 'no id – skipped' }, { id: 'v1', note: 'twice – once' }];
	const lay = bandLayout({ shot: S, variants, takes: ['t1'], records: [rec(1), rec(8, { generator: 'upload', inputs: [] })], aspect: '9:16' });
	const v1 = `variant:${S}:v1`, v2 = `variant:${S}:v2`;
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(1)], [v1], [v2], [`take:${S}:t1`], [id(8)]]);
	assert.deepEqual(lay.rows.map((r) => r.label), [LABELS.source, LABELS.variant, LABELS.variant, LABELS.source, LABELS.source]);
	const n = at(lay, v1);
	assert.deepEqual([n.kind, n.source, n.variant.note, n.record, n.x], ['source', 'variant', 'twice – once', null, PAD]);
	assert.deepEqual(rejectIds(lay, v1), [], 'a source can\'t be rejected');
	// the keys walk them as cards: ↓ from the render, ↑ from the take
	const bands = [{ shot: S, layout: lay }];
	assert.deepEqual(neighbour(bands, { shot: S, key: id(1) }, 'down'), { shot: S, key: v1 });
	assert.deepEqual(neighbour(bands, { shot: S, key: `take:${S}:t1` }, 'up'), { shot: S, key: v2 });
	// a result whose main input is a variant's saved frame hangs in that variant's row (stage 3 sends one) – a variant
	// that is gone (taken): the earlier row, nothing guessed
	const frame = (v) => ({ inputs: [{ role: 'still', key: `out:s1c3/variants/${S}--${v}.png` }] });
	const from = bandLayout({ shot: S, variants, records: [rec(2, frame('v1')), rec(3, frame('v9'))], aspect: '9:16' });
	assert.deepEqual(from.rows.map((r) => r.keys), [[`render:${S}`], [v1, id(2)], [v2], [id(3)]]);
	assert.equal(from.rows.at(-1).label, LABELS.earlier);
});

test('a running job is a placeholder at the end of its main input\'s row (else the render\'s) – the keys skip it', () => {
	const jobs = [
		{ id: 'j3', status: 'running', progress: 0.4, inputs: [CLEAN], parents: [] },
		{ id: 'j4', status: 'queued', progress: null, inputs: [GEN(id(1))], parents: [id(1)] },
		{ id: 'j5', status: 'running', progress: 0.1, inputs: [], parents: [] },
		{ id: 'j6', status: 'running', progress: 0.9, inputs: [GEN(id(3))], parents: [id(3)] }
	];
	const records = [rec(1), rec(2), rec(3, { mark: 'rejected', marked: '2026-10-20T12:00:00Z' }), rec(7, { parents: [id(1)] })];
	const lay = bandLayout({ shot: S, records, jobs, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3), 'job:j3', 'job:j5'], ['job:j6'], [id(7), 'job:j4']]);
	const j3 = at(lay, 'job:j3');
	assert.deepEqual([j3.kind, j3.record, j3.job.progress, j3.parent, j3.w, j3.h], ['running', null, 0.4, `render:${S}`, cardSize('9:16').w, PIC_H]);
	assert.equal(at(lay, 'job:j4').parent, id(1), 'from g1: in g1\'s row, after g7 – a new run lands at the row\'s end');
	// under a rejected card: something open under it – the strip stands, the placeholder's row stays
	assert.equal(at(lay, id(3)).kind, 'strip');
	assert.equal(at(lay, id(3)).whole, false);
	assert.equal(at(lay, 'job:j6').parent, id(3));
	// a line into a placeholder as into a card; the keys skip it
	const tick = (k) => lay.lines.some(([x1, , x2, y2]) => x1 === x2 && Math.abs(x1 - (at(lay, k).x + at(lay, k).pw / 2)) < 1e-9 && Math.abs(y2 - (at(lay, k).y + TOP)) < 1e-9);
	assert.ok(tick('job:j3') && tick('job:j4'));
	const bands = [{ shot: S, layout: lay }];
	assert.equal(neighbour(bands, { shot: S, key: id(2) }, 'right'), null, 'g2 →: the strip g3 and the placeholders skipped');
	assert.deepEqual(neighbour(bands, { shot: S, key: id(7) }, 'right'), null);
	assert.deepEqual(rejectIds(lay, id(1)), [id(1), id(7)], 'X on g1: its records only (a running job has none)');
	// no jobs: the same layout as before
	assert.deepEqual(bandLayout({ shot: S, records, aspect: '9:16' }).rows.map((r) => r.keys), [[`render:${S}`, id(1), id(2), id(3)], [id(7)]]);
});
