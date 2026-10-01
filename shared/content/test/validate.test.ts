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

test('slice video: variants { low, medium, high }, controls, viewport; sources is gone', () => {
	const chapter = (video: unknown, extra: object = {}) => ({
		title: 'P', number: 0, groups: [{ slices: [{ image: 'images/p.webp', width: 390, height: 206, video, ...extra }] }]
	});
	const videoProblems = (video: unknown, extra?: object) => problems('chapter', chapter(video, extra)).filter((p) => p.includes('.video') || p.includes('.dialog'));
	const variants = { low: 'videos/p.480p.mp4', medium: 'videos/p.720p.mp4', high: 'videos/p.1080p.mp4' };
	assert.deepEqual(videoProblems({ src: 'videos/p.720p.mp4', variants, player: true, controls: false, viewport: { position: 'top', fit: 'cover' } }), []);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', variants: { low: '', ultra: 'videos/p.4k.mp4' } }), [
		'error groups.0.slices.0.video.variants.low',
		'error groups.0.slices.0.video.variants.ultra'
	]);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', variants: ['videos/q.mp4'] }), ['error groups.0.slices.0.video.variants']);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', sources: [{ src: 'videos/q.mp4' }] }), ['error groups.0.slices.0.video.sources']);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', controls: 'yes', viewport: { position: 'left', fit: 'fill' } }), [
		'error groups.0.slices.0.video.controls',
		'error groups.0.slices.0.video.viewport.position',
		'error groups.0.slices.0.video.viewport.fit'
	]);
	assert.deepEqual(videoProblems({ src: 'videos/p.mp4', viewport: {} }, { dialog: [{ rect: { x: 1, y: 1, width: 10 }, text: 'x' }] }), ['warning groups.0.slices.0.dialog']);
	for (const fit of ['contain', 'width', 'height', 'cover']) assert.deepEqual(videoProblems({ src: 'videos/p.mp4', viewport: { fit, position: 'bottom' } }), []);
});

test('slice video frame: a ratio within 4:1 … 1:4, the slice height follows it', () => {
	const chapter = (video: unknown, height: number) => ({
		title: 'P', number: 0, groups: [{ slices: [{ image: 'images/p.webp', width: 390, height, video }] }]
	});
	const framed = (frame: unknown, height = 693, extra: object = {}) =>
		problems('chapter', chapter({ src: 'videos/p.mp4', frame, ...extra }, height)).filter((p) => p.includes('.video') || p.endsWith('.height'));
	assert.deepEqual(framed({ ratio: '9:16', fit: 'cover', position: 'top' }), []);
	assert.deepEqual(framed({ ratio: [1, 4] }, 1560), []);
	assert.deepEqual(framed({ ratio: '6:13' }, 845), []);
	assert.deepEqual(framed({ ratio: '1:5' }, 1950), ['error groups.0.slices.0.video.frame.ratio']);
	assert.deepEqual(framed({ ratio: 'tall' }), ['error groups.0.slices.0.video.frame.ratio']);
	assert.deepEqual(framed({ ratio: '9:16', fit: 'stretch' }), ['error groups.0.slices.0.video.frame.fit']);
	assert.deepEqual(framed({ ratio: '9:16' }, 625), ['warning groups.0.slices.0.height']);
	assert.deepEqual(framed({ ratio: '9:16' }, 693, { viewport: {} }), ['warning groups.0.slices.0.video.frame']);
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

test('toon settings: announce (boolean) and the landing tile { image, color? }', () => {
	const toon = (extra: object) => ({
		id: 't', title: 'T', description: 'D', language: 'de', authors: ['A'], chapters: 0, release: '2026-10-04', status: 'draft',
		thumbnail: 'images/t.webp', cover: { square: 'images/c.webp' }, map: { layers: { terrain: 'images/m.webp' }, width: 1, height: 1, center: { x: 0, y: 0 } },
		...extra
	});
	const tileProblems = (extra: object) => problems('toon', toon(extra)).filter((p) => /announce|tile/.test(p));
	assert.deepEqual(tileProblems({}), []);
	assert.deepEqual(tileProblems({ announce: true, tile: { image: '../characters/y/y_helmet_portrait-neutral.webp' } }), []);
	assert.deepEqual(tileProblems({ announce: false, tile: { image: 'images/tile.webp', color: '#ff0' } }), []);
	assert.deepEqual(tileProblems({ tile: { image: 'images/tile.webp', color: 'darkgreen' } }), []);
	assert.deepEqual(tileProblems({ announce: 'yes' }), ['error announce']);
	assert.deepEqual(tileProblems({ tile: { color: 'blue' } }), ['error tile.image']);
	assert.deepEqual(tileProblems({ tile: {} }), [], 'an emptied tile shows nothing');
	assert.deepEqual(tileProblems({ tile: { image: 'images/t.webp', color: 'url(x)' } }), ['error tile.color']);
	assert.deepEqual(tileProblems({ tile: 'images/t.webp' }), ['error tile']);
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
