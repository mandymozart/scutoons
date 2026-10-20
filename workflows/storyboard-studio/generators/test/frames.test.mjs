// Frame → still (frames.mjs, VARIANTS-CANVAS §3.7): the name and the ffmpeg arguments of a first / last / ms frame, the
// cut with a stubbed ffmpeg runner – written to _out/<ch>/generated/frames/, named by the video's mtime + size (cut again
// when the video is replaced – also by a copy with an older timestamp; the frame before goes; a video changing during the
// cut doesn't keep it), the same frame asked twice at once cut once, a time inside the last frame = the last frame, a time
// past the end and anything that is not a video under _out refused, ffmpeg's failures said – and, when ffmpeg is there,
// real frames of a tiny ffmpeg-made video (skipped without; no GPU, no network).
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { createFrames, frameArgs, frameName } from '../frames.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-frames-'));
after(() => rmSync(tmp, { recursive: true, force: true }));
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==', 'base64');
const ffmpeg = process.env.FFMPEG || 'ffmpeg';
const hasFfmpeg = !spawnSync(ffmpeg, ['-version'], { windowsHide: true }).error;

/** a stub ffmpeg: writes a PNG to its last argument (the target), counts the calls; `fail` → no file + its message;
 *  `nothing(args)` → it ends well but writes no frame (a seek after the last frame's start); `during()` runs while it cuts */
function stub({ fail = null, wait = 0, nothing = () => false, during = () => {} } = {}) {
	const calls = [];
	const run = async (args) => {
		calls.push(args);
		if (wait) await new Promise((ok) => setTimeout(ok, wait));
		during(args);
		if (fail) return { err: fail, out: 'Invalid data found when processing input\n' };
		if (nothing(args)) return { err: null, out: '' };
		writeFileSync(args.at(-1), PNG);
		return { err: null, out: '' };
	};
	return { calls, run };
}
const put = (rel, bytes = 'video') => { const f = join(tmp, rel); mkdirSync(join(f, '..'), { recursive: true }); writeFileSync(f, bytes); return f; };
/** a frame's name with its video's tag (a hash of mtime + size) as <tag> */
const tag = (name) => name.replace(/--[0-9a-f]{10}\.png$/, '--<tag>.png');

test('frameName + frameArgs: the video\'s mtime + size in the name; first decoded from the start, last from the final second (-update), ms seeked', () => {
	const st = { mtimeMs: 1000, size: 5 };
	assert.match(frameName('/x/s-1--g2.mp4', 'first', st), /^s-1--g2--first--[0-9a-f]{10}\.png$/);
	assert.equal(tag(frameName('/x/s-1--t3--beauty.webm', 'last', st)), 's-1--t3--beauty--last--<tag>.png');
	assert.equal(tag(frameName('/x/s-1--g2.mp4', 1250, st)), 's-1--g2--1250ms--<tag>.png');
	assert.equal(frameName('/x/s-1--g2.mp4', 'first', st), frameName('/y/s-1--g2.mp4', 'first', { ...st }), 'the video as it is: the same name');
	assert.notEqual(frameName('/x/s-1--g2.mp4', 'first', st), frameName('/x/s-1--g2.mp4', 'first', { ...st, mtimeMs: 999 }), 'an older copy: another name');
	assert.notEqual(frameName('/x/s-1--g2.mp4', 'first', st), frameName('/x/s-1--g2.mp4', 'first', { ...st, size: 6 }), 'another size: another name');
	assert.deepEqual(frameArgs('v.mp4', 'first', 't.png'), ['-hide_banner', '-loglevel', 'error', '-y', '-i', 'v.mp4', '-an', '-frames:v', '1', 't.png']);
	assert.deepEqual(frameArgs('v.mp4', 'last', 't.png'), ['-hide_banner', '-loglevel', 'error', '-y', '-sseof', '-1', '-i', 'v.mp4', '-an', '-update', '1', 't.png']);
	assert.deepEqual(frameArgs('v.mp4', 1250, 't.png'), ['-hide_banner', '-loglevel', 'error', '-y', '-ss', '1.250', '-i', 'v.mp4', '-an', '-frames:v', '1', 't.png']);
});

