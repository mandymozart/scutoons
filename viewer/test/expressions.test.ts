// The reader's answers for expressions in texts (lib/expressions/reader.ts, agents/EXPRESSIONS.md §2):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EXPR_NAMESPACES, chapterCheckpointUri, evaluateText, type ExprResolver } from '@scutoons/content';
import { readerCall, readerName, type ReaderSources } from '../src/lib/expressions/reader.ts';

const now = new Date(2026, 9, 6, 23, 15); // a Tuesday, 23:15 local
const sources: ReaderSources = {
	toon: 'catpope',
	variable: (name) => ({ coins: 1, t: 330, who: 'Pope' })[name as 'coins'],
	signedIn: () => true,
	member: () => true,
	accountCreated: () => '2026-09-26 10:00:00',
	language: () => 'de',
	startedAt: () => '2026-10-01T08:00:00.000Z',
	checkpoints: () => [
		{ uri: chapterCheckpointUri('catpope', 1), isComplete: true, position: 1, created_at: '', updated_at: '' },
		{ uri: chapterCheckpointUri('catpope', 2), isComplete: false, position: 0.5, created_at: '', updated_at: '' },
		{ uri: chapterCheckpointUri('other', 3), isComplete: true, created_at: '', updated_at: '' }
	],
	chapters: () => [0, 1, 2],
	entries: () => [
		{ item_id: 'coin', qty: 3, source: 'story', edition_number: null },
		{ item_id: 'cucumber', qty: 1, source: 'gift', edition_number: null },
		{ item_id: 'camera', qty: 1, source: 'qr', edition_number: 17 }
	],
	wears: (item) => item === 'camera',
	body: () => ({ character: 'catpope', look: 'swim' }),
	unlockedCharacters: () => 2,
	stat: (id) => ({ testo: 62, appetite: 40 })[id as 'testo'],
	effects: () => ['dizzy'],
	map: () => ({ places: { harbour: { unlocked: 1, seen: 2 }, forest: { unlocked: 3 } }, routes: {}, positions: { catpope: { location: 'harbour', at: 1 } } }),
	places: () => [{ id: 'harbour', name: 'Hafen' }, { id: 'forest', name: 'Wald' }, { id: 'airport', name: 'Flughafen' }],
	now: () => now
};
const resolver: ExprResolver = { name: (p) => readerName(sources, p), call: (n, a) => readerCall(sources, n, a), language: 'de' };
const say = (text: string) => evaluateText(text, resolver).text;

test('every value of the registry namespaces has an answer (reader.name: askText, step 3)', () => {
	for (const ns of EXPR_NAMESPACES)
		for (const m of ns.members)
			if (m.kind === 'value' && `${ns.name}.${m.name}` !== 'reader.name')
				assert.notEqual(readerName(sources, [ns.name, m.name]), undefined, `${ns.name}.${m.name}`);
	assert.equal(readerName(sources, ['reader', 'name']), undefined);
});

test('names and state functions', () => {
	assert.equal(say('{coins} {coins == 1: coin|coins}'), '1 coin');
	assert.equal(say('{clock(t)} – {who}'), '5:30 – Pope');
	assert.equal(say('{map.total - map.unlocked} left, {map.seen} seen, here: {map.here}'), '1 left, 1 seen, here: Hafen');
	assert.equal(say('{reader.signedIn} {reader.member} {reader.memberDays} {reader.since} {reader.gifts} {reader.language} {reader.daysSinceStart}'), 'true true 10 2026-09-26 1 de 5');
	assert.equal(say('{read.chapters} {read.finished} {read.chapter(1)} {read_chapter(2)} {read.position(2)} {read.position(9)}'), '1 false true false 0.5 0');
	assert.equal(say('{has("coin")} {has("onion")} {count_of("coin")} {inventory.count("coin")} {inventory.items} {inventory.edition("camera")}'), 'true false 3 3 3 17');
	assert.equal(say('{avatar.character}/{avatar.look} {avatar.wears("camera")} {avatar.unlocked}'), 'catpope/swim true 2');
	assert.equal(say('{stats.testo} {stats.appetite} {effects.count} {effects.active("dizzy")} {map.place("forest")} {map.place("airport")}'), '62 40 1 true true false');
	assert.equal(say('{now.hour} {now.weekday} {now.night} {now.date}'), '23 2 true 2026-10-06');
	assert.equal(readerCall(sources, 'resolved', ['x']), undefined, 'events: step 3');
});

test('a guest: no account values, nothing exposed', () => {
	const guest: ReaderSources = { toon: 'catpope', signedIn: () => false, member: () => false, accountCreated: () => '2026-01-01' };
	assert.equal(readerName(guest, ['reader', 'memberDays']), 0);
	assert.equal(readerName(guest, ['reader', 'since']), undefined);
	assert.equal(readerName(guest, ['reader', 'email']), undefined);
	assert.equal(readerName(guest, ['map', 'total']), undefined, 'the map not loaded yet');
	assert.equal(readerName(guest, ['coins']), undefined);
	assert.equal(readerName(guest, ['a', 'b', 'c']), undefined);
});
