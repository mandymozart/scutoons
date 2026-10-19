// The clips API (clips.mjs): MP4 durations from the mvhd box (v0 / v1, a moov after the mdat, a 64-bit mdat, a
// fragmented file), WebM's Info Duration, missing / broken files → null, the chapter listing from a temp folder tree, and
// sendFile's byte ranges (a local server).
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { activeTake, chapterClips, mp4DurationMs, sendFile, webmDurationMs } from '../clips.mjs';

const tmp = mkdtempSync(join(tmpdir(), 'studio-clips-'));
after(() => rmSync(tmp, { recursive: true, force: true }));
const put = (rel, bytes, seconds) => {
	const f = join(tmp, rel);
	mkdirSync(dirname(f), { recursive: true });
	writeFileSync(f, bytes);
	if (seconds) utimesSync(f, seconds, seconds);
	return f;
};

// --- ISO BMFF builders -----------------------------------------------------------------------------------------------
const u32 = (...n) => { const b = Buffer.alloc(4 * n.length); n.forEach((x, i) => b.writeUInt32BE(x, 4 * i)); return b; };
const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; };
const box = (type, ...parts) => { const body = Buffer.concat(parts); return Buffer.concat([u32(8 + body.length), Buffer.from(type, 'latin1'), body]); };
const ftyp = box('ftyp', Buffer.from('isom'), u32(512), Buffer.from('isomiso2avc1mp41'));
const mvhd0 = (scale, d) => box('mvhd', u32(0, 0, 0, scale, d), Buffer.alloc(80));
const mvhd1 = (scale, d) => box('mvhd', u32(0x01000000), u64(0), u64(0), u32(scale), u64(d), Buffer.alloc(80));
const mdat = (n) => box('mdat', Buffer.alloc(n));
const mdat64 = (n) => Buffer.concat([u32(1), Buffer.from('mdat'), u64(16 + n), Buffer.alloc(n)]);
const mp4 = (scale, d) => Buffer.concat([ftyp, box('moov', mvhd0(scale, d)), mdat(64)]);
const trak = (id, scale, kind) => box('trak', box('tkhd', u32(0, 0, 0, id), Buffer.alloc(68)), box('mdia', box('mdhd', u32(0, 0, 0, scale, 0), Buffer.alloc(4)), box('hdlr', u32(0, 0), Buffer.from(kind), Buffer.alloc(13))));
const trex = (id, dur) => box('trex', u32(0, id, 1, dur, 0, 0));
const tfhd = (id, dflt) => box('tfhd', dflt ? u32(8, id, dflt) : u32(0, id));
const trun = (flags, samples) => box('trun', u32(flags, samples.length), ...(flags & 1 ? [u32(0)] : []), ...samples.map((s) => u32(...s)));

