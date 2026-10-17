import assert from 'node:assert/strict';
import { test } from 'node:test';
import { marqueeRun } from '../src/lib/components/items/marquee.ts';

test('a line that fits (or a hidden bar measuring 0) stands still', () => {
	assert.equal(marqueeRun(0), null);
	assert.equal(marqueeRun(1), null);
	assert.equal(marqueeRun(-40), null);
	assert.equal(marqueeRun(Number.NaN), null);
	assert.equal(marqueeRun(50, 0), null);
});

test('an overflowing line: a pause, through, a pause, back', () => {
	const run = marqueeRun(56, 28, 2000);
	assert.ok(run);
	// 56 px at 28 px/s = 2 s each way + two 2 s pauses
	assert.equal(run.duration, 8000);
	assert.deepEqual(
		run.keyframes.map((k) => [k.offset, k.transform]),
		[
			[0, 'translateX(0px)'],
			[0.25, 'translateX(0px)'],
			[0.5, 'translateX(-56px)'],
			[0.75, 'translateX(-56px)'],
			[1, 'translateX(0px)']
		]
	);
});
