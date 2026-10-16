import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ARM_REST, armAngle, nextTrack, previousTrack } from '../src/lib/components/items/playlist.ts';

test('the album plays in order and starts again after the last song', () => {
	assert.equal(nextTrack(0, 6), 1);
	assert.equal(nextTrack(4, 6), 5);
	assert.equal(nextTrack(5, 6), 0);
	assert.equal(nextTrack(0, 1), 0);
	assert.equal(nextTrack(0, 0), null);
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

test('the seek bar: times as m:ss, the length from the file, else the catalog', async () => {
	const { formatTime, songLength } = await import('../src/lib/components/items/playlist.ts');
	assert.equal(formatTime(0), '0:00');
	assert.equal(formatTime(65.9), '1:05');
	assert.equal(formatTime(475), '7:55');
	assert.equal(formatTime(NaN), '–:––');
	assert.equal(songLength(417.24, 417), 417.24);
	assert.equal(songLength(NaN, 417), 417);
	assert.equal(songLength(Infinity, undefined), 0);
	assert.equal(songLength(0, 0), 0);
});

test('record sides: playing on stays on the side, turning goes to the other side’s first song, no side = A', async () => {
	const { sideIndices, nextOnSide, previousOnSide, turnTo, twoSided, sideOf } = await import('../src/lib/components/items/playlist.ts');
	// Awakening (owner 2026-10-16): Roar, Cloud, Not My City on A, the rest on B
	const album = [{ side: 'A' }, { side: 'A' }, {}, { side: 'B' }, { side: 'B' }, { side: 'B' }];
	assert.equal(sideOf(album[2]), 'A');
	assert.deepEqual(sideIndices(album, 'A'), [0, 1, 2]);
	assert.deepEqual(sideIndices(album, 'B'), [3, 4, 5]);
	assert.ok(twoSided(album));
	assert.ok(!twoSided([{}, { side: 'A' }]));
	assert.equal(nextOnSide(album, 0), 1);
	assert.equal(nextOnSide(album, 2), 0);
	assert.equal(nextOnSide(album, 3), 4);
	assert.equal(nextOnSide(album, 5), 3);
	assert.equal(previousOnSide(album, 4, 1), 3);
	assert.equal(previousOnSide(album, 3, 1), 3);
	assert.equal(previousOnSide(album, 4, 10), 4);
	assert.equal(turnTo(album, 1), 3);
	assert.equal(turnTo(album, 5), 0);
	assert.equal(turnTo([{}, {}], 0), null);
	// one side only: the album loop as before
	assert.equal(nextOnSide([{}, {}, {}], 2), 0);
});
