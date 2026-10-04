// Season downloads (season.download, moved from the toon settings 2026-09-30).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseEntityKey, sceneEntityOfRef, unifyAudio, unifyGeometry, validateFile } from '../src/index.ts';

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

test('season.trailer: { loop, poster, variants?, chapter? } (sources: older data)', () => {
	const sequence = [{ type: 'chapter', number: 0, status: 'published' }, { type: 'chapter', number: 1, status: 'published' }];
	const trailer = { loop: 'videos/s1-trailer-loop.1280w.mp4', variants: { high: 'videos/s1-trailer-loop.1920w.mp4' }, poster: 'images/s1-trailer-poster.webp' };
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

test('cover (season; owner 2026-10-02: not the toon): square / portrait images + lettering (boolean or a layout, cover.test.ts)', () => {
	const entry = (cover: unknown) => problems('season', season({ cover }));
	assert.deepEqual(entry({ square: 'images/s1-cover-square.webp', portrait: 'images/s1-cover-portrait.webp' }), []);
	assert.deepEqual(entry({ square: 'images/s1-cover-square-textless.webp', lettering: true }), []);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: 'yes' }), ['error cover.lettering']);
	assert.deepEqual(entry({ lettering: true }), ['warning cover.lettering'], 'lettering without an image');
	assert.deepEqual(entry('images/c.webp'), ['error cover']);
	// a cover left in the toon entry is a warning: it belongs to the season, the toon is shown with its avatar
	assert.deepEqual(problems('index', { id: 't', uri: 't', status: 'draft', avatar: 'a.webp', i18n: {}, cover: { square: 'images/c.webp' } }), ['warning cover']);
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
	assert.deepEqual(problems('season', season({ cover: { square: 'images/1x/en/s1-cover-square.jpg' } })), ['warning cover.square']);
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

test('slice items: placed catalog items with an action chosen by the creator; text boxes / onomatopoeia scale', () => {
	const chapter = (slice: object) => ({ title: 'P', number: 2, groups: [{ slices: [{ image: 's1/c2/a.webp', width: 390, height: 625, ...slice }] }] });
	const sliceProblems = (slice: object) => problems('chapter', chapter(slice)).filter((p) => p.includes('slices.0.'));
	const rect = { x: 40, y: 50, width: 25 };
	assert.deepEqual(sliceProblems({ items: [{ id: 'repair_coupon-1', item: 'repair_coupon', rect }] }), [], 'action none by default');
	assert.deepEqual(sliceProblems({ items: [{ id: 'a', item: 'repair_coupon', rect, rotation: -12, action: 'pickup', qty: 1 }] }), []);
	assert.deepEqual(
		sliceProblems({ items: [{ id: 'a', item: 'repair_coupon', rect, action: 'events', events: [{ type: 'showMessage', params: { text: 'Hi' } }] }] }),
		[],
		'events without a trigger (the tap is the trigger)'
	);
	assert.deepEqual(sliceProblems({ items: [{ id: 'a', item: 'x', rect, action: 'events' }] }), ['warning groups.0.slices.0.items.0.events']);
	assert.deepEqual(sliceProblems({ items: [{ item: 'x', rect, action: 'take', qty: 0 }] }), [
		'error groups.0.slices.0.items.0.id',
		'error groups.0.slices.0.items.0.action',
		'error groups.0.slices.0.items.0.qty'
	]);
	assert.deepEqual(sliceProblems({ items: [{ id: 'a', item: 'x', rect }, { id: 'a', item: 'y', rect }] }), ['error groups.0.slices.0.items']);
	const withCatalog = validateFile('chapter', chapter({ items: [{ id: 'a', item: 'nope', rect }] }), 'de', { file: 'x', items: ['repair_coupon'] } as never);
	assert.deepEqual(withCatalog.map((p) => `${p.level} ${p.path.join('.')}`).filter((p) => p.includes('slices.0.')), ['warning groups.0.slices.0.items.0.item']);
	// scale + rotation of text boxes; sfx is no variant any more
	assert.deepEqual(sliceProblems({ dialog: [{ rect, text: 'BAM', rotation: 10, scale: 1.5 }] }), []);
	assert.deepEqual(sliceProblems({ dialog: [{ rect, text: 'BAM', scale: 0 }] }), ['error groups.0.slices.0.dialog.0.scale']);
	assert.deepEqual(sliceProblems({ dialog: [{ rect, text: 'BAM', variant: 'sfx' }] }), ['error groups.0.slices.0.dialog.0.variant']);
});

test('onomatopoeia event → text box (params.dialog); bubble frame + tail; outline; pickup mode', () => {
	const rect = { x: 10, y: 10, width: 30 };
	const box = (extra: object = {}) => ({ id: 'b1', rect, text: 'BAM', ...extra });
	const ono = (params: object) => ({ type: 'showOnomatopeia', params, trigger: { type: 'intersect' } });
	const chapter = (slice: object, events?: object[]) => ({
		title: 'P',
		number: 2,
		groups: [{ label: 'g', renderer: { type: 'vertical' }, slices: [{ image: 's1/c2/a.webp', width: 390, height: 625, ...slice }], ...(events ? { events } : {}) }]
	});
	const list = (data: object) => problems('chapter', data).filter((p) => p.includes('groups.0.'));
	// a slice event revealing a box on its slice; an event without a box is an error (no own onomatopoeia any more)
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 'b1', effect: 'shiver', duration: 1500, delay: 200, mode: 'mono', wait: true })] })), []);
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({})] })), ['error groups.0.slices.0.events.0.params.dialog']);
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 'b1', effect: 'boom', mode: 'x', wait: 1 })] })), [
		'error groups.0.slices.0.events.0.params.effect',
		'error groups.0.slices.0.events.0.params.mode',
		'error groups.0.slices.0.events.0.params.wait'
	]);
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 'nope' })] })), ['warning groups.0.slices.0.events.0.params.dialog']);
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 3 })] })), ['error groups.0.slices.0.events.0.params.dialog']);
	// a group script may reveal a box on any of the group's slices
	const groupEvent = { type: 'showOnomatopeia', params: { dialog: 'b1' } };
	assert.deepEqual(list(chapter({ dialog: [box()] }, [groupEvent])), []);
	assert.deepEqual(list(chapter({}, [groupEvent])), ['warning groups.0.events.0.params.dialog']);
	// bubble: frame (a drawn bubble) + tail; outline
	assert.deepEqual(list(chapter({ dialog: [box({ variant: 'bubble', frame: true, tail: 'bottom-right', stroke: '#000', strokeWidth: 0.1, color: '#fff' })] })), []);
	assert.deepEqual(list(chapter({ dialog: [box({ tail: 'left', stroke: 3 })] })), [
		'error groups.0.slices.0.dialog.0.stroke',
		'error groups.0.slices.0.dialog.0.tail',
		'info groups.0.slices.0.dialog.0.tail'
	]);
	// placed item pickup mode
	const item = (extra: object) => ({ items: [{ id: 'tuna_can-1', item: 'tuna_can', rect, action: 'pickup', ...extra }] });
	for (const respawn of ['never', 'visit', { minutes: 30 }, { hours: 6 }, { days: 2 }]) assert.deepEqual(list(chapter(item({ respawn }))), [], JSON.stringify(respawn));
	for (const respawn of ['daily', { hours: 0.5 }, { hours: 1, days: 1 }, { weeks: 1 }]) {
		assert.deepEqual(list(chapter(item({ respawn }))), ['error groups.0.slices.0.items.0.respawn'], JSON.stringify(respawn));
	}
	// against the catalog's rule: tuna every 1 … 24 h (default 4 h), weed never
	const withRules = (respawn: unknown, id = 'tuna_can') =>
		validateFile('chapter', chapter({ items: [{ id: 'x-1', item: id, rect, action: 'pickup', respawn }] }), 'de', {
			file: 'x',
			itemRespawn: { tuna_can: { default: { hours: 4 }, min: { hours: 1 }, max: { days: 1 } }, weed: null }
		} as never).map((p) => `${p.level} ${p.path.join('.')}`).filter((p) => p.includes('items.0'));
	assert.deepEqual(withRules({ hours: 2 }), []);
	assert.deepEqual(withRules('never'), []);
	assert.deepEqual(withRules({ minutes: 30 }), ['error groups.0.slices.0.items.0.respawn'], 'shorter than the item allows');
	assert.deepEqual(withRules('visit'), ['error groups.0.slices.0.items.0.respawn'], 'every reload is shorter than 1 h');
	assert.deepEqual(withRules({ days: 2 }), ['error groups.0.slices.0.items.0.respawn'], 'longer than the item allows');
	assert.deepEqual(withRules({ hours: 2 }, 'weed'), ['error groups.0.slices.0.items.0.respawn'], 'the item never reappears');
	// a placed item's tap events may reveal a text box on its slice
	const tapEvents = (dialog: string) => ({ items: [{ id: 'x-1', item: 'x', rect, action: 'events', events: [{ type: 'showOnomatopeia', params: { dialog } }] }] });
	assert.deepEqual(list(chapter({ dialog: [box()], ...tapEvents('b1') })), []);
	assert.deepEqual(list(chapter({ dialog: [box()], ...tapEvents('b2') })), ['warning groups.0.slices.0.items.0.events.0.params.dialog']);
	// the box stays or disappears again; the wobbly (shout) bubble
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 'b1', stay: false })] })), []);
	assert.deepEqual(list(chapter({ dialog: [box()], events: [ono({ dialog: 'b1', stay: 'no' })] })), ['error groups.0.slices.0.events.0.params.stay']);
	assert.deepEqual(list(chapter({ dialog: [box({ variant: 'bubble', frame: true, shape: 'wobbly' })] })), []);
	assert.deepEqual(list(chapter({ dialog: [box({ variant: 'bubble', frame: true, shape: 'star' })] })), ['error groups.0.slices.0.dialog.0.shape']);
});

