// The map painted (owner 2026-10-11: "die Karte generieren" – a painted landscape in the dioramas' style): the line map
// (the terrain SVG) is the LAYOUT, Luma paints it region by region as an image edit, the regions become one picture
// with the SVG's land as its mask (the sea stays the toon's colour) and a tile pyramid for deep zoom (BACKLOG 10.63).
//
//   1. regions: squares of REGION world units, every STEP units – neighbours overlap by REGION − STEP; only regions with
//      land. Rendered at PX px (PX / REGION px per world unit) by headless Chrome from the SVG, the sea flat blue.
//   2. paint (Luma, image_edit): the layout + the strips the neighbours above / left already painted (so the seams
//      continue) → `logs/map-paint/<name>/r_<x>_<y>.png`. A region already painted is skipped (delete it to redo).
//   3. assemble: the regions blended over their overlaps, masked by the SVG's land → the base image
//      `media/<out>.webp` (world size, 1 px per unit) + tiles `media/<tiles>/<z>/<x>_<y>.webp` (levels 1 … --levels).
//   Then add the layer (under the line layer, or instead of it) with `tiles: { path, levels }` in a map draft.
//
// Costs money: one Luma image per land region (uni-1-max ≈ $0.10) – --dry prints the count and the prompt and renders the
// layouts only; --only x_y paints one region (a test); --assemble skips painting. Key: LUMA_AGENT(S)_API_KEY in .env.
//   node --experimental-transform-types scripts/map-paint.mjs [--dry] [--only 6_1] [--assemble] [--model uni-1-max] [--levels 3]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TILE_PX } from '../shared/content/src/map.ts';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
try { process.loadEnvFile(resolve(ROOT, '.env')); } catch {}
process.env.LUMA_AGENTS_API_KEY ??= process.env.LUMA_AGENT_API_KEY;
const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const has = (name) => process.argv.includes(`--${name}`);
const svgPath = arg('svg', 'catpope/map/map-layer-terrain.svg');
const name = arg('name', 'painted');
const out = arg('out', `catpope/map/painted/terrain-${name}`); // the base image (without .webp)
const tilesOut = arg('tiles', `catpope/map/tiles/${name}`);
const levels = Number(arg('levels', '3'));
const model = arg('model', 'uni-1-max');
const style = arg('style', 'catpope/map/places/cathedral.2x.webp'); // a finished diorama: the look (materials, light)
const only = arg('only', null);
const dry = has('dry'), assembleOnly = has('assemble');
const REGION = 256, STEP = 192, PX = 2048; // 8 px per world unit; neighbours share 64 units
const PPU = PX / REGION;
const SEA = '#3d8fd4';
const work = join(ROOT, 'logs', 'map-paint', name);
mkdirSync(work, { recursive: true });

const svg = readFileSync(join(ROOT, 'media', svgPath), 'utf8');
const head = svg.match(/^<svg\b[^>]*>/);
const world = { width: Number(head[0].match(/width="([\d.]+)"/)[1]), height: Number(head[0].match(/height="([\d.]+)"/)[1]) };

export const PROMPT = [
	"THIS JOB'S DELIVERABLE: The SAME map region as the source image, PAINTED as a miniature landscape seen from STRAIGHT ABOVE (a top-down map, orthographic – not isometric, no horizon). The source is the LAYOUT: every coastline, island outline and line stays EXACTLY where it is.",
	'INPUT ASSETS: the source = the layout (white land with dark outlines, flat blue sea). image 1 = a finished diorama of the same world – the STYLE reference only (materials, colours, light, the miniature model look); do NOT draw its building.',
	'EXPECTED OUTPUT: 1 image, same size and framing as the source. Land painted as a hand-built model landscape: meadows, fields, forests (dense small trees), rocky hills where outlines crowd together, sandy or rocky shores along the coast; the dense networks of small cells are a CITY seen from above – streets along the lines, tiny roofs in the cells, parks. The sea stays a flat plain blue (it is replaced later). Strips along the edges that are already painted: keep them unchanged and continue them seamlessly.',
	'TECHNICAL REQUIREMENTS: keep the geometry of the layout; the highest resolution; no borders, no frame, no vignette.',
	'LIGHT: soft daylight from the upper left, gentle model shadows, slightly warm – the same as the style reference.',
	'LOOK: a miniature tilt-shift model world, realistic materials, slightly simplified so it reads small, rich but natural colours. No text, letters, numbers, labels, map symbols, compass, grid, people or vehicles.',
	'CREATIVE DIRECTION: a collectible little world built on a table – inviting, readable from far away, full of small details close up.',
	'CONTEXT: SCUTOONS world map (the islands of the SCUTOONS universe), one region of a tiled map; the painted regions are put together edge to edge.'
].join('\n');

