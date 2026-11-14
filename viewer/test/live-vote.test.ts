// #231: the audience vote screen's pure bits (src/lib/live-vote.ts, live-vote-words.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deviceLocale, groupLook, howKey, percent, pickLabels, pickText } from '../src/lib/live-vote.ts';
import { WORDS, wordsFor } from '../src/lib/live-vote-words.ts';

test('the device language: two letters, English when nothing is said', () => {
	assert.equal(deviceLocale(['de-AT', 'en']), 'de');
	assert.equal(deviceLocale(['EN-gb']), 'en');
	assert.equal(deviceLocale(undefined), 'en');
});

test('a question by locale: the language, else English, else the first', () => {
	assert.equal(pickText({ en: 'Yes?', de: 'Ja?' }, 'de'), 'Ja?');
	assert.equal(pickText({ en: 'Yes?', de: 'Ja?' }, 'fr'), 'Yes?');
	assert.equal(pickText({ de: 'Ja?' }, 'fr'), 'Ja?');
	assert.equal(pickText(undefined, 'de', '–'), '–');
});

test('labels by locale; a wrong length falls back to options', () => {
	const open = { options: ['Yes', 'No'], labels: { de: ['Ja', 'Nein'], fr: ['Oui'] } };
	assert.deepEqual(pickLabels(open, 'de'), ['Ja', 'Nein']);
	assert.deepEqual(pickLabels(open, 'fr'), ['Yes', 'No']);
	assert.deepEqual(pickLabels({ options: ['A', 'B'] }, 'de'), ['A', 'B']);
});

test('how it was decided: the rule by its name, else fallback / override', () => {
	assert.equal(howKey({ rule: 'groups' }, { how: 'rule' }), 'groups');
	assert.equal(howKey({ rule: 'groups' }, { how: 'override' }), 'override');
});

test('percent of the counts, whole numbers, 0 with no votes', () => {
	assert.equal(percent(1, [1, 2]), 33);
	assert.equal(percent(2, [1, 2]), 67);
	assert.equal(percent(0, [0, 0]), 0);
});

test('the phone speaks de and en, anything else English; every how has words', () => {
	assert.equal(wordsFor('de'), WORDS.de);
	assert.equal(wordsFor('fr'), WORDS.en);
	for (const w of Object.values(WORDS)) assert.deepEqual(Object.keys(w.how).sort(), ['fallback', 'first', 'groups', 'majority', 'override']);
});

test('a group keeps its look', () => {
	assert.deepEqual(groupLook('balkon'), groupLook('balkon'));
	assert.ok(groupLook('x').colour.startsWith('var(--badge-'));
});
