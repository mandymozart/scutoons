// #321: the clock's state block read into names (deck/live-core clock.rs `STATE`), and the line between two frames.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ClockWindow, beatAt, clockOf } from '../src/clock.ts';

test('the state block reads into the clock', () => {
	// heard at 1000 ms: beat 8 at 120 BPM in 3, on the relay, locked + anchor + an anchor to send, the click on
	const s = clockOf([1000, 8, 120, 3, 1, 4, 1 | 2 | 32, -3.5, 2.25, 0.4, 121, 3, 9.5, 5_000_000, 1, 1, 0, 1, 980, 60]);
	assert.equal(s.source, 'relay');
	assert.deepEqual([s.mode, s.midiOut, s.flash], [0, true, { atMs: 980, ms: 60 }], '#370: the choice, the clock out, the flash');
	assert.deepEqual([s.locked, s.anchor, s.leading, s.midiLost, s.ping, s.click], [true, true, false, false, false, true]);
	assert.deepEqual(s.send, { bpm: 121, perBar: 3, beat: 9.5, at: 5_000_000, running: true });
	assert.equal(beatAt(s, 1500), 9, 'half a second at two beats a second');
	const stopped = clockOf([1000, 8, 120, 4, 0, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0, 0, 0]);
	assert.equal(stopped.send, null);
	assert.deepEqual([stopped.mode, stopped.midiOut, stopped.flash], [-1, false, null], 'none of the four, no flash while it stands');
	assert.equal(stopped.source, 'midi');
	assert.ok(stopped.midiLost);
	assert.equal(beatAt(stopped, 9000), 8, 'stopped: the beat stands');
});

test("the clock window keeps the last minute of a follower's error – not a leader's, not before the anchor", () => {
	const at = (errorMs: number, flags = 1 | 2) => clockOf([0, 0, 84, 4, 1, 4, flags, 0, 1, errorMs, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	const w = new ClockWindow(60_000);
	assert.equal(w.summary(), null);
	w.add(0, at(50, 1)); // no anchor yet
	w.add(0, at(50, 1 | 2 | 4)); // leads
	w.add(0, at(1_023_277.9)); // the join's first frame, before the snap onto the show's line
	assert.equal(w.summary(), null);
	for (let i = 0; i < 100; i++) w.add(1000 + i * 25, at(i % 2 ? 1 : -1));
	w.add(3000, at(-80)); // a snap after the line was reached: it counts
	w.add(4000, at(-9));
	const s = w.summary()!;
	assert.equal(s.n, 102);
	assert.deepEqual([s.p50, s.p95, s.max], [1, 1, 80]);
	assert.ok(Math.abs(s.mean - -89 / 102) < 1e-9);
	w.add(70_000, at(2)); // a minute later: the old frames are gone
	assert.deepEqual([w.summary()!.n, w.summary()!.max], [1, 2]);
});
