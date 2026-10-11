// The world map as many documents (DRAFT, agents/MAP-DOCUMENTS.md): place:<id> / route:<id> / map, composed into one
// WorldMap for everything that reads it, split again for saving, removals as tombstones, problems per document.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { acceptedWarningKey } from '../src/accepted.ts';
import { entities, parseEntityKey, entityOf } from '../src/entities.ts';
import { applyMapDocChanges, checkMapDocs, composeWorldMap, mapDocChanges, mapDocOfPath, splitWorldMap, stableJson, takenMapIds, worldMapDocs, type MapDocs } from '../src/map-docs.ts';
import type { WorldMap } from '../src/map.ts';

const fixture = JSON.parse(readFileSync(new URL('../../../server/tests/fixtures/content/scutoons/_/map.json', import.meta.url), 'utf8')) as WorldMap;

const docsOf = (map: WorldMap): MapDocs => {
	const s = splitWorldMap(map);
	return { settings: s.settings, places: s.places, routes: s.routes };
};

test('keys: place:<id> and route:<id> parse, other shapes do not', () => {
	assert.deepEqual(parseEntityKey('place:cathedral'), { kind: 'place', id: 'cathedral' });
	assert.deepEqual(parseEntityKey('route:season1'), { kind: 'route', id: 'season1' });
	assert.equal(entities.place('well'), 'place:well');
	assert.equal(entityOf('place:well'), 'place:well');
	for (const bad of ['place', 'place:', 'place:Cathedral', 'place:a:b', 'route:-x', 'place:catpope:de']) assert.equal(parseEntityKey(bad), null, bad);
	assert.deepEqual(parseEntityKey('map'), { kind: 'map' });
});

test('split → compose gives the same map (the fixture)', () => {
	const docs = docsOf(fixture);
	assert.equal(stableJson(composeWorldMap(docs)), stableJson(fixture));
	for (const p of docs.places) assert.equal('id' in p.doc, false, 'the key holds the id, not the document');
	const keys = Object.keys(worldMapDocs(fixture));
	assert.equal(keys[0], 'map');
	assert.equal(keys.length, 1 + fixture.locations.length + fixture.routes.length);
});

test('a removed place (tombstone) is not on the map, its id stays taken', () => {
	const docs = docsOf(fixture);
	const id = docs.places[0].id;
	docs.places[0] = { id, doc: { ...docs.places[0].doc, removed: true } };
	const map = composeWorldMap(docs);
	assert.equal(map.locations.some((p) => p.id === id), false);
	assert.ok(takenMapIds(docs, 'place').includes(id));
});

test('changes: added, changed, removed (tombstone), restored, settings – and applying them', () => {
	const map: WorldMap = {
		width: 100,
		height: 50,
		layers: [],
		locations: [
			{ id: 'a', name: { de: 'A' }, x: 1, y: 1, toons: ['t'] },
			{ id: 'b', name: { de: 'B' }, x: 2, y: 2, toons: ['t'] }
		],
		routes: [{ id: 'r', name: { de: 'R' }, toons: ['t'], waypoints: [{ location: 'a' }, { location: 'b' }] }]
	};
	const stored = docsOf(map);
	assert.deepEqual(mapDocChanges(stored, map), [], 'nothing changed');
	// key order does not count
	const reordered = structuredClone(map);
	reordered.locations[0] = { y: 1, x: 1, toons: ['t'], name: { de: 'A' }, id: 'a' };
	assert.deepEqual(mapDocChanges(stored, reordered), []);

	const next = structuredClone(map);
	next.width = 120;
	next.locations[0].x = 9;
	next.locations.push({ id: 'c', name: { de: 'C' }, x: 3, y: 3, toons: ['t'] });
	next.routes = [];
	const changes = mapDocChanges(stored, next);
	assert.deepEqual(
		changes.map((c) => [c.key, c.kind]),
		[['map', 'settings'], ['place:a', 'changed'], ['place:c', 'added'], ['route:r', 'removed']]
	);
	assert.equal((changes.find((c) => c.key === 'route:r')!.doc as { removed?: boolean }).removed, true);
	assert.deepEqual((changes.find((c) => c.key === 'route:r')!.doc as { waypoints: unknown }).waypoints, map.routes[0].waypoints, 'the tombstone keeps the last document');

	const after = applyMapDocChanges(stored, changes);
	assert.equal(stableJson(composeWorldMap(after)), stableJson(next));
	assert.deepEqual(mapDocChanges(after, next), [], 'saved');
	// the route comes back
	const back = mapDocChanges(after, map).find((c) => c.key === 'route:r');
	assert.equal(back?.kind, 'restored');
	assert.equal((back!.doc as { removed?: boolean }).removed, undefined);
});

test('accepted warnings travel with their place', () => {
	const map = structuredClone(fixture);
	const i = 1;
	const id = map.locations[i].id;
	const keyOfPlace = acceptedWarningKey({ path: ['locations', i, 'hidden'], message: 'x' }, map);
	const keyOfWhole = acceptedWarningKey({ path: [], message: 'whole map' }, map);
	map.accepted_warnings = [keyOfPlace, keyOfWhole];
	assert.ok(keyOfPlace.startsWith(`locations.@${id}.hidden#`));
	const docs = docsOf(map);
	assert.deepEqual(docs.settings!.accepted_warnings, [keyOfWhole]);
	assert.deepEqual(docs.places[i].doc.accepted_warnings, [keyOfPlace.replace(`locations.@${id}.`, '')]);
	assert.deepEqual(composeWorldMap(docs).accepted_warnings?.sort(), [keyOfPlace, keyOfWhole].sort());
});

test('problems are addressed to their document', () => {
	const docs = docsOf(fixture);
	docs.places[2] = { id: docs.places[2].id, doc: { ...docs.places[2].doc, x: -5 } };
	const removed = fixture.routes[0].waypoints.find((w) => 'location' in w) as { location: string };
	const at = docs.places.findIndex((p) => p.id === removed.location);
	docs.places[at] = { id: removed.location, doc: { ...docs.places[at].doc, removed: true } };
	const problems = checkMapDocs(docs).filter((p) => p.level === 'error');
	assert.ok(problems.some((p) => p.key === entities.place(docs.places[2].id) && p.docPath[0] === 'x'), 'outside the map → the place');
	assert.ok(problems.some((p) => p.key === entities.route(fixture.routes[0].id) && p.docPath[0] === 'waypoints'), 'a route through a removed place → the route');
	assert.deepEqual(mapDocOfPath(fixture, ['width']), { key: 'map', path: ['width'] });
});
