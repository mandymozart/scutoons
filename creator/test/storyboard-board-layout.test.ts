// The variants board's layout (src/lib/editor/storyboard-board-layout.ts, #125 – the port of #62's
// test/board-layout.test.mjs): sources in column 0, a parent's results as one row, a generation's results one row down
// and one column right, the main input, the rightmost parent's rows first, the take / the render for records without a
// parent on the board, a loop in the parents, the lines, the card sizes, the keys across rows and bands, X's ids, the
// head's figures; the fold (§3.5): strips, a branch in one strip, shared strips, a row lying down, the look, Show
// rejected, the keys skipping strips and folded bands; running jobs; the captions below 60 %.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	bandFacts,
	bandLayout,
	branchOf,
	CAPTION_W,
	cardSize,
	COMB,
	GAP_X,
	GAP_Y,
	LIE_W,
	lookKeys,
	META,
	neighbour,
	nodeOf,
	PAD,
	PAD_B,
	PAD_T,
	PIC_H,
	ratioOf,
	rejectIds,
	sameStamp,
	shortName,
	statusOf,
	stripLabel,
	SW,
	TOP,
	unrejectIds,
	type BandLayout,
	type BoardRecord
} from '../src/lib/editor/storyboard-board-layout.ts';

const S = 'sb-25-2';
const R0 = `render:${S}`;
/** a generation as the server lists it (only what the layout reads) */
const rec = (n: number, f: Partial<BoardRecord> & { shot?: string } = {}): BoardRecord => {
	const shot = f.shot ?? S;
	return { id: n, shot, kind: 'image', take: null, parents: [], generator: 'runway', model: 'gen4_turbo', status: 'succeeded', approved: false, mark: null, marked_at: null, file: `${shot}--g${n}.png`, ...f };
};
const at = (lay: BandLayout, key: string | number) => lay.nodes.find((n) => n.key === String(key))!;
const k = (n: number) => String(n);

test('sizes: one picture height, the aspect, a caption wide at least', () => {
	assert.equal(ratioOf('9:16'), 9 / 16);
	assert.equal(ratioOf('1:3'), 1 / 3);
	assert.equal(ratioOf('nonsense'), 9 / 16);
	assert.deepEqual(cardSize('9:16'), { h: PIC_H, pw: (PIC_H * 9) / 16, w: (PIC_H * 9) / 16, meta: META });
	const narrow = cardSize('1:3');
	assert.equal(narrow.pw, 3);
	assert.equal(narrow.w, CAPTION_W);
	assert.equal(cardSize('2:1').w, 18);
	assert.deepEqual(cardSize('1:3', 0.5), { h: PIC_H / 2, pw: 1.5, w: 1.5, meta: 0 }, 'below 60 %: no caption, the picture\'s width');
	const small = bandLayout({ shot: S, records: [rec(1), rec(2, { parents: [1] })], aspect: '9:16', zoom: 0.5 });
	assert.equal(small.rows[1].y, PAD_T + TOP + PIC_H / 2 + GAP_Y, 'a row is the picture high (no caption)');
	assert.equal(shortName(rec(7)), 'g7');
	assert.equal(shortName({ id: 12, file: null }), '#12');
});

test('the status: chosen before the mark, review, rejected, none', () => {
	assert.equal(statusOf(rec(1, { approved: true, mark: 'review' })), 'chosen');
	assert.equal(statusOf(rec(1, { mark: 'review' })), 'review');
	assert.equal(statusOf(rec(1, { mark: 'rejected' })), 'rejected');
	assert.equal(statusOf(rec(1)), null);
	assert.equal(statusOf(null), null);
});

test('a source row: the render at column 0, its results oldest left, one row; other shots\' records stay out', () => {
	const lay = bandLayout({ shot: S, records: [rec(3), rec(1), rec(2), rec(4, { shot: 'sb-25-3' })], aspect: '9:16' });
	const w = cardSize('9:16').w;
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(2), k(3)]]);
	assert.equal(lay.rows[0].label, 'source');
	assert.equal(at(lay, R0).x, PAD);
	assert.equal(at(lay, 1).x, PAD + w + GAP_X);
	assert.equal(at(lay, 3).x, PAD + 3 * (w + GAP_X));
	assert.equal(at(lay, 2).parent, R0);
	assert.equal(lay.width, at(lay, 3).x + w + PAD);
});

