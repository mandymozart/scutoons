import assert from 'node:assert/strict';
import { test } from 'node:test';
import { entityOf, inspection, nodeAtPath, sameRef, sceneRows } from '../src/lib/editor/xr-nodes.ts';

const configuration = {
	scenes: [{ id: 'poster', surfaces: [{ id: 'a3', label: 'A3' }] }, { id: 'back', surfaces: [] }],
	entries: [
		{ id: 'catpope', title: 'Cat Pope', scene: 'poster', target: { id: 'catpope', imageSrc: '/assets/content/p.webp', place: { x: 10, y: 20, width: 200 }, entity: { ref: 'overlay' } } },
		{ id: 'loose', scene: 'back', target: { id: 'note', entity: { type: 'text', name: 'hello' } } }
	],
	entities: {
		overlay: {
			type: 'group',
			name: 'overlay',
			components: [{ type: 'toon' }],
			children: [
				{ type: 'light', name: 'key', params: { position: [-0.3, 0.4, 0.9] }, light: { kind: 'directional', intensity: 10 } },
				{ type: 'model', assets: [{ id: 'p0', assetType: 'glb', src: '/assets/content/p0.glb' }], params: { scale: [1.4, 1.4, 0.84] }, components: [{ type: 'reveal', look: 'inflateDissolve', duration: 2600 }] }
			]
		}
	}
};

test('a target refers to a shared entity or holds its own', () => {
	assert.equal(entityOf(configuration, configuration.entries[0].target)?.name, 'overlay');
	assert.equal(entityOf(configuration, configuration.entries[1].target)?.name, 'hello');
	assert.equal(entityOf(configuration, { entity: { ref: 'missing' } }), null);
	assert.equal(nodeAtPath(entityOf(configuration, configuration.entries[0].target), [1])?.type, 'model');
	assert.equal(nodeAtPath(entityOf(configuration, configuration.entries[0].target), [5]), null);
});

test('the scene as rows: surface, entry, target, the node tree; a folded node hides its children', () => {
	const rows = sceneRows(configuration, 'poster');
	assert.deepEqual(
		rows.map((r) => `${r.depth}:${r.kind}:${r.label}`),
		['0:surface:A3', '1:entry:Cat Pope', '2:target:catpope', '3:node:overlay', '4:node:key', '4:node:model']
	);
	assert.equal(sceneRows(configuration, 'poster', new Set(['catpope:'])).length, 4);
	assert.deepEqual(sceneRows(configuration, 'back').map((r) => r.kind), ['entry', 'target', 'node']);
	assert.deepEqual(sceneRows(configuration, 'nowhere'), []);
});

test('the same node or print', () => {
	assert.ok(sameRef({ target: 't', path: [1, 0] }, { target: 't', path: [1, 0] }));
	assert.ok(sameRef({ target: 't', path: null }, { target: 't', path: null }));
	assert.ok(!sameRef({ target: 't', path: [] }, { target: 't', path: null }));
	assert.ok(!sameRef({ target: 't', path: [1] }, null));
});

test('the Inspector shows a node as stored, and a print with its entry', () => {
	const model = inspection(configuration, { target: 'catpope', path: [1] });
	assert.equal(model?.kind, 'model');
	assert.deepEqual(model?.groups.map((g) => g.label), ['node', 'placement', 'files', 'reveal']);
	assert.deepEqual(model?.groups[1].fields, [{ label: 'scale', value: '1.4  1.4  0.84' }]);
	assert.deepEqual(inspection(configuration, { target: 'catpope', path: [0] })?.groups.map((g) => g.label), ['node', 'placement', 'light']);
	const print = inspection(configuration, { target: 'catpope', path: null });
	assert.equal(print?.kind, 'target');
	assert.ok(print?.groups[0].fields.some((f) => f.label === 'entity' && f.value === 'overlay'));
	assert.equal(inspection(configuration, { target: 'nobody', path: [] }), null);
	assert.equal(inspection(configuration, null), null);
});
