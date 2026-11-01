// `npm test -w shared/live-deck` (#228): a Gamepad API pad on the deck's sticks and selection, without a browser – the
// press and release of a stick (with the hysteresis), the deflection in stick radii with y up, A's press edge, a pad
// that goes away while a stick is down, the selection's cycle over master, the being and the knobs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PadSticks, nextSelection, selectionNames, type PadSink, type PadState } from '../src/gamepad.ts';

/** a stand-in pad: axes 0..3 (left x, y, right x, y), A pressed or not */
function pad(axes: [number, number, number, number] | [number, number], a = false): PadState {
	return { axes: [...axes, 0, 0].slice(0, 4), buttons: [{ pressed: a }] };
}

function recorder() {
	const calls: string[] = [];
	const sink: PadSink = {
		stickDown: (s) => calls.push(`down ${s}`),
		stickMove: (s, dx, dy) => calls.push(`move ${s} ${dx.toFixed(3)} ${dy.toFixed(3)}`),
		stickUp: (s) => calls.push(`up ${s}`)
	};
	return { calls, sink };
}

test('a stick pushed past the press point goes down, follows its deflection (y up), and goes up at the release point', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	p.poll([pad([0.1, 0])]);
	assert.deepEqual(calls, [], 'a resting stick does nothing');
	p.poll([pad([0.6, -0.8])]);
	assert.deepEqual(calls, ['down 0', 'move 0 0.600 0.800'], 'the axis y is down-positive: −0.8 is up 0.8');
	p.poll([pad([0.2, 0])]);
	assert.equal(calls[calls.length - 1], 'move 0 0.200 0.000', 'between the points it still follows');
	p.poll([pad([0.1, 0])]);
	assert.equal(calls[calls.length - 1], 'up 0', 'back inside the release point: up');
	p.poll([pad([0.1, 0])]);
	assert.equal(calls.length, 4, 'a resting stick after that does nothing more');
});

test('a stick that never reaches the press point never goes down', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	for (const x of [0.05, -0.12, 0.2, 0.25]) p.poll([pad([x, 0])]);
	assert.deepEqual(calls, []);
});

test('a full diagonal deflection is clamped to the unit circle', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	p.poll([pad([0, 0, 1, 1])]);
	assert.deepEqual(calls, ['down 1', 'move 1 0.707 -0.707']);
});

test('the right stick is the axes 2 and 3', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	p.poll([pad([0, 0, 0.9, 0])]);
	assert.deepEqual(calls, ['down 1', 'move 1 0.900 0.000']);
});

test('A selects on its press edge, not while held', () => {
	let selected = 0;
	const { sink } = recorder();
	const p = new PadSticks(sink, () => selected++);
	p.poll([pad([0, 0], true)]);
	p.poll([pad([0, 0], true)]);
	p.poll([pad([0, 0], false)]);
	p.poll([pad([0, 0], true)]);
	assert.equal(selected, 2);
});

test('no pad: the sticks that were down go up once, and nothing else happens', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	p.poll([pad([0.9, 0.9, 0.9, 0.9])]);
	calls.length = 0;
	p.poll([null, undefined]);
	assert.deepEqual(calls, ['up 0', 'up 1']);
	p.poll([]);
	assert.deepEqual(calls, ['up 0', 'up 1'], 'no second up');
});

test('the first connected pad drives the sticks', () => {
	const { calls, sink } = recorder();
	const p = new PadSticks(sink, () => {});
	p.poll([null, pad([0.5, 0])]);
	assert.deepEqual(calls, ['down 0', 'move 0 0.500 0.000']);
});

test('the selection cycles master → being → each knob → master', () => {
	const names = selectionNames(['a', 'b']);
	assert.deepEqual(names, ['', 'being', 'knob:a', 'knob:b']);
	let s = '';
	const seen: string[] = [];
	for (let i = 0; i < 5; i++) {
		s = nextSelection(s, names);
		seen.push(s);
	}
	assert.deepEqual(seen, ['being', 'knob:a', 'knob:b', '', 'being']);
	assert.equal(nextSelection('gone', names), '', 'an unknown selection starts at the first');
	assert.equal(nextSelection('x', []), 'x', 'nothing to cycle');
});
