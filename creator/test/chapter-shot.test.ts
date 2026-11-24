// A chapter opened for a storyboard shot (src/lib/editor/chapter-shot.ts): the slice found, or made on first use.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { ChapterStoryboard, Shot } from '@scutoons/content/storyboard';
import { shotSlice, type ShotSliceDoc } from '../src/lib/editor/chapter-shot.ts';

const shot = (id: string, extra: Partial<Shot> = {}): Shot => ({ id, format: 'viewport', aspect: '9:16', action: { de: 'x' }, ...extra });
const storyboard = (shots: Shot[]): ChapterStoryboard => ({
	storyboard: 2, toon: 'catpope', chapter: 3, scenes: [{ storyboard: 2, toon: 'catpope', source: { scene: 1 }, groups: [{ label: 'scene-1', shots }] }]
});
const docOf = (file: any): ShotSliceDoc & { edits: number } => {
	const d = { edits: 0, data: () => file, edit: (change: (f: any) => void) => (d.edits++, change(file)) };
	return d;
};
const at = { toon: 'catpope', chapter: 3, shot: 'sb-1-2' };

test('the slice cut from the shot is found; nothing is made', async () => {
	const doc = docOf({ groups: [{ slices: [{ image: 'a.webp', shot: 'sb-1-1' }, { image: 'b.webp', shot: 'sb-1-2' }] }] });
	assert.deepEqual(await shotSlice(doc, at, async () => null), { ok: true, spot: { group: 0, slice: 1 }, made: false });
	assert.equal(doc.edits, 0);
});

test('first use: a slice after the earlier shot\'s, with the shot\'s picture', async () => {
	const file = { groups: [{ slices: [{ image: 's1/c3/s1c3-slice-1.webp', shot: 'sb-1-1' }, { image: 's1/c3/s1c3-slice-2.webp', shot: 'sb-1-3' }] }] };
	const doc = docOf(file);
	const sb = storyboard([shot('sb-1-1'), shot('sb-1-2', { asset: 'image:catpope/s1/c3/new.webp' }), shot('sb-1-3')]);
	assert.deepEqual(await shotSlice(doc, at, async () => sb), { ok: true, spot: { group: 0, slice: 1 }, made: true });
	assert.deepEqual(file.groups[0].slices[1], { image: 's1/c3/new.webp', width: 390, height: 624, shot: 'sb-1-2' });
});

test('first use without a picture: the next slice name; a generator is no picture', async () => {
	const file = { groups: [{ slices: [{ image: 's1/c3/s1c3-slice-4.webp', shot: 'sb-1-1' }] }] };
	const sb = storyboard([shot('sb-1-1'), shot('sb-1-2', { asset: 'generator:luma' })]);
	await shotSlice(docOf(file), at, async () => sb);
	assert.equal((file.groups[0].slices[1] as { image: string }).image, 's1/c3/s1c3-slice-5.webp');
});

test('first use where no shot has a slice yet: the end of the shot\'s scene\'s group', async () => {
	const file = { groups: [{ id: 'scene-1', slices: [{ image: 'a-1.webp' }] }, { id: 'scene-2', slices: [{ image: 'b-1.webp' }] }] };
	assert.deepEqual(await shotSlice(docOf(file), at, async () => storyboard([shot('sb-1-1'), shot('sb-1-2')])), { ok: true, spot: { group: 0, slice: 1 }, made: true });
});

test('no such shot, no group: nothing is made', async () => {
	assert.deepEqual(await shotSlice(docOf({ groups: [{ slices: [] }] }), at, async () => storyboard([shot('sb-1-1')])), { ok: false, why: 'no-shot' });
	assert.deepEqual(await shotSlice(docOf({ groups: [] }), at, async () => storyboard([shot('sb-1-2')])), { ok: false, why: 'no-group' });
});
