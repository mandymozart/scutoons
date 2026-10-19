// The Runway adapter against the mock (runway-mock.mjs) – never the real API.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRunway } from '../runway.mjs';
import { createManager } from '../jobs.mjs';
import { mp4Duration, nearestRatio } from '../media.mjs';
import { startMock, tinyMp4 } from './runway-mock.mjs';

let mock, dir, still, control, big;
before(async () => {
	mock = await startMock({ polls: 2 });
	dir = mkdtempSync(join(tmpdir(), 'sb-runway-'));
	still = join(dir, 'sb-1-1.png');
	writeFileSync(still, Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==', 'base64'));
	control = join(dir, 'sb-1-1--t1--grey.mp4');
	writeFileSync(control, tinyMp4(4.2));
	big = join(dir, 'big.mp4');
	writeFileSync(big, Buffer.concat([tinyMp4(6), Buffer.alloc(4 * 1024 * 1024)]));
});
after(() => mock.close());

const env = () => ({ RUNWAY_API_URL: mock.url, RUNWAY_API_KEY: 'mock-key' });
const base = { generator: 'runway', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1', aspect: '9:16' };

test('helpers: mp4 duration, the nearest ratio', () => {
	assert.equal(mp4Duration(control), 4.2);
	assert.equal(nearestRatio('9:16', ['1280:720', '720:1280', '960:960']), '720:1280');
	assert.equal(nearestRatio('1:3', ['1280:720', '720:1280', '960:960']), '720:1280');
	assert.equal(nearestRatio('4:3', ['1280:720', '1104:832', '960:960']), '1104:832');
});

test('no key → not configured, with the how-to; the key comes from the repo .env too', async () => {
	assert.equal((await createRunway({ root: dir, env: {} }).status()).configured, false);
	assert.match((await createRunway({ root: dir, env: {} }).status()).how, /RUNWAY_API_KEY/);
	const root = mkdtempSync(join(tmpdir(), 'sb-root-'));
	writeFileSync(join(root, '.env'), 'OTHER=1\nRUNWAY_API_KEY=from-dotenv\n');
	assert.equal((await createRunway({ root, env: {} }).status()).configured, true);
});

test('estimates: gen4_turbo 5 credits/s, gen4.5 12/s, aleph2 28/s with 56 minimum from the video length', () => {
	const r = createRunway({ env: env() });
	assert.deepEqual(r.estimate({ model: 'gen4_turbo', duration: 5, aspect: '9:16' }), { credits: 25, usd: 0.25, seconds: 5, ratio: '720:1280', note: '', rate: '5 credits/s' });
	assert.equal(r.estimate({ model: 'gen4.5', duration: 10 }).usd, 1.2);
	assert.equal(r.estimate({ model: 'gen4_turbo', duration: 99 }).seconds, 10);
	const a = r.estimate({ model: 'aleph2', controlFile: control });
	assert.equal(a.credits, 118); // ceil(28 × 4.2)
	assert.equal(a.usd, 1.18);
	assert.match(r.estimate({ model: 'aleph2' }).error, /handheld render pass/);
});

test('image to video: one create call (headers, data URI, ratio, duration), polls, downloads the MP4', async () => {
	const r = createRunway({ env: env(), pollMs: 5 });
	mock.requests.length = 0;
	const seen = [];
	const ctx = { signal: new AbortController().signal, progress: (p, m) => seen.push([p, m]), remote: (id, c) => seen.push(['task', id, c]), cost: (c) => seen.push(['cost', c]) };
	const res = await r.run({ ...base, model: 'gen4_turbo', prompt: 'the tiger roars', duration: 5, stillFile: still }, ctx);
	const creates = mock.requests.filter((q) => q.method === 'POST' && q.path === '/v1/image_to_video');
	assert.equal(creates.length, 1);
	assert.equal(creates[0].headers.authorization, 'Bearer mock-key');
	assert.equal(creates[0].headers['x-runway-version'], '2024-11-06');
	assert.deepEqual({ ...creates[0].body, promptImage: creates[0].body.promptImage.slice(0, 22) }, { model: 'gen4_turbo', promptImage: 'data:image/png;base64,', ratio: '720:1280', duration: 5, promptText: 'the tiger roars' });
	assert.equal(res.files.length, 1);
	assert.equal(res.files[0].ext, 'mp4');
	writeFileSync(join(dir, 'out.mp4'), res.files[0].bytes);
	assert.equal(mp4Duration(join(dir, 'out.mp4')), 5);
	assert.ok(seen.some((s) => s[0] === 'task' && s[2] === 25));
	assert.ok(seen.some((s) => s[0] === 'cost' && s[1] === 25));
	assert.ok(seen.some((s) => s[1] === 'Running'));
});

test('video to video (aleph2): the handheld pass + the still as a keyframe; a big video goes as an ephemeral upload', async () => {
	const r = createRunway({ env: env(), pollMs: 5 });
	const ctx = { signal: new AbortController().signal, progress() {}, remote() {}, cost() {} };
	mock.requests.length = 0;
	await r.run({ ...base, model: 'aleph2', prompt: 'film noir, snow', controlFile: control, stillFile: still }, ctx);
	const c = mock.requests.find((q) => q.path === '/v1/video_to_video').body;
	assert.equal(c.model, 'aleph2');
	assert.match(c.videoUri, /^data:video\/mp4;base64,/);
	assert.equal(c.keyframes.length, 1);
	assert.equal(c.keyframes[0].at, 0);
	assert.equal(c.ratio, undefined);
	mock.requests.length = 0;
	await r.run({ ...base, model: 'aleph2', controlFile: big }, ctx);
	assert.deepEqual(mock.requests.find((q) => q.path === '/v1/uploads').body, { filename: 'big.mp4', type: 'ephemeral' });
	assert.ok(mock.requests.some((q) => q.path === '/upload' && q.size > 4 * 1024 * 1024));
	assert.equal(mock.requests.find((q) => q.path === '/v1/video_to_video').body.videoUri, 'runway://mock/big.mp4');
});

test('a failed task, a refused request and a wrong key end the job – never a second create call', async () => {
	const r = createRunway({ env: env(), pollMs: 5 });
	const ctx = { signal: new AbortController().signal, progress() {}, remote() {}, cost() {} };
	mock.requests.length = 0;
	await assert.rejects(r.run({ ...base, model: 'gen4_turbo', prompt: 'FAIL please', stillFile: still }, ctx), /did not pass moderation.*SAFETY\.INPUT\.TEXT/);
	await assert.rejects(r.run({ ...base, model: 'gen4.5', prompt: '', stillFile: still }, ctx), /needs a prompt/);
	await assert.rejects(createRunway({ env: { ...env(), RUNWAY_API_KEY: 'wrong' }, pollMs: 5 }).run({ ...base, model: 'gen4_turbo', stillFile: still }, ctx), /401: the key was refused/);
	assert.equal(mock.requests.filter((q) => q.method === 'POST' && q.path === '/v1/image_to_video').length, 2);
});

test('through the manager: confirm, cancel deletes the remote task, results land as variants with task id + cost', async () => {
	const out = mkdtempSync(join(tmpdir(), 'sb-out-'));
	const m = createManager({ generators: [createRunway({ env: env(), pollMs: 10 })], out });
	const est = m.estimate({ ...base, model: 'gen4_turbo', duration: 4 });
	assert.equal(est.usd, 0.2);
	await assert.rejects(m.start({ ...base, model: 'gen4_turbo', duration: 4, stillFile: still }), /confirm the cost first/);
	const j = await m.start({ ...base, model: 'gen4_turbo', duration: 4, stillFile: still, still: 'out:s1c3/clean/sb-1-1.png', prompt: 'ok' }, { confirm: est.usd });
	await m.wait(j.id);
	const done = m.jobs()[0];
	assert.equal(done.status, 'succeeded');
	assert.deepEqual(done.files, ['sb-1-1--g1.mp4']);
	const line = JSON.parse(readFileSync(join(out, 's1c3/generated.jsonl'), 'utf8'));
	assert.equal(line.generator, 'runway');
	assert.equal(line.model, 'gen4_turbo');
	assert.equal(line.job, done.task);
	assert.deepEqual(line.cost, { credits: 20, usd: 0.2 });
	assert.deepEqual(line.images, ['sb-1-1.png']);
	assert.deepEqual(m.spend(), { usd: 0.2, credits: 20, jobs: 1 });

	const slow = await m.start({ ...base, model: 'gen4_turbo', duration: 2, stillFile: still, prompt: 'SLOW' }, { confirm: 0.1 });
	await new Promise((ok) => setTimeout(ok, 60));
	m.cancel(slow.id);
	await m.wait(slow.id);
	assert.equal(m.jobs()[0].status, 'cancelled');
	assert.equal(mock.tasks.get(m.jobs()[0].task).cancelled, true);
});
