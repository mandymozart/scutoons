// The creators' docs (lib/docs): the list of use cases holds together and the tutorials keep the format.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { allCases, findCase, nextCase, written } from '../src/lib/docs/catalog.ts';
import { GROUPS } from '../src/lib/docs/tutorials.ts';

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
