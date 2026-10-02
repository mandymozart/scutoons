// Players, albums, related items (agents/ITEMS.md "Players, albums, related items"): against the real server catalog
// (server/config/items.json) and the catpope seed.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { albumOf, mediaPlayback, playersFor, validateCatalog, validateFile, validateLocales, type Catalog, type ItemDef } from '../src/index.ts';

const json = (rel: string) => JSON.parse(readFileSync(new URL(rel, import.meta.url), 'utf8'));
const catalog: Catalog = json('../../../server/config/items.json');
const item = (id: string) => catalog.items.find((i) => i.id === id)!;
const itemIds = catalog.items.map((i) => i.id);

test('albums: the catalog has the players, the album and media items that point at it', () => {
	assert.deepEqual(validateCatalog(catalog).filter((p) => p.level === 'error'), []);
	assert.deepEqual(item('record_player').plays, ['vinyl']);
	assert.deepEqual(item('tape_player').plays, ['tape']);
	assert.deepEqual(item('cd_player').plays, ['cd']);
	for (const id of ['record_player', 'tape_player', 'cd_player']) {
		assert.equal(item(id).category, 'tool');
		assert.equal(item(id).stackable, false);
		assert.deepEqual(item(id).grantable, []);
	}
	for (const id of ['awakening_vinyl', 'awakening_test_press']) {
		assert.equal(item(id).format, 'vinyl');
		assert.equal(albumOf(catalog, item(id))?.id, 'awakening');
		assert.equal(item(id).skills.some((s) => s.type === 'play_audio'), false, `${id}: play_audio is gone`);
	}
	const album = albumOf(catalog, item('awakening_vinyl'))!;
	assert.ok(album.tracks.length >= 1);
	assert.match(album.download ?? '', /\.zip$/);
	assert.deepEqual(playersFor(catalog, 'vinyl').map((p) => p.id), ['record_player']);
});

test('albums: a record plays only with a record player, otherwise the download is offered', () => {
	const vinyl = item('awakening_vinyl');
	const without = mediaPlayback(catalog, vinyl, ['awakening_vinyl', 'camera']);
	assert.equal(without?.kind, 'needs_player');
	if (without?.kind !== 'needs_player') return;
	assert.equal(without.format, 'vinyl');
	assert.equal(without.players[0].id, 'record_player');
	assert.equal(without.download, 'catpope/audio/CatPope-Awakening-Music.zip');
	// a tape or CD player does not play vinyl
	assert.equal(mediaPlayback(catalog, vinyl, ['awakening_vinyl', 'tape_player', 'cd_player'])?.kind, 'needs_player');
	const withPlayer = mediaPlayback(catalog, vinyl, ['awakening_vinyl', 'record_player']);
	assert.equal(withPlayer?.kind, 'play');
	if (withPlayer?.kind === 'play') {
		assert.equal(withPlayer.player?.id, 'record_player');
		assert.equal(withPlayer.tracks[0].src, 'catpope/audio/music/s1c1-instrumental-brass-morton.128.mp3');
	}
	// not media
	assert.equal(mediaPlayback(catalog, item('camera'), ['record_player']), null);
	assert.equal(mediaPlayback(catalog, item('record_player'), ['record_player']), null);
});

test('albums: the old play_audio skill is still read (plays without a player)', () => {
	const old: ItemDef = { ...item('awakening_vinyl'), album: null, format: null, skills: [{ type: 'play_audio', src: 'catpope/audio/x.mp3' }] };
	const playback = mediaPlayback(catalog, old, []);
	assert.equal(playback?.kind, 'play');
	if (playback?.kind === 'play') assert.equal(playback.tracks[0].src, 'catpope/audio/x.mp3');
});

test('albums: validation errors', () => {
	const bad = structuredClone(catalog);
	bad.items.find((i) => i.id === 'awakening_vinyl')!.album = 'nope';
	bad.items.find((i) => i.id === 'awakening_test_press')!.format = null;
	bad.items.find((i) => i.id === 'cd_player')!.plays = ['laserdisc' as never];
	bad.albums!.push({ id: 'awakening', toon: null, title: { de: 'x' }, tracks: [{ id: 'a', title: 'A', src: 'a.txt' }, { id: 'a', title: 'B', src: 'b.mp3' }], download: 'x.rar' });
	const messages = validateCatalog(bad).filter((p) => p.level === 'error').map((p) => p.message);
	for (const part of ['unknown album "nope"', 'needs both album and format', 'plays must be a list', 'album id "awakening" is used twice', 'track id "a" is used twice', 'src must be an audio path', 'download must be a .zip path']) {
		assert.ok(messages.some((m) => m.includes(part)), `expected an error: ${part}\n${messages.join('\n')}`);
	}
	// a format nobody plays
	const lonely = structuredClone(catalog);
	lonely.items = lonely.items.filter((i) => i.id !== 'record_player');
	assert.ok(validateCatalog(lonely).some((p) => p.level === 'warning' && p.message.includes('no player plays "vinyl"')));
});

test('related items: the seed season lists catalog items, the same in every language', () => {
	const de = json('../seed/catpope/de/season1.json');
	const en = json('../seed/catpope/en/season1.json');
	assert.deepEqual(de.related_items, ['awakening_vinyl', 'awakening_test_press']);
	for (const [l, doc] of [['de', de], ['en', en]] as const) {
		const problems = validateFile('season', doc, l, { file: 'season:catpope:1', items: itemIds }).filter((p) => p.path[0] === 'related_items');
		assert.deepEqual(problems, []);
	}
	assert.deepEqual(validateLocales('season', 'season:catpope:1', { de, en }, 'de').filter((p) => p.path[0] === 'related_items'), []);
	// differing lists, unknown ids, duplicates, wrong type
	const other = { ...en, related_items: ['awakening_vinyl'] };
	assert.ok(validateLocales('season', 'season:catpope:1', { de, en: other }, 'de').some((p) => p.level === 'warning' && p.path[0] === 'related_items'));
	const wrong = validateFile('season', { ...de, related_items: ['awakening_vinyl', 'nope', 'awakening_vinyl'] }, 'de', { file: 's', items: itemIds });
	assert.ok(wrong.some((p) => p.message.includes('"nope" is not in the item catalog')));
	assert.ok(wrong.some((p) => p.message.includes('listed twice')));
	assert.ok(validateFile('season', { ...de, related_items: 'awakening_vinyl' }, 'de', { file: 's' }).some((p) => p.level === 'error' && p.path[0] === 'related_items'));
	// chapters take the same field
	const chapter = json('../seed/catpope/de/chapter1.json');
	const ch = validateFile('chapter', { ...chapter, related_items: ['record_player', 'nope'] }, 'de', { file: 'c', items: itemIds }).filter((p) => p.path[0] === 'related_items');
	assert.equal(ch.length, 1);
	assert.match(ch[0].message, /"nope"/);
});
