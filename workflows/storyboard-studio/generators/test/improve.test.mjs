// "Improve on this" on the server (VARIANTS-CANVAS §3.6, §3.7 – issue #62 stage 3): the job's fields ↔ the record's
// inputs (job-fields.mjs – a frame as its video's key + `frame`, Vary's fieldsOf), what a shot offers (generated images
// as stills, generated videos as Aleph's video and their first / last frames as stills, layout variants' frames),
// POST /api/generate/frame, jobs whose input is a card (a frame, a generated video, the chosen) → the record's inputs +
// parents, comfy-local's --from (a stub instead of comfy.mjs – no ComfyUI, no GPU), Runway's estimate for a generated
// clip (local only – nothing is sent), the manual result slot's record. ffmpeg is a stub here (frames.test.mjs).
import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { generatorRoutes } from '../index.mjs';
import { createFrames } from '../frames.mjs';
import { fieldsOf, frameKey, inputsOf, parseFrameKey } from '../job-fields.mjs';
import { generatedRoutes } from '../../generated.mjs';
import { tinyMp4 } from './runway-mock.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-improve-'));
const boards = join(tmp, 'boards'), out = join(boards, '_out');
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==', 'base64');
const FRAME_PNG = Buffer.concat([PNG, Buffer.from('frame')]);
const put = (rel, bytes) => { const f = join(boards, rel); mkdirSync(dirname(f), { recursive: true }); writeFileSync(f, bytes); return f; };
const log = () => readFileSync(join(out, 'c1/generated.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
const dataUrl = (type, bytes) => `data:${type};base64,${Buffer.from(bytes).toString('base64')}`;
const G = (f) => `out:c1/generated/${f}`;
const CHOSEN = 'boards:dataset/files/c1/s-1/chosen.png';

// ffmpeg: a stub that writes a PNG to its target
const cuts = [];
const frames = createFrames({ out, durationOf: () => 3000, run: async (args) => { cuts.push(args); writeFileSync(args.at(-1), FRAME_PNG); return { err: null, out: '' }; } });
// ComfyUI's /system_stats (comfy-local's status) – the job itself runs a stub script, never comfy.mjs
const comfy = createServer((req, res) => { res.writeHead(req.url === '/system_stats' ? 200 : 404); res.end('{}'); });
const argsFile = join(tmp, 'comfy-args.json');
let routes, manager, manual, generated;

before(async () => {
	await new Promise((ok) => comfy.listen(0, '127.0.0.1', ok));
	put('_out/c1/clean/s-1.png', PNG);
	put('_out/c1/frames/s-1.png', PNG);
	put('_out/c1/variants/s-1--v2.png', PNG);
	put('_out/c1/generated/s-1--g1.png', 'one');
	put('_out/c1/generated/s-1--g2.mp4', tinyMp4(3));
	put('_out/c1/generated/s-1--g10.png', PNG);
	put('_out/c1/generated/s-1--g11.webm', 'webm');
	put('_out/c1/generated/s-2--g1.png', PNG);
	put('_out/c1/handheld/s-1--t1--beauty.mp4', tinyMp4(2));
	put('dataset/files/c1/s-1/chosen.png', 'one');
	put('dataset/manifest.jsonl', JSON.stringify({ date: '2026-10-20', toon: 'cat', chapter: 1, ch: 'c1', shot: 's-1', files: { render: true, chosen: 'chosen.png', from: 's-1--g1.png' } }) + '\n');
	// the stand-in for scripts/storyboard/comfy.mjs under this root: keeps its arguments, prints what comfy.mjs prints
	mkdirSync(join(tmp, 'scripts/storyboard'), { recursive: true });
	writeFileSync(join(tmp, 'scripts/storyboard/comfy.mjs'), `import { writeFileSync } from 'node:fs';
writeFileSync(process.env.STUB_ARGS, JSON.stringify(process.argv.slice(2)));
console.log('s-1 … s-1--g40.png (1 s)');`);
	// RUNWAY_API_KEY empty + no .env under this root: Runway is not configured – nothing can be sent there
	const env = { ...process.env, STUDIO_FAKE_GENERATOR: '1', COMFY_URL: `http://127.0.0.1:${comfy.address().port}`, STUB_ARGS: argsFile, RUNWAY_API_KEY: '', RUNWAY_API_URL: 'http://127.0.0.1:9' };
	// as api.mjs's: a shot that isn't in toon/ch throws
	const findShot = (toon, ch, id) => {
		if (toon !== 'cat' || ch !== 'c1' || !/^s-[12]$/.test(id)) throw new Error(`no shot ${id}`);
		return { sb: {}, group: { shots: [{ id, prompt: { en: 'the shot' }, aspect: '9:16' }] }, index: 0 };
	};
	({ routes, manager, manual } = generatorRoutes({ root: tmp, out, boards, media: null, findShot, shotCharacters: () => [], styleFiles: () => [], env, rate: { get: () => null }, frames }));
	generated = generatedRoutes({ out, boards, manual, run: async () => ({ ok: true }), now: () => new Date('2026-10-20T12:00:00.000Z') });
});
after(() => { comfy.close(); rmSync(tmp, { recursive: true, force: true }); });

test('job-fields: frame keys, the inputs a model takes with their roles (a frame as its video), and back (Vary)', () => {
	assert.deepEqual(parseFrameKey('frame:out:c1/generated/s-1--g2.mp4#first'), { key: 'out:c1/generated/s-1--g2.mp4', frame: 'first' });
	assert.deepEqual(parseFrameKey('frame:out:c1/handheld/s-1--t1--beauty.mp4#1250'), { key: 'out:c1/handheld/s-1--t1--beauty.mp4', frame: 1250 });
	for (const k of ['out:c1/generated/s-1--g2.mp4', 'frame:boards:x.mp4#first', 'frame:out:c1/x.mp4#middle', 'frame:out:c1/x.mp4#-1', 'frame:out:c1/x.mp4', null]) assert.equal(parseFrameKey(k), null, String(k));
	assert.equal(frameKey(G('s-1--g2.mp4'), 'last'), 'frame:out:c1/generated/s-1--g2.mp4#last');
	assert.equal(frameKey(G('s-1--g2.mp4'), 1250.4), 'frame:out:c1/generated/s-1--g2.mp4#1250');
	assert.equal(frameKey(G('s-1--g2.mp4'), '800'), 'frame:out:c1/generated/s-1--g2.mp4#800');
	for (const [k, f] of [[G('x.mp4'), 'middle'], [G('x.mp4'), -1], [G('x.mp4'), null], ['boards:x.mp4', 'first'], [G('x.mp4#1'), 'first']]) assert.throws(() => frameKey(k, f), /bad frame/, `${k} ${f}`);

	const i2v = { mode: 'image_to_video', inputs: ['still', 'prompt'] };
	const v2v = { mode: 'video_to_video', inputs: ['control', 'still', 'prompt'] };
	const byHand = { mode: 'manual', inputs: ['still', 'prompt'] };
	const frame = 'frame:out:c1/generated/s-1--g2.mp4#last';
	assert.deepEqual(inputsOf(i2v, { still: frame, control: G('x.mp4') }), [{ role: 'still', key: G('s-1--g2.mp4'), frame: 'last' }]);
	assert.deepEqual(inputsOf(v2v, { control: G('s-1--g2.mp4'), still: frame }), [{ role: 'control', key: G('s-1--g2.mp4') }, { role: 'keyframe', key: G('s-1--g2.mp4'), frame: 'last', at: 0 }]);
	assert.deepEqual(inputsOf(byHand, { still: G('s-1--g1.png') }), [{ role: 'reference', key: G('s-1--g1.png') }]);
	assert.deepEqual(inputsOf({ mode: 'comfy', inputs: [] }, { still: G('s-1--g1.png') }), []);
	// Vary: a record's inputs → the fields again (the frame as its key); what has no key is named, not guessed
	const rec = [...inputsOf(v2v, { control: G('s-1--g2.mp4'), still: frame }), { role: 'character', key: 'media:c/plf.webp' }, { role: 'style', key: 'boards:style/a.png' }, { role: 'still', key: null, name: 'old.png' }];
	assert.deepEqual(fieldsOf(rec), { still: frame, control: G('s-1--g2.mp4'), characters: ['media:c/plf.webp'], style: ['boards:style/a.png'], missing: ['old.png'] });
	assert.deepEqual(inputsOf(v2v, fieldsOf(rec)), inputsOf(v2v, { control: G('s-1--g2.mp4'), still: frame }));
	assert.equal(fieldsOf([{ role: 'reference', key: CHOSEN }]).still, CHOSEN);
});

test('inputs(): generated images as stills, generated videos as Aleph\'s video + their first / last frames, layout variants\' frames – each with its id', async () => {
	const r = routes['GET /api/generate/inputs']({ toon: 'cat', ch: 'c1', id: 's-1' });
	assert.deepEqual(r.stills.map((s) => [s.key, s.id ?? null, s.frame ?? null]), [
		[CHOSEN, null, null],
		['out:c1/clean/s-1.png', null, null],
		['out:c1/frames/s-1.png', null, null],
		['out:c1/variants/s-1--v2.png', null, null],
		[G('s-1--g10.png'), 's-1--g10', null],
		[G('s-1--g1.png'), 's-1--g1', null],
		[`frame:${G('s-1--g11.webm')}#first`, 's-1--g11', 'first'],
		[`frame:${G('s-1--g11.webm')}#last`, 's-1--g11', 'last'],
		[`frame:${G('s-1--g2.mp4')}#first`, 's-1--g2', 'first'],
		[`frame:${G('s-1--g2.mp4')}#last`, 's-1--g2', 'last']
	]);
	assert.deepEqual(r.stills.slice(3, 7).map((s) => s.label), ['Layout variant v2 (its frame)', 'Generated g10.png', 'Generated g1.png', 'Generated g11.webm – the first frame']);
	// a frame not cut yet has no picture – and listing cuts nothing
	assert.equal(r.stills.at(-1).url, null);
	assert.equal(cuts.length, 0);
	assert.deepEqual(r.controls.map((c) => [c.key, c.id ?? null, c.label]), [
		['out:c1/handheld/s-1--t1--beauty.mp4', null, 'Take t1 · beauty'],
		[G('s-1--g11.webm'), 's-1--g11', 'Generated g11.webm'],
		[G('s-1--g2.mp4'), 's-1--g2', 'Generated g2.mp4']
	]);

	// POST /api/generate/frame: cut now → its picture; the inputs show it then
	const f = await routes['POST /api/generate/frame']({ key: `frame:${G('s-1--g2.mp4')}#last` });
	// named by the video's mtime + size (frames.mjs) – <tag> here
	assert.deepEqual([f.key, f.url.replace(/--[0-9a-f]{10}\.png$/, '--<tag>.png'), Number.isInteger(f.mtime)], [`frame:${G('s-1--g2.mp4')}#last`, '/out/c1/generated/frames/s-1--g2--last--<tag>.png', true]);
	assert.deepEqual(readFileSync(join(out, f.url.slice('/out/'.length))), FRAME_PNG);
	assert.equal(routes['GET /api/generate/inputs']({ toon: 'cat', ch: 'c1', id: 's-1' }).stills.at(-1).url, f.url);
	await routes['POST /api/generate/frame']({ key: `frame:${G('s-1--g2.mp4')}#last` });
	assert.equal(cuts.length, 1, 'cached');
	await assert.rejects(routes['POST /api/generate/frame']({ key: G('s-1--g2.mp4') }), /bad frame/);
	await assert.rejects(routes['POST /api/generate/frame']({ key: `frame:${G('s-1--g1.png')}#first` }), /not a video/);
	await assert.rejects(routes['POST /api/generate/frame']({ key: `frame:${G('s-1--g9.mp4')}#first` }), /no input/);
	await assert.rejects(routes['POST /api/generate/frame']({ key: 'frame:out:../x.mp4#first' }), /no input/);
});

test('a job on a card: a frame as the still (its parent the video), a generated video as Aleph\'s video + the chosen as its keyframe; Vary repeats it', async () => {
	const job = (over) => ({ generator: 'fake', model: 'fake-1', toon: 'cat', ch: 'c1', shot: 's-1', prompt: 'better', duration: 2, ...over });
	const first = `frame:${G('s-1--g2.mp4')}#first`;
	// the estimate cuts the frame (the fake's estimate needs no file – a real one gets it)
	const e = await routes['POST /api/generate/estimate']({ job: job({ still: first }) });
	assert.deepEqual([e.paid, e.usd], [true, 0.02]);
	assert.equal(cuts.length, 2);
	const view = await routes['POST /api/generate']({ job: job({ still: first }), confirm: 0.02 });
	// the running placeholder's place: the record's inputs + parents already in the view
	assert.deepEqual([view.inputs, view.parents], [[{ role: 'still', key: G('s-1--g2.mp4'), frame: 'first' }], ['s-1--g2']]);
	await manager.wait(view.id);
	const [r1] = log();
	assert.deepEqual([r1.id, r1.kind, r1.inputs, r1.parents, r1.prompt], ['s-1--g12', 'image', view.inputs, ['s-1--g2'], 'better']);
	assert.deepEqual(readFileSync(join(out, 'c1/generated', r1.file)), FRAME_PNG, 'the fake returns the still it got – the cut frame');
	assert.equal(cuts.length, 2, 'cut once');

	// a video card into a video to video model: the control first, the shot's chosen image as the keyframe at 0
	const v2v = await routes['POST /api/generate']({ job: job({ model: 'fake-v2v', control: G('s-1--g2.mp4'), still: CHOSEN }), confirm: 0.02 });
	assert.deepEqual([v2v.kind, v2v.inputs, v2v.parents], ['video', [{ role: 'control', key: G('s-1--g2.mp4') }, { role: 'keyframe', key: CHOSEN, at: 0 }], ['s-1--g2', 's-1--g1']]);
	await manager.wait(v2v.id);
	const r2 = log().at(-1);
	assert.deepEqual([r2.inputs, r2.parents, r2.take], [v2v.inputs, ['s-1--g2', 's-1--g1'], null]);

	// Vary: the card's own inputs, prompt, model and generator, a new seed – the same parents (a sibling)
	const vary = await routes['POST /api/generate']({ job: { ...job({ model: r2.model, prompt: r2.prompt }), ...fieldsOf(r2.inputs), seed: null }, confirm: 0.02 });
	await manager.wait(vary.id);
	const r3 = log().at(-1);
	assert.deepEqual([r3.model, r3.inputs, r3.parents], [r2.model, r2.inputs, r2.parents]);
	assert.ok(Number.isInteger(r3.seed));

	// refused before a job exists: a frame as the control, a key that names nothing
	const n = manager.jobs().length;
	await assert.rejects(routes['POST /api/generate']({ job: job({ model: 'fake-v2v', control: first }), confirm: 1 }), /control input is a video, not a frame/);
	await assert.rejects(routes['POST /api/generate']({ job: job({ still: G('s-1--g99.png') }), confirm: 1 }), /no input/);
	await assert.rejects(routes['POST /api/generate']({ job: job({ still: 'frame:out:c1/generated/s-1--g2.mp4#9000' }), confirm: 1 }), /no frame at 9\.00 s/);
	assert.equal(manager.jobs().length, n);
});

test('a job for a shot that isn\'t in toon/ch, or with another chapter\'s inputs, is refused – estimate, start and the manual slot; no frame cut', async () => {
	put('_out/c2/generated/s-1--g1.png', PNG);
	put('_out/c2/generated/s-1--g2.mp4', tinyMp4(3));
	put('dataset/files/c2/s-1/chosen.png', PNG);
	const job = (over) => ({ generator: 'fake', model: 'fake-1', toon: 'cat', ch: 'c1', shot: 's-1', prompt: 'p', duration: 1, ...over });
	const n = manager.jobs().length, cut = cuts.length, lines = log().length;
	// a form left from another chapter / shot: the shot isn't there
	for (const bad of [{ ch: 'c2' }, { shot: 's-9' }, { toon: 'dog' }, { toon: undefined }]) {
		await assert.rejects(routes['POST /api/generate/estimate']({ job: job(bad) }), /no shot/, JSON.stringify(bad));
		await assert.rejects(routes['POST /api/generate']({ job: job(bad), confirm: 1 }), /no shot/, JSON.stringify(bad));
	}
	// c1's shot with c2's files (ids are per chapter – its parents would name c1's files)
	for (const still of ['out:c2/generated/s-1--g1.png', 'boards:dataset/files/c2/s-1/chosen.png', 'boards:_out/c2/generated/s-1--g1.png', 'frame:out:c2/generated/s-1--g2.mp4#first']) {
		await assert.rejects(routes['POST /api/generate/estimate']({ job: job({ still }) }), /belongs to c2, not c1/, still);
		await assert.rejects(routes['POST /api/generate']({ job: job({ still }), confirm: 1 }), /belongs to c2, not c1/, still);
	}
	await assert.rejects(routes['POST /api/generate']({ job: job({ model: 'fake-v2v', control: 'out:c2/generated/s-1--g2.mp4' }), confirm: 1 }), /belongs to c2, not c1/);
	await assert.rejects(generated['POST /api/generated']({ ch: 'c1', id: 's-1', name: 'x.png', data: dataUrl('image/png', PNG), job: { generator: 'luma-manual', toon: 'cat', still: 'out:c2/generated/s-1--g1.png' } }), /belongs to c2, not c1/);
	await assert.rejects(generated['POST /api/generated']({ ch: 'c1', id: 's-1', name: 'x.png', data: dataUrl('image/png', PNG), job: { generator: 'luma-manual' } }), /no shot s-1 in undefined\/c1/, 'the slot names its toon');
	assert.deepEqual([manager.jobs().length, cuts.length, log().length], [n, cut, lines], 'no job, no frame cut, no record');
	// what all chapters share stays allowed: the style folder, characters (media / boards outside dataset/files)
	put('style/look.png', PNG);
	assert.equal((await routes['POST /api/generate/estimate']({ job: job({ still: 'out:c1/clean/s-1.png', style: ['boards:style/look.png'] }) })).error, undefined);
});

test('comfy-local: a generated image of the shot or the chosen goes in as --from (the edit); the render is its own; others refused up front', async () => {
	const start = async (still) => {
		const v = await routes['POST /api/generate']({ job: { generator: 'comfy-local', model: 'qwen-image-edit', toon: 'cat', ch: 'c1', shot: 's-1', ...(still ? { still } : {}), seed: 7 } });
		await manager.wait(v.id);
		const done = manager.jobs()[0];
		assert.equal(done.status, 'succeeded', done.message);
		return [JSON.parse(readFileSync(argsFile, 'utf8')), v];
	};
	const base = ['storyboards/cat/c1', '--only', 's-1', '--seed', '7'];
	let [args, v] = await start(G('s-1--g10.png'));
	assert.deepEqual(args, [...base, '--from', 's-1--g10.png']);
	assert.deepEqual([v.inputs, v.parents], [[{ role: 'still', key: G('s-1--g10.png') }], ['s-1--g10']]);
	[args, v] = await start(CHOSEN);
	assert.deepEqual(args, [...base, '--from', 's-1--g1.png'], 'the chosen = the file it was approved from');
	assert.deepEqual(v.parents, ['s-1--g1']);
	assert.deepEqual((await start('out:c1/clean/s-1.png'))[0], base);
	assert.deepEqual((await start(null))[0], base);

	const n = manager.jobs().length;
	for (const [still, why] of [[`frame:${G('s-1--g2.mp4')}#first`, /not s-1--g2\.mp4 \(first frame\)/], ['out:c1/frames/s-1.png', /not s-1\.png/], ['out:c1/variants/s-1--v2.png', /not s-1--v2\.png/], [G('s-2--g1.png'), /of s-1 – not s-2--g1\.png/]]) {
		const e = await routes['POST /api/generate/estimate']({ job: { generator: 'comfy-local', toon: 'cat', ch: 'c1', shot: 's-1', still } });
		assert.match(e.error, why, still);
		await assert.rejects(routes['POST /api/generate']({ job: { generator: 'comfy-local', toon: 'cat', ch: 'c1', shot: 's-1', still } }), why);
	}
	assert.equal(manager.jobs().length, n, 'no job for a refused still');
	assert.match((await routes['POST /api/generate/estimate']({ job: { generator: 'comfy-local', toon: 'cat', ch: 'c1', shot: 's-1', still: G('s-1--g10.png') } })).note, /edits s-1--g10\.png/);
});

test('Runway (estimate only – nothing is sent): Aleph takes a generated MP4 as its video, not a WebM; a frame is a start frame; the caps', async () => {
	const est = (job) => routes['POST /api/generate/estimate']({ job: { generator: 'runway', toon: 'cat', ch: 'c1', shot: 's-1', aspect: '9:16', duration: 5, ...job } });
	assert.deepEqual(await est({ model: 'aleph2', control: G('s-1--g2.mp4') }).then((e) => [e.seconds, e.credits, e.error]), [3, 84, undefined]);
	assert.match((await est({ model: 'aleph2', control: G('s-1--g11.webm') })).error, /Aleph takes an MP4 – s-1--g11\.webm is not one/);
	assert.match((await est({ model: 'aleph2' })).error, /pick the video first/);
	assert.deepEqual(await est({ model: 'gen4_turbo', still: `frame:${G('s-1--g2.mp4')}#last` }).then((e) => [e.credits, e.ratio, e.error]), [25, '720:1280', undefined]);
	const { generators } = await routes['GET /api/generators']();
	const caps = Object.fromEntries(generators.flatMap((g) => g.models.map((m) => [`${g.id}:${m.id}`, [m.inputs.join(' '), m.startFrame ?? null, m.keyframes?.max ?? null, m.source ?? null]])));
	assert.deepEqual(caps, {
		'comfy-local:qwen-image-edit': ['still', null, null, 'edit'],
		'runway:gen4_turbo': ['still prompt duration ratio', true, null, null],
		'runway:gen4.5': ['still prompt duration ratio', true, null, null],
		'runway:aleph2': ['control still prompt', null, 1, null],
		'luma-manual:manual': ['still prompt', null, null, null],
		'fake:fake-1': ['still prompt duration', true, null, null],
		'fake:fake-v2v': ['control still prompt', null, 1, null]
	});
	assert.equal(generators.find((g) => g.id === 'runway').configured, false, 'no key in this test – nothing can be sent');
});

test('the manual result slot: the dropped result recorded as Luma – by hand\'s candidate (prompt, inputs, parent) – not an upload', async () => {
	const drop = generated['POST /api/generated'];
	const before = () => readdirSync(join(out, 'c1/generated')).length;
	const n = before();
	const r = await drop({ ch: 'c1', id: 's-1', name: 'luma.png', data: dataUrl('image/png', PNG), job: { generator: 'luma-manual', toon: 'cat', model: 'manual', prompt: 'built in the studio', still: G('s-1--g10.png'), control: G('s-1--g2.mp4') }, parent: 's-1--g2' });
	assert.equal(r.generator, 'luma-manual');
	const rec = log().find((x) => x.id === r.id);
	assert.deepEqual([rec.file, rec.kind, rec.generator, rec.model, rec.mode, rec.prompt, rec.status], [r.file, 'image', 'luma-manual', 'manual', 'manual', 'built in the studio', 'succeeded']);
	// only what the model takes (the still – Luma's image reference); the Improve-on card's id first in the parents
	assert.deepEqual([rec.inputs, rec.parents, rec.params], [[{ role: 'reference', key: G('s-1--g10.png') }], ['s-1--g2', 's-1--g10'], { name: 'luma.png', url: null }]);
	// a frame as the picture attached: its video is the parent; a clip dropped is a video candidate
	const clip = await drop({ ch: 'c1', id: 's-1', name: 'luma.mp4', data: dataUrl('video/mp4', tinyMp4(5)), job: { generator: 'luma-manual', toon: 'cat', prompt: 'p', still: `frame:${G('s-1--g2.mp4')}#last` } });
	const crec = log().find((x) => x.id === clip.id);
	assert.deepEqual([crec.kind, crec.model, crec.inputs, crec.parents], ['video', 'manual', [{ role: 'reference', key: G('s-1--g2.mp4'), frame: 'last' }], ['s-1--g2']]);
	assert.equal(before(), n + 2);
	// GET: the records carry their key – the card as an input
	assert.equal(generated['GET /api/generated']({ ch: 'c1', ids: 's-1' }).find((x) => x.id === clip.id).key, G(clip.file));

	// refused before anything is written: a generator with jobs, a parent of another shot / unknown, a parent without a
	// job, an input that names nothing
	const refused = [
		[{ job: { generator: 'runway', model: 'gen4_turbo', prompt: 'p' } }, /runway is not done by hand/],
		[{ job: { generator: 'luma-manual', toon: 'cat' }, parent: 's-2--g1' }, /bad parent s-2--g1 – a generation of s-1/],
		[{ job: { generator: 'luma-manual', toon: 'cat' }, parent: 's-1--g77' }, /no generation s-1--g77/],
		[{ parent: 's-1--g2' }, /a parent only with the manual slot's job/],
		[{ job: { generator: 'luma-manual', toon: 'cat', still: G('s-1--g99.png') } }, /no input/]
	];
	const lines = log().length;
	for (const [extra, why] of refused) await assert.rejects(drop({ ch: 'c1', id: 's-1', name: 'x.png', data: dataUrl('image/png', PNG), ...extra }), why);
	assert.deepEqual([before(), log().length], [n + 2, lines]);
	// without the generators wired in, the slot says so
	await assert.rejects(generatedRoutes({ out, boards, run: async () => ({}) })['POST /api/generated']({ ch: 'c1', id: 's-1', data: dataUrl('image/png', PNG), job: { generator: 'luma-manual' } }), /needs the generators/);
});
