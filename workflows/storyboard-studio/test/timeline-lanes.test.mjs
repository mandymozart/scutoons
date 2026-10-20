// The studio's clips lane (src/lib/timeline-lanes.js): the picked shots back to back, one clip each by the view mode
// with its fallbacks (generated → rendered → take → a 5 s still), the take's trim in Take mode, unknown video lengths,
// the clip under the playhead, Play's skip over cut parts, the trim's edge / playhead and I / O.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clipAt, clipOf, clipsLane, FRAME_MS, isVideo, kept, MODES, movedEdge, STILL_MS, trimAt, trimPoint } from '../src/lib/timeline-lanes.js';

const take = { id: 't2', in: 1200, out: 4800, length: 6010, kept: 3600 };
const clips = {
	'sb-1': {
		take,
		rendered: [
			{ file: 'sb-1--t3--beauty.mp4', url: '/r/t3-beauty', pass: 'beauty', take: 't3', ms: 9000 },
			{ file: 'sb-1--t2--depth.mp4', url: '/r/t2-depth', pass: 'depth', take: 't2', ms: 3600 },
			{ file: 'sb-1--t2--beauty.mp4', url: '/r/t2-beauty', pass: 'beauty', take: 't2', ms: 3650 }
		],
		generated: [
			{ file: 'sb-1--g3.png', url: '/g/3.png', ms: null, video: false, approved: false },
			{ file: 'sb-1--g2.mp4', url: '/g/2.mp4', ms: 8000, video: true, approved: false },
			{ file: 'sb-1--g1.mp4', url: '/g/1.mp4', ms: 10000, video: true, approved: true }
		]
	},
	'sb-2': { take: null, rendered: [], generated: [{ file: 'sb-2--g1.mp4', url: '/g/b1.mp4', ms: null, video: true, approved: false }] },
	'sb-3': { take: { id: 't1', in: 0, out: 2000, length: 2000, kept: 2000 }, rendered: [], generated: [] }
};

test('the modes and the still', () => {
	assert.deepEqual(MODES, ['shot', 'take', 'rendered', 'generated']);
	assert.equal(STILL_MS, 5000);
	assert.equal(isVideo('generated') && isVideo('rendered'), true);
	assert.equal(isVideo('take') || isVideo('still'), false);
});

test('generated: the approved video, else the newest – an image never', () => {
	const c = clipOf(clips['sb-1'], 'generated');
	assert.deepEqual([c.source, c.url, c.length, c.from], ['generated', '/g/1.mp4', 10000, 0]);
	const newest = clipOf({ ...clips['sb-1'], generated: clips['sb-1'].generated.map((g) => ({ ...g, approved: false })) }, 'generated');
	assert.equal(newest.url, '/g/2.mp4');
	const images = clipOf({ take: null, generated: [clips['sb-1'].generated[0]] }, 'generated');
	assert.equal(images.source, 'still');
});

test('rendered: the active take\'s beauty pass (an older take\'s render is not it)', () => {
	const c = clipOf(clips['sb-1'], 'rendered');
	assert.deepEqual([c.source, c.url, c.length], ['rendered', '/r/t2-beauty', 3650]);
	// no render of the active take: its take
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
	const c = clipOf({ take }, 'generated');
	assert.deepEqual([c.source, c.length, c.from], ['take', 3600, 1200]);
	const whole = clipOf({ take }, 'take');
	assert.deepEqual([whole.length, whole.from], [6010, 0]);
	const { lane } = clipsLane(['sb-1'], 'take', { 'sb-1': { take } });
	assert.deepEqual(lane.items[0].trim, { in: 1200, out: 4800, min: 100 });
	// the trim only in Take mode
	assert.equal(clipsLane(['sb-1'], 'rendered', { 'sb-1': { take } }).lane.items[0].trim, undefined);
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
	assert.deepEqual(probe, ['/g/b1.mp4']); // … until its metadata is read
	assert.deepEqual([lane.id, lane.tall], ['clips', true]);
	// a generated clip names its generation and its status (the chosen = approved)
	assert.deepEqual(lane.items.map((i) => i.label), ['sb-1 · generated g1 · chosen', 'sb-2 · generated g1', 'sb-3 · take', 'sb-9 · still']);
	assert.equal(lane.selectable, undefined); // a click on a clip makes its shot the current one
});

test('a length read from the video replaces the 5 s; the live take overrides the clips data', () => {
	const { lane, probe } = clipsLane(['sb-2'], 'generated', clips, { '/g/b1.mp4': 7300 });
	assert.deepEqual([lane.items[0].length, probe], [7300, []]);
	const trimmed = { ...take, out: 3000, kept: 1800 };
	assert.deepEqual(clipsLane(['sb-1'], 'take', clips, {}, { 'sb-1': trimmed }).lane.items[0].trim, { in: 1200, out: 3000, min: 100 });
	// the handheld camera switched off: no take, a still
	assert.equal(clipsLane(['sb-3'], 'take', clips, {}, { 'sb-3': null }).lane.items[0].state, 'still');
});

