// "Improve on this" · Vary · More on the Generate form (src/lib/editor/storyboard-improve.ts, #125 – the port of #62's
// improve-form.test.mjs) with the server's models (Luma uni-1, Runway gen4_turbo / aleph2, Luma by hand): a card as an
// input, which models take it, where it goes, the generator an Improve switches to, the line's code, why not, Vary's
// fields from the record, More ▾'s items, a frame's moment. Pure – nothing runs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cardOf, durationFor, fieldsFor, fits, frameSeconds, frameStillOf, howOf, parentsOf, pickImproveModel, refusal, takesSeed, useAsOf, varyFields, type ImproveFrom } from '../src/lib/editor/storyboard-improve.ts';
import type { BoardNode, BoardRecord } from '../src/lib/editor/storyboard-board-layout.ts';
import type { GeneratorInfo, GeneratorModel } from '../src/lib/editor/storyboard-generations.ts';

const price = { currency: 'USD' };
const UNI: GeneratorModel = { id: 'uni-1', label: 'uni-1', kind: 'image', mode: 'image', refs: { roles: ['still', 'reference', 'character', 'style'], max: 9 }, price };
const GEN4: GeneratorModel = { id: 'gen4_turbo', label: 'Gen-4 Turbo', kind: 'video', mode: 'image_to_video', refs: { roles: ['still'], max: 1, required: ['still'] }, durations: { min: 2, max: 10 }, price };
const ALEPH: GeneratorModel = { id: 'aleph2', label: 'Aleph 2', kind: 'video', mode: 'video_to_video', refs: { roles: ['control', 'still'], max: 2, required: ['control'] }, price };
const MANUAL: GeneratorModel = { id: 'manual', label: 'lumalabs.ai', kind: 'image', mode: 'manual', refs: { roles: ['still', 'reference', 'character', 'style'], max: 9 }, price };
const gen = (id: string, models: GeneratorModel[], configured = true): GeneratorInfo => ({ id, label: id, configured, setting: '', paid: true, unavailable: null, kinds: [], models });
const luma = gen('luma-image', [UNI]), runway = gen('runway', [GEN4, ALEPH]), manual = gen('luma-manual', [MANUAL]);
const gens = [luma, manual, runway];

const rec = (n: number, f: Partial<BoardRecord & { prompt: string; seed: number; duration: number; inputs: { role: string; work?: string; url?: string }[] }> = {}) =>
	({ id: n, shot: 's', kind: 'image', take: null, parents: [], generator: 'luma-image', model: 'uni-1', status: 'succeeded', approved: false, mark: null, file: `s--g${n}.png`, prompt: 'Film noir.', seed: 4410, ...f }) as BoardRecord & { prompt: string; seed: number };
const node = (r: BoardRecord | null, f: Partial<BoardNode> = {}) => ({ kind: 'result' as const, source: null, take: null, record: r, ...f });
const improve = (n: ReturnType<typeof node>): ImproveFrom => ({ ...cardOf(n)!, mode: 'improve', shot: 's', useSeed: false, switched: null, prev: null });

test('a card as an input: a result, a video, a dropped file, the render, a take', () => {
	const img = cardOf(node(rec(2, { parents: [1] })))!;
	assert.deepEqual([img.name, img.id, img.as, img.video, img.prompt, img.seed, img.parent], ['g2', 2, 'image', false, 'Film noir.', 4410, 1]);
	assert.deepEqual([cardOf(node(rec(3, { kind: 'video', file: 's--g3.mp4' })))!.as, cardOf(node(rec(3, { kind: 'video' })))!.video], ['video', true]);
	assert.equal(cardOf(node(rec(4, { generator: 'upload' }), { kind: 'source', source: 'upload' }))!.as, 'image', 'a dropped image goes in as an image');
	assert.deepEqual(cardOf(node(null, { kind: 'source', source: 'render' }))!.as, 'render');
	assert.deepEqual([cardOf(node(null, { kind: 'source', source: 'take', take: 't3' }))!.as, cardOf(node(null, { kind: 'source', source: 'take', take: 't3' }))!.name], ['take', 't3']);
	assert.equal(cardOf(null), null);
});

test('which models take it: an image → a still (not one that needs a video), a video → video to video, the render → any', () => {
	const ids = (from: ImproveFrom) => gens.flatMap((g) => g.models).filter((m) => fits(from, m)).map((m) => m.id);
	assert.deepEqual(ids(improve(node(rec(2)))), ['uni-1', 'manual', 'gen4_turbo']);
	assert.deepEqual(ids(improve(node(rec(3, { kind: 'video' })))), ['aleph2']);
	assert.equal(ids(improve(node(null, { kind: 'source', source: 'render' }))).length, 4);
	assert.equal(ids({ ...improve(node(rec(2))), mode: 'vary' }).length, 4, 'Vary: its own model – no filter');
});

