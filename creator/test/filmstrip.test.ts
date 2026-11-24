// The Filmstrip (src/lib/editor/filmstrip.ts) and the storyboard's shots as its groups (storyboard-shots.ts shotStrip).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { measureShot, type ChapterStoryboard } from '@scutoons/content/storyboard';
import { frameWidth, groupOpen, sliceAt, sliceId, sliceStrip, type FilmGroup } from '../src/lib/editor/filmstrip.ts';
import { shotAspect, shotEntries, shotStrip } from '../src/lib/editor/storyboard-shots.ts';

test('a frame keeps its own shape between a sliver and a wide band', () => {
	assert.equal(frameWidth(9 / 16, 3), 3 * (9 / 16));
	assert.equal(frameWidth(1 / 5, 3), 3 * 0.3);
	assert.equal(frameWidth(3, 3), 3 * 2.2);
	assert.equal(frameWidth(Number.NaN, 2), 2);
	assert.equal(frameWidth(0, 2), 2);
});

test('a group is open while it holds the current part, or as the member left it', () => {
	const g: FilmGroup = { key: 'a', label: 'A', frames: [{ id: 'x', label: 'x', aspect: 1 }] };
	assert.equal(groupOpen(g, 'x', {}), true);
	assert.equal(groupOpen(g, 'y', {}), false);
	assert.equal(groupOpen(g, 'x', { a: false }), false);
	assert.equal(groupOpen(g, 'y', { a: true }), true);
});

test('a shot frame is as wide as its aspect says', () => {
	assert.equal(shotAspect({ aspect: '1:3' }), 1 / 3);
	assert.equal(shotAspect({ aspect: '3:1' }), 3);
	assert.equal(shotAspect({ aspect: 'nonsense' }), 9 / 16);
});

test('the storyboard as the filmstrip: a group per scene, a frame per shot with its format, want and reading time', () => {
	const doc: ChapterStoryboard = {
		storyboard: 2,
		toon: 'catpope',
		chapter: 3,
		scenes: [
			{ storyboard: 2, toon: 'catpope', source: { scene: 25, heading: 'EXT. BERGDORF' }, groups: [{ label: 'a', shots: [{ id: 'sb-25-1', format: 'vertical', aspect: '1:3', action: { de: 'Schnee' }, source: { want: 'photo' } }] }] },
			{ storyboard: 2, toon: 'catpope', source: { scene: 26 }, groups: [{ label: 'b', shots: [{ id: 'sb-26-1', format: 'loop360', aspect: '3:1', action: { de: '' } }] }] }
		]
	};
	const entries = shotEntries(doc);
	const strip = shotStrip(doc, entries, (e) => `frame:${e.id}`, (s) => `${s} s`);
	assert.equal(strip.length, 2);
	assert.deepEqual([strip[0].key, strip[0].number, strip[0].label], ['scene-0', '25', 'EXT. BERGDORF']);
	const f = strip[0].frames[0];
	assert.equal(f.id, 'sb-25-1');
	assert.equal(f.detail, 'vertical 1:3 · photo');
	assert.equal(f.meta, `${measureShot(entries[0].shot).seconds.toFixed(1)} s`);
	assert.equal(f.aspect, 1 / 3);
	assert.equal(f.src, 'frame:sb-25-1');
	assert.equal(f.title, 'Schnee');
	assert.equal(strip[1].frames[0].detail, 'loop360 3:1');
	assert.equal(strip[1].frames[0].title, undefined);
	assert.equal(strip[1].label, '');
});

test("a chapter's slices as the filmstrip: a group per slice group, a frame per slice in its picture's shape", () => {
	const strip = sliceStrip([{ slices: [{ image: 'c3/1.webp', width: 390, height: 1248, shot: 'sb-25-1' }, null, {}] }, {}], (g) => `G${g}`, (p) => `cdn/${p}`);
	assert.deepEqual(strip.map((g) => [g.key, g.label, g.frames.length]), [['group-0', 'G0', 3], ['group-1', 'G1', 0]]);
	assert.deepEqual(strip[0].frames[0], { id: '0/0', label: '1', detail: 'sb-25-1', aspect: 390 / 1248, src: 'cdn/c3/1.webp' });
	assert.equal(strip[0].frames[1].aspect, 390 / 624);
	assert.equal(strip[0].frames[2].src, null);
	assert.deepEqual(sliceAt(sliceId(2, 7)), { group: 2, slice: 7 });
	assert.equal(sliceAt('sb-25-1'), null);
});
