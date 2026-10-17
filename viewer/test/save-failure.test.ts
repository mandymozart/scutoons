// Why saving a chapter failed, and what a download tries again (lib/offline/save-failure.ts; owner 2026-10-17: saving
// "often fails" on the phone with only a "!"):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { SaveError, failureOf, retryable } from '../src/lib/offline/save-failure.ts';

const domError = (name: string) => Object.assign(new Error(name), { name });

test('the reason the reader is told', () => {
	assert.equal(failureOf(new SaveError('server', 'x 403'), true), 'server');
	assert.equal(failureOf(domError('QuotaExceededError'), true), 'quota');
	assert.equal(failureOf(domError('QuotaExceededError'), false), 'quota', 'a full storage is said even offline');
	assert.equal(failureOf(domError('NS_ERROR_DOM_QUOTA_REACHED'), true), 'quota');
	assert.equal(failureOf(new TypeError('Failed to fetch'), false), 'offline');
	assert.equal(failureOf(new TypeError('NetworkError when attempting to fetch resource.'), true), 'network');
	assert.equal(failureOf(domError('SecurityError'), true), 'storage', 'a private window / blocked storage');
	assert.equal(failureOf(domError('UnknownError'), true), 'storage');
});

test('tried again: a broken connection, a timeout, an overloaded server – not a refusal', () => {
	assert.equal(retryable(new TypeError('Failed to fetch')), true);
	assert.equal(retryable(domError('QuotaExceededError')), false);
	assert.equal(retryable(new SaveError('server', 'x'), 503), true);
	assert.equal(retryable(new SaveError('server', 'x'), 408), true);
	assert.equal(retryable(new SaveError('server', 'x'), 429), true);
	assert.equal(retryable(new SaveError('server', 'x'), 403), false);
	assert.equal(retryable(domError('QuotaExceededError'), 200), false, 'the download was fine, storing it failed');
});
