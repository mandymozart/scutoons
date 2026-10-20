// The timeline's filmstrip (clips.mjs, GET /api/thumbs): the query's checks (a video under /out/ only, the sizes), the
// plan (tiles at the video's aspect, the step that covers the clip, the caps), the cache key – and, when ffmpeg is there,
// one real sprite from a tiny ffmpeg-made video in a temp folder (skipped without ffmpeg); a sprite is served cacheable.
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { clipThumbs, sendFile, thumbName, thumbParams, thumbPlan } from '../clips.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-thumbs-'));
after(() => rmSync(tmp, { recursive: true, force: true }));
const out = join(tmp, '_out');
const ffmpeg = process.env.FFMPEG || 'ffmpeg';
const hasFfmpeg = !spawnSync(ffmpeg, ['-version'], { windowsHide: true }).error;

test('thumbParams: a video under /out/ in a chapter folder, the sizes checked, the tile height rounded up to 16', () => {
	const p = thumbParams({ url: '/out/s1c3/generated/sb-1-2--g2.mp4', w: '640.4', h: '90', ms: '5000' }, out);
	assert.equal(p.file, resolve(out, 's1c3', 'generated', 'sb-1-2--g2.mp4'));
	assert.deepEqual([p.ch, p.w, p.h, p.th, p.ms], ['s1c3', 640, 90, 96, 5000]);
	const enc = thumbParams({ url: `/out/s1c3/handheld/${encodeURIComponent('sb-1--t 1--beauty.webm')}`, w: 10, h: 1 }, out);
	assert.equal(enc.file, resolve(out, 's1c3', 'handheld', 'sb-1--t 1--beauty.webm'));
	assert.deepEqual([enc.h, enc.th, enc.ms], [1, 32, null], 'the smallest tile height; no ms');
	assert.equal(thumbParams({ url: '/out/s1c3/a.mov', w: 1, h: 99999 }, out).th, 480, 'the largest');
	assert.equal(thumbParams({ url: '/out/s1c3/a.mp4?v=3', w: 1, h: 20, ms: '-4' }, out).ms, null, 'a query is ignored, a bad ms dropped');
	for (const url of [undefined, '', 'x.mp4', '/media/s1c3/a.mp4', 'http://evil/out/s1c3/a.mp4', '//evil/out/s1c3/a.mp4',
		'/out/../catpope/s1c3/a.mp4', '/out/%2e%2e/a.mp4', '/out/s1c3/..%5C..%5Ca.mp4', '/out/s1c3/%2e%2e%2f%2e%2e%2fa.mp4',
		'/out/a.mp4', '/out/S1C3/a.mp4', '/out/s1c3/a.png', '/out/s1c3/thumbs/a.jpg', '/out/s1c3/a.mp4%00.png', '/out/%E0%A4%A.mp4'])
		assert.throws(() => thumbParams({ url, w: 100, h: 50 }, out), /bad url/, String(url));
	for (const [w, h] of [[0, 50], [100, 0], ['x', 50], [100, 'y'], [-5, 50], [2e6, 50], [undefined, 50]])
		assert.throws(() => thumbParams({ url: '/out/s1c3/a.mp4', w, h }, out), /bad size/, `${w} × ${h}`);
});

