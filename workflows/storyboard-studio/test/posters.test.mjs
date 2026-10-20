// The variants board's pictures (src/lib/posters.js, agents/research/VARIANTS-CANVAS.md §3.3): the poster's height in
// steps, the hover scrub – the pointer's x → the sprite's tile and the time there, the time as the length chip says it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pxStep, SCRUB_TILES, scrubAt, secondsText } from '../src/lib/posters.js';

test('the poster\'s height: steps of 1.5×, the first at least as high', () => {
	assert.equal(pxStep(10), 64);
	assert.equal(pxStep(64), 64);
	assert.equal(pxStep(65), 96);
	assert.equal(pxStep(130), 144);
});

test('the scrub: x = time – the tile under the pointer (tile j = the frame at j · length / n), the time there', () => {
	assert.equal(SCRUB_TILES, 32);
	assert.deepEqual(scrubAt(0, 32, 5000), { tile: 0, ms: 0 });
	assert.deepEqual(scrubAt(0.5, 32, 5000), { tile: 16, ms: 2500 });
	assert.deepEqual(scrubAt(1, 32, 5000), { tile: 31, ms: 5000 }, 'the right edge: the last tile');
	assert.deepEqual(scrubAt(0.999, 4, 4000), { tile: 3, ms: 3996 });
	assert.deepEqual(scrubAt(0.24, 4, 4000), { tile: 0, ms: 960 });
	assert.deepEqual(scrubAt(0.25, 4, 4000), { tile: 1, ms: 1000 });
	// outside the picture (a pointer on its frame), nonsense, no sprite yet (n = 1), no length known
	assert.deepEqual(scrubAt(-0.2, 32, 5000), { tile: 0, ms: 0 });
	assert.deepEqual(scrubAt(1.4, 32, 5000), { tile: 31, ms: 5000 });
	assert.deepEqual(scrubAt(NaN, 32, 5000), { tile: 0, ms: 0 });
	assert.deepEqual(scrubAt(0.6, 1, 5000), { tile: 0, ms: 3000 });
	assert.deepEqual(scrubAt(0.6, 32, null), { tile: 19, ms: 0 });
});

test('the time under the pointer as the chip says it', () => {
	assert.equal(secondsText(1900), '1.9 s');
	assert.equal(secondsText(0), '0.0 s');
	assert.equal(secondsText(5000), '5.0 s');
	assert.equal(secondsText(-3), '0.0 s');
});
