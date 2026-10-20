// "Improve on this" · Vary · More on the Generate form (src/lib/improve.js, agents/research/VARIANTS-CANVAS.md §3.6;
// issue #62 stage 3) with the real models (runway.mjs, comfy-local.mjs, luma-manual.mjs, the fake ones): a card as an
// input, which models take it, where it goes for each model, the generator an Improve switches to, the line's words,
// ComfyUI's stills and its pre-fill (the render), More ▾'s items, the words for a frame at a time. Pure – nothing runs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MODELS as RUNWAY } from '../generators/runway.mjs';
import { createComfyLocal } from '../generators/comfy-local.mjs';
import { createLumaManual } from '../generators/luma-manual.mjs';
import { createFake } from '../generators/others.mjs';
import { fieldsOf } from '../generators/job-fields.mjs';
import { cardOf, chosenImage, defaultStill, durationFor, editable, fieldsFor, fits, frameStillOf, howText, pickModel, pinChosen, stillLabel, takesStill, useAsOf } from '../src/lib/improve.js';

const S = 'sb-25-2', CH = 's1c3';
const AT = { ch: CH, shot: S };
const G = (f) => `out:${CH}/generated/${S}--${f}`;
const RENDER = `out:${CH}/clean/${S}.png`;
const CHOSEN = `boards:dataset/files/${CH}/${S}/chosen.png`;
const model = (id) => [...RUNWAY, ...gens.flatMap((g) => g.models)].find((m) => m.id === id);
const comfy = { ...createComfyLocal({ root: '.' }), configured: false };
const luma = { ...createLumaManual(), configured: true };
const fake = { ...createFake(), configured: true };
const runway = { id: 'runway', label: 'Runway', models: RUNWAY, configured: true };
const gens = [comfy, runway, luma, fake];
const rec = (n, f = {}) => ({ id: `${S}--g${n}`, file: `${S}--g${n}.png`, key: G(`g${n}.png`), url: `/out/${CH}/generated/${S}--g${n}.png`, kind: 'image', prompt: 'Film noir.', seed: 4410, parents: [], inputs: [{ role: 'still', key: RENDER }], ...f });
const improve = (node, take) => ({ mode: 'improve', ...cardOf(node, { ...AT, take }) });

test('a card as an input: a result, a video, a dropped file, the render, a layout variant, a take', () => {
	const img = cardOf({ kind: 'result', record: rec(2, { parents: [`${S}--g1`] }) }, AT);
	assert.deepEqual([img.name, img.id, img.as, img.key, img.video, img.prompt, img.seed, img.parent], ['g2', `${S}--g2`, 'image', G('g2.png'), false, 'Film noir.', 4410, `${S}--g1`]);
	const vid = cardOf({ kind: 'result', record: rec(3, { kind: 'video', file: `${S}--g3.webm`, key: undefined }) }, AT);
	assert.deepEqual([vid.as, vid.key, vid.video], ['video', G('g3.webm'), true], 'no key from the server: made from the file');
	assert.equal(cardOf({ kind: 'source', source: 'upload', record: rec(4, { generator: 'upload' }) }, AT).as, 'image', 'a dropped image goes in as an image');
	const render = cardOf({ kind: 'source', source: 'render' }, AT);
	assert.deepEqual([render.name, render.as, render.key, render.id], ['the render', 'render', RENDER, null]);
	const v = cardOf({ kind: 'source', source: 'variant', variant: { id: 'v1' } }, AT);
	assert.deepEqual([v.name, v.as, v.key, v.url], ['v1', 'image', `out:${CH}/variants/${S}--v1.png`, `/out/${CH}/variants/${S}--v1.png`]);
	const t = cardOf({ kind: 'source', source: 'take', take: 't3' }, { ...AT, take: { url: `/out/${CH}/handheld/${S}--t3--beauty.mp4?v=4` } });
	assert.deepEqual([t.name, t.as, t.key, t.video], ['take t3', 'take', `out:${CH}/handheld/${S}--t3--beauty.mp4`, true]);
	assert.equal(cardOf({ kind: 'source', source: 'take', take: 't3' }, AT), null, 'a take without its render: nothing goes in');
	assert.equal(cardOf({ kind: 'result', record: { id: `${S}--g9` } }, AT), null, 'a record without a file');
});

