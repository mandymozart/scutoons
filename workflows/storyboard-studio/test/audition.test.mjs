// The audition (src/lib/audition.js, the lane's override in src/lib/timeline-lanes.js – research/VARIANTS-CANVAS.md §3.1
// Q3): the current shot's Generated clip = the board's current card – a rejected one, a source, a gone file, another
// shot's card: the chosen; an image stays a still (5 s) that shows the image; the override changes only its shot; the
// status bullets (✓ the chosen from the clips data, ? / ✕ the board's marks; a rejected video is no fallback); the
// board's current card when none was clicked (its chosen, the video first); the audition holds while the board is open
// and while Play from it runs – not after Play's end, the board closed by hand or another step.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { auditionPick, boardMarks, currentCard, holds, marksOf } from '../src/lib/audition.js';
import { clipAt, clipOf, clipsLane, labelOf, STILL_MS } from '../src/lib/timeline-lanes.js';

const S = 'sb-25-2';
const rec = (n, f) => ({ id: `${S}--g${n}`, shot: S, file: `${S}--g${n}.${f.kind === 'video' ? 'webm' : 'png'}`, url: `/out/c/generated/${S}--g${n}.${f.kind === 'video' ? 'webm' : 'png'}`, generator: 'runway', parents: [], mark: null, approved: false, missing: false, earlier: false, ...f });
const records = [
	rec(1, { kind: 'image', approved: true, generator: 'comfy-local' }),
	rec(2, { kind: 'video', ms: 2000, mark: 'review', parents: [`${S}--g1`] }),
	rec(3, { kind: 'video', ms: 4000, parents: [`${S}--g1`] }),
	rec(4, { kind: 'image', parents: [`${S}--g1`] }),
	rec(5, { kind: 'video', ms: null, duration: 5, mark: 'rejected', parents: [`${S}--g1`] }),
	rec(6, { kind: 'video', ms: 3000, generator: 'upload' }),
	rec(7, { kind: 'video', ms: 3000, missing: true }),
	rec(8, { kind: 'video', ms: null, duration: 6, earlier: true, generator: null }),
	rec(9, { kind: 'video', ms: 2500, approved: true })
];
const at = (key, shot = S) => ({ shot, key });

test('the pick: the current card of the current shot – its file, url, kind, length and status', () => {
	assert.deepEqual(auditionPick({ shot: S, current: at(`${S}--g2`), records }), { id: `${S}--g2`, file: `${S}--g2.webm`, url: `/out/c/generated/${S}--g2.webm`, video: true, ms: 2000, status: 'review' });
	assert.deepEqual(auditionPick({ shot: S, current: at(`${S}--g4`), records }), { id: `${S}--g4`, file: `${S}--g4.png`, url: `/out/c/generated/${S}--g4.png`, video: false, ms: null, status: null });
	// the chosen as the current card: it plays, with its ✓
	assert.equal(auditionPick({ shot: S, current: at(`${S}--g9`), records }).status, 'chosen');
	// a length the file didn't give: the asked duration; a file without a record (earlier) is a result too
	assert.equal(auditionPick({ shot: S, current: at(`${S}--g8`), records }).ms, 6000);
});

test('no pick (the chosen plays): rejected, a source, a gone file, a running job, another shot, none clicked', () => {
	for (const key of [`${S}--g5`, `${S}--g6`, `${S}--g7`, `render:${S}`, `take:${S}:t1`, `variant:${S}:v1`, 'job:abc', `${S}--g99`]) {
		assert.equal(auditionPick({ shot: S, current: at(key), records }), null, key);
	}
	assert.equal(auditionPick({ shot: S, current: at(`sb-25-1--g1`, 'sb-25-1'), records }), null);
	assert.equal(auditionPick({ shot: S, current: null, records }), null);
	assert.equal(auditionPick({ shot: null, current: at(`${S}--g2`), records }), null);
	assert.equal(auditionPick({ shot: S, current: at(`${S}--g2`), records: undefined }), null);
	// a dropped file under a parent (the manual result slot) is a result – it can be auditioned
	const manual = [rec(6, { kind: 'video', ms: 3000, generator: 'upload', parents: [`${S}--g3`] })];
	assert.equal(auditionPick({ shot: S, current: at(`${S}--g6`), records: manual })?.id, `${S}--g6`);
});

test('the marks the board knows, by file – the chosen\'s ✓ comes from the clips data', () => {
	assert.deepEqual(marksOf(records), { [`${S}--g2.webm`]: 'review', [`${S}--g5.webm`]: 'rejected' });
	assert.deepEqual(marksOf([{ file: 'x.webm', approved: true, mark: 'review' }]), {});
	assert.deepEqual(marksOf(undefined), {});
});

