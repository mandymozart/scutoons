// The one-time conversion of the generated log (scripts/convert-generated-log.mjs, VARIANTS-CANVAS §3.7 "The existing
// log") on a temp tree: every old line shape (comfy, luma, gemini, the studio's jobs), basenames resolved inside the
// chapter or reported (ambiguous / not found – never guessed), roles from the mode, parents, the take; the lines it keeps
// (overwritten, written over later, a bad name, an unknown generator); dry run, write (+ the .v1 backup), idempotent.
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { chaptersOf, convertLog, printReport } from '../scripts/convert-generated-log.mjs';
import { readLog } from '../../../scripts/storyboard/generated-log.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-convert-'));
after(() => rmSync(tmp, { recursive: true, force: true }));
const T = '2026-10-20T10:00:00.000Z', t = Date.parse(T) / 1000;
const put = (rel, bytes = 'x', when = t) => { const f = join(tmp, rel); mkdirSync(dirname(f), { recursive: true }); writeFileSync(f, bytes); utimesSync(f, when, when); return f; };
const boards = join(tmp, 'boards'), media = join(tmp, 'media'), out = join(boards, '_out');
const logPath = join(out, 'c1/generated.jsonl');

put('boards/_out/c1/clean/s-1.png');
put('boards/_out/c1/frames/s-1.png'); // the same name as the clean render: a studio job's "s-1.png" is ambiguous
for (const f of ['g1.png', 'g2.png', 'g3.png', 'g4.mp4', 'g5.mp4', 'g6.png', 'g10.png']) put(`boards/_out/c1/generated/s-1--${f}`);
put('boards/_out/c1/generated/s-1--g7.png', 'a drop over it', t + 3600); // written an hour after its line
put('boards/_out/c1/handheld/s-1--t2--beauty.mp4');
put('boards/dataset/files/c1/s-1/chosen.png');
put('boards/style/look.png');
put('boards/style/refs/cat/human/default/face-1.jpg');
put('boards/style/refs/cat/regular/default/face-1.jpg'); // two refs with one name
put('media/characters/plf/plf_full-neutral.webp');

const comfy = (file, over = {}) => ({ date: T, shot: 's-1', file, engine: 'comfy', model: 'qwen_image_edit_2509_fp8_e4m3fn.safetensors', lora: 'Lightning-4steps', steps: 4, cfg: 1, refMethod: null, look: 'photo', seed: 123, host: '127.0.0.1:8188', images: ['s-1.png', 'plf_full-neutral.webp', 'look.png'], prompt: 'P1', ...over });
const OLD = [
	JSON.stringify(comfy('s-1--g1.png')),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g2.png', engine: 'luma', model: 'uni-1', style: 'auto', aspect: '9:16', job: 'luma-abc', images: ['s-1.png', 'face-1.jpg'], prompt: 'P2' }),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g3.png', model: 'gemini-2.5-flash-image', images: ['s-1.png', 'look.png', 'gone.webp'], prompt: 'P3' }),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g4.mp4', engine: 'runway', generator: 'runway', model: 'gen4_turbo', job: 'task-1', mode: 'image_to_video', duration: 5, ratio: '720:1280', estimate: { credits: 25, usd: 0.25 }, cost: { credits: 25, usd: 0.25 }, images: ['s-1--g1.png'], prompt: 'P4' }),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g5.mp4', engine: 'runway', generator: 'runway', model: 'aleph2', job: 'task-2', mode: 'video_to_video', duration: 4.2, images: ['chosen.png', 's-1--t2--beauty.mp4'], prompt: 'P5' }),
	JSON.stringify(comfy('s-1--g6.png', { prompt: 'first – overwritten' })),
	JSON.stringify(comfy('s-1--g6.png', { prompt: 'second', seed: 7 })),
	JSON.stringify(comfy('s-1--g7.png')),
	JSON.stringify({ date: T, shot: 's-1', file: 'luma.png', engine: 'luma', images: [] }),
	'not json',
	JSON.stringify({ v: 2, id: 's-1--g8', shot: 's-1', file: 's-1--g8.png', kind: 'image', generator: 'upload', status: 'succeeded', parents: [], inputs: [], date: T, mark: null, marked: null }),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g9.png', images: [] }),
	JSON.stringify({ date: T, shot: 's-1', file: 's-1--g10.png', engine: 'fake', generator: 'fake', model: 'fake-1', job: 'fake-1', mode: 'fake', duration: 3, fake: true, images: ['s-1.png'], prompt: 'P10' })
].join('\n') + '\n';
writeFileSync(logPath, OLD);

test('dry run: the report, nothing written', () => {
	assert.deepEqual(chaptersOf(boards), ['c1']);
	const r = convertLog({ boards, media, ch: 'c1' });
	assert.deepEqual([r.lines, r.converted, r.v2, r.broken, r.written], [13, 7, 1, 1, false]);
	assert.deepEqual(r.kept.map((k) => [k.line, k.file, k.why.split(' (')[0].split(' – ')[0]]), [
		[6, 's-1--g6.png', 'overwritten'], [8, 's-1--g7.png', 'the file is newer than its line'], [9, 'luma.png', 'the file luma.png is not s-1--g<n>.<ext>'], [12, 's-1--g9.png', 'unknown generator']
	]);
	assert.deepEqual(r.unresolved.map((u) => [u.line, u.role, u.name, u.why.split(' (')[0]]), [
		[2, 'character', 'face-1.jpg', 'ambiguous'], [3, 'character', 'gone.webp', 'not found'], [13, 'still', 's-1.png', 'ambiguous']
	]);
	assert.match(printReport(r, false), /c1: 13 lines – 7 old → v2, 1 v2 already, 4 kept as they are, 1 broken[\s\S]*dry run – nothing written \(--write applies it\)/);
	assert.equal(readFileSync(logPath, 'utf8'), OLD);
	assert.equal(existsSync(`${logPath}.v1`), false);
});

