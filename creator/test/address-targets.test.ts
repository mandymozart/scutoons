// address-targets.ts (#477 slice B): the typeahead of a VN event's target filters the toon's address book by kind and
// words, and a pick fills the event's fields from the entry's name and the names above it – no reference is parsed; an event's target is a reference, the creator never needs an entry's address.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { AddressBook, AddressEntry } from '@scutoons/api';
import { chainLabel, childNames, currentEntry, fillValues, indexOf, inScope, matchEntries, opensScope, pickValues, scopeOf, TARGETS } from '../src/lib/editor/address-targets.ts';

const origin = (key: string, state: 'published' | 'draft' = 'published') => ({ key, state });
const e = (ref: string, kind: AddressEntry['kind'], name: string, labels: Record<string, string>, parent?: string, state: 'published' | 'draft' = 'published'): AddressEntry => ({
	ref,
	...(['character', 'persona', 'look', 'item'].includes(kind) ? { address: ref } : {}),
	kind,
	name,
	...(parent ? { parent } : {}),
	labels,
	origin: origin(ref, state)
});

const BOOK: AddressBook = {
	address_book: 2,
	toon: 'catpope',
	version: 'v1',
	source: 'en',
	entries: [
		e('character:catpope', 'character', 'catpope', { de: 'Katzenpapst', en: 'Cat Pope' }),
		e('character:catpope.human', 'persona', 'human', { de: 'Mensch', en: 'human' }, 'character:catpope'),
		e('character:catpope.human.helmet', 'look', 'helmet', { de: 'Helm' }, 'character:catpope.human'),
		e('character:boy', 'character', 'boy', { de: 'Junge' }, undefined, 'draft'),
		e('location:dom', 'location', 'dom', { de: 'Dom', en: 'Cathedral' }),
		e('region:hills', 'region', 'hills', { de: 'Hügel' }),
		e('item:camera', 'item', 'camera', { de: 'Kamera', en: 'Camera' }),
		e('season:1', 'season', '1', { de: 'Staffel Eins' }),
		e('chapter:2', 'chapter', '2', { de: 'Zwei' }),
		e('chapter:2.intro', 'group', 'intro', {}, 'chapter:2'),
		e('chapter:2.Das-Ende', 'group', 'Das Ende.', {}, 'chapter:2'),
		e('chapter:10', 'chapter', '10', { de: 'Zehn' }),
		e('product:hoodie', 'product', 'hoodie', { en: 'Dago Hoodie' }),
		e('toon:catpope', 'toon', 'catpope', { de: 'Katzenpapst' }),
		e('toon:toptoy', 'toon', 'toptoy', { de: 'Top Toy' }),
		e('variable:global.coins', 'variable', 'global.coins', {})
	]
};
/** another toon's book (#477 slice D: a jump into it narrows the field to it) */
const TOPTOY: AddressBook = {
	address_book: 2,
	toon: 'toptoy',
	version: 'v1',
	source: 'de',
	entries: [
		e('season:1', 'season', '1', { de: 'Erste' }),
		e('chapter:1', 'chapter', '1', { de: 'Eins' }),
		e('chapter:1.start', 'group', 'start', {}, 'chapter:1'),
		e('toon:toptoy', 'toon', 'toptoy', { de: 'Top Toy' }),
		e('toon:catpope', 'toon', 'catpope', { de: 'Katzenpapst' })
	]
};
const refs = (list: AddressEntry[]) => list.map((x) => x.ref);

test('the filter takes the kinds a target may be, in the book order', () => {
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.jump.kinds, '', 'de')), ['season:1', 'chapter:2', 'chapter:2.intro', 'chapter:2.Das-Ende', 'chapter:10', 'product:hoodie', 'toon:catpope', 'toon:toptoy']);
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.location.kinds, '', 'de')), ['location:dom']);
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.cast.kinds, '', 'de')).length, 4);
});

