// #208: the worker's adaptive lead – a dropout grows it a step up to the cap, a calm minute shrinks it a step.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { AdaptiveLead } from '../src/lead.ts';

test('a dropout grows the lead a step, up to the cap', () => {
	const l = new AdaptiveLead(2048, 3072, 512);
	assert.equal(l.update(0, 0), 2048, 'calm: the start');
	assert.equal(l.update(1_000, 1), 2560);
	assert.equal(l.update(1_100, 1), 2560, 'the same dropout counts once');
	assert.equal(l.update(2_000, 4), 3072, 'three more at once: one step per look');
	assert.equal(l.update(3_000, 5), 3072, 'the cap');
});

test('a calm minute shrinks it a step, never below the start', () => {
	const l = new AdaptiveLead(1024, 4096, 512);
	l.update(0, 0);
	l.update(10_000, 1);
	l.update(11_000, 2);
	assert.equal(l.value, 2048);
	assert.equal(l.update(70_000, 2), 2048, '59 s calm');
	assert.equal(l.update(71_000, 2), 1536, 'a minute after the last dropout');
	assert.equal(l.update(131_000, 2), 1024);
	assert.equal(l.update(500_000, 2), 1024, 'the start is the floor');
	assert.equal(l.update(501_000, 3), 1536, 'up again');
});
