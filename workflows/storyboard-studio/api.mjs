// The storyboard studio's local API (owner 2026-10-09: "just start the studio over night. i want to see where this
// goes"): mounted into the studio's Vite dev server (vite.config.js) – `npm run dev:studio` → http://localhost:14000.
//
// The layers of the storyboard workflow side by side – script → scene → shots → render – for one chapter; a shot
// opened renders live in the page (the same renderer as the frames: tools/storyboard-render/core.js); edits, figure
// drags, camera / light changes, variants, reviews and taste rules are written back to the repo's files
// (storyboards/<toon>/<chapter>/…, workflows/scenes/storyboard-taste.md). Listens on 127.0.0.1 only.

import { execFile } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync, appendFileSync } from 'node:fs';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mediaRoot, readEntity } from '../../scripts/storyboard/lib.mjs';
import { loadStoryboards, measureShot, summarize, validate } from '../../scripts/storyboard/storyboard.mjs';
import { lumaPrompt } from '../../scripts/storyboard/luma.js';
import { shotCharacters, styleFiles } from '../../scripts/storyboard/refs.mjs';
import { generatorRoutes } from './generators/index.mjs';
import { lanIps, phone } from './phone-https.mjs';
import { chapterClips, clipThumbs, sendFile } from './clips.mjs';
import { generatedRoutes } from './generated.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const BOARDS = resolve(ROOT, 'storyboards');
const OUT = resolve(BOARDS, '_out');
const threeDir = [resolve(ROOT, 'tools/storyboard-render/node_modules/three'), resolve(ROOT, 'node_modules/three')].find((d) => existsSync(resolve(d, 'build/three.module.js')));
const media = mediaRoot();

const TYPES = { '.mp4': 'video/mp4', '.webm': 'video/webm', '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.png': 'image/png', '.webp': 'image/webp', '.jpg': 'image/jpeg', '.glb': 'model/gltf-binary', '.svg': 'image/svg+xml' };
const MOUNTS = {
	'/render/': resolve(ROOT, 'tools/storyboard-render'),
	'/three/': threeDir,
	'/media/': media,
	'/out/': OUT,
	'/style/': resolve(BOARDS, 'style')
};

