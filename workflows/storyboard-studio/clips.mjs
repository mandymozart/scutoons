// The clips of a chapter's shots (owner 2026-10-19: "depending on how long a camera is recorded and or trimmed to, thats
// the length of that clip for that shot") – what the studio's timeline lays out for each picked shot, falling back
// generated → rendered → take → still (research/TIMELINE.md). `GET /api/clips?toon=&ch=` (api.mjs) answers
//   { [shot]: { take: { id, in, out, length, kept } | null,      – the active take while the handheld camera is on (ms)
//               rendered: [{ file, url, pass, take, ms }],     – _out/<ch>/handheld/<shot>--<take>--<pass>.mp4|webm,
//                                                                 the newest take first, the beauty pass first
//               generated: [{ file, url, ms, video, approved }] } } – _out/<ch>/generated/<shot>--g<n>.<ext>, newest
//                                                                 first; approved = the shot's chosen video and chosen image:
//                                                                 approve.mjs's `from` (an older approval: the dataset copy's bytes)
// Shots with nothing are left out. ms = the real duration read from the video file (MP4 / MOV: the mvhd box wherever
// the moov sits, a fragmented file summed; WebM: the Info's Duration), null = unknown – the client then takes the
// video's metadata; images: ms null, video false. Everything read is cached by mtime + size, so a chapter with many
// files answers from memory. sendFile: api.mjs serves its mounts (/out/ …) with byte ranges – a <video> seeks only in
// a file it got as 206. The timeline's filmstrip: GET /api/thumbs (clipThumbs, at the end – ffmpeg). No dependencies.
// test/clips.test.mjs, test/thumbs.test.mjs.

import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, createReadStream, fstatSync, mkdirSync, openSync, readdirSync, readFileSync, readSync, renameSync, rmSync, statSync } from 'node:fs';
import { basename, relative, resolve, sep } from 'node:path';
import { pipeline } from 'node:stream';

const PASSES = ['beauty', 'depth', 'normals', 'grey']; // api.mjs PASSES – the beauty pass first
const RENDER = new RegExp(`^([\\w-]+?)--([\\w-]+?)--(${PASSES.join('|')})\\.(mp4|webm)$`);
const GENERATED = /^([\w-]+?)--([\w.-]*?)\.(png|jpe?g|webp|mp4|webm|mov|m4v)$/i;
const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const MAX_BOX = 64 * 1024 * 1024; // a moov / moof larger than this isn't read (→ null)
const WEBM_HEAD = 1024 * 1024; // the part of a WebM searched for its Info

// --- the cache: one value per key, kept while the files' mtime + size stay ---------------------------------------
const memo = new Map();
function cached(key, stats, compute) {
	const tag = stats.map((s) => `${s.mtimeMs}:${s.size}`).join('|');
	const hit = memo.get(key);
	if (hit && hit.tag === tag) return hit.value;
	if (memo.size > 5000) memo.clear();
	const value = compute();
	memo.set(key, { tag, value });
	return value;
}
const statOf = (file) => { try { const s = statSync(file); return s.isFile() ? s : null; } catch { return null; } };
const list = (dir) => { try { return readdirSync(dir); } catch { return []; } };
function withFile(file, fn) {
	let fd;
	try { fd = openSync(file, 'r'); return fn(fd, fstatSync(fd).size); } catch { return null; } finally { if (fd !== undefined) closeSync(fd); }
}
const readAt = (fd, pos, len) => { const b = Buffer.alloc(len); return b.subarray(0, readSync(fd, b, 0, len, pos)); };