test('which models take it: an image → first frame / edit / by hand, a video → video to video only, the render → any', () => {
	const ids = (from) => gens.flatMap((g) => g.models).filter((m) => fits(from, m, AT)).map((m) => m.id);
	assert.deepEqual(ids(improve({ record: rec(2) })), ['qwen-image-edit', 'gen4_turbo', 'gen4.5', 'manual', 'fake-1'], 'a generated image: ComfyUI edits it');
	assert.deepEqual(ids(improve({ source: 'variant', variant: { id: 'v1' } })), ['gen4_turbo', 'gen4.5', 'manual', 'fake-1'], 'a layout frame: ComfyUI can\'t edit it');
	assert.deepEqual(ids(improve({ record: rec(3, { kind: 'video' }) })), ['aleph2', 'fake-v2v'], 'a video: Aleph\'s video – not by hand (a frame of it is More ▾\'s)');
	assert.equal(ids(improve({ source: 'render' })).length, 7, 'the render: the shot\'s own inputs – any');
	assert.equal(ids({ mode: 'vary', key: G('g3.webm') }).length, 7, 'Vary: its own model – no filter');
	assert.equal(ids(null).length, 7);
});

test('the generator an Improve starts with: the current one if it fits, else the first (configured) that has one', () => {
	const video = improve({ record: rec(3, { kind: 'video' }) });
	assert.deepEqual(pickModel(gens, { generator: 'runway', model: 'gen4_turbo' }, video, AT), { generator: 'runway', model: 'aleph2', switched: false }, 'the generator\'s own model that fits');
	assert.deepEqual(pickModel(gens, { generator: 'comfy-local', model: 'qwen-image-edit' }, video, AT), { generator: 'runway', model: 'aleph2', switched: true }, 'ComfyUI takes no video: Runway (configured)');
	assert.deepEqual(pickModel([{ ...runway, configured: false }, fake], { generator: 'comfy-local' }, video, AT), { generator: 'fake', model: 'fake-v2v', switched: true }, 'a configured one first');
	assert.deepEqual(pickModel([comfy, { ...runway, configured: false }, luma, fake], { generator: 'comfy-local' }, video, AT), { generator: 'fake', model: 'fake-v2v', switched: true }, 'one that makes it before Luma by hand');
	assert.deepEqual(pickModel([comfy, { ...runway, configured: false }, luma], { generator: 'comfy-local' }, video, AT), { generator: 'runway', model: 'aleph2', switched: true }, 'a video never goes by hand: Aleph, even unconfigured (the panel says so)');
	assert.deepEqual(pickModel([comfy, { ...runway, configured: false }, luma], { generator: 'comfy-local' }, improve({ source: 'variant', variant: { id: 'v1' } }), AT), { generator: 'luma-manual', model: 'manual', switched: true }, 'an image: by hand when nothing configured makes it');
	const image = improve({ record: rec(2) });
	assert.deepEqual(pickModel(gens, { generator: 'runway', model: 'gen4.5' }, image, AT), { generator: 'runway', model: 'gen4.5', switched: false }, 'the current model fits: kept');
	assert.deepEqual(pickModel(gens, { generator: 'comfy-local', model: 'qwen-image-edit' }, image, AT), { generator: 'comfy-local', model: 'qwen-image-edit', switched: false });
	assert.equal(pickModel([comfy], { generator: 'comfy-local' }, video, AT), null, 'none takes it');
});