// ── Chrome ─────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('Chrome not found (set CHROME)');
const port = 9950 + Math.floor(Math.random() * 40);
const profile = mkdtempSync(join(tmpdir(), 'map-paint-'));
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
const png64 = (file) => readFileSync(file).toString('base64');

// the page: the SVG body once; helpers to render a world square, seed neighbour strips, assemble, cut tiles
await evaluate(`(() => {
	window.__body = ${JSON.stringify(svg.slice(head[0].length))};
	window.__root = ${JSON.stringify(head[0])};
	const svgImage = async (vx, vy, vw, vh, pw, ph) => {
		const root = window.__root.replace(/\\bwidth="[^"]*"/, 'width="' + pw + '"').replace(/\\bheight="[^"]*"/, 'height="' + ph + '"').replace(/\\bviewBox="[^"]*"/, 'viewBox="' + vx + ' ' + vy + ' ' + vw + ' ' + vh + '"');
		const url = URL.createObjectURL(new Blob([root, window.__body], { type: 'image/svg+xml' }));
		const img = new Image();
		img.src = url;
		await img.decode();
		URL.revokeObjectURL(url);
		return img;
	};
	const toB64 = async (canvas, type = 'image/png', quality) => {
		const b = new Uint8Array(await (await canvas.convertToBlob({ type, quality })).arrayBuffer());
		let s = '';
		for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode(...b.subarray(i, i + 0x8000));
		return btoa(s);
	};
	const load = async (b64) => { const img = new Image(); img.src = 'data:image/png;base64,' + b64; await img.decode(); return img; };
	window.__svgImage = svgImage;
	window.__toB64 = toB64;
	window.__load = load;
	/** a region's layout: the sea flat, the land from the SVG; null when it has no land */
	window.__layout = async (vx, vy, vs, px, sea) => {
		const img = await svgImage(vx, vy, vs, vs, px, px);
		const c = new OffscreenCanvas(px, px), g = c.getContext('2d');
		g.drawImage(img, 0, 0, px, px);
		const a = g.getImageData(0, 0, px, px).data;
		let land = 0;
		for (let i = 3; i < a.length; i += 16) if (a[i] > 128) land++;
		if (land < 50) return null;
		g.globalCompositeOperation = 'destination-over';
		g.fillStyle = sea;
		g.fillRect(0, 0, px, px);
		return toB64(c);
	};
	/** the layout with the neighbours' painted pixels in its overlap bands (dx / dy in px; band = the overlap in px) */
	window.__seed = async (layout, neighbours, px, band) => {
		const c = new OffscreenCanvas(px, px), g = c.getContext('2d');
		g.drawImage(await load(layout), 0, 0);
		for (const n of neighbours) {
			const img = await load(n.b64);
			g.save();
			g.beginPath();
			// only the bands this neighbour shares: left / right columns, top / bottom rows
			const x0 = n.dx < 0 ? 0 : n.dx > 0 ? px - band : 0, x1 = n.dx < 0 ? band : n.dx > 0 ? px : px;
			const y0 = n.dy < 0 ? 0 : n.dy > 0 ? px - band : 0, y1 = n.dy < 0 ? band : n.dy > 0 ? px : px;
			g.rect(x0, y0, x1 - x0, y1 - y0);
			g.clip();
			g.drawImage(img, n.dx, n.dy, px, px);
			g.restore();
		}
		return toB64(c);
	};
	return true;
})()`);

