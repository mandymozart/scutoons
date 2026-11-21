import { test } from 'node:test';
import assert from 'node:assert/strict';
import { describeGroupIdChange, groupIdProblem, isGroupId, newGroupId, renameGroupId, slugGroupId } from '../src/group-ids.ts';

test('group ids: slug-safe, unique in the chapter, a new one is free', () => {
	assert.equal(isGroupId('scene-12'), true);
	assert.equal(isGroupId('a_b'), true);
	assert.equal(isGroupId('-a'), false);
	assert.equal(isGroupId('a b'), false);
	assert.equal(isGroupId(''), false);
	const groups = [{ id: 'g1' }, { id: 'forest' }, { id: 'g3' }];
	assert.equal(groupIdProblem(groups, 0, 'g1'), null, 'its own id is no clash');
	assert.match(groupIdProblem(groups, 0, 'forest') ?? '', /group 2 .* already called "forest"/);
	assert.match(groupIdProblem(groups, 0, 'a b') ?? '', /isn't a valid id/);
	assert.match(groupIdProblem(groups, 0, '') ?? '', /needs an id/);
	assert.equal(newGroupId(groups), 'g4');
	assert.equal(newGroupId([{ id: 'g2' }]), 'g3', 'past ids already taken');
	assert.equal(newGroupId([]), 'g1');
	assert.equal(slugGroupId(' Mama im Wald! '), 'Mama-im-Wald');
	assert.equal(slugGroupId('Über-Straße'), 'Uber-Strae');
});

test('renameGroupId: every reference in the toon is rewritten along, other toons and chapters stay', () => {
	const jumpTo = (target: object) => ({ type: 'jumpTo', params: { target } });
	const chapters = [
		{
			number: 2,
			doc: { groups: [
				{ id: 'start', slices: [{ events: [jumpTo({ type: 'chapter', value: '2', label: 'forest' })], items: [{ item: 'key', events: [jumpTo({ type: 'chapter', value: '2', anyOf: ['forest', 'harbour'] })] }] }] },
				{ id: 'forest', events: [{ type: 'addChoice', params: { label: 'forest', onSelect: [{ jumpTo: { target: { type: 'chapter', value: '2', label: 'forest' } } }] } }] },
				{ id: 'harbour', end: { target: { type: 'chapter', value: '2', label: 'forest', toon: 'catpope' } } }
			] }
		},
		{
			number: 3,
			doc: { groups: [
				{ id: 'forest', end: { target: { type: 'chapter', value: '2', label: 'forest' }, auto: true } },
				{ id: 'other', events: [jumpTo({ type: 'chapter', value: '3', label: 'forest' }), jumpTo({ type: 'chapter', value: '2', label: 'forest', toon: 'toptoy' })] }
			] }
		}
	];
	const changes = renameGroupId({ chapters, chapter: 2, from: 'forest', to: 'woods', toon: 'catpope' });
	const c2 = chapters[0].doc.groups as any[];
	const c3 = chapters[1].doc.groups as any[];
	assert.equal(c2[1].id, 'woods');
	assert.equal(c2[0].slices[0].events[0].params.target.label, 'woods', 'a slice event');
	assert.deepEqual(c2[0].slices[0].items[0].events[0].params.target.anyOf, ['woods', 'harbour'], 'a random list');
	assert.equal(c2[1].events[0].params.onSelect[0].jumpTo.target.label, 'woods', 'a choice; its text stays');
	assert.equal(c2[1].events[0].params.label, 'forest');
	assert.equal(c2[2].end.target.label, 'woods', 'an end naming its own toon');
	assert.equal(c3[0].id, 'forest', "chapter 3's own forest stays");
	assert.equal(c3[0].end.target.label, 'woods', 'another chapter jumping into chapter 2');
	assert.equal(c3[1].events[0].params.target.label, 'forest', "a jump to chapter 3's forest stays");
	assert.equal(c3[1].events[1].params.target.label, 'forest', 'a jump into another toon stays');
	assert.equal(changes.length, 5);
	assert.deepEqual(
		changes.map((c) => describeGroupIdChange(c, chapters)),
		[
			'Chapter 2 · group 1 (start) · slice 1 · event 1 · jump target',
			'Chapter 2 · group 1 (start) · slice 1 · item 1 · event 1 · random jump',
			'Chapter 2 · group 2 (woods) · event 1 · jump target',
			'Chapter 2 · group 3 (harbour) · end · jump target',
			'Chapter 3 · group 1 (forest) · end · jump target'
		]
	);
	assert.throws(() => renameGroupId({ chapters, chapter: 2, from: 'woods', to: 'start' }), /already called "start"/);
	assert.throws(() => renameGroupId({ chapters, chapter: 2, from: 'nope', to: 'x' }), /no group "nope"/);
	assert.deepEqual(renameGroupId({ chapters, chapter: 2, from: 'woods', to: 'woods' }), []);
});