test('nothing picked: an empty lane', () => {
	const { lane, span } = clipsLane([], 'generated', clips);
	assert.deepEqual([lane.items.length, span], [0, 0]);
	assert.equal(clipAt(lane.items, 0), null);
});

test('the clip under the playhead: [start, end), the end of the last one = the last one at its end', () => {
	const { lane } = clipsLane(['sb-1', 'sb-3'], 'generated', clips);
	assert.deepEqual([clipAt(lane.items, 0).item.id, clipAt(lane.items, 0).off], ['sb-1', 0]);
	assert.deepEqual([clipAt(lane.items, 9999).item.id, clipAt(lane.items, 10000).item.id], ['sb-1', 'sb-3']);
	assert.deepEqual([clipAt(lane.items, 12000).item.id, clipAt(lane.items, 12000).off], ['sb-3', 2000]);
	assert.deepEqual([clipAt(lane.items, 99999).item.id, clipAt(lane.items, 99999).off], ['sb-3', 2000]);
	assert.equal(clipAt(lane.items, -1), null);
});

test('Play skips the cut parts: before the in → the in, from the out → the next clip (and its in)', () => {
	const t3 = { id: 't1', in: 500, out: 1500, length: 2000, kept: 1000 };
	const { lane } = clipsLane(['sb-1', 'sb-3', 'sb-2'], 'take', { 'sb-1': { take }, 'sb-3': { take: t3 } });
	const items = lane.items; // sb-1 0–6010 (kept 1200–4800), sb-3 6010–8010 (kept 6510–7510), sb-2 a still
	assert.equal(kept(items, 0), 1200);
	assert.equal(kept(items, 3000), 3000);
	assert.equal(kept(items, 4800), 6510);
	assert.equal(kept(items, 7600), 8010);
	assert.equal(kept(items, 9000), 9000);
	assert.equal(kept(items, 13010), 13010); // the end
});

test('a trim: the moved edge (both = a slip) and where the playhead goes – the in, or the last kept frame', () => {
	assert.equal(FRAME_MS, 1000 / 24);
	assert.equal(clipsLane(['sb-1'], 'take', { 'sb-1': { take } }).lane.step, FRAME_MS);
	const was = { in: 1200, out: 4800 };
	assert.equal(movedEdge(was, { in: 1000, out: 4800 }), 'in');
	assert.equal(movedEdge(was, { in: 1200, out: 5000 }), 'out');
	assert.equal(movedEdge(was, { in: 1400, out: 5000 }), 'slip');
	const it = { start: 6010 };
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'in'), 6510);
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'slip'), 6510);
	assert.equal(trimPoint(it, { in: 500, out: 1500 }, 'out'), 6010 + 1500 - FRAME_MS);
	// a kept part shorter than a frame: never before the in
	assert.equal(trimPoint(it, { in: 500, out: 520 }, 'out'), 6510);
});

test('I / O at the playhead: I = the frame under it is the first kept, O = the last kept; the other edge stays', () => {
	const t3 = { id: 't1', in: 500, out: 1500, length: 2000, kept: 1000 };
	const items = clipsLane(['sb-1', 'sb-3', 'sb-2'], 'take', { 'sb-1': { take }, 'sb-3': { take: t3 } }).lane.items;
	const r = (x) => x && [x.item.id, Math.round(x.in * 1000) / 1000, Math.round(x.out * 1000) / 1000];
	assert.deepEqual(r(trimAt(items, 2000, 'in')), ['sb-1', 2000, 4800]); // frame 48 starts at 2000
	assert.deepEqual(r(trimAt(items, 2030, 'in')), ['sb-1', 2000, 4800]); // still frame 48
	assert.deepEqual(r(trimAt(items, 2000, 'out')), ['sb-1', 1200, 2041.667]); // frame 48 is the last kept
	// the kept part stays ≥ its min: I past the out, O before the in
	assert.deepEqual(r(trimAt(items, 5000, 'in')), ['sb-1', 4700, 4800]);
	assert.deepEqual(r(trimAt(items, 1000, 'out')), ['sb-1', 1200, 1300]);
	// O on the take's last frame: its end
	assert.deepEqual(r(trimAt(items, 6009, 'out')), ['sb-1', 1200, 6010]);
	// the next take, ms of its own take
	assert.deepEqual(r(trimAt(items, 6010 + 1000, 'in')), ['sb-3', 1000, 1500]);
	// the O of a trimPoint keeps the out (the playhead on the last kept frame)
	const back = trimAt(items, trimPoint(items[0], { in: 1200, out: 2000 }, 'out'), 'out');
	assert.equal(Math.round(back.out * 1000) / 1000, 2000);
	// a still, a take outside Take mode: nothing to trim
	assert.equal(trimAt(items, 9000, 'in'), null);
	assert.equal(trimAt(clipsLane(['sb-1'], 'rendered', clips).lane.items, 2000, 'in'), null);
	assert.equal(trimAt([], 0, 'out'), null);
});