// --- the script, per scene (the local dev DB, read-only; cached) -------------------------------------------------
const scriptCache = {};
function scriptScenes(toon) {
	if (scriptCache[toon]) return scriptCache[toon];
	const scenes = {};
	try {
		const text = readEntity(`script:${toon}:de`).text ?? '';
		const lines = text.split('\n');
		let cur = null;
		lines.forEach((line, i) => {
			const m = /^(\d+)\s{1,4}\S/.exec(line);
			if (m) { cur = Number(m[1]); scenes[cur] = { heading: line.trim(), from: i + 1, lines: [] }; }
			if (/^\s+# \d+\. Kapitel/.test(line)) cur = null;
			if (cur != null) scenes[cur].lines.push(line);
		});
	} catch (e) {
		scenes.error = `script not readable (${e.message.split('\n')[0]}) – the dev DB (npm run dev:db) must be running`;
	}
	return (scriptCache[toon] = scenes);
}

// --- storyboard files ----------------------------------------------------------------------------------------------
const chapterDir = (toon, ch) => {
	const d = resolve(BOARDS, toon, ch);
	if (!d.startsWith(BOARDS + sep) || !/^[a-z0-9-]+$/.test(toon) || !/^[a-z0-9-]+$/.test(ch)) throw new Error('bad chapter');
	return d;
};
function chapters() {
	const out = [];
	for (const toon of readdirSync(BOARDS)) {
		const td = resolve(BOARDS, toon);
		if (toon.startsWith('_') || !statSync(td).isDirectory()) continue;
		for (const ch of readdirSync(td)) {
			const cd = resolve(td, ch);
			if (statSync(cd).isDirectory() && readdirSync(cd).some((f) => /^scene-.*\.json$/.test(f))) out.push({ toon, ch });
		}
	}
	return out;
}
function findShot(toon, ch, id) {
	for (const { file, sb } of loadStoryboards(chapterDir(toon, ch))) {
		for (const g of sb.groups ?? []) {
			const i = (g.shots ?? []).findIndex((s) => s.id === id);
			if (i >= 0) return { file, sb, group: g, index: i };
		}
	}
	throw new Error(`no shot ${id}`);
}
const save = (file, sb) => writeFileSync(file, JSON.stringify(sb, null, 2) + '\n');
const variantsFile = (toon, ch, id) => resolve(chapterDir(toon, ch), 'variants', `${id}.json`);
const readVariants = (toon, ch, id) => (existsSync(variantsFile(toon, ch, id)) ? JSON.parse(readFileSync(variantsFile(toon, ch, id), 'utf8')) : []);
function writeVariants(toon, ch, id, list) {
	mkdirSync(dirname(variantsFile(toon, ch, id)), { recursive: true });
	writeFileSync(variantsFile(toon, ch, id), JSON.stringify(list, null, 2) + '\n');
}
function writePng(file, dataUrl) {
	mkdirSync(dirname(file), { recursive: true });
	writeFileSync(file, Buffer.from(dataUrl.replace(/^data:image\/png;base64,/, ''), 'base64'));
}
const run = (script, args) => new Promise((ok) => execFile(process.execPath, [resolve(ROOT, script), ...args], { cwd: ROOT, maxBuffer: 32 * 1024 * 1024, timeout: 15 * 60 * 1000 }, (err, stdout, stderr) => ok({ ok: !err, out: `${stdout}${stderr}`.trim() })));

// --- the handheld camera's takes (README "Handheld camera", research/HANDHELD-CAMERA.md) -------------------------------
// storyboards/<toon>/<chapter>/takes/<shot>.json = { on, active, takes: [{ id, created, smooth, scale, in, out, samples }] }
// – next to the shot like its variants; one sample per line { t, position, quaternion, fov } (the scene's metres, Y up).
const takesFile = (toon, ch, id) => { if (!/^[\w-]+$/.test(id)) throw new Error('bad id'); return resolve(chapterDir(toon, ch), 'takes', `${id}.json`); };
const readTakes = (toon, ch, id) => (existsSync(takesFile(toon, ch, id)) ? JSON.parse(readFileSync(takesFile(toon, ch, id), 'utf8')) : { on: false, active: null, takes: [] });
function writeTakes(toon, ch, id, doc) {
	const file = takesFile(toon, ch, id);
	mkdirSync(dirname(file), { recursive: true });
	const takes = (doc.takes ?? []).map(({ samples = [], ...t }) => [`    ${JSON.stringify({ ...t, samples: 0 }).slice(0, -2)}[`, samples.map((x) => `      ${JSON.stringify(x)}`).join(',\n'), '    ]}'].join('\n'));
	const head = `{\n  "on": ${!!doc.on},\n  "active": ${JSON.stringify(doc.active ?? null)},\n  "takes": [`;
	writeFileSync(file, `${head}${takes.length ? `\n${takes.join(',\n')}\n  ` : ''}]\n}\n`);
}
const handheldDir = (ch, id, take) => { if (![id, take].every((x) => /^[\w-]+$/.test(x)) || !/^[a-z0-9-]+$/.test(ch)) throw new Error('bad id'); return resolve(OUT, ch, 'handheld'); };
const PASSES = ['beauty', 'depth', 'normals', 'grey'];
const ffmpeg = process.env.FFMPEG || 'ffmpeg';
const exec = (cmd, args) => new Promise((ok) => execFile(cmd, args, { maxBuffer: 32 * 1024 * 1024, timeout: 30 * 60 * 1000 }, (err, stdout, stderr) => ok({ ok: !err, out: `${err?.code === 'ENOENT' ? `${cmd} not found (FFMPEG=<path>) ` : ''}${stderr}`.trim().split('\n').slice(-3).join('\n') })));

// --- the generators (README "Generators", generators/jobs.mjs): comfy-local · runway · comfyui-remote --------------------
const generators = generatorRoutes({ root: ROOT, out: OUT, boards: BOARDS, media, findShot, shotCharacters, styleFiles });
const generated = generatedRoutes({ out: OUT, boards: BOARDS, run });

// --- API -----------------------------------------------------------------------------------------------------------
const api = {
	...generators.routes,
	'GET /api/chapters': () => chapters(),
	'GET /api/chapter': ({ toon, ch }) => {
		const list = loadStoryboards(chapterDir(toon, ch));
		const script = scriptScenes(toon);
		const problems = list.flatMap(({ file, sb }) => { const v = validate(sb, file); return [...v.errors.map((m) => ({ level: 'error', m })), ...v.warnings.map((m) => ({ level: 'warning', m }))]; });
		const variants = {};
		const vd = resolve(chapterDir(toon, ch), 'variants');
		if (existsSync(vd)) for (const f of readdirSync(vd)) variants[f.replace(/\.json$/, '')] = JSON.parse(readFileSync(resolve(vd, f), 'utf8')).length;
		// the shots with the handheld camera on: { id: number of takes }
		const handheld = {};
		const hd = resolve(chapterDir(toon, ch), 'takes');
		if (existsSync(hd)) for (const f of readdirSync(hd).filter((x) => x.endsWith('.json'))) { const d = JSON.parse(readFileSync(resolve(hd, f), 'utf8')); if (d.on) handheld[f.replace(/\.json$/, '')] = d.takes?.length ?? 0; }
		// the reading time of each shot (the timeline's widths): { id: seconds }
		const seconds = {};
		for (const { sb } of list) for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) seconds[shot.id] = +measureShot(shot).seconds.toFixed(2);
		return {
			toon, ch, summary: summarize(list), problems, variants, handheld, seconds, scriptError: script.error ?? null,
			scenes: list.map(({ file, sb }) => ({ file: file.slice(ROOT.length + 1).replaceAll('\\', '/'), sb, script: script[sb.source?.scene] ?? null }))
		};
	},
	'PUT /api/shot': ({ toon, ch, shot }) => {
		const hit = findShot(toon, ch, shot.id);
		hit.group.shots[hit.index] = shot;
		save(hit.file, hit.sb);
		const v = validate(hit.sb, hit.file);
		return { saved: true, problems: [...v.errors, ...v.warnings].filter((m) => m.includes(` ${shot.id}:`) || m.includes(` ${shot.id} `)) };
	},
	'PUT /api/scene': ({ toon, ch, scene, patch }) => {
		const hit = loadStoryboards(chapterDir(toon, ch)).find(({ sb }) => sb.source?.scene === scene);
		if (!hit) throw new Error(`no scene ${scene}`);
		Object.assign(hit.sb, patch); // light default, notes, cast
		save(hit.file, hit.sb);
		return { saved: true };
	},
	'POST /api/frame': ({ toon, ch, id, variant, png }) => {
		if (!/^[\w-]+$/.test(id) || (variant && !/^[\w-]+$/.test(variant))) throw new Error('bad id');
		const file = variant ? resolve(OUT, ch, 'variants', `${id}--${variant}.png`) : resolve(OUT, ch, 'frames', `${id}.png`);
		writePng(file, png);
		return { file: file.slice(ROOT.length + 1) };
	},
	// the shot's generated files and their records – dropped files (Luma results, a photo, a drawing, a clip), approve,
	// the marks (generated.mjs: GET / POST /api/generated, /approve, /mark)
	...generated,
	// the prompt for Luma for the shot as it is edited right now (scripts/storyboard/luma.js) + the references to attach
	'POST /api/prompt': ({ toon, ch, shot }) => {
		const hit = findShot(toon, ch, shot.id);
		const scene = scriptScenes(toon)[hit.sb.source?.scene];
		const chars = shotCharacters(hit.sb, shot);
		const refs = [
			{ role: 'Image reference (strong) – the layout (the render without texts)', name: `${shot.id}.png`, url: existsSync(resolve(OUT, ch, 'clean', `${shot.id}.png`)) ? `/out/${ch}/clean/${shot.id}.png` : `/out/${ch}/frames/${shot.id}.png` },
			...styleFiles().map((f) => ({ role: 'Style reference', name: f, url: `/style/${encodeURIComponent(f)}` })),
			...chars.map(({ fig, cast, image }) => image
				? { role: `Character reference – ${fig.cast} (${cast.persona ?? 'regular'}, ${image.mood})`, name: image.rel.split('/').pop(), url: `/media/${image.rel}` }
				: { role: `Character reference – ${fig.cast} (${cast.persona ?? 'regular'})`, name: null, url: null })
		];
		const text = lumaPrompt(hit.sb, shot, {
			script: scene ? scene.lines.join('\n') : '',
			references: { structure: refs[0].name, style: refs.filter((r) => r.role === 'Style reference').map((r) => r.name), characters: refs.filter((r) => r.role.startsWith('Character')).map((r) => r.name ?? '(none yet – the render only)') }
		});
		return { text, refs };
	},
	'GET /api/variants': ({ toon, ch, id }) => readVariants(toon, ch, id),
	'POST /api/variants': ({ toon, ch, id, shot, note }) => {
		const list = readVariants(toon, ch, id);
		const vid = `v${list.reduce((m, v) => Math.max(m, Number(v.id.slice(1)) || 0), 0) + 1}`;
		list.push({ id: vid, created: new Date().toISOString(), note: note ?? '', shot });
		writeVariants(toon, ch, id, list);
		return { id: vid };
	},
	'POST /api/variants/pick': ({ toon, ch, id, vid }) => {
		const list = readVariants(toon, ch, id);
		const v = list.find((x) => x.id === vid);
		if (!v) throw new Error(`no variant ${vid}`);
		const hit = findShot(toon, ch, id);
		// the current shot becomes a variant, the picked one the shot
		list.push({ id: `v${list.reduce((m, x) => Math.max(m, Number(x.id.slice(1)) || 0), 0) + 1}`, created: new Date().toISOString(), note: `before picking ${vid}`, shot: hit.group.shots[hit.index] });
		hit.group.shots[hit.index] = { ...v.shot, id };
		save(hit.file, hit.sb);
		writeVariants(toon, ch, id, list.filter((x) => x.id !== vid));
		return { picked: vid };
	},
	'POST /api/review': async ({ toon, ch, id, status, note }) => {
		const args = [`storyboards/${toon}/${ch}`, id, ...(status === 'change' ? ['--change', note || 'change'] : note ? ['--note', note] : [])];
		return run('scripts/storyboard/approve.mjs', args);
	},
	'POST /api/taste': ({ rule, quote }) => {
		const file = resolve(ROOT, 'workflows/scenes/storyboard-taste.md');
		let text = readFileSync(file, 'utf8');
		if (!text.includes('\n## From the studio\n')) text += '\n## From the studio\n';
		text += `- ${rule.trim()}${quote ? ` "${quote.trim()}"` : ''} (${new Date().toISOString().slice(0, 10)})\n`;
		writeFileSync(file, text);
		return { saved: true };
	},
	'GET /api/takes': ({ toon, ch, id }) => readTakes(toon, ch, id),
	'PUT /api/takes': ({ toon, ch, id, doc }) => { writeTakes(toon, ch, id, doc); return { saved: true }; },
	// every shot of the chapter with the handheld camera on and an active take (the render, the zip)
	'GET /api/takes/all': ({ toon, ch }) => {
		const d = resolve(chapterDir(toon, ch), 'takes');
		const out = {};
		if (existsSync(d)) for (const f of readdirSync(d).filter((x) => x.endsWith('.json'))) {
			const doc = JSON.parse(readFileSync(resolve(d, f), 'utf8'));
			const take = doc.on && doc.takes?.find((t) => t.id === doc.active);
			if (take) out[f.replace(/\.json$/, '')] = take;
		}
		return out;
	},
	// each shot's clips – the active take, the handheld renders, the generated files, with their real durations (clips.mjs)
	'GET /api/clips': ({ toon, ch }) => chapterClips({ boards: BOARDS, toon, ch }),
	// a video clip's frames one after another for the timeline (w / h = the clip on screen, px): one sprite (clips.mjs, ffmpeg)
	'GET /api/thumbs': (q) => clipThumbs(q, { out: OUT, ffmpeg }),
	// the phone's address(es): this PC on the LAN over HTTPS (phone-https.mjs), or why not
	'GET /api/handheld/urls': () => ({ wifi: phone.on, port: phone.port, error: phone.error, ips: lanIps() }),
	// one frame of a pass → storyboards/_out/<chapter>/handheld/<shot>--<take>/<pass>/f00001.png
	'POST /api/handheld/frame': ({ ch, id, take, pass, n, png }) => {
		if (!PASSES.includes(pass) || !Number.isInteger(n)) throw new Error('bad frame');
		if (n === 0) rmSync(resolve(handheldDir(ch, id, take), `${id}--${take}`, pass), { recursive: true, force: true });
		writePng(resolve(handheldDir(ch, id, take), `${id}--${take}`, pass, `f${String(n + 1).padStart(5, '0')}.png`), png);
		return { ok: true };
	},
	// the frames of a pass → <shot>--<take>--<pass>.mp4 / .webm next to them (ffmpeg); the frames are removed when it worked
	'POST /api/handheld/encode': async ({ ch, id, take, pass, fps = 24, formats = ['mp4'] }) => {
		if (!PASSES.includes(pass)) throw new Error('bad pass');
		const base = handheldDir(ch, id, take);
		const frames = resolve(base, `${id}--${take}`, pass);
		const files = [];
		for (const f of formats) {
			const file = resolve(base, `${id}--${take}--${pass}.${f}`);
			const codec = f === 'webm' ? ['-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '32', '-row-mt', '1', '-deadline', 'good', '-cpu-used', '4'] : ['-c:v', 'libx264', '-crf', '18', '-preset', 'medium', '-movflags', '+faststart'];
			const r = await exec(ffmpeg, ['-y', '-loglevel', 'error', '-framerate', String(Number(fps) || 24), '-i', resolve(frames, 'f%05d.png'), '-vf', 'scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p', ...codec, file]);
			if (!r.ok) return { ok: false, out: r.out, frames: frames.slice(ROOT.length + 1) };
			files.push(file.slice(ROOT.length + 1).replaceAll('\\', '/'));
		}
		rmSync(frames, { recursive: true, force: true });
		try { if (!readdirSync(dirname(frames)).length) rmSync(dirname(frames), { recursive: true }); } catch {}
		return { ok: true, files };
	},
	'POST /api/run': ({ toon, ch, cmd, only }) => {
		const dir = `storyboards/${toon}/${ch}`;
		const scripts = {
			validate: ['scripts/storyboard/validate.mjs', [dir]],
			page: ['scripts/storyboard/render.mjs', [dir]],
			refpacks: ['scripts/storyboard/refpack.mjs', [dir, ...(only ? ['--only', only] : [])]],
			prompts: ['scripts/storyboard/prompts.mjs', [dir]],
			compile: ['scripts/storyboard/compile.mjs', [dir]],
			shotlist: ['scripts/storyboard/shotlist.mjs', [dir]],
			frames: ['tools/storyboard-render/render-shots.mjs', [dir, ...(only ? ['--only', only] : [])]]
		};
		if (!scripts[cmd]) throw new Error(`unknown ${cmd}`);
		return run(...scripts[cmd]);
	}
};

// --- the handler (connect-style middleware: unknown paths go on to Vite) -----------------------------------------------
export async function handle(req, res, next) {
	const url = new URL(req.url, 'http://x');
	const send = (code, body, type = 'application/json') => { res.writeHead(code, { 'content-type': type, 'cache-control': 'no-store' }); res.end(typeof body === 'string' || Buffer.isBuffer(body) ? body : JSON.stringify(body)); };
	try {
		// other machines only read – every write stays on this PC (the Wi-Fi phone comes through phone-https.mjs, which lets only
		// GET / HEAD and the socket through: behind it every request looks local)
		const local = /^(::1|127\.|::ffff:127\.)/.test(req.socket?.remoteAddress ?? '127.');
		if (!local && req.method !== 'GET') return send(403, { error: 'writes only from this PC' });
		const route = api[`${req.method} ${url.pathname}`];
		if (route) {
			let body = {};
			if (req.method !== 'GET') {
				const chunks = [];
				for await (const c of req) chunks.push(c);
				body = chunks.length ? JSON.parse(Buffer.concat(chunks).toString('utf8')) : {};
			}
			return send(200, await route({ ...Object.fromEntries(url.searchParams), ...body }));
		}
		const mount = req.method === 'GET' && Object.keys(MOUNTS).find((m) => url.pathname.startsWith(m));
		if (!mount) return next();
		const base = MOUNTS[mount];
		if (!base) return send(404, 'not found', 'text/plain');
		const file = normalize(join(base, decodeURIComponent(url.pathname.slice(mount.length))));
		if (!file.startsWith(base)) return send(403, 'no', 'text/plain');
		if (!existsSync(file) || statSync(file).isDirectory()) return send(404, 'not found', 'text/plain');
		// byte ranges: a <video> (ClipView) seeks only in a file it got as 206 (clips.mjs)
		return sendFile(req, res, file, TYPES[extname(file)] ?? 'application/octet-stream');
	} catch (e) {
		return send(500, { error: e.message });
	}
}
