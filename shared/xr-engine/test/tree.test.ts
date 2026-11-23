// OSCT's tree edits through the package's `./tree` entry (src/tree.ts): what the XR workspace's Tree and Inspector do.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { xrAddNode, xrAddSurface, xrDefaultAddPlace, xrDuplicateNode, xrFilterTreeAdd, xrGroupNode, xrTreeAddItems, xrIndentNode, xrNewNode, xrNodeFileTypes, xrNodeTypes, xrOutdentNode, xrRemoveNode, xrShiftNode, type XrNode } from '../src/tree.ts';

const root = (): XrNode => ({ type: 'group', name: 'root', children: [{ type: 'light', name: 'a' }, { type: 'model', name: 'b', assets: [{ id: 'm', assetType: 'glb', src: '/m.glb' }] }] });

test('the node types and the files they take', () => {
	assert.deepEqual(xrNodeTypes().slice(0, 6), ['group', 'model', 'video', 'image', 'text', 'light']);
	assert.deepEqual(xrNodeFileTypes('model'), ['glb', 'gltf']);
	assert.equal(xrNodeFileTypes('group'), undefined);
	assert.deepEqual(xrNewNode('text'), { type: 'text', assets: [], text: '{{entry.title}}', color: '#ffffff' });
	assert.deepEqual(xrNewNode('image', { id: 'i', assetType: 'image', src: '/i.webp' }).assets, [{ id: 'i', assetType: 'image', src: '/i.webp' }]);
});

test('add: inside, next to, the first node is the root, next to the root wraps it', () => {
	const inside = xrAddNode(root(), [], xrNewNode('group'), 'inside');
	assert.deepEqual(inside.path, [2]);
	assert.equal(inside.tree.children?.length, 3);
	const next = xrAddNode(root(), [0], xrNewNode('light'), 'next');
	assert.deepEqual(next.path, [1]);
	assert.equal(next.tree.children?.[2]?.name, 'b');
	assert.deepEqual(xrAddNode(null, [], xrNewNode('group'), 'inside'), { tree: { type: 'group', assets: [] }, path: [] });
	const wrapped = xrAddNode(root(), [], xrNewNode('light'), 'next');
	assert.deepEqual(wrapped.path, [1]);
	assert.equal(wrapped.tree.children?.[0]?.name, 'root');
	assert.equal(xrDefaultAddPlace(root().children?.[1]), 'next');
	assert.equal(xrDefaultAddPlace(root()), 'inside');
});

test('remove, shift, indent, outdent; the tree passed in stays as it is', () => {
	const before = root();
	assert.deepEqual(xrRemoveNode(before, [0]).children?.map((c) => c.name), ['b']);
	assert.equal(before.children?.length, 2);
	assert.deepEqual(xrShiftNode(before, [1], -1)?.tree.children?.map((c) => c.name), ['b', 'a']);
	assert.equal(xrShiftNode(before, [0], -1), null);
	const indented = xrIndentNode(before, [1]);
	assert.deepEqual(indented?.path, [0, 0]);
	assert.equal(indented?.tree.children?.[0]?.children?.[0]?.name, 'b');
	assert.deepEqual(xrOutdentNode(indented!.tree, [0, 0])?.path, [1]);
});

test('the footer: duplicate with free names and no ids, group in place', () => {
	const named: XrNode = { type: 'group', name: 'root', children: [{ type: 'model', name: 'door', id: 'd1', children: [{ type: 'light', name: 'lamp' }] }] };
	const copy = xrDuplicateNode(named, [0]);
	assert.deepEqual(copy?.path, [1]);
	assert.equal(copy?.tree.children?.[1]?.name, 'door 2');
	assert.equal(copy?.tree.children?.[1]?.children?.[0]?.name, 'lamp 2');
	assert.equal('id' in (copy?.tree.children?.[1] ?? {}), false);
	const grouped = xrGroupNode(named, [0]);
	assert.deepEqual(grouped?.path, [0]);
	assert.equal(grouped?.tree.children?.[0]?.type, 'group');
	assert.equal(grouped?.tree.children?.[0]?.children?.[0]?.name, 'door');
});

test('the + menu: surfaces always, nodes only with a target, target and entry as the context says; a typed filter', () => {
	const base = { hasEntity: true, node: null, builtIn: ['group', 'model'], plugins: ['sparkle'], takesFile: (t: string) => t === 'model', canAddTarget: false, canAddEntry: false };
	const none = xrTreeAddItems({ ...base, target: null });
	assert.deepEqual(none.filter((i) => i.group === 'scene').map((i) => i.key), ['surface:double', 'surface:single', 'surface:wall', 'surface:custom', 'target', 'entry']);
	assert.ok(none.filter((i) => i.add.add === 'node').every((i) => i.disabled));
	const onTarget = xrTreeAddItems({ ...base, target: 'cat' });
	assert.ok(onTarget.filter((i) => i.add.add === 'node').every((i) => !i.disabled));
	assert.equal(onTarget.find((i) => i.key === 'node:sparkle')?.group, 'plugin nodes');
	assert.deepEqual(xrFilterTreeAdd(onTarget, 'wall').map((i) => i.key), ['surface:wall']);
	const { surfaces, added } = xrAddSurface('single', [{ id: 'page', widthMm: 230, heightMm: 320, place: { x: 0, y: 0, z: 0 } }]);
	assert.equal(surfaces.length, 2);
	assert.notEqual(added.id, 'page');
});
