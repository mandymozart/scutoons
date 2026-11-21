// Renaming a group's id along (lib/editor/group-rename.ts): the dry run lists every reference, the other chapters that
// refer to it are the saves, the open chapter changes only when applied; a clash is refused.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applyGroupRename, groupRenameProblem, planGroupRename } from '../src/lib/editor/group-rename.ts';

const jump = (value: string, label: string) => ({ type: 'jumpTo', params: { target: { type: 'chapter', value, label } } });

test('a rename along: the list, the saves, the open chapter', () => {
	const current = { groups: [{ id: 'start', events: [jump('2', 'forest')] }, { id: 'forest', slices: [] }] };
	const others = [
		{ key: 'chapter:catpope:1', number: 1, content: { groups: [{ id: 'a', end: { target: { type: 'chapter', value: '2', label: 'forest' } } }] }, base: 41 },
		{ key: 'chapter:catpope:3', number: 3, content: { groups: [{ id: 'forest', events: [jump('3', 'forest')] }] }, base: 7 }
	];
	const input = { toon: 'catpope', chapter: 2, from: 'forest', to: 'woods', current, others };
	const plan = planGroupRename(input);
	assert.deepEqual(plan.lines, ['Chapter 2 · group 1 (start) · event 1 · jump target', 'Chapter 1 · group 1 (a) · end · jump target']);
	assert.deepEqual(plan.saves, [{ key: 'chapter:catpope:1', content: { groups: [{ id: 'a', end: { target: { type: 'chapter', value: '2', label: 'woods' } } }] }, base: 41 }]);
	assert.equal(current.groups[1].id, 'forest', 'the dry run leaves the open chapter');
	assert.equal((others[0].content.groups[0] as any).end.target.label, 'forest', 'and the loaded ones');
	applyGroupRename(current, input);
	assert.equal(current.groups[1].id, 'woods');
	assert.equal((current.groups[0].events![0] as any).params.target.label, 'woods');
});

test('a clash or a bad id is refused', () => {
	const groups = [{ id: 'start' }, { id: 'forest' }];
	assert.match(groupRenameProblem(groups, 'forest', 'start') ?? '', /already called "start"/);
	assert.match(groupRenameProblem(groups, 'forest', 'two words') ?? '', /isn't a valid id/);
	assert.equal(groupRenameProblem(groups, 'forest', 'woods'), null);
	assert.throws(() => planGroupRename({ toon: 't', chapter: 1, from: 'forest', to: 'start', current: { groups }, others: [] }), /already called/);
});