test('every word matches the label in the creator\'s locale, the name or the reference', () => {
	// the label in the asked locale: "Kamera" in de, "Camera" in en
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.item.kinds, 'kamera', 'de')), ['item:camera']);
	assert.deepEqual(refs(matchEntries(BOOK, ['location'], 'cathedral', 'en')), ['location:dom']);
	assert.deepEqual(refs(matchEntries(BOOK, ['location'], 'cathedral', 'de')), [], 'not the other language\'s label');
	// the name and the reference
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.cast.kinds, 'helmet', 'de')), ['character:catpope.human.helmet']);
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.cast.kinds, 'catpope HUMAN', 'de')), ['character:catpope.human', 'character:catpope.human.helmet']);
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.jump.kinds, 'ende', 'de')), ['chapter:2.Das-Ende']);
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
	assert.deepEqual(fillValues(by('chapter:2.Das-Ende'), index, TARGETS.jump.fields), { value: '2', label: 'Das Ende.' }, 'the label exactly, not the reference word');
	assert.deepEqual(fillValues(by('chapter:10'), index, TARGETS.jump.fields), { value: '10', label: undefined });
	assert.deepEqual(fillValues(by('variable:global.coins'), index, TARGETS.variable.fields), { variable: 'global.coins' });
	assert.deepEqual(fillValues(by('location:dom'), index, TARGETS.location.fields), { location: 'dom' });
});

test('the current entry is the one whose pick would write the fields as they are', () => {
	assert.equal(currentEntry(BOOK, TARGETS.jump, { type: 'chapter', value: '2', label: 'intro' })?.ref, 'chapter:2.intro');
	assert.equal(currentEntry(BOOK, TARGETS.jump, { type: 'chapter', value: 2 })?.ref, 'chapter:2', 'a number as stored');
	assert.equal(currentEntry(BOOK, TARGETS.cast, { character: 'catpope', persona: 'human', mood: 'happy' })?.ref, 'character:catpope.human');
	assert.equal(currentEntry(BOOK, TARGETS.jump, { type: 'chapter', value: '7' }), null, 'not in the book');
	assert.equal(currentEntry(BOOK, TARGETS.jump, {}), null, 'empty');
});

// ── a jump's target is ONE field (#477 slice D): the entry's kind is the target type, another toon is a scope ──────────

const jump = TARGETS.jump;
const at = (book: AddressBook, ref: string) => indexOf(book).get(ref)!;

test('another toon narrows the field: its entry opens the scope, its book offers its chapters, groups and its page', () => {
	assert.equal(opensScope(at(BOOK, 'toon:toptoy'), BOOK, jump), true, 'another toon: into its book');
	assert.equal(opensScope(at(BOOK, 'toon:catpope'), BOOK, jump), false, 'this toon: its page, picked');
	assert.equal(opensScope(at(BOOK, 'chapter:2'), BOOK, jump), false);
	assert.equal(opensScope(at(BOOK, 'toon:toptoy'), BOOK, TARGETS.cast), false, 'only a field with a scope');
	// in there: chapters, groups and the toon itself (its page) – no season of another toon, no scope in a scope
	const inside = refs(matchEntries(TOPTOY, jump.scope.kinds, '', 'de', (x) => inScope(x, TOPTOY, jump)));
	assert.deepEqual(inside, ['chapter:1', 'chapter:1.start', 'toon:toptoy']);
	assert.deepEqual(refs(matchEntries(TOPTOY, jump.scope.kinds, 'start', 'de', (x) => inScope(x, TOPTOY, jump))), ['chapter:1.start'], 'typing goes on inside');
	// the scope a stored target is in: its `toon`, when it is another toon
	assert.equal(scopeOf({ type: 'chapter', value: '1', toon: 'toptoy' }, jump, 'catpope'), 'toptoy');
	assert.equal(scopeOf({ type: 'chapter', value: '1', toon: 'catpope' }, jump, 'catpope'), null, 'this toon is no scope');
	assert.equal(scopeOf({ type: 'chapter', value: '1' }, jump, 'catpope'), null);
	assert.equal(scopeOf({ toon: 'toptoy' }, TARGETS.cast, 'catpope'), null, 'a field without a scope');
});

