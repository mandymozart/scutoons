// What a bulk publish says (lib/editor/publish-report.ts, #803): what went live by name, every refusal with its reason.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { nameList, publishNotices } from '../src/lib/editor/publish-report.ts';

const words = {
	done: (count: number, names: string) => `${count} published: ${names}.`,
	refused: (name: string, reason: string) => `Not published: ${name} – ${reason}`,
	nothing: () => 'Nothing to publish.'
};
const label = (key: string) => key.split(':')[1] ?? key;

test('the published ones by name, then one line per refusal with its reason', () => {
	const lines = publishNotices(['instrument:theme', 'preset:theme.base'], [{ key: 'preset:theme.loud', reason: 'needs preset theme.base, which was refused' }], label, words);
	assert.deepEqual(lines, [
		{ type: 'success', text: '2 published: theme, theme.base.' },
		{ type: 'error', text: 'Not published: theme.loud – needs preset theme.base, which was refused' }
	]);
});

test('only refusals: no success line', () => {
	assert.deepEqual(publishNotices([], [{ key: 'module:walk', reason: 'compile it first' }], label, words), [{ type: 'error', text: 'Not published: walk – compile it first' }]);
	assert.deepEqual(publishNotices([], [], label, words), [{ type: 'info', text: 'Nothing to publish.' }]);
});

test('a long list is cut in the line and whole in its detail', () => {
	const keys = Array.from({ length: 8 }, (_, i) => `item:i${i}`);
	assert.equal(nameList(keys.map(label)), 'i0, i1, i2, i3, i4, i5 …');
	const [line] = publishNotices(keys, [], label, words);
	assert.equal(line.detail, keys.map(label).join('\n'));
});
