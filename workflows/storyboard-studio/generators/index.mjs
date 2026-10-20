// The studio's generator routes (api.mjs spreads them in): the registry (comfy-local · runway · luma-manual ·
// comfyui-remote, + fake with STUDIO_FAKE_GENERATOR=1), the inputs a shot offers, the estimate, start / cancel, the jobs
// and the session's spend (in USD) + the euro rate the UI shows them in (rate.mjs). The interface: jobs.mjs. README
// "Generators".
//
// Inputs travel as keys, never as paths: "out:<ch>/clean/<shot>.png" (storyboards/_out), "boards:<…>"
// (storyboards/ – the approved images, the style folder), "media:<…>" (the media root) – resolved and checked here;
// "frame:out:<…video>#first|last|<ms>" = a frame of a video as a still (job-fields.mjs), cut on the first use
// (frames.mjs). "Improve on this" (agents/research/VARIANTS-CANVAS.md §3.6): a card goes in as one of these keys – a
// generated image as the still, a generated video as Aleph's video (`control`), a frame of a video as the still – and
// the record names it (key + role) and its parent. The manual result slot: manual() (generated.mjs's drop with a job).

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createManager } from './jobs.mjs';
import { createRunway } from './runway.mjs';
import { createComfyLocal } from './comfy-local.mjs';
import { createLumaManual } from './luma-manual.mjs';
import { createComfyRemote, createFake } from './others.mjs';
import { createRate } from './rate.mjs';
import { createFrames } from './frames.mjs';
import { FRAME_MARKS, frameKey, inputsOf, parseFrameKey } from './job-fields.mjs';
import { approvedOf } from '../clips.mjs';
import { idOf, kindOf, parentsOf } from '../../../scripts/storyboard/generated-log.mjs';

const PASS_ORDER = ['grey', 'beauty', 'depth', 'normals'];
const IMAGE = /\.(png|jpe?g|webp)$/i;
const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const CHOSEN = /^boards:dataset\/files\/([a-z0-9-]+)\/([\w-]+)\/chosen\.\w+$/;
const nOf = (f) => Number(/--g(\d+)\./.exec(f)?.[1] ?? 0);
const list = (dir) => (existsSync(dir) ? readdirSync(dir) : []);
/** the euro rate's cache – gitignored (workflows/storyboard-studio/.gitignore) */
const RATE_FILE = resolve(dirname(fileURLToPath(import.meta.url)), '../.cache/eur-usd.json');

