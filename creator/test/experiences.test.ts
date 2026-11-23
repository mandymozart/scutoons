// The Experiences workspace's logic: the list's rows, the facts, the outline (scenes › surfaces › entries), the shared
// entities' node counts and the plugins, all read defensively from a document the server checks only roughly.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { CatalogDoc } from '@scutoons/api';
import { contentUrl, entityCounts, experienceContents, experienceFacts, experienceHref, experienceOutline, experienceRows, experienceState, kb, pluginRows, readerHref, surfaceSize, titleIn } from '../src/lib/editor/experiences.ts';

const doc = (id: string, content: Record<string, unknown>, state: Partial<CatalogDoc> = {}): CatalogDoc => ({
	id,
	key: `experience:${id}`,
	latest: 3,
	published: null,
	live: false,
	draft: true,
	removed: false,
	versions: 1,
	latest_at: null,
	latest_author: null,
	latest_message: null,
	physical: false,
	toons: ['*'],
	content,
	...state
});

const configuration = {
	version: { schema: '5.1' },
	scenes: [
		{ id: 'poster', label: 'Poster', surfaces: [{ id: 'front', label: 'Front', widthMm: 420, heightMm: 594 }] },
		{ id: 'spread', label: 'Spread', surfaces: [{ id: 'left', widthMm: 210, heightMm: 297 }, { id: 'right', widthMm: 210, heightMm: 297 }] }
	],
	entries: [
		{ id: 'pope', title: 'Cat Pope', category: 'image', scene: 'poster', target: { id: 'pope-t', imageSrc: '/assets/content/targets/pope.jpg', entity: { ref: 'overlay' } } },
		{ id: 'r', title: 'Right', scene: 'spread', surface: 'right', target: { imageSrc: '/assets/content/targets/r.jpg', enabled: false } },
		{ id: 'nowhere', title: 'Lost', scene: 'spread' },
		{ id: 'ghost', title: 'Ghost', scene: 'gone' },
		'not an entry'
	],
	entities: {
		overlay: { type: 'group', children: [{ type: 'model' }, { type: 'model' }, { type: 'light', children: [{ type: 'text' }] }] },
		plain: { type: 'image' }
	}
};

test('rows: removed ones left out, sorted by the title in the language, the state from the versions', () => {
	const rows = experienceRows(
		[
			doc('b', { title: { en: 'Zebra', de: 'Affe' }, type: 'poster', toons: ['catpope', 3] }),
			doc('a', { title: { en: 'Moon' }, type: 'book' }, { published: 3, draft: false }),
			doc('gone', { removed: true }, { removed: true }),
			doc('raw', 'nonsense' as unknown as Record<string, unknown>)
		],
		'de'
	);
	assert.deepEqual(rows.map((r) => r.id), ['b', 'a', 'raw']);
	assert.deepEqual(rows[0], { id: 'b', key: 'experience:b', title: { en: 'Zebra', de: 'Affe' }, type: 'poster', toons: ['catpope'], state: 'unpublished', latest: 3 });
	assert.equal(rows[1].state, 'published');
	assert.equal(titleIn(rows[2].title, 'de', rows[2].id), 'raw');
	assert.equal(experienceState({ latest: 5, published: 3, draft: true }), 'draft');
	assert.equal(experienceState({ latest: null, published: null, draft: false }), 'published', 'only in the table: readers have it');
});

test('facts: the source, the folder trimmed, anything missing empty', () => {
	const f = experienceFacts({ id: 'poster-catpope', type: 'poster', title: { en: 'Cat Pope', de: '' }, toons: ['catpope'], assets: '/xr/poster-catpope/e8f9/', source: { osct: { experience: 'demo', scene: 'catpope-poster', build: 'e8f9', schema: '5.1', app: '4.1.0' } } }, 'x');
	assert.deepEqual(f, { id: 'poster-catpope', type: 'poster', title: { en: 'Cat Pope' }, toons: ['catpope'], assets: 'xr/poster-catpope/e8f9', source: { experience: 'demo', scene: 'catpope-poster', build: 'e8f9', schema: '5.1', app: '4.1.0' } });
	assert.deepEqual(experienceFacts(null, 'x'), { id: 'x', type: '', title: {}, toons: [], source: null, assets: '' });
});

