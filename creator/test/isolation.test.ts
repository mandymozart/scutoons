import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mustLeave, needsIsolation, shouldReload } from '../src/lib/isolation.ts';

test('the engine routes are isolated, the rest of the creator is not', () => {
	for (const p of ['/edit', '/edit/', '/edit/rig:character.catpope', '/edit/module:walk']) assert.ok(needsIsolation(p), p);
	for (const p of ['/', '/toons', '/toptoy/chapter/1', '/catalog/items', '/map', '/engine', '/engines', '/editor', '/characters']) assert.ok(!needsIsolation(p), p);
});

test('a page that arrived without its headers loads once afresh – never twice', () => {
	assert.ok(shouldReload(false, false));
	assert.ok(!shouldReload(false, true), 'a host that sends no headers must not loop');
	assert.ok(!shouldReload(true, false));
});

test('an isolated document leaves for a page that must not be isolated by a full load', () => {
	assert.ok(mustLeave(true, '/toptoy/chapter/1'));
	assert.ok(!mustLeave(true, '/edit/rig:character.catpope'));
	assert.ok(!mustLeave(false, '/toptoy/chapter/1'));
});
