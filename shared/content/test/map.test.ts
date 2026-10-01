// Unit tests of the world map (agents/MAP.md): migration (every value survives), the old answer, the state merge, the
// progress-based unlocks, labels, validation.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	bezierPath,
	chapterSlices,
	placesUnlockedBy,
	sliceUnlockPoint,
	checkWorldMap,
	emptyMapState,
	isPlaceVisible,
	legacyToonMap,
	legacyToonMapSettings,
	markPlacesSeen,
	mergeMapState,
	migrateLegacyMap,
	resolveWorldMap,
	revealRoute,
	routeDrawnTo,
	routePoints,
	unlockPlace,
	unlocksFromProgress,
	visibleLabels,
	type MapLocation,
	type MapState,
	type WorldMap
} from '../src/index.ts';

const seed = new URL('../seed/', import.meta.url);
const json = (path: string) => JSON.parse(readFileSync(new URL(path, seed), 'utf8'));

test('migration: every old catpope value survives in the seed world map (ids, coordinates, labels, route order)', () => {
	const world: WorldMap = json('scutoons/_/map.json');
	for (const locale of ['de', 'en']) {
		const old: MapLocation[] = json(`catpope/${locale}/locations.json`);
		const computed = legacyToonMap(world, 'catpope', locale);
		for (const o of old) {
			const n = computed.locations.find((l) => l.id === o.id);
			assert.ok(n, `${o.id} is still there`);
			assert.deepEqual({ ...n, chapterNumber: undefined }, { ...o, chapterNumber: undefined }, `${locale} ${o.id}`);
		}
		const oldRoute = json(`catpope/${locale}/routes.json`)[0];
		assert.deepEqual(computed.routes, [{ id: oldRoute.id, title: oldRoute.title ?? oldRoute.titel, locations: oldRoute.locations }]);
	}
	// the toon's old map settings come back from the world + the start view
	const toons = json('toons.json');
	assert.deepEqual(legacyToonMapSettings(world, 'catpope', toons.find((t: any) => t.id === 'catpope').map), {
		layers: { terrain: 'images/map/map-layer-terrain.svg' },
		width: 2048,
		height: 1046,
		center: { x: 1320, y: 440 }
	});
	assert.equal(legacyToonMapSettings(world, 'sandbox', toons.find((t: any) => t.id === 'sandbox').map).layers.terrain, '../catpope/images/map/map-layer-terrain.svg');
	// owner 2026-10-01: garden is a place now (the route is complete), places unlock over time
	assert.equal(checkWorldMap(world).filter((p) => p.level === 'error').length, 0);
	assert.ok(world.locations.every((p) => p.hidden));
	assert.deepEqual(world.locations.find((p) => p.id === 'garden')?.unlock, { toon: 'catpope', chapter: 5, at: 1, scene: 38 });
});

test('migrateLegacyMap: names per locale, the en `titel` typo, unknown route places kept', () => {
	const loc = (id: string, label: string) => ({ id, label, toonId: 't', type: 'chapter', chapterNumber: '2', image: '', coordinates: { x: 1, y: 2 } });
	const { map, view } = migrateLegacyMap({
		toon: 't',
		toonMap: { layers: { terrain: 'images/map.svg' }, width: 10, height: 20, center: { x: 3, y: 4 } },
		source: 'de',
		locations: { de: [loc('a', 'Ah')], en: [loc('a', 'Ay')] },
		routes: { de: [{ id: 'r', title: 'Weg', locations: ['a', 'gone'] }], en: [{ id: 'r', titel: 'Way', locations: ['a', 'gone'] }] }
	});
	assert.deepEqual(map.layers, [{ id: 'terrain', image: 't/images/map.svg' }]);
	assert.deepEqual(map.locations, [{ id: 'a', name: { de: 'Ah', en: 'Ay' }, x: 1, y: 2, toons: ['t'], chapter: { toon: 't', number: 2 } }]);
	assert.deepEqual(map.routes, [{ id: 'r', name: { de: 'Weg', en: 'Way' }, toons: ['t'], waypoints: [{ location: 'a' }, { location: 'gone' }], style: 'road' }]);
	assert.deepEqual(view, { center: { x: 3, y: 4 } });
	assert.deepEqual(
		checkWorldMap(map).map((p) => `${p.level} ${p.path.join('.')}`),
		['error routes.0.waypoints.1'],
		'the dangling id is reported'
	);
});

