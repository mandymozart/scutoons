// The creators' docs (lib/docs): the list of use cases holds together and the tutorials keep the format.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { allCases, findCase, nextCase, written } from '../src/lib/docs/catalog.ts';
import { parseCase, parseGroups } from '../src/lib/docs/markdown.ts';

// the same files the pages read (lib/docs/tutorials.ts globs them at build time)
const dir = fileURLToPath(new URL('../docs/tutorials/', import.meta.url));
const files = Object.fromEntries(
	readdirSync(dir, { withFileTypes: true })
		.filter((d) => d.isDirectory())
		.flatMap((d) => readdirSync(join(dir, d.name)).map((f) => [`${d.name}/${f}`, readFileSync(join(dir, d.name, f), 'utf8')]))
);
const GROUPS = parseGroups(files);

test('every use case has a unique slug, a title, a summary, who and where', () => {
	const slugs = new Set<string>();
	for (const c of allCases(GROUPS)) {
		assert.match(c.slug, /^[a-z0-9-]+$/, c.slug);
		assert.ok(!slugs.has(c.slug), `twice: ${c.slug}`);
		slugs.add(c.slug);
		assert.ok(c.title && c.summary, c.slug);
		assert.ok(!c.summary.endsWith('.'), `the page adds the full stop: ${c.slug}`);
		assert.ok(c.who.length > 0 && c.where.length > 0, c.slug);
	}
	assert.equal(new Set(GROUPS.map((g) => g.key)).size, GROUPS.length);
});

test('a written tutorial has three to six steps of at most two sentences', () => {
	for (const c of allCases(GROUPS).filter(written)) {
		const n = c.steps?.length ?? 0;
		assert.ok(n >= 3 && n <= 6, `${c.slug}: ${n} steps`);
		for (const s of c.steps ?? []) assert.ok((s.text.match(/[.!?](\s|$)/g) ?? []).length <= 2, `${c.slug}: ${s.text}`);
	}
});

test('the next guide skips the ones not written yet, and the last has none', () => {
	const list = allCases(GROUPS).filter(written);
	assert.ok(list.length > 0);
	for (let i = 0; i < list.length - 1; i++) assert.equal(nextCase(GROUPS, list[i].slug)?.slug, list[i + 1].slug);
	assert.equal(nextCase(GROUPS, list[list.length - 1].slug), undefined);
	assert.equal(findCase(GROUPS, 'no-such-case'), undefined);
});

test('a use case reads from its Markdown: front matter, facts, steps, note; a template has no steps', () => {
	const c = parseCase(`---
slug: x
title: X
summary: S
who: writer, lead
where: creator
often: 2
partly: true
---

## Facts

- Size: big: very

## Steps

1. One.
2. Two.

## Note

Mind it.
`);
	assert.deepEqual(c, {
		slug: 'x',
		title: 'X',
		summary: 'S',
		who: ['writer', 'lead'],
		where: ['creator'],
		often: 2,
		partly: true,
		facts: [{ label: 'Size', text: 'big: very' }],
		steps: [{ text: 'One.' }, { text: 'Two.' }],
		note: 'Mind it.'
	});
	const t = parseCase(`---
slug: y
title: Y
summary: S
who: all
where: deck
often: 1
---

## Steps

<!-- to write -->
`);
	assert.equal(t.steps, undefined);
	assert.equal(written(t), false);
});

test('the phases and their order come from the folders: 11 phases, 64 use cases, 36 written', () => {
	assert.equal(GROUPS.length, 11);
	assert.equal(GROUPS[0].key, 'start');
	assert.equal(GROUPS[0].cases[0].slug, 'sign-in');
	assert.equal(allCases(GROUPS).length, 64);
	assert.equal(allCases(GROUPS).filter(written).length, 36);
});
