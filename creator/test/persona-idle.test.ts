// persona-idle.ts (#817): the picker offers what the core says plays – its own clips, the library's animations that fit –
// and the field's value round-trips to the stored idle. The rule itself is the core's (deck-catalog skeleton.rs, tested in Rust).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { idleChoices, idleOf, idleValue, type LibraryAnimation } from '../src/lib/editor/persona-idle.ts';
import type { IdleCheck } from '../src/lib/editor/core-model.ts';

const model = { family: 'mixamo', joints: ['mixamorig:Hips', 'mixamorig:Spine'], clips: ['walk', 'stand'] };

test('the choices are the ones the core lets through, each asked once', async () => {
	const asked: IdleCheck[] = [];
	const rule = async (c: IdleCheck) => {
		asked.push(c);
		if (c.clip) return c.clip === 'stand' ? ["no clip 'stand'"] : [];
		return c.animation?.joints.includes('tail') ? ['the model lacks 1 of the 1 joints the animation turns: tail'] : [];
	};
	const library: LibraryAnimation[] = [
		{ path: 'characters/animations/catpope-idle.glb', title: 'Cat Pope idle', skeleton: { joints: ['mixamorig:Hips'], animated: ['mixamorig:Hips'] } },
		{ path: 'characters/animations/wag.glb', title: '', skeleton: { joints: ['tail'], animated: ['tail'] } },
		{ path: 'characters/animations/unread.glb', title: 'not read yet' }
	];
	const { clips, library: fits } = await idleChoices(model, library, rule);
	assert.deepEqual(clips, [{ kind: 'clip', value: 'walk', label: 'walk' }]);
	assert.deepEqual(fits, [{ kind: 'animation', value: 'characters/animations/catpope-idle.glb', label: 'Cat Pope idle' }]);
	assert.equal(asked.length, 4, 'two clips, two read animations; an unread one is not offered');
	assert.deepEqual(asked[2], { animation: library[0].skeleton, model });
});

test('the value round-trips to the stored idle', () => {
	for (const idle of [{ clip: 'walk' }, { animation: 'characters/animations/catpope-idle.glb' }]) assert.deepEqual(idleOf(idleValue(idle)), idle);
	assert.equal(idleValue(undefined), '');
	assert.equal(idleOf(''), undefined);
});
