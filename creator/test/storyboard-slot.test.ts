// The shot's asset slot (src/lib/editor/storyboard-slot.ts): the picks kept per storyboard, what the slot shows.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readSlots, slotContent, writeSlot } from '../src/lib/editor/storyboard-slot.ts';

const memory = () => {
	const data = new Map<string, string>();
	return { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v), removeItem: (k: string) => void data.delete(k), data };
};

test('a pick is kept per storyboard and read back; an empty slot takes it out', () => {
	const store = memory();
	let slots = writeSlot(store, 'storyboard:catpope:3', {}, 'sb-25-1', { file: 'catpope/3/a.webp' });
	slots = writeSlot(store, 'storyboard:catpope:3', slots, 'sb-25-2', { generator: 'generator:luma' });
	assert.deepEqual(readSlots(store, 'storyboard:catpope:3'), { 'sb-25-1': { file: 'catpope/3/a.webp' }, 'sb-25-2': { generator: 'generator:luma' } });
	assert.deepEqual(readSlots(store, 'storyboard:catpope:4'), {});
	slots = writeSlot(store, 'storyboard:catpope:3', slots, 'sb-25-1', null);
	slots = writeSlot(store, 'storyboard:catpope:3', slots, 'sb-25-2', null);
	assert.deepEqual(slots, {});
	assert.equal(store.data.size, 0);
});

test('what cannot be read is dropped, a blocked store still answers', () => {
	const store = memory();
	store.setItem('sc-storyboard-slots:s', JSON.stringify({ a: { file: 'x.png' }, b: { what: 1 }, c: 'no' }));
	assert.deepEqual(readSlots(store, 's'), { a: { file: 'x.png' } });
	store.setItem('sc-storyboard-slots:s', '{broken');
	assert.deepEqual(readSlots(store, 's'), {});
	assert.deepEqual(readSlots(null, 's'), {});
	const blocked = { getItem: () => null, setItem: () => { throw new Error('quota'); }, removeItem: () => {} };
	assert.deepEqual(writeSlot(blocked, 's', {}, 'a', { file: 'x.png' }), { a: { file: 'x.png' } });
});

test('the slot shows the pick, else the approved generation, else nothing', () => {
	assert.deepEqual(slotContent({ file: 'a/b.mp4' }, undefined), { at: 'file', path: 'a/b.mp4', video: true });
	assert.deepEqual(slotContent({ file: 'a/b.webp' }, { id: 4, file: 'g.png' }), { at: 'file', path: 'a/b.webp', video: false });
	assert.deepEqual(slotContent({ generator: 'generator:luma' }, undefined), { at: 'generator', address: 'generator:luma', name: 'luma' });
	assert.deepEqual(slotContent({ generator: 'generator:luma', name: 'Luma' }, undefined), { at: 'generator', address: 'generator:luma', name: 'Luma' });
	assert.deepEqual(slotContent(undefined, { id: 4, file: 'g.mp4', kind: 'video' }), { at: 'approved', generation: 4, video: true });
	assert.deepEqual(slotContent(undefined, { id: 4, file: null }), { at: 'empty' });
	assert.deepEqual(slotContent(undefined, undefined), { at: 'empty' });
});