test('a generation\'s results: a row under it, one column right; the rightmost parent\'s rows first', () => {
	const records = [rec(1), rec(2), rec(5), rec(7, { parents: [5] }), rec(9, { parents: [2] }), rec(10, { parents: [5, 2] }), rec(12, { parents: [5] }), rec(13, { parents: [7] })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(2), k(5)], [k(7), k(10), k(12)], [k(13)], [k(9)]]);
	const g5 = at(lay, 5), g7 = at(lay, 7), g2 = at(lay, 2);
	assert.equal(g7.x, g5.x + g5.w + GAP_X);
	assert.equal(at(lay, 13).x, g7.x + g7.w + GAP_X);
	assert.equal(at(lay, 9).x, g2.x + g2.w + GAP_X);
	assert.equal(at(lay, 10).parent, k(5), 'the main input (the first parent)');
	const rowH = TOP + PIC_H + META;
	assert.deepEqual(lay.rows.map((r) => r.y), [0, 1, 2, 3].map((i) => PAD_T + i * (rowH + GAP_Y)));
	assert.equal(lay.height, PAD_T + 4 * rowH + 3 * GAP_Y + PAD_B);
	for (const [x1, y1, x2, y2] of lay.lines) {
		if (x1 !== x2) continue;
		for (const n of lay.nodes) {
			const inside = x1 > n.x && x1 < n.x + n.w && Math.max(y1, y2) > n.y + TOP && Math.min(y1, y2) < n.y + TOP + PIC_H + META;
			assert.equal(inside, false, `${[x1, y1, x2, y2]} crosses ${n.key}`);
		}
	}
});

test('the lines: a source\'s comb out of its side, a drop + comb from a parent, a tick into each result', () => {
	const lay = bandLayout({ shot: S, records: [rec(1), rec(2), rec(4, { parents: [2] })], aspect: '9:16' });
	const { pw, w } = cardSize('9:16');
	const r0 = lay.rows[0].y, r1 = lay.rows[1].y;
	const src = at(lay, R0), g1 = at(lay, 1), g2 = at(lay, 2), g4 = at(lay, 4);
	const mid = r0 + TOP + PIC_H / 2, xc = src.x + w + GAP_X / 2, c = (n: { x: number }) => n.x + pw / 2;
	assert.deepEqual(lay.lines, [
		[src.x + pw, mid, xc, mid], [xc, mid, xc, r0 + COMB], [xc, r0 + COMB, c(g2), r0 + COMB],
		[c(g1), r0 + COMB, c(g1), r0 + TOP], [c(g2), r0 + COMB, c(g2), r0 + TOP],
		[c(g2), r0 + TOP + PIC_H + META, c(g2), r1 + COMB], [c(g2), r1 + COMB, c(g4), r1 + COMB],
		[c(g4), r1 + COMB, c(g4), r1 + TOP]
	]);
	assert.deepEqual(bandLayout({ shot: S, records: [], aspect: '9:16' }).lines, []);
});

test('without a parent on the board: its take\'s card, else the render; uploads are sources; failed jobs no card', () => {
	const records = [
		rec(1),
		rec(2, { take: 't3' }),
		rec(3, { parents: [99] }),
		rec(5, { generator: 'upload' }),
		rec(6, { parents: [5] }),
		rec(11, { take: 't9' }),
		rec(10, { status: 'failed', file: null }),
		rec(14, { status: 'cancelled', file: null })
	];
	const lay = bandLayout({ shot: S, takes: ['t3'], records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => [r.label, ...r.keys]), [
		['source', R0, k(1), k(3), k(11)],
		['source', `take:${S}:t3`, k(2)],
		['source', k(5), k(6)]
	]);
	assert.deepEqual([at(lay, 5).kind, at(lay, 5).source], ['source', 'upload']);
	assert.deepEqual(rejectIds(lay, k(5)), [], 'a dropped file is a source – not rejectable (§3.5)');
	assert.deepEqual(rejectIds(lay, k(6)), [6], 'what came from it is');
	assert.equal(at(lay, `take:${S}:t3`).take, 't3');
	assert.equal(lay.nodes.some((n) => n.key === k(10) || n.key === k(14)), false);
});

test('the MAIN input decides: the first parent on the board, before the take', () => {
	// Aleph: g2's video + g1 as the keyframe, from take t3 → under g2 (its first parent)
	const records = [rec(1), rec(2, { kind: 'video' }), rec(4, { take: 't3', parents: [2, 1] }), rec(6, { parents: [77, 1] })];
	const lay = bandLayout({ shot: S, takes: ['t3'], records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(2)], [k(4)], [k(6)], [`take:${S}:t3`]]);
	assert.equal(at(lay, 4).parent, k(2));
	assert.equal(at(lay, 6).parent, k(1), 'a parent not on the board is skipped');
});