// --- MP4 / MOV (ISO BMFF) ------------------------------------------------------------------------------------------
/** a box header at buf[at]: { type, size (to the end when 0), header } or null */
function boxHead(buf, at, rest) {
	if (buf.length < at + 8) return null;
	let size = buf.readUInt32BE(at), header = 8;
	if (size === 1) { if (buf.length < at + 16) return null; size = Number(buf.readBigUInt64BE(at + 8)); header = 16; }
	else if (size === 0) size = rest;
	return size >= header ? { type: buf.toString('latin1', at + 4, at + 8), size, header } : null;
}
/** the boxes inside buf: { type, body } – stops at one that doesn't fit */
function* boxes(buf) {
	for (let pos = 0; pos + 8 <= buf.length; ) {
		const head = boxHead(buf, pos, buf.length - pos);
		if (!head || pos + head.size > buf.length) return;
		yield { type: head.type, body: buf.subarray(pos + head.header, pos + head.size) };
		pos += head.size;
	}
}
const child = (buf, type) => { for (const b of boxes(buf)) if (b.type === type) return b.body; return null; };
/** the file's top-level boxes, read header by header (an mdat is skipped, never read) */
function* topBoxes(fd, size) {
	for (let pos = 0, n = 0; pos + 8 <= size && n < 100000; n++) {
		const head = boxHead(readAt(fd, pos, 16), 0, size - pos);
		if (!head) return;
		yield { ...head, at: pos };
		pos += head.size;
	}
}
const bodyOf = (fd, box) => (box.size > MAX_BOX ? null : readAt(fd, box.at + box.header, box.size - box.header));
/** mvhd / mdhd: { scale, duration } (version 0: 32 bit, 1: 64 bit; all ones = unknown → 0) */
function timing(b) {
	const v1 = b?.[0] === 1;
	if (!b || b.length < (v1 ? 32 : 20)) return null;
	const d = v1 ? b.readBigUInt64BE(24) : BigInt(b.readUInt32BE(16));
	return { scale: b.readUInt32BE(v1 ? 20 : 12), duration: d === (v1 ? 0xffffffffffffffffn : 0xffffffffn) ? 0 : Number(d) };
}

/** the duration (ms) of an MP4 / MOV: moov → mvhd, wherever the moov sits; fragmented (an mvex): its mehd, else the
 *  moov's own samples + every moof's samples of the first video track; null = unknown or not an MP4 */
export function mp4DurationMs(file) {
	return withFile(file, (fd, size) => {
		let moov = null;
		const moofs = [];
		for (const box of topBoxes(fd, size)) {
			if (box.type === 'moov') moov = bodyOf(fd, box);
			else if (box.type === 'moof') moofs.push(box);
		}
		const head = moov && timing(child(moov, 'mvhd'));
		if (!head?.scale) return null;
		const mvex = child(moov, 'mvex');
		if (!mvex) return head.duration > 0 ? Math.round((head.duration / head.scale) * 1000) : null;
		const mehd = child(mvex, 'mehd');
		const whole = mehd?.length >= (mehd?.[0] === 1 ? 12 : 8) ? (mehd[0] === 1 ? Number(mehd.readBigUInt64BE(4)) : mehd.readUInt32BE(4)) : 0;
		if (whole > 0 && whole !== 0xffffffff) return Math.round((whole / head.scale) * 1000);
		const seconds = head.duration / head.scale + (fragments(fd, moov, mvex, moofs) ?? 0);
		return seconds > 0 ? Math.round(seconds * 1000) : null;
	});
}

/** the fragments' length (s): the trun sample durations of one track (the first video track, else the first) */
function fragments(fd, moov, mvex, moofs) {
	const tracks = [...boxes(moov)].filter((b) => b.type === 'trak').map(({ body }) => {
		const tkhd = child(body, 'tkhd'), mdia = child(body, 'mdia');
		const hdlr = mdia && child(mdia, 'hdlr');
		return {
			id: tkhd?.length >= 24 ? tkhd.readUInt32BE(tkhd[0] === 1 ? 20 : 12) : null,
			scale: timing(mdia && child(mdia, 'mdhd'))?.scale,
			video: hdlr?.length >= 12 && hdlr.toString('latin1', 8, 12) === 'vide'
		};
	}).filter((t) => t.id != null && t.scale > 0);
	const track = tracks.find((t) => t.video) ?? tracks[0];
	if (!track) return null;
	let dflt = 0;
	for (const { type, body } of boxes(mvex)) if (type === 'trex' && body.length >= 16 && body.readUInt32BE(4) === track.id) dflt = body.readUInt32BE(12);
	let sum = 0, found = false;
	for (const box of moofs) {
		const moof = bodyOf(fd, box);
		for (const { type, body: traf } of moof ? boxes(moof) : []) {
			const tfhd = type === 'traf' && child(traf, 'tfhd');
			if (!tfhd || tfhd.length < 8 || tfhd.readUInt32BE(4) !== track.id) continue;
			const tf = tfhd.readUInt32BE(0) & 0xffffff;
			const at = 8 + (tf & 1 ? 8 : 0) + (tf & 2 ? 4 : 0);
			const own = tf & 8 && tfhd.length >= at + 4 ? tfhd.readUInt32BE(at) : dflt;
			for (const { type: t, body: trun } of boxes(traf)) {
				if (t !== 'trun' || trun.length < 8) continue;
				const fl = trun.readUInt32BE(0) & 0xffffff, count = trun.readUInt32BE(4);
				const first = 8 + (fl & 1 ? 4 : 0) + (fl & 4 ? 4 : 0);
				const stride = 4 * [0x100, 0x200, 0x400, 0x800].filter((f) => fl & f).length;
				found = true;
				if (!(fl & 0x100)) { sum += count * own; continue; }
				for (let i = 0; i < count && first + i * stride + 4 <= trun.length; i++) sum += trun.readUInt32BE(first + i * stride);
			}
		}
	}
	return found && sum > 0 ? sum / track.scale : null;
}

