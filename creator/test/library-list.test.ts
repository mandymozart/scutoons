// The Library's list from the server's answer (lib/editor/library-list.ts): the query a place asks, Find, the order and
// runs shown, the levels of a thing stepped into.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { LibraryEntry, LibraryLevel } from '@scutoons/api';
import { filePath, findIn, levelColumns, levelKey, placeAvailable, placeQuery, sortEntries, startPlace, typeRuns, TYPE_CHOICES } from '../src/lib/editor/library-list.ts';

const entry = (address: string, name: string, more: Partial<LibraryEntry> = {}): LibraryEntry => ({
	address,
	name,
	picture: null,
	toons: [],
	usedIn: 0,
	updated: null,
	...(address.startsWith('image:') || address.startsWith('model:') ? { kind: address.split(':')[0] as 'image' } : { type: address.split(':')[0] as 'character' }),
	...more
});

test('a place asks its own query: the item, the cast of a season, the toon, the universe', () => {
	const ctx = { toon: 'catpope', open: 'item:camera', season: 'season:catpope:1' };
	assert.deepEqual(placeQuery('item', ctx, 'all', null, null, 'en'), { locale: 'en', limit: 5000, toon: 'catpope', place: 'item:item:camera' });
	assert.deepEqual(placeQuery('cast', { ...ctx, toon: 'toptoy', season: 'season:catpope:2' }, 'all', null, null, 'de'), { locale: 'de', limit: 5000, toon: 'catpope', place: 'cast:2' });
	assert.deepEqual(placeQuery('all', ctx, 'all', null, null, 'en'), { locale: 'en', limit: 5000, place: 'all' });
	assert.deepEqual(placeQuery('recent', ctx, 'all', 'uploaded', null, 'en'), { locale: 'en', limit: 5000, toon: 'catpope', place: 'recent', from: 'uploaded' });
});

test('the type select asks types or kinds; a pick asks only the kinds its slot takes', () => {
	const ctx = { toon: 'catpope', open: null, season: null };
	assert.deepEqual(placeQuery('toon', ctx, 'instrument', null, null, 'en').type, ['instrument', 'rig']);
	assert.deepEqual(placeQuery('toon', ctx, 'model', null, null, 'en').kind, ['model']);
	const pick = placeQuery('toon', ctx, 'character', null, ['image'], 'en');
	assert.deepEqual([pick.type, pick.kind], [undefined, ['image']]);
	assert.equal(TYPE_CHOICES.filter((c) => c.group === 'things').length, 11);
});

test('a place needs what it narrows to', () => {
	assert.equal(placeAvailable('item', { toon: 'catpope', open: null, season: null }), false);
	assert.equal(placeAvailable('item', { toon: 'catpope', open: 'toon:catpope', season: null }), false);
	assert.equal(placeAvailable('cast', { toon: 'catpope', open: null, season: 'season:catpope:1' }), true);
	assert.equal(startPlace({ toon: 'catpope', open: 'chapter:catpope:3', season: null }), 'item');
	assert.equal(startPlace({ toon: null, open: null, season: null }), 'all');
});

test('Find keeps entries whose name or address hold every word; a file knows its path', () => {
	const list = [entry('character:catpope', 'Cat Pope'), entry('image:characters/boy/boy.webp', 'boy.webp')];
	assert.deepEqual(findIn(list, 'cat pope').map((e) => e.address), ['character:catpope']);
	assert.deepEqual(findIn(list, 'characters boy').map((e) => e.address), ['image:characters/boy/boy.webp']);
	assert.equal(filePath(list[1]), 'characters/boy/boy.webp');
	assert.equal(filePath(list[0]), null);
});

test('sorting keeps the server blocks (things, then files) and sorts inside each', () => {
	const list = [entry('character:b', 'Boy', { usedIn: 1 }), entry('character:a', 'Zed', { usedIn: 9 }), entry('image:x.webp', 'b.webp'), entry('image:y.webp', 'a.webp')];
	assert.deepEqual(sortEntries(list, 'name').map((e) => e.name), ['Boy', 'Zed', 'a.webp', 'b.webp']);
	assert.deepEqual(sortEntries(list, 'uses').map((e) => e.name), ['Zed', 'Boy', 'b.webp', 'a.webp']);
	assert.deepEqual(typeRuns(list).map((r) => [r.key, r.entries.length]), [['character', 2], ['image', 2]]);
});

test('a character stepped into: personas, the chosen persona\'s looks, the chosen look\'s moods', () => {
	const at = 'character:catpope';
	const level = (address: string, kind: LibraryLevel['level'], parent: string, mood?: string): LibraryLevel => ({ address, level: kind, name: mood ?? address, picture: null, parent, ...(mood ? { mood } : {}) });
	const levels = [
		level('character:catpope.regular', 'persona', at),
		level('character:catpope.regular.default', 'look', 'character:catpope.regular'),
		level('character:catpope.regular.default', 'mood', 'character:catpope.regular.default', 'happy'),
		level('character:catpope.regular.default', 'mood', 'character:catpope.regular.default', 'sad'),
		level('character:catpope.human', 'persona', at)
	];
	assert.deepEqual(levelColumns(levels, at, []).map((c) => [c.level, c.rows.length]), [['persona', 2]]);
	const deep = levelColumns(levels, at, ['character:catpope.regular', 'character:catpope.regular.default', 'character:catpope.regular.default#happy']);
	assert.deepEqual(deep.map((c) => [c.level, c.rows.length]), [['persona', 2], ['look', 1], ['mood', 2]]);
	assert.equal(levelKey(deep[2].rows[0]), 'character:catpope.regular.default#happy');
	// a toon: seasons, then the chosen season's chapters
	const toon = [level('season:catpope:1', 'season', 'toon:catpope'), level('chapter:catpope:1', 'chapter', 'season:catpope:1')];
	assert.deepEqual(levelColumns(toon, 'toon:catpope', ['season:catpope:1']).map((c) => c.level), ['season', 'chapter']);
});
