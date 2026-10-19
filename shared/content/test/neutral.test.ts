// Language-neutral documents (agents/NEUTRAL.md): resolution against the golden fixtures that the server's
// NeutralDocTest reads too (test/fixtures/neutral/*.json – both implementations must give these answers), the keys.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { entityKeyFor, isNeutralDocKey, isNeutralKey, neutralLocales, parseEntityKey, resolveNeutral, resolveText, servedLocale } from '../src/index.ts';

const dir = join(import.meta.dirname, 'fixtures/neutral');
const fixtures = readdirSync(dir).filter((f) => f.endsWith('.json'));

test('golden fixtures: served locale, fallback and the resolved document', () => {
	assert.ok(fixtures.length >= 6);
	for (const file of fixtures) {
		const f = JSON.parse(readFileSync(join(dir, file), 'utf8'));
		for (const c of f.cases) {
			const got = resolveNeutral(f.kind, f.doc, c.locale, f.languages, f.status ?? undefined);
			assert.deepEqual(JSON.parse(JSON.stringify(got)), c.expected, `${file} ${c.locale}`);
		}
	}
});

test('resolution never changes the stored document', () => {
	const f = JSON.parse(readFileSync(join(dir, 'chapter.json'), 'utf8'));
	const before = JSON.stringify(f.doc);
	resolveNeutral('chapter', f.doc, 'en', f.languages, 'published' as never);
	assert.equal(JSON.stringify(f.doc), before);
});

test('texts: served locale → source → other languages → any; blank is missing; plain strings pass', () => {
	const chain = ['en', 'de'];
	assert.equal(resolveText({ de: 'a', en: 'b' }, chain), 'b');
	assert.equal(resolveText({ de: 'a', en: ' ' }, chain), 'a');
	assert.equal(resolveText({ ko: 'k' }, chain), 'k');
	assert.equal(resolveText({}, chain), undefined);
	assert.deepEqual(resolveText({ en: ['', 'x'] }, chain), ['', 'x']);
	assert.equal(resolveText({ de: 'a', en: [''] }, chain), 'a', 'a list of blank lines is missing');
	assert.equal(resolveText('legacy', chain), 'legacy');
	assert.equal(resolveText({ de: 'a', en: '  ﻿' }, chain), 'a', 'blank like JS trim() (the server matches it)');
	assert.deepEqual(neutralLocales('chapter', { title: { de: 'x', en: '', ko: 'y' } }), ['de', 'ko']);
	assert.deepEqual(servedLocale('season', { title: { en: 'x' } }, 'de', { source: 'de' }), { locale: 'en', fallback: true });
});

test('keys: chapter / season are stored neutral or (legacy) per locale; scenes are gone', () => {
	assert.equal(isNeutralDocKey('chapter:catpope:2'), true);
	assert.equal(isNeutralDocKey('season:catpope:1'), true);
	assert.equal(isNeutralDocKey('chapter:catpope:2:de'), false);
	assert.equal(isNeutralDocKey('toon:catpope'), false, 'not a chapter / season');
	assert.equal(isNeutralDocKey('toon-settings:catpope'), false);
	assert.equal(isNeutralKey('toon:catpope'), true);
	assert.equal(isNeutralKey('chapter:catpope:2'), true);
	assert.equal(isNeutralKey('season:catpope:1:en'), false);
	assert.equal(isNeutralKey('nonsense'), false);
	// the legacy write path is unchanged: a locale gives the per-locale key, the neutral locale the entity
	assert.equal(entityKeyFor('chapter:catpope:2', 'de'), 'chapter:catpope:2:de');
	assert.equal(entityKeyFor('chapter:catpope:2', '_'), 'chapter:catpope:2');
	// one document per chapter since 2026-10-19: a scene document's key is no key any more
	assert.equal(parseEntityKey('scene:catpope:2:scene-14'), null);
});
