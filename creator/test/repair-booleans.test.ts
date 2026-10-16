import { test } from 'node:test';
import assert from 'node:assert/strict';
import { repairBooleans } from '../src/lib/editor/repair-booleans.ts';

test('repairBooleans: the bool fields\' "true" / "false" strings become booleans', () => {
	const scene = {
		label: 'Intro',
		renderer: { type: 'horizontal', params: { endless: true } },
		events: [
			{ type: 'showMessage', params: { text: 'true', wait: 'true' } },
			{ type: 'trigger', trigger: { gate: 'false' } }
		],
		slices: [
			{
				image: 'a.webp',
				video: { src: 'v.mp4', loop: 'false', frame: { ratio: '1:1' } },
				audio: { samples: [{ id: 'rain', loop: 'true' }] },
				dialog: [{ text: 'false', uppercase: 'true', frame: 'false' }]
			}
		]
	};
	assert.equal(repairBooleans(scene), 6);
	assert.equal(scene.slices[0].video.loop, false);
	assert.equal(scene.slices[0].audio.samples[0].loop, true);
	assert.equal(scene.events[0].params!.wait, true);
	assert.equal(scene.events[1].trigger!.gate, false);
	assert.equal(scene.slices[0].dialog[0].uppercase, true);
	assert.equal(scene.slices[0].dialog[0].frame, false);
	// texts and other values stay as they are
	assert.equal(scene.events[0].params!.text, 'true');
	assert.equal(scene.slices[0].dialog[0].text, 'false');
	assert.deepEqual(scene.slices[0].video.frame, { ratio: '1:1' });
	assert.deepEqual(scene.renderer.params, { endless: true });
	assert.equal(repairBooleans(scene), 0, 'nothing left to repair');
});
