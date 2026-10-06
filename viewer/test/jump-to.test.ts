// buildJumpToUrl: a chapter of this toon or of another one (owner 2026-10-06, "at label"), random labels, other kinds.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildJumpToUrl } from '../src/lib/visual-novel/jump-to.ts';

test('a chapter of this toon / of another toon at a label', () => {
	assert.equal(buildJumpToUrl({ type: 'chapter', value: '2', label: 'mama-crossroads' } as any, 'catpope'), '/catpope/chapter/2?label=mama-crossroads');
	assert.equal(buildJumpToUrl({ type: 'chapter', value: '1', toon: 'toptoy', label: 'mama-hypercity' } as any, 'catpope'), '/toptoy/chapter/1?label=mama-hypercity');
	assert.equal(buildJumpToUrl({ type: 'chapter', value: '3', toon: 'catpope' } as any, 'catpope'), '/catpope/chapter/3', 'its own toon named');
	assert.equal(buildJumpToUrl({ type: 'toon', value: 'yellowmouse' } as any, 'catpope'), '/yellowmouse', "a toon's page");
	assert.equal(buildJumpToUrl({ type: 'product', value: 'vinyl' } as any, 'catpope'), '/shop/product/vinyl');
});

test('anyOf: never the current group – except in another toon (a group of the same name there is another group)', () => {
	const here = { type: 'chapter', value: '2', anyOf: ['a', 'b'] } as any;
	for (let i = 0; i < 20; i++) assert.equal(buildJumpToUrl(here, 'catpope', 'a'), '/catpope/chapter/2?label=b');
	const there = { type: 'chapter', value: '1', toon: 'toptoy', anyOf: ['a', 'b'] } as any;
	const seen = new Set<string | null>();
	for (let i = 0; i < 200; i++) seen.add(buildJumpToUrl(there, 'catpope', 'a'));
	assert.deepEqual([...seen].sort(), ['/toptoy/chapter/1?label=a', '/toptoy/chapter/1?label=b']);
});
