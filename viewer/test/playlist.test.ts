import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ARM_REST, armAngle, nextTrack, previousTrack } from '../src/lib/components/items/playlist.ts';

test('the album plays in order and stops after the last song', () => {
	assert.equal(nextTrack(0, 6), 1);
	assert.equal(nextTrack(4, 6), 5);
	assert.equal(nextTrack(5, 6), null);
	assert.equal(nextTrack(0, 1), null);
});

test('back: restarts a song a few seconds in, else the one before; the first song only restarts', () => {
	assert.equal(previousTrack(3, 10), 3);
	assert.equal(previousTrack(3, 1), 2);
	assert.equal(previousTrack(0, 1), 0);
});

test('the tonearm rests off the record, then moves in from the outer groove as the song plays', () => {
	assert.equal(armAngle(false, 0.5), ARM_REST);
	assert.equal(armAngle(true, 0), 0);
	assert.ok(armAngle(true, 0.5) > 0 && armAngle(true, 0.5) < armAngle(true, 1));
	assert.equal(armAngle(true, 2), armAngle(true, 1));
	assert.equal(armAngle(true, NaN), 0);
});