test('a loop in the parents lands in the earlier row, nothing is lost', () => {
	const records = [rec(1, { parents: [2] }), rec(2, { parents: [1] }), rec(3, { parents: [3] })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.equal(lay.nodes.length, 4);
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(3)], [k(1)], [k(2)]]);
	assert.equal(lay.rows[1].label, 'earlier');
});

test('keys: ← → in the row, ↑ ↓ the nearest column, over a band\'s edge into the next band', () => {
	const a = bandLayout({ shot: S, records: [rec(1), rec(2), rec(5), rec(7, { parents: [5] })], aspect: '9:16' });
	const B = 'sb-25-3';
	const b = bandLayout({ shot: B, records: [rec(21, { shot: B })], aspect: '1:3' });
	const bands = [{ shot: S, layout: a }, { shot: B, layout: b }];
	const go = (key: string, dir: 'left' | 'right' | 'up' | 'down', shot = S) => neighbour(bands, { shot, key }, dir);
	assert.deepEqual(go(k(1), 'right'), { shot: S, key: k(2) });
	assert.deepEqual(go(k(1), 'left'), { shot: S, key: R0 });
	assert.equal(go(R0, 'left'), null);
	assert.equal(go(k(5), 'right'), null);
	assert.deepEqual(go(k(5), 'down'), { shot: S, key: k(7) });
	assert.deepEqual(go(k(7), 'up'), { shot: S, key: k(5) });
	assert.deepEqual(go(k(7), 'down'), { shot: B, key: k(21) }, 'the nearest column of the next band');
	assert.deepEqual(go(`render:${B}`, 'up', B), { shot: S, key: k(7) });
	assert.equal(go(R0, 'up'), null);
	assert.equal(go(k(21), 'down', B), null);
	assert.equal(go('nothing', 'down'), null);
});

test('X: a card and its branch (not the chosen, not what is rejected already); X again: what that X folded, to its marks before', () => {
	const T = '2026-10-20T15:22:00.000Z';
	const records = [rec(1), rec(3), rec(4, { parents: [3] }), rec(6, { parents: [3], approved: true }), rec(8, { parents: [4], mark: 'rejected', marked_at: '2026-10-20T10:00:00.000Z' })];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(branchOf(lay, k(3)), [k(3), k(4), k(8), k(6)]);
	assert.deepEqual(rejectIds(lay, k(3)), [3, 4]);
	assert.deepEqual(rejectIds(lay, k(6)), [], 'the chosen can\'t be rejected');
	assert.deepEqual(rejectIds(lay, R0), [], 'the render has no status');
	assert.deepEqual(rejectIds(lay, k(8)), [], 'rejected already – X unrejects');

	// the server stamps each PUT: one X's marks are seconds apart at most
	const after = bandLayout({
		shot: S, aspect: '9:16', records: [
			rec(1), rec(3, { mark: 'rejected', marked_at: T }), rec(4, { parents: [3], mark: 'rejected', marked_at: '2026-10-20T15:22:02.000Z' }),
			rec(6, { parents: [3], approved: true }), rec(8, { parents: [4], mark: 'rejected', marked_at: '2026-10-20T10:00:00.000Z' })
		]
	});
	const before = (id: number) => (id === 3 ? 'review' : null);
	assert.deepEqual(unrejectIds(after, k(3), before), [{ mark: 'review', ids: [3] }, { mark: null, ids: [4] }]);
	assert.deepEqual(unrejectIds(after, k(3)), [{ mark: null, ids: [3, 4] }], 'what the session doesn\'t know: none');
	assert.deepEqual(unrejectIds(after, k(4)), [{ mark: null, ids: [4] }], 'inside its X: only the card');
	assert.deepEqual(unrejectIds(after, k(8)), [{ mark: null, ids: [8] }], 'its own X');
	assert.deepEqual(unrejectIds(after, k(1)), []);
	assert.equal(sameStamp(T, '2026-10-20T15:22:09.000Z'), true);
	assert.equal(sameStamp(T, '2026-10-20T15:23:00.000Z'), false);
	assert.equal(sameStamp(T, null), false);
});

