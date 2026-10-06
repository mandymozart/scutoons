// The one recurrence model (agents/EXPRESSIONS.md §5): intervals + periods (placed items' respawn) and the events'
// repeat rule against the history.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { intervalMinutes, isDue, periodOf, repeatAsRespawn, repeatOf, repeatProblem, respawnAsRepeat, type PageView } from '../src/index.ts';

test('intervals: visit, minutes, hours, days', () => {
	assert.equal(intervalMinutes('visit'), 0);
	assert.equal(intervalMinutes({ minutes: 30 }), 30);
	assert.equal(intervalMinutes({ hours: 2 }), 120);
	assert.equal(intervalMinutes({ days: 1 }), 1440);
	for (const bad of ['day', { hours: 0 }, { hours: 1.5 }, { hours: 1, minutes: 5 }, { weeks: 1 }, { days: 400 }, null, 3]) assert.equal(intervalMinutes(bad), null, JSON.stringify(bad));
});

test('periods: a page view id or a fixed time bucket', () => {
	assert.equal(periodOf('visit', 0, 'abc'), 'v:abc');
	const hour = 3_600_000;
	assert.equal(periodOf({ hours: 2 }, 5 * hour, 'x'), 'm120:2');
	assert.equal(periodOf({ hours: 2 }, 5.9 * hour, 'x'), 'm120:2');
	assert.equal(periodOf({ hours: 2 }, 6 * hour, 'x'), 'm120:3');
});

test('repeat: the stored rule, none = every time', () => {
	assert.deepEqual(repeatOf(undefined), { mode: 'every' });
	assert.deepEqual(repeatOf({ mode: 'view' }), { mode: 'view' });
	assert.deepEqual(repeatOf({ mode: 'interval', every: { minutes: 5 } }), { mode: 'interval', every: { minutes: 5 } });
	assert.deepEqual(repeatOf({ mode: 'nope' }), { mode: 'every' }, 'a broken rule runs every time (validation says so)');
	assert.equal(repeatProblem(undefined), null);
	assert.equal(repeatProblem({ mode: 'once' }), null);
	assert.match(repeatProblem(true) ?? '', /repeat must be/);
	assert.match(repeatProblem({ mode: 'interval' }) ?? '', /needs every/);
	assert.match(repeatProblem({ mode: 'interval', every: 'visit' }) ?? '', /needs every/, 'a page view is mode view');
	assert.match(repeatProblem({ mode: 'view', every: { hours: 1 } }) ?? '', /belongs to mode interval/);
	assert.match(repeatProblem({ mode: 'once', x: 1 }) ?? '', /repeat.x is unknown/);
});

test('isDue: every time, once per page view, once for good, every n …', () => {
	const minute = 60_000;
	const view: PageView = { id: 'v1', start: 100 * minute };
	const now = 105 * minute;
	const ranInView = { n: 3, last: 101 * minute };
	const ranBefore = { n: 3, last: 99 * minute };
	// never ran: always due
	for (const mode of ['every', 'view', 'once'] as const) assert.equal(isDue({ mode }, undefined, now, view), true, mode);
	assert.equal(isDue({ mode: 'interval', every: { minutes: 10 } }, null, now, view), true);
	assert.equal(isDue(undefined, ranInView, now, view), true, 'no rule = every time');
	assert.equal(isDue({ mode: 'every' }, ranInView, now, view), true);
	assert.equal(isDue({ mode: 'view' }, ranInView, now, view), false, 'ran in this page view');
	assert.equal(isDue({ mode: 'view' }, ranBefore, now, view), true, 'ran in an earlier page view (a reload)');
	assert.equal(isDue({ mode: 'once' }, ranBefore, now, view), false, 'once for good: never again');
	assert.equal(isDue({ mode: 'once' }, { n: 0, last: 0 }, now, view), true);
	// every 10 minutes: buckets 100–109, 110–119 … (the same as an item's respawn)
	const ten = { mode: 'interval', every: { minutes: 10 } } as const;
	assert.equal(isDue(ten, { n: 1, last: 101 * minute }, now, view), false, 'same bucket');
	assert.equal(isDue(ten, { n: 1, last: 99 * minute }, now, view), true, 'the bucket before');
	assert.equal(isDue(ten, { n: 1, last: 101 * minute }, 110 * minute, view), true, 'the next bucket');
});

test('pickups in the same terms (the creator\'s one Repeats field)', () => {
	assert.deepEqual(respawnAsRepeat(null), { mode: 'once' });
	assert.deepEqual(respawnAsRepeat('visit'), { mode: 'view' });
	assert.deepEqual(respawnAsRepeat({ hours: 4 }), { mode: 'interval', every: { hours: 4 } });
	assert.equal(repeatAsRespawn({ mode: 'once' }), 'never');
	assert.equal(repeatAsRespawn({ mode: 'view' }), 'visit');
	assert.deepEqual(repeatAsRespawn({ mode: 'interval', every: { days: 1 } }), { days: 1 });
	assert.equal(repeatAsRespawn({ mode: 'every' }), null);
});
