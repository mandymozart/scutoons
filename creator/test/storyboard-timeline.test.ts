// The storyboard page's clips lane (src/lib/editor/storyboard-timeline.ts, #126 – the studio's timeline-lanes tests
// ported): the picked shots back to back, one clip each by the view mode with its fallbacks (generated → rendered → take
// → a 5 s still), the clips from the shot's takes + the generation records, the take's trim in Take mode, unknown video
// lengths, the clip under the playhead, Play's skip over cut parts, the trim's edge / playhead and I / O, the steps'
// timelines, the filmstrip's tiles.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Shot } from '@scutoons/content/storyboard';
import { clipAt, clipOf, clipsInOf, clipsLane, filmstrip, FRAME_MS, isVideo, kept, MODES, movedEdge, STILL_MS, stepTimeline, trimAt, trimPoint, type ClipsIn, type GenerationLike } from '../src/lib/editor/storyboard-timeline.ts';

const take = { id: 't2', in: 1200, out: 4800, length: 6010, kept: 3600 };
const clips: Record<string, ClipsIn> = {
	'sb-1': {
		take,
		rendered: [
			{ id: 9, file: 'sb-1--g9.webm', take: 't3', ms: 9000 },
			{ id: 7, file: 'sb-1--g7.webm', take: 't2', ms: 3650 }
		],
		generated: [
			{ id: 6, file: 'sb-1--g6.mp4', ms: 8000, approved: false },
			{ id: 5, file: 'sb-1--g5.mp4', ms: 10000, approved: true }
		]
	},
	'sb-2': { take: null, rendered: [], generated: [{ id: 11, file: 'sb-2--g1.mp4', ms: null, approved: false }] },
	'sb-3': { take: { id: 't1', in: 0, out: 2000, length: 2000, kept: 2000 }, rendered: [], generated: [] }
};

test('the modes and the still', () => {
	assert.deepEqual(MODES, ['shot', 'take', 'rendered', 'generated']);
	assert.equal(STILL_MS, 5000);
	assert.equal(isVideo('generated') && isVideo('rendered'), true);
	assert.equal(isVideo('take') || isVideo('still'), false);
});

test('the clips of a shot: its active take (camera on), the renders of that take, the generated videos not rejected', () => {
	const shot = { id: 'sb-1', takes: { on: true, active: 't2', takes: [{ id: 't2', created: '', smooth: 0, scale: 1, in: 1, out: 2, samples: [{ t: 0, position: [0, 0, 0], quaternion: [0, 0, 0, 1] }, { t: 3, position: [0, 0, 0], quaternion: [0, 0, 0, 1] }] }] } } as Pick<Shot, 'id' | 'takes'>;
	const g = (o: Partial<GenerationLike>): GenerationLike => ({ id: 1, shot: 'sb-1', kind: 'video', status: 'succeeded', generator: 'runway', take: null, file: 'f.mp4', duration: null, mark: null, approved: false, ...o });
	const gens = [
		g({ id: 1, generator: 'handheld-render', take: 't2', duration: 1 }),
		g({ id: 2, generator: 'handheld-render', take: 't1' }),
		g({ id: 3, duration: 4.5 }),
		g({ id: 4, mark: 'rejected' }),
		g({ id: 5, kind: 'image' }),
		g({ id: 6, status: 'running' }),
		g({ id: 7, shot: 'sb-2' }),
		g({ id: 8, generator: 'handheld-render', take: 't2' })
	];
	const c = clipsInOf(shot, gens);
	assert.deepEqual(c.take, { id: 't2', in: 1000, out: 2000, length: 3000, kept: 1000 });
	assert.deepEqual(c.rendered.map((v) => v.id), [8, 1]);
	assert.deepEqual(c.generated.map((v) => [v.id, v.ms]), [[3, 4500]]);
	// the camera off: no take, no renders
	const off = clipsInOf({ ...shot, takes: { ...shot.takes!, on: false } }, gens);
	assert.deepEqual([off.take, off.rendered], [null, []]);
});

test('generated: the approved video, else the newest', () => {
	const c = clipOf(clips['sb-1'], 'generated');
	assert.deepEqual([c.source, c.gen, c.length, c.from], ['generated', 5, 10000, 0]);
	assert.equal(clipOf({ ...clips['sb-1'], generated: clips['sb-1'].generated.map((v) => ({ ...v, approved: false })) }, 'generated').gen, 6);
});

test('rendered: the active take\'s render (an older take\'s is not it)', () => {
	const c = clipOf(clips['sb-1'], 'rendered');
	assert.deepEqual([c.source, c.gen, c.length], ['rendered', 7, 3650]);
	assert.equal(clipOf({ ...clips['sb-1'], rendered: [clips['sb-1'].rendered[0]] }, 'rendered').source, 'take');
});