test('mergeMapState: union, earliest unlock, largest reveal, newest position; commutative and idempotent', () => {
	const a: MapState = { places: { home: { unlocked: 100, seen: 150 } }, routes: { way: { revealed: 1 } }, positions: { t: { location: 'home', at: 100 } } };
	const b: MapState = { places: { home: { unlocked: 50 }, tower: { unlocked: 300 } }, routes: { way: { revealed: 0, travelled: 0, seen: 0 } }, positions: { t: { location: 'tower', at: 300 } } };
	const ab = mergeMapState(a, b);
	assert.deepEqual(ab, {
		places: { home: { unlocked: 50, seen: 150 }, tower: { unlocked: 300 } },
		routes: { way: { revealed: 1, travelled: 0, seen: 0 } },
		positions: { t: { location: 'tower', at: 300 } }
	});
	assert.deepEqual(mergeMapState(b, a), ab);
	assert.deepEqual(mergeMapState(ab, ab), ab);
	assert.deepEqual(mergeMapState(null, undefined), emptyMapState());
	assert.deepEqual(mergeMapState({ places: { x: { unlocked: 'no' } } } as never, null), emptyMapState(), 'junk is dropped');
});

test('state operations are idempotent set operations', () => {
	const s = emptyMapState();
	assert.equal(unlockPlace(s, 'home', 1), true);
	assert.equal(unlockPlace(s, 'home', 2), false);
	assert.equal(revealRoute(s, 'way', 2), true);
	assert.equal(revealRoute(s, 'way', 1), false);
	assert.deepEqual(markPlacesSeen(s, ['home', 'nope'], 5), ['home']);
	assert.deepEqual(markPlacesSeen(s, ['home'], 6), []);
	assert.deepEqual(s.places.home, { unlocked: 1, seen: 5 });
});

test('unlocksFromProgress: a hidden place unlocks when its chapter is read up to its point', () => {
	const places = [
		{ id: 'home', hidden: true, unlock: { toon: 't', chapter: 1, at: 0 } },
		{ id: 'street', hidden: true, unlock: { toon: 't', chapter: 1, at: 0.8 } },
		{ id: 'end', hidden: true, unlock: { toon: 't', chapter: 2, at: 1 } },
		{ id: 'other', hidden: true, unlock: { toon: 'x', chapter: 1 } },
		{ id: 'open', unlock: { toon: 't', chapter: 1 } }
	];
	const cp = (chapter: number, position: number, isComplete = false) => ({ uri: `t/chapter/${chapter}`, position, isComplete });
	assert.deepEqual(unlocksFromProgress(places, 't', []), []);
	assert.deepEqual(unlocksFromProgress(places, 't', [cp(1, 0.1)]), ['home']);
	assert.deepEqual(unlocksFromProgress(places, 't', [cp(1, 0.8)]), ['home', 'street']);
	assert.deepEqual(unlocksFromProgress(places, 't', [cp(1, 0.2, true), cp(2, 0.99)]), ['home', 'street'], 'finished once = every point; 0.99 < the end');
	assert.deepEqual(unlocksFromProgress(places, 't', [cp(2, 1, true)]), ['end']);
});

test('visibility and the drawn part of a route', () => {
	const s = emptyMapState();
	const route = { id: 'season1', waypoints: [{ location: 'airport' }, { location: 'home' }, { location: 'street' }, { location: 'garden' }] };
	const visible = (id: string) => isPlaceVisible({ id, hidden: true }, s);
	assert.equal(routeDrawnTo(route, s, visible), 3, 'a visible route is drawn entirely (invisible places are skipped when drawing)');
	assert.equal(routeDrawnTo({ ...route, hidden: true }, s, visible), -1);
	unlockPlace(s, 'street', 1);
	assert.equal(routeDrawnTo({ ...route, hidden: true }, s, visible), 2, 'a hidden route: up to its last visible place');
	revealRoute(s, 'season1', 3);
	assert.equal(routeDrawnTo({ ...route, hidden: true }, s, visible), 3);
	const places = [{ id: 'home', x: 0, y: 0 }, { id: 'street', x: 10, y: 0 }];
	assert.deepEqual(routePoints(places, [...route.waypoints, { id: 'p', x: 20, y: 5 }]), [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 20, y: 5 }]);
	assert.match(bezierPath([{ x: 0, y: 0 }, { x: 10, y: 0 }]), /^M 0 0 C /);
	assert.equal(bezierPath([{ x: 0, y: 0 }]), '');
});

test('visibleLabels hides overlapping labels (first wins)', () => {
	const box = (id: string, x: number, y: number) => ({ id, x, y, width: 80, height: 14 });
	assert.deepEqual([...visibleLabels([box('a', 100, 100), box('b', 120, 105), box('c', 300, 100)])], ['a', 'c']);
});