test('the board\'s marks reach the lane only when the board read the log after the clips did – else the clips\' own (the log\'s)', () => {
	const marks = { [`${S}--g2.webm`]: 'review', [`${S}--g5.webm`]: 'rejected' };
	// the board loaded after the clips (a mark set on it): its marks
	assert.deepEqual(boardMarks(records, 200, 100), marks);
	assert.deepEqual(boardMarks(records, 100, 100), marks);
	// a board closed earlier keeps its records – clips asked since (another window's mark, one by hand) win: none
	assert.equal(boardMarks(records, 100, 200), null);
	// the lane then reads the clips' marks: g5 rejected in the log is no fallback although the board's old records say none
	const shot = { take: null, generated: [{ file: `${S}--g5.webm`, url: '/g5', ms: 4000, video: true, approved: false, mark: 'rejected' }, { file: `${S}--g2.webm`, url: '/g2', ms: 3000, video: true, approved: false, mark: 'review' }] };
	const stale = boardMarks([{ file: `${S}--g5.webm`, approved: false, mark: null }], 100, 200);
	assert.equal(clipOf(shot, 'generated', {}, stale ? { marks: stale } : null).url, '/g2');
	const fresh = boardMarks([{ file: `${S}--g5.webm`, approved: false, mark: null }], 300, 200);
	assert.equal(clipOf(shot, 'generated', {}, fresh ? { marks: fresh } : null).url, '/g5');
});

test('it holds while the board is open and while Play from it runs – not after Play\'s end, by hand or another step', () => {
	assert.deepEqual(holds({ open: true }), { on: true, carried: false });
	// Play switched the main to the Viewport: it holds while that Play runs
	assert.deepEqual(holds({ open: false, was: true, byPlay: true, playing: true }), { on: true, carried: true });
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: true }), { on: true, carried: true }, 'still playing');
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: false }), { on: false, carried: false }, 'Play ended (or paused)');
	assert.deepEqual(holds({ open: false, was: true, carried: true, playing: true, stepped: true }), { on: false, carried: false }, 'another step');
	assert.deepEqual(holds({ open: false, was: true, byPlay: false, playing: false }), { on: false, carried: false }, 'V / the switch / a source\'s Load');
	assert.deepEqual(holds({ open: false, was: false, byPlay: true, playing: true }), { on: false, carried: false }, 'Play from the Viewport: no audition');
	assert.deepEqual(holds({ open: false, was: false, playing: true }), { on: false, carried: false }, 'Play again after it ended: no audition');
	assert.deepEqual(holds({ open: true, carried: true, playing: true }), { on: true, carried: false }, 'V back to the board');
});

test('the board\'s current card: the last clicked there – none (or gone, or another shot\'s): the chosen, the video first', () => {
	assert.deepEqual(currentCard({ shot: S, current: at(`${S}--g3`), records }), at(`${S}--g3`));
	assert.deepEqual(currentCard({ shot: S, current: null, records }), at(`${S}--g9`), 'the chosen video');
	assert.deepEqual(currentCard({ shot: S, current: at('sb-25-1--g1', 'sb-25-1'), records }), at(`${S}--g9`), 'another shot\'s card');
	assert.deepEqual(currentCard({ shot: S, current: at(`${S}--g99`), records }), at(`${S}--g9`), 'gone from the board');
	const imageOnly = records.filter((r) => r.id !== `${S}--g9`);
	assert.deepEqual(currentCard({ shot: S, current: null, records: imageOnly }), at(`${S}--g1`), 'no chosen video: the chosen image');
	// a source as the current card stays the current card (no pick: the lane keeps its chosen)
	assert.deepEqual(currentCard({ shot: S, current: at(`render:${S}`), records }), at(`render:${S}`));
	assert.equal(currentCard({ shot: S, current: null, records: [rec(3, { kind: 'video' })] }), null, 'no chosen: none (Board: the render)');
	assert.equal(currentCard({ shot: null, current: at(`${S}--g3`), records }), null);
	// the board's own test of what is on it
	assert.deepEqual(currentCard({ shot: S, current: at(`${S}--g3`), records, has: () => false }), at(`${S}--g9`));
});

// --- the lane's override (timeline-lanes.js) ----------------------------------------------------------------------------
const clips = {
	'sb-25-1': { take: null, rendered: [], generated: [{ file: 'sb-25-1--g1.webm', url: '/g/a1.webm', ms: 3000, video: true, approved: true }] },
	[S]: {
		take: null,
		rendered: [],
		generated: [
			{ file: `${S}--g3.webm`, url: '/g/b3.webm', ms: 4000, video: true, approved: false },
			{ file: `${S}--g2.webm`, url: '/g/b2.webm', ms: 2000, video: true, approved: false },
			{ file: `${S}--g1.png`, url: '/g/b1.png', ms: null, video: false, approved: true }
		]
	},
	'sb-25-3': { take: null, rendered: [], generated: [] }
};
const ids = ['sb-25-1', S, 'sb-25-3'];

