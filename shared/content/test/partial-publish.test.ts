// Publishing part of a document (owner 2026-10-11/12): the map's places / routes / layers, the characters – each change a
// line of the publish checklist; the ticked ones go on top of the published version, the rest stays a draft.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { applyParts, diffParts, DOCUMENT_PARTS } from '../src/partial-publish.ts';

const F = DOCUMENT_PARTS.map;
const published = {
	width: 100,
	layers: [{ id: 'terrain', image: 'a.svg' }],
	locations: [
		{ id: 'apartment', name: { de: 'Apartment' }, x: 1 },
		{ id: 'tower', name: { de: 'Turm' }, x: 2 },
		{ id: 'church', name: { de: 'Kirche' }, x: 3 }
	],
	routes: [{ id: 'r1', waypoints: [] }]
};
const draft = {
	width: 120,
	layers: [{ id: 'terrain', image: 'a.svg' }],
	locations: [
		{ id: 'apartment', name: { de: 'Apartment' }, x: 9, image: { src: 'tower.webp' } },
		{ id: 'church', name: { de: 'Kirche' }, x: 3 },
		{ id: 'well', name: { de: 'Brunnen' }, x: 5 }
	],
	routes: [{ id: 'r1', waypoints: [] }]
};

test('the changes, one per entry, and the settings', () => {
	const changes = diffParts(published, draft, F);
	assert.deepEqual(
		changes.map((c) => `${c.line} ${c.kind}`),
		['locations:apartment changed', 'locations:well added', 'locations:tower removed', 'settings settings']
	);
	assert.deepEqual(diffParts(draft, draft, F), [], 'no difference');
});

test('only the ticked changes go on top of the published version', () => {
	const merged = applyParts(published, draft, F, new Set(['locations:apartment', 'locations:tower']));
	assert.equal(merged.width, 100, 'the settings were not ticked');
	assert.deepEqual(
		(merged.locations as { id: string; x: number }[]).map((p) => `${p.id}:${p.x}`),
		['apartment:9', 'church:3'],
		'the apartment changed, the tower removed, the well (not ticked) not added'
	);
	assert.deepEqual(merged.routes, published.routes);
	// everything ticked = the draft
	const all = new Set(diffParts(published, draft, F).map((c) => c.line));
	assert.deepEqual(applyParts(published, draft, F, all), draft);
	// nothing ticked = the published version
	assert.deepEqual(applyParts(published, draft, F, new Set()), { ...published, locations: published.locations });
});

test('a removal not ticked keeps the entry near where it was', () => {
	const merged = applyParts(published, draft, F, new Set(['locations:well']));
	assert.deepEqual((merged.locations as { id: string }[]).map((p) => p.id), ['apartment', 'tower', 'church', 'well']);
});
