// `npm test -w @scutoons/rig-designer`: a descriptor as the read-only summary the console prints after a compile.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { detailLines, headline, summarize } from '../src/descriptor.ts';
import { failureText, isQuiet, nameProblemText, problemText } from '../src/reasons.ts';
import type { Descriptor } from '../src/types.ts';

const walk: Descriptor = {
	abi: 1,
	name: 'walk',
	version: '0.1.0',
	params: [
		{ name: 'spread', min: 0, max: 1, default: 0 },
		{ name: 'rate', min: 0, max: 16, default: 2 },
		{ name: 'base', min: 20, max: 2000, default: 147 }
	],
	ports: [
		{ name: 'cv', dir: 'out', kind: 'audio' },
		{ name: 'notes', dir: 'out', kind: 'event' },
		{ name: 'step', dir: 'in', kind: 'control' }
	]
};

test('summary: name, version, ABI, parameter count, ports by direction', () => {
	const s = summarize(walk);
	assert.deepEqual([s.name, s.version, s.abi, s.paramCount, s.inputs.length, s.outputs.length], ['walk', '0.1.0', 1, 3, 1, 2]);
	assert.equal(headline(walk), 'walk 0.1.0 · ABI 1 · 3 params · 1 in · 2 out');
});

test('detail lines: one per parameter, one per port', () => {
	assert.deepEqual(detailLines(walk), [
		'param spread  0…1  = 0',
		'param rate  0…16  = 2',
		'param base  20…2000  = 147',
		'out cv  audio',
		'out notes  event',
		'in  step  control'
	]);
	assert.deepEqual(detailLines({ ...walk, params: [{ name: 'x', min: 0, max: 1, default: 0.33333333 }], ports: [] }), ['param x  0…1  = 0.3333']);
});

test('a failure is one sentence in the language (English without a setup); a 403 is quiet', () => {
	assert.equal(failureText({ reason: 'creator_only', code: 'creator_only', message: 'x' }), 'Only creators compile modules.');
	assert.equal(failureText({ reason: 'unavailable', code: 'live_show', message: '' }), 'A show is live – compiling waits until it is over.');
	assert.equal(failureText({ reason: 'unavailable', code: 'build_busy', message: '' }), 'The compiler is busy – try again in a moment.');
	assert.equal(failureText({ reason: 'unavailable', code: 'no_build_service', message: '' }), 'The compiler is not set up.');
	assert.equal(failureText({ reason: 'error', code: 'http_500', message: 'boom' }), 'That went wrong: boom');
	assert.ok(isQuiet('creator_only') && isQuiet('signed_out') && !isQuiet('error') && !isQuiet('unavailable'));
});

test('limit and name problems as sentences', () => {
	assert.equal(problemText({ code: 'too_many', limit: 32, actual: 33 }), 'At most 32 files (now 33).');
	assert.equal(problemText({ code: 'file_too_large', file: 'lib.rs', limit: 262144, actual: 300000 }), 'lib.rs is 300000 bytes – at most 262144.');
	assert.equal(problemText({ code: 'no_main', file: 'lib.rs' }), 'It needs a lib.rs.');
	assert.equal(nameProblemText('exists'), 'That file exists already.');
	assert.equal(nameProblemText('too_many'), 'There are already 32 files.');
});
