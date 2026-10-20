// The generator interface with the fake generator: job states, cancel, the cost gate, the spend counter, the variants
// written like comfy.mjs's (generated/<shot>--g<n>.<ext> + the v2 record in generated.jsonl – a failed or cancelled job's
// too), the seed, the inputs' roles.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createManager } from '../jobs.mjs';
import { createFake, createComfyRemote } from '../others.mjs';
import { createComfyLocal } from '../comfy-local.mjs';
import { generatorRoutes } from '../index.mjs';
import { writeNew } from '../../../../scripts/storyboard/generated-log.mjs';

const tmp = () => mkdtempSync(join(tmpdir(), 'sb-gen-'));
const lines = (out) => readFileSync(join(out, 's1c3/generated.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
const job = (over = {}) => ({ generator: 'fake', model: 'fake-1', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1', prompt: 'a shot', duration: 3, ...over });

test('a paid job needs the confirmed cost, then runs to succeeded and writes a variant', async () => {
	const out = tmp();
	const m = createManager({ generators: [createFake({ stepMs: 5 })], out });
	assert.deepEqual(m.estimate(job()), { paid: true, model: 'fake-1', credits: 3, usd: 0.03, seconds: 3, note: 'fake credits' });
	await assert.rejects(m.start(job()), /confirm the cost first/);
	await assert.rejects(m.start(job(), { confirm: 0.02 }), /confirm the cost first/);
	const j = await m.start(job({ still: 'out:s1c3/clean/sb-1-1.png', characters: ['media:characters/plf/plf_full-neutral.webp'] }), { confirm: 0.03 });
	assert.equal(j.status, 'running');
	// the view carries what the variants board needs for its running placeholder: the inputs (the main one first), the parents
	assert.deepEqual([j.kind, j.inputs, j.parents], ['image', [{ role: 'still', key: 'out:s1c3/clean/sb-1-1.png' }], []]);
	await m.wait(j.id);
	const [done] = m.jobs({ ch: 's1c3', shot: 'sb-1-1' });
	assert.equal(done.status, 'succeeded');
	assert.equal(done.progress, 1);
	assert.deepEqual(done.files, ['sb-1-1--g1.png']);
	assert.ok(existsSync(join(out, 's1c3/generated/sb-1-1--g1.png')));
	const log = lines(out);
	assert.equal(log.length, 1);
	// the v2 record (VARIANTS-CANVAS §3.7): the model takes a seed – none sent, so the manager picked one
	assert.ok(Number.isInteger(log[0].seed) && log[0].seed >= 0 && log[0].seed <= 2 ** 32 - 1);
	assert.equal(done.seed, log[0].seed);
	assert.match(log[0].job, /^fake-/);
	assert.match(log[0].date, /^\d{4}-\d\d-\d\dT/);
	assert.deepEqual({ ...log[0], seed: 0, job: 'x', date: 'd' }, {
		v: 2, id: 'sb-1-1--g1', shot: 'sb-1-1', file: 'sb-1-1--g1.png', kind: 'image', take: null, generator: 'fake', model: 'fake-1', mode: 'fake',
		job: 'x', status: 'succeeded', parents: [],
		// only what the model takes (the fake: a still – the character is not sent)
		inputs: [{ role: 'still', key: 'out:s1c3/clean/sb-1-1.png' }],
		prompt: 'a shot', seed: 0, ratio: null, duration: 3, estimate: { credits: 3, usd: 0.03 }, cost: { credits: 3, usd: 0.03 }, date: 'd',
		mark: null, marked: null, params: { fake: true }
	});
	assert.deepEqual(m.spend(), { usd: 0.03, credits: 3, jobs: 1 });
});

test('a given seed is kept (and sent), an empty one picked, a bad one refused; a model without a seed records none', async () => {
	const out = tmp();
	let sent = null;
	const fake = createFake({ stepMs: 1 });
	const run = fake.run;
	fake.run = (input, ctx) => { sent = input.seed; return run(input, ctx); };
	const m = createManager({ generators: [fake], out });
	await m.wait((await m.start(job({ seed: 4294967295 }), { confirm: 1 })).id);
	assert.equal(sent, 4294967295);
	await m.wait((await m.start(job({ seed: '' }), { confirm: 1 })).id);
	assert.ok(Number.isInteger(sent));
	await assert.rejects(m.start(job({ seed: 2 ** 32 }), { confirm: 1 }), /bad seed/);
	await assert.rejects(m.start(job({ seed: 1.5 }), { confirm: 1 }), /bad seed/);
	assert.deepEqual(lines(out).map((l) => l.seed), [4294967295, sent]);
	fake.models[0].seed = false;
	await m.wait((await m.start(job({ seed: 7 }), { confirm: 1 })).id);
	assert.equal(lines(out).at(-1).seed, null);
});

test('video to video: the control video first, the still its keyframe at 0; parents = the generated inputs, the take from the control', async () => {
	const out = tmp();
	const v2v = { ...createFake({ stepMs: 1 }), id: 'v2v', kinds: ['video'] };
	v2v.models = [{ id: 'v2v-1', kind: 'video', mode: 'video_to_video', inputs: ['control', 'still', 'prompt'], seed: true }];
	v2v.run = async () => ({ files: [{ bytes: Buffer.from('mp4'), ext: 'mp4' }] });
	const m = createManager({ generators: [v2v], out });
	await m.wait((await m.start(job({ generator: 'v2v', model: 'v2v-1', control: 'out:s1c3/handheld/sb-1-1--t2--beauty.mp4', still: 'out:s1c3/generated/sb-1-1--g3.png' }), { confirm: 1 })).id);
	const [r] = lines(out);
	assert.deepEqual([r.id, r.file, r.kind, r.take, r.parents], ['sb-1-1--g1', 'sb-1-1--g1.mp4', 'video', 't2', ['sb-1-1--g3']]);
	assert.deepEqual(r.inputs, [{ role: 'control', key: 'out:s1c3/handheld/sb-1-1--t2--beauty.mp4' }, { role: 'keyframe', key: 'out:s1c3/generated/sb-1-1--g3.png', at: 0 }]);
	// a generated video as the control (VARIANTS-CANVAS §3.6 "improve a video"): the parent is that video, the take none
	await m.wait((await m.start(job({ generator: 'v2v', model: 'v2v-1', control: 'out:s1c3/generated/sb-1-1--g1.mp4' }), { confirm: 1 })).id);
	assert.deepEqual([lines(out)[1].id, lines(out)[1].parents, lines(out)[1].take], ['sb-1-1--g2', ['sb-1-1--g1'], null]);
});

test('the next variant number skips past existing files (a deleted one is never overwritten)', async () => {
	const out = tmp();
	mkdirSync(join(out, 's1c3/generated'), { recursive: true });
	for (const f of ['sb-1-1--g1.png', 'sb-1-1--g4.png', 'sb-2-1--g9.png']) writeFileSync(join(out, 's1c3/generated', f), 'x');
	const m = createManager({ generators: [createFake({ stepMs: 1 })], out });
	const j = await m.start(job(), { confirm: 1 });
	await m.wait(j.id);
	assert.deepEqual(m.jobs()[0].files, ['sb-1-1--g5.png']);
	// the log's ids count too: a record whose file was deleted (g7) is never reused
	writeFileSync(join(out, 's1c3/generated.jsonl'), '{"v":2,"id":"sb-1-1--g7","shot":"sb-1-1","file":"sb-1-1--g7.png"}\n', { flag: 'a' });
	await m.wait((await m.start(job(), { confirm: 1 })).id);
	assert.deepEqual(m.jobs()[0].files, ['sb-1-1--g8.png']);
});

test('cancel stops a running job – no file, nothing spent, its line in the log (an id, no file)', async () => {
	const out = tmp();
	const m = createManager({ generators: [createFake({ stepMs: 50, steps: 20 })], out });
	const j = await m.start(job({ still: 'out:s1c3/clean/sb-1-1.png' }), { confirm: 1 });
	await new Promise((ok) => setTimeout(ok, 80));
	assert.equal(m.cancel(j.id).status, 'running');
	await m.wait(j.id);
	const v = m.jobs()[0];
	assert.equal(v.status, 'cancelled');
	assert.ok(v.progress > 0 && v.progress < 1);
	assert.equal(existsSync(join(out, 's1c3/generated')), false);
	assert.deepEqual(m.spend(), { usd: 0, credits: 0, jobs: 0 });
	const [r] = lines(out);
	assert.deepEqual([r.v, r.id, r.file, r.kind, r.status, r.cost, r.error, r.inputs[0].key], [2, 'sb-1-1--g1', null, 'image', 'cancelled', null, undefined, 'out:s1c3/clean/sb-1-1.png']);
	assert.equal(r.seed, v.seed);
	// the next result does not take the cancelled job's id
	const fast = createManager({ generators: [createFake({ stepMs: 1 })], out });
	await fast.wait((await fast.start(job(), { confirm: 1 })).id);
	assert.deepEqual(fast.jobs()[0].files, ['sb-1-1--g2.png']);
});

test('a failing job ends failed with its message, without a retry', async () => {
	const out = tmp();
	let runs = 0;
	const fake = createFake({ stepMs: 1, fail: true });
	const run = fake.run;
	fake.run = (...a) => { runs++; return run(...a); };
	const m = createManager({ generators: [fake], out });
	const j = await m.start(job(), { confirm: 1 });
	await m.wait(j.id);
	assert.equal(m.jobs()[0].status, 'failed');
	assert.equal(m.jobs()[0].message, 'the fake failed on purpose');
	assert.equal(runs, 1);
	const [r] = lines(out);
	assert.deepEqual([r.id, r.file, r.status, r.error, r.generator, r.model, r.estimate], ['sb-1-1--g1', null, 'failed', 'the fake failed on purpose', 'fake', 'fake-1', { credits: 3, usd: 0.03 }]);
});

test('a generator that is not configured refuses to start; the list says why', async () => {
	const m = createManager({ generators: [createComfyRemote()], out: tmp() });
	const [g] = await m.list();
	assert.equal(g.configured, false);
	assert.match(g.how, /Not built yet/);
	await assert.rejects(m.start({ ...job(), generator: 'comfyui-remote' }, { confirm: 100 }), /not configured/);
	assert.throws(() => m.estimate({ generator: 'nope' }), /no generator nope/);
});

test('bad shot ids are refused (they become file names)', async () => {
	const m = createManager({ generators: [createFake({ stepMs: 1 })], out: tmp() });
	await assert.rejects(m.start(job({ shot: '../x' }), { confirm: 1 }), /bad shot/);
	assert.equal(readdirSync(tmp()).length, 0);
});

test('every generator in the registry says how a job gets made: GET /api/generators carries its transport, its credit price and the euro rate', async () => {
	const dir = tmp();
	// the rate fixed by the env – no ECB fetch from a test (rate.test.mjs)
	const { routes } = generatorRoutes({ root: dir, out: dir, boards: dir, media: dir, findShot: () => null, shotCharacters: () => [], styleFiles: () => [], env: { STUDIO_FAKE_GENERATOR: '1', COMFY_URL: 'http://127.0.0.1:9', STUDIO_EUR_PER_USD: '0.9' } });
	const { generators, rate } = await routes['GET /api/generators']();
	assert.deepEqual(Object.fromEntries(generators.map((g) => [g.id, g.transport])), { 'comfy-local': 'local', runway: 'api', 'luma-manual': 'manual', 'comfyui-remote': 'api', fake: 'api' });
	assert.equal(generators.find((g) => g.id === 'runway').creditUsd, 0.01);
	assert.deepEqual(rate, { eurPerUsd: 0.9, usdPerEur: 1.111111, date: null, source: 'env' });
	assert.equal((await routes['GET /api/generate/jobs']({ ch: 's1c3', id: 'sb-1-1' })).rate.source, 'env');
});

test('comfy-local: the picked seed goes to comfy.mjs (--seed); its own record stays its own; a failed run gets the manager\'s line', async () => {
	const dir = tmp(), out = join(dir, 'out'), argsFile = join(dir, 'args.json');
	// a stand-in for scripts/storyboard/comfy.mjs: keeps its arguments, prints what comfy.mjs prints for a result
	const script = join(dir, 'comfy-stub.mjs');
	writeFileSync(script, `import { writeFileSync } from 'node:fs';
writeFileSync(process.env.STUB_ARGS, JSON.stringify(process.argv.slice(2)));
console.log(process.env.STUB_FAIL ? 'failed: no ComfyUI' : 'sb-1-1 … sb-1-1--g1.png (1 s)');`);
	const make = (env) => { const g = createComfyLocal({ root: dir, script, env: { ...process.env, STUB_ARGS: argsFile, ...env } }); g.status = async () => ({ configured: true }); return g; };
	const m = createManager({ generators: [make({})], out });
	const j = await m.start({ generator: 'comfy-local', model: 'qwen-image-edit', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1' });
	await m.wait(j.id);
	const [done] = m.jobs();
	assert.deepEqual([done.status, done.files], ['succeeded', ['sb-1-1--g1.png']]);
	assert.ok(Number.isInteger(done.seed) && done.seed >= 0 && done.seed <= 2 ** 32 - 1);
	assert.deepEqual(JSON.parse(readFileSync(argsFile, 'utf8')), ['storyboards/catpope/s1c3', '--only', 'sb-1-1', '--seed', String(done.seed)]);
	// comfy.mjs writes the result's record itself – the manager adds none
	assert.equal(existsSync(join(out, 's1c3/generated.jsonl')), false);
	const failing = createManager({ generators: [make({ STUB_FAIL: '1' })], out });
	await failing.wait((await failing.start({ generator: 'comfy-local', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1', seed: 5 })).id);
	assert.deepEqual(JSON.parse(readFileSync(argsFile, 'utf8')).slice(-2), ['--seed', '5']);
	const [r] = lines(out);
	assert.deepEqual([r.id, r.file, r.generator, r.model, r.mode, r.status, r.error, r.seed, r.inputs, r.cost], ['sb-1-1--g1', null, 'comfy-local', 'qwen-image-edit', 'comfy', 'failed', 'failed: no ComfyUI', 5, [], null]);
});

test('the approved image as the still: its parent is the generated file it was approved from (approve.mjs `from`)', async () => {
	const dir = tmp(), boards = join(dir, 'boards'), out = join(boards, '_out');
	const put = (rel, bytes) => { const f = join(boards, rel); mkdirSync(join(f, '..'), { recursive: true }); writeFileSync(f, bytes); };
	put('_out/c1/generated/s-1--g2.png', 'two');
	put('_out/c1/generated/s-1--g3.png', 'two'); // the same bytes – only `from` says which one was approved
	put('dataset/files/c1/s-1/chosen.png', 'two');
	put('dataset/manifest.jsonl', JSON.stringify({ date: '2026-10-20', toon: 'cat', chapter: 1, ch: 'c1', shot: 's-1', files: { render: false, chosen: 'chosen.png', from: 's-1--g2.png' } }) + '\n');
	const { routes, manager } = generatorRoutes({ root: dir, out, boards, media: null, findShot: () => null, shotCharacters: () => [], styleFiles: () => [], env: { STUDIO_FAKE_GENERATOR: '1', COMFY_URL: 'http://127.0.0.1:9', STUDIO_EUR_PER_USD: '0.9' } });
	const view = await routes['POST /api/generate']({ job: { generator: 'fake', model: 'fake-1', toon: 'cat', ch: 'c1', shot: 's-1', still: 'boards:dataset/files/c1/s-1/chosen.png', duration: 1 }, confirm: 0.01 });
	await manager.wait(view.id);
	const [r] = readFileSync(join(out, 'c1/generated.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
	assert.deepEqual([r.id, r.inputs, r.parents], ['s-1--g4', [{ role: 'still', key: 'boards:dataset/files/c1/s-1/chosen.png' }], ['s-1--g2']]);
});

test('writeNew: the highest n + 1, created exclusively – an existing file is never written over', () => {
	const out = tmp();
	mkdirSync(join(out, 'c1/generated'), { recursive: true });
	writeFileSync(join(out, 'c1/generated/s-1--g2.png'), 'two');
	assert.deepEqual(writeNew(out, 'c1', 's-1', 'png', Buffer.from('three')), { id: 's-1--g3', file: 's-1--g3.png' });
	assert.deepEqual(writeNew(out, 'c1', 's-1', 'mp4', Buffer.from('four')), { id: 's-1--g4', file: 's-1--g4.mp4' });
	assert.deepEqual(readdirSync(join(out, 'c1/generated')).map((f) => [f, readFileSync(join(out, 'c1/generated', f), 'utf8')]), [['s-1--g2.png', 'two'], ['s-1--g3.png', 'three'], ['s-1--g4.mp4', 'four']]);
});