// --- WebM / Matroska (EBML) ----------------------------------------------------------------------------------------
/** a variable-length integer at buf[at]: { length, value } (an id keeps its marker; a size of all ones = null, unknown) */
function vint(buf, at, id) {
	const first = buf[at];
	if (!first) return null;
	const length = Math.clz32(first) - 23;
	if (at + length > buf.length) return null;
	const mask = 0xff >> length;
	let value = id ? first : first & mask, ones = (first & mask) === mask;
	for (let i = 1; i < length; i++) { value = value * 256 + buf[at + i]; ones &&= buf[at + i] === 0xff; }
	return { length, value: !id && ones ? null : value };
}
/** an EBML element at buf[at]: { id, size (null = unknown), data (where its content starts) } or null */
function element(buf, at) {
	const id = vint(buf, at, true);
	const size = id && vint(buf, at + id.length, false);
	return size ? { id: id.value, size: size.value, data: at + id.length + size.length } : null;
}

/** the duration (ms) of a WebM: Segment → Info → Duration × TimestampScale, in the file's first MB (where ffmpeg and
 *  the browsers write it); null = none written (a MediaRecorder file), not found, or not a WebM */
export function webmDurationMs(file) {
	return withFile(file, (fd, size) => {
		const buf = readAt(fd, 0, Math.min(size, WEBM_HEAD));
		const ebml = element(buf, 0);
		if (ebml?.id !== 0x1a45dfa3 || ebml.size == null) return null;
		const seg = element(buf, ebml.data + ebml.size);
		if (seg?.id !== 0x18538067) return null;
		for (let at = seg.data; ; ) {
			const el = element(buf, at);
			if (!el || el.id === 0x1f43b675) return null; // a Cluster before any Info
			if (el.id === 0x1549a966) return webmInfo(buf, el);
			if (el.size == null) return null;
			at = el.data + el.size;
		}
	});
}
function webmInfo(buf, info) {
	let scale = 1e6, duration = null;
	const end = info.size == null ? buf.length : Math.min(buf.length, info.data + info.size);
	for (let at = info.data; at < end; ) {
		const el = element(buf, at);
		if (!el || el.size == null || el.data + el.size > end) break;
		if (el.id === 0x2ad7b1) { scale = 0; for (let i = 0; i < el.size; i++) scale = scale * 256 + buf[el.data + i]; }
		if (el.id === 0x4489) duration = el.size === 4 ? buf.readFloatBE(el.data) : el.size === 8 ? buf.readDoubleBE(el.data) : null;
		at = el.data + el.size;
	}
	return duration > 0 && scale > 0 ? Math.round((duration * scale) / 1e6) : null;
}

/** a video file's duration (ms) by its extension; null = unknown */
export const videoDurationMs = (file) => (/\.(webm|mkv)$/i.test(file) ? webmDurationMs(file) : mp4DurationMs(file));

// --- the listing ---------------------------------------------------------------------------------------------------
/** a takes file's doc (api.mjs writeTakes, seconds) → its active take in ms, or null (camera off, no active take, no
 *  samples, shorter than 1 ms) */
export function activeTake(doc) {
	const take = doc?.on ? doc.takes?.find((t) => t.id === doc.active) : null;
	const last = take?.samples?.at(-1)?.t;
	const length = Math.round(last * 1000);
	if (!(length > 0)) return null;
	const tin = Math.min(length, Math.max(0, Math.round((Number(take.in) || 0) * 1000)));
	const tout = Math.max(tin, Math.min(length, Math.round((Number.isFinite(Number(take.out ?? NaN)) ? Number(take.out) : last) * 1000)));
	return { id: take.id, in: tin, out: tout, length, kept: tout - tin };
}
const readTake = (file) => { try { return activeTake(JSON.parse(readFileSync(file, 'utf8'))); } catch { return null; } };
const durationOf = (file, st) => cached(`ms:${file}`, [st], () => videoDurationMs(file));

