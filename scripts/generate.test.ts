// generate.ts (#372): the check never writes, the generate run does, and the steps are the generators named in its header.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { steps } from './generate.ts';

test('--check compares only: no step writes', () => {
	for (const s of steps(true)) {
		assert.equal(s.env?.UPDATE_FIXTURES, undefined, s.name);
	}
	assert.equal(steps(true).filter((s) => s.args.at(-1) === '--check').length, 2, 'the protocol and the codecs compare with --check, the ABI test without UPDATE_FIXTURES');
});

test('generate writes: the ABI test is told to update its file', () => {
	const abi = steps(false).find((s) => s.name.startsWith('module ABI'));
	assert.equal(abi?.env?.UPDATE_FIXTURES, '1');
	assert.ok(steps(false).every((s) => !s.args.includes('--check')));
});

test('the codecs generator is built with its feature', () => {
	const codecs = steps(false).find((s) => s.args.includes('codecs-ts'));
	assert.ok(codecs?.args.includes('--features') && codecs.args.includes('codegen'));
});