// --- EBML builders ---------------------------------------------------------------------------------------------------
const vsize = (n) => (n < 127 ? Buffer.from([0x80 | n]) : Buffer.concat([Buffer.from([0x01, 0]), (() => { const b = Buffer.alloc(6); b.writeUIntBE(n, 0, 6); return b; })()]));
const el = (id, ...parts) => { const body = Buffer.concat(parts); return Buffer.concat([Buffer.from(id), vsize(body.length), body]); };
const f64 = (x) => { const b = Buffer.alloc(8); b.writeDoubleBE(x); return b; };
const f32 = (x) => { const b = Buffer.alloc(4); b.writeFloatBE(x); return b; };
const UNKNOWN = Buffer.from([0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
function webm(infoParts) {
	const head = el([0x1a, 0x45, 0xdf, 0xa3], el([0x42, 0x82], Buffer.from('webm')));
	const seekHead = el([0x11, 0x4d, 0x9b, 0x74], el([0x4d, 0xbb], el([0x53, 0xab], Buffer.from([0x15, 0x49, 0xa9, 0x66]))));
	const info = el([0x15, 0x49, 0xa9, 0x66], el([0x2a, 0xd7, 0xb1], Buffer.from([0x0f, 0x42, 0x40])), el([0x4d, 0x80], Buffer.from('test')), ...infoParts);
	const cluster = Buffer.concat([Buffer.from([0x1f, 0x43, 0xb6, 0x75]), UNKNOWN, Buffer.alloc(32)]);
	return Buffer.concat([head, Buffer.from([0x18, 0x53, 0x80, 0x67]), UNKNOWN, seekHead, el([0xec], Buffer.alloc(200)), info, cluster]);
}

test('mp4: mvhd version 0 (32 bit) and version 1 (64 bit)', () => {
	assert.equal(mp4DurationMs(put('v0.mp4', mp4(1000, 5040))), 5040);
	assert.equal(mp4DurationMs(put('v0-90k.mp4', mp4(90000, 315000))), 3500);
	// 6 000 000 000 > 2^32 – only a 64-bit read gets 6000 s
	assert.equal(mp4DurationMs(put('v1.mp4', Buffer.concat([ftyp, box('moov', mvhd1(1e6, 6e9)), mdat(16)]))), 6e6);
});

test('mp4: a moov after the mdat (also a 64-bit mdat), an mdat to the end of the file', () => {
	assert.equal(mp4DurationMs(put('late.mp4', Buffer.concat([ftyp, mdat(1 << 20), box('moov', mvhd0(600, 2970))]))), 4950);
	assert.equal(mp4DurationMs(put('late64.mp4', Buffer.concat([ftyp, box('free'), mdat64(4096), box('moov', box('udta'), mvhd0(24, 120))]))), 5000);
	const toEnd = Buffer.concat([ftyp, box('moov', mvhd0(1000, 1234)), u32(0), Buffer.from('mdat'), Buffer.alloc(100)]);
	assert.equal(mp4DurationMs(put('to-end.mp4', toEnd)), 1234);
});

test('mp4: missing, broken, cut or unknown → null', () => {
	assert.equal(mp4DurationMs(join(tmp, 'nope.mp4')), null);
	assert.equal(mp4DurationMs(put('text.mp4', 'not a video at all, just some text in a file')), null);
	assert.equal(mp4DurationMs(put('empty.mp4', Buffer.alloc(0))), null);
	assert.equal(mp4DurationMs(put('no-moov.mp4', Buffer.concat([ftyp, mdat(64)]))), null);
	const whole = Buffer.concat([ftyp, box('moov', mvhd0(1000, 5040))]);
	assert.equal(mp4DurationMs(put('cut.mp4', whole.subarray(0, ftyp.length + 8 + 18))), null); // cut inside the mvhd
	assert.equal(mp4DurationMs(put('zero.mp4', mp4(1000, 0))), null);
	assert.equal(mp4DurationMs(put('ones.mp4', mp4(1000, 0xffffffff))), null);
	assert.equal(mp4DurationMs(put('noscale.mp4', mp4(0, 5000))), null);
});

test('mp4: fragmented – the mehd, else the video track\'s trun durations summed; no fragments → null', () => {
	const moov = (mvex) => box('moov', mvhd0(1000, 0), trak(1, 48000, 'soun'), trak(2, 12800, 'vide'), mvex);
	const frag1 = box('moof', box('mfhd', u32(0, 1)), box('traf', tfhd(1), trun(0x100, [[48000]])), box('traf', tfhd(2), trun(0, Array(25).fill([]))));
	const frag2 = box('moof', box('mfhd', u32(0, 2)), box('traf', tfhd(2, 1024), trun(0x301, [[6400, 10], [6400, 10], [6400, 10]])));
	const mvex = box('mvex', trex(1, 1024), trex(2, 512));
	// 25 × 512 (the trex default) + 3 × 6400 = 32000 at 12 800 per second = 2.5 s; the audio track is not counted
	assert.equal(mp4DurationMs(put('frag.mp4', Buffer.concat([ftyp, moov(mvex), frag1, mdat(32), frag2, mdat(32)]))), 2500);
	const withMehd = box('mvex', box('mehd', u32(0, 4321)), trex(2, 512));
	assert.equal(mp4DurationMs(put('frag-mehd.mp4', Buffer.concat([ftyp, moov(withMehd), frag1, mdat(32)]))), 4321);
	assert.equal(mp4DurationMs(put('frag-empty.mp4', Buffer.concat([ftyp, moov(mvex)]))), null);
});

test('webm: the Info\'s Duration × TimestampScale; none written / not a WebM → null', () => {
	assert.equal(webmDurationMs(put('a.webm', webm([el([0x44, 0x89], f64(4200))]))), 4200);
	assert.equal(webmDurationMs(put('b.webm', webm([el([0x44, 0x89], f32(1234.5))]))), 1235);
	assert.equal(webmDurationMs(put('recorder.webm', webm([]))), null);
	assert.equal(webmDurationMs(put('c.webm', mp4(1000, 5000))), null);
	assert.equal(webmDurationMs(join(tmp, 'nope.webm')), null);
});

test('activeTake: ms, the trim clamped, off / no active / no samples → null', () => {
	const doc = (t, on = true) => ({ on, active: 't1', takes: [{ id: 't1', ...t }] });
	assert.deepEqual(activeTake(doc({ in: 0.5, out: 2.25, samples: [{ t: 0 }, { t: 3.0004 }] })), { id: 't1', in: 500, out: 2250, length: 3000, kept: 1750 });
	assert.deepEqual(activeTake(doc({ in: -1, out: 9, samples: [{ t: 0 }, { t: 2 }] })), { id: 't1', in: 0, out: 2000, length: 2000, kept: 2000 });
	assert.deepEqual(activeTake(doc({ samples: [{ t: 0 }, { t: 2 }] })), { id: 't1', in: 0, out: 2000, length: 2000, kept: 2000 });
	assert.equal(activeTake(doc({ in: 0, out: 2, samples: [{ t: 0 }, { t: 2 }] }, false)), null);
	assert.equal(activeTake(doc({ in: 0, out: 0, samples: [] })), null);
	// shorter than 1 ms: nothing to lay out (a take of length 0 would be a clip of nothing)
	assert.equal(activeTake(doc({ in: 0, out: 0.0004, samples: [{ t: 0 }, { t: 0.0004 }] })), null);
	assert.deepEqual(activeTake(doc({ samples: [{ t: 0 }, { t: 0.001 }] })), { id: 't1', in: 0, out: 1, length: 1, kept: 1 });
	assert.equal(activeTake({ on: true, active: 'x', takes: [] }), null);
	assert.equal(activeTake(null), null);
});

test('chapterClips: takes on / off, renders of two takes, generated images + videos, the approved one, the cache', () => {
	const boards = join(tmp, 'boards');
	const T = Date.now() / 1000 - 1000;
	const takes = (t) => JSON.stringify({ on: true, active: 't2', takes: [{ id: 't1', in: 0, out: 1, samples: [{ t: 0 }, { t: 1 }] }, { id: 't2', in: 0.5, out: t, samples: [{ t: 0 }, { t: 1.5 }, { t: 3 }] }] });
	put('boards/cat/c1/takes/s-1.json', takes(2.25), T);
	put('boards/cat/c1/takes/s-2.json', JSON.stringify({ on: false, active: 't1', takes: [{ id: 't1', in: 0, out: 1, samples: [{ t: 0 }, { t: 1 }] }] }));
	put('boards/cat/c1/takes/s-4.json', JSON.stringify({ on: true, active: 't1', takes: [{ id: 't1', in: 0, out: 0, samples: [] }] }));
	put('boards/cat/c1/takes/broken.json', '{');
	put('boards/_out/c1/handheld/s-1--t1--beauty.mp4', mp4(1000, 3000));
	put('boards/_out/c1/handheld/s-1--t2--normals.mp4', mp4(600, 1050));
	put('boards/_out/c1/handheld/s-1--t2--beauty.mp4', mp4(600, 1050));
	put('boards/_out/c1/handheld/s-1--t2--beauty.webm', webm([el([0x44, 0x89], f64(1750))]));
	put('boards/_out/c1/handheld/s-1--t10--grey.mp4', mp4(1000, 800));
	put('boards/_out/c1/handheld/s-1--t2/beauty/f00001.png', 'frame');
	put('boards/_out/c1/handheld/s-2--t1--beauty.mp4', mp4(1000, 1000));
	put('boards/_out/c1/handheld/notes.txt', 'x');
	const g1 = Buffer.from('a png');
	const g2 = mp4(1000, 5000);
	put('boards/_out/c1/generated/s-1--g1.png', g1);
	put('boards/_out/c1/generated/s-1--g2.mp4', g2);
	put('boards/_out/c1/generated/s-1--g3.webm', webm([]));
	put('boards/_out/c1/generated/s-1--g10.jpg', 'a jpg');
	put('boards/_out/c1/generated/s-3--g1.mp4', 'broken');
	put('boards/_out/c1/generated/s-3--notes.txt', 'x');
	put('boards/_out/c1/generated.jsonl', '{"shot":"s-1","file":"s-1--g2.mp4","model":"gen4_turbo","duration":5}\n');
	put('boards/dataset/files/c1/s-1/chosen.png', g1, T - 100); // an older approval
	put('boards/dataset/files/c1/s-1/chosen.mp4', g2, T); // the newest: the video

	const gen = (file, ms, video, approved = false) => ({ file, url: `/out/c1/generated/${file}`, ms, video, approved });
	const ren = (file, pass, take, ms) => ({ file, url: `/out/c1/handheld/${file}`, pass, take, ms });
	assert.deepEqual(chapterClips({ boards, toon: 'cat', ch: 'c1' }), {
		's-1': {
			take: { id: 't2', in: 500, out: 2250, length: 3000, kept: 1750 },
			rendered: [
				ren('s-1--t10--grey.mp4', 'grey', 't10', 800),
				ren('s-1--t2--beauty.mp4', 'beauty', 't2', 1750),
				ren('s-1--t2--beauty.webm', 'beauty', 't2', 1750),
				ren('s-1--t2--normals.mp4', 'normals', 't2', 1750),
				ren('s-1--t1--beauty.mp4', 'beauty', 't1', 3000)
			],
			// approved per kind: the newest chosen video (g2) and the newest chosen image (g1, an older approval – still the panel's)
			generated: [gen('s-1--g10.jpg', null, false), gen('s-1--g3.webm', null, true), gen('s-1--g2.mp4', 5000, true, true), gen('s-1--g1.png', null, false, true)]
		},
		's-2': { take: null, rendered: [ren('s-2--t1--beauty.mp4', 'beauty', 't1', 1000)], generated: [] },
		's-3': { take: null, rendered: [], generated: [gen('s-3--g1.mp4', null, true)] }
	});

	// a changed file is read again (same size, a newer mtime): the trim, the approval – a newer chosen image (g10's bytes,
	// as long as g1's) moves the image's mark only, the approved video stays
	put('boards/cat/c1/takes/s-1.json', takes(2.75), T + 10);
	put('boards/dataset/files/c1/s-1/chosen.png', 'a jpg', T + 10);
	const again = chapterClips({ boards, toon: 'cat', ch: 'c1' })['s-1'];
	assert.deepEqual(again.take, { id: 't2', in: 500, out: 2750, length: 3000, kept: 2250 });
	assert.deepEqual(again.generated.filter((g) => g.approved).map((g) => g.file), ['s-1--g10.jpg', 's-1--g2.mp4']);
});

test('sendFile: byte ranges as 206 (a <video> seeks only then), a plain GET whole, an unsatisfiable range 416', async () => {
	put('serve/clip.mp4', Buffer.from(Array.from({ length: 100 }, (_, i) => i)));
	put('serve/empty.mp4', Buffer.alloc(0));
	const server = createServer((req, res) => sendFile(req, res, join(tmp, 'serve', req.url.slice(1)), 'video/mp4'));
	await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
	const base = `http://127.0.0.1:${server.address().port}`;
	const get = async (path, range) => {
		const r = await fetch(base + path, range ? { headers: { range } } : {});
		return { status: r.status, range: r.headers.get('content-range'), accept: r.headers.get('accept-ranges'), type: r.headers.get('content-type'), bytes: [...new Uint8Array(await r.arrayBuffer())] };
	};
	try {
		const whole = await get('/clip.mp4');
		assert.deepEqual([whole.status, whole.accept, whole.type, whole.bytes.length], [200, 'bytes', 'video/mp4', 100]);
		const open = await get('/clip.mp4', 'bytes=0-'); // what a <video> asks first
		assert.deepEqual([open.status, open.range, open.bytes.length], [206, 'bytes 0-99/100', 100]);
		const mid = await get('/clip.mp4', 'bytes=10-19');
		assert.deepEqual([mid.status, mid.range, mid.bytes], [206, 'bytes 10-19/100', [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]]);
		const tail = await get('/clip.mp4', 'bytes=-3');
		assert.deepEqual([tail.status, tail.range, tail.bytes], [206, 'bytes 97-99/100', [97, 98, 99]]);
		const over = await get('/clip.mp4', 'bytes=90-500'); // the end clamped
		assert.deepEqual([over.status, over.range, over.bytes.length], [206, 'bytes 90-99/100', 10]);
		assert.deepEqual([(await get('/clip.mp4', 'bytes=100-')).status, (await get('/clip.mp4', 'bytes=5-2')).status], [416, 416]);
		assert.equal((await get('/clip.mp4', 'bytes=0-1,5-6')).status, 200); // several ranges: the whole file
		const empty = await get('/empty.mp4', 'bytes=0-');
		assert.deepEqual([empty.status, empty.bytes.length], [200, 0]);
	} finally {
		server.close();
	}
});

test('chapterClips: an empty chapter → {}, a bad toon / chapter throws', () => {
	assert.deepEqual(chapterClips({ boards: join(tmp, 'boards'), toon: 'cat', ch: 'none' }), {});
	assert.throws(() => chapterClips({ boards: tmp, toon: '..', ch: 'c1' }), /bad chapter/);
	assert.throws(() => chapterClips({ boards: tmp, toon: 'cat', ch: 'c1/../x' }), /bad chapter/);
	assert.throws(() => chapterClips({ boards: tmp }), /bad chapter/);
});
