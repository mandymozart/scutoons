import { test } from 'node:test';
import assert from 'node:assert/strict';
import { assignEventIds, newEventId } from '../src/lib/editor/event-ids.ts';

test('assignEventIds: every event of a chapter / scene gets a hidden id; copies get a fresh one', () => {
	assert.match(newEventId(), /^e[a-z0-9]{7}$/);
	const chapter = {
		groups: [
			{ ref: { src: 'chapter1/scene-1.json' } },
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
	assert.equal(chapter.groups[0].events, undefined, 'a scene ref is its own document');
	assert.equal(assignEventIds(chapter, 'chapter'), 0, 'nothing left');
	const scene = { events: [{ id: 'taken', type: 'hideMessage' }] } as any;
	assert.equal(assignEventIds(scene, 'group', ['taken']), 1, 'an id another document uses');
	assert.notEqual(scene.events[0].id, 'taken');
});
