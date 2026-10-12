// Unit tests of the world map (agents/MAP.md): migration (every value survives), the old answer, the state merge, the
// progress-based unlocks, labels, validation.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	bezierPath,
	chapterSlices,
	checkWorldMap,
	emptyMapState,
	isPlaceVisible,
	mapImageRequests,
	placeImageTarget,
	mapImageRequestsText,
	mapMediaRequests,
	placeImagePath,
	markPlacesSeen,
	mergeMapState,
	migrateLegacyMap,
	resolveWorldMap,
	revealRoute,
	routeDrawnTo,
	routePoints,
	unlockPlace,
	placeView,
	PLACE_ZOOM,
	visibleLabels,
	type MapLocation,
	type MapState,
	type WorldMap
} from '../src/index.ts';

const seed = new URL('../../../server/tests/fixtures/content/', import.meta.url);
const json = (path: string) => JSON.parse(readFileSync(new URL(path, seed), 'utf8'));

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
	assert.deepEqual(map.routes, [{ id: 'r', name: { de: 'Weg', en: 'Way' }, toons: ['t'], waypoints: [{ location: 'a' }, { location: 'gone' }] }]);
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

test('checkWorldMap: ids, positions, waypoints; the old unlock link is an error', () => {
	const problems = (map: object) => checkWorldMap(map, { toons: ['t'] }).map((p) => `${p.level} ${p.path.join('.')}`);
	const ok = { width: 100, height: 50, layers: [{ id: 'terrain', image: 'a/t.svg' }], locations: [{ id: 'a', name: { de: 'A' }, x: 1, y: 1, toons: ['t'] }], routes: [] };
	assert.deepEqual(problems(ok), []);
	assert.deepEqual(problems({ ...ok, locations: [...ok.locations, { id: 'a', name: { de: 'B' }, x: 200, y: 1, toons: [] }] }), ['error locations.1.id', 'error locations.1.x', 'warning locations.1.toons']);
	assert.deepEqual(problems({ ...ok, routes: [{ id: 'r', name: { de: 'R' }, toons: ['t'], waypoints: [{ location: 'a' }] }] }), ['error routes.0.waypoints']);
	assert.deepEqual(problems({ ...ok, routes: [{ id: 'r', name: { de: 'R' }, toons: ['t'], style: 'boat', waypoints: [{ location: 'a' }, { id: 'a', x: 1, y: 1 }] }] }), ['error routes.0.style', 'error routes.0.waypoints.1.id']);
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], hidden: true }] }), [], 'hidden: an unlockLocation event unlocks it (the map cannot see the events)');
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], hidden: true, unlock: null }] }), ['error locations.0.unlock'], 'unlock is gone (2026-10-11)');
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], hidden: true, unlock: { toon: 't', chapter: 1 } }] }), ['error locations.0.unlock']);
	assert.deepEqual(problems({ ...ok, locations: [{ ...ok.locations[0], image: { src: 'map/a.webp', anchor: { x: 50, y: 120 } } }] }), ['error locations.0.image.anchor']);
});

