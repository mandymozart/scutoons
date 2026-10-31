// The browser status's underruns per minute (#188): the worklet's running count, sampled once a second.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dropoutAlert, perMinute } from '../src/index.ts';

test('#208: the dropout warning holds five seconds', () => {
	assert.equal(dropoutAlert(-Infinity, 0), false, 'no dropout yet');
	assert.equal(dropoutAlert(1_000, 1_000), true);
	assert.equal(dropoutAlert(1_000, 5_999), true);
	assert.equal(dropoutAlert(1_000, 6_000), false);
});

test('underruns per minute: the count over the last 60 s', () => {
	const runs = [{ t: 0, n: 0 }];
	assert.equal(perMinute(runs, 0, 1_000), 0);
	assert.equal(perMinute(runs, 2, 2_000), 2, 'two in the first seconds');
	for (let s = 3; s <= 61; s++) perMinute(runs, 2, s * 1_000);
	assert.equal(perMinute(runs, 3, 62_000), 1, 'the first two are older than a minute now');
	assert.ok(runs.length <= 62, 'the window stays a minute long');
	assert.equal(perMinute(runs, 3, 200_000), 0);
});
