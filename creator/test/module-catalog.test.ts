// `npm test -w scutoons-creator`: the Sound workspace's module list, New module and Derive (lib/editor/module-catalog.ts, #350).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { factoryChoices, firstDraft, idProblem, readCatalog, templateOf } from '../src/lib/editor/module-catalog.ts';

const files = { 'lib.rs': '// walk' };
const descriptor = { abi: 2, name: 'walk', version: '0.1.0', params: [], ports: [] };
const wasm = { hash: 'a'.repeat(64), bytes: 9 };

test('the list: live module ids sorted, the factory modules marked with what they hold', () => {
	const catalog = readCatalog([
		{ id: 'walk', removed: false, content: { id: 'walk', factory: true, files } },
		{ id: 'mine', removed: false, content: { id: 'mine', origin: 'walk', files } },
		{ id: 'gone', removed: true, content: { id: 'gone', factory: true, files } },
		{ id: 'Bad Id', removed: false, content: { files } },
		{ id: 'analog', removed: false, content: { id: 'analog', factory: true, files, descriptor, wasm } }
	]);
	assert.deepEqual(catalog.ids, ['analog', 'mine', 'walk']);
	assert.deepEqual(Object.keys(catalog.factory).sort(), ['analog', 'walk']);
	assert.deepEqual(catalog.factory.analog.descriptor, descriptor);
	assert.deepEqual(factoryChoices(catalog), [
		{ id: 'analog', label: 'module:analog' },
		{ id: 'walk', label: 'module:walk' }
	]);
});

test('a new module copies its template and names it as its origin; without one it is the starter', () => {
	const from = { id: 'analog', module: { name: 'analog', files, descriptor, wasm, factory: true } };
	assert.deepEqual(firstDraft('my_analog', from, { 'lib.rs': '// starter' }), { id: 'my_analog', name: 'my_analog', origin: 'analog', files, descriptor, wasm: { hash: wasm.hash, bytes: 9 } });
	assert.deepEqual(firstDraft('fresh', null, { 'lib.rs': '// starter' }), { id: 'fresh', name: 'fresh', files: { 'lib.rs': '// starter' } });
	// the factory flag is never copied: a derivative is the member's own
	assert.equal('factory' in firstDraft('x', from, {}), false);
	// Derive: the open module, its compile only when there is one
	assert.deepEqual(templateOf('walk', 'walk', files, null), { id: 'walk', module: { name: 'walk', files } });
	assert.deepEqual(templateOf('walk', 'walk', files, { files, descriptor, wasm }).module.wasm, wasm);
});

test('a new id: valid and not taken', () => {
	assert.equal(idProblem('my_walk', ['walk']), '');
	assert.equal(idProblem('walk', ['walk']), 'taken');
	assert.equal(idProblem('My Walk', []), 'invalid');
});
