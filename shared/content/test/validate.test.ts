// Season downloads (season.download, moved from the toon settings 2026-09-30).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseEntityKey, sceneEntityOfRef, validateFile } from '../src/index.ts';

const season = (extra: object) => ({ title: 'S', description: 'D', language: 'de', number: 1, sequence: [], ...extra });
const problems = (kind: string, data: object) => validateFile(kind, data, 'de', { file: 'x' } as never).map((p) => `${p.level} ${p.path.join('.')}`);

test('season.download: optional, music is a path, complete a path or null', () => {
	assert.deepEqual(problems('season', season({})), []);
	assert.deepEqual(problems('season', season({ download: { music: 'audio/a.zip' } })), []);
	assert.deepEqual(problems('season', season({ download: { music: 'audio/a.zip', complete: null } })), []);
	assert.deepEqual(problems('season', season({ download: { music: 3 } })), ['error download.music']);
	assert.deepEqual(problems('season', season({ download: {} })), [], 'an emptied download field shows nothing');
	assert.deepEqual(problems('season', season({ download: 'x' })), ['error download']);
});

test('season.supporters: ordered { name, logo?, url? }', () => {
	assert.deepEqual(problems('season', season({ supporters: [] })), []);
	assert.deepEqual(problems('season', season({ supporters: [{ name: 'Stadt Wien', logo: 'images/supporters/wien.svg', url: 'https://wien.gv.at' }, { name: 'Friends' }] })), []);
	assert.deepEqual(problems('season', season({ supporters: [{ name: '' }, { name: 'X', url: 'wien.gv.at' }, { name: 'Y', logo: 3 }] })), ['error supporters.0.name', 'error supporters.1.url', 'error supporters.2.logo']);
	assert.deepEqual(problems('season', season({ supporters: {} })), ['error supporters']);
});

test('entity keys: chapter 0 (a prologue) is a chapter, season 0 is none', () => {
	assert.deepEqual(parseEntityKey('chapter:catpope:0:de'), { kind: 'chapter', toon: 'catpope', number: 0, locale: 'de' });
	assert.equal(sceneEntityOfRef('catpope', 'chapter0/intro.json'), 'scene:catpope:0:intro');
	for (const key of ['season:catpope:0:de', 'chapter:catpope::de', 'chapter:catpope:01:de', 'chapter:catpope:-1:de', 'chapter:catpope:1.5:de']) {
		assert.equal(parseEntityKey(key), null, key);
	}
	assert.equal(parseEntityKey('season:catpope:1:de')?.number, 1);
});

test('slice video: player is a boolean', () => {
	const chapter = (video: unknown) => ({
		title: 'P', number: 0, groups: [{ slices: [{ image: 'images/p.webp', width: 390, height: 206, video }] }]
	});
	const videoProblems = (video: unknown) => problems('chapter', chapter(video)).filter((p) => p.includes('.video'));
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', player: true }), []);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', player: 'yes' }), ['error groups.0.slices.0.video.player']);
});

test('slice video: sources [{ src, media? }] before src', () => {
	const chapter = (video: unknown) => ({
		title: 'P', number: 0, groups: [{ slices: [{ image: 'images/p.webp', width: 390, height: 206, video }] }]
	});
	const videoProblems = (video: unknown) => problems('chapter', chapter(video)).filter((p) => p.includes('.video'));
	assert.deepEqual(videoProblems({ src: 'videos/p.1080p.mp4', player: true, sources: [{ src: 'videos/p.480p.mp4', media: '(max-width: 600px)' }, { src: 'videos/p.720p.mp4' }] }), []);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', sources: [{ src: '' }, { src: 'videos/p.mp4', media: 600 }] }), [
		'error groups.0.slices.0.video.sources.0.src',
		'error groups.0.slices.0.video.sources.1.media'
	]);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', sources: 'videos/q.mp4' }), ['error groups.0.slices.0.video.sources']);
});