test('the head\'s figures: results, rejected, the euro of every generation', () => {
	const rate = { eur_per_usd: 0.5, usd_per_eur: 2, date: '2026-10-20', source: 'ecb' };
	const records = [rec(1, { cost: 0.25, currency: 'USD' }), rec(2, { mark: 'rejected', cost: 0.6, currency: 'EUR' }), rec(3, { status: 'failed', cost: 0.25, currency: 'USD' }), rec(4)];
	assert.deepEqual(bandFacts(records, rate), { done: 3, rejected: 1, eur: 0.125 + 0.6 + 0.125 });
	assert.deepEqual(bandFacts(), { done: 0, rejected: 0, eur: 0 });
});

// --- the fold (§3.5) ---------------------------------------------------------------------------------------------------
const T = '2026-10-20T15:22:00.000Z';
const R = (n: number, f: Partial<BoardRecord> = {}) => rec(n, { mark: 'rejected', marked_at: T, ...f });
/** render → g1 (rejected), g2 (chosen), g3 (rejected with g4 + g6), g5; g5 → g7; g2 → g9, g11 (both rejected) */
const FOLD = () => [R(1, { marked_at: '2026-10-20T15:20:00.000Z' }), rec(2, { approved: true }), R(3), R(4, { parents: [3] }), rec(5), R(6, { parents: [3] }), rec(7, { parents: [5] }), R(9, { parents: [2], marked_at: '2026-10-20T15:24:00.000Z' }), R(11, { parents: [2], marked_at: '2026-10-20T15:24:00.000Z' })];

test('a rejected card is a strip in its place; its branch with nothing open under it is ONE strip; the row closes up', () => {
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const { w } = cardSize('9:16');
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(2), k(3), k(5)], [k(7)], [k(9)]]);
	const g1 = at(lay, 1), g2 = at(lay, 2), g3 = at(lay, 3), g5 = at(lay, 5);
	assert.equal(g1.kind, 'strip');
	assert.deepEqual([g1.w, g1.pw, g1.h, g1.lie], [SW, SW, PIC_H, false]);
	assert.equal(g2.x, g1.x + SW + GAP_X, 'the row closes up');
	assert.deepEqual(g3.keys, [k(3), k(4), k(6)], 'g3 with its branch');
	assert.equal(g3.whole, true);
	assert.equal(stripLabel(g3), 'g3 +2');
	assert.equal(stripLabel(g1), 'g1');
	assert.equal(g5.x, g3.x + SW + GAP_X);
	assert.equal(nodeOf(lay, k(4)), g3, 'a folded card is found in its strip');
	assert.ok(lay.tree.get(k(4))!.record, 'the tree keeps every card');
	const r0 = lay.rows[0].y;
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g1.x + SW / 2 && x2 === x1 && y1 === r0 + COMB && y2 === r0 + TOP));
	assert.equal(at(lay, 7).x, g5.x + w + GAP_X);
});

test('a row of nothing but strips lies down as one line – the height comes back', () => {
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const { w } = cardSize('9:16');
	const g2 = at(lay, 2), g9 = at(lay, 9);
	const row = lay.rows[2];
	assert.equal(row.lie, true);
	assert.deepEqual([g9.kind, g9.lie, g9.h, g9.w], ['strip', true, SW, Math.max(w, LIE_W)]);
	assert.deepEqual(g9.keys, [k(9), k(11)], 'rejected neighbours share one strip');
	assert.equal(stripLabel(g9), 'g9 +1');
	assert.equal(g9.x, g2.x + w + GAP_X);
	assert.equal(lay.height, row.y + TOP + SW + PAD_B);
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g2.x + g2.pw / 2 && x2 === x1 && y1 === g2.y + TOP + PIC_H + META && y2 === row.y + COMB));
	assert.equal(lay.rows[0].lie, false, 'a source row never lies down');
});

test('folded neighbours share one strip; something open under a rejected card keeps its row', () => {
	const records = [R(1), R(2, { marked_at: '2026-10-20T16:00:00.000Z' }), R(3), R(4, { parents: [3] }), rec(6, { parents: [3], approved: true }), rec(8)];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(3), k(8)], [k(4), k(6)]]);
	const g1 = at(lay, 1), g3 = at(lay, 3), g4 = at(lay, 4);
	assert.deepEqual(g1.keys, [k(1), k(2)]);
	assert.deepEqual([g3.kind, g3.whole, g3.keys], ['strip', false, [k(3)]]);
	assert.equal(g3.x, g1.x + SW + GAP_X);
	assert.equal(lay.rows[1].lie, false, 'the chosen is a card – the row stands');
	assert.equal(lay.rows[1].x, g3.x + SW + GAP_X);
	assert.deepEqual([g4.kind, g4.w], ['strip', SW]);
	assert.ok(lay.lines.some(([x1, y1, x2, y2]) => x1 === g3.x + SW / 2 && x2 === x1 && y1 === g3.y + TOP + PIC_H && y2 === lay.rows[1].y + COMB));
});