test('thumbPlan: tiles at the video\'s aspect, the smallest step that covers the clip, capped', () => {
	assert.deepEqual(thumbPlan({ width: 1280, height: 720, w: 1000, h: 96 }), { tw: 170, th: 96, n: 6 }, '1000 / 170.7 = 5.86 → 6');
	assert.deepEqual(thumbPlan({ width: 1280, height: 720, w: 1025, h: 96 }), { tw: 170, th: 96, n: 8 }, '1025 / 170.7 = 6.005 → the next step');
	assert.equal(thumbPlan({ width: 1280, height: 720, w: 30, h: 96 }).n, 1, 'narrower than a tile: one');
	assert.deepEqual(thumbPlan({ width: 1280, height: 720, w: 485, h: 89, th: 96 }), { tw: 170, th: 96, n: 4 },
		'counted at the height on screen (89: 158 px a tile, 485 / 158 = 3.07 → 4), made at the rounded one');
	assert.equal(thumbPlan({ width: 720, height: 1280, w: 1000, h: 96 }).tw, 54, 'portrait, even');
	assert.equal(thumbPlan({ width: 1280, height: 720, w: 1e6, h: 96 }).n, 64, 'at most 64');
	assert.equal(thumbPlan({ width: 1280, height: 720, w: 1e6, h: 480 }).n, 16, 'the sprite at most 16384 px: 16 × 854');
	assert.equal(thumbPlan({ width: 4000, height: 100, w: 1e6, h: 480 }).n, 1, 'one tile wider than the cap: still one');
});

test('thumbName: the same video, n and h → the same name; another mtime, size, n or h → another', () => {
	const st = { mtimeMs: 1000, size: 5 };
	const file = resolve(out, 's1c3', 'generated', 'sb-1 x--g2.mp4');
	const name = thumbName({ file, st, n: 8, h: 96 });
	assert.match(name, /^sb-1_x--g2--[0-9a-f]{12}-n8-h96\.jpg$/);
	assert.equal(thumbName({ file, st: { ...st }, n: 8, h: 96 }), name);
	for (const other of [{ file, st: { ...st, mtimeMs: 1001 }, n: 8, h: 96 }, { file, st: { ...st, size: 6 }, n: 8, h: 96 },
		{ file, st, n: 12, h: 96 }, { file, st, n: 8, h: 112 }, { file: file.replace('g2', 'g3'), st, n: 8, h: 96 }])
		assert.notEqual(thumbName(other), name);
});

/** a JPEG's size from its SOF marker */
function jpegSize(file) {
	const b = readFileSync(file);
	for (let at = 2; at + 9 < b.length; ) {
		if (b[at] !== 0xff) return null;
		const marker = b[at + 1], len = b.readUInt16BE(at + 2);
		if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) return { width: b.readUInt16BE(at + 7), height: b.readUInt16BE(at + 5) };
		at += 2 + len;
	}
	return null;
}

test('clipThumbs: one sprite of n tiles from a real video, made once, under the chapter\'s thumbs folder', { skip: !hasFfmpeg && 'no ffmpeg' }, async () => {
	const dir = join(out, 's1c3', 'generated');
	mkdirSync(dir, { recursive: true });
	const video = join(dir, 'sb-t--g1.mp4');
	const made = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=size=160x90:rate=24:duration=2', '-c:v', 'mpeg4', video], { windowsHide: true });
	assert.equal(made.status, 0, String(made.stderr));
	const a = await clipThumbs({ url: '/out/s1c3/generated/sb-t--g1.mp4', w: '200', h: '40' }, { out, ffmpeg });
	assert.deepEqual([a.n, a.tw, a.th], [3, 86, 48], 'h 40 → 48, 48 · 16 / 9 = 85.3 → 86, 200 / 86 = 2.3 → 3');
	assert.match(a.url, /^\/out\/s1c3\/thumbs\/sb-t--g1--[0-9a-f]{12}-n3-h48\.jpg$/);
	const sprite = join(out, ...a.url.split('/').slice(2));
	assert.ok(sprite.startsWith(out + sep));
	assert.deepEqual(jpegSize(sprite), { width: 3 * 86, height: 48 });
	const mtime = statSync(sprite).mtimeMs;
	const again = await clipThumbs({ url: '/out/s1c3/generated/sb-t--g1.mp4', w: '230', h: '48' }, { out, ffmpeg });
	assert.equal(again.url, a.url, 'the same step and height: the cached sprite');
	assert.equal(statSync(sprite).mtimeMs, mtime, 'not made again');
	const more = await clipThumbs({ url: '/out/s1c3/generated/sb-t--g1.mp4', w: '700', h: '48' }, { out, ffmpeg });
	assert.equal(more.n, 12, 'zoomed in: a denser sprite');
	assert.deepEqual(jpegSize(join(out, ...more.url.split('/').slice(2))), { width: 12 * 86, height: 48 });
	await assert.rejects(clipThumbs({ url: '/out/s1c3/generated/none.mp4', w: '200', h: '40' }, { out, ffmpeg }), /no such clip/);
});