test('geometry is the same in every language: unifyGeometry copies the source locale geometry', () => {
	const box = (width: number, text: string, extra: object = {}) => ({ id: 'b', rect: { x: 1, y: 2, width }, text, ...extra });
	const chapter = (dialog: object[], items: object[] = []) => ({ groups: [{ slices: [{ image: 'a.webp', dialog, items }] }] });
	const files: Record<string, any> = {
		de: chapter([box(20, 'Hallo', { rotation: 5 })], [{ id: 'x-1', item: 'x', rect: { x: 1, y: 1, width: 10 } }]),
		en: chapter([box(25, 'Hello', { scale: 2 })], [{ id: 'x-1', item: 'x', rect: { x: 9, y: 1, width: 10 }, rotation: 3 }])
	};
	assert.equal(unifyGeometry('chapter', files, 'de'), 2);
	const en = files.en.groups[0].slices[0];
	assert.deepEqual(en.dialog[0], { id: 'b', rect: { x: 1, y: 2, width: 20 }, text: 'Hello', rotation: 5 }, 'the text stays per language');
	assert.deepEqual(en.items[0], { id: 'x-1', item: 'x', rect: { x: 1, y: 1, width: 10 } });
	assert.equal(unifyGeometry('chapter', files, 'de'), 0, 'nothing left to do');
});

