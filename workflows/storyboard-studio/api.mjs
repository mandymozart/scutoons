// The storyboard studio's local API (owner 2026-10-09: "just start the studio over night. i want to see where this
// goes"): mounted into the studio's Vite dev server (vite.config.js) – `npm run dev:studio` → http://localhost:14000.
//
// The layers of the storyboard workflow side by side – script → scene → shots → render – for one chapter; a shot
// opened renders live in the page (the same renderer as the frames: tools/storyboard-render/core.js); edits, figure
// drags, camera / light changes, variants, reviews and taste rules are written back to the repo's files
// (storyboards/<toon>/<chapter>/…, workflows/scenes/storyboard-taste.md). Listens on 127.0.0.1 only.

import { execFile } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync, appendFileSync } from 'node:fs';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mediaRoot, readEntity } from '../../scripts/storyboard/lib.mjs';
import { loadStoryboards, summarize, validate } from '../../scripts/storyboard/storyboard.mjs';
import { lumaPrompt } from '../../scripts/storyboard/luma.js';
import { shotCharacters, styleFiles } from '../../scripts/storyboard/refs.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const BOARDS = resolve(ROOT, 'storyboards');
const OUT = resolve(BOARDS, '_out');
const threeDir = [resolve(ROOT, 'tools/storyboard-render/node_modules/three'), resolve(ROOT, 'node_modules/three')].find((d) => existsSync(resolve(d, 'build/three.module.js')));
const media = mediaRoot();

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.png': 'image/png', '.webp': 'image/webp', '.jpg': 'image/jpeg', '.glb': 'model/gltf-binary', '.svg': 'image/svg+xml' };
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

// --- API -----------------------------------------------------------------------------------------------------------
const api = {
	'GET /api/chapters': () => chapters(),
	'GET /api/chapter': ({ toon, ch }) => {
		const list = loadStoryboards(chapterDir(toon, ch));
		const script = scriptScenes(toon);
		const problems = list.flatMap(({ file, sb }) => { const v = validate(sb, file); return [...v.errors.map((m) => ({ level: 'error', m })), ...v.warnings.map((m) => ({ level: 'warning', m }))]; });
		const variants = {};
		const vd = resolve(chapterDir(toon, ch), 'variants');
		if (existsSync(vd)) for (const f of readdirSync(vd)) variants[f.replace(/\.json$/, '')] = JSON.parse(readFileSync(resolve(vd, f), 'utf8')).length;
		return {
			toon, ch, summary: summarize(list), problems, variants, scriptError: script.error ?? null,
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
	// generated images (Luma results, a photo, a drawing) dropped onto a shot: kept next to its frame
	'GET /api/generated': ({ ch, id }) => {
		const d = resolve(OUT, ch, 'generated');
		return existsSync(d) ? readdirSync(d).filter((f) => f.startsWith(`${id}--`)).sort() : [];
	},
	'POST /api/generated': ({ ch, id, name, data }) => {
		if (!/^[\w-]+$/.test(id)) throw new Error('bad id');
		const m = /^data:image\/(png|jpeg|webp);base64,/.exec(data ?? '');
		if (!m) throw new Error('png / jpeg / webp only');
		const ext = m[1] === 'jpeg' ? 'jpg' : m[1];
		const d = resolve(OUT, ch, 'generated');
		mkdirSync(d, { recursive: true });
		const n = readdirSync(d).filter((f) => f.startsWith(`${id}--`)).length + 1;
		const file = `${id}--g${n}.${ext}`;
		writeFileSync(resolve(d, file), Buffer.from(data.slice(m[0].length), 'base64'));
		return { file, from: name ?? '' };
	},
	'POST /api/generated/approve': ({ toon, ch, id, file, note }) => {
		if (!/^[\w.-]+$/.test(file)) throw new Error('bad file');
		return run('scripts/storyboard/approve.mjs', [`storyboards/${toon}/${ch}`, id, '--image', resolve(OUT, ch, 'generated', file), ...(note ? ['--note', note] : [])]);
	},
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
		return send(200, readFileSync(file), TYPES[extname(file)] ?? 'application/octet-stream');
	} catch (e) {
		return send(500, { error: e.message });
	}
}