test('the chain falls back generated → rendered → take → still; Shot mode = every shot a still', () => {
	const noGen = { ...clips['sb-1'], generated: [] };
	assert.equal(clipOf(noGen, 'generated').source, 'rendered');
	assert.equal(clipOf({ ...noGen, rendered: [] }, 'generated').source, 'take');
	assert.equal(clipOf(undefined, 'generated').source, 'still');
	assert.equal(clipOf(clips['sb-1'], 'take').source, 'take');
	for (const id of Object.keys(clips)) assert.deepEqual([clipOf(clips[id], 'shot').source, clipOf(clips[id], 'shot').length], ['still', STILL_MS]);
	assert.equal(clipOf(clips['sb-1'], 'nonsense').source, 'still');
});

test('a take: its kept part from its in – in Take mode the whole take with the trim', () => {
	const only = { take, rendered: [], generated: [] };
	const c = clipOf(only, 'generated');
	assert.deepEqual([c.source, c.length, c.from], ['take', 3600, 1200]);
	const whole = clipOf(only, 'take');
	assert.deepEqual([whole.length, whole.from], [6010, 0]);
	assert.deepEqual(clipsLane(['sb-1'], 'take', { 'sb-1': only }).lane.items[0].trim, { in: 1200, out: 4800, min: 100 });
	assert.equal(clipsLane(['sb-1'], 'rendered', { 'sb-1': only }).lane.items[0].trim, undefined);
});

test('the picked shots back to back in the given (toon) order, one lane, the source as the state', () => {
	const { lane, span, probe } = clipsLane(['sb-1', 'sb-2', 'sb-3', 'sb-9'], 'generated', clips);
	assert.deepEqual(lane.items.map((i) => [i.id, i.start, i.length, i.state]), [
		['sb-1', 0, 10000, 'generated'],
		['sb-2', 10000, STILL_MS, 'generated'], // a video of unknown length counts 5 s …
		['sb-3', 15000, 2000, 'take'],
		['sb-9', 17000, STILL_MS, 'still']
	]);
	assert.equal(span, 22000);
	assert.deepEqual(probe, [11]); // … until its metadata is read
	assert.deepEqual([lane.id, lane.tall, lane.step], ['clips', true, FRAME_MS]);
	assert.deepEqual(lane.items.map((i) => i.label), ['sb-1 · generated g5 · chosen', 'sb-2 · generated g1', 'sb-3 · take', 'sb-9 · still']);
	// the words come from the page (de / en)
	assert.equal(clipsLane(['sb-3'], 'shot', clips, {}, { still: 'Standbild' }).lane.items[0].label, 'sb-3 · Standbild');
});

test('the board\'s current card auditions as the Generated clip; the marks are the status bullets', () => {
	const over = { 'sb-1': { pick: { id: 6, file: 'sb-1--g6.mp4', ms: 8000, approved: false } } };
	const { lane } = clipsLane(['sb-1', 'sb-2'], 'generated', clips, {}, {}, 'Clips', over, { 6: 'review', 11: 'review' });
	assert.deepEqual(lane.items.map((i) => [i.clip.gen, i.clip.audition ?? false, i.clip.status, i.length]), [
		[6, true, 'review', 8000],
		[11, false, 'review', STILL_MS]
	]);
	assert.equal(lane.items[0].label, 'sb-1 · generated g6 · review · audition');
	// without the audition: the chosen, its ✓
	assert.deepEqual(clipsLane(['sb-1'], 'generated', clips).lane.items[0].clip.status, 'chosen');
	// the audition only in the Generated chain: a Take-mode clip stays the take
	assert.equal(clipsLane(['sb-1'], 'take', clips, {}, {}, 'Clips', over).lane.items[0].clip.source, 'take');
});

test('a length read from the video replaces the 5 s', () => {
	const { lane, probe } = clipsLane(['sb-2'], 'generated', clips, { 11: 7300 });
	assert.deepEqual([lane.items[0].length, probe], [7300, []]);
});

test('nothing picked: an empty lane', () => {
	const { lane, span } = clipsLane([], 'generated', clips);
	assert.deepEqual([lane.items.length, span], [0, 0]);
	assert.equal(clipAt(lane.items, 0), null);
});

test('the clip under the playhead: [start, end), the end of the last one = the last one at its end', () => {
	const { lane } = clipsLane(['sb-1', 'sb-3'], 'generated', clips);
	assert.deepEqual([clipAt(lane.items, 0)!.item.id, clipAt(lane.items, 0)!.off], ['sb-1', 0]);
	assert.deepEqual([clipAt(lane.items, 9999)!.item.id, clipAt(lane.items, 10000)!.item.id], ['sb-1', 'sb-3']);
	assert.deepEqual([clipAt(lane.items, 12000)!.item.id, clipAt(lane.items, 12000)!.off], ['sb-3', 2000]);
	assert.deepEqual([clipAt(lane.items, 99999)!.item.id, clipAt(lane.items, 99999)!.off], ['sb-3', 2000]);
	assert.equal(clipAt(lane.items, -1), null);
});

