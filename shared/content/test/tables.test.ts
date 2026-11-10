// The generated content tables (#477, server/schema/tables): the one locale order runs the cases every language runs
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { LEGACY_CHARACTER_IDS, LOCALE_ORDER, pickContentText, resolveCharacterId } from '../src/index.ts';

type Case = { text: Record<string, string>; asked: string; source: string | null; expect: string | null };
const vectors = JSON.parse(readFileSync(new URL('../../../server/schema/tables/locale-order.vectors.json', import.meta.url), 'utf8')) as { cases: Case[] };

test('the shared locale order cases', () => {
	assert.ok(vectors.cases.length > 5);
	for (const c of vectors.cases) assert.equal(pickContentText(c.text, c.asked, c.source) ?? null, c.expect, JSON.stringify(c));
	assert.deepEqual([...LOCALE_ORDER], ['asked', 'source', 'en', 'any']);
	assert.equal(pickContentText(undefined, 'de'), undefined);
	assert.equal(pickContentText({ de: 'Katze', x: 3 }, 'x'), 'Katze', 'only strings count');
});

test('legacy character ids keep the persona their art went to', () => {
	assert.deepEqual(LEGACY_CHARACTER_IDS.plf, { character: 'catpope', persona: 'human' });
	assert.deepEqual(resolveCharacterId('mountain-tiger'), { character: 'catpope', persona: 'mountaintiger' });
});