test('a place’s view: its own zoom (0.25–32, default 8) and offset; validation', () => {
	const problems = (view: unknown) =>
		checkWorldMap({ width: 100, height: 50, layers: [], locations: [{ id: 'a', name: { de: 'A' }, x: 10, y: 10, toons: ['t'], view }], routes: [] }, { toons: ['t'] }).map((p) => `${p.level} ${p.path.join('.')}`);
	assert.deepEqual(problems(undefined), []);
	assert.deepEqual(problems({ zoom: 3, offset: { x: 5, y: -5 } }), []);
	assert.deepEqual(problems({ zoom: 1.5 }), [], 'below the default 8 is fine (owner 2026-10-11: "kleiner geht schon")');
	assert.deepEqual(problems({ zoom: 0.1 }), ['error locations.0.view.zoom'], 'below the map minimum');
	assert.deepEqual(problems({ zoom: 33 }), ['error locations.0.view.zoom']);
	assert.deepEqual(problems({ offset: { x: 'a', y: 0 } }), ['error locations.0.view.offset']);
	assert.deepEqual(problems({ offset: { x: 200, y: 0 } }), ['warning locations.0.view.offset']);
	assert.deepEqual(problems(7), ['error locations.0.view']);
	// the reader: the point + offset at the place zoom; without a view: the point at 8; junk → ignored, out of range → clamped
	assert.equal(PLACE_ZOOM, 8);
	assert.deepEqual(placeView({ x: 10, y: 20 }), { center: { x: 10, y: 20 }, zoom: 8 });
	assert.deepEqual(placeView({ x: 10, y: 20, view: { zoom: 4, offset: { x: 5, y: -3 } } }), { center: { x: 15, y: 17 }, zoom: 4 });
	assert.deepEqual(placeView({ x: 10, y: 20, view: { zoom: 1 } }), { center: { x: 10, y: 20 }, zoom: 1 });
	assert.deepEqual(placeView({ x: 10, y: 20, view: { zoom: 0.1 } }), { center: { x: 10, y: 20 }, zoom: 0.25 });
	assert.deepEqual(placeView({ x: 10, y: 20, view: { zoom: NaN, offset: { x: NaN, y: 1 } } as any }), { center: { x: 10, y: 20 }, zoom: 8 });
});