test('Play skips the cut parts: before the in → the in, from the out → the next clip (and its in)', () => {
	const t3 = { id: 't1', in: 500, out: 1500, length: 2000, kept: 1000 };
	const items = clipsLane(['sb-1', 'sb-3', 'sb-2'], 'take', { 'sb-1': { take, rendered: [], generated: [] }, 'sb-3': { take: t3, rendered: [], generated: [] } }).lane.items;
	assert.equal(kept(items, 0), 1200);
	assert.equal(kept(items, 3000), 3000);
	assert.equal(kept(items, 4800), 6510);
	assert.equal(kept(items, 7600), 8010);
	assert.equal(kept(items, 9000), 9000);
	assert.equal(kept(items, 13010), 13010);
});

test('a trim: the moved edge (both = a slip) and where the playhead goes – the in, or the last kept frame', () => {
	const was = { in: 1200, out: 4800 };
	assert.equal(movedEdge(was, { in: 1000, out: 4800 }), 'in');
	assert.equal(movedEdge(was, { in: 1200, out: 5000 }), 'out');
	assert.equal(movedEdge(was, { in: 1400, out: 5000 }), 'slip');
	const it = { start: 6010 };
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'in'), 6510);
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'slip'), 6510);
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'out'), 6010 + 1500 - FRAME_MS);
	assert.equal(trimPoint(it, { in: 500, out: 520 }, 'out'), 6510);
});

test('I / O at the playhead: I = the frame under it is the first kept, O = the last kept; the other edge stays', () => {
	const t3 = { id: 't1', in: 500, out: 1500, length: 2000, kept: 1000 };
	const items = clipsLane(['sb-1', 'sb-3', 'sb-2'], 'take', { 'sb-1': { take, rendered: [], generated: [] }, 'sb-3': { take: t3, rendered: [], generated: [] } }).lane.items;
	const r = (x: ReturnType<typeof trimAt>) => x && [x.item.id, Math.round(x.in * 1000) / 1000, Math.round(x.out * 1000) / 1000];
	assert.deepEqual(r(trimAt(items, 2000, 'in')), ['sb-1', 2000, 4800]);
	assert.deepEqual(r(trimAt(items, 2030, 'in')), ['sb-1', 2000, 4800]);
	assert.deepEqual(r(trimAt(items, 2000, 'out')), ['sb-1', 1200, 2041.667]);
	assert.deepEqual(r(trimAt(items, 5000, 'in')), ['sb-1', 4700, 4800]);
	assert.deepEqual(r(trimAt(items, 1000, 'out')), ['sb-1', 1200, 1300]);
	assert.deepEqual(r(trimAt(items, 6009, 'out')), ['sb-1', 1200, 6010]);
	assert.deepEqual(r(trimAt(items, 6010 + 1000, 'in')), ['sb-3', 1000, 1500]);
	const back = trimAt(items, trimPoint(items[0], { in: 1200, out: 2000 }, 'out'), 'out')!;
	assert.equal(Math.round(back.out * 1000) / 1000, 2000);
	assert.equal(trimAt(items, 9000, 'in'), null);
	assert.equal(trimAt(clipsLane(['sb-1'], 'rendered', clips).lane.items, 2000, 'in'), null);
	assert.equal(trimAt([], 0, 'out'), null);
});

test('the steps\' timelines: Handheld = Take open, Generate = Generated open, Export = Rendered folded; a remembered one wins', () => {
	assert.deepEqual(stepTimeline(null, 'handheld'), { mode: 'take', folded: false });
	assert.deepEqual(stepTimeline(null, 'generate'), { mode: 'generated', folded: false });
	assert.deepEqual(stepTimeline(null, 'export'), { mode: 'rendered', folded: true });
	assert.deepEqual(stepTimeline(null, 'shots'), { mode: 'shot', folded: true });
	assert.deepEqual(stepTimeline({ mode: 'generated', folded: false }, 'shots'), { mode: 'generated', folded: false });
	assert.deepEqual(stepTimeline({ mode: 'nope', folded: 'x' }, 'handheld'), { mode: 'take', folded: false });
});

test('the filmstrip: tiles at the video\'s aspect along the clip (the last one cut), each the frame at its left edge', () => {
	// a 16:9 video, a clip 1000 × 90 px: a tile is 160 px → 7 tiles (6.25 → 7)
	const t = filmstrip({ width: 1280, height: 720, w: 1000, h: 90, length: 10000 });
	assert.equal(t.length, 7);
	assert.deepEqual(t.slice(0, 3), [0, 1600, 3200]);
	assert.equal(filmstrip({ width: 1280, height: 720, w: 30, h: 90, length: 1000 }).length, 1, 'narrower than a tile: one');
	assert.equal(filmstrip({ width: 1280, height: 720, w: 1e6, h: 90, length: 1000 }).length, 24, 'capped');
	assert.deepEqual(filmstrip({ width: 0, height: 720, w: 100, h: 90, length: 1000 }), []);
});
