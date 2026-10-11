// The map's terrain as a tile pyramid (BACKLOG 10.63, owner 2026-10-11: "Kacheln und Culling – zumindest so anlegen, dass
// wir das schon mal machen können"): a layer's SVG rendered by headless Chrome, tile by tile from its own viewBox (sharp
// at every level – no cut from one giant bitmap), as WebP into the dev media copy:
//   media/<out>/<z>/<x>_<y>.webp   (256 px; level z = 2^z px per world unit; levels 1 … --levels, level 0 = the SVG)
// Tiles with nothing on them (the sea is the page's colour) are not written – the reader hides a missing tile.
// Then: upload the folder to the CDN, and set the layer's `tiles: { path: <out>, levels }` in the map builder / a draft.
//   node --experimental-transform-types scripts/map-tiles.mjs --svg catpope/map/map-layer-terrain.svg [--out catpope/map/tiles/terrain] [--levels 5] [--from 1]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TILE_PX } from '../shared/content/src/map.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const svgPath = arg('svg', 'catpope/map/map-layer-terrain.svg');
const out = arg('out', svgPath.replace(/[^/]+$/, 'tiles/') + svgPath.replace(/^.*\/|\.svg$/g, '').replace(/^map-layer-/, ''));
const levels = Number(arg('levels', '5'));
const from = Number(arg('from', '1'));
const svg = readFileSync(join(root, 'media', svgPath), 'utf8');
const head = svg.match(/^<svg\b[^>]*>/);
if (!head) throw new Error('no <svg> root');
const world = { width: Number(head[0].match(/width="([\d.]+)"/)?.[1]), height: Number(head[0].match(/height="([\d.]+)"/)?.[1]) };
if (!world.width || !world.height) throw new Error('the <svg> root needs width and height (world units)');

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('Chrome not found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'map-tiles-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', 'about:blank'], { stdio: 'ignore' });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
ws.addEventListener('message', (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) (pending.get(m.id)(m), pending.delete(m.id)); });
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
	return r.result?.result?.value;
};

try {
	// the page keeps the SVG's body once; a tile = the same body under a root with that tile's viewBox
	await evaluate(`(() => {
		window.__body = ${JSON.stringify(svg.slice(head[0].length))};
		window.__root = ${JSON.stringify(head[0])};
		window.__tile = async (vx, vy, vs, px) => {
			const root = window.__root.replace(/\\bwidth="[^"]*"/, 'width="' + px + '"').replace(/\\bheight="[^"]*"/, 'height="' + px + '"').replace(/\\bviewBox="[^"]*"/, 'viewBox="' + vx + ' ' + vy + ' ' + vs + ' ' + vs + '"');
			const url = URL.createObjectURL(new Blob([root, window.__body], { type: 'image/svg+xml' }));
			try {
				const img = new Image();
				img.src = url;
				await img.decode();
				const c = new OffscreenCanvas(px, px);
				const g = c.getContext('2d');
				g.drawImage(img, 0, 0, px, px);
				const a = g.getImageData(0, 0, px, px).data;
				let ink = false;
				for (let i = 3; i < a.length; i += 4) if (a[i]) { ink = true; break; }
				if (!ink) return null;
				const b = new Uint8Array(await (await c.convertToBlob({ type: 'image/webp', quality: 0.85 })).arrayBuffer());
				let s = '';
				for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode(...b.subarray(i, i + 0x8000));
				return btoa(s);
			} finally {
				URL.revokeObjectURL(url);
			}
		};
		return true;
	})()`);
	for (let z = from; z <= levels; z++) {
		const size = TILE_PX / 2 ** z;
		const cols = Math.ceil(world.width / size), rows = Math.ceil(world.height / size);
		let written = 0;
		const t0 = Date.now();
		for (let y = 0; y < rows; y++) {
			// one row per round trip
			const row = await evaluate(`Promise.all(Array.from({ length: ${cols} }, (_, x) => window.__tile(x * ${size}, ${y * size}, ${size}, ${TILE_PX})))`);
			row.forEach((b64, x) => {
				if (!b64) return;
				const file = join(root, 'media', out, String(z), `${x}_${y}.webp`);
				mkdirSync(dirname(file), { recursive: true });
				writeFileSync(file, Buffer.from(b64, 'base64'));
				written++;
			});
		}
		console.log(`level ${z}: ${cols} × ${rows} tiles, ${written} written (${Math.round((Date.now() - t0) / 1000)} s)`);
	}
	console.log(`media/${out} – layer: tiles: { path: "${out}", levels: ${levels} }`);
} finally {
	chrome.kill();
	await sleep(500);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