test('where the card goes for each model: the still, Aleph\'s video + the chosen keyframe, a frame by hand, a take', () => {
	const image = improve({ record: rec(2) });
	assert.deepEqual(fieldsFor(image, model('gen4_turbo')), { still: G('g2.png') });
	assert.deepEqual(fieldsFor(image, model('qwen-image-edit')), { still: G('g2.png') }, 'ComfyUI: image 1 (--from)');
	const video = improve({ record: rec(3, { kind: 'video', key: G('g3.mp4') }) });
	assert.deepEqual(fieldsFor(video, model('aleph2'), { chosen: CHOSEN }), { control: G('g3.mp4'), still: CHOSEN }, 'Aleph: the video + the chosen image at 0');
	assert.deepEqual(fieldsFor(video, model('aleph2')), { control: G('g3.mp4'), still: '' }, 'no chosen image: no keyframe');
	assert.deepEqual(fieldsFor(video, model('gen4_turbo')), { still: `frame:${G('g3.mp4')}#first` }, 'a still model (Improve offers none for a video – fits): its first frame');
	const take = improve({ source: 'take', take: 't1' }, { url: `/out/${CH}/handheld/${S}--t1--beauty.mp4` });
	assert.deepEqual(fieldsFor(take, model('aleph2')), { control: `out:${CH}/handheld/${S}--t1--beauty.mp4` }, 'a take: its render as the video');
	assert.deepEqual(fieldsFor(take, model('gen4_turbo')), {}, 'an image to video model: the shot\'s own');
	assert.deepEqual(fieldsFor(improve({ source: 'render' }), model('gen4_turbo')), {}, 'the render: the shot\'s own inputs');
	const r = rec(5, { kind: 'video', inputs: [{ role: 'control', key: G('g3.mp4') }, { role: 'keyframe', key: G('g3.mp4'), frame: 1500, at: 0 }, { role: 'character', key: 'media:cast/cat.png' }] });
	assert.deepEqual(fieldsFor({ mode: 'vary', fields: fieldsOf(r.inputs) }, model('aleph2')), { still: `frame:${G('g3.mp4')}#1500`, control: G('g3.mp4'), characters: ['media:cast/cat.png'], style: [] }, 'Vary: its own inputs – a frame as its frame key');
	assert.deepEqual(fieldsFor(null, model('aleph2')), {});
});

test('the line\'s words', () => {
	const image = improve({ record: rec(2) });
	assert.equal(howText(image, model('gen4_turbo')), 'g2 as the first frame');
	assert.equal(howText(image, model('qwen-image-edit')), 'g2 as image 1 – an edit');
	assert.equal(howText(image, model('manual')), 'g2 as the reference to attach');
	const video = improve({ record: rec(3, { kind: 'video' }) });
	assert.equal(howText(video, model('aleph2'), { chosen: CHOSEN }), 'g3 as the video – the chosen image as the keyframe at 0');
	assert.equal(howText(video, model('aleph2')), 'g3 as the video – no keyframe (no chosen image)');
	assert.equal(howText(video, model('manual')), 'g3\'s first frame as the reference');
	assert.equal(howText(improve({ source: 'render' }), model('gen4_turbo')), 'the shot\'s own inputs');
	assert.match(howText({ mode: 'vary', name: 'g2' }, model('gen4_turbo')), /own inputs, prompt and model/);
});

