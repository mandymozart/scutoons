// address-targets.ts (#477 slice B): the typeahead of a VN event's target filters the toon's address book by kind and
// words, and a pick fills the event's fields from the entry's name and the names above it – no address is parsed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { AddressBook, AddressEntry } from '@scutoons/api';
import { chainLabel, childNames, currentEntry, fillValues, indexOf, matchEntries, TARGETS } from '../src/lib/editor/address-targets.ts';

const origin = (key: string, state: 'published' | 'draft' = 'published') => ({ key, state });
const e = (address: string, kind: AddressEntry['kind'], name: string, labels: Record<string, string>, parent?: string, state: 'published' | 'draft' = 'published'): AddressEntry => ({
	address,
	kind,
	name,
	...(parent ? { parent } : {}),
	labels,
	origin: origin(address, state)
});

const BOOK: AddressBook = {
	address_book: 1,
	toon: 'catpope',
	version: 'v1',
	source: 'en',
	entries: [
		e('character:catpope', 'character', 'catpope', { de: 'Katzenpapst', en: 'Cat Pope' }),
		e('character:catpope.human', 'persona', 'human', { de: 'Mensch', en: 'human' }, 'character:catpope'),
		e('character:catpope.human.helmet', 'look', 'helmet', { de: 'Helm' }, 'character:catpope.human'),
		e('character:boy', 'character', 'boy', { de: 'Junge' }, undefined, 'draft'),
		e('location:dom', 'location', 'dom', { de: 'Dom', en: 'Cathedral' }),
		e('item:camera', 'item', 'camera', { de: 'Kamera', en: 'Camera' }),
		e('chapter:2', 'chapter', '2', { de: 'Zwei' }),
		e('chapter:2.intro', 'group', 'intro', {}, 'chapter:2'),
		e('chapter:2.Das-Ende', 'group', 'Das Ende.', {}, 'chapter:2'),
		e('chapter:10', 'chapter', '10', { de: 'Zehn' }),
		e('variable:global.coins', 'variable', 'global.coins', {})
	]
};
const addresses = (list: AddressEntry[]) => list.map((x) => x.address);

test('the filter takes the kinds a target may be, in the book order', () => {
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.jump.kinds, '', 'de')), ['chapter:2', 'chapter:2.intro', 'chapter:2.Das-Ende', 'chapter:10']);
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.location.kinds, '', 'de')), ['location:dom']);
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.cast.kinds, '', 'de')).length, 4);
});

test('every word matches the label in the creator\'s locale, the name or the address', () => {
	// the label in the asked locale: "Kamera" in de, "Camera" in en
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.item.kinds, 'kamera', 'de')), ['item:camera']);
	assert.deepEqual(addresses(matchEntries(BOOK, ['location'], 'cathedral', 'en')), ['location:dom']);
	assert.deepEqual(addresses(matchEntries(BOOK, ['location'], 'cathedral', 'de')), [], 'not the other language\'s label');
	// the name and the address
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.cast.kinds, 'helmet', 'de')), ['character:catpope.human.helmet']);
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.cast.kinds, 'catpope HUMAN', 'de')), ['character:catpope.human', 'character:catpope.human.helmet']);
	assert.deepEqual(addresses(matchEntries(BOOK, TARGETS.jump.kinds, 'ende', 'de')), ['chapter:2.Das-Ende']);
	// accept narrows (taking: only takeable items)
	assert.deepEqual(matchEntries(BOOK, TARGETS.item.kinds, '', 'de', (x) => x.name !== 'camera'), []);
	assert.equal(matchEntries(BOOK, TARGETS.cast.kinds, '', 'de', undefined, 2).length, 2, 'a limit');
});

test('a pick fills the fields from the name and the parents; the levels it lacks are removed', () => {
	const index = indexOf(BOOK);
	const by = (a: string) => index.get(a)!;
	assert.deepEqual(fillValues(by('character:catpope.human.helmet'), index, TARGETS.cast.fields), { character: 'catpope', persona: 'human', look: 'helmet' });
	assert.deepEqual(fillValues(by('character:catpope.human'), index, TARGETS.cast.fields), { character: 'catpope', persona: 'human', look: undefined });
	assert.deepEqual(fillValues(by('character:boy'), index, TARGETS.cast.fields), { character: 'boy', persona: undefined, look: undefined });
	assert.deepEqual(fillValues(by('chapter:2.Das-Ende'), index, TARGETS.jump.fields), { value: '2', label: 'Das Ende.' }, 'the label exactly, not the address word');
	assert.deepEqual(fillValues(by('chapter:10'), index, TARGETS.jump.fields), { value: '10', label: undefined });
	assert.deepEqual(fillValues(by('variable:global.coins'), index, TARGETS.variable.fields), { variable: 'global.coins' });
	assert.deepEqual(fillValues(by('location:dom'), index, TARGETS.location.fields), { location: 'dom' });
});

test('the current entry is the one whose pick would write the fields as they are', () => {
	assert.equal(currentEntry(BOOK, TARGETS.jump.kinds, TARGETS.jump.fields, { type: 'chapter', value: '2', label: 'intro' })?.address, 'chapter:2.intro');
	assert.equal(currentEntry(BOOK, TARGETS.jump.kinds, TARGETS.jump.fields, { type: 'chapter', value: '2' })?.address, 'chapter:2');
	assert.equal(currentEntry(BOOK, TARGETS.cast.kinds, TARGETS.cast.fields, { character: 'catpope', persona: 'human', mood: 'happy' })?.address, 'character:catpope.human');
	assert.equal(currentEntry(BOOK, TARGETS.jump.kinds, TARGETS.jump.fields, { value: '7' }), null, 'not in the book');
	assert.equal(currentEntry(BOOK, TARGETS.jump.kinds, TARGETS.jump.fields, {}), null, 'empty');
});

test('labels for the list: the chain in the creator\'s locale, a name where there is no label', () => {
	const index = indexOf(BOOK);
	assert.equal(chainLabel(index.get('character:catpope.human.helmet')!, index, 'de', 'en'), 'Katzenpapst › Mensch › Helm');
	assert.equal(chainLabel(index.get('character:catpope.human.helmet')!, index, 'en', 'en'), 'Cat Pope › human › Helm', 'the one locale order (generated)');
	assert.equal(chainLabel(index.get('chapter:2.intro')!, index, 'de', 'en'), 'Zwei › intro');
});

test('a chapter\'s groups by the chapter\'s name', () => {
	assert.deepEqual(childNames(BOOK, 'chapter', '2', 'group'), ['intro', 'Das Ende.']);
	assert.deepEqual(childNames(BOOK, 'chapter', '10', 'group'), []);
	assert.deepEqual(childNames(BOOK, 'chapter', '99', 'group'), []);
});