test('cut: into generated/frames/, named by the video as it is – again when it is replaced (an older copy too), the frame before goes; one cut for two asks at once', async () => {
	const out = join(tmp, 'a/_out');
	const video = put('a/_out/c1/generated/s-1--g2.mp4');
	const ff = stub({ wait: 20 });
	const frames = createFrames({ out, run: ff.run, durationOf: () => 5000 });
	const dir = join(out, 'c1/generated/frames');
	const named = (frame) => join(dir, frameName(video, frame, statSync(video)));
	assert.equal(frames.fresh(video, 'first'), null, 'nothing cut yet – and fresh() cuts nothing');
	assert.equal(ff.calls.length, 0);
	const target = named('first');
	assert.equal(await frames.cut(video, 'first'), target);
	assert.deepEqual(readFileSync(target), PNG);
	assert.equal(ff.calls.length, 1);
	assert.equal(ff.calls[0][ff.calls[0].indexOf('-i') + 1], video);
	assert.match(ff.calls[0].at(-1), /s-1--g2--first--[0-9a-f]{10}\.\d+\.tmp\.png$/, 'written next to its place, then renamed');
	assert.deepEqual(readdirSync(dir).map(tag), ['s-1--g2--first--<tag>.png'], 'no tmp file left');
	// cached: the video is as it was
	assert.equal(await frames.cut(video, 'first'), target);
	assert.equal(frames.fresh(video, 'first'), target);
	assert.equal(ff.calls.length, 1);
	// the video re-rendered under its name (newer) → cut again; the frame before goes
	const later = new Date(statSync(target).mtimeMs + 60000);
	utimesSync(video, later, later);
	assert.equal(frames.fresh(video, 'first'), null);
	const again = await frames.cut(video, 'first');
	assert.equal(ff.calls.length, 2);
	assert.notEqual(again, target);
	assert.deepEqual(readdirSync(dir), [basename(again)], 'the frame before is gone');
	// replaced by a copy that keeps an OLDER timestamp (robocopy, a restored backup) → cut again too
	writeFileSync(video, 'another video');
	const older = new Date(statSync(again).mtimeMs - 3600000);
	utimesSync(video, older, older);
	assert.equal(frames.fresh(video, 'first'), null, 'an older video is not the one the frame was cut from');
	assert.equal(await frames.cut(video, 'first'), named('first'));
	assert.equal(ff.calls.length, 3);
	// the last frame asked twice at once: one ffmpeg
	const [a, b] = await Promise.all([frames.cut(video, 'last'), frames.cut(video, 'last')]);
	assert.equal(a, named('last'));
	assert.equal(a, b);
	assert.equal(ff.calls.length, 4);
	// a time: seeked; past the end refused before ffmpeg runs
	assert.equal(await frames.cut(video, 1250), named(1250));
	assert.deepEqual(ff.calls[4].slice(4, 6), ['-ss', '1.250']);
	await assert.rejects(frames.cut(video, 5001), /no frame at 5\.00 s – s-1--g2\.mp4 is 5\.00 s long/);
	assert.equal(ff.calls.length, 5);
	// a handheld render's frame goes to the same folder
	const pass = put('a/_out/c1/handheld/s-1--t3--beauty.mp4');
	const passFrame = await frames.cut(pass, 'first');
	assert.equal(tag(basename(passFrame)), 's-1--t3--beauty--first--<tag>.png');
	assert.equal(dirname(passFrame), dir);
});

test('a time inside the last frame (a seek there writes nothing) is the last frame; past the end it stays refused', async () => {
	const out = join(tmp, 'e/_out');
	const video = put('e/_out/c1/generated/s-1--g2.webm');
	// a 2.0 s clip at 12 fps: its last frame starts at 1917 ms – a seek to 1940 finds no frame
	const ff = stub({ nothing: (args) => args.includes('-ss') && Number(args[args.indexOf('-ss') + 1]) > 1.917 });
	const frames = createFrames({ out, run: ff.run, durationOf: () => 2000 });
	const file = await frames.cut(video, 1940);
	assert.equal(tag(basename(file)), 's-1--g2--1940ms--<tag>.png');
	assert.deepEqual(readFileSync(file), PNG);
	assert.deepEqual(ff.calls.map((a) => a.slice(4, 6)), [['-ss', '1.940'], ['-sseof', '-1']], 'the seek, then the last frame');
	assert.ok(await frames.cut(video, 2000), 'the very end');
	await assert.rejects(frames.cut(video, 2001), /no frame at 2\.00 s/);
	// an unknown length: no guess – what ffmpeg did
	const unknown = createFrames({ out, run: stub({ nothing: () => true }).run, durationOf: () => null });
	await assert.rejects(unknown.cut(video, 1990), /no frame at 1990 ms of s-1--g2\.webm/);
});

