// The map's landmasses, numbered (owner 2026-10-11: which island is a desert, a steppe, the Alps …): renders the terrain
// SVG at 1 px per world unit in headless Chrome, finds the connected landmasses (the sea is transparent) and writes
//   logs/map-paint/zones-sheet.png  – the world with every landmass ≥ --min px numbered (to label by eye)
//   logs/map-paint/landmasses.json  – id, area, bbox, a point inside (world units)
// The looks go into scripts/map-zones.json as { "at": { x, y }, "look": "…" } – a point inside the island, so it stays
// right when the numbering changes; scripts/map-paint.mjs reads them into each region's prompt.
//   node --experimental-transform-types scripts/map-zones.mjs [--svg catpope/map/map-layer-terrain.svg] [--min 150]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const svgPath = arg('svg', 'catpope/map/map-layer-terrain.svg');
const min = Number(arg('min', '150'));
const svg = readFileSync(join(ROOT, 'media', svgPath), 'utf8');
const work = join(ROOT, 'logs', 'map-paint');
mkdirSync(work, { recursive: true });

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9990 + Math.floor(Math.random() * 9);
const profile = mkdtempSync(join(tmpdir(), 'map-zones-'));
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
	const result = await evaluate(`(async () => {
		const url = URL.createObjectURL(new Blob([${JSON.stringify(svg)}], { type: 'image/svg+xml' }));
		const img = new Image();
		img.src = url;
		await img.decode();
		const W = img.naturalWidth, H = img.naturalHeight;
		const c = new OffscreenCanvas(W, H), g = c.getContext('2d');
		g.drawImage(img, 0, 0);
		const a = g.getImageData(0, 0, W, H).data;
		// connected landmasses (4-neighbourhood) on the alpha
		const label = new Int32Array(W * H);
		const masses = [];
		const stack = [];
		for (let i = 0; i < W * H; i++) {
			if (label[i] || a[i * 4 + 3] < 128) continue;
			const id = masses.length + 1;
			let area = 0, x0 = W, y0 = H, x1 = 0, y1 = 0, sx = 0, sy = 0;
			label[i] = id;
			stack.push(i);
			while (stack.length) {
				const j = stack.pop(), x = j % W, y = (j / W) | 0;
				area++; sx += x; sy += y;
				if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
				for (const k of [j - 1, j + 1, j - W, j + W]) {
					if (k < 0 || k >= W * H || label[k] || a[k * 4 + 3] < 128) continue;
					if ((k === j - 1 && x === 0) || (k === j + 1 && x === W - 1)) continue;
					label[k] = id;
					stack.push(k);
				}
			}
			masses.push({ id, area, bbox: { x: x0, y: y0, width: x1 - x0 + 1, height: y1 - y0 + 1 }, centroid: { x: Math.round(sx / area), y: Math.round(sy / area) } });
		}
		const big = masses.filter((m) => m.area >= ${min}).sort((p, q) => q.area - p.area);
		// a point inside each (the centroid when it is on land, else the land pixel nearest to it)
		for (const m of big) {
			let best = null, d = Infinity;
			for (let y = m.bbox.y; y < m.bbox.y + m.bbox.height; y += 2) for (let x = m.bbox.x; x < m.bbox.x + m.bbox.width; x += 2) {
				if (label[y * W + x] !== m.id) continue;
				const e = (x - m.centroid.x) ** 2 + (y - m.centroid.y) ** 2;
				if (e < d) { d = e; best = { x, y }; }
			}
			m.at = best;
		}
		// the sheet: the world on a sea colour, each landmass numbered (by size: 1 = the biggest)
		const s = new OffscreenCanvas(W, H), sg = s.getContext('2d');
		sg.fillStyle = '#3d8fd4'; sg.fillRect(0, 0, W, H);
		sg.drawImage(img, 0, 0);
		sg.font = 'bold 22px sans-serif'; sg.textAlign = 'center'; sg.textBaseline = 'middle';
		big.forEach((m, i) => {
			m.n = i + 1;
			sg.strokeStyle = 'rgba(220,0,90,0.8)'; sg.lineWidth = 2;
			sg.strokeRect(m.bbox.x, m.bbox.y, m.bbox.width, m.bbox.height);
			sg.fillStyle = '#dc005a'; sg.beginPath(); sg.arc(m.at.x, m.at.y, 15, 0, Math.PI * 2); sg.fill();
			sg.fillStyle = '#fff'; sg.fillText(String(m.n), m.at.x, m.at.y + 1);
		});
		const b = new Uint8Array(await (await s.convertToBlob({ type: 'image/png' })).arrayBuffer());
		let str = '';
		for (let i = 0; i < b.length; i += 0x8000) str += String.fromCharCode(...b.subarray(i, i + 0x8000));
		return { sheet: btoa(str), masses: big.map(({ n, area, bbox, at }) => ({ n, area, bbox, at })) };
	})()`);
	writeFileSync(join(work, 'zones-sheet.png'), Buffer.from(result.sheet, 'base64'));
	writeFileSync(join(work, 'landmasses.json'), JSON.stringify(result.masses, null, 1));
	console.log(`${result.masses.length} landmasses ≥ ${min} px → logs/map-paint/zones-sheet.png, landmasses.json`);
	for (const m of result.masses.slice(0, 25)) console.log(`${m.n}: area ${m.area}, bbox ${m.bbox.x},${m.bbox.y} ${m.bbox.width}×${m.bbox.height}, at ${m.at.x},${m.at.y}`);
} finally {
	chrome.kill();
	await sleep(500);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
