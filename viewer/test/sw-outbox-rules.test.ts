// Reading progress made offline (lib/sw/outbox-rules.ts, agents/research/PWA.md §2 "Offline reading progress"): the
// outbox keeps the newest write, a sent write settles what it covers, what an answer means, when a failed push is queued.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { newer, progressUrl, queueAfter, sendOutcome, settledBy, type QueuedProgress } from '../src/lib/sw/outbox-rules.ts';

const write = (updatedAt: number, state: unknown = { at: updatedAt }): QueuedProgress => ({ id: 'scutoons', state, updatedAt, queuedAt: updatedAt });

test('one write per id: the newer one stays (the device state is a superset of the older)', () => {
	assert.equal(newer(undefined, write(5)).updatedAt, 5);
	assert.equal(newer(write(5), write(7)).updatedAt, 7);
	assert.equal(newer(write(9), write(7)).updatedAt, 9, 'an older write never replaces a newer one');
	assert.deepEqual(newer(write(5, 'a'), write(5, 'b')).state, 'b', 'the same time: the later call');
});

test('a sent write settles every queued one up to its time – not a newer one queued meanwhile', () => {
	assert.ok(settledBy(write(5), 5));
	assert.ok(settledBy(write(4), 5));
	assert.ok(!settledBy(write(6), 5));
});

test('what an answer means: done, try again, leave it for the page, or drop', () => {
	assert.equal(sendOutcome(200), 'done');
	assert.equal(sendOutcome('network'), 'retry');
	assert.equal(sendOutcome(503), 'retry');
	assert.equal(sendOutcome(429), 'retry');
	assert.equal(sendOutcome(408), 'retry');
	assert.equal(sendOutcome(401), 'page', 'no cookie session in the worker: the page sends it');
	assert.equal(sendOutcome(403), 'page');
	assert.equal(sendOutcome(400), 'drop', 'a broken state is never sent again');
	assert.equal(sendOutcome(422), 'drop');
});

test('a failed push is queued when the network or the server failed – not for a refused write', () => {
	assert.ok(queueAfter(new TypeError('Failed to fetch')));
	assert.ok(queueAfter({ status: 0, code: 'network' }), 'ApiError network');
	assert.ok(queueAfter({ status: 502 }));
	assert.ok(!queueAfter({ status: 401 }));
	assert.ok(!queueAfter({ status: 400 }));
});

test('the URL is the one @scutoons/api putProgress writes', () => {
	assert.equal(progressUrl('https://api.scutoons.com/api/', 'scutoons'), 'https://api.scutoons.com/api/progress/scutoons/');
	assert.equal(progressUrl('http://localhost:9000/api', 'a b'), 'http://localhost:9000/api/progress/a%20b/');
});
