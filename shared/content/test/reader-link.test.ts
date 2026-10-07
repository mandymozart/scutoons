// The creator's dock Reader messages (BACKLOG 10.49): only a well-formed "show" is a place, anything else is ignored.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isReaderReady, parseReaderShow, READER_READY, readerShowMessage } from '../src/index.ts';

test('a well-formed show message is a place', () => {
	const place = { toon: 'toptoy', chapter: 1, group: 2, slice: -1 };
	assert.deepEqual(parseReaderShow(readerShowMessage(place)), place);
	assert.deepEqual(parseReaderShow({ type: 'scutoons:reader-show', toon: 'cat_pope-2', chapter: 0, group: 0, slice: 3 }), { toon: 'cat_pope-2', chapter: 0, group: 0, slice: 3 });
});

test('anything else is ignored', () => {
	const ok = { type: 'scutoons:reader-show', toon: 'toptoy', chapter: 1, group: 0, slice: 0 };
	for (const bad of [
		null,
		'scutoons:reader-show',
		{ ...ok, type: 'show' },
		{ ...ok, toon: '../admin' },
		{ ...ok, toon: 'a/b' },
		{ ...ok, toon: '' },
		{ ...ok, chapter: '1' },
		{ ...ok, chapter: -1 },
		{ ...ok, group: 1.5 },
		{ ...ok, group: -1 },
		{ ...ok, slice: -2 },
		{ ...ok, slice: 1e9 }
	])
		assert.equal(parseReaderShow(bad), null, JSON.stringify(bad));
});

test('ready', () => {
	assert.equal(isReaderReady({ type: READER_READY }), true);
	assert.equal(isReaderReady({ type: 'scutoons:reader-show' }), false);
	assert.equal(isReaderReady(null), false);
});