test('resolveWorldMap: names in a locale, fallback to de', () => {
	const map: WorldMap = {
		width: 100,
		height: 100,
		layers: [],
		locations: [{ id: 'a', name: { de: 'Ah' }, x: 1, y: 1, toons: ['t'] }],
		routes: [{ id: 'r', name: { de: 'W', en: 'Way' }, toons: ['t'], waypoints: [{ location: 'a' }, { id: 'p', x: 2, y: 2, name: { de: 'Pass' } }] }]
	};
	const r = resolveWorldMap(map, 'en');
	assert.equal(r.locations[0].name, 'Ah');
	assert.equal(r.routes[0].name, 'Way');
	assert.deepEqual(r.routes[0].waypoints[1], { id: 'p', x: 2, y: 2, name: 'Pass' });
});

test('checkWorldMap: ids, positions, waypoints, unlock points', () => {
	const problems = (map: object) => checkWorldMap(map, { toons: ['t'] }).map((p) => `${p.level} ${p.path.join('.')}`);
	const ok = { width: 100, height: 50, layers: [{ id: 'terrain', image: 'a/t.svg' }], locations: [{ id: 'a', name: { de: 'A' }, x: 1, y: 1, toons: ['t'] }], routes: [] };
	assert.deepEqual(problems(ok), []);
	assert.deepEqual(problems({ ...ok, locations: [...ok.locations, { id: 'a', name: { de: 'B' }, x: 200, y: 1, toons: [] }] }), ['error locations.1.id', 'error locations.1.x', 'warning locations.1.toons']);
	assert.deepEqual(problems({ ...ok, routes: [{ id: 'r', name: { de: 'R' }, toons: ['t'], waypoints: [{ location: 'a' }] }] }), ['error routes.0.waypoints']);
	assert.deepEqual(problems({ ...ok, routes: [{ id: 'r', name: { de: 'R' }, toons: ['t'], style: 'boat', waypoints: [{ location: 'a' }, { id: 'a', x: 1, y: 1 }] }] }), ['error routes.0.style', 'error routes.0.waypoints.1.id']);
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], hidden: true }] }), ['warning locations.0.hidden'], 'hidden and nothing unlocks it');
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], hidden: true, unlock: { toon: 't', chapter: 1, at: 2 } }] }), ['error locations.0.unlock.at']);
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], image: { src: 'map/a.webp', anchor: { x: 50, y: 120 } } }] }), ['error locations.0.image.anchor']);
});

test('the unlocking slice: its reading point (the reader checkpoint rule) and the places it unlocks', () => {
	const groups = [{ slices: [{ image: 'a.webp' }, { image: 'b.webp' }] }, { label: 'scene' }, { slices: [{ image: 'c.webp' }, { image: 'd.webp' }] }];
	const slices = chapterSlices(groups);
	assert.deepEqual(
		slices.map((s) => [s.image, s.group, s.index, s.overall]),
		[['a.webp', 0, 0, 0], ['b.webp', 0, 1, 1], ['c.webp', 2, 0, 2], ['d.webp', 2, 1, 3]]
	);
	assert.equal(sliceUnlockPoint(slices, 'a.webp'), 0.25);
	assert.equal(sliceUnlockPoint(slices, 'c.webp'), 0.75);
	assert.equal(sliceUnlockPoint(slices, 'd.webp'), 1, 'the last slice = the end');
	assert.equal(sliceUnlockPoint(slices, 'x.webp'), null);
	const many = chapterSlices([{ slices: Array.from({ length: 200 }, (_, i) => ({ image: `${i}.webp` })) }]);
	assert.equal(sliceUnlockPoint(many, '198.webp'), 0.99, 'capped at 0.99 before the end, like the reader');
	const map = {
		locations: [
			{ id: 'p', name: { de: 'P' }, x: 1, y: 1, toons: ['t'], hidden: true, unlock: { toon: 't', chapter: 1, slice: 'b.webp', at: 0.5 } },
			{ id: 'q', name: { de: 'Q' }, x: 1, y: 1, toons: ['t'], hidden: true, unlock: { toon: 't', chapter: 2, slice: 'b.webp' } }
		]
	};
	assert.deepEqual(placesUnlockedBy(map, 't', 1, 'b.webp').map((p) => p.id), ['p']);
	assert.deepEqual(placesUnlockedBy(map, 't', 1, ''), []);
});
