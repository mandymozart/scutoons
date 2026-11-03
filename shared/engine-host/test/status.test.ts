// `npm test -w @scutoons/engine-host` (#298): the phases and how a page words them
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { canStart, canStop, emptySnapshot, transportNote } from '../src/status.ts';

test('Start is live once the deck exists – the sound does not wait for the 3D – and no sound runs; after an error it tries again', () => {
	assert.deepEqual((['idle', 'mounting', 'ready', 'starting', 'running', 'stopped', 'error'] as const).filter(canStart), ['mounting', 'ready', 'stopped', 'error']);
	assert.deepEqual((['idle', 'mounting', 'ready', 'starting', 'running', 'stopped', 'error'] as const).filter(canStop), ['running']);
});

test('the transport in a sentence says why it is the slow one', () => {
	assert.match(transportNote('shared', true), /SharedArrayBuffer/);
	assert.match(transportNote('transfer', false), /not cross-origin isolated/);
	assert.match(transportNote(null, false), /not started.*transfer fallback/);
	assert.match(transportNote(null, true), /not started.*shared rings/);
});

test('a fresh snapshot has no monitor report yet', () => {
	assert.equal(emptySnapshot().stats, null);
});
