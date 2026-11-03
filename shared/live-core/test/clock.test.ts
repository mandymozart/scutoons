// #321: the clock's state block read into names (deck/live-core clock.rs `STATE`), and the line between two frames.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { beatAt, clockOf } from '../src/clock.ts';

test('the state block reads into the clock', () => {
	// heard at 1000 ms: beat 8 at 120 BPM in 3, on the relay, locked + anchor + an anchor to send, the click on
	const s = clockOf([1000, 8, 120, 3, 1, 4, 1 | 2 | 32, -3.5, 2.25, 0.4, 121, 3, 9.5, 5_000_000, 1, 1]);
	assert.equal(s.source, 'relay');
	assert.deepEqual([s.locked, s.anchor, s.leading, s.midiLost, s.ping, s.click], [true, true, false, false, false, true]);
	assert.deepEqual(s.send, { bpm: 121, perBar: 3, beat: 9.5, at: 5_000_000, running: true });
	assert.equal(beatAt(s, 1500), 9, 'half a second at two beats a second');
	const stopped = clockOf([1000, 8, 120, 4, 0, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	assert.equal(stopped.send, null);
	assert.equal(stopped.source, 'midi');
	assert.ok(stopped.midiLost);
	assert.equal(beatAt(stopped, 9000), 8, 'stopped: the beat stands');
});
