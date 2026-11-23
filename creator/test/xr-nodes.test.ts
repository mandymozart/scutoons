import assert from 'node:assert/strict';
import { test } from 'node:test';
import { entityOf, entitySlot, experienceFiles, fileName, inspection, targetDocPath, targetPlace, nodeAtPath, nodeDocPath, sameRef, sceneRows, targetsSharing } from '../src/lib/editor/xr-nodes.ts';

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

test("the Inspector: a node as editable rows, a list of numbers one row per axis, a print's place editable, the rest read only", () => {
	const model = inspection(configuration, { target: 'catpope', path: [1] });
	assert.equal(model?.kind, 'model');
	assert.deepEqual(model?.groups.map((g) => g.label), ['node', 'placement', 'files', 'reveal']);
	assert.deepEqual(model?.groups[1].fields, [
		{ label: 'scale x', kind: 'number', value: 1.4, at: ['params', 'scale', 0] },
		{ label: 'scale y', kind: 'number', value: 1.4, at: ['params', 'scale', 1] },
		{ label: 'scale z', kind: 'number', value: 0.84, at: ['params', 'scale', 2] }
	]);
	assert.deepEqual(
		model?.groups[3].fields.map((f) => [f.label, f.kind, f.at.join('.')]),
		[
			['look', 'text', 'components.0.look'],
			['duration', 'number', 'components.0.duration']
		]
	);
	assert.equal(model?.groups[2].fields[0].kind, 'readonly');
	const light = inspection(configuration, { target: 'catpope', path: [0] });
	assert.deepEqual(light?.groups.map((g) => g.label), ['node', 'placement', 'light']);
	assert.deepEqual(light?.groups[0].fields[1], { label: 'name', kind: 'text', value: 'key', at: ['name'] });
	const print = inspection(configuration, { target: 'catpope', path: null });
	assert.equal(print?.kind, 'target');
	assert.deepEqual(
		print?.groups[0].fields.filter((f) => f.kind !== 'readonly').map((f) => `${f.label}:${f.at.join('.')}`),
		['x:place.x', 'y:place.y', 'width:place.width']
	);
	assert.ok(print?.groups[1].fields.every((f) => f.kind === 'readonly'));
	assert.equal(inspection(configuration, { target: 'nobody', path: [] }), null);
	assert.equal(inspection(configuration, null), null);
});

test('a node lives in the shared entity or in the target own; every target of a shared entity changes with it', () => {
	assert.deepEqual(nodeDocPath(configuration, { target: 'catpope', path: [1] }), ['configuration', 'entities', 'overlay', 'children', 1]);
	assert.deepEqual(nodeDocPath(configuration, { target: 'note', path: [] }), ['configuration', 'entries', 1, 'target', 'entity']);
	assert.equal(nodeDocPath(configuration, { target: 'catpope', path: [9] }), null);
	assert.equal(nodeDocPath(configuration, { target: 'catpope', path: null }), null);
	const two = { ...configuration, entries: [...configuration.entries, { id: 'b', scene: 'back', target: { id: 'second', entity: { ref: 'overlay' } } }] };
	assert.deepEqual(targetsSharing(two, 'catpope'), ['catpope', 'second']);
	assert.deepEqual(targetsSharing(two, 'note'), ['note']);
	assert.deepEqual(targetsSharing(two, 'nobody'), []);
});

test("a target's whole tree: the shared entity or its own, its own field cleared when its content goes", () => {
	const shared = entitySlot(configuration, 'catpope');
	assert.deepEqual(shared?.at, ['configuration', 'entities', 'overlay']);
	assert.deepEqual(shared?.own, ['configuration', 'entries', 0, 'target', 'entity']);
	assert.equal(shared?.shared, 'overlay');
	assert.equal(shared?.tree?.name, 'overlay');
	const own = entitySlot(configuration, 'note');
	assert.deepEqual(own?.at, ['configuration', 'entries', 1, 'target', 'entity']);
	assert.equal(own?.shared, null);
	const empty = entitySlot({ entries: [{ id: 'e', target: { id: 't' } }] }, 't');
	assert.deepEqual(empty?.at, ['configuration', 'entries', 0, 'target', 'entity']);
	assert.equal(empty?.tree, null);
	assert.equal(entitySlot(configuration, 'missing'), null);
});

test('the files a new part can take: each once, from own and shared entities, then the targets pictures', () => {
	const twice = { ...configuration, entries: [...configuration.entries, { id: 'x', target: { id: 'x', entity: { type: 'model', assets: [{ id: 'q', assetType: 'glb', src: '/assets/content/p0.glb' }, { id: 'v', assetType: 'video', src: '/assets/content/v.mp4' }] } } }] };
	assert.deepEqual(experienceFiles(twice).map((f) => `${f.assetType}:${fileName(f.src)}`), ['glb:p0.glb', 'video:v.mp4', 'image:p.webp']);
	assert.deepEqual(experienceFiles({}), []);
});

test('a target in the document and its place', () => {
	assert.deepEqual(targetDocPath(configuration, 'note'), ['configuration', 'entries', 1, 'target']);
	assert.equal(targetDocPath(configuration, 'missing'), null);
	assert.deepEqual(targetPlace(configuration, 'catpope'), { x: 10, y: 20, width: 200 });
	assert.equal(targetPlace(configuration, 'note'), null);
});
