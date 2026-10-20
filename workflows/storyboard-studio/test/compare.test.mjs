// Compare (src/lib/compare.js, agents/research/VARIANTS-CANVAS.md §3.4): the sides it opens with (A = the first
// selected; one selected → the current shot's chosen as A; none → the chosen + the current card), a click while it is
// open (the LAST side replaced, never A; the empty side filled; a rejected card stays out), a side taken out (its place
// empty while open, the letters kept; closed, the rest close up), a card gone from the board (an empty side – never
// hides or replaces A), the sides at one height in their aspects inside the pane, the one clock (its length, a side's
// time incl. holding the last frame, play, drift), the words.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { addSide, advance, chosenKey, clickSide, clockLength, clockText, drifted, filled, fitSides, fromText, HOLD_MS, lettersOf, MAX_SIDES, resolveSides, roleWord, sideAt, sidesOnOpen, takeOut } from '../src/lib/compare.js';

const rec = (n, f = {}) => ({ id: `s--g${n}`, shot: 's', file: `s--g${n}.${f.kind === 'video' ? 'webm' : 'png'}`, kind: 'image', ...f });
const card = (key, shot = 's') => ({ shot, key });

test('the chosen: of the asked kind, else the chosen video, else any; never a missing file', () => {
	const records = [rec(1, { approved: true }), rec(2, { kind: 'video', approved: true }), rec(3)];
	assert.equal(chosenKey(records, 'image'), 's--g1');
	assert.equal(chosenKey(records, 'video'), 's--g2');
	assert.equal(chosenKey(records, null), 's--g2', 'no kind: the video first');
	assert.equal(chosenKey([rec(1, { approved: true })], 'video'), 's--g1', 'no chosen video: the image');
	assert.equal(chosenKey([rec(1, { approved: true, missing: true })], 'image'), null);
	assert.equal(chosenKey([rec(1)], 'image'), null);
	assert.equal(chosenKey(undefined), null);
});

test('opening: 2–4 selected are the sides, A the first', () => {
	const sel = [card('s--g3'), card('t--g1', 't'), card('s--g5')];
	assert.deepEqual(sidesOnOpen({ selected: sel, shot: 's', records: [rec(1, { approved: true })] }).sides, sel, 'the chosen is not pushed in');
	const five = ['a', 'b', 'c', 'd', 'e'].map((k) => card(k));
	assert.equal(sidesOnOpen({ selected: five, shot: 's' }).sides.length, MAX_SIDES);
});

test('opening with one selected: the current shot\'s chosen of its kind is A', () => {
	const records = [rec(1, { approved: true }), rec(2, { kind: 'video', approved: true }), rec(4, { kind: 'video' })];
	const kindOf = (c) => records.find((r) => r.id === c.key)?.kind ?? null;
	assert.deepEqual(sidesOnOpen({ selected: [card('s--g4')], shot: 's', records, kindOf }).sides, [card('s--g2'), card('s--g4')], 'a video beside the chosen video');
	// a selected card of another shot: still the CURRENT shot's chosen as A (continuity across the cut)
	assert.deepEqual(sidesOnOpen({ selected: [card('t--g7', 't')], shot: 's', records, kindOf: () => 'image' }).sides, [card('s--g1'), card('t--g7', 't')]);
	// the selected one is the chosen: one side, B empty
	assert.deepEqual(sidesOnOpen({ selected: [card('s--g2')], shot: 's', records, kindOf }).sides, [card('s--g2')]);
	// no chosen: the selected alone (B empty)
	assert.deepEqual(sidesOnOpen({ selected: [card('s--g4')], shot: 's', records: [rec(4, { kind: 'video' })], kindOf }).sides, [card('s--g4')]);
});

test('opening with nothing selected: the chosen beside the current card – or why not', () => {
	const records = [rec(1, { approved: true }), rec(3)];
	assert.deepEqual(sidesOnOpen({ shot: 's', records, current: { shot: 's', key: 's--g3', kind: 'image' } }).sides, [card('s--g1'), card('s--g3')]);
	assert.deepEqual(sidesOnOpen({ shot: 's', records, current: { shot: 's', key: 's--g3', kind: 'image', rejected: true } }).sides, [card('s--g1')], 'a rejected current card stays out');
	assert.deepEqual(sidesOnOpen({ shot: 's', records: [rec(3)], current: { shot: 's', key: 's--g3' } }).sides, [card('s--g3')]);
	const none = sidesOnOpen({ shot: 's', records: [rec(3)] });
	assert.deepEqual(none.sides, []);
	assert.match(none.why, /Select 2–4/);
});

