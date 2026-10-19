import { test } from 'node:test';
import assert from 'node:assert/strict';
import { assignEventIds, newEventId } from '../src/lib/editor/event-ids.ts';

test('assignEventIds: every event of a chapter / group gets a hidden id; copies get a fresh one', () => {
	assert.match(newEventId(), /^e[a-z0-9]{7}$/);
	const chapter = {
		groups: [
			{ label: 'empty' },
			{
				events: [{ type: 'showMessage', params: { text: {} } }, { id: 'keep', type: 'showChoices' }],
				slices: [
					{ events: [{ type: 'hideMessage', trigger: { type: 'click' } }], items: [{ id: 'coin-1', events: [{ id: 'keep', type: 'hideMessage' }] }] },
					{ image: 'b.webp' }
				]
			}
		]
	} as any;
	assert.equal(assignEventIds(chapter, 'chapter'), 3, 'two without, one copy of "keep"');
	const ids = [chapter.groups[1].events[0].id, chapter.groups[1].events[1].id, chapter.groups[1].slices[0].events[0].id, chapter.groups[1].slices[0].items[0].events[0].id];
	assert.equal(ids[1], 'keep', 'an id stays');
	assert.equal(new Set(ids).size, 4, 'all different');
	assert.equal(chapter.groups[0].events, undefined, 'a group without events keeps none');
	assert.equal(assignEventIds(chapter, 'chapter'), 0, 'nothing left');
	const group = { events: [{ id: 'taken', type: 'hideMessage' }] } as any;
	assert.equal(assignEventIds(group, 'group', ['taken']), 1, 'an id used elsewhere');
	assert.notEqual(group.events[0].id, 'taken');
});
