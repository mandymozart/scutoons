// What the shell keeps in this browser (lib/shell/kept.ts) and the /kit page's Shell sample, which must keep nothing.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { test } from 'node:test';
import { holdWrites, readJson, writeJson, writesHeld } from '../src/lib/shell/kept.ts';

const store = new Map<string, string>();
Object.assign(globalThis, {
	localStorage: {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v),
		removeItem: (k: string) => void store.delete(k)
	}
});
const src = (path: string) => readFileSync(new URL(`../src/${path}`, import.meta.url), 'utf8');

test('a write lands in this browser and reads back; a broken value reads as null', () => {
	store.clear();
	writeJson('sc-creator-tabs', { front: 'character:catpope' });
	assert.deepEqual(readJson('sc-creator-tabs'), { front: 'character:catpope' });
	store.set('broken', '{');
	assert.equal(readJson('broken'), null);
});

test('while held nothing is kept – the real tabs, workspaces and chosen layouts stay as they were', () => {
	store.clear();
	writeJson('sc-creator-tabs', { front: 'character:catpope' });
	const release = holdWrites();
	assert.ok(writesHeld());
	writeJson('sc-creator-tabs', { front: 'character:giantpopstar' });
	writeJson('sc-creator-workspaces', { character: 3 });
	writeJson('sc-creator-chosen', { character: 'own-1' });
	assert.deepEqual(readJson('sc-creator-tabs'), { front: 'character:catpope' });
	assert.equal(store.has('sc-creator-workspaces'), false);
	assert.equal(store.has('sc-creator-chosen'), false);
	release();
	release();
	assert.equal(writesHeld(), false, 'a release counts once');
	writeJson('sc-creator-tabs', { front: 'toon:catpope' });
	assert.deepEqual(readJson('sc-creator-tabs'), { front: 'toon:catpope' });
});

test('holds nest: the writes come back only when the last one ends', () => {
	const a = holdWrites();
	const b = holdWrites();
	a();
	assert.ok(writesHeld());
	b();
	assert.equal(writesHeld(), false);
});

test('every write of the shell goes through kept.ts, so a sample can hold it', () => {
	for (const file of readdirSync(new URL('../src/lib/shell/', import.meta.url))) {
		if (file === 'kept.ts' || !file.endsWith('.ts')) continue;
		assert.doesNotMatch(src(`lib/shell/${file}`), /localStorage\.setItem/, file);
	}
});

test('the /kit Shell sample starts the shell’s sample before it opens its tabs and ends it when it goes', () => {
	const kit = src('lib/components/page/kit-shell.svelte');
	const start = kit.indexOf('shell.sample()');
	assert.ok(start > 0, 'kit-shell.svelte runs inside shell.sample()');
	assert.ok(start < kit.indexOf('shell.visit('), 'the sample starts before the first visit');
	assert.match(kit, /end\(\);\s*\};/, 'the cleanup ends the sample');
	const shell = src('lib/shell/shell.svelte.ts');
	assert.match(shell, /sample\(\): \(\) => void \{[\s\S]*holdWrites\(\)[\s\S]*this\.state = was\.state;[\s\S]*layouts\.chosen = was\.chosen;/);
});
