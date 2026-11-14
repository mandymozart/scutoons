// #231: the Live vote demo's bots (src/lib/live-vote-demo.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { changesMind, pickOption, voteDelay } from '../src/lib/live-vote-demo.ts';

test('pickOption follows the weights', () => {
	assert.equal(pickOption([1, 3], 2, 0), 0);
	assert.equal(pickOption([1, 3], 2, 0.5), 1);
	assert.equal(pickOption([0, 1], 2, 0.01), 1);
	assert.equal(pickOption([], 3, 0.9), 2, 'no weights: the last');
});

test('a phone votes early rather than late, never after 80 % of the time', () => {
	assert.equal(voteDelay(10, 0), 0);
	assert.ok(voteDelay(10, 1) <= 8000);
	assert.ok(voteDelay(10, 0.5) < voteDelay(10, 1) / 2);
	assert.equal(changesMind(0.05), true);
	assert.equal(changesMind(0.5), false);
});