/** two files with the same bytes (the approved copy and its source), read in 1 MB steps */
function sameBytes(a, b) {
	let fa, fb;
	try {
		fa = openSync(a, 'r');
		fb = openSync(b, 'r');
		const x = Buffer.alloc(1 << 20), y = Buffer.alloc(1 << 20);
		for (let pos = 0; ; pos += x.length) {
			const n = readSync(fa, x, 0, x.length, pos), m = readSync(fb, y, 0, y.length, pos);
			if (n !== m || !x.subarray(0, n).equals(y.subarray(0, m))) return false;
			if (n < x.length) return true;
		}
	} catch { return false; } finally { for (const fd of [fa, fb]) if (fd !== undefined) closeSync(fd); }
}

/** the approvals that name their generated file (approve.mjs `from`, since 2026-10-20 – VARIANTS-CANVAS §3.5) from
 *  storyboards/dataset/manifest.jsonl: "<ch>/<shot>" → { image?, video? } – per kind the newest approval's `from`
 *  (null = the chosen was no generated file); a kind without such an approval is missing */
function chosenFrom(boards) {
	const file = resolve(boards, 'dataset', 'manifest.jsonl');
	const st = statOf(file);
	return st ? cached(`manifest:${file}`, [st], () => {
		const map = new Map();
		for (const line of readFileSync(file, 'utf8').split('\n')) {
			let j;
			try { j = JSON.parse(line); } catch { continue; }
			const f = j?.files;
			if (!j?.ch || !j.shot || !f?.chosen || !('from' in f)) continue;
			const at = map.get(`${j.ch}/${j.shot}`) ?? {};
			at[VIDEO.test(f.chosen) ? 'video' : 'image'] = f.from ?? null;
			map.set(`${j.ch}/${j.shot}`, at);
		}
		return map;
	}) : new Map();
}

/** the shot's generated files that are its chosen ones – per kind (a chosen.png and a chosen.mp4 live side by side: the
 *  panel's still and the clip's video are two choices): the newest approval's `from`; an older approval (without one):
 *  the newest copy of that kind in storyboards/dataset/files/<ch>/<shot>/, matched by its bytes to the newest generated
 *  file. files = names in _out/<ch>/generated/; a Set of names */
export function approvedOf({ boards, ch, shot, files }) {
	const gd = resolve(boards, '_out', ch, 'generated');
	const items = files.map((name) => ({ name, file: resolve(gd, name), st: statOf(resolve(gd, name)), n: Number(/--g(\d+)\./.exec(name)?.[1] ?? 0) }))
		.filter((g) => g.st).sort((a, b) => b.n - a.n || b.st.mtimeMs - a.st.mtimeMs || b.name.localeCompare(a.name));
	const from = chosenFrom(boards).get(`${ch}/${shot}`) ?? {};
	const dir = resolve(boards, 'dataset', 'files', ch, shot);
	const copies = list(dir).filter((f) => f.startsWith('chosen.')).map((f) => ({ file: resolve(dir, f), st: statOf(resolve(dir, f)) }))
		.filter((c) => c.st).sort((a, b) => b.st.mtimeMs - a.st.mtimeMs);
	const approved = new Set();
	for (const video of [true, false]) {
		const kind = video ? 'video' : 'image';
		if (kind in from) {
			if (items.some((g) => g.name === from[kind])) approved.add(from[kind]);
			continue;
		}
		const chosen = copies.find((c) => VIDEO.test(c.file) === video);
		const hit = chosen && items.find((g) => VIDEO.test(g.name) === video && g.st.size === chosen.st.size && cached(`same:${chosen.file}|${g.file}`, [chosen.st, g.st], () => sameBytes(chosen.file, g.file)));
		if (hit) approved.add(hit.name);
	}
	return approved;
}

/** a video file's duration (ms, cached by mtime + size), null = unknown or not there */
export function clipMs(file) {
	const st = statOf(file);
	return st ? durationOf(file, st) : null;
}