test('a video that changes while its frame is cut: that frame is not kept – cut again from the video as it is now', async () => {
	const out = join(tmp, 'f/_out');
	const video = put('f/_out/c1/generated/s-1--g2.mp4');
	let n = 0;
	// the first run: the video is rewritten under it
	const ff = stub({ during: () => { if (!n++) writeFileSync(video, 'rewritten while it was cut'); } });
	const frames = createFrames({ out, run: ff.run, durationOf: () => 5000 });
	const file = await frames.cut(video, 'first');
	assert.equal(ff.calls.length, 2, 'cut once more');
	assert.equal(basename(file), frameName(video, 'first', statSync(video)), 'named by the video as it is now');
	assert.deepEqual(readdirSync(join(out, 'c1/generated/frames')), [basename(file)], 'the cut of the old video left nothing');
	// changing on every run: said, nothing kept
	const always = createFrames({ out, run: stub({ during: () => writeFileSync(video, `changed ${n++}`) }).run, durationOf: () => 5000 });
	await assert.rejects(always.cut(video, 'last'), /s-1--g2\.mp4 changed while its frame was cut – try again/);
	assert.deepEqual(readdirSync(join(out, 'c1/generated/frames')).filter((f) => f.includes('--last')), []);
});

test('cut refuses what is not a video under _out (in a chapter folder), a bad frame, a missing video', async () => {
	const out = join(tmp, 'b/_out');
	const ff = stub();
	const frames = createFrames({ out, run: ff.run, durationOf: () => null });
	await assert.rejects(frames.cut(put('b/_out/c1/generated/s-1--g1.png', 'png'), 'first'), /not a video under _out/);
	await assert.rejects(frames.cut(put('b/elsewhere/c1/x.mp4'), 'first'), /not a video under _out/);
	await assert.rejects(frames.cut(put('b/_out/x.mp4'), 'first'), /not a video under _out/, 'no chapter folder');
	await assert.rejects(frames.cut(put('b/_out/C1/x.mp4'), 'first'), /not a video under _out/);
	const video = put('b/_out/c1/generated/s-1--g2.webm');
	for (const bad of ['middle', -1, 1.5, '12', null]) await assert.rejects(frames.cut(video, bad), /bad frame/, String(bad));
	await assert.rejects(frames.cut(join(out, 'c1/generated/s-1--g9.mp4'), 'first'), /no video s-1--g9\.mp4/);
	assert.equal(ff.calls.length, 0);
	// an unknown length: the time goes to ffmpeg
	await frames.cut(video, 99999);
	assert.equal(ff.calls.length, 1);
});

test('cut says why ffmpeg failed – not installed (FFMPEG=<path>), or its last line – and leaves nothing behind', async () => {
	const out = join(tmp, 'c/_out');
	const video = put('c/_out/c1/generated/s-1--g2.mp4');
	const missing = createFrames({ out, ffmpeg: 'ffmpeg-x', run: stub({ fail: Object.assign(new Error('spawn'), { code: 'ENOENT' }) }).run });
	await assert.rejects(missing.cut(video, 'first'), /ffmpeg-x not found \(FFMPEG=<path>\)/);
	const broken = createFrames({ out, run: stub({ fail: new Error('exit 1') }).run });
	await assert.rejects(broken.cut(video, 'last'), /no last frame of s-1--g2\.mp4 – ffmpeg: Invalid data found when processing input/);
	assert.deepEqual(readdirSync(join(out, 'c1/generated/frames')), []);
	assert.equal(existsSync(join(out, 'c1/generated/frames', frameName(video, 'last', statSync(video)))), false);
});

test('real frames of a tiny video (ffmpeg on the PATH or FFMPEG)', { skip: !hasFfmpeg && 'no ffmpeg' }, async () => {
	const out = join(tmp, 'd/_out');
	const video = join(out, 'c1/generated/s-1--g3.mp4');
	mkdirSync(join(out, 'c1/generated'), { recursive: true });
	// 1 s, 10 fps, 64 × 36: red, then blue from 0.5 s
	const made = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'color=c=red:s=64x36:r=10:d=0.5', '-f', 'lavfi', '-i', 'color=c=blue:s=64x36:r=10:d=0.5',
		'-filter_complex', '[0:v][1:v]concat=n=2:v=1[v]', '-map', '[v]', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', video], { windowsHide: true });
	assert.equal(made.status, 0, String(made.stderr));
	const frames = createFrames({ out, ffmpeg });
	const color = (file) => { const r = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-i', file, '-vf', 'scale=1:1', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'], { windowsHide: true }); return [...r.stdout]; };
	const isRed = ([r, , b]) => r > 200 && b < 60;
	const isBlue = ([r, , b]) => b > 200 && r < 60;
	const first = await frames.cut(video, 'first');
	assert.deepEqual([...readFileSync(first).subarray(1, 4)].map((c) => String.fromCharCode(c)).join(''), 'PNG');
	assert.ok(isRed(color(first)), `first: ${color(first)}`);
	assert.ok(isBlue(color(await frames.cut(video, 'last'))), 'last');
	assert.ok(isRed(color(await frames.cut(video, 200))), '0.2 s');
	assert.ok(isBlue(color(await frames.cut(video, 800))), '0.8 s');
	assert.ok(isBlue(color(await frames.cut(video, 990))), '0.99 s – inside the last frame');
});
