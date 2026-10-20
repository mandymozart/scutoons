// A shot's handheld takes (src/lib/editor/storyboard-takes.ts, #126): ids, the stored samples (thinned, rounded, t from
// 0), the changes of a shot (on, add, active, remove, smooth / scale, trim), the take on the timeline (ms), the record of
// a rendered take.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Shot, TakeSample } from '@scutoons/content/storyboard';
import { activeTake, addTake, newTake, nextTakeId, removeTake, renderRecord, setActive, setOn, setTakeField, setTrim, sha256, shotTake, takeIn, takesOf, thinSamples } from '../src/lib/editor/storyboard-takes.ts';

const sample = (t: number): TakeSample => ({ t, position: [t, 1.5, 0], quaternion: [0, 0, 0, 1], fov: 50 });
const run = (shot: Shot, ...changes: ((s: Shot) => void)[]) => (changes.forEach((c) => c(shot)), shot);
const shot = (): Shot => ({ id: 'sb-1', format: 'viewport', aspect: '1:1.6', action: {} }) as Shot;

test('ids: t1 after none, after the highest', () => {
	assert.equal(nextTakeId([]), 't1');
	assert.equal(nextTakeId([{ id: 't1' }, { id: 't4' }, { id: 't2' }]), 't5');
	assert.deepEqual(takesOf(undefined), { on: false, active: null, takes: [] });
});

test('stored samples: at most 30 Hz, t from 0, the last one kept, rounded', () => {
	const sixty = Array.from({ length: 61 }, (_, i) => sample(100 + i / 60));
	const kept = thinSamples(sixty);
	assert.equal(kept[0].t, 0);
	assert.equal(kept.at(-1)!.t, 1);
	assert.ok(kept.length >= 30 && kept.length <= 32, String(kept.length));
	assert.ok(kept.every((s, i) => i === 0 || s.t - kept[i - 1].t > 0.03 || i === kept.length - 1));
	assert.equal(newTake([], [sample(5)], { smooth: 0, scale: 1 }), null, 'one sample is no take');
	const t = newTake([{ id: 't1' }], sixty, { smooth: 0.2, scale: 0.5, now: '2026-10-21T10:00:00Z' })!;
	assert.deepEqual([t.id, t.in, t.out, t.smooth, t.scale, t.created], ['t2', 0, 1, 0.2, 0.5, '2026-10-21T10:00:00Z']);
});

test('the changes of a shot: on, a take added = active, another active, removed → the newest left', () => {
	const a = newTake([], [sample(0), sample(2)], { smooth: 0, scale: 1 })!;
	const s = run(shot(), setOn(true), addTake(a));
	const b = newTake(s.takes!.takes, [sample(0), sample(3)], { smooth: 0, scale: 1 })!;
	run(s, addTake(b));
	assert.deepEqual([s.takes!.on, s.takes!.active, s.takes!.takes.map((t) => t.id)], [true, 't2', ['t1', 't2']]);
	run(s, setActive('t1'), setActive('t9'));
	assert.equal(activeTake(s.takes)!.id, 't1');
	run(s, setTakeField('t1', 'smooth', 0.4), setTakeField('t1', 'scale', 2));
	assert.deepEqual([s.takes!.takes[0].smooth, s.takes!.takes[0].scale], [0.4, 2]);
	run(s, removeTake('t1'));
	assert.deepEqual([s.takes!.active, s.takes!.takes.map((t) => t.id)], ['t2', ['t2']]);
	run(s, removeTake('t2'));
	assert.equal(s.takes!.active, null);
});

test('the trim: ms in, s stored, clamped to the take', () => {
	const s = run(shot(), setOn(true), addTake(newTake([], [sample(0), sample(6)], { smooth: 0, scale: 1 })!));
	run(s, setTrim('t1', 1200.4, 4800));
	assert.deepEqual([s.takes!.takes[0].in, s.takes!.takes[0].out], [1.2, 4.8]);
	run(s, setTrim('t1', -50, 99999));
	assert.deepEqual([s.takes!.takes[0].in, s.takes!.takes[0].out], [0, 6]);
	run(s, setTrim('t1', 5000, 3000));
	assert.deepEqual([s.takes!.takes[0].in, s.takes!.takes[0].out], [5, 5], 'the out never before the in');
});

test('the take on the timeline (ms) – only the active one, only with the camera on', () => {
	const s = run(shot(), setOn(true), addTake(newTake([], [sample(0), sample(6.01)], { smooth: 0, scale: 1 })!), setTrim('t1', 1200, 4800));
	assert.deepEqual(shotTake(s), { id: 't1', in: 1200, out: 4800, length: 6010, kept: 3600 });
	run(s, setOn(false));
	assert.equal(shotTake(s), null);
	assert.equal(takeIn(null), null);
	assert.deepEqual(takeIn({ id: 't3', created: '', smooth: 0, scale: 1, in: undefined as unknown as number, out: undefined as unknown as number, samples: [sample(0), sample(2)] }), { id: 't3', in: 0, out: 2000, length: 2000, kept: 2000 });
});

test('a rendered take\'s record: kind video, generator handheld-render, the take, file + hash + machine', async () => {
	const take = newTake([], [sample(0), sample(2)], { smooth: 0.1, scale: 1 })!;
	const r = renderRecord({ toon: 'catpope', chapter: 3, shot: 'sb-1', take, file: 'sb-1--g4.webm', hash: 'ab'.repeat(32), machine: 'pc', fps: 24, seconds: 2.0001, ratio: '1:1.6' });
	assert.deepEqual([r.kind, r.generator, r.take, r.file, r.machine, r.duration, 'ratio' in r], ['video', 'handheld-render', 't1', 'sb-1--g4.webm', 'pc', 2, false]);
	assert.equal(renderRecord({ toon: 'catpope', chapter: 3, shot: 'sb-1', take, file: 'x.webm', hash: '', machine: 'pc', fps: 24, seconds: 1, ratio: '9:16' }).ratio, '9:16');
	assert.equal(await sha256(new Blob(['abc'])), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
});
