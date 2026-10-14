// The avatar's home: walking, furniture, home scenes, the avatar position (agents/research/CAVE.md).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	characterHome,
	emptyMapState,
	furnitureDepth,
	mergeMapState,
	normalizeMapState,
	setAvatarPosition,
	validateCatalog,
	walkPath,
	type ItemDef,
	type RouteLike
} from '../src/index.ts';

const base = { toon: null, name: { en: 'x' }, description: {}, kind: 'virtual', icon: 'x.png', stackable: false, starter: false, grantable: ['story'], slot: null, skills: [], edition_id: null, product: null, variants: null } as const;
const def = (id: string, extra: Partial<ItemDef>): ItemDef => ({ ...base, id, category: 'collectible', ...extra }) as ItemDef;
const at = (location: string) => ({ location });

// home – square – cathedral; square – station; the crypt lies behind a hidden route
const routes: RouteLike[] = [
	{ id: 'ring', waypoints: [at('home'), { id: 'bend', x: 1, y: 1 } as any, at('square'), at('cathedral')] },
	{ id: 'line', waypoints: [at('square'), at('station')] },
	{ id: 'secret', hidden: true, waypoints: [at('cathedral'), at('crypt')] }
];
const state = emptyMapState();
const unlocked = new Set(['home', 'square', 'cathedral', 'station']);
const open = (id: string) => unlocked.has(id);

test('the avatar walks the fewest stretches along visible routes, between open places', () => {
	assert.deepEqual(walkPath(routes, state, 'cathedral', 'home', open), [
		{ route: 'ring', from: 3, to: 2, place: 'square' },
		{ route: 'ring', from: 2, to: 0, place: 'home' }
	]);
	assert.deepEqual(walkPath(routes, state, 'home', 'station', open)?.map((s) => s.place), ['square', 'station']);
	assert.deepEqual(walkPath(routes, state, 'home', 'home', open), []);
});

test('no walk to a locked place or along a hidden route (no spoilers)', () => {
	assert.equal(walkPath(routes, state, 'cathedral', 'crypt', open), null, 'crypt is locked');
	const revealed = { ...emptyMapState(), routes: { secret: { revealed: 0 } } };
	assert.equal(walkPath(routes, revealed, 'cathedral', 'crypt', () => true), null, 'revealed only up to the cathedral');
	revealed.routes.secret.revealed = 1;
	assert.deepEqual(walkPath(routes, revealed, 'cathedral', 'crypt', () => true)?.map((s) => s.place), ['crypt']);
	assert.equal(walkPath(routes, state, 'home', 'cathedral', (id) => id !== 'square'), null, 'a locked place in between blocks the way');
});

test('furniture is smaller further back', () => {
	assert.equal(furnitureDepth(100), 1);
	assert.equal(furnitureDepth(40), 0.65);
	assert.ok(furnitureDepth(70) > 0.65 && furnitureDepth(70) < 1);
});

test('a character’s home: its place and scene (cave by default)', () => {
	assert.deepEqual(characterHome({ home: 'church-square', home_scene: 'tatami_room' }), { place: 'church-square', scene: 'tatami_room' });
	assert.deepEqual(characterHome({}), { place: undefined, scene: 'cave' });
});

test('the catalog checks home scenes, furniture and workshop recipes', () => {
	const catalog = {
		slots: [],
		home_scenes: [
			{ id: 'cave', name: { en: 'Cave' }, image: { src: 'scutoons/homes/cave.1080p.webp', variants: { high: 'scutoons/homes/cave.4k.webp' } }, video: { src: 'scutoons/homes/cave.1080p.mp4' } },
			{ id: 'cave', name: { en: 'Again' }, image: { src: '' }, video: { src: 'cave.gif' } }
		],
		items: [
			def('workbench', { category: 'furniture', furniture: { size: 35, image: 'scutoons/furniture/workbench.webp', workshop: true } }),
			def('cd_rack', { category: 'furniture', furniture: { size: 12, image: '' }, holds: { categories: ['media'], max: 50 } }),
			def('giant', { category: 'furniture', furniture: { size: 90, image: 'x.webp' } }),
			def('no_size', { category: 'furniture' }),
			def('vr_headset', { category: 'tool', skills: [{ type: 'enter_home' }] }),
			def('songbook_diy_broken', {}),
			def('songbook_diy', {})
		],
		recipes: [
			{ id: 'repair', at: 'workshop', inputs: [{ item: 'songbook_diy_broken', qty: 1 }], output: { item: 'songbook_diy', qty: 1 } },
			{ id: 'odd', at: 'kitchen', inputs: [{ item: 'songbook_diy_broken', qty: 1 }], output: { item: 'songbook_diy', qty: 1 } }
		]
	};
	const problems = validateCatalog(catalog).map((p) => `${p.level} ${p.path.join('.')}`);
	assert.ok(problems.includes('error home_scenes.1.id'), 'duplicate scene');
	assert.ok(problems.includes('warning home_scenes.1.image'), 'scene without a picture');
	assert.ok(problems.includes('error home_scenes.1.video'), 'a video is an .mp4 / .webm');
	assert.ok(!problems.some((p) => p.includes('home_scenes.0')), 'the first scene is fine');
	assert.ok(problems.includes('warning items.1.furniture.image'), 'furniture without a home image');
	assert.ok(problems.includes('error items.2.furniture'), 'size out of range');
	assert.ok(problems.includes('error items.3.furniture'), 'the category needs furniture');
	assert.ok(problems.includes('error recipes.1.at'), 'only workshop');
	assert.ok(!problems.some((p) => p.startsWith('error items.0') || p.startsWith('error items.4') || p.includes('recipes.0')), problems.join('\n'));
});

test('the avatar position: the newer one wins, a map reset sends it home', () => {
	const a = emptyMapState();
	setAvatarPosition(a, { location: 'square' }, 100);
	const b = emptyMapState();
	setAvatarPosition(b, { location: 'station' }, 200);
	assert.deepEqual(mergeMapState(a, b).avatar, { location: 'station', at: 200 });
	assert.deepEqual(mergeMapState(b, a).avatar, { location: 'station', at: 200 });
	assert.equal(mergeMapState(a, { ...emptyMapState(), resetAt: 150 }).avatar, undefined);
	assert.equal(normalizeMapState({ avatar: { location: 3 } }).avatar, undefined, 'junk dropped');
});