/** the clips of a chapter's shots (the shape at the top); boards = the storyboards folder, toon / ch as in api.mjs */
export function chapterClips({ boards, toon, ch }) {
	if (!/^[a-z0-9-]+$/.test(toon ?? '') || !/^[a-z0-9-]+$/.test(ch ?? '')) throw new Error('bad chapter');
	const shots = {};
	const at = (id) => (shots[id] ??= { take: null, rendered: [], generated: [] });
	const td = resolve(boards, toon, ch, 'takes');
	for (const f of list(td)) {
		const id = /^([\w-]+)\.json$/.exec(f)?.[1];
		const st = id && statOf(resolve(td, f));
		const take = st && cached(`take:${resolve(td, f)}`, [st], () => readTake(resolve(td, f)));
		if (take) at(id).take = take;
	}
	const hd = resolve(boards, '_out', ch, 'handheld');
	for (const f of list(hd)) {
		const m = RENDER.exec(f);
		const st = m && statOf(resolve(hd, f));
		if (st) at(m[1]).rendered.push({ file: f, url: `/out/${ch}/handheld/${encodeURIComponent(f)}`, pass: m[3], take: m[2], ms: durationOf(resolve(hd, f), st) });
	}
	const gd = resolve(boards, '_out', ch, 'generated');
	const generated = {};
	for (const f of list(gd)) {
		const m = GENERATED.exec(f);
		const st = m && statOf(resolve(gd, f));
		if (!st) continue;
		const video = VIDEO.test(f);
		const entry = { file: f, url: `/out/${ch}/generated/${encodeURIComponent(f)}`, ms: video ? durationOf(resolve(gd, f), st) : null, video, approved: false };
		(generated[m[1]] ??= []).push({ entry, file: resolve(gd, f), st, n: Number(/^g(\d+)$/.exec(m[2])?.[1] ?? 0) });
	}
	for (const [id, items] of Object.entries(generated)) {
		items.sort((a, b) => b.n - a.n || b.st.mtimeMs - a.st.mtimeMs || b.entry.file.localeCompare(a.entry.file));
		const approved = approvedOf({ boards, ch, shot: id, files: items.map((g) => g.entry.file) });
		for (const g of items) g.entry.approved = approved.has(g.entry.file);
		at(id).generated = items.map((g) => g.entry);
	}
	const fmt = (f) => (f.endsWith('.mp4') ? 0 : 1);
	for (const s of Object.values(shots)) s.rendered.sort((a, b) => b.take.localeCompare(a.take, 'en', { numeric: true }) || PASSES.indexOf(a.pass) - PASSES.indexOf(b.pass) || fmt(a.file) - fmt(b.file));
	return shots;
}

// --- serving a clip ------------------------------------------------------------------------------------------------
/** a mounted file (api.mjs MOUNTS: /out/ …) with one byte range when asked – without a 206 Chromium's seekable range is
 *  [0, 0] and every currentTime snaps back to 0 (ClipView's seek); a plain GET gets the whole file, as before */
export function sendFile(req, res, file, type) {
	const size = statSync(file).size;
	// a filmstrip sprite's name carries its video's mtime + size (thumbName): the browser keeps it – the sprite ClipStrip
	// decoded before the swap is the one its tiles show (no-store fetched it a second time: a blank between two sprites)
	const keep = SPRITE.test(file);
	const head = { 'content-type': type, 'cache-control': keep ? 'max-age=31536000, immutable' : 'no-store', 'accept-ranges': 'bytes' };
	const m = /^bytes=(\d*)-(\d*)$/.exec(String(req.headers?.range ?? '').trim());
	if (!m || (!m[1] && !m[2]) || !size) {
		const body = readFileSync(file);
		res.writeHead(200, { ...head, 'content-length': body.length });
		return res.end(body);
	}
	const start = m[1] ? Number(m[1]) : Math.max(0, size - Number(m[2]));
	const end = m[1] && m[2] ? Math.min(Number(m[2]), size - 1) : size - 1;
	if (start > end || start >= size) {
		res.writeHead(416, { ...head, 'content-range': `bytes */${size}` });
		return res.end();
	}
	res.writeHead(206, { ...head, 'content-range': `bytes ${start}-${end}/${size}`, 'content-length': end - start + 1 });
	// pipeline, not pipe: a <video> drops most range requests early – the file is closed then too (Windows locks open files)
	pipeline(createReadStream(file, { start, end }), res, () => {});
}

