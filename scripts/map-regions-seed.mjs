// The map's regions, pre-drawn (agents/MAP.md §8.10, owner 2026-10-12: "according to the regions we identified – pre drawn
// with hexagons of course"): the terrain SVG rendered by headless Chrome, its landmasses found (the same flood fill as
// scripts/map-paint.mjs), each zone of scripts/map-zones.json → one region whose cover is every map hex cell (HEX_CELL)
// whose centre lies on the zone's land. Writes
//   logs/map-paint/regions.json                               the regions with their covers (row runs [r, q0, q1])
//   server/migrations/mysql/2026-10-12-map-regions.mysql.sql  the seed: one published `region:<id>` document each
//                                                             (idempotent: only where no region document exists yet)
// The owner corrects the covers with the hex brush in the map builder – a region is never drawn from scratch.
//   node --experimental-transform-types scripts/map-regions-seed.mjs [--svg catpope/map/map-layer-terrain.svg]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { HEX_CELL, hexCenter } from '../shared/content/src/map.ts';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const svgPath = arg('svg', 'catpope/map/map-layer-terrain.svg');
const svgFile = [join(ROOT, 'media', svgPath), join(ROOT, '..', '..', '..', 'media', svgPath)].find((f) => existsSync(f));
if (!svgFile) throw new Error(`media/${svgPath} missing (npm run dev:db -- media --apply)`);
const svg = readFileSync(svgFile, 'utf8');
const head = svg.match(/^<svg\b[^>]*>/);
const world = { width: Number(head[0].match(/width="([\d.]+)"/)[1]), height: Number(head[0].match(/height="([\d.]+)"/)[1]) };
const zones = JSON.parse(readFileSync(join(ROOT, 'scripts', 'map-zones.json'), 'utf8')).zones;
const REGIONS = JSON.parse(readFileSync(join(ROOT, 'scripts', 'map-regions.json'), 'utf8')).regions;
// a zone → its region (ids, names and claims as on the example signs, scripts/region-signs.mjs)
const ZONE_REGION = { desert: 'dunes', altiplano: 'altiplano', alps: 'alps', jungle: 'jungle', wasteland: 'wasteland', archipelago: 'archipelago', mountains: 'highlands' };
const COLOURS = { dunes: '#ff6a3d', altiplano: '#e8b500', alps: '#7fd8ff', jungle: '#1fae5b', wasteland: '#c7d800', archipelago: '#d04fd8', highlands: '#b4553a' };
const GREETINGS = {
	dunes: { de: 'Willkommen im Dünenmeer. Wasser bitte selbst mitbringen.', en: 'Welcome to the Sea of Dunes. Bring your own water.' },
	altiplano: { de: 'Hier oben ist die Luft dünn und die Farben dick.', en: 'Up here the air is thin and the colours are thick.' },
	alps: { de: 'Ewigschnee grüßt. Mütze auf.', en: 'Everlasting Snow says hello. Hat on.' },
	jungle: { de: 'Leise treten. Hier hört alles mit.', en: 'Tread softly. Everything here is listening.' },
	wasteland: { de: 'Fracktal. Wir sind noch da.', en: 'Fracktal. We are still here.' },
	archipelago: { de: 'Jede Insel ein Schatz. Bitte nichts einstecken.', en: 'Every island a treasure. Please take nothing.' },
	highlands: { de: 'Grüß Gott in den Erzbergen. Geizig, garstig, steil – und deins.', en: 'Welcome to the Ore Mountains. Stingy, grumpy, steep – and yours.' }
};