test('--write: v2 records in the old lines\' places, the old log kept as .v1; what it kept stays as it was', () => {
	const r = convertLog({ boards, media, ch: 'c1', write: true });
	assert.equal(r.written, true);
	assert.equal(readFileSync(`${logPath}.v1`, 'utf8'), OLD);
	const raw = readFileSync(logPath, 'utf8').trim().split('\n');
	assert.equal(raw.length, 13);
	assert.equal(raw[9], 'not json');
	const { records, earlier, broken } = readLog(out, 'c1');
	assert.deepEqual([...records.keys()], ['s-1--g1', 's-1--g2', 's-1--g3', 's-1--g4', 's-1--g5', 's-1--g6', 's-1--g8', 's-1--g10']);
	assert.deepEqual([earlier.length, broken], [4, 1]);
	const g = (n) => records.get(`s-1--g${n}`);
	// comfy.mjs: image 1 = the clean render (its code), then the character (media), the style picture
	assert.deepEqual(g(1), {
		v: 2, id: 's-1--g1', shot: 's-1', file: 's-1--g1.png', kind: 'image', take: null, generator: 'comfy-local', model: 'qwen-image-edit', mode: 'comfy',
		job: null, status: 'succeeded', parents: [],
		inputs: [{ role: 'still', key: 'out:c1/clean/s-1.png' }, { role: 'character', key: 'media:characters/plf/plf_full-neutral.webp' }, { role: 'style', key: 'boards:style/look.png' }],
		prompt: 'P1', seed: 123, ratio: null, duration: null, estimate: null, cost: null, date: T, mark: null, marked: null, before: null,
		params: { unet: 'qwen_image_edit_2509_fp8_e4m3fn.safetensors', lora: 'Lightning-4steps', steps: 4, cfg: 1, look: 'photo', host: '127.0.0.1:8188' }
	});
	// luma: the layout as a reference; an ambiguous name stays a name
	assert.deepEqual([g(2).generator, g(2).model, g(2).job, g(2).ratio, g(2).params], ['luma', 'uni-1', 'luma-abc', '9:16', { style: 'auto' }]);
	assert.deepEqual(g(2).inputs, [{ role: 'reference', key: 'out:c1/clean/s-1.png' }, { role: 'character', key: null, name: 'face-1.jpg' }]);
	assert.deepEqual([g(3).generator, g(3).inputs.map((i) => [i.role, i.key])], ['gemini', [['reference', 'out:c1/clean/s-1.png'], ['style', 'boards:style/look.png'], ['character', null]]]);
	// the studio's runway jobs: a generated still → its parent; video to video: the control first, the take, the keyframe at 0
	assert.deepEqual([g(4).kind, g(4).parents, g(4).inputs, g(4).cost, g(4).duration, g(4).ratio], ['video', ['s-1--g1'], [{ role: 'still', key: 'out:c1/generated/s-1--g1.png' }], { credits: 25, usd: 0.25 }, 5, '720:1280']);
	assert.deepEqual([g(5).take, g(5).parents, g(5).inputs], ['t2', [], [{ role: 'control', key: 'out:c1/handheld/s-1--t2--beauty.mp4' }, { role: 'keyframe', key: 'boards:dataset/files/c1/s-1/chosen.png', at: 0 }]]);
	// the later of two lines for one file is the file's
	assert.deepEqual([g(6).prompt, g(6).seed], ['second', 7]);
	assert.deepEqual([g(10).generator, g(10).params, g(10).inputs], ['fake', { fake: true }, [{ role: 'still', key: null, name: 's-1.png' }]]);
});

test('a second run finds nothing to do and writes nothing; lines appended later stay v2', () => {
	writeFileSync(logPath, JSON.stringify({ v: 2, id: 's-1--g11', mark: 'review', marked: T }) + '\n', { flag: 'a' });
	const before = readFileSync(logPath, 'utf8');
	const r = convertLog({ boards, media, ch: 'c1', write: true });
	assert.deepEqual([r.converted, r.written, r.kept.length], [0, false, 4]);
	assert.match(printReport(r, true), /nothing to convert/);
	assert.equal(readFileSync(logPath, 'utf8'), before);
	assert.deepEqual(convertLog({ boards, media, ch: 'none' }), { ch: 'none', lines: 0, v2: 0, converted: 0, kept: [], unresolved: [], broken: 0, written: false });
});

test('old lines that turn up later (an old writer still running): a later --write converts them, the first backup stays', () => {
	put('boards/_out/c1/generated/s-1--g12.png');
	const old = JSON.stringify(comfy('s-1--g12.png', { prompt: 'late' }));
	writeFileSync(logPath, old + '\n', { flag: 'a' });
	const before = readFileSync(logPath, 'utf8');
	const r = convertLog({ boards, media, ch: 'c1', write: true });
	assert.deepEqual([r.converted, r.written], [1, true]);
	assert.match(r.backup, /^generated\.jsonl\.v1-\d+$/);
	assert.equal(readFileSync(join(out, 'c1', r.backup), 'utf8'), before);
	assert.equal(readFileSync(`${logPath}.v1`, 'utf8'), OLD);
	assert.equal(readLog(out, 'c1').records.get('s-1--g12').prompt, 'late');
});
