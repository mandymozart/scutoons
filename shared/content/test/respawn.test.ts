// Respawn of placed items: item rule (catalog) + placement override.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { effectiveRespawn, intervalMinutes, itemRespawnProblems, placementRespawnProblem, respawnPeriod, respawnRange } from '../src/index.ts';

test('respawn intervals: visit, minutes, hours, days', () => {
	assert.equal(intervalMinutes('visit'), 0);
	assert.equal(intervalMinutes({ minutes: 30 }), 30);
	assert.equal(intervalMinutes({ hours: 2 }), 120);
	assert.equal(intervalMinutes({ days: 1 }), 1440);
	for (const bad of ['day', { hours: 0 }, { hours: 1.5 }, { hours: 1, minutes: 5 }, { weeks: 1 }, { days: 400 }, null, 3]) assert.equal(intervalMinutes(bad), null, JSON.stringify(bad));
});

test('item rules: default within min … max; null = never', () => {
	assert.deepEqual(itemRespawnProblems(null), []);
	assert.deepEqual(itemRespawnProblems({ default: { hours: 4 }, min: { hours: 1 }, max: { days: 1 } }), []);
	assert.deepEqual(itemRespawnProblems({ default: 'visit', min: 'visit' }), []);
	assert.equal(itemRespawnProblems({ default: { hours: 4 }, min: { hours: 6 } }).length, 1);
	assert.equal(itemRespawnProblems({ default: 'x' }).length, 1);
	assert.deepEqual(respawnRange({ default: { hours: 4 } }), { min: 240, max: 365 * 1440 }, 'no min = the default is the shortest');
});

test('placement: the item default, an override within the range, never', () => {
	const tuna = { default: { hours: 4 }, min: { hours: 1 }, max: { days: 1 } } as const;
	assert.deepEqual(effectiveRespawn(undefined, tuna), { hours: 4 });
	assert.deepEqual(effectiveRespawn({ hours: 2 }, tuna), { hours: 2 });
	assert.deepEqual(effectiveRespawn({ minutes: 5 }, tuna), { hours: 4 }, 'out of range → the default');
	assert.equal(effectiveRespawn('never', tuna), null);
	assert.equal(effectiveRespawn({ hours: 2 }, null), null, 'a one-time item never reappears');
	assert.equal(placementRespawnProblem({ minutes: 5 }, tuna), 'shorter than the item allows (catalog respawn.min)');
	assert.equal(placementRespawnProblem({ hours: 2 }, undefined), null, 'catalog unknown: only the shape');
});

test('periods: a visit id or a fixed time bucket', () => {
	assert.equal(respawnPeriod('visit', 0, 'abc'), 'v:abc');
	const hour = 3_600_000;
	assert.equal(respawnPeriod({ hours: 2 }, 5 * hour, 'x'), 'm120:2');
	assert.equal(respawnPeriod({ hours: 2 }, 5.9 * hour, 'x'), 'm120:2');
	assert.equal(respawnPeriod({ hours: 2 }, 6 * hour, 'x'), 'm120:3');
});
