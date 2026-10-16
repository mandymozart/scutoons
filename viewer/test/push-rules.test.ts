// Web Push in the worker – the payload, the tap's target, the badge (lib/sw/push-rules.ts, agents/research/PWA.md §2b):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nextBadge, noticeOf, safePath } from '../src/lib/sw/push-rules.ts';

test('the server payload shows as sent', () => {
	const notice = noticeOf(JSON.stringify({ title: 'Catpope: new chapter', body: 'Eins – read it now', url: '/en/catpope/chapter/1', tag: 'chapter:catpope:1' }));
	assert.deepEqual(notice, { title: 'Catpope: new chapter', body: 'Eins – read it now', url: '/en/catpope/chapter/1', tag: 'chapter:catpope:1' });
});

test('an empty or odd payload still shows something plain', () => {
	assert.deepEqual(noticeOf(null), { title: 'SCUTOONS', body: '', url: '/', tag: 'scutoons' });
	assert.deepEqual(noticeOf('not json'), { title: 'SCUTOONS', body: '', url: '/', tag: 'scutoons' });
	assert.equal(noticeOf(JSON.stringify({ title: 'x'.repeat(500) })).title.length, 120);
});

test('a tap opens only a path on our origin', () => {
	assert.equal(safePath('/catpope/chapter/2'), '/catpope/chapter/2');
	for (const bad of ['https://evil.test/', '//evil.test/x', '/\\evil.test', 'javascript:alert(1)', 42, null]) assert.equal(safePath(bad), '/', String(bad));
});

test('the badge counts each chapter once until the app is opened', () => {
	let state = nextBadge(null, 'chapter:catpope:1');
	assert.equal(state.count, 1);
	state = nextBadge(state.stored, 'chapter:catpope:1');
	assert.equal(state.count, 1, 'the same chapter sent again');
	state = nextBadge(state.stored, 'chapter:catpope:2');
	assert.equal(state.count, 2);
	assert.equal(nextBadge('{broken', 'x').count, 1);
});