test('VN flow in a group: enter before speaking, choices added then shown (owner 2026-10-03)', () => {
	const group = (events: object[], slices: object[] = []) => ({ label: 'g', renderer: { type: 'vertical' }, slices, events });
	const flow = (g: object) => problems('group', g).filter((p) => /^warning (events|slices\.\d+\.events)/.test(p));
	const msg = (character: string) => ({ type: 'showMessage', params: { text: 'Hi', character } });
	const join = (character: string) => ({ type: 'joinScene', params: { characters: [{ character, persona: 'regular', variant: 'avatar' }] } });
	const leave = (character: string) => ({ type: 'leaveScene', params: { characters: [{ character }] } });
	// the narrator (no character) never needs to enter
	assert.deepEqual(flow(group([{ type: 'showMessage', params: { text: 'Once upon a time' } }])), []);
	assert.deepEqual(flow(group([join('catpope'), msg('catpope'), leave('catpope')])), []);
	assert.deepEqual(flow(group([msg('catpope')])), ['warning events.0.params.character']);
	assert.deepEqual(flow(group([join('catpope'), leave('catpope'), msg('catpope')])), ['warning events.2.params.character']);
	// joining again updates the character on stage (another look / view) – no problem
	assert.deepEqual(flow(group([join('catpope'), join('catpope'), msg('catpope')])), []);
	assert.deepEqual(flow(group([leave('boy')])), ['warning events.0.params.characters.0']);
	// slice events continue the group's scene, in reading order
	const slice = (events: object[]) => ({ image: 'a.webp', width: 390, height: 625, events: events.map((e) => ({ ...e, trigger: { type: 'intersect' } })) });
	assert.deepEqual(flow(group([join('catpope')], [slice([msg('catpope')])])), []);
	assert.deepEqual(flow(group([], [slice([msg('catpope')]), slice([join('catpope')])])), ['warning slices.0.events.0.params.character']);
	// choices
	const add = { type: 'addChoice', params: { label: 'A' } };
	assert.deepEqual(flow(group([add, add, { type: 'showChoices' }])), []);
	assert.deepEqual(flow(group([{ type: 'showChoices' }])), ['warning events.0']);
	assert.deepEqual(flow(group([add])), ['warning events.0']);
	// a divert / jump leaves before they are shown (the stroll end's showChoices further down doesn't count)
	assert.deepEqual(flow(group([add, { type: 'divert', params: { to: 'x' } }, { type: 'label', params: { name: 'x' } }, add, { type: 'showChoices' }])), ['warning events.0']);
});

test('audio is the same in every language: unifyAudio replaces the old audio.samples of a locale with the source audio', () => {
	const samples = [{ id: 'bells', trigger: { type: 'intersect' }, volume: 0.8 }];
	const files: Record<string, any> = {
		de: { audio: { sampleLibrary: [{ id: 'bells', path: 's1/c1/b.128.mp3' }] }, groups: [{ slices: [{ image: 'a.webp', audio: { audio: { samples } } }, { image: 'b.webp' }] }] },
		en: { audio: { sampleLibrary: [{ id: 'bells', path: 's1/c1/b.128.mp3' }] }, groups: [{ slices: [{ image: 'a.webp', audio: { samples } }, { image: 'b.webp', audio: { music: { tracks: [{ id: 'x', volume: 1 }] } } }] }] }
	};
	assert.equal(unifyAudio('chapter', files, 'de'), 2);
	const en = files.en.groups[0].slices;
	assert.deepEqual(en[0].audio, { audio: { samples } });
	assert.equal(en[1].audio, undefined);
	assert.equal(unifyAudio('chapter', files, 'de'), 0);
	assert.ok(!problems('chapter', files.en).some((p) => p.includes('audio.samples')));
});
