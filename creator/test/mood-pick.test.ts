// `npm test -w scutoons-creator`: the mood chip's faces (lib/editor/mood-pick.ts, #282).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { moodFace } from '../src/lib/editor/mood-pick.ts';

test('a mood shows its own portrait, else neutral, else nothing', () => {
	const p = { neutral: { path: 'catpope/n.png' }, happy: { path: 'catpope/h.png' } };
	assert.equal(moodFace(p, 'happy'), 'catpope/h.png');
	assert.equal(moodFace(p, 'angry'), 'catpope/n.png');
	assert.equal(moodFace({}, 'happy'), '');
});