test('the generator an Improve starts with: the current one if it fits, else the first that can run, by hand last', () => {
	const video = improve(node(rec(3, { kind: 'video' })));
	assert.deepEqual(pickImproveModel(gens, { generator: 'runway', model: 'gen4_turbo' }, video), { generator: 'runway', model: 'aleph2', switched: false });
	assert.deepEqual(pickImproveModel(gens, { generator: 'luma-image', model: 'uni-1' }, video), { generator: 'runway', model: 'aleph2', switched: true });
	assert.equal(pickImproveModel([luma, manual], { generator: 'luma-image', model: 'uni-1' }, video), null, 'none takes a video');
	const image = improve(node(rec(2)));
	assert.deepEqual(pickImproveModel(gens, { generator: 'runway', model: 'aleph2' }, image), { generator: 'runway', model: 'gen4_turbo', switched: false }, 'the generator\'s own that fits');
	assert.deepEqual(pickImproveModel([gen('luma-image', [UNI], false), manual], { generator: 'luma-image', model: 'uni-1' }, image), { generator: 'luma-image', model: 'uni-1', switched: false }, 'the current fits – kept (the panel says it is not set up)');
	assert.deepEqual(pickImproveModel([gen('luma-image', [UNI], false), manual, runway], { generator: 'x', model: 'y' }, image), { generator: 'runway', model: 'gen4_turbo', switched: true }, 'one that can run before one that can\'t, by hand');
	assert.deepEqual(pickImproveModel([gen('luma-image', [UNI], false), manual], { generator: 'x', model: 'y' }, image), { generator: 'luma-manual', model: 'manual', switched: true }, 'by hand before one that can\'t run');
});

test('where the card goes: the still, Aleph\'s video + the chosen keyframe, the render nothing, a still set by hand', () => {
	const image = improve(node(rec(2)));
	assert.deepEqual(fieldsFor(image, GEN4), { still: 2 });
	assert.deepEqual(fieldsFor(image, UNI), { still: 2 });
	const video = improve(node(rec(3, { kind: 'video' })));
	assert.deepEqual(fieldsFor(video, ALEPH, 1), { control: 3, still: 1 });
	assert.deepEqual(fieldsFor(video, ALEPH), { control: 3, still: null });
	assert.deepEqual(fieldsFor(video, GEN4), {}, 'a still model: nothing (More ▾ cuts a frame)');
	assert.deepEqual(fieldsFor(improve(node(null, { kind: 'source', source: 'render' })), GEN4), {});
	assert.deepEqual(fieldsFor({ ...image, still: 7 }, GEN4), { still: 7 });
	assert.deepEqual(fieldsFor({ ...video, still: 9 }, ALEPH, 1), { control: 3, still: 9 });
	const vary: ImproveFrom = { ...image, mode: 'vary', fields: { still: 5, control: 6, characters: ['c/k'] } };
	assert.deepEqual(fieldsFor(vary, ALEPH), { still: 5, control: 6, characters: ['c/k'] });
	assert.deepEqual(fieldsFor(vary, UNI), { still: 5, control: null, characters: ['c/k'] });
	assert.deepEqual(fieldsFor(null, GEN4), {});
	assert.deepEqual(parentsOf({ control: 3, still: 1 }), [3, 1], 'the main input first');
	assert.deepEqual(parentsOf({ still: 2, control: null }), [2]);
	assert.deepEqual(parentsOf({ still: 2, control: 2 }), [2]);
});

test('the line\'s code', () => {
	const image = improve(node(rec(2)));
	assert.equal(howOf(image, GEN4), 'start');
	assert.equal(howOf(image, UNI), 'edit');
	assert.equal(howOf(image, MANUAL), 'reference');
	assert.equal(howOf(image, ALEPH), 'none');
	const video = improve(node(rec(3, { kind: 'video' })));
	assert.equal(howOf(video, ALEPH, 1), 'video_key');
	assert.equal(howOf(video, ALEPH), 'video_nokey');
	assert.equal(howOf({ ...video, still: 4 }, ALEPH), 'video_key', 'a keyframe set here');
	assert.equal(howOf(video, UNI), 'none');
	assert.equal(howOf(improve(node(null, { kind: 'source', source: 'render' })), GEN4), 'own');
	assert.equal(howOf({ ...image, still: 7 }, GEN4), 'set');
	assert.equal(howOf({ ...image, mode: 'vary' }, GEN4), 'vary');
	assert.equal(howOf(null, GEN4), null);
});

