// Documents made of many documents (DRAFT, agents/MAP-DOCUMENTS.md): the world map (map + place:<id> / route:<id>) and
// the character registry (characters + character:<id>) – composed into one document for everything that reads them,
// split again for saving, removals as tombstones (or deleted when never published – the caller), problems per document.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { acceptedWarningKey } from '../src/accepted.ts';
import { entities, parseEntityKey, entityOf } from '../src/entities.ts';
import {
	applyCollectionChanges,
	collectionChanges,
	collectionDocsFromEntities,
	collectionDocsOf,
	collectionKinds,
	splitCollection,
	stableJson,
	takenCollectionIds,
	type CollectionDocs
} from '../src/collection-docs.ts';
import { MAP_COLLECTION, checkMapDocs, composeWorldMap, mapDocOfPath } from '../src/map-docs.ts';
import { CHARACTERS_COLLECTION, checkCharacterDocs, composeCharacterRegistry } from '../src/character-docs.ts';
import type { WorldMap } from '../src/map.ts';
import type { CharacterRegistry } from '../src/characters.ts';

const read = (path: string) => JSON.parse(readFileSync(new URL(`../../../server/tests/fixtures/content/scutoons/_/${path}`, import.meta.url), 'utf8'));
const fixture = read('map.json') as WorldMap;
const registry = read('characters.json') as CharacterRegistry;

const mapDocs = (map: WorldMap): CollectionDocs => splitCollection(MAP_COLLECTION, map as unknown as Record<string, any>);

test('keys: place:<id>, route:<id>, character:<id> parse, other shapes do not', () => {
	assert.deepEqual(parseEntityKey('place:cathedral'), { kind: 'place', id: 'cathedral' });
	assert.deepEqual(parseEntityKey('route:season1'), { kind: 'route', id: 'season1' });
	assert.deepEqual(parseEntityKey('character:catpope'), { kind: 'character', id: 'catpope' });
	assert.equal(entities.place('well'), 'place:well');
	assert.equal(entities.character('boy'), 'character:boy');
	assert.equal(entityOf('place:well'), 'place:well');
	for (const bad of ['place', 'place:', 'place:Cathedral', 'place:a:b', 'route:-x', 'place:catpope:de', 'character:']) assert.equal(parseEntityKey(bad), null, bad);
	assert.deepEqual(parseEntityKey('map'), { kind: 'map' });
	assert.deepEqual(collectionKinds(MAP_COLLECTION), ['map', 'place', 'route', 'region']);
	assert.deepEqual(parseEntityKey('region:alps'), { kind: 'region', id: 'alps' });
	assert.equal(entities.region('alps'), 'region:alps');
});

test('split → compose gives the same map (the fixture)', () => {
	const docs = mapDocs(fixture);
	assert.equal(stableJson(composeWorldMap(docs)), stableJson(fixture));
	for (const p of docs.parts.place) assert.equal('id' in p.doc, false, 'the key holds the id, not the document');
	const keys = Object.keys(collectionDocsOf(MAP_COLLECTION, fixture as unknown as Record<string, any>));
	assert.equal(keys[0], 'map');
	assert.equal(keys.length, 1 + fixture.locations.length + fixture.routes.length);
});

test('from stored entities (creation order), one document swapped in', () => {
	const list = Object.entries(collectionDocsOf(MAP_COLLECTION, fixture as unknown as Record<string, any>)).map(([key, content]) => ({ key, content }));
	const docs = collectionDocsFromEntities(MAP_COLLECTION, [...list, { key: 'characters', content: {} }, { key: 'place:nothing', content: null }]);
	assert.equal(stableJson(composeWorldMap(docs)), stableJson(fixture));
	const first = fixture.locations[0].id;
	const swapped = composeWorldMap(collectionDocsFromEntities(MAP_COLLECTION, list, { key: `place:${first}`, doc: { ...docs.parts.place[0].doc, x: 7 } }));
	assert.equal(swapped.locations[0].x, 7);
});

test('a removed place (tombstone) is not on the map, its id stays taken', () => {
	const docs = mapDocs(fixture);
	const id = docs.parts.place[0].id;
	docs.parts.place[0] = { id, doc: { ...docs.parts.place[0].doc, removed: true } };
	const map = composeWorldMap(docs);
	assert.equal(map.locations.some((p) => p.id === id), false);
	assert.ok(takenCollectionIds(docs, 'place').includes(id));
});