test('wanted images: every place without an image, derived from the map (all 20 of the fixture)', () => {
	const world: WorldMap = json('scutoons/_/map.json');
	const wanted = mapImageRequests(world);
	assert.equal(wanted.length, world.locations.filter((p) => !p.image?.src).length);
	assert.equal(wanted.length, 20, 'every catpope place incl. Garten Eden (no image yet)');
	const garden = wanted.find((r) => r.place === 'garden')!;
	assert.equal(garden.path, 'catpope/map/places/garden.webp');
	assert.equal(garden.size, '256 × 256');
	assert.match(garden.brief, /Garden of Eden/);
	assert.match(wanted.find((r) => r.place === 'cathedral')!.brief, /St. Stephen's cathedral/);
	assert.equal(placeImagePath({ id: 'x', toons: ['a', 'b'] }), 'scutoons/map/places/x.webp', 'a shared place goes to scutoons/');
	assert.equal(mapImageRequests({ locations: [{ ...world.locations[0], image: { src: 'a.webp' } }] }).length, 0);
	assert.match(mapImageRequestsText(wanted.slice(0, 1)), /catpope\/map\/places/);
	const media = mapMediaRequests(world);
	assert.equal(media.length, 20);
	assert.ok(media.every((r) => r.view === 'media' && r.toon === 'catpope' && r.size === '256 × 256' && r.brief && r.status === 'open'));
	assert.equal(media.find((r) => r.id === 'map-place:airport')?.path, 'catpope/map/places/airport.webp');
});

test('a chapter’s slices in reading order', () => {
	const groups = [{ slices: [{ image: 'a.webp' }, { image: 'b.webp' }] }, { label: 'scene' }, { slices: [{ image: 'c.webp' }, { image: 'd.webp' }] }];
	const slices = chapterSlices(groups);
	assert.deepEqual(
		slices.map((s) => [s.image, s.group, s.index, s.overall]),
		[['a.webp', 0, 0, 0], ['b.webp', 0, 1, 1], ['c.webp', 2, 0, 2], ['d.webp', 2, 1, 3]]
	);
});

test('the hex grid: cells, a place’s size, snapping to the nearest free spot', async () => {
	const { hexCellAt, hexCenter, hexArea, placeCells, snapPlace, HEX_CELL } = await import('../src/map.ts');
	const c = hexCellAt({ x: 100, y: 100 });
	const centre = hexCenter(c);
	assert.deepEqual(hexCellAt(centre), c, 'a centre lies in its own cell');
	assert.ok(Math.hypot(centre.x - 100, centre.y - 100) <= HEX_CELL / Math.sqrt(3) + 1e-9, 'within one cell radius');
	assert.equal(hexArea(c, 1).length, 7);
	assert.equal(hexArea(c, 2).length, 19);
	assert.equal(placeCells({ x: centre.x, y: centre.y, size: HEX_CELL / 2 }).length, 1, 'small: its own cell');
	assert.equal(placeCells({ x: centre.x, y: centre.y, size: 2 * HEX_CELL }).length, 7, 'a ring');
	assert.ok(placeCells({ x: centre.x, y: centre.y, size: 12 * HEX_CELL }).length > 100, 'the ore mountain takes many');
	const map = { width: 400, height: 300, locations: [{ id: 'a', x: centre.x, y: centre.y, toons: [], name: {}, size: HEX_CELL / 2 } as any] };
	const free = snapPlace(map, centre, HEX_CELL / 2);
	assert.notDeepEqual(hexCellAt(free), c, 'an occupied cell is not taken');
	assert.ok(Math.hypot(free.x - centre.x, free.y - centre.y) < HEX_CELL * 1.01, 'the next cell');
	assert.deepEqual(hexCellAt(snapPlace(map, centre, HEX_CELL / 2, 'a')), c, 'the moved place itself does not block');
	const big = snapPlace(map, centre, 4 * HEX_CELL);
	assert.ok(!placeCells({ ...big, size: 4 * HEX_CELL }).some((x) => x.q === c.q && x.r === c.r), 'a big place keeps its whole area free');
});

test('zoom percentages: 100 % is the place zoom (owner 2026-10-11)', async () => {
	const { zoomPercent, zoomFromPercent, PLACE_ZOOM, MAP_MAX_ZOOM, MAP_MIN_ZOOM } = await import('../src/map.ts');
	assert.equal(zoomPercent(PLACE_ZOOM), 100);
	assert.equal(zoomPercent(MAP_MAX_ZOOM), 400);
	assert.equal(zoomPercent(MAP_MIN_ZOOM), 3);
	assert.equal(zoomFromPercent(100), PLACE_ZOOM);
	assert.equal(zoomFromPercent(50), 4);
});

test('tiles and culling: the level for a zoom, the tiles on screen, the point index (BACKLOG 10.63)', async () => {
	const { tileLevel, visibleTiles, pointIndex, TILE_PX, checkWorldMap } = await import('../src/map.ts');
	assert.equal(tileLevel(1, 1, 5), 0);
	assert.equal(tileLevel(2, 1, 5), 1);
	assert.equal(tileLevel(3, 1, 5), 2, 'the next sharper level');
	assert.equal(tileLevel(8, 2, 5), 4);
	assert.equal(tileLevel(32, 2, 5), 5, 'capped at the deepest');
	assert.equal(tileLevel(0.25, 1, 5), 0);
	const world = { width: 2048, height: 1046 };
	assert.equal(visibleTiles({ x: 0, y: 0, width: 2048, height: 1046 }, 0, world).length, 8 * 5, 'level 0: the world in 256 px tiles');
	const deep = visibleTiles({ x: 1000, y: 300, width: 40, height: 20 }, 5, world);
	assert.ok(deep.every((t) => t.size === TILE_PX / 32 && t.z === 5));
	assert.equal(deep.length, 6 * 4, "x 125–130, y 37–40");
	assert.equal(visibleTiles({ x: -500, y: -500, width: 100, height: 100 }, 0, world).length, 0, 'off the world');
	const idx = pointIndex([{ x: 10, y: 10, id: 'a' }, { x: 100, y: 100, id: 'b' }, { x: 1000, y: 500, id: 'c' }]);
	assert.deepEqual(idx.query({ x: 0, y: 0, width: 150, height: 150 }).map((p) => p.id), ['a', 'b']);
	assert.deepEqual(idx.query({ x: 990, y: 490, width: 20, height: 20 }).map((p) => p.id), ['c']);
	const bad = (tiles: unknown) => checkWorldMap({ width: 10, height: 10, layers: [{ id: 't', image: 'a.svg', tiles } as any], locations: [], routes: [] }).map((p) => p.path.join('.'));
	assert.deepEqual(bad({ path: 'm/tiles', levels: 5 }), []);
	assert.deepEqual(bad({ path: 'm/tiles', levels: 12 }), ['layers.0.tiles']);
});

test('fog of war: the unlocked places, the layer field (owner 2026-10-12, MAP.md §8.9)', async () => {
	const { fogPlaces, checkWorldMap } = await import('../src/map.ts');
	const places = [{ id: 'a', x: 1, y: 1 }, { id: 'b', x: 2, y: 2 }, { id: 'c', x: 3, y: 3 }];
	assert.deepEqual(fogPlaces(places, { places: { b: { unlocked: 1 }, zz: { unlocked: 2 } } }).map((p) => p.id), ['b'], 'only unlocked places of the map');
	assert.deepEqual(fogPlaces(places, { places: {} }), [], 'a fresh reader: nothing painted');
	const bad = (fog: unknown) => checkWorldMap({ width: 10, height: 10, layers: [{ id: 'p', image: 'a.webp', fog } as any], locations: [], routes: [] }).map((p) => p.path.join('.'));
	assert.deepEqual(bad(undefined), []);
	assert.deepEqual(bad({ radius: 40 }), []);
	assert.deepEqual(bad({ radius: 0 }), ['layers.0.fog']);
	assert.deepEqual(bad(40), ['layers.0.fog'], 'an object, not a bare number');
});

test('resetting the map: the merge drops what came before the reset – also from a device that missed it (owner 2026-10-11)', async () => {
	const { mergeMapState, resetMapState } = await import('../src/map.ts');
	const old = { places: { apartment: { unlocked: 100, seen: 110 }, cathedral: { unlocked: 200 } }, routes: { season1: { revealed: 3 } }, positions: { catpope: { location: 'apartment', at: 120 } } };
	const reset = resetMapState(1000);
	// the phone still has the old map; the reset is newer
	const merged = mergeMapState(old as any, reset);
	assert.deepEqual(merged, { places: {}, routes: {}, positions: {}, resetAt: 1000 }, 'nothing from before the reset');
	assert.deepEqual(mergeMapState(reset, old as any), merged, 'either order');
	// read on after the reset: new unlocks stay – also one the phone made after the reset without knowing of it
	const phone = { ...old, places: { ...old.places, well: { unlocked: 1500 } } };
	const later = { ...reset, places: { apartment: { unlocked: 1200 } } };
	const m2 = mergeMapState(phone as any, later);
	assert.deepEqual(Object.keys(m2.places).sort(), ['apartment', 'well']);
	assert.equal(m2.places.apartment.unlocked, 1200, 'the new unlock, not the old one');
	assert.deepEqual(m2.routes, {}, 'the phone’s routes are from before the reset');
	assert.equal(m2.resetAt, 1000);
	// without any reset nothing changes (no resetAt key)
	assert.equal('resetAt' in mergeMapState(old as any, old as any), false);
});

test('unlockLocation: map open / toast or unset (owner 2026-10-11: chosen in the creator)', async () => {
	const { validateFile } = await import('../src/validate.ts');
	const doc = (params: unknown) => ({ groups: [{ label: 'g', events: [{ type: 'unlockLocation', params }] }] });
	const errs = (params: unknown) => validateFile('chapter', doc(params), 'de', {}).filter((p) => p.level === 'error' && p.path.join('.').includes('params')).map((p) => p.path.at(-1));
	assert.deepEqual(errs({ location: 'apartment' }), []);
	assert.deepEqual(errs({ location: 'apartment', map: 'open' }), []);
	assert.deepEqual(errs({ location: 'apartment', map: 'toast' }), []);
	assert.deepEqual(errs({ location: 'apartment', map: 'always' }), ['map']);
});

test('placeImageTarget: the place id, or <id>-plaster for its plaster model; an exact id wins', () => {
	const places = [{ id: 'old-church' }, { id: 'Cathedral' }, { id: 'kiln-plaster' }, { id: 'kiln' }];
	assert.deepEqual(placeImageTarget('old-church', places), { place: 'old-church', plaster: false });
	assert.deepEqual(placeImageTarget('cathedral', places), { place: 'Cathedral', plaster: false });
	assert.deepEqual(placeImageTarget('Old-Church-Plaster', places), { place: 'old-church', plaster: true });
	assert.deepEqual(placeImageTarget('old-church_plaster', places), { place: 'old-church', plaster: true });
	assert.deepEqual(placeImageTarget('kiln-plaster', places), { place: 'kiln-plaster', plaster: false });
	assert.equal(placeImageTarget('nowhere', places), null);
	assert.equal(placeImageTarget('nowhere-plaster', places), null);
});

test('route gates: a route stops at the place entry and goes on from its exit (BACKLOG 10.66 (4), variant A)', async () => {
	const { routeGeometry, placeGate, checkWorldMap } = await import('../src/map.ts');
	const places = [
		{ id: 'a', x: 0, y: 0, size: 10 },
		{ id: 'b', x: 100, y: 0, size: 10 },
		{ id: 'c', x: 200, y: 0, size: 10, gates: { entry: { x: 0, y: 3 } } }
	];
	const w = [{ location: 'a' }, { location: 'b' }, { location: 'c' }] as any;
	const g = routeGeometry(places, w);
	const r = (p: { x: number; y: number }) => ({ x: Math.round(p.x * 100) / 100, y: Math.round(p.y * 100) / 100 });
	// a → b, b → c: two lines; a starts at its exit, c ends at its own entry
	assert.equal(g.segments.length, 2);
	assert.deepEqual(r(g.segments[0][0]), { x: 2.5, y: 0 });
	assert.deepEqual(r(g.segments[0][1]), { x: 97.5, y: 0 });
	assert.deepEqual(r(g.segments[1][0]), { x: 102.5, y: 0 });
	assert.deepEqual(r(g.segments[1][1]), { x: 200, y: 3 });
	assert.deepEqual(g.gates.map((x) => `${x.place}:${x.which}`), ['a:exit', 'b:entry', 'b:exit', 'c:entry']);
	// a free point in between stays a point of the line; progress `to` ends at the reached place's entry
	assert.equal(routeGeometry(places, w, 0, 1).segments.length, 1);
	assert.deepEqual(placeGate(places[0], 'entry', null), { x: 0, y: 0 });
	const bad = { width: 10, height: 10, layers: [], routes: [], locations: [{ id: 'x', name: { de: 'X' }, x: 1, y: 1, toons: [], gates: { exit: { x: 'no' } } }] } as any;
	assert.ok(checkWorldMap(bad).some((p: any) => p.path.join('.').endsWith('gates.exit')));
});

test('fog of war along routes: between two unlocked places in a row, free points included (owner 2026-10-12)', async () => {
	const { fogRoutePoints, FOG_ROUTE_SCALE } = await import('../src/map.ts');
	const places = [
		{ id: 'a', x: 0, y: 0 },
		{ id: 'b', x: 16, y: 0 },
		{ id: 'c', x: 32, y: 0 }
	];
	const routes = [{ id: 'r', waypoints: [{ location: 'a' }, { id: 'p', x: 8, y: 8 }, { location: 'b' }, { location: 'c' }] }] as any;
	const open = (ids: string[]) => (id: string) => ids.includes(id);
	assert.deepEqual(fogRoutePoints(places, routes, open(['a', 'c'])), [], 'a and c are not in a row (b is locked)');
	const ab = fogRoutePoints(places, routes, open(['a', 'b']), 8);
	assert.ok(ab.length >= 3 && ab.every((p) => p.scale === FOG_ROUTE_SCALE));
	assert.ok(ab.some((p) => p.x === 8 && p.y === 8), 'through the free point');
	assert.ok(ab.every((p) => p.x <= 16), 'only the stretch a → b');
	assert.equal(new Set(ab.map((p) => p.id)).size, ab.length, 'stable, unique ids');
	assert.ok(fogRoutePoints(places, routes, open(['a', 'b', 'c'])).some((p) => p.x > 16), 'b → c too');
});