// ── regions ─────────────────────────────────────────────────────────────────────────────────────────────────────────
const pad = (REGION - STEP) / 2;
const regions = [];
for (let gy = 0; gy * STEP < world.height; gy++) for (let gx = 0; gx * STEP < world.width; gx++) regions.push({ gx, gy, x: gx * STEP - pad, y: gy * STEP - pad, file: join(work, `r_${gx}_${gy}.png`) });
const layouts = new Map();
for (const r of regions) {
	const layout = await evaluate(`window.__layout(${r.x}, ${r.y}, ${REGION}, ${PX}, ${JSON.stringify(SEA)})`);
	if (layout) layouts.set(r, layout);
}
const land = regions.filter((r) => layouts.has(r));
const todo = land.filter((r) => (only ? `${r.gx}_${r.gy}` === only : !existsSync(r.file)));
console.log(`world ${world.width} × ${world.height}: ${regions.length} regions, ${land.length} with land, ${land.filter((r) => existsSync(r.file)).length} painted; to paint now: ${todo.length} (${model} ≈ $${(todo.length * (model === 'uni-1-max' ? 0.1 : 0.04)).toFixed(2)})`);

async function luma(source, refs) {
	const h = { authorization: `Bearer ${process.env.LUMA_AGENTS_API_KEY}`, 'content-type': 'application/json' };
	const body = { type: 'image_edit', model, prompt: PROMPT, output_format: 'png', source: { data: source, media_type: 'image/png' }, image_ref: refs };
	const r = await fetch('https://agents.lumalabs.ai/v1/generations', { method: 'POST', headers: h, body: JSON.stringify(body) });
	let j = await r.json();
	if (!r.ok) throw new Error(`luma ${r.status}: ${JSON.stringify(j).slice(0, 300)}`);
	while (!['completed', 'failed'].includes(j.state)) {
		await sleep(3000);
		j = await (await fetch(`https://agents.lumalabs.ai/v1/generations/${j.id}`, { headers: h })).json();
	}
	if (j.state === 'failed') throw new Error(`luma: ${j.failure_code ?? ''} ${j.failure_reason ?? ''}`);
	return Buffer.from(await (await fetch(j.output.find((o) => o.type === 'image').url)).arrayBuffer());
}

