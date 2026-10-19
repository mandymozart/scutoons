// The generator interface with the fake generator: job states, cancel, the cost gate, the spend counter, the variants
// written like comfy.mjs's (generated/<shot>--g<n>.<ext> + generated.jsonl).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createManager } from '../jobs.mjs';
import { createFake, createComfyRemote } from '../others.mjs';
import { generatorRoutes } from '../index.mjs';

const tmp = () => mkdtempSync(join(tmpdir(), 'sb-gen-'));
const job = (over = {}) => ({ generator: 'fake', model: 'fake-1', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1', prompt: 'a shot', duration: 3, ...over });

test('a paid job needs the confirmed cost, then runs to succeeded and writes a variant', async () => {
	const out = tmp();
	const m = createManager({ generators: [createFake({ stepMs: 5 })], out });
	assert.deepEqual(m.estimate(job()), { paid: true, model: 'fake-1', credits: 3, usd: 0.03, seconds: 3, note: 'fake credits' });
	await assert.rejects(m.start(job()), /confirm the cost first/);
	await assert.rejects(m.start(job(), { confirm: 0.02 }), /confirm the cost first/);
	const j = await m.start(job(), { confirm: 0.03 });
	assert.equal(j.status, 'running');
	await m.wait(j.id);
	const [done] = m.jobs({ ch: 's1c3', shot: 'sb-1-1' });
	assert.equal(done.status, 'succeeded');
	assert.equal(done.progress, 1);
	assert.deepEqual(done.files, ['sb-1-1--g1.png']);
	assert.ok(existsSync(join(out, 's1c3/generated/sb-1-1--g1.png')));
	const log = readFileSync(join(out, 's1c3/generated.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
	assert.equal(log.length, 1);
	assert.equal(log[0].shot, 'sb-1-1');
	assert.equal(log[0].file, 'sb-1-1--g1.png');
	assert.equal(log[0].engine, 'fake');
	assert.equal(log[0].generator, 'fake');
	assert.equal(log[0].model, 'fake-1');
	assert.match(log[0].job, /^fake-/);
	assert.deepEqual(log[0].estimate, { credits: 3, usd: 0.03 });
	assert.deepEqual(log[0].cost, { credits: 3, usd: 0.03 });
	assert.equal(log[0].prompt, 'a shot');
	assert.deepEqual(m.spend(), { usd: 0.03, credits: 3, jobs: 1 });
});

test('the next variant number skips past existing files (a deleted one is never overwritten)', async () => {
	const out = tmp();
	mkdirSync(join(out, 's1c3/generated'), { recursive: true });
	for (const f of ['sb-1-1--g1.png', 'sb-1-1--g4.png', 'sb-2-1--g9.png']) writeFileSync(join(out, 's1c3/generated', f), 'x');
	const m = createManager({ generators: [createFake({ stepMs: 1 })], out });
	const j = await m.start(job(), { confirm: 1 });
	await m.wait(j.id);
	assert.deepEqual(m.jobs()[0].files, ['sb-1-1--g5.png']);
});

test('cancel stops a running job – no file, nothing spent', async () => {
	const out = tmp();
	const m = createManager({ generators: [createFake({ stepMs: 50, steps: 20 })], out });
	const j = await m.start(job(), { confirm: 1 });
	await new Promise((ok) => setTimeout(ok, 80));
	assert.equal(m.cancel(j.id).status, 'running');
	await m.wait(j.id);
	const v = m.jobs()[0];
	assert.equal(v.status, 'cancelled');
	assert.ok(v.progress > 0 && v.progress < 1);
	assert.equal(existsSync(join(out, 's1c3/generated')), false);
	assert.deepEqual(m.spend(), { usd: 0, credits: 0, jobs: 0 });
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

test('every generator in the registry says how a job gets made: GET /api/generators carries its transport', async () => {
	const dir = tmp();
	const { routes } = generatorRoutes({ root: dir, out: dir, boards: dir, media: dir, findShot: () => null, shotCharacters: () => [], styleFiles: () => [], env: { STUDIO_FAKE_GENERATOR: '1', COMFY_URL: 'http://127.0.0.1:9' } });
	const { generators } = await routes['GET /api/generators']();
	assert.deepEqual(Object.fromEntries(generators.map((g) => [g.id, g.transport])), { 'comfy-local': 'local', runway: 'api', 'luma-manual': 'manual', 'comfyui-remote': 'api', fake: 'api' });
});
