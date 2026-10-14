import assert from 'node:assert/strict';
import { test } from 'node:test';
import { notesFromTimemap, padEffect, panOf } from '../src/score/synth.ts';

test('the notes of a timemap: each start paired with its stop, sorted, without pitchless ids', () => {
	const pitches: Record<string, number> = { a: 60, b: 64, c: 67 };
	const notes = notesFromTimemap(
		[
			{ tstamp: 0, on: ['a', 'rest'] },
			{ tstamp: 500, on: ['b'], off: ['a', 'rest'] },
			{ tstamp: 750, on: ['c'] },
			{ tstamp: 1000, off: ['b', 'c'] }
		],
		(id) => pitches[id] ?? null
	);
	assert.deepEqual(notes, [
		{ id: 'a', at: 0, dur: 500, pitch: 60 },
		{ id: 'b', at: 500, dur: 500, pitch: 64 },
		{ id: 'c', at: 750, dur: 250, pitch: 67 }
	]);
});

test('a stop without a start is ignored; a very short note still sounds a little', () => {
	const notes = notesFromTimemap([{ tstamp: 0, off: ['x'] }, { tstamp: 10, on: ['y'] }, { tstamp: 12, off: ['y'] }], () => 60);
	assert.deepEqual(notes, [{ id: 'y', at: 10, dur: 30, pitch: 60 }]);
});

test('the pad: left = short slap-back, right = long ringing echo; top all echo, bottom dry; outside is clamped', () => {
	const left = padEffect(0, 1);
	const right = padEffect(1, 0);
	assert.ok(Math.abs(left.time - 0.06) < 1e-9 && Math.abs(right.time - 0.9) < 1e-9);
	assert.ok(left.feedback < right.feedback && right.feedback < 0.8);
	assert.equal(left.wet, 0);
	assert.equal(right.wet, 1);
	assert.deepEqual(padEffect(-3, 5), padEffect(0, 1));
});

test('the pan spreads low voices left and high ones right, never fully', () => {
	assert.ok(panOf(40) === -0.5 && panOf(62) === 0 && panOf(96) === 0.5);
});