test('ComfyUI\'s stills: the render or a generated image of the shot – its pre-fill the render, not the chosen', () => {
	const q = model('qwen-image-edit');
	for (const k of [RENDER, G('g2.png'), G('g4.webp'), CHOSEN]) assert.ok(editable(k, AT) && takesStill(q, k, AT), k);
	for (const k of [`out:${CH}/frames/${S}.png`, `out:${CH}/variants/${S}--v1.png`, `frame:${G('g3.mp4')}#first`, G('g3.mp4'), `out:${CH}/generated/sb-9--g1.png`, `out:x/generated/${S}--g2.png`]) assert.ok(!takesStill(q, k, AT), k);
	assert.ok(takesStill(model('gen4_turbo'), `frame:${G('g3.mp4')}#first`, AT), 'a first frame model takes any still');
	assert.ok(!takesStill({ inputs: ['control'] }, RENDER, AT));
	const stills = [{ key: CHOSEN }, { key: RENDER }, { key: `out:${CH}/frames/${S}.png` }];
	assert.equal(defaultStill(q, stills, AT), RENDER, 'ComfyUI: the render (the chosen would edit its original)');
	assert.equal(defaultStill(model('gen4_turbo'), stills, AT), CHOSEN, 'else the first: the chosen image');
	assert.equal(defaultStill(model('gen4_turbo'), [], AT), '');
	assert.equal(defaultStill(q, [{ key: `out:${CH}/frames/${S}.png` }], AT), '', 'nothing ComfyUI takes');
	assert.equal(defaultStill(q, [{ key: CHOSEN }, { key: G('g2.png') }], AT), '', 'no render: none (comfy.mjs\'s own pass) – never an edit of an earlier result');
	assert.equal(defaultStill(model('manual'), stills, AT), RENDER, 'by hand: the render – what its panel says to attach');
	assert.equal(defaultStill(model('manual'), [{ key: CHOSEN }], AT), '', 'by hand without a render: none');
	assert.equal(chosenImage(stills), CHOSEN);
	assert.equal(chosenImage([{ key: RENDER }, { key: `boards:dataset/files/${CH}/${S}/chosen.mp4` }]), null, 'the chosen video is no keyframe');
});

test('More ▾: Use as start frame / keyframe where the model takes it, a frame as the still, a frame\'s words', () => {
	assert.deepEqual(useAsOf(model('gen4_turbo')), { label: 'Use as start frame', ok: true, why: '' });
	assert.deepEqual(useAsOf(model('aleph2')), { label: 'Use as keyframe (at 0)', ok: true, why: '' });
	const no = useAsOf(model('qwen-image-edit'), 'ComfyUI (this PC)');
	assert.ok(!no.ok && /ComfyUI \(this PC\) takes no start frame or keyframe/.test(no.why));
	assert.ok(!useAsOf(model('manual')).ok, 'by hand: the reference, no start frame');
	assert.ok(frameStillOf(model('gen4_turbo')).ok && frameStillOf(model('aleph2')).ok && frameStillOf(model('manual')).ok);
	assert.match(frameStillOf(model('qwen-image-edit'), 'ComfyUI').why, /not a frame of a video/);
	assert.match(frameStillOf({ inputs: ['control'] }, 'X').why, /takes no still/);
	assert.equal(stillLabel(`frame:${G('g3.webm')}#1500`, S), 'Generated g3.webm – the frame at 1.5 s');
	assert.equal(stillLabel(`frame:${G('g3.webm')}#last`, S), 'Generated g3.webm – the last frame');
	assert.equal(stillLabel(G('g2.png'), S), 'Generated g2.png');
	assert.equal(stillLabel('media:cast/cat.png', S), 'cat.png');
});