// --- the filmstrip: a video clip's frames one after another (Timeline.svelte → ClipStrip.svelte) ---------------------
// Owner 2026-10-20: "previews in the timeline, should be not cover, but show the frame after frame". `GET /api/thumbs?
// url=/out/<ch>/…/<video>&w=&h=` (the clip's width and height on screen, device px; `ms` = its length when neither the
// file nor ffmpeg says) → { url, n, tw, th }: ONE sprite (a JPEG under /out/) of n tiles tw × th side by side at the
// video's aspect, tile j = the frame showing at j · duration / n. n = the smallest of TILE_STEPS whose tiles cover the
// clip's width (at most 64, the sprite at most MAX_SPRITE px wide); the client lays tiles at that aspect along the clip
// and gives each the sprite's latest tile at or before its time – so a zoom inside one step needs no new sprite.
// ffmpeg (on the PATH or FFMPEG=<path>, as the handheld encode) reads the size (`ffmpeg -i`) and makes the sprite
// (fps → scale → tile); cached in _out/<ch>/thumbs/ by file + mtime + size + n + h (a changed video gets new names, the
// oldest go past THUMB_KEEP), at most two ffmpeg at a time. Only video files under /out/ (the mount's own check).
const TILE_STEPS = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64];
const MAX_SPRITE = 16384; // px – a sprite's width
const THUMB_H = { min: 32, max: 480, step: 16 }; // px – a tile's height, rounded up to the step
const THUMB_KEEP = { over: 300, down: 200 }; // a chapter's thumbs folder: past 300 sprites the oldest go, down to 200
const SPRITE = /[\\/]thumbs[\\/][\w-]+--[0-9a-f]{12}-n\d+-h\d+\.jpg$/; // a sprite as thumbName names it (sendFile: cached)

/** GET /api/thumbs's query → { file, ch, w, h, th, ms } – or throws: `url` a video under /out/ (the mount's own path
 *  check – nothing outside `out`), in a chapter folder; w, h > 0; th = the tiles' height (h rounded up to 16, within
 *  32…480); ms > 0, else null */
export function thumbParams(q, out) {
	const root = resolve(out);
	let file = null;
	try {
		const u = new URL(String(q?.url ?? ''), 'http://x');
		if (u.origin === 'http://x' && String(q.url).startsWith('/out/') && u.pathname.startsWith('/out/')) {
			const rel = decodeURIComponent(u.pathname.slice(5));
			if (!/[\\\0]/.test(rel)) file = resolve(root, rel); // a backslash or a NUL: no file
		}
	} catch {}
	const ch = file?.startsWith(root + sep) ? relative(root, file).split(sep)[0] : '';
	if (!/^[a-z0-9-]+$/.test(ch) || !VIDEO.test(file)) throw new Error('bad url – a video under /out/');
	const w = Number(q.w), h = Number(q.h), ms = Number(q.ms);
	if (!(w > 0 && w <= 1e6) || !(h > 0 && h <= 1e5)) throw new Error('bad size');
	const th = Math.min(THUMB_H.max, Math.max(THUMB_H.min, Math.ceil(h / THUMB_H.step) * THUMB_H.step));
	return { file, ch, w: Math.round(w), h: Math.round(h) || 1, th, ms: ms > 0 && ms < 864e5 ? Math.round(ms) : null };
}

/** the sprite of a width × height video for a clip w × h px: { tw, th, n } – tiles th high at the video's aspect (tw
 *  even); n the smallest step whose tiles AS SHOWN (h high) cover w – every tile on screen its own frame – capped */
export function thumbPlan({ width, height, w, h, th = h }) {
	const tw = Math.max(2, 2 * Math.round((th * width) / height / 2));
	const cap = TILE_STEPS.filter((s) => s * tw <= MAX_SPRITE).at(-1) ?? 1;
	return { tw, th, n: Math.min(cap, TILE_STEPS.find((s) => (s * h * width) / height >= w) ?? cap) };
}

/** a sprite's file name (the cache key): the video's name, a hash of its path + mtime + size, n, h */
export function thumbName({ file, st, n, h }) {
	const hash = createHash('sha1').update(`${file}|${st.mtimeMs}|${st.size}`).digest('hex').slice(0, 12);
	return `${basename(file).replace(/\.[^.]+$/, '').replace(/[^\w-]+/g, '_').slice(0, 60)}--${hash}-n${n}-h${h}.jpg`;
}

