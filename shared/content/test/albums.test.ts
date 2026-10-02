// Players, albums, related items (agents/ITEMS.md "Players, albums, related items"): against the real server catalog
// (server/config/items.json) and the catpope seed.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	albumAccess,
	albumByDownload,
	albumDownload,
	albumOf,
	mediaPlayback,
	playerAlbums,
	playersFor,
	seasonMusic,
	validateCatalog,
	validateFile,
	validateLocales,
	type Catalog,
	type ItemDef
} from '../src/index.ts';

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
	}
	// the record player comes from a recipe (the broken one + a repair coupon, both story finds); tape and CD player:
	// not obtainable yet
	assert.deepEqual(item('record_player').grantable, []);
	assert.deepEqual(item('tape_player').grantable, []);
	assert.deepEqual(item('cd_player').grantable, []);
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

const guest = (...owned: string[]) => ({ owned, member: false });
const member = (...owned: string[]) => ({ owned, member: true });

test('albums: a record plays only with a record player, otherwise the download is offered', () => {
	const vinyl = item('awakening_vinyl');
	const without = mediaPlayback(catalog, vinyl, member('awakening_vinyl', 'camera'));
	assert.equal(without?.kind, 'needs_player');
	if (without?.kind !== 'needs_player') return;
	assert.equal(without.format, 'vinyl');
	assert.equal(without.players[0].id, 'record_player');
	assert.equal(without.download, 'catpope/s1/CatPope-Awakening-Music.zip');
	// a tape or CD player does not play vinyl
	assert.equal(mediaPlayback(catalog, vinyl, member('awakening_vinyl', 'tape_player', 'cd_player'))?.kind, 'needs_player');
	const withPlayer = mediaPlayback(catalog, vinyl, member('awakening_vinyl', 'record_player'));
	assert.equal(withPlayer?.kind, 'play');
	if (withPlayer?.kind === 'play') {
		assert.equal(withPlayer.player?.id, 'record_player');
		assert.equal(withPlayer.tracks[0].src, 'catpope/s1/c1/s1c1-instrumental-brass-morton.128.mp3');
		assert.equal(withPlayer.download, 'catpope/s1/CatPope-Awakening-Music.zip');
	}
	// not media
	assert.equal(mediaPlayback(catalog, item('camera'), member('record_player')), null);
	assert.equal(mediaPlayback(catalog, item('record_player'), member('record_player')), null);
});

test('albums: access – download for members (as before), listening for owners with a player', () => {
	const album = albumOf(catalog, item('awakening_vinyl'))!;
	assert.deepEqual(albumAccess(album), { download: 'members', listen: 'owners' });
	assert.equal(albumByDownload(catalog, 'catpope/s1/CatPope-Awakening-Music.zip')?.id, 'awakening');
	// members (reading access) and owners get the download, a visitor without access does not
	assert.equal(albumDownload(catalog, album, member()), album.download);
	assert.equal(albumDownload(catalog, album, guest('awakening_test_press')), album.download, 'an owner of any carrier');
	assert.equal(albumDownload(catalog, album, guest()), null);
	// the record player's albums: owning only the player → the vinyl to buy, no listening
	const player = item('record_player');
	const [entry] = playerAlbums(catalog, player, member('record_player'));
	assert.equal(entry.album.id, 'awakening');
	assert.equal(entry.carrier.id, 'awakening_vinyl');
	assert.equal(entry.owned, false);
	assert.equal(entry.listen, false);
	// with the test press: that carrier, listening allowed
	const [owned] = playerAlbums(catalog, player, guest('record_player', 'awakening_test_press'));
	assert.equal(owned.carrier.id, 'awakening_test_press');
	assert.equal(owned.owned, true);
	assert.equal(owned.listen, true);
	// other flags
	const open = structuredClone(catalog);
	open.albums![0].access = { download: 'everyone', listen: 'members' };
	assert.equal(albumDownload(open, open.albums![0], guest()), album.download);
	assert.equal(playerAlbums(open, player, member('record_player'))[0].listen, true);
	assert.equal(playerAlbums(open, player, guest('record_player'))[0].listen, false);
	const owners = structuredClone(catalog);
	owners.albums![0].access = { download: 'owners' };
	assert.equal(albumDownload(owners, owners.albums![0], member()), null);
	assert.equal(albumDownload(owners, owners.albums![0], guest('awakening_vinyl')), album.download);
	// a player owner without the right to listen and without a download gets nothing to play
	assert.equal(mediaPlayback(owners, item('awakening_vinyl'), member('awakening_vinyl', 'record_player'))?.kind, 'play');
	const bad = structuredClone(catalog);
	bad.albums![0].access = { download: 'friends' as never, stream: 'owners' } as never;
	const messages = validateCatalog(bad).filter((p) => p.level === 'error').map((p) => p.message);
	assert.ok(messages.some((m) => m.includes('access.download must be one of')));
	assert.ok(messages.some((m) => m.includes('unknown access "stream"')));
});

test('albums: the old play_audio skill is still read (plays without a player)', () => {
	const old: ItemDef = { ...item('awakening_vinyl'), album: null, format: null, skills: [{ type: 'play_audio', src: 'catpope/audio/x.mp3' }] };
	const playback = mediaPlayback(catalog, old, guest());
	assert.equal(playback?.kind, 'play');
	if (playback?.kind === 'play') assert.equal(playback.tracks[0].src, 'catpope/audio/x.mp3');
});