test('changes: added, changed, removed (tombstone), restored, settings – and applying them (deletes too)', () => {
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
	const changes = (stored: CollectionDocs, next: WorldMap) => collectionChanges(MAP_COLLECTION, stored, next as unknown as Record<string, any>);
	const stored = mapDocs(map);
	assert.deepEqual(changes(stored, map), [], 'nothing changed');
	const reordered = structuredClone(map);
	reordered.locations[0] = { y: 1, x: 1, toons: ['t'], name: { de: 'A' }, id: 'a' };
	assert.deepEqual(changes(stored, reordered), [], 'key order does not count');

	const next = structuredClone(map);
	next.width = 120;
	next.locations[0].x = 9;
	next.locations.push({ id: 'c', name: { de: 'C' }, x: 3, y: 3, toons: ['t'] });
	next.routes = [];
	const list = changes(stored, next);
	assert.deepEqual(
		list.map((c) => [c.key, c.kind]),
		[['map', 'settings'], ['place:a', 'changed'], ['place:c', 'added'], ['route:r', 'removed']]
	);
	assert.equal(list.find((c) => c.key === 'route:r')!.doc.removed, true);
	assert.deepEqual(list.find((c) => c.key === 'route:r')!.doc.waypoints, map.routes[0].waypoints, 'the tombstone keeps the last document');

	const after = applyCollectionChanges(MAP_COLLECTION, stored, list);
	assert.equal(stableJson(composeWorldMap(after)), stableJson(next));
	assert.deepEqual(changes(after, next), [], 'saved');
	const back = changes(after, map).find((c) => c.key === 'route:r');
	assert.equal(back?.kind, 'restored');
	assert.equal(back!.doc.removed, undefined);

	// a never-published one is deleted instead of a tombstone (the caller decides): gone from the documents, id free
	const deleted = applyCollectionChanges(MAP_COLLECTION, stored, list.filter((c) => c.key !== 'route:r'), ['route:r']);
	assert.deepEqual(takenCollectionIds(deleted, 'route'), []);
	assert.deepEqual(changes(deleted, next), []);
});

test('accepted warnings travel with their entry', () => {
	const map = structuredClone(fixture);
	const i = 1;
	const id = map.locations[i].id;
	const keyOfPlace = acceptedWarningKey({ path: ['locations', i, 'hidden'], message: 'x' }, map);
	const keyOfWhole = acceptedWarningKey({ path: [], message: 'whole map' }, map);
	map.accepted_warnings = [keyOfPlace, keyOfWhole];
	assert.ok(keyOfPlace.startsWith(`locations.@${id}.hidden#`));
	const docs = mapDocs(map);
	assert.deepEqual(docs.settings!.accepted_warnings, [keyOfWhole]);
	assert.deepEqual(docs.parts.place[i].doc.accepted_warnings, [keyOfPlace.replace(`locations.@${id}.`, '')]);
	assert.deepEqual(composeWorldMap(docs).accepted_warnings?.sort(), [keyOfPlace, keyOfWhole].sort());
});

test('map problems are addressed to their document', () => {
	const docs = mapDocs(fixture);
	const places = docs.parts.place;
	places[2] = { id: places[2].id, doc: { ...places[2].doc, x: -5 } };
	const removed = fixture.routes[0].waypoints.find((w) => 'location' in w) as { location: string };
	const at = places.findIndex((p) => p.id === removed.location);
	places[at] = { id: removed.location, doc: { ...places[at].doc, removed: true } };
	const problems = checkMapDocs(docs).filter((p) => p.level === 'error');
	assert.ok(problems.some((p) => p.key === entities.place(places[2].id) && p.docPath[0] === 'x'), 'outside the map → the place');
	assert.ok(problems.some((p) => p.key === entities.route(fixture.routes[0].id) && p.docPath[0] === 'waypoints'), 'a route through a removed place → the route');
	assert.deepEqual(mapDocOfPath(fixture, ['width']), { key: 'map', path: ['width'] });
});

test('the character registry: split → compose, one character changed, problems per character', () => {
	const docs = splitCollection(CHARACTERS_COLLECTION, registry as unknown as Record<string, any>);
	assert.deepEqual(docs.settings, {}, 'the registry has no fields of its own today');
	assert.equal(docs.parts.character.length, registry.characters.length);
	assert.equal(stableJson(composeCharacterRegistry(docs)), stableJson(registry));
	const next = structuredClone(registry);
	next.characters[0].color = '#123456';
	const list = collectionChanges(CHARACTERS_COLLECTION, docs, next as unknown as Record<string, any>);
	assert.deepEqual(list.map((c) => [c.key, c.kind]), [[`character:${registry.characters[0].id}`, 'changed']]);
	const broken = structuredClone(docs);
	broken.parts.character[1] = { id: broken.parts.character[1].id, doc: { ...broken.parts.character[1].doc, testo: 500 } };
	const problems = checkCharacterDocs(broken, { languages: ['de', 'en'] }).filter((p) => p.level === 'error');
	assert.ok(problems.length && problems.every((p) => p.key === `character:${registry.characters[1].id}`), JSON.stringify(problems.slice(0, 2)));
});
