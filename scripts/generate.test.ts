// generate.ts (#372): the check never writes, the generate run does, and the steps are the generators named in its header
// (#343: the module ABI's TS file is gone with the TS module host – the layout is derived per module, research/ABI-V2.md §7).
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { steps } from './generate.ts';

test('--check compares only: no step writes', () => {
	for (const s of steps(true)) {
		assert.equal(s.env?.UPDATE_FIXTURES, undefined, s.name);
	}
	assert.equal(steps(true).filter((s) => s.args.at(-1) === '--check').length, 3, 'the protocol, the codecs and the server contracts compare with --check');
	assert.equal(steps(true).length, 3, 'three generators: no module ABI table in TS (#343); the server contracts (#477)');
});

test('generate writes: no step runs with --check', () => {
	assert.ok(steps(false).every((s) => !s.args.includes('--check')));
});

test('the codecs generator is built with its feature', () => {
	const codecs = steps(false).find((s) => s.args.includes('codecs-ts'));
	assert.ok(codecs?.args.includes('--features') && codecs.args.includes('codegen'));
});

test('the server contracts run beside their sources (#477)', () => {
	const contracts = steps(false).find((s) => s.args.includes('server/schema/generate.ts'));
	assert.ok(contracts && contracts.args.includes('--experimental-transform-types'));
});
