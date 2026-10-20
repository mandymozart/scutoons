// Compare (src/lib/editor/storyboard-compare.ts, #125 – the port of #62's compare.test.mjs): the sides it opens with,
// a click while it is open (the LAST side replaced, never A; the empty side filled; a rejected card stays out), a side
// taken out (its place empty while open; closed, the rest close up), a card gone from the board, the sides at one
// height in their aspects, the one clock, the words.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { addSide, advance, chosenKey, clickSide, clockLength, clockText, drifted, filled, fitSides, HOLD_MS, lettersOf, MAX_SIDES, resolveSides, roleWord, sideAt, sidesOnOpen, takeOut, type Side } from '../src/lib/editor/storyboard-compare.ts';

type R = { id: number; kind: 'image' | 'video'; approved: boolean; status: 'succeeded' | 'failed' };
const rec = (n: number, f: Partial<R> = {}): R => ({ id: n, kind: 'image', approved: false, status: 'succeeded', ...f });
const card = (key: string, shot = 's'): Side => ({ shot, key });

test('the chosen: of the asked kind, else the chosen video, else any; never a failed one', () => {
	const records = [rec(1, { approved: true }), rec(2, { kind: 'video', approved: true }), rec(3)];
	assert.equal(chosenKey(records, 'image'), '1');
	assert.equal(chosenKey(records, 'video'), '2');
	assert.equal(chosenKey(records, null), '2', 'no kind: the video first');
	assert.equal(chosenKey([rec(1, { approved: true })], 'video'), '1');
	assert.equal(chosenKey([rec(1, { approved: true, status: 'failed' })], 'image'), null);
	assert.equal(chosenKey([rec(1)], 'image'), null);
	assert.equal(chosenKey(undefined, null), null);
});

test('opening: 2–4 selected are the sides, A the first', () => {
	const sel = [card('3'), card('11', 't'), card('5')];
	assert.deepEqual(sidesOnOpen({ selected: sel, shot: 's', records: [rec(1, { approved: true })] }).sides, sel);
	assert.equal(sidesOnOpen({ selected: ['a', 'b', 'c', 'd', 'e'].map((k) => card(k)), shot: 's' }).sides.length, MAX_SIDES);
});

test('opening with one selected: the current shot\'s chosen of its kind is A', () => {
	const records = [rec(1, { approved: true }), rec(2, { kind: 'video', approved: true }), rec(4, { kind: 'video' })];
	const kindOf = (c: Side) => records.find((r) => String(r.id) === c.key)?.kind ?? null;
	assert.deepEqual(sidesOnOpen({ selected: [card('4')], shot: 's', records, kindOf }).sides, [card('2'), card('4')]);
	assert.deepEqual(sidesOnOpen({ selected: [card('7', 't')], shot: 's', records, kindOf: () => 'image' }).sides, [card('1'), card('7', 't')], 'another shot\'s card: the current shot\'s chosen as A');
	assert.deepEqual(sidesOnOpen({ selected: [card('2')], shot: 's', records, kindOf }).sides, [card('2')]);
	assert.deepEqual(sidesOnOpen({ selected: [card('4')], shot: 's', records: [rec(4, { kind: 'video' })], kindOf }).sides, [card('4')]);
});

test('opening with nothing selected: the chosen beside the current card – or why not', () => {
	const records = [rec(1, { approved: true }), rec(3)];
	assert.deepEqual(sidesOnOpen({ shot: 's', records, current: { shot: 's', key: '3', kind: 'image' } }).sides, [card('1'), card('3')]);
	assert.deepEqual(sidesOnOpen({ shot: 's', records, current: { shot: 's', key: '3', kind: 'image', rejected: true } }).sides, [card('1')]);
	assert.deepEqual(sidesOnOpen({ shot: 's', records: [rec(3)], current: { shot: 's', key: '3' } }).sides, [card('3')]);
	assert.deepEqual(sidesOnOpen({ shot: 's', records: [rec(3)] }), { sides: [], why: 'select' });
});

test('a click while Compare is open replaces the LAST side – A stays; the empty side fills; rejected stays out', () => {
	const ab = [card('a'), card('b')];
	assert.deepEqual(clickSide(ab, card('c')).selected, [card('a'), card('c')]);
	const abcd = [card('a'), card('b'), card('c'), card('d')];
	assert.deepEqual(clickSide(abcd, card('e')).selected, [card('a'), card('b'), card('c'), card('e')]);
	assert.equal(clickSide(ab, card('a')).selected, ab);
	assert.deepEqual(clickSide([card('a')], card('b')).selected, ab);
	assert.deepEqual(clickSide([], card('a')).selected, [card('a')]);
	assert.deepEqual(clickSide(ab, card('x'), true), { selected: ab, why: 'rejected' });
});