test('a click while Compare is open replaces the LAST side – A stays; the empty side fills; rejected stays out', () => {
	const ab = [card('a'), card('b')];
	assert.deepEqual(clickSide(ab, card('c')).selected, [card('a'), card('c')]);
	const abcd = [card('a'), card('b'), card('c'), card('d')];
	assert.deepEqual(clickSide(abcd, card('e')).selected, [card('a'), card('b'), card('c'), card('e')]);
	assert.equal(clickSide(ab, card('a')).selected, ab, 'already a side: no change');
	assert.equal(clickSide(ab, card('b')).selected, ab);
	assert.deepEqual(clickSide([card('a')], card('b')).selected, ab, 'one side: B is filled, A stays');
	assert.deepEqual(clickSide([], card('a')).selected, [card('a')]);
	const out = clickSide(ab, card('x'), true);
	assert.equal(out.selected, ab);
	assert.equal(out.why, 'rejected');
});

test('a side taken out while Compare is open stays empty – the letters stay; the next click fills it; closed, they close up', () => {
	const abcd = [card('a'), card('b'), card('c'), card('d')];
	// X on C (its branch with it: c and an unselected one)
	const holed = takeOut(abcd, ['c', 'x'], true);
	assert.deepEqual(holed, [card('a'), card('b'), null, card('d')]);
	assert.deepEqual([...lettersOf(holed)], [['a', 'A'], ['b', 'B'], ['d', 'D']], 'D stays D');
	// a plain click fills the empty side – not the last one, never A
	assert.deepEqual(clickSide(holed, card('e')).selected, [card('a'), card('b'), card('e'), card('d')]);
	// A taken out: the next click fills A (it was asked for), B–D keep their letters
	const noA = takeOut(abcd, ['a'], true);
	assert.deepEqual(clickSide(noA, card('e')).selected, [card('e'), card('b'), card('c'), card('d')]);
	// two out: the first empty one first
	assert.deepEqual(clickSide(takeOut(abcd, ['b', 'd'], true), card('e')).selected, [card('a'), card('e'), card('c'), null]);
	// Ctrl + click / More ▾ add: the first empty side too; none empty: at the end, a fifth replaces D
	assert.deepEqual(addSide(holed, card('e')), [card('a'), card('b'), card('e'), card('d')]);
	assert.deepEqual(addSide([card('a')], card('b')), [card('a'), card('b')]);
	assert.deepEqual(addSide(abcd, card('e')), [card('a'), card('b'), card('c'), card('e')]);
	// closed: the rest close up (and an empty place goes)
	assert.deepEqual(takeOut(abcd, ['b'], false), [card('a'), card('c'), card('d')]);
	assert.deepEqual(takeOut(holed, [], false), [card('a'), card('b'), card('d')]);
	assert.deepEqual(filled(holed), [card('a'), card('b'), card('d')]);
	// 2 sides, X on B: A and an empty B (the next click is B)
	assert.deepEqual(clickSide(takeOut([card('a'), card('b')], ['b'], true), card('c')).selected, [card('a'), card('c')]);
});

test('a card gone from the board (a variant taken, a record gone) is an empty side – it never hides or replaces A', () => {
	const sel = [card('variant:s:v1'), card('s--g3'), card('s--g5')];
	const exists = (c) => c.key !== 'variant:s:v1';
	const sides = resolveSides(sel, exists);
	assert.deepEqual(sides, [null, card('s--g3'), card('s--g5')]);
	assert.deepEqual([...lettersOf(sides)], [['s--g3', 'B'], ['s--g5', 'C']], 'open: by place');
	assert.deepEqual([...lettersOf(filled(sides))], [['s--g3', 'A'], ['s--g5', 'B']], 'closed: closed up – the board and Compare agree');
	// the click fills the gone card's place – g3 and g5 stay
	assert.deepEqual(clickSide(sides, card('s--g4')).selected, [card('s--g4'), card('s--g3'), card('s--g5')]);
	// the reviewer's case: a dead first entry beside g3 – g4 fills the dead place, g3 stays
	assert.deepEqual(clickSide(resolveSides([card('variant:s:v1'), card('s--g3')], exists), card('s--g4')).selected, [card('s--g4'), card('s--g3')]);
	// opening with one live card and a dead one: the dead one is no side
	const records = [rec(1, { approved: true }), rec(3)];
	assert.deepEqual(sidesOnOpen({ selected: resolveSides([card('variant:s:v1'), card('s--g3')], exists), shot: 's', records, kindOf: () => 'image' }).sides, [card('s--g1'), card('s--g3')]);
	assert.deepEqual(resolveSides(undefined), []);
	assert.deepEqual(resolveSides([null, card('a')]), [null, card('a')]);
});

