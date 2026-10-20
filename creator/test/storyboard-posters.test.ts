// The variants board's pictures (src/lib/editor/storyboard-posters.ts, #125 – the port of #62's posters.test.mjs): the
// hover scrub – the pointer's x → the time there –, the time and the length as the chips say them.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { lengthText, scrubAt, secondsText } from '../src/lib/editor/storyboard-posters.ts';

test('the scrub: x = time', () => {
	assert.deepEqual(scrubAt(0, 5000), { share: 0, ms: 0 });
	assert.deepEqual(scrubAt(0.5, 5000), { share: 0.5, ms: 2500 });
	assert.deepEqual(scrubAt(1, 5000), { share: 1, ms: 5000 });
	assert.deepEqual(scrubAt(-0.2, 5000), { share: 0, ms: 0 }, 'outside the picture');
	assert.deepEqual(scrubAt(1.4, 5000), { share: 1, ms: 5000 });
	assert.deepEqual(scrubAt(NaN, 5000), { share: 0, ms: 0 });
	assert.deepEqual(scrubAt(0.6, null), { share: 0.6, ms: 0 }, 'no length known');
});

test('the time under the pointer and a length as the chips say them', () => {
	assert.equal(secondsText(1900), '1.9 s');
	assert.equal(secondsText(0), '0.0 s');
	assert.equal(secondsText(-3), '0.0 s');
	assert.equal(lengthText(5000), '5 s');
	assert.equal(lengthText(6240), '6.2 s');
	assert.equal(lengthText(null), '');
	assert.equal(lengthText(0), '');
});