test('season.trailer: { loop, poster, sources?, chapter? }', () => {
	const sequence = [{ type: 'chapter', number: 0, status: 'published' }, { type: 'chapter', number: 1, status: 'published' }];
	const trailer = { loop: 'videos/s1-trailer-loop.1280w.mp4', sources: [{ src: 'videos/s1-trailer-loop.1920w.mp4', media: '(min-width: 1600px)' }], poster: 'images/s1-trailer-poster.webp' };
	assert.deepEqual(problems('season', season({ sequence, trailer })), []);
	assert.deepEqual(problems('season', season({ sequence, trailer: { ...trailer, chapter: 1 } })), []);
	assert.deepEqual(problems('season', season({ sequence, trailer: { poster: 'images/p.webp', chapter: -1 } })), ['error trailer.loop', 'error trailer.chapter']);
	assert.deepEqual(problems('season', season({ sequence, trailer: { loop: 'videos/l.mp4' } })), ['error trailer.poster']);
	assert.deepEqual(problems('season', season({ sequence: [], trailer })), ['warning trailer.chapter']);
	assert.deepEqual(problems('season', season({ sequence, trailer: 'videos/l.mp4' })), ['error trailer']);
});

test('season.download.image: an image path on the CDN (decorative, beside the music button)', () => {
	const sequence = [{ type: 'chapter', number: 1, status: 'published' }];
	const download = { music: 'audio/m.zip', image: 'images/s1-music-vinyl.webp' };
	assert.deepEqual(problems('season', season({ sequence, download })), []);
	assert.deepEqual(problems('season', season({ sequence, download: { ...download, image: 'audio/m.zip' } })), ['warning download.image']);
	assert.deepEqual(problems('season', season({ sequence, download: { ...download, image: 3 } })), ['error download.image']);
});

test('toon entry: announce (boolean) and the landing tile { image, color? }', () => {
	const entry = (extra: object) => ({
		id: 't', uri: 't', status: 'draft', avatar: 'avatar.webp', i18n: {}, authors: ['A'], chapters: 0, thumbnail: 'images/t.webp',
		map: { center: { x: 0, y: 0 }, zoom: 1.5 },
		...extra
	});
	const tileProblems = (extra: object) => problems('index', entry(extra)).filter((p) => /announce|tile/.test(p));
	assert.deepEqual(problems('index', entry({})), []);
	assert.deepEqual(problems('index', { id: 't', uri: 't', status: 'draft', avatar: 'a.webp', i18n: {} }), [], 'every neutral setting is optional');
	assert.deepEqual(tileProblems({ announce: true, tile: { image: '../characters/y/y_helmet_portrait-neutral.webp' } }), []);
	assert.deepEqual(tileProblems({ announce: false, tile: { image: 'images/tile.webp', color: '#ff0' } }), []);
	assert.deepEqual(tileProblems({ tile: { image: 'images/tile.webp', color: 'darkgreen' } }), []);
	assert.deepEqual(tileProblems({ announce: 'yes' }), ['error announce']);
	assert.deepEqual(tileProblems({ tile: { color: 'blue' } }), ['error tile.image']);
	assert.deepEqual(tileProblems({ tile: {} }), [], 'an emptied tile shows nothing');
	assert.deepEqual(tileProblems({ tile: { image: 'images/t.webp', color: 'url(x)' } }), ['error tile.color']);
	assert.deepEqual(tileProblems({ tile: 'images/t.webp' }), ['error tile']);
	assert.deepEqual(problems('index', entry({ map: { layers: { terrain: '' }, width: 0, height: 0, center: { x: 0 } } })), [
		'warning map.layers',
		'warning map.width',
		'warning map.height',
		'error map.center.y'
	]);
	assert.deepEqual(problems('index', entry({ chapters: 1.5, authors: 'A' })), ['error authors', 'error chapters']);
});

test('toon settings (per language): the neutral settings are warned about, they belong to the toon entry', () => {
	const settings = { id: 't', title: 'T', description: 'D', language: 'de', release: '2026-10-04', status: 'draft' };
	assert.deepEqual(problems('toon', settings), []);
	const neutral = {
		authors: ['A'], chapters: 5, thumbnail: 'images/t.webp', announce: true, tile: { image: 'images/tile.webp' }, cover: { square: 'images/c.webp' },
		map: { layers: { terrain: 'images/m.svg' }, width: 2048, height: 1046, center: { x: 1320, y: 440 } }
	};
	assert.deepEqual(problems('toon', { ...settings, ...neutral }), [
		'warning authors',
		'warning chapters',
		'warning thumbnail',
		'warning map',
		'warning announce',
		'warning tile',
		'warning cover'
	]);
});

