// `npm test -w @scutoons/rig-designer`: the module document – its id rule and the content the designer writes and reads back.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isModuleId, moduleContent, moduleIdOf, readModuleContent, sameFiles } from '../src/store.ts';

const files = { 'lib.rs': 'fn a() {}' };
const descriptor = { abi: 2, name: 'walk', version: '0.1.0', params: [{ name: 'rate', min: 0, max: 16, default: 2 }], ports: [] };
const wasm = { hash: 'a'.repeat(64), bytes: 99, toolchain: 'rustc 1.90', url: 'https://cdn/x.wasm' };

test('module ids follow the server rule; module: keys give their id', () => {
	for (const ok of ['walk', 'a', 'my-mod_2', '0x']) assert.ok(isModuleId(ok), ok);
	for (const bad of ['', 'Walk', '-a', 'a b', 'x'.repeat(33), 'm:01']) assert.ok(!isModuleId(bad), bad);
	assert.equal(moduleIdOf('module:walk'), 'walk');
	assert.equal(moduleIdOf('module:Walk'), null);
	assert.equal(moduleIdOf('rig:character.catpope'), null);
});

test('the content keeps descriptor + wasm only for the sources they were read from, and without the url', () => {
	const outcome = { files, descriptor, wasm };
	assert.deepEqual(moduleContent('walk', 'walk', files, outcome), { id: 'walk', name: 'walk', files, descriptor, wasm: { hash: wasm.hash, bytes: 99, toolchain: 'rustc 1.90' } });
	assert.deepEqual(moduleContent('walk', 'walk', { 'lib.rs': 'fn b() {}' }, outcome), { id: 'walk', name: 'walk', files: { 'lib.rs': 'fn b() {}' } });
	assert.deepEqual(moduleContent('walk', 'walk', files, null), { id: 'walk', name: 'walk', files });
});

test('a stored document reads back; no sources = no module', () => {
	const m = readModuleContent({ id: 'walk', files, descriptor, wasm: { hash: wasm.hash, bytes: 99 } }, 'walk');
	assert.ok(m && m.name === 'walk' && m.files['lib.rs'] === 'fn a() {}' && m.descriptor?.name === 'walk' && m.wasm?.hash === wasm.hash);
	assert.equal(readModuleContent({ id: 'x' }, 'x'), null);
	assert.equal(readModuleContent({ files: {} }, 'x'), null);
	assert.equal(readModuleContent(null, 'x'), null);
	// descriptor and wasm come together
	assert.equal(readModuleContent({ files, descriptor }, 'x')?.descriptor, undefined);
});

test('a factory module reads as one; a derivative keeps its origin (#350)', () => {
	assert.equal(readModuleContent({ files, factory: true }, 'walk')?.factory, true);
	assert.equal(readModuleContent({ files, factory: 'yes' }, 'walk')?.factory, undefined);
	assert.equal(readModuleContent({ files, origin: 'walk' }, 'mine')?.origin, 'walk');
	assert.equal(readModuleContent({ files, origin: 'Not An Id' }, 'mine')?.origin, undefined);
	assert.deepEqual(moduleContent('mine', 'mine', files, null, 'walk'), { id: 'mine', name: 'mine', origin: 'walk', files });
	assert.deepEqual(moduleContent('mine', 'mine', files, null), { id: 'mine', name: 'mine', files });
});

test('sameFiles compares names and texts', () => {
	assert.ok(sameFiles(files, { 'lib.rs': 'fn a() {}' }));
	assert.ok(!sameFiles(files, { 'lib.rs': 'fn a() {}', 'b.rs': '' }));
	assert.ok(!sameFiles(files, { 'lib.rs': 'x' }));
});