test('a side taken out while Compare is open stays empty – the letters stay; closed, they close up', () => {
	const abcd = [card('a'), card('b'), card('c'), card('d')];
	const holed = takeOut(abcd, ['c', 'x'], true);
	assert.deepEqual(holed, [card('a'), card('b'), null, card('d')]);
	assert.deepEqual([...lettersOf(holed)], [['a', 'A'], ['b', 'B'], ['d', 'D']]);
	assert.deepEqual(clickSide(holed, card('e')).selected, [card('a'), card('b'), card('e'), card('d')]);
	assert.deepEqual(clickSide(takeOut(abcd, ['a'], true), card('e')).selected, [card('e'), card('b'), card('c'), card('d')]);
	assert.deepEqual(clickSide(takeOut(abcd, ['b', 'd'], true), card('e')).selected, [card('a'), card('e'), card('c'), null]);
	assert.deepEqual(addSide(holed, card('e')), [card('a'), card('b'), card('e'), card('d')]);
	assert.deepEqual(addSide(abcd, card('e')), [card('a'), card('b'), card('c'), card('e')]);
	assert.deepEqual(takeOut(abcd, ['b'], false), [card('a'), card('c'), card('d')]);
	assert.deepEqual(filled(holed), [card('a'), card('b'), card('d')]);
});

test('a card gone from the board is an empty side – it never hides or replaces A', () => {
	const sel = [card('9'), card('3'), card('5')];
	const exists = (c: Side) => c.key !== '9';
	const sides = resolveSides(sel, exists);
	assert.deepEqual(sides, [null, card('3'), card('5')]);
	assert.deepEqual([...lettersOf(filled(sides))], [['3', 'A'], ['5', 'B']]);
	assert.deepEqual(clickSide(sides, card('4')).selected, [card('4'), card('3'), card('5')]);
	assert.deepEqual(resolveSides(undefined), []);
});

test('the sides: one height, each in its aspect, inside the pane', () => {
	const two = fitSides({ ratios: [9 / 16, 9 / 16], width: 27, height: 40, gap: 1, minCol: 6 });
	assert.ok(Math.abs(two.h - 23.11) < 0.011, `h ${two.h}`);
	const tall = fitSides({ ratios: [9 / 16, 9 / 16], width: 60, height: 20, gap: 1, minCol: 6 });
	assert.deepEqual(tall, { h: 20, widths: [11.25, 11.25] });
	const mixed = fitSides({ ratios: [1 / 3, 9 / 16], width: 27, height: 40, gap: 1, minCol: 8 });
	assert.ok(mixed.widths[0] >= 8 - 1e-9 && mixed.widths[0] + mixed.widths[1] + 1 <= 27 + 1e-9);
	const four = fitSides({ ratios: [9 / 16, 1 / 3, 9 / 16, 9 / 16], width: 20, height: 30, gap: 1, minCol: 8 });
	assert.ok(four.widths.reduce((s, w) => s + w, 0) + 3 <= 20 + 1e-9 && four.h > 0);
	assert.deepEqual(fitSides({ ratios: [1], width: 0, height: 10 }), { h: 0, widths: [0] });
});

test('one clock: as long as the longest; a shorter video holds its last frame; play stops at the end', () => {
	assert.equal(clockLength([2000, 5000, null, NaN, 0]), 5000);
	assert.equal(clockLength([]), 0);
	assert.deepEqual(sideAt(1500, 2000), { ms: 1500, held: false });
	assert.deepEqual(sideAt(3000, 2000), { ms: 2000 - HOLD_MS, held: true });
	assert.deepEqual(sideAt(500, 0), { ms: 0, held: false });
	assert.deepEqual(advance(4990, 16, 5000), { t: 5000, ended: true });
	assert.deepEqual(advance(1000, -5, 5000), { t: 1000, ended: false });
	assert.equal(drifted(1000, 1100), false);
	assert.equal(drifted(1000, 1300), true);
});

test('the words: the time, how the main input went in', () => {
	assert.equal(clockText(0), '0:00.0');
	assert.equal(clockText(65340), '1:05.3');
	assert.equal(roleWord({ role: 'still' }, 'image_to_video'), 'start');
	assert.equal(roleWord({ role: 'still' }, 'image_edit'), 'still');
	assert.equal(roleWord({ role: 'control' }, null), 'video');
	assert.equal(roleWord({ role: 'character' }, null), 'other');
	assert.equal(roleWord(null, null), '');
});
