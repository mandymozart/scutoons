// #356: the block-time histogram behind the stats' p50 / p99.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { BlockTimes } from '../src/timing.ts';

test('p50 and p99 within a bin (≈ 9 %), the longest exact, the mean from the sum', () => {
	const t = new BlockTimes();
	assert.equal(t.quantile(0.5), 0, 'empty');
	for (let i = 0; i < 980; i++) t.push(100);
	for (let i = 0; i < 19; i++) t.push(1_000);
	t.push(9_800);
	const p50 = t.quantile(0.5);
	assert.ok(p50 >= 100 && p50 < 110, `p50 ${p50}`);
	const p99 = t.quantile(0.99);
	assert.ok(p99 >= 1_000 && p99 < 1_100, `p99 ${p99}`);
	assert.equal(t.quantile(1), 9_800);
	assert.equal(t.max, 9_800);
	assert.equal(Math.round(t.mean), Math.round((980 * 100 + 19 * 1_000 + 9_800) / 1_000));
});

test('tiny and huge blocks land in the edge bins; a quantile never passes the longest; reset empties', () => {
	const t = new BlockTimes();
	t.push(0.5);
	t.push(1e6);
	assert.ok(t.quantile(0.5) <= 5);
	assert.equal(t.quantile(1), 1e6);
	const one = new BlockTimes();
	one.push(50);
	assert.equal(one.quantile(0.99), 50);
	t.reset();
	assert.equal(t.count, 0);
	assert.equal(t.max, 0);
	assert.equal(t.quantile(0.5), 0);
});