// ── Chrome: render the SVG at 2 px per unit, label the landmasses ───────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9900 + Math.floor(Math.random() * 50);
const profile = mkdtempSync(join(tmpdir(), 'map-regions-'));
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
const S = 2; // px per world unit
try {
	const masses = await evaluate(`(async () => {
		const W = ${Math.ceil(world.width * S)}, H = ${Math.ceil(world.height * S)};
		const root = ${JSON.stringify(head[0])}.replace(/\\bwidth="[^"]*"/, 'width="' + W + '"').replace(/\\bheight="[^"]*"/, 'height="' + H + '"');
		const url = URL.createObjectURL(new Blob([root, ${JSON.stringify(svg.slice(head[0].length))}], { type: 'image/svg+xml' }));
		const img = new Image(); img.src = url; await img.decode();
		const c = new OffscreenCanvas(W, H), g = c.getContext('2d'); g.drawImage(img, 0, 0);
		const a = g.getImageData(0, 0, W, H).data;
		const label = new Int32Array(W * H), stack = [], masses = [];
		for (let i = 0; i < W * H; i++) {
			if (label[i] || a[i * 4 + 3] < 128) continue;
			const id = masses.length + 1; let n = 0, sx = 0, sy = 0;
			label[i] = id; stack.push(i);
			while (stack.length) {
				const j = stack.pop(), x = j % W; n++; sx += x; sy += (j / W) | 0;
				for (const k of [j - 1, j + 1, j - W, j + W]) {
					if (k < 0 || k >= W * H || label[k] || a[k * 4 + 3] < 128 || (k === j - 1 && x === 0) || (k === j + 1 && x === W - 1)) continue;
					label[k] = id; stack.push(k);
				}
			}
			masses.push({ id, n, cx: sx / n / ${S}, cy: sy / n / ${S} });
		}
		window.__labels = label; window.__W = W; window.__H = H;
		return masses;
	})()`);
	const labelAt = (p) => evaluate(`(() => { const x = Math.round(${p.x} * ${S}), y = Math.round(${p.y} * ${S}); return x < 0 || y < 0 || x >= window.__W || y >= window.__H ? 0 : window.__labels[y * window.__W + x]; })()`);
	// landmass → region: `at` zones first, then `rect` zones (the scripts/map-paint.mjs rule)
	const regionOf = new Map();
	for (const z of zones) if (z.at && ZONE_REGION[z.name]) { const id = await labelAt(z.at); if (id) regionOf.set(id, ZONE_REGION[z.name]); else console.log(`zone ${z.name}: not on land`); }
	for (const z of zones) if (z.rect && ZONE_REGION[z.name]) for (const m of masses) if (!regionOf.has(m.id) && m.cx >= z.rect.x && m.cx <= z.rect.x + z.rect.width && m.cy >= z.rect.y && m.cy <= z.rect.y + z.rect.height) regionOf.set(m.id, ZONE_REGION[z.name]);
	await evaluate(`(() => { window.__region = ${JSON.stringify(Object.fromEntries(regionOf))}; return true; })()`);
	// every cell centre on the map: its region (row by row, one round trip per row)
	const R = HEX_CELL / Math.sqrt(3);
	const rows = Math.ceil(world.height / (1.5 * R)) + 1;
	const runs = Object.fromEntries(Object.values(ZONE_REGION).map((id) => [id, []]));
	for (let r = 0; r < rows; r++) {
		const q0 = Math.floor(-r / 2) - 1, q1 = Math.ceil(world.width / HEX_CELL - r / 2) + 1;
		const centres = [];
		for (let q = q0; q <= q1; q++) centres.push(hexCenter({ q, r }));
		const ids = await evaluate(`${JSON.stringify(centres.map((c) => [c.x, c.y]))}.map(([x, y]) => { const px = Math.round(x * ${S}), py = Math.round(y * ${S}); if (px < 0 || py < 0 || px >= window.__W || py >= window.__H) return null; return window.__region[window.__labels[py * window.__W + px]] ?? null; })`);
		let start = null, current = null;
		ids.forEach((id, k) => {
			if (id !== current) {
				if (current) runs[current].push([r, q0 + start, q0 + k - 1]);
				current = id;
				start = k;
			}
		});
		if (current) runs[current].push([r, q0 + start, q0 + ids.length - 1]);
	}
	const regions = REGIONS.map((s) => {
		const zone = zones.find((z) => ZONE_REGION[z.name] === s.id);
		const cells = runs[s.id] ?? [];
		return {
			id: s.id,
			doc: {
				name: { de: s.name, en: s.name },
				claim: { de: s.claim, en: s.claim },
				cells,
				vegetation: zone?.look ?? '',
				greeting: GREETINGS[s.id],
				colour: COLOURS[s.id],
				// the welcome sign (scripts/region-signs.mjs, uploaded to the CDN as a new path)
				sign: { src: `catpope/map/regions/${s.id}/sign.webp` }
			},
			count: cells.reduce((n, [, a, b]) => n + b - a + 1, 0)
		};
	});
	for (const r of regions) console.log(`${r.id}: ${r.count} cells in ${r.doc.cells.length} runs`);
	mkdirSync(join(ROOT, 'logs', 'map-paint'), { recursive: true });
	writeFileSync(join(ROOT, 'logs', 'map-paint', 'regions.json'), JSON.stringify(regions, null, '\t'));
	// the migration: one published document per region, only where no region document exists yet (a second run changes nothing)
	const esc = (s) => s.replace(/\\/g, '\\\\').replace(/'/g, "''");
	const sql = [
		'-- The map\'s regions, pre-drawn from the painted map\'s zones (agents/MAP.md §8.10, owner 2026-10-12: "according to the',
		'-- regions we identified – pre drawn with hexagons of course"). Generated by scripts/map-regions-seed.mjs: one DRAFT',
		'-- `region:<id>` document per zone, its cover = the map hex cells on the zone\'s land as row runs [r, q0, q1].',
		'-- Idempotent: a region that already has any version is left alone; only where a world map exists (an empty / test',
		'-- database stays empty).',
		'-- Check: SELECT entity_key, status FROM entity_versions WHERE kind = \'region\';',
		...regions.map(
			(r) =>
				`INSERT INTO \`entity_versions\` (\`entity_key\`, \`kind\`, \`toon\`, \`locale\`, \`json\`, \`author\`, \`created_at\`, \`status\`, \`message\`)\n` +
				`SELECT 'region:${r.id}', 'region', NULL, NULL, '${esc(JSON.stringify(r.doc))}', NULL, NOW(), 'draft', 'pre-drawn from the painted map zones – to review'\n` +
				`FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM \`entity_versions\` WHERE \`entity_key\` = 'region:${r.id}')
` +
				`  AND EXISTS (SELECT 1 FROM \`entity_versions\` WHERE \`entity_key\` = 'map');`
		)
	].join('\n');
	writeFileSync(join(ROOT, 'server', 'migrations', 'mysql', '2026-10-12-map-regions.mysql.sql'), sql + '\n');
	console.log(`migration: ${Math.round(sql.length / 1024)} KB`);
} finally {
	chrome.kill();
	await sleep(400);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