try {
	if (dry) {
		for (const r of todo.slice(0, 3)) writeFileSync(join(work, `layout_${r.gx}_${r.gy}.png`), Buffer.from(layouts.get(r), 'base64'));
		console.log(`layouts of the first ${Math.min(3, todo.length)} in ${work}\n\nPROMPT:\n${PROMPT}`);
	} else if (!assembleOnly) {
		if (!process.env.LUMA_AGENTS_API_KEY) throw new Error('LUMA_AGENT_API_KEY is not set in .env');
		const styleRef = { data: readFileSync(join(ROOT, 'media', style)).toString('base64'), media_type: style.endsWith('.webp') ? 'image/webp' : 'image/png' };
		for (const r of todo) {
			// the neighbours already painted (row by row: above and left first) give their strips
			const neighbours = land
				.filter((n) => n !== r && Math.abs(n.gx - r.gx) <= 1 && Math.abs(n.gy - r.gy) <= 1 && existsSync(n.file))
				.map((n) => ({ b64: png64(n.file), dx: (n.gx - r.gx) * STEP * PPU, dy: (n.gy - r.gy) * STEP * PPU }));
			const source = neighbours.length ? await evaluate(`window.__seed(${JSON.stringify(layouts.get(r))}, ${JSON.stringify(neighbours)}, ${PX}, ${(REGION - STEP) * PPU})`) : layouts.get(r);
			writeFileSync(join(work, `source_${r.gx}_${r.gy}.png`), Buffer.from(source, 'base64'));
			const t0 = Date.now();
			const bytes = await luma(source, [styleRef]);
			// back to the region's size (the edit's output follows the source, but not always to the pixel)
			const fixed = await evaluate(`(async () => { const img = await window.__load(${JSON.stringify(bytes.toString('base64'))}); const c = new OffscreenCanvas(${PX}, ${PX}); c.getContext('2d').drawImage(img, 0, 0, ${PX}, ${PX}); return window.__toB64(c); })()`);
			writeFileSync(r.file, Buffer.from(fixed, 'base64'));
			console.log(`r_${r.gx}_${r.gy} painted (${Math.round((Date.now() - t0) / 1000)} s)`);
		}
	}

	if (!dry && !only) {
		const painted = land.filter((r) => existsSync(r.file));
		if (painted.length < land.length) console.log(`assemble: ${land.length - painted.length} land regions not painted yet – they stay empty`);
		// one picture of the world at PPU px per unit: each region over the earlier ones, its overlap bands feathered in
		const W = Math.ceil(world.width * PPU), H = Math.ceil(world.height * PPU);
		await evaluate(`(() => { window.__world = new OffscreenCanvas(${W}, ${H}); return true; })()`);
		for (const r of painted)
			await evaluate(`(async () => {
				const img = await window.__load(${JSON.stringify(png64(r.file))});
				const px = ${PX}, band = ${(REGION - STEP) * PPU};
				const c = new OffscreenCanvas(px, px), g = c.getContext('2d');
				g.drawImage(img, 0, 0);
				g.globalCompositeOperation = 'destination-in';
				const ramp = (x0, y0, x1, y1) => { const l = g.createLinearGradient(x0, y0, x1, y1); l.addColorStop(0, 'rgba(0,0,0,0)'); l.addColorStop(1, 'rgba(0,0,0,1)'); return l; };
				${r.gx > 0 ? "g.fillStyle = ramp(0, 0, band, 0); g.fillRect(0, 0, band, px); g.fillStyle = '#000'; g.fillRect(band, 0, px - band, px);" : ''}
				${r.gy > 0 ? 'const t = new OffscreenCanvas(px, px), tg = t.getContext("2d"); tg.drawImage(c, 0, 0); g.clearRect(0, 0, px, px); g.globalCompositeOperation = "source-over"; g.drawImage(t, 0, 0); g.globalCompositeOperation = "destination-in"; g.fillStyle = ramp(0, 0, 0, band); g.fillRect(0, 0, px, band); g.fillStyle = "#000"; g.fillRect(0, band, px, px - band);' : ''}
				window.__world.getContext('2d').drawImage(c, ${r.x * PPU}, ${r.y * PPU});
				return true;
			})()`);
		// the land mask: the SVG at the same scale (its alpha) – the sea becomes transparent again
		await evaluate(`(async () => {
			const img = await window.__svgImage(0, 0, ${world.width}, ${world.height}, ${W}, ${H});
			const g = window.__world.getContext('2d');
			g.globalCompositeOperation = 'destination-in';
			g.drawImage(img, 0, 0, ${W}, ${H});
			g.globalCompositeOperation = 'source-over';
			return true;
		})()`);
		// the base image: the world at 1 px per unit
		const base = await evaluate(`(async () => { const c = new OffscreenCanvas(${world.width}, ${world.height}); c.getContext('2d').drawImage(window.__world, 0, 0, ${world.width}, ${world.height}); return window.__toB64(c, 'image/webp', 0.85); })()`);
		mkdirSync(dirname(join(ROOT, 'media', out)), { recursive: true });
		writeFileSync(join(ROOT, 'media', `${out}.webp`), Buffer.from(base, 'base64'));
		// tiles, level 1 … levels (level z: 2^z px per unit; the deepest one PPU)
		for (let z = 1; z <= levels; z++) {
			const size = TILE_PX / 2 ** z, cols = Math.ceil(world.width / size), rows = Math.ceil(world.height / size);
			let written = 0;
			for (let y = 0; y < rows; y++) {
				const row = await evaluate(`Promise.all(Array.from({ length: ${cols} }, async (_, x) => {
					const c = new OffscreenCanvas(${TILE_PX}, ${TILE_PX}), g = c.getContext('2d');
					const s = ${size * PPU};
					g.drawImage(window.__world, x * s, ${y} * s, s, s, 0, 0, ${TILE_PX}, ${TILE_PX});
					const a = g.getImageData(0, 0, ${TILE_PX}, ${TILE_PX}).data;
					let ink = false;
					for (let i = 3; i < a.length; i += 4) if (a[i]) { ink = true; break; }
					return ink ? window.__toB64(c, 'image/webp', 0.85) : null;
				}))`);
				row.forEach((b64, x) => {
					if (!b64) return;
					const file = join(ROOT, 'media', tilesOut, String(z), `${x}_${y}.webp`);
					mkdirSync(dirname(file), { recursive: true });
					writeFileSync(file, Buffer.from(b64, 'base64'));
					written++;
				});
			}
			console.log(`tiles level ${z}: ${written}`);
		}
		console.log(`layer: { id: "${name}", image: "${out}.webp", tiles: { path: "${tilesOut}", levels: ${levels} } }`);
	}
} finally {
	chrome.kill();
	await sleep(500);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
