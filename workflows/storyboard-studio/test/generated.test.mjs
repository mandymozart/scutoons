// The generated routes (generated.mjs, VARIANTS-CANVAS §3.7) on a temp folder tree: drops (png … mp4 / webm, the highest
// n + 1, the upload record), the old file list, the records of the picked shots (log order, earlier files, missing files,
// approved from the manifest's `from`, an older approval by its bytes), the marks (append-only, the last wins, `before`,
// the chosen not rejectable), approve (approve.mjs's args, then mark: null).
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { generatedRoutes } from '../generated.mjs';
import { tinyMp4 } from '../generators/test/runway-mock.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-generated-'));
after(() => rmSync(tmp, { recursive: true, force: true }));
const put = (rel, bytes) => { const f = join(tmp, rel); mkdirSync(dirname(f), { recursive: true }); writeFileSync(f, bytes); return f; };
const boards = join(tmp, 'boards'), out = join(boards, '_out');
const T = '2026-10-20T10:00:00.000Z';
const log = () => readFileSync(join(out, 'c1/generated.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
const dataUrl = (type, bytes) => `data:${type};base64,${Buffer.from(bytes).toString('base64')}`;
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==', 'base64');
const runs = [];
let runOk = true;
const routes = generatedRoutes({
	out, boards, now: () => new Date(T),
	run: async (script, args) => { runs.push([script, ...args]); return { ok: runOk, out: runOk ? 's-1: ok → dataset' : 'no shot' }; },
	fetchImpl: async () => ({ ok: true, headers: new Headers({ 'content-type': 'video/mp4' }), arrayBuffer: async () => tinyMp4(3) })
});
const get = (q) => routes['GET /api/generated'](q);

test('drops: images and videos, named by the highest n + 1 over the files and the log, each with an upload record', async () => {
	assert.deepEqual(await routes['POST /api/generated']({ ch: 'c1', id: 's-1', name: 'luma.png', data: dataUrl('image/png', PNG) }), { file: 's-1--g1.png', id: 's-1--g1', from: 'luma.png', generator: 'upload' });
	assert.equal((await routes['POST /api/generated']({ ch: 'c1', id: 's-1', name: 'clip.mp4', data: dataUrl('video/mp4', tinyMp4(5)) })).file, 's-1--g2.mp4');
	assert.equal((await routes['POST /api/generated']({ ch: 'c1', id: 's-1', name: 'clip.webm', data: dataUrl('video/webm', 'webm') })).file, 's-1--g3.webm');
	// dragged in from a web page (Luma's site): fetched – a video too
	assert.equal((await routes['POST /api/generated']({ ch: 'c1', id: 's-1', url: 'https://cdn.example/x/gen.mp4?sig=1' })).file, 's-1--g4.mp4');
	await assert.rejects(routes['POST /api/generated']({ ch: 'c1', id: 's-1', data: dataUrl('image/gif', 'gif') }), /png \/ jpeg \/ webp \/ mp4 \/ webm only/);
	await assert.rejects(routes['POST /api/generated']({ ch: '..', id: 's-1', data: dataUrl('image/png', PNG) }), /bad chapter/);
	await assert.rejects(routes['POST /api/generated']({ ch: 'c1', id: '../s', data: dataUrl('image/png', PNG) }), /bad id/);
	const recs = log();
	assert.deepEqual(recs.map((r) => [r.id, r.kind, r.generator, r.status, r.params]), [
		['s-1--g1', 'image', 'upload', 'succeeded', { name: 'luma.png', url: null }],
		['s-1--g2', 'video', 'upload', 'succeeded', { name: 'clip.mp4', url: null }],
		['s-1--g3', 'video', 'upload', 'succeeded', { name: 'clip.webm', url: null }],
		['s-1--g4', 'video', 'upload', 'succeeded', { name: 'gen.mp4', url: 'https://cdn.example/x/gen.mp4?sig=1' }]
	]);
	assert.deepEqual([recs[0].v, recs[0].date, recs[0].parents, recs[0].inputs, recs[0].mark, recs[0].marked], [2, T, [], [], null, null]);

	// the count + 1 bug is gone: g1 and g4 on disk, g6 only in the log (a failed job) → g7, nothing overwritten
	put('boards/_out/c1/generated/s-2--g1.png', 'one');
	put('boards/_out/c1/generated/s-2--g4.png', 'four');
	writeFileSync(join(out, 'c1/generated.jsonl'), JSON.stringify({ v: 2, id: 's-2--g6', shot: 's-2', file: null, kind: 'video', generator: 'runway', status: 'failed', parents: [], inputs: [], date: T, mark: null, marked: null }) + '\n', { flag: 'a' });
	assert.equal((await routes['POST /api/generated']({ ch: 'c1', id: 's-2', data: dataUrl('image/jpeg', 'jpg') })).file, 's-2--g7.jpg');
	assert.equal(readFileSync(join(out, 'c1/generated/s-2--g1.png'), 'utf8'), 'one');
});

test('GET without ids is refused (the Variants panel\'s file names left with it – the board asks for records)', () => {
	assert.throws(() => get({ ch: 'c1', id: 's-1' }), /ids missing/);
	assert.throws(() => get({ ch: '../x', ids: 's-1' }), /bad chapter/);
});

test('GET ?ch&ids: the picked shots\' records – log order, earlier files, missing files, approved from `from`', () => {
	// s-3: an old line (never read as a record), a file without any line, a record whose file is gone, a v2 result
	put('boards/_out/c1/generated/s-3--g1.png', 'old');
	put('boards/_out/c1/generated/s-3--g2.png', 'no line');
	put('boards/_out/c1/generated/s-3--g5.png', 'v2');
	const add = (o) => writeFileSync(join(out, 'c1/generated.jsonl'), JSON.stringify(o) + '\n', { flag: 'a' });
	add({ date: T, shot: 's-3', file: 's-3--g1.png', engine: 'comfy', images: ['s-3.png'], prompt: 'old' });
	add({ v: 2, id: 's-3--g4', shot: 's-3', file: 's-3--g4.png', kind: 'image', generator: 'comfy-local', status: 'succeeded', parents: [], inputs: [], date: T, mark: null, marked: null });
	add({ v: 2, id: 's-3--g5', shot: 's-3', file: 's-3--g5.png', kind: 'image', generator: 'runway', model: 'gen4_turbo', status: 'succeeded', parents: ['s-3--g4'], inputs: [{ role: 'still', key: 'out:c1/generated/s-3--g4.png' }], prompt: 'better', seed: 42, date: T, mark: null, marked: null });
	// approvals: s-1's video names its file (from) – the bytes of the copy don't matter then; s-3's image is an older
	// approval (no `from`): its copy's bytes find the file
	put('boards/dataset/manifest.jsonl', [
		{ date: '2026-10-19', toon: 'cat', chapter: 1, shot: 's-3', files: { render: true, chosen: 'chosen.png' } },
		{ date: '2026-10-20', toon: 'cat', chapter: 1, ch: 'c1', shot: 's-1', files: { render: true, chosen: 'chosen.mp4', from: 's-1--g2.mp4' } }
	].map((x) => JSON.stringify(x)).join('\n') + '\n');
	put('boards/dataset/files/c1/s-1/chosen.mp4', 'other bytes');
	put('boards/dataset/files/c1/s-3/chosen.png', 'old');

	const recs = get({ ch: 'c1', ids: 's-3,s-1' });
	assert.deepEqual(recs.map((r) => [r.id, r.file, r.earlier, r.missing, r.approved, r.status]), [
		['s-3--g4', 's-3--g4.png', false, true, false, 'succeeded'],
		['s-3--g5', 's-3--g5.png', false, false, false, 'succeeded'],
		['s-3--g1', 's-3--g1.png', true, false, true, null],
		['s-3--g2', 's-3--g2.png', true, false, false, null],
		['s-1--g1', 's-1--g1.png', false, false, false, 'succeeded'],
		['s-1--g2', 's-1--g2.mp4', false, false, true, 'succeeded'],
		['s-1--g3', 's-1--g3.webm', false, false, false, 'succeeded'],
		['s-1--g4', 's-1--g4.mp4', false, false, false, 'succeeded']
	]);
	const g5 = recs[1];
	assert.deepEqual([g5.parents, g5.seed, g5.prompt, g5.model, g5.url, g5.mark, g5.marked, g5.before, 'v' in g5], [['s-3--g4'], 42, 'better', 'gen4_turbo', '/out/c1/generated/s-3--g5.png', null, null, null, false]);
	// the card as an input (Improve on, Vary, More – generators/job-fields.mjs): its key (an earlier file and a missing one too)
	assert.deepEqual([g5.key, recs[2].key, recs[0].key], ['out:c1/generated/s-3--g5.png', 'out:c1/generated/s-3--g1.png', 'out:c1/generated/s-3--g4.png']);
	// an earlier file says only what the file says – nothing from its old line
	assert.deepEqual([recs[2].generator, recs[2].prompt, recs[2].kind, recs[2].inputs], [null, null, 'image', []]);
	assert.equal(recs.find((r) => r.id === 's-1--g2').ms, 5000);
	assert.throws(() => get({ ch: 'c1', ids: 's-1,../x' }), /bad id/);
	assert.deepEqual(get({ ch: 'c1', ids: 'none' }), []);
});

test('mark: one line per id, the last wins, `before` = what it replaced; unknown ids, bad marks and the chosen refused', () => {
	const mark = routes['POST /api/generated/mark'];
	const stamp = '2026-10-20T11:00:00.000Z';
	// one X: a card and its branch with one stamp; an earlier file can be marked too
	assert.deepEqual(mark({ ch: 'c1', ids: ['s-3--g4', 's-3--g5', 's-3--g2'], mark: 'rejected', marked: stamp }), { ids: ['s-3--g4', 's-3--g5', 's-3--g2'], mark: 'rejected', marked: stamp });
	assert.deepEqual(log().slice(-3), ['s-3--g4', 's-3--g5', 's-3--g2'].map((id) => ({ v: 2, id, mark: 'rejected', marked: stamp })));
	let byId = Object.fromEntries(get({ ch: 'c1', ids: 's-3' }).map((r) => [r.id, r]));
	assert.deepEqual([byId['s-3--g5'].mark, byId['s-3--g5'].marked, byId['s-3--g2'].mark, byId['s-3--g1'].mark], ['rejected', stamp, 'rejected', null]);
	// X again on g5 alone: back to none – `before` keeps the rejected stamp
	mark({ ch: 'c1', ids: ['s-3--g5'], mark: null });
	byId = Object.fromEntries(get({ ch: 'c1', ids: 's-3' }).map((r) => [r.id, r]));
	assert.deepEqual([byId['s-3--g5'].mark, byId['s-3--g5'].marked, byId['s-3--g5'].before], [null, null, { mark: 'rejected', marked: stamp }]);
	assert.equal(byId['s-3--g4'].mark, 'rejected');
	// review without a stamp: the server's time
	assert.deepEqual(mark({ ch: 'c1', ids: ['s-1--g1'], mark: 'review' }), { ids: ['s-1--g1'], mark: 'review', marked: T });
	assert.throws(() => mark({ ch: 'c1', ids: ['s-1--g99'], mark: 'review' }), /no generation s-1--g99/);
	assert.throws(() => mark({ ch: 'c1', ids: ['s-1--g1'], mark: 'chosen' }), /bad mark/);
	assert.throws(() => mark({ ch: 'c1', ids: [], mark: null }), /bad ids/);
	assert.throws(() => mark({ ch: 'c1', ids: ['s-1--g1'], mark: 'review', marked: 'yesterday' }), /bad marked/);
	const n = log().length;
	// the chosen (s-1's video, by `from`; s-3's image, by its bytes) can't be rejected – nothing is written then
	assert.throws(() => mark({ ch: 'c1', ids: ['s-1--g1', 's-1--g2'], mark: 'rejected', marked: stamp }), /s-1--g2 is chosen/);
	assert.throws(() => mark({ ch: 'c1', ids: ['s-3--g1'], mark: 'rejected', marked: stamp }), /s-3--g1 is chosen/);
	assert.equal(log().length, n);
	mark({ ch: 'c1', ids: ['s-1--g2'], mark: 'review' }); // review is fine
});

test('approve: approve.mjs with the file, then the card\'s mark back to none; a failed run writes nothing', async () => {
	const approve = routes['POST /api/generated/approve'];
	assert.deepEqual(await approve({ toon: 'cat', ch: 'c1', id: 's-1', file: 's-1--g1.png', note: 'yes' }), { ok: true, out: 's-1: ok → dataset' });
	assert.deepEqual(runs.at(-1), ['scripts/storyboard/approve.mjs', 'storyboards/cat/c1', 's-1', '--image', join(out, 'c1/generated/s-1--g1.png'), '--note', 'yes']);
	assert.deepEqual(log().at(-1), { v: 2, id: 's-1--g1', mark: null, marked: null });
	assert.equal(get({ ch: 'c1', ids: 's-1' }).find((r) => r.id === 's-1--g1').mark, null);
	runOk = false;
	const n = log().length;
	assert.equal((await approve({ toon: 'cat', ch: 'c1', id: 's-1', file: 's-1--g3.webm' })).ok, false);
	assert.equal(log().length, n);
	await assert.rejects(approve({ toon: 'cat', ch: 'c1', id: 's-1', file: 's-3--g1.png' }), /bad file/);
	await assert.rejects(approve({ toon: 'cat', ch: 'c1', id: 's-1', file: '../x.png' }), /bad file/);
});