test('a still set by hand over the card\'s (from.still): it stays where the model takes it, and the line names it', () => {
	const label = (k) => stillLabel(k, S);
	const image = { ...improve({ record: rec(1) }), still: G('g3.png') };
	assert.deepEqual(fieldsFor(image, model('gen4_turbo'), { at: AT }), { still: G('g3.png') });
	assert.equal(howText(image, model('gen4_turbo'), { at: AT, label }), 'Generated g3.png as the first frame, set here – g1 is not in the form now');
	assert.deepEqual(fieldsFor({ ...image, still: `frame:${G('g2.webm')}#first` }, model('qwen-image-edit'), { at: AT }), { still: G('g1.png') }, 'a model that can\'t take it: the card\'s');
	const video = { ...improve({ record: rec(2, { kind: 'video', key: G('g2.webm') }) }), still: `frame:${G('g2.webm')}#500` };
	assert.deepEqual(fieldsFor(video, model('fake-v2v'), { chosen: CHOSEN, at: AT }), { control: G('g2.webm'), still: `frame:${G('g2.webm')}#500` }, 'the video stays the control');
	assert.equal(howText(video, model('fake-v2v'), { chosen: CHOSEN, at: AT, label }), 'g2 as the video – Generated g2.webm – the frame at 0.5 s as the keyframe at 0, set here');
	assert.equal(howText({ ...video, still: '' }, model('fake-v2v'), { chosen: CHOSEN, at: AT, label }), 'g2 as the video – no keyframe at 0, set here');
	assert.equal(howText({ ...improve({ source: 'render' }), still: CHOSEN }, model('gen4_turbo'), { at: AT, label: () => 'the chosen image' }), 'the shot\'s own inputs – the chosen image as the first frame, set here');
	const vary = { mode: 'vary', name: 'g4', fields: { still: G('g1.png') }, still: G('g3.png') };
	assert.equal(howText(vary, model('gen4_turbo'), { at: AT, label }), 'its prompt and model – Generated g3.png as the first frame, set here instead of its own');
	assert.match(howText({ mode: 'vary', name: 'g4', unpinned: true }, model('gen4_turbo')), /the chosen image as it is NOW/);
});

test('Vary pins the chosen image to the file it was when the card was made (its parent), its length within the model\'s', () => {
	const records = [rec(1), rec(2, { kind: 'video', file: `${S}--g2.mp4` }), rec(3), rec(5, { missing: true })];
	// g9 = Aleph on g2 with the chosen image (g1 then) as its keyframe: parents [g2, g1]
	const inputs = [{ role: 'control', key: G('g2.mp4') }, { role: 'keyframe', key: CHOSEN, at: 0 }];
	const p = pinChosen(inputs, [`${S}--g2`, `${S}--g1`], records, CH);
	assert.deepEqual(p, { inputs: [inputs[0], { role: 'keyframe', key: G('g1.png'), at: 0 }], gone: [], unpinned: false });
	assert.equal(fieldsOf(p.inputs).still, G('g1.png'), 'the same bytes as then – whatever is approved now');
	// a still made from the chosen image alone (g3 then)
	assert.deepEqual(pinChosen([{ role: 'still', key: CHOSEN }], [`${S}--g3`], records, CH).inputs, [{ role: 'still', key: G('g3.png') }]);
	// its file gone: left out, said
	assert.deepEqual(pinChosen([{ role: 'still', key: CHOSEN }], [`${S}--g5`], records, CH), { inputs: [], gone: ['g5'], unpinned: false });
	assert.deepEqual(pinChosen([{ role: 'still', key: CHOSEN }], [`${S}--g7`], records, CH).gone, ['g7'], 'no record of it any more');
	// no parent known (the chosen wasn't a generated file then): the alias stays – said
	assert.deepEqual(pinChosen([{ role: 'still', key: CHOSEN }], [], records, CH), { inputs: [{ role: 'still', key: CHOSEN }], gone: [], unpinned: true });
	// nothing to pin: as it was
	assert.deepEqual(pinChosen([{ role: 'still', key: G('g3.png') }], [`${S}--g3`], records, CH), { inputs: [{ role: 'still', key: G('g3.png') }], gone: [], unpinned: false });
	assert.deepEqual(pinChosen(undefined, undefined, records, CH), { inputs: [], gone: [], unpinned: false });
	// the length: the card's, within the model's (Gen-4 Turbo 2 … 10 s); a model without a length: none
	assert.equal(durationFor(model('gen4_turbo'), 2), 2);
	assert.equal(durationFor(model('gen4_turbo'), 12), 10);
	assert.equal(durationFor(model('gen4_turbo'), 0.4), 2);
	assert.equal(durationFor(model('gen4_turbo'), null), null);
	assert.equal(durationFor(model('aleph2'), 5), null, 'Aleph takes the length of its video');
});