test('clipThumbs: a file longer than its picture (a longer sound track) holds the last frame in its last tiles, not black', { skip: !hasFfmpeg && 'no ffmpeg' }, async () => {
	const dir = join(out, 's1c3', 'generated');
	mkdirSync(dir, { recursive: true });
	const video = join(dir, 'sb-t--g2.mp4');
	const made = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=size=160x90:rate=24:duration=1',
		'-f', 'lavfi', '-i', 'sine=duration=3', '-c:v', 'mpeg4', '-c:a', 'aac', video], { windowsHide: true });
	assert.equal(made.status, 0, String(made.stderr));
	const a = await clipThumbs({ url: '/out/s1c3/generated/sb-t--g2.mp4', w: '450', h: '48' }, { out, ffmpeg });
	assert.equal(a.n, 6, '450 / 85.3 = 5.3 → 6 tiles over the file\'s 3 s: four of them after the picture\'s 1 s');
	// the last tile's mean brightness (ffmpeg scales it to one grey pixel): testsrc's frame, not black
	const sprite = join(out, ...a.url.split('/').slice(2));
	const mean = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-i', sprite, '-vf', `crop=${a.tw}:${a.th}:${(a.n - 1) * a.tw}:0,scale=1:1`,
		'-f', 'rawvideo', '-pix_fmt', 'gray', '-'], { windowsHide: true });
	assert.equal(mean.status, 0, String(mean.stderr));
	assert.ok(mean.stdout[0] > 40, `the last tile is the last frame (mean ${mean.stdout[0]})`);
});

test('sendFile: a sprite (named by its video\'s hash) is kept by the browser, everything else no-store', async () => {
	const dir = join(out, 's1c5', 'thumbs');
	mkdirSync(dir, { recursive: true });
	const sprite = thumbName({ file: join(out, 's1c5', 'generated', 'sb-1--g1.mp4'), st: { mtimeMs: 1, size: 2 }, n: 4, h: 48 });
	for (const f of [sprite, 'other.jpg', 'sb-1--g1.mp4']) writeFileSync(join(dir, f), Buffer.from('x'));
	const server = createServer((req, res) => sendFile(req, res, join(dir, req.url.slice(1)), 'image/jpeg'));
	await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
	try {
		const cache = async (f) => (await fetch(`http://127.0.0.1:${server.address().port}/${f}`)).headers.get('cache-control');
		assert.equal(await cache(sprite), 'max-age=31536000, immutable');
		assert.equal(await cache('other.jpg'), 'no-store');
		assert.equal(await cache('sb-1--g1.mp4'), 'no-store');
	} finally {
		server.close();
	}
});

test('clipThumbs: a missing ffmpeg or a broken video is an error (the client falls back to the first frame)', async () => {
	const dir = join(out, 's1c4', 'generated');
	mkdirSync(dir, { recursive: true });
	writeFileSync(join(dir, 'sb-x--g1.mp4'), Buffer.from('not a video'));
	await assert.rejects(clipThumbs({ url: '/out/s1c4/generated/sb-x--g1.mp4', w: '200', h: '40' }, { out, ffmpeg: join(tmp, 'no-ffmpeg-here') }), /not found/);
	if (hasFfmpeg) await assert.rejects(clipThumbs({ url: '/out/s1c4/generated/sb-x--g1.mp4', w: '200', h: '40' }, { out, ffmpeg }), /no video stream/);
});
