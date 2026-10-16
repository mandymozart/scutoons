import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupMarks, sliceMarks } from '../src/lib/editor/slice-marks.ts';

test('a slice: text boxes, placed items (their tap events count), audio, video, map places', () => {
	const slice = {
		dialog: [{}, {}],
		items: [{ item: 'cucumber', events: [{ type: 'unlockLocation', params: { location: 'x' } }] }],
		audio: { onEnter: [] },
		events: [{ type: 'showTextBox' }, { type: 'moveAvatar', params: { location: 'x' } }]
	};
	assert.deepEqual(sliceMarks(slice), { texts: 2, items: 1, audio: 1, video: 0, places: 2, events: 3 });
	assert.deepEqual(sliceMarks(undefined), { texts: 0, items: 0, audio: 0, video: 0, places: 0, events: 0 });
});

test('a group: its slices added up plus its own script', () => {
	const group = { slices: [{ video: { src: 'a.mp4' } }, { items: [{ item: 'a' }, { item: 'b' }] }], events: [{ type: 'unlockLocation' }] };
	assert.deepEqual(groupMarks(group), { texts: 0, items: 2, audio: 0, video: 1, places: 1, events: 1 });
});