test('cover (toon entry, season): square / portrait images + lettering (boolean)', () => {
	const entry = (cover: unknown) => problems('index', { id: 't', uri: 't', status: 'draft', avatar: 'a.webp', i18n: {}, cover });
	assert.deepEqual(entry({ square: 'images/s1-cover-square.webp', portrait: 'images/s1-cover-portrait.webp' }), []);
	assert.deepEqual(entry({ square: 'images/s1-cover-square-textless.webp', lettering: true }), []);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: 'yes' }), ['error cover.lettering']);
	assert.deepEqual(entry({ lettering: true }), ['warning cover.lettering'], 'lettering without an image');
	assert.deepEqual(entry('images/c.webp'), ['error cover']);
	assert.deepEqual(problems('season', season({ cover: { square: 'images/c.webp', lettering: false } })), []);
	assert.deepEqual(problems('season', season({ cover: { square: 'images/c.webp', lettering: 1 } })), ['error cover.lettering']);
});

test('media paths with a locale segment are warnings in every entity (media are language-independent)', async () => {
	const { mediaLocaleSegment, validateLocales } = await import('../src/index.ts');
	for (const [path, locale] of [
		['images/1x/de/s1-cover-square.jpg', 'de'],
		['de/s1.webp', 'de'],
		['images/webtoon/en/S1C1 - Slice 1.jpg', 'en'],
		['images/s1-cover_de.webp', 'de'],
		['audio/intro-en.mp3', 'en'],
		['videos/trailer.en-US.mp4', 'en-us'],
		['images/EN/x.png', 'en']
	] as const) assert.equal(mediaLocaleSegment(path), locale, path);
	for (const path of ['images/s1-cover-square.webp', 'images/webtoon/s1c1/s1c1-slice-1.webp', 'images/dense.webp', 'audio/de.txt', 'de/notes.json', 'images/legend.svg', 'videos/s1-trailer-loop.1280w.mp4'])
		assert.equal(mediaLocaleSegment(path), null, path);
	const chapter = { title: 'C', groups: [{ slices: [{ image: 'images/webtoon/de/S1C1 - Slice 1.jpg' }] }] };
	assert.ok(problems('chapter', chapter).includes('warning groups.0.slices.0.image'));
	assert.deepEqual(problems('index', { id: 't', uri: 't', status: 'draft', avatar: 'a.webp', i18n: {}, cover: { square: 'images/1x/en/s1-cover-square.jpg' } }), ['warning cover.square']);
	assert.deepEqual(problems('season', season({ download: { image: 'images/vinyl-de.webp' } })), ['warning download.image']);
	// a season's cover is the same in every language
	const de = season({ cover: { square: 'images/a.webp' } });
	const en = { ...season({ cover: { square: 'images/b.webp' } }), language: 'en', title: 'S en', description: 'D en' };
	assert.deepEqual(validateLocales('season', 'x', { de, en }, 'de').map((p) => `${p.level} ${p.locale} ${p.path.join('.')}`), ['warning en cover']);
});

test('isToonListed + toonTileColor: published or announced; the tile colour from the settings, else the star', async () => {
	const { isToonListed, toonTileColor } = await import('../src/index.ts');
	assert.equal(isToonListed({ status: 'published' as never }), true);
	assert.equal(isToonListed({ status: 'draft' as never }), false);
	assert.equal(isToonListed({ status: 'draft' as never, announced: true }), true);
	const registry = { characters: [{ id: 'yellowmouse', own_toon: 'yellowmouse', color: 'yellow' }, { id: 'boy', color: 'red' }] } as never;
	assert.equal(toonTileColor(registry, 'yellowmouse'), 'yellow');
	assert.equal(toonTileColor(registry, 'yellowmouse', { color: '#000' }), '#000');
	assert.equal(toonTileColor(registry, 'boy'), null);
	assert.equal(toonTileColor(null, 'x'), null);
});
