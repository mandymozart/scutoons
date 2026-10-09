// SCUTOONS sketch → 3D – a local tool, no service, no ML model: a drawing or a cut-out (the characters' A-poses) becomes
// a closed low-poly mesh; a character in A-pose also gets a skeleton like the storyboard figurines' (figures.js), so the
// storyboard's pose table moves it. Everything runs in the page (three.js); this server only serves files and writes the
// GLBs. Listens on 127.0.0.1 only.
//
//   node tools/sketch3d/server.mjs                → http://localhost:14001 (shared/dev-ports.json "sketch3d")
//   node tools/sketch3d/server.mjs --batch [names] → every character's A-pose (or the named ones) → out/<char>.glb,
//                                                   in headless Edge / Chrome, then exits

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from '../../scripts/dev/ports.mjs';
import { mediaRoot } from '../../scripts/storyboard/lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const OUT = resolve(HERE, 'out');
const args = process.argv.slice(2);
const flag = (f) => args.includes(f);
const port = Number(args[args.indexOf('--port') + 1]) || PORTS.sketch3d;
// three: this checkout's, or the main checkout's (a worktree without its own npm install)
const threeDir = [process.env.SCUTOONS_THREE, resolve(ROOT, 'node_modules/three'), resolve(ROOT, '../scutoons/node_modules/three')]
	.filter(Boolean).find((d) => existsSync(resolve(d, 'build/three.module.js')));
if (!threeDir) throw new Error('three is not installed – npm install');
const media = mediaRoot();

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.png': 'image/png', '.webp': 'image/webp', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.glb': 'model/gltf-binary', '.svg': 'image/svg+xml' };
const MOUNTS = {
	'/three/': threeDir,
	'/render/': resolve(ROOT, 'tools/storyboard-render'),
	'/core/': resolve(HERE, 'core'),
	'/media/': media,
	'/out/': OUT,
	'/': resolve(HERE, 'public')
};

/** every character's A-pose image: media/characters/<char>/<char>_apose-swim.(png|webp) first, else any *apose* image */
function characters() {
	const dir = media && resolve(media, 'characters');
	if (!dir || !existsSync(dir)) return [];
	const list = [];
	for (const c of readdirSync(dir)) {
		const d = resolve(dir, c);
		if (!statSync(d).isDirectory()) continue;
		const files = readdirSync(d).filter((f) => /a-?pose/i.test(f) && /\.(png|webp|jpe?g)$/i.test(f));
		if (!files.length) continue;
		const rank = (f) => (/_apose-swim\.png$/i.test(f) ? 0 : /_apose-swim\./i.test(f) ? 1 : /apose/i.test(f) ? 2 : 3);
		files.sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
		list.push({ name: c, file: files[0], url: `/media/characters/${c}/${files[0]}`, all: files.map((f) => `/media/characters/${c}/${f}`) });
	}
	return list;
}

function models() {
	if (!existsSync(OUT)) return [];
	return readdirSync(OUT).filter((f) => f.endsWith('.glb')).map((f) => ({ name: f.replace(/\.glb$/, ''), url: `/out/${f}`, bytes: statSync(resolve(OUT, f)).size }));
}

const body = (req) => new Promise((ok, fail) => { const parts = []; req.on('data', (c) => parts.push(c)); req.on('end', () => ok(Buffer.concat(parts))); req.on('error', fail); });
const json = (res, data, status = 200) => { res.writeHead(status, { 'content-type': 'application/json' }); res.end(JSON.stringify(data)); };
const safeName = (s) => String(s ?? '').toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60);

let batchDone = null;
const server = createServer(async (req, res) => {
	try {
		const url = new URL(req.url, 'http://x');
		const p = decodeURIComponent(url.pathname);
		if (p === '/api/characters') return json(res, characters());
		if (p === '/api/models') return json(res, models());
		if (p === '/api/save' && req.method === 'POST') {
			const name = safeName(url.searchParams.get('name'));
			if (!name) return json(res, { error: 'name' }, 400);
			const kind = url.searchParams.get('kind') === 'json' ? 'json' : 'glb';
			mkdirSync(OUT, { recursive: true });
			const file = resolve(OUT, `${name}.${kind}`);
			writeFileSync(file, await body(req));
			console.log(`  wrote ${file}`);
			return json(res, { file: file.slice(ROOT.length + 1).replaceAll(sep, '/'), url: `/out/${name}.${kind}` });
		}
		if (p === '/api/log' && req.method === 'POST') { console.log('  ' + (await body(req)).toString()); return json(res, {}); }
		if (p === '/api/done' && req.method === 'POST') { json(res, {}); batchDone?.(); return; }
		const mount = Object.keys(MOUNTS).find((m) => p.startsWith(m));
		const base = mount && MOUNTS[mount];
		if (!base) { res.writeHead(404); return res.end(); }
		const file = normalize(join(base, p.slice(mount.length) || 'index.html'));
		if (!file.startsWith(base) || !existsSync(file) || statSync(file).isDirectory()) { res.writeHead(404); return res.end('not found'); }
		res.writeHead(200, { 'content-type': TYPES[extname(file).toLowerCase()] ?? 'application/octet-stream', 'cache-control': 'no-store' });
		res.end(readFileSync(file));
	} catch (e) {
		json(res, { error: e.message }, 500);
	}
});

server.listen(port, '127.0.0.1', () => {
	console.log(`sketch → 3D: http://localhost:${port}  (three ${threeDir}, media ${media ?? '–'})`);
	if (!flag('--batch')) return;
	// batch: the page builds + rigs every A-pose and posts the GLBs here
	const names = args.filter((a) => !a.startsWith('--') && a !== String(port));
	const browser = [process.env.CHROME, 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium', '/usr/bin/google-chrome'].filter(Boolean).find((b) => existsSync(b));
	if (!browser) throw new Error('no Edge / Chrome found (CHROME=<path>)');
	const profile = resolve(OUT, '.headless-profile');
	const child = spawn(browser, ['--headless=new', '--disable-gpu', `--user-data-dir=${profile}`, `http://127.0.0.1:${port}/?batch=${names.join(',') || 'all'}`], { stdio: 'ignore' });
	const stop = (code) => { child.kill(); server.close(); process.exit(code); };
	batchDone = () => stop(0);
	setTimeout(() => { console.error('batch: timeout'); stop(1); }, 10 * 60 * 1000);
});