test('the override: the current shot\'s clip is the pick – the others and the later starts follow', () => {
	const plain = clipsLane(ids, 'generated', clips).lane.items;
	assert.deepEqual(plain.map((i) => [i.id, i.start, i.length, i.clip.url ?? null]), [['sb-25-1', 0, 3000, '/g/a1.webm'], [S, 3000, 4000, '/g/b3.webm'], ['sb-25-3', 7000, STILL_MS, null]]);
	assert.equal(plain[1].clip.gen.audition, false);
	const pick = auditionPick({ shot: S, current: at(`${S}--g2`), records });
	const { lane, span } = clipsLane(ids, 'generated', clips, {}, {}, { [S]: { pick } });
	assert.deepEqual(lane.items.map((i) => [i.id, i.start, i.length]), [['sb-25-1', 0, 3000], [S, 3000, 2000], ['sb-25-3', 5000, STILL_MS]]);
	assert.equal(span, 10000);
	const c = lane.items[1].clip;
	assert.deepEqual([c.source, c.url, c.file, c.gen], ['generated', `/out/c/generated/${S}--g2.webm`, `${S}--g2.webm`, { id: `${S}--g2`, status: 'review', audition: true }]);
	assert.equal(lane.items[1].label, `${S} · generated g2 · in review · audition`);
	// the others keep their chosen – the override is per shot
	assert.deepEqual(lane.items[0].clip.gen, { id: 'sb-25-1--g1', status: 'chosen', audition: false });
	assert.equal(clipAt(lane.items, 4500).item.id, S);
});

test('an auditioned image stays a still (5 s) that shows the image', () => {
	const pick = auditionPick({ shot: S, current: at(`${S}--g4`), records });
	const c = clipOf(clips[S], 'generated', {}, { pick });
	assert.deepEqual([c.source, c.length, c.image, c.url, c.probe], ['still', STILL_MS, `/out/c/generated/${S}--g4.png`, undefined, false]);
	assert.equal(labelOf(S, c), `${S} · generated image g4 · audition`);
	// a video pick of unknown length: probed like any video, 5 s until then
	const unknown = clipOf(clips[S], 'generated', {}, { pick: { ...auditionPick({ shot: S, current: at(`${S}--g2`), records }), ms: null } });
	assert.deepEqual([unknown.length, unknown.probe], [STILL_MS, true]);
});

test('the override counts only in Generated mode – the other modes keep their chain', () => {
	const pick = auditionPick({ shot: S, current: at(`${S}--g2`), records });
	for (const mode of ['shot', 'take', 'rendered']) assert.equal(clipOf(clips[S], mode, {}, { pick }).gen, undefined, mode);
});

test('the status bullets: ✓ the approved, the board\'s marks for the others, none unknown', () => {
	// the newest video (no chosen video) in review → ?
	assert.equal(clipOf(clips[S], 'generated', {}, { marks: marksOf(records) }).gen.status, null); // g3 has no mark
	const reviewNewest = { ...clips[S], generated: clips[S].generated.slice(1) };
	assert.equal(clipOf(reviewNewest, 'generated', {}, { marks: marksOf(records) }).gen.status, 'review');
	assert.equal(clipOf(reviewNewest, 'generated').gen.status, null); // the board's marks not known: no bullet
	assert.equal(clipOf(clips['sb-25-1'], 'generated').gen.status, 'chosen');
	// not a generated clip: no bullet
	assert.equal(clipOf(clips['sb-25-3'], 'generated').gen, undefined);
});

test('none clicked, the chosen image only: the lane auditions the board\'s current card (a still that shows it)', () => {
	const imageOnly = records.filter((r) => r.id !== `${S}--g9`);
	const pick = auditionPick({ shot: S, current: currentCard({ shot: S, current: null, records: imageOnly }), records: imageOnly });
	const c = clipOf(clips[S], 'generated', {}, { pick });
	assert.deepEqual([c.source, c.image, c.gen], ['still', `/out/c/generated/${S}--g1.png`, { id: `${S}--g1`, status: 'chosen', audition: true }]);
});

test('a video the board knows as rejected is no fallback – the next one, else the next source', () => {
	// no chosen video: the newest (g3) rejected → g2
	const c = clipOf(clips[S], 'generated', {}, { marks: { [`${S}--g3.webm`]: 'rejected' } });
	assert.deepEqual([c.url, c.gen.id], ['/g/b2.webm', `${S}--g2`]);
	// every video rejected: no generated clip – the still (5 s)
	const none = clipOf(clips[S], 'generated', {}, { marks: { [`${S}--g3.webm`]: 'rejected', [`${S}--g2.webm`]: 'rejected' } });
	assert.deepEqual([none.source, none.length, none.gen], ['still', STILL_MS, undefined]);
	// X on the auditioned card: no pick → the fallback, which skips it too
	const rejected = records.map((r) => (r.id === `${S}--g3` ? { ...r, mark: 'rejected' } : r));
	assert.equal(auditionPick({ shot: S, current: at(`${S}--g3`), records: rejected }), null);
	const item = clipsLane(ids, 'generated', clips, {}, {}, { [S]: { marks: marksOf(rejected) } }).lane.items[1];
	assert.equal(item.label, `${S} · generated g2 · in review`);
});