test('a target picture lies below the experience folder on the CDN; a full URL stays', () => {
	const cdn = (p: string) => `https://cdn.test/${p}`;
	assert.equal(contentUrl('xr/p/b1', '/assets/content/targets/pope.jpg', cdn), 'https://cdn.test/xr/p/b1/assets/content/targets/pope.jpg');
	assert.equal(contentUrl('', 'a.jpg', cdn), 'https://cdn.test/a.jpg');
	assert.equal(contentUrl('xr/p', 'https://elsewhere/a.jpg', cdn), 'https://elsewhere/a.jpg');
	assert.equal(contentUrl('xr/p', '', cdn), null);
});

test('outline: scenes › surfaces › entries; the only surface takes the entries without one, the rest are loose or orphans', () => {
	const o = experienceOutline(configuration);
	assert.equal(o.entries, 4);
	assert.deepEqual(o.scenes.map((s) => [s.id, s.surfaces.map((x) => [x.id, x.entries.map((e) => e.id)]), s.loose.map((e) => e.id)]), [
		['poster', [['front', ['pope']]], []],
		['spread', [['left', []], ['right', ['r']]], ['nowhere']]
	]);
	assert.deepEqual(o.scenes[0].surfaces[0].entries[0], { id: 'pope', title: 'Cat Pope', category: 'image', target: { id: 'pope-t', image: '/assets/content/targets/pope.jpg', entity: 'overlay', enabled: true } });
	assert.deepEqual(o.scenes[1].surfaces[1].entries[0].target, { id: 'r', image: '/assets/content/targets/r.jpg', entity: null, enabled: false });
	assert.deepEqual(o.orphans.map((e) => e.id), ['ghost']);
	assert.equal(surfaceSize(o.scenes[0].surfaces[0]), '420 × 594 mm');
	assert.equal(surfaceSize({ widthMm: null, heightMm: 3 }), '');
	assert.deepEqual(experienceOutline({ scenes: 'no', entries: null }), { scenes: [], orphans: [], entries: 0 });
});

test('shared entities: every node counted by type, most first', () => {
	assert.deepEqual(entityCounts(configuration), [
		{ id: 'overlay', nodes: 5, types: [{ type: 'model', count: 2 }, { type: 'group', count: 1 }, { type: 'light', count: 1 }, { type: 'text', count: 1 }] },
		{ id: 'plain', nodes: 1, types: [{ type: 'image', count: 1 }] }
	]);
	assert.deepEqual(entityCounts(undefined), []);
});

test('plugins: id, API, the code size in bytes; the contents in one call', () => {
	const plugins = [{ id: 'toon', api: '3.1', code: 'export const ä = 1;' }, { id: 'empty', api: '3.1' }, 7];
	assert.deepEqual(pluginRows(plugins), [
		{ id: 'toon', api: '3.1', bytes: 20 },
		{ id: 'empty', api: '3.1', bytes: 0 }
	]);
	assert.equal(kb(20), 1);
	assert.equal(kb(0), 0);
	assert.equal(kb(204800), 200);
	const c = experienceContents({ configuration, plugins });
	assert.equal(c.outline.scenes.length, 2);
	assert.equal(c.entities.length, 2);
	assert.equal(c.plugins.length, 2);
});

test('the links: the page in the creator, the reader preview of the draft', () => {
	assert.equal(experienceHref('poster-catpope'), '/experiences/poster-catpope');
	assert.equal(readerHref('https://scutoons.com', 'poster-catpope'), 'https://scutoons.com/xr/poster-catpope?draft=1');
});
