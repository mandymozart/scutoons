import { test } from 'node:test';
import assert from 'node:assert/strict';
import { showId, showRows } from '../src/shows.ts';

test('a show id: the bare id or its key, lowercase letters, digits, - and _', () => {
	assert.equal(showId('demo-far-nodes'), 'demo-far-nodes');
	assert.equal(showId('show:demo-far-nodes'), 'demo-far-nodes');
	assert.equal(showId('Demo'), null);
	assert.equal(showId('show:'), null);
	assert.equal(showId('a/b'), null);
});

test('list_shows: a show with several toons is listed under each of them, its toons in short (#708)', () => {
	const all = [
		{ key: 'show:mixed', latest: 3, published: null, content: { id: 'mixed', toons: ['catpope', 'yellowmouse'], outlets: [{ id: 'a' }], rows: [{}, {}] } },
		{ key: 'show:demo', latest: 7, published: 6, content: { id: 'demo', toons: ['sandbox'], outlets: [], rows: [] } }
	];
	assert.deepEqual(showRows(all, 'yellowmouse').map((s) => s.id), ['mixed']);
	assert.deepEqual(showRows(all, 'sandbox').map((s) => s.id), ['demo']);
	assert.deepEqual(showRows(all).map((s) => s.toons), [['catpope', 'yellowmouse'], ['sandbox']]);
	assert.deepEqual(showRows(all, 'catpope')[0], { id: 'mixed', toons: ['catpope', 'yellowmouse'], outlets: ['a'], rows: 2, draft: 3, published: null });
});
