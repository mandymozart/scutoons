import { test } from 'node:test';
import assert from 'node:assert/strict';
import { showId } from '../src/shows.ts';

test('a show id: the bare id or its key, lowercase letters, digits, - and _', () => {
	assert.equal(showId('demo-far-nodes'), 'demo-far-nodes');
	assert.equal(showId('show:demo-far-nodes'), 'demo-far-nodes');
	assert.equal(showId('Demo'), null);
	assert.equal(showId('show:'), null);
	assert.equal(showId('a/b'), null);
});