test('the sides: one height, each in its aspect, inside the pane – narrow pictures keep a column for their words', () => {
	// two 9:16 in 27 × 40 (gap 1): the width decides – 26 / 1.125 = 23.11
	const two = fitSides({ ratios: [9 / 16, 9 / 16], width: 27, height: 40, gap: 1, minCol: 6 });
	assert.ok(Math.abs(two.h - 23.11) < 0.011, `h ${two.h}`);
	assert.ok(two.widths.reduce((s, w) => s + w, 0) + 1 <= 27 + 1e-9, 'fits the width');
	// the height decides
	const tall = fitSides({ ratios: [9 / 16, 9 / 16], width: 60, height: 20, gap: 1, minCol: 6 });
	assert.equal(tall.h, 20);
	assert.deepEqual(tall.widths, [11.25, 11.25]);
	// a 1:3 beside a 9:16: one height, the 1:3 gets its column floor (8) when its picture is narrower
	const mixed = fitSides({ ratios: [1 / 3, 9 / 16], width: 27, height: 40, gap: 1, minCol: 8 });
	assert.ok(mixed.widths[0] >= 8 - 1e-9);
	assert.ok(Math.abs(mixed.widths[1] - mixed.h * 9 / 16) < 1e-9);
	assert.ok(mixed.widths[0] + mixed.widths[1] + 1 <= 27 + 1e-9);
	assert.ok(mixed.h > two.h, 'the narrow side leaves the other more room');
	// four in a narrow pane: the floor shrinks to a share – nothing scrolls sideways
	const four = fitSides({ ratios: [9 / 16, 1 / 3, 9 / 16, 9 / 16], width: 20, height: 30, gap: 1, minCol: 8 });
	assert.ok(four.widths.reduce((s, w) => s + w, 0) + 3 <= 20 + 1e-9, `four fit: ${four.widths}`);
	assert.ok(four.h > 0);
	// no room
	assert.deepEqual(fitSides({ ratios: [1], width: 0, height: 10 }), { h: 0, widths: [0] });
	assert.deepEqual(fitSides({ ratios: [], width: 10, height: 10 }), { h: 0, widths: [] });
});

test('one clock: as long as the longest; a shorter video holds its last frame; play stops at the end', () => {
	assert.equal(clockLength([2000, 5000, null, NaN, 0]), 5000);
	assert.equal(clockLength([]), 0);
	assert.deepEqual(sideAt(1500, 2000), { ms: 1500, held: false });
	assert.deepEqual(sideAt(3000, 2000), { ms: 2000 - HOLD_MS, held: true }, 'past its end: its last frame');
	assert.deepEqual(sideAt(2000, 2000), { ms: 2000 - HOLD_MS, held: true });
	assert.deepEqual(sideAt(-5, 2000), { ms: 0, held: false });
	assert.deepEqual(sideAt(500, 0), { ms: 0, held: false }, 'no length yet: at its start');
	// the same moment in each: 1.2 s into a 2 s and a 5 s clip
	assert.deepEqual([2000, 5000].map((l) => sideAt(1200, l).ms), [1200, 1200]);
	assert.deepEqual(advance(1000, 16, 5000), { t: 1016, ended: false });
	assert.deepEqual(advance(4990, 16, 5000), { t: 5000, ended: true });
	assert.deepEqual(advance(1000, -5, 5000), { t: 1000, ended: false }, 'time never runs back');
	assert.equal(drifted(1000, 1100), false);
	assert.equal(drifted(1000, 1300), true);
});

test('the words: the time, how the main input went in, where it came from', () => {
	assert.equal(clockText(0), '0:00.0');
	assert.equal(clockText(2000), '0:02.0');
	assert.equal(clockText(65340), '1:05.3');
	assert.equal(clockText(-3), '0:00.0');
	assert.equal(roleWord({ role: 'still' }, 'image_to_video'), 'start frame');
	assert.equal(roleWord({ role: 'still' }, 'comfy'), 'still');
	assert.equal(roleWord({ role: 'control' }), 'video');
	assert.equal(roleWord(null), '');
	const r = { id: 's--g7', mode: 'image_to_video', inputs: [{ role: 'still', key: 'out:c/generated/s--g5.png' }], parents: ['s--g5'] };
	assert.equal(fromText(r, 'g5'), 'from g5 (start frame)');
	assert.equal(fromText({ ...r, mode: 'video_to_video', inputs: [{ role: 'control' }] }, 'take t3', ['g5']), 'from take t3 + g5 (video)');
	assert.equal(fromText({ ...r, earlier: true }, 'g5'), '', 'no record: nothing guessed');
	assert.equal(fromText({ id: 's--g1' }, ''), '');
});
