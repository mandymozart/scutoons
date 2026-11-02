import assert from 'node:assert/strict';
import { test } from 'node:test';
import { pickKinds, type KindCatalog } from '../src/kinds.ts';

const catalog: KindCatalog = {
	catalog_version: 1,
	kinds: [
		{ name: 'osc', domain: 'audio', doc: 'A band-limited oscillator', ports: [{ name: 'freq' }] },
		{ name: 'lfo', domain: 'control', doc: 'A low-frequency oscillator', ports: [] },
		{ name: 'module:walk', domain: 'audio', doc: 'The published module walk.' }
	]
};

test('pickKinds: no filter = a short index without ports', () => {
	const r = pickKinds(catalog);
	assert.deepEqual(r.kinds.map((k) => k.name), ['osc', 'lfo', 'module:walk']);
	assert.equal('ports' in r.kinds[0], false);
});

test('pickKinds: names give the full descriptors, q searches all fields', () => {
	assert.deepEqual(pickKinds(catalog, ['osc']).kinds, [catalog.kinds[0]]);
	assert.deepEqual(pickKinds(catalog, undefined, 'low-frequency').kinds.map((k) => k.name), ['lfo']);
	assert.deepEqual(pickKinds(catalog, undefined, 'freq').kinds.map((k) => k.name), ['osc', 'lfo']);
});