let busy = 0;
const waiting = [];
/** at most two ffmpeg at a time (memory): a finished one hands its place to the next */
async function slot(fn) {
	if (busy < 2) busy++;
	else await new Promise((go) => waiting.push(go));
	try { return await fn(); } finally { const next = waiting.shift(); if (next) next(); else busy--; }
}
const ff = (ffmpeg, args) => slot(() => new Promise((ok) => execFile(ffmpeg, args, { timeout: 120000, maxBuffer: 4 << 20, windowsHide: true }, (err, _out, stderr) => ok({ err, out: String(stderr ?? '') }))));
const noFfmpeg = (ffmpeg) => new Error(`${ffmpeg} not found (FFMPEG=<path>)`);

/** the video's size and duration as ffmpeg reads them (`ffmpeg -i` prints them and stops) – cached by mtime + size */
function probe(ffmpeg, file, st) {
	return cached(`probe:${ffmpeg}|${file}`, [st], () => ff(ffmpeg, ['-hide_banner', '-i', file]).then(({ err, out }) => {
		if (err?.code === 'ENOENT') throw noFfmpeg(ffmpeg);
		const size = /Stream #[^\n]*?Video:[^\n]*?\b(\d{2,5})x(\d{2,5})\b/.exec(out);
		const d = /Duration: (\d+):(\d\d):(\d\d(?:\.\d+)?)/.exec(out);
		if (!size) throw new Error('no video stream');
		return { width: +size[1], height: +size[2], ms: d ? Math.round(((+d[1] * 60 + +d[2]) * 60 + +d[3]) * 1000) : null };
	}));
}

const making = new Map();
/** GET /api/thumbs: the sprite (made once) → { url, n, tw, th } */
export async function clipThumbs(q, { out, ffmpeg = 'ffmpeg' }) {
	const p = thumbParams(q, out);
	const st = statOf(p.file);
	if (!st) throw new Error('no such clip');
	const size = await probe(ffmpeg, p.file, st);
	const ms = durationOf(p.file, st) ?? size.ms ?? p.ms;
	if (!(ms > 0)) throw new Error('unknown duration');
	const plan = thumbPlan({ ...size, w: p.w, h: p.h, th: p.th });
	const name = thumbName({ file: p.file, st, n: plan.n, h: plan.th });
	const dir = resolve(out, p.ch, 'thumbs');
	const target = resolve(dir, name);
	if (!statOf(target)) {
		if (!making.has(target)) making.set(target, makeSprite(ffmpeg, p.file, dir, name, plan, ms).finally(() => making.delete(target)));
		await making.get(target);
	}
	return { url: `/out/${p.ch}/thumbs/${name}`, n: plan.n, tw: plan.tw, th: plan.th };
}

/** ffmpeg: n frames at j · ms / n (round=up: the frame showing then; start_time=0: the first tile is the first frame),
 *  scaled to tw × th, side by side – written next to its place, then renamed (a reader never sees half a file). tpad
 *  holds the last frame past the picture's end: a file longer than its picture (a longer sound track – ms is the file's)
 *  fills its last tiles with that frame, as the player shows it, not black */
async function makeSprite(ffmpeg, file, dir, name, { tw, th, n }, ms) {
	mkdirSync(dir, { recursive: true });
	const tmp = resolve(dir, `${name}.${process.pid}.tmp.jpg`);
	const vf = `tpad=stop=-1:stop_mode=clone,fps=fps=${((n * 1000) / ms).toFixed(6)}:start_time=0:round=up,scale=${tw}:${th},tile=${n}x1`;
	const r = await ff(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-threads', '2', '-i', file, '-an', '-vf', vf, '-frames:v', '1', '-q:v', '5', tmp]);
	if (r.err || !statOf(tmp)) {
		rmSync(tmp, { force: true });
		throw r.err?.code === 'ENOENT' ? noFfmpeg(ffmpeg) : new Error(`ffmpeg: ${r.out.trim().split('\n').at(-1) || 'no sprite'}`);
	}
	renameSync(tmp, resolve(dir, name));
	const sprites = list(dir).filter((f) => f.endsWith('.jpg') && !f.includes('.tmp.'));
	if (sprites.length <= THUMB_KEEP.over) return;
	sprites.map((f) => ({ f, t: statOf(resolve(dir, f))?.mtimeMs ?? 0 })).sort((a, b) => a.t - b.t)
		.slice(0, sprites.length - THUMB_KEEP.down).forEach(({ f }) => rmSync(resolve(dir, f), { force: true }));
}