test('albums: the record player is repaired - broken record player + repair coupon, both consumed', () => {
	const repairs = catalog.recipes.filter((r) => r.output.item === 'record_player');
	assert.equal(repairs.length, 1);
	assert.deepEqual(repairs[0].inputs.map((i) => `${i.item}×${i.qty}`).sort(), ['broken_record_player×1', 'repair_coupon×1']);
	assert.equal(repairs[0].output.qty, 1);
	for (const id of ['broken_record_player', 'repair_coupon']) {
		assert.deepEqual(item(id).grantable, ['story'], id);
		assert.equal(item(id).stackable, false, id);
		assert.equal(item(id).kind, 'virtual', id);
		assert.ok(item(id).name.de && item(id).name.en && item(id).description.de && item(id).description.en, id);
		assert.match(item(id).icon, new RegExp(`^scutoons/items/${id}\.png$`));
	}
	// the broken one plays nothing
	assert.equal(item('broken_record_player').plays, undefined);
	assert.deepEqual(playersFor(catalog, 'vinyl').map((p) => p.id), ['record_player']);
});

const pickup = (events: any[], variable: string) => events.find((e: any) => e.params?.onSelect?.add?.variable === variable);

test('albums: chapter 2 - the broken record player in the first scene, the repair coupon at the end, once', () => {
	for (const l of ['de', 'en']) {
		const chapter = json(`../seed/catpope/${l}/chapter2.json`);
		// the first scene: the first group (a scene file) and its first slice
		assert.equal(chapter.groups[0].ref?.src, 'chapter2/scene-14.json');
		const scene = json(`../seed/catpope/${l}/chapter2/scene-14.json`);
		const first = scene.slices[0].events ?? [];
		const broken = pickup(first, 'broken_record_player');
		assert.ok(broken, `${l}: the first slice adds broken_record_player`);
		assert.equal(broken.params.onSelect.add.value, 1);
		assert.equal(first.filter((e: any) => e.type === 'addChoice').length, 1, `${l}: one choice only (intersect order)`);
		assert.ok(first.every((e: any) => e.trigger?.type === 'intersect' && e.trigger.once === true));
		assert.deepEqual(validateFile('group', scene, l, { file: 'scene:catpope:2:scene-14' }).filter((p) => p.level === 'error'), []);

		const last = chapter.groups.at(-1).slices.at(-1);
		const events = last.events ?? [];
		const coupon = pickup(events, 'repair_coupon');
		assert.ok(coupon, `${l}: the last slice adds repair_coupon`);
		assert.equal(coupon.params.onSelect.add.value, 1);
		assert.equal(pickup(events, 'record_player'), undefined, `${l}: the record player is no longer handed out`);
		assert.equal(events.filter((e: any) => e.type === 'addChoice').length, 1, `${l}: one choice only (intersect order)`);
		assert.ok(events.every((e: any) => e.trigger?.type === 'intersect' && e.trigger.once === true));
		assert.deepEqual(validateFile('chapter', chapter, l, { file: 'chapter:catpope:2' }).filter((p) => p.level === 'error'), []);
	}
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
	assert.deepEqual(de.related_items, ['awakening_vinyl']);
	for (const [l, doc] of [['de', de], ['en', en]] as const) {
		const problems = validateFile('season', doc, l, { file: 'season:catpope:1', items: itemIds }).filter((p) => p.path[0] === 'related_items');
		assert.deepEqual(problems, []);
	}
	assert.deepEqual(validateLocales('season', 'season:catpope:1', { de, en }, 'de').filter((p) => p.path[0] === 'related_items'), []);
	// differing lists, unknown ids, duplicates, wrong type
	const other = { ...en, related_items: ['awakening_vinyl', 'awakening_test_press'] };
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

test('season music comes from the related items’ album; old seasons keep their download fields', () => {
	const season = json('../seed/catpope/de/season1.json');
	const music = seasonMusic(catalog, season, 'catpope');
	assert.equal(music?.album?.id, 'awakening');
	assert.equal(music?.carrier, 'awakening_vinyl');
	assert.equal(music?.download, 'catpope/s1/CatPope-Awakening-Music.zip', 'the album’s zip = the season’s old download.music');
	assert.equal(music?.image, 'catpope/s1/s1-music-vinyl.webp', 'the album cover = the old download.image');
	// without the season's own fields: the same answer (the season editor no longer writes them)
	assert.deepEqual(seasonMusic(catalog, { related_items: season.related_items }, 'catpope'), music);
	// an old season without related items: its download fields, the album found by the zip
	const old = seasonMusic(catalog, { download: { music: 's1/CatPope-Awakening-Music.zip', image: 'images/x.webp' } }, 'catpope');
	assert.equal(old?.album?.id, 'awakening');
	assert.equal(old?.carrier, undefined);
	assert.equal(old?.image, 'catpope/s1/s1-music-vinyl.webp');
	assert.deepEqual(seasonMusic(catalog, { download: { music: 'audio/other.zip', image: 'images/x.webp' } }, 'catpope'), { album: undefined, download: 'catpope/audio/other.zip', image: 'catpope/images/x.webp' });
	assert.equal(seasonMusic(catalog, { related_items: ['camera'] }, 'catpope'), null, 'no album, no music');
	assert.equal(seasonMusic(catalog, null, 'catpope'), null);
});
