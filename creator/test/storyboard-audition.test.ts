// The audition (src/lib/editor/storyboard-audition.ts, #125 – the port of #62's audition.test.mjs): the current shot's
// Generated clip = the board's current card – a rejected one, a source, a drop, a running job, another shot's card: the
// chosen; the marks for the status bullets; the board's current card when none was clicked (its chosen, the video
// first); the audition holds while the board is open and while Play from it runs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { auditionPick, currentCard, holds, marksOf } from '../src/lib/editor/storyboard-audition.ts';
import type { BoardRecord } from '../src/lib/editor/storyboard-board-layout.ts';

const S = 'sb-25-2';
const rec = (n: number, f: Partial<BoardRecord>): BoardRecord => ({ id: n, shot: S, kind: 'image', take: null, parents: [], generator: 'runway', status: 'succeeded', approved: false, mark: null, file: `${S}--g${n}.png`, ...f });
const records = [
	rec(1, { kind: 'image', approved: true, generator: 'luma-image' }),
	rec(2, { kind: 'video', mark: 'review', parents: [1] }),
	rec(3, { kind: 'video', parents: [1] }),
	rec(4, { kind: 'image', parents: [1] }),
	rec(5, { kind: 'video', mark: 'rejected', parents: [1] }),
	rec(6, { kind: 'video', generator: 'upload' }),
	rec(7, { kind: 'video', status: 'running', file: null }),
	rec(9, { kind: 'video', approved: true })
];
const at = (key: string, shot = S) => ({ shot, key });

test('the pick: the current card of the current shot – its kind and status', () => {
	assert.deepEqual(auditionPick({ shot: S, current: at('2'), records }), { id: 2, video: true, status: 'review' });
	assert.deepEqual(auditionPick({ shot: S, current: at('4'), records }), { id: 4, video: false, status: null });
	assert.equal(auditionPick({ shot: S, current: at('9'), records })?.status, 'chosen');
});

test('no pick (the chosen plays): rejected, a drop, running, a source, another shot, none clicked', () => {
	for (const key of ['5', '6', '7', `render:${S}`, `take:${S}:t1`, '99']) assert.equal(auditionPick({ shot: S, current: at(key), records }), null, key);
	assert.equal(auditionPick({ shot: S, current: at('1', 'sb-25-1'), records }), null);
	assert.equal(auditionPick({ shot: S, current: null, records }), null);
	assert.equal(auditionPick({ shot: null, current: at('2'), records }), null);
	// a dropped file under a parent (a by-hand result) is a result – it can be auditioned
	assert.equal(auditionPick({ shot: S, current: at('6'), records: [rec(6, { kind: 'video', generator: 'upload', parents: [3] })] })?.id, 6);
});

test('the marks by id – the chosen\'s ✓ is its approval', () => {
	assert.deepEqual(marksOf(records), { 2: 'review', 5: 'rejected' });
	assert.deepEqual(marksOf([rec(1, { approved: true, mark: 'review' })]), {});
	assert.deepEqual(marksOf(undefined), {});
});

test('it holds while the board is open and while Play from it runs', () => {
	assert.deepEqual(holds({ open: true }), { on: true, carried: false });
	assert.deepEqual(holds({ open: false, was: true, byPlay: true, playing: true }), { on: true, carried: true });
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: true }), { on: true, carried: true });
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: false }), { on: false, carried: false }, 'Play ended');
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: true, stepped: true }), { on: false, carried: false }, 'another step');
	assert.deepEqual(holds({ open: false, was: true, byPlay: false, playing: false }), { on: false, carried: false }, 'V / the switch');
	assert.deepEqual(holds({ open: false, was: false, byPlay: true, playing: true }), { on: false, carried: false }, 'Play from the Viewport');
});

test('the board\'s current card: the last clicked there – else the chosen, the video first', () => {
	assert.deepEqual(currentCard({ shot: S, current: at('3'), records }), at('3'));
	assert.deepEqual(currentCard({ shot: S, current: null, records }), at('9'), 'the chosen video');
	assert.deepEqual(currentCard({ shot: S, current: at('1', 'sb-25-1'), records }), at('9'), 'another shot\'s card');
	assert.deepEqual(currentCard({ shot: S, current: at('99'), records }), at('9'), 'gone from the board');
	assert.deepEqual(currentCard({ shot: S, current: null, records: records.filter((r) => r.id !== 9) }), at('1'), 'no chosen video: the chosen image');
	assert.deepEqual(currentCard({ shot: S, current: at(`render:${S}`), records }), at(`render:${S}`), 'a source stays the current card');
	assert.equal(currentCard({ shot: S, current: null, records: [rec(3, { kind: 'video' })] }), null);
	assert.equal(currentCard({ shot: null, current: at('3'), records }), null);
	assert.deepEqual(currentCard({ shot: S, current: at('3'), records, has: () => false }), at('9'));
});