test('why not: rejected, running, a source, a dropped file, a generator / model gone', () => {
	assert.equal(refusal(node(rec(2, { mark: 'rejected' })), 'improve'), 'rejected');
	assert.equal(refusal(node(rec(2)), 'improve'), '');
	assert.equal(refusal(node(null, { kind: 'source', source: 'render' }), 'improve'), '');
	assert.equal(refusal(node(null, { kind: 'source', source: 'render' }), 'vary'), 'source');
	assert.equal(refusal(node(null, { kind: 'source', source: 'render' }), 'use'), 'source');
	assert.equal(refusal(node(null, { kind: 'running' }), 'improve'), 'running');
	assert.equal(refusal(node(rec(4, { generator: 'upload' })), 'vary'), 'dropped');
	assert.equal(refusal(node(rec(4, { generator: 'frame' })), 'vary'), 'dropped');
	assert.equal(refusal(node(rec(4, { generator: 'comfy-local' })), 'vary', gens), 'generator');
	assert.equal(refusal(node(rec(4, { model: 'uni-0' })), 'vary', gens), 'model');
	assert.equal(refusal(node(rec(4)), 'vary', gens), '');
});

test('Vary\'s fields from the record: work files → the shot\'s generations, characters by their url', () => {
	const inputs = [{ role: 'control', work: 's--g3.mp4' }, { role: 'still', work: 's--g1.png' }, { role: 'character', url: 'https://cdn/x.png' }, { role: 'character', url: 'https://cdn/gone.png' }, { role: 'still', work: 's--g9.png' }];
	const shotGens = [{ id: 1, work_path: 's--g1.png' }, { id: 3, work_path: 's--g3.mp4' }];
	assert.deepEqual(varyFields(inputs, shotGens, [{ key: 'cat/regular/neutral/full', url: 'https://cdn/x.png' }]), { still: 1, control: 3, characters: ['cat/regular/neutral/full'] });
	assert.deepEqual(varyFields(null, shotGens), { still: null, control: null, characters: [] });
});

test('More ▾ and the rest: start frame / keyframe, a frame as the still, its moment, the length, the seed', () => {
	assert.equal(useAsOf(GEN4), 'start');
	assert.equal(useAsOf(ALEPH), 'keyframe');
	assert.equal(useAsOf(UNI), null);
	assert.equal(useAsOf(MANUAL), null);
	assert.equal(useAsOf(null), null);
	assert.ok(frameStillOf(GEN4) && frameStillOf(UNI) && frameStillOf(MANUAL) && !frameStillOf(null));
	assert.equal(frameSeconds('first', 5), 0);
	assert.equal(frameSeconds('last', 5), 4.95);
	assert.equal(frameSeconds('last', NaN), 0);
	assert.equal(frameSeconds(1500, 5), 1.5);
	assert.equal(frameSeconds(9000, 5), 4.95, 'past the end: the last frame');
	assert.equal(frameSeconds(1500, null), 1.5);
	assert.equal(durationFor(GEN4, 2), 2);
	assert.equal(durationFor(GEN4, 12), 10);
	assert.equal(durationFor(GEN4, 0.4), 2);
	assert.equal(durationFor(GEN4, null), null);
	assert.equal(durationFor(ALEPH, 5), null, 'Aleph takes its video\'s length');
	assert.ok(takesSeed(GEN4) && takesSeed(ALEPH) && !takesSeed(UNI) && !takesSeed(MANUAL));
});

test('the job names its generated inputs as parents (the main input first) and sends a card\'s seed only to a model that takes one', async () => {
	const { jobBody, pickedParents } = await import('../src/lib/editor/storyboard-generate.ts');
	const g = (id: number) => ({ id }) as never;
	assert.deepEqual(pickedParents([{ role: 'control', gen: g(3) }, { role: 'still', gen: g(1) }, { role: 'character', url: 'https://x' }]), [3, 1]);
	const at = { toon: 'catpope', chapter: 3, shot: 's' };
	const form = { generator: 'runway', model: 'aleph2', prompt: '', still: 1, control: 3, duration: 5, characters: [], seed: 4410 };
	assert.deepEqual(jobBody(at, form, ALEPH, [], [3, 1]), { ...at, generator: 'runway', model: 'aleph2', params: { seed: 4410 }, parents: [3, 1] });
	assert.deepEqual(jobBody(at, { ...form, generator: 'luma-image', model: 'uni-1' }, UNI, []), { ...at, generator: 'luma-image', model: 'uni-1' }, 'Luma: no seed');
	assert.deepEqual(jobBody(at, { ...form, seed: null }, GEN4, []).params, { duration: 5 });
});