export function generatorRoutes({ root, out, boards, media, findShot, shotCharacters, styleFiles, env = process.env, rate = createRate({ cacheFile: RATE_FILE, env }), frames = createFrames({ out, ffmpeg: env.FFMPEG || 'ffmpeg' }) }) {
	const roots = { out, boards, media };
	// the approved image (or video) as an input is a copy of a generated file: that file (approve.mjs `from`, clips.mjs
	// approvedOf) – its parent; comfy-local edits it (--from) – a clip made from the chosen image hangs under it (§3.7)
	function originalOf(key) {
		const m = CHOSEN.exec(String(key ?? ''));
		if (!m) return null;
		const files = list(resolve(boards, '_out', m[1], 'generated')).filter((f) => f.startsWith(`${m[2]}--`));
		return [...approvedOf({ boards, ch: m[1], shot: m[2], files })].find((f) => kindOf(f) === kindOf(key)) ?? null;
	}
	const parentOf = (key) => { const f = originalOf(key); return f ? idOf(f) : null; };
	const generators = [createComfyLocal({ root, env, originalOf }), createRunway({ root, env }), createLumaManual(), createComfyRemote()];
	if (env.STUDIO_FAKE_GENERATOR === '1') generators.push(createFake({ read: (f) => readFileSync(f) }));
	const manager = createManager({ generators, out, parentOf });
	// the first fetch starts with the studio (in the background), so the first page load already has today's rate
	rate.get();

	const keyOf = (rootName, file) => `${rootName}:${relative(roots[rootName], file).replaceAll('\\', '/')}`;
	/** a plain key → its file (under its root, a file) – or throws */
	function fileAt(key) {
		const m = /^(out|boards|media):(.+)$/.exec(String(key));
		const base = m && roots[m[1]];
		if (!base) throw new Error(`bad input ${key}`);
		const file = resolve(base, m[2]);
		if (!file.startsWith(base + sep) || !existsSync(file) || statSync(file).isDirectory()) throw new Error(`no input ${key}`);
		return file;
	}
	/** a key → its file; a frame key → the frame, cut now unless cut before (frames.mjs) */
	async function fileOf(key) {
		if (!key) return null;
		if (!String(key).startsWith('frame:')) return fileAt(key);
		const f = parseFrameKey(key);
		if (!f) throw new Error(`bad frame ${key} – frame:out:<video>#first|last|<ms>`);
		const video = fileAt(f.key);
		if (!VIDEO.test(video)) throw new Error(`not a video ${f.key}`);
		return frames.cut(video, f.frame);
	}
	const urlOf = (key) => { const [r, rel] = key.split(/:(.*)/s); return r === 'out' ? `/out/${rel}` : r === 'media' ? `/media/${rel}` : rel.startsWith('style/') ? `/style/${rel.slice(6)}` : null; };
	const item = (rootName, file, label) => { const key = keyOf(rootName, file); return { key, label, url: urlOf(key) }; };

	/** what a shot offers as inputs (the UI pre-fills from it): stills = the approved image, the render, the saved frame,
	 *  the layout variants' frames, the generated images, the generated videos' first / last frames; controls = the
	 *  handheld render passes, the generated videos (Aleph's video) – a generated one carries its record `id`, a frame
	 *  its `frame` (and `url` null until it is cut – POST /api/generate/frame) */
	function inputs({ toon, ch, id }) {
		const hit = findShot(toon, ch, id);
		const shot = hit.group.shots[hit.index];
		const stills = [];
		const chosen = resolve(boards, 'dataset/files', ch, id);
		for (const f of list(chosen).filter((x) => /^chosen\.(png|jpe?g|webp)$/i.test(x))) stills.push(item('boards', resolve(chosen, f), 'The approved image (chosen)'));
		for (const [dir, label] of [['clean', 'The render (no texts)'], ['frames', 'The saved frame']]) {
			const f = resolve(out, ch, dir, `${id}.png`);
			if (existsSync(f)) stills.push(item('out', f, label));
		}
		// a layout variant's saved frame (the board's variant source cards – Q5 A)
		const vd = resolve(out, ch, 'variants');
		for (const f of list(vd).filter((x) => x.startsWith(`${id}--`) && x.endsWith('.png'))) stills.push(item('out', resolve(vd, f), `Layout variant ${f.slice(id.length + 2, -4)} (its frame)`));
		// the shot's generated files, the newest first
		const gd = resolve(out, ch, 'generated');
		const gen = list(gd).filter((f) => f.startsWith(`${id}--`) && (IMAGE.test(f) || VIDEO.test(f))).sort((a, b) => nOf(b) - nOf(a) || b.localeCompare(a));
		const named = (f) => `Generated ${f.slice(id.length + 2)}`;
		for (const f of gen.filter((x) => IMAGE.test(x))) stills.push({ ...item('out', resolve(gd, f), named(f)), id: idOf(f) });
		for (const f of gen.filter((x) => VIDEO.test(x))) for (const frame of FRAME_MARKS) {
			const cut = frames.fresh(resolve(gd, f), frame);
			stills.push({ key: frameKey(keyOf('out', resolve(gd, f)), frame), label: `${named(f)} – the ${frame} frame`, url: cut ? urlOf(keyOf('out', cut)) : null, id: idOf(f), frame });
		}
		const hd = resolve(out, ch, 'handheld');
		const controls = list(hd).map((f) => [f, new RegExp(`^${id}--([\\w-]+?)--(\\w+)\\.mp4$`).exec(f)]).filter(([, m]) => m)
			.sort((a, b) => b[1][1].localeCompare(a[1][1]) || PASS_ORDER.indexOf(a[1][2]) - PASS_ORDER.indexOf(b[1][2]))
			.map(([f, m]) => item('out', resolve(hd, f), `Take ${m[1]} · ${m[2]}`));
		for (const f of gen.filter((x) => VIDEO.test(x))) controls.push({ ...item('out', resolve(gd, f), named(f)), id: idOf(f) });
		const characters = shotCharacters(hit.sb, shot).filter((c) => c.image).map((c) => {
			const inMedia = media && c.image.file.startsWith(media + sep);
			return item(inMedia ? 'media' : 'boards', c.image.file, `${c.fig.cast} (${c.image.mood})`);
		});
		const style = styleFiles().map((f) => item('boards', resolve(boards, 'style', f), f));
		return { stills, controls, characters, style, prompt: shot.prompt?.en ?? '', aspect: shot.aspect ?? '9:16', duration: 5 };
	}

	/** the chapter a file belongs to: its folder under _out (a frame: its video's) or under the approved images
	 *  (dataset/files/<ch>/…) – null for what all chapters share (the style folder, character images, media) */
	const dataset = resolve(boards, 'dataset', 'files');
	function chapterOfFile(file) {
		for (const base of [out, dataset]) if (base && file.startsWith(base + sep)) return relative(base, file).split(sep)[0];
		return null;
	}
	/** the client's job → the generator's input (keys resolved to files; a frame cut) – for a shot of toon/ch that exists,
	 *  from inputs of that chapter (a form left from another chapter or shot never goes out: ids are per chapter) */
	async function jobOf(body) {
		const j = body.job ?? body;
		try { findShot(j.toon, j.ch, j.shot); } catch { throw new Error(`no shot ${j.shot} in ${j.toon}/${j.ch}`); }
		if (String(j.control ?? '').startsWith('frame:')) throw new Error('the control input is a video, not a frame of one');
		// checked before a frame is cut: the video's chapter
		const own = async (key) => {
			if (!key) return null;
			const f = parseFrameKey(key);
			const c = f || !String(key).startsWith('frame:') ? chapterOfFile(fileAt(f ? f.key : String(key))) : null;
			if (c && c !== j.ch) throw new Error(`${key} belongs to ${c}, not ${j.ch}`);
			return fileOf(key);
		};
		return {
			generator: j.generator, model: j.model, toon: j.toon, ch: j.ch, shot: j.shot,
			prompt: j.prompt ?? '', duration: j.duration, aspect: j.aspect, seed: j.seed ?? null,
			still: j.still ?? null, control: j.control ?? null, characters: j.characters ?? [], style: j.style ?? [],
			stillFile: await own(j.still), controlFile: await own(j.control),
			characterFiles: await Promise.all((j.characters ?? []).map(own)), styleFiles: await Promise.all((j.style ?? []).map(own))
		};
	}

	/** the manual result slot (§3.7 "Manual"): a by-hand generator's candidate – its model, the form's prompt, the inputs
	 *  the model takes (each must exist – a frame is cut) and their parents → the record's fields (generated.mjs writes
	 *  the file and the record; an explicit parent goes first there) */
	async function manual(job) {
		const g = generators.find((x) => x.id === job?.generator);
		if (g?.transport !== 'manual') throw new Error(`${job?.generator || 'no generator'} is not done by hand – its results come from its jobs`);
		const m = g.models.find((x) => x.id === job.model) ?? g.models[0];
		const input = await jobOf({ ...job, model: m.id });
		const inputs = inputsOf(m, input);
		return { generator: g.id, model: m.id, mode: m.mode, prompt: String(job.prompt ?? '').slice(0, 20000), inputs, parents: parentsOf(inputs, (k) => { try { return parentOf(k); } catch { return null; } }) };
	}

	return {
		manager,
		manual,
		routes: {
			'GET /api/generators': async () => ({ generators: await manager.list(), spend: manager.spend(), rate: rate.get() }),
			'GET /api/generate/inputs': inputs,
			'POST /api/generate/estimate': async (body) => manager.estimate(await jobOf(body)),
			'POST /api/generate': async (body) => manager.start(await jobOf(body), { confirm: body.confirm }),
			// a frame of a video as a still (More ▾ "Still · First | <time> | Last"): cut now (cached) → its picture
			'POST /api/generate/frame': async ({ key }) => {
				if (!parseFrameKey(key)) throw new Error(`bad frame ${key} – frame:out:<video>#first|last|<ms>`);
				const file = await fileOf(key);
				return { key, url: urlOf(keyOf('out', file)), mtime: Math.round(statSync(file).mtimeMs) };
			},
			'GET /api/generate/jobs': ({ ch, id }) => ({ jobs: manager.jobs({ ch, shot: id }), spend: manager.spend(), rate: rate.get() }),
			'POST /api/generate/cancel': ({ job }) => manager.cancel(job)
		}
	};
}