test('filling a jump from an entry of each target type (the event keeps its fields)', () => {
	const pick = (book: AddressBook, ref: string, scope: string | null = null) => pickValues(at(book, ref), book, jump, scope);
	const none = { toon: undefined };
	assert.deepEqual(pick(BOOK, 'chapter:10'), { type: 'chapter', value: '10', label: undefined, ...none });
	assert.deepEqual(pick(BOOK, 'chapter:2.Das-Ende'), { type: 'chapter', value: '2', label: 'Das Ende.', ...none }, 'a group: a chapter target with its label');
	assert.deepEqual(pick(BOOK, 'season:1'), { type: 'season', value: '1', label: undefined, ...none });
	assert.deepEqual(pick(BOOK, 'product:hoodie'), { type: 'product', value: 'hoodie', label: undefined, ...none });
	assert.deepEqual(pick(BOOK, 'toon:catpope'), { type: 'toon', value: 'catpope', label: undefined, ...none }, 'this toon\'s page');
	// in another toon's scope: its chapter and group keep the toon; its page is a toon target without it
	assert.deepEqual(pick(TOPTOY, 'chapter:1', 'toptoy'), { type: 'chapter', value: '1', label: undefined, toon: 'toptoy' });
	assert.deepEqual(pick(TOPTOY, 'chapter:1.start', 'toptoy'), { type: 'chapter', value: '1', label: 'start', toon: 'toptoy' });
	assert.deepEqual(pick(TOPTOY, 'toon:toptoy', 'toptoy'), { type: 'toon', value: 'toptoy', label: undefined, ...none });
});

test('an existing jump of each type shows its target as the field\'s value', () => {
	const shown = (target: Record<string, unknown>, book = BOOK, scope: string | null = null) => {
		const entry = currentEntry(book, jump, target, scope);
		return entry ? chainLabel(entry, indexOf(book), 'de', book.source) : null;
	};
	assert.equal(shown({ type: 'chapter', value: 2 }), 'Zwei');
	assert.equal(shown({ type: 'chapter', value: '2', label: 'Das Ende.' }), 'Zwei › Das Ende.');
	assert.equal(shown({ type: 'chapter', value: '2', label: 'intro', toon: 'catpope' }), 'Zwei › intro', '"into" this toon is this toon');
	assert.equal(shown({ type: 'chapter', value: '2', anyOf: ['intro', 'Das Ende.'] }), 'Zwei', 'the random list is its own control');
	assert.equal(shown({ type: 'season', value: '1' }), 'Staffel Eins');
	assert.equal(shown({ type: 'product', value: 'hoodie' }), 'Dago Hoodie');
	assert.equal(shown({ type: 'toon', value: 'catpope' }), 'Katzenpapst');
	assert.equal(shown({ type: 'toon', value: 'toptoy' }), 'Top Toy', 'another toon\'s page, from this book');
	// another toon's chapter and group: from its book, in its scope (the field shows the chip)
	assert.equal(shown({ type: 'chapter', value: '1', toon: 'toptoy' }, TOPTOY, 'toptoy'), 'Eins');
	assert.equal(shown({ type: 'chapter', value: '1', label: 'start', toon: 'toptoy' }, TOPTOY, 'toptoy'), 'Eins › start');
	// the type decides: a season 2 is not chapter 2; XR has no entries (shown as stored, dashed)
	assert.equal(shown({ type: 'season', value: '2' }), null);
	assert.equal(shown({ type: 'xr', value: 'garden' }), null);
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

test('migrate: a region of the world map, filled and shown like any other target', () => {
	assert.deepEqual(refs(matchEntries(BOOK, TARGETS.region.kinds, 'hüg', 'de')), ['region:hills']);
	assert.deepEqual(pickValues(at(BOOK, 'region:hills'), BOOK, TARGETS.region), { region: 'hills' });
	assert.equal(currentEntry(BOOK, TARGETS.region, { region: 'hills', who: 'catpope' })?.ref, 'region:hills');
	assert.equal(currentEntry(BOOK, TARGETS.region, { region: 'gone' }), null, 'a region no longer on the map: shown as stored');
});
