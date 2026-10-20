// Frame → still (agents/research/VARIANTS-CANVAS.md §3.6, §3.7): the first, the last or the frame at <ms> of a video
// under storyboards/_out – a generated clip, a handheld render – as an image input, named by the key
// "frame:<video key>#first|last|<ms>" (job-fields.mjs). generators/index.mjs resolves such a key here: the frame is cut
// once by ffmpeg to _out/<ch>/generated/frames/<video name>--first|last|<ms>ms--<hash>.png – the hash of the video's
// mtime + size (as the filmstrip's sprites, clips.mjs thumbName): a video replaced under its name gets a new frame, also
// one copied with an older timestamp; the frame before it goes. ffmpeg as for the filmstrip (clips.mjs runFfmpeg: at
// most two at a time; on the PATH or FFMPEG=<path>); written next to its place and renamed, so a reader never sees half
// a file – and thrown away when the video changed while it was cut; the same frame asked twice at once is cut once. A
// time inside the last frame (after its start, before the end – a seek finds nothing there) is the last frame. A last
// frame → image to video makes a NEW clip from where the old one ends (§3.6). test: generators/test/frames.test.mjs.

import { createHash } from 'node:crypto';
import { mkdirSync, readdirSync, renameSync, rmSync, statSync } from 'node:fs';
import { basename, relative, resolve, sep } from 'node:path';
import { clipMs, runFfmpeg } from '../clips.mjs';
import { FRAME_MARKS } from './job-fields.mjs';

const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const CH = /^[a-z0-9-]+$/;
const statOf = (file) => { try { const s = statSync(file); return s.isFile() ? s : null; } catch { return null; } };
const isFrame = (f) => FRAME_MARKS.includes(f) || (Number.isInteger(f) && f >= 0 && f < 1e9);

/** the video's identity in a frame's name: a hash of its mtime + size */
const tagOf = (st) => createHash('sha1').update(`${st.mtimeMs}|${st.size}`).digest('hex').slice(0, 10);
/** a frame's name without the video's tag: "<video name without its extension>--first" · "--last" · "--1250ms" */
const stemOf = (video, frame) => `${basename(video).replace(/\.[^.]+$/, '')}--${typeof frame === 'number' ? `${frame}ms` : frame}`;
/** a frame's file name: "<stem>--<hash of the video's mtime + size>.png" (st = the video's stat) */
export const frameName = (video, frame, st) => `${stemOf(video, frame)}--${tagOf(st)}.png`;
const same = (a, b) => !!a && !!b && a.mtimeMs === b.mtimeMs && a.size === b.size;

/** ffmpeg's arguments for one frame of `video` into `target` (a .png): the first (decoded from the start), the last
 *  (the final second decoded, each frame written over the one before – `-update`), or the frame showing at ms (seeked) */
export function frameArgs(video, frame, target) {
	const head = ['-hide_banner', '-loglevel', 'error', '-y'];
	if (frame === 'first') return [...head, '-i', video, '-an', '-frames:v', '1', target];
	if (frame === 'last') return [...head, '-sseof', '-1', '-i', video, '-an', '-update', '1', target];
	return [...head, '-ss', (frame / 1000).toFixed(3), '-i', video, '-an', '-frames:v', '1', target];
}

/** out = storyboards/_out; run(args) → Promise<{ err, out }> (ffmpeg – a stub in the tests) */
export function createFrames({ out, ffmpeg = 'ffmpeg', run = (args) => runFfmpeg(ffmpeg, args), durationOf = clipMs } = {}) {
	const root = resolve(out);
	const making = new Map();

	/** the video's chapter (its first folder under _out) – or throws: a video file under out, in a chapter folder */
	function chapterOf(video) {
		const file = resolve(video);
		const ch = file.startsWith(root + sep) ? relative(root, file).split(sep)[0] : '';
		if (!CH.test(ch) || !VIDEO.test(file)) throw new Error(`not a video under _out: ${basename(file)}`);
		return ch;
	}
	/** where a frame of `video` (as it is now – st) is cut to */
	const target = (video, frame, st = statOf(video)) => resolve(root, chapterOf(video), 'generated', 'frames', frameName(video, frame, st));
	/** the frame's file when it is cut from the video as it is now, else null (nothing is cut) */
	function fresh(video, frame) {
		if (!isFrame(frame)) return null;
		const v = statOf(video);
		const file = v && target(video, frame, v);
		return file && statOf(file) ? file : null;
	}
	/** the frames cut from the video before it changed go (same stem, another tag) */
	function prune(video, frame, keep) {
		const dir = resolve(keep, '..'), stem = `${stemOf(video, frame)}--`;
		try { for (const f of readdirSync(dir)) if (f.startsWith(stem) && /^[0-9a-f]{10}\.png$/.test(f.slice(stem.length)) && resolve(dir, f) !== keep) rmSync(resolve(dir, f), { force: true }); } catch {}
	}

	/** one ffmpeg run into tmp – a time inside the last frame (a seek there finds nothing) is the last frame */
	async function run1(video, frame, tmp, ms) {
		const r = await run(frameArgs(video, frame, tmp));
		if (!r?.err && statOf(tmp)) return r;
		if (typeof frame === 'number' && ms != null && frame <= ms && r?.err?.code !== 'ENOENT') return run(frameArgs(video, 'last', tmp));
		return r;
	}

	/** the frame's file – cut now unless it is cut from the video as it is now */
	async function cut(video, frame, again = true) {
		if (!isFrame(frame)) throw new Error(`bad frame ${frame} – first | last | ms`);
		chapterOf(video);
		const st = statOf(video);
		if (!st) throw new Error(`no video ${basename(video)}`);
		const file = target(video, frame, st);
		if (statOf(file)) return file;
		const ms = typeof frame === 'number' ? durationOf(video) : null;
		if (ms != null && frame > ms) throw new Error(`no frame at ${(frame / 1000).toFixed(2)} s – ${basename(video)} is ${(ms / 1000).toFixed(2)} s long`);
		if (!making.has(file)) making.set(file, (async () => {
			mkdirSync(resolve(file, '..'), { recursive: true });
			const tmp = `${file.slice(0, -4)}.${process.pid}.tmp.png`;
			const r = await run1(video, frame, tmp, ms);
			if (r?.err || !statOf(tmp)) {
				rmSync(tmp, { force: true });
				throw r?.err?.code === 'ENOENT' ? new Error(`${ffmpeg} not found (FFMPEG=<path>)`) : new Error(`no ${typeof frame === 'number' ? `frame at ${frame} ms` : `${frame} frame`} of ${basename(video)}${r?.out?.trim() ? ` – ffmpeg: ${r.out.trim().split('\n').at(-1)}` : ''}`);
			}
			// the video changed while it was cut: not its frame
			if (!same(st, statOf(video))) {
				rmSync(tmp, { force: true });
				return null;
			}
			renameSync(tmp, file);
			prune(video, frame, file);
			return file;
		})().finally(() => making.delete(file)));
		const done = await making.get(file);
		if (done) return done;
		if (!again) throw new Error(`${basename(video)} changed while its frame was cut – try again`);
		return cut(video, frame, false);
	}

	return { cut, fresh, target };
}