test('Show rejected folds nothing; a look unfolds a strip with its branch', () => {
	const all = bandLayout({ shot: S, records: FOLD(), aspect: '9:16', showRejected: true });
	assert.equal(all.nodes.some((n) => n.kind === 'strip'), false);
	assert.deepEqual(all.rows.map((r) => r.keys), [[R0, k(1), k(2), k(3), k(5)], [k(7)], [k(4), k(6)], [k(9), k(11)]]);
	const lay = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	assert.deepEqual(lookKeys(lay, k(3)), [k(3), k(4), k(6)]);
	assert.deepEqual(lookKeys(lay, k(4)), [k(3), k(4), k(6)], 'a card inside a strip opens the whole strip');
	const look = bandLayout({ shot: S, records: FOLD(), aspect: '9:16', open: lookKeys(lay, k(3)) });
	assert.deepEqual(look.rows.map((r) => r.keys), [[R0, k(1), k(2), k(3), k(5)], [k(7)], [k(4), k(6)], [k(9)]]);
	assert.equal(at(look, 3).kind, 'result');
	assert.equal(at(look, 1).kind, 'strip');
});

test('the keys skip strips and folded bands; from a strip they go to the nearest card', () => {
	const a = bandLayout({ shot: S, records: FOLD(), aspect: '9:16' });
	const B = 'sb-25-3', C = 'sb-25-4';
	const one = (s: string, n: number) => bandLayout({ shot: s, records: [rec(n, { shot: s })], aspect: '9:16' });
	const bands = [{ shot: S, layout: a }, { shot: B, layout: one(B, 31) }, { shot: C, layout: one(C, 41) }];
	const go = (key: string, dir: 'left' | 'right' | 'up' | 'down', shot = S, list: typeof bands = bands) => neighbour(list, { shot, key }, dir);
	assert.deepEqual(go(R0, 'right'), { shot: S, key: k(2) }, 'over the strip g1');
	assert.deepEqual(go(k(2), 'right'), { shot: S, key: k(5) }, 'over the strip g3 +2');
	assert.deepEqual(go(k(4), 'right'), { shot: S, key: k(5) }, 'from a strip (its folded card current)');
	assert.deepEqual(go(k(2), 'down'), { shot: S, key: k(7) });
	assert.deepEqual(go(k(7), 'down'), { shot: B, key: k(31) }, 'the lying row is skipped');
	assert.deepEqual(go(k(9), 'up'), { shot: S, key: k(7) });
	const folded = bands.map((b) => ({ ...b, folded: b.shot === B }));
	assert.deepEqual(go(k(7), 'down', S, folded), { shot: C, key: k(41) });
	assert.equal(go(`render:${B}`, 'right', B, folded), null);
	assert.deepEqual(go(k(31), 'down', B, folded), { shot: C, key: k(41) });
});

test('a running job is a placeholder at the end of its main input\'s row (else the render\'s) – the keys skip it', () => {
	const records = [
		rec(1), rec(2), rec(3, { mark: 'rejected', marked_at: '2026-10-20T12:00:00Z' }), rec(7, { parents: [1] }),
		rec(20, { status: 'running', progress: 0.4, file: null }), rec(21, { status: 'queued', parents: [1], file: null }),
		rec(22, { status: 'running', parents: [3], file: null })
	];
	const lay = bandLayout({ shot: S, records, aspect: '9:16' });
	assert.deepEqual(lay.rows.map((r) => r.keys), [[R0, k(1), k(2), k(3), k(20)], [k(22)], [k(7), k(21)]]);
	const j = at(lay, 20);
	assert.deepEqual([j.kind, j.record, j.job?.progress, j.parent, j.h], ['running', null, 0.4, R0, PIC_H]);
	assert.equal(at(lay, 21).parent, k(1), 'a new run lands at its row\'s end');
	assert.deepEqual([at(lay, 3).kind, at(lay, 3).whole], ['strip', false], 'something open under it: the strip stands');
	const bands = [{ shot: S, layout: lay }];
	assert.equal(neighbour(bands, { shot: S, key: k(2) }, 'right'), null);
	assert.deepEqual(rejectIds(lay, k(1)), [1, 7], 'a running job has no record to mark');
});
