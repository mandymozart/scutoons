// A season's cast edited in the toon's season (lib/editor/season-cast.ts) and the character's "cast in".
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { ToonCast } from '@scutoons/content';
import { castIn, moveInCast, setCastPersona, setCastRole, takeOutOfCast } from '../src/lib/editor/season-cast.ts';

const cast = (): ToonCast => ({
	seasons: {
		'1': [
			{ character: 'catpope', personas: ['regular', 'cat'] },
			{ character: 'boy', personas: ['regular'] },
			{ character: 'crows', personas: ['regular'], role: { de: 'Krähen', en: '' } }
		],
		'2': [{ character: 'boy', personas: ['biker'] }]
	}
});
const order = (c: ToonCast, s = 1) => c.seasons[String(s)].map((e) => e.character);

test('the order moves within one season', () => {
	const c = cast();
	assert.equal(moveInCast(c, 1, 2, 0), true);
	assert.deepEqual(order(c), ['crows', 'catpope', 'boy']);
	assert.equal(moveInCast(c, 1, 0, 9), false);
	assert.equal(moveInCast(c, 3, 0, 1), false);
});

test('a character is taken out of one season – never the star', () => {
	const c = cast();
	takeOutOfCast(c, 1, 'catpope', 'catpope');
	assert.deepEqual(order(c), ['catpope', 'boy', 'crows']);
	takeOutOfCast(c, 1, 'boy', 'catpope');
	assert.deepEqual(order(c), ['catpope', 'crows']);
	assert.deepEqual(order(c, 2), ['boy']);
});

test('personas go on and off; the last one stays', () => {
	const c = cast();
	setCastPersona(c, 1, 'catpope', 'mountaintiger', true);
	assert.deepEqual(c.seasons['1'][0].personas, ['regular', 'cat', 'mountaintiger']);
	setCastPersona(c, 1, 'boy', 'regular', false);
	assert.deepEqual(c.seasons['1'][1].personas, ['regular']);
});

test('the role is set per language and goes when it has no text', () => {
	const c = cast();
	setCastRole(c, 1, 'boy', 'en', 'Narrator');
	assert.deepEqual(c.seasons['1'][1].role, { en: 'Narrator' });
	setCastRole(c, 1, 'crows', 'de', ' ');
	assert.equal(c.seasons['1'][2].role, undefined);
});

test('cast in: every season of every toon that lists the character, with its personas there', () => {
	const casts = { toptoy: { seasons: { '1': [{ character: 'boy', personas: [] }] } }, catpope: cast(), sandbox: null };
	assert.deepEqual(castIn('boy', casts), [
		{ toon: 'catpope', season: 1, personas: ['regular'] },
		{ toon: 'catpope', season: 2, personas: ['biker'] },
		{ toon: 'toptoy', season: 1, personas: ['regular'] }
	]);
	assert.deepEqual(castIn('nobody', casts), []);
});
