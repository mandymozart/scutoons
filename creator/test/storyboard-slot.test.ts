// The shot's asset slot (src/lib/editor/storyboard-slot.ts): the address a pick stores, what the slot shows.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pickedFile, slotContent } from '../src/lib/editor/storyboard-slot.ts';

test('a picked file stores its Library address: the family\'s main file, else the path', () => {
	assert.equal(pickedFile('catpope/3/a.small.webp', { kind: 'image', main: 'catpope/3/a.webp' }), 'image:catpope/3/a.webp');
	assert.equal(pickedFile('catpope/3/t.mp4'), 'video:catpope/3/t.mp4');
	assert.equal(pickedFile('catpope/3/a.webp'), 'image:catpope/3/a.webp');
	assert.equal(pickedFile('catpope/3/m.glb', { kind: 'model', main: 'catpope/3/m.glb' }), null);
});

test('the slot shows the stored asset, else the approved generation', () => {
	assert.deepEqual(slotContent('video:a/b.mp4', undefined), { at: 'file', path: 'a/b.mp4', video: true });
	assert.deepEqual(slotContent('image:a/b.webp', { id: 4, file: 'g.png' }), { at: 'file', path: 'a/b.webp', video: false });
	assert.deepEqual(slotContent('generator:luma', undefined), { at: 'generator', address: 'generator:luma', name: 'luma' });
	assert.deepEqual(slotContent(undefined, { id: 4, file: 'g.mp4', kind: 'video' }), { at: 'approved', generation: 4, video: true });
	assert.deepEqual(slotContent('not an address', { id: 4, file: null }), { at: 'empty' });
	assert.deepEqual(slotContent(undefined, undefined), { at: 'empty' });
});
