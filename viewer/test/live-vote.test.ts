// #231: the vote screens' pure bits (src/lib/live-vote.ts, live-vote-words.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildOpen, deviceLocale, groupLook, howKey, percent, pickLabels, pickText, type VoteForm } from '../src/lib/live-vote.ts';
import { WORDS, wordsFor } from '../src/lib/live-vote-words.ts';

const form = (more: Partial<VoteForm> = {}): VoteForm => ({
	id: 'v1',
	question: { en: 'Trade?', de: '' },
	options: { en: 'Yes\nNo', de: '' },
	rule: 'majority',
	n: 20,
	seconds: 30,
	fallback: 0,
	groups: '',
	show: true,
	...more
});

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

test('buildOpen: the form → vote.open', () => {
	const r = buildOpen(form({ options: { en: ' Yes \n\nNo ', de: 'Ja\nNein' }, groups: 'left, right', rule: 'first', n: 3 }));
	assert.ok('open' in r);
	assert.deepEqual(r.open, {
		vote: 'v1',
		options: ['Yes', 'No'],
		rule: 'first',
		n: 3,
		seconds: 30,
		fallback: 0,
		groups: ['left', 'right'],
		show: true,
		question: { en: 'Trade?' },
		labels: { en: ['Yes', 'No'], de: ['Ja', 'Nein'] }
	});
});

test('buildOpen: says what is wrong', () => {
	const why = (f: Partial<VoteForm>) => (buildOpen(form(f)) as { error: string }).error;
	assert.match(why({ options: { en: 'Yes', de: '' } }), /two options/);
	assert.match(why({ options: { en: 'a\nb', de: 'x' } }), /same number/);
	assert.match(why({ seconds: 0 }), /Seconds/);
	assert.match(why({ fallback: 2 }), /fallback/);
	assert.match(why({ rule: 'first', n: 0 }), /n is/);
});

test('a group keeps its look', () => {
	assert.deepEqual(groupLook('balkon'), groupLook('balkon'));
	assert.ok(groupLook('x').colour.startsWith('var(--badge-'));
});
