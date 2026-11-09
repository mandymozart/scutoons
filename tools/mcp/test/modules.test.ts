import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ApiError } from '@scutoons/api';
import { compiled, compileRefused, compileRequest, MODULE_ID } from '../src/modules.ts';
import { creatorHref } from '../src/services.ts';

test('the request compiles the newest draft of the module', () => {
	assert.deepEqual(compileRequest('my_walk'), { factory: 'my_walk' });
});

test('a module id is what the server accepts', () => {
	assert.ok(MODULE_ID.test('my_walk-2'));
	assert.ok(!MODULE_ID.test('My Walk'));
	assert.ok(!MODULE_ID.test('a'.repeat(33)));
});

test('a compile answers with the draft that holds the result', () => {
	const r = compiled({ hash: 'abc', descriptor: { name: 'x' }, wasm: { hash: 'abc', bytes: 12 }, warnings: ['w'], module: 'module:x', version: 7 }, 'http://c/edit/module:x');
	assert.deepEqual(r, { compiled: true, module: 'module:x', version: 7, hash: 'abc', bytes: 12, warnings: ['w'], descriptor: { name: 'x' }, open: 'http://c/edit/module:x' });
});

test('compiler errors reach the agent with file and line', () => {
	const errors = [{ file: 'lib.rs', line: 3, column: 9, level: 'error', message: 'expected `;`' }];
	assert.deepEqual(compileRefused(new ApiError(422, 'compile_failed', 'no', { errors })), { compiled: false, reason: 'the compiler refused the sources', errors });
});

test('problems of the sources reach the agent', () => {
	assert.deepEqual(compileRefused(new ApiError(422, 'invalid_module', 'no lib.rs', { problems: ['no lib.rs'] })), { compiled: false, reason: 'no lib.rs', problems: ['no lib.rs'] });
});

test('every other answer stays the usual API error', () => {
	assert.equal(compileRefused(new ApiError(403, 'creator_only', 'no')), null);
	assert.equal(compileRefused(new ApiError(503, 'build_busy', 'no')), null);
	assert.equal(compileRefused(new Error('x')), null);
});

test('a saved rig or module links to its page in the creator', () => {
	const links = { creator: 'http://c' } as Parameters<typeof creatorHref>[0];
	assert.equal(creatorHref(links, 'module:my_walk'), 'http://c/edit/module:my_walk');
	assert.equal(creatorHref(links, 'rig:character.catpope'), 'http://c/edit/rig:character.catpope');
});
