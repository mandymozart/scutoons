// World map check (agents/MAP.md M0–M2): the reader's map and the creator's map builder, headless Chrome via CDP (own temp
// profile, no dependencies; Node 22 WebSocket), like scripts/check-next-bar.mjs.
//
// Reader (guest; 390 de and 1440 en):
//   1. a fresh guest: the map opens (toggle), its sea is the toon's colour, no place is visible (catpope's places unlock
//      over time), the list is empty;
//   2. reading progress (chapter 1 read, chapter 2 at 70 %) unlocks exactly the places whose unlock point was reached
//      (computed here from /library/map), written to the world state `scutoons_game_scutoons`; the toggle shows the badge,
//      the opened map shows the places with the halo, then marks them seen (badge gone);
//   3. pan: a mouse drag moves the world by the drag; zoom: "+" (×1.5) and the wheel (at the cursor) change the scale;
//   4. no two visible labels overlap (the phone overlap fix);
//   5. a click on a place opens its card (name; "Kapitel N lesen" only when that chapter is readable for this reader);
//   6. "Orte" lists the visible places A–Z;
//   7. universe-wide: in another toon (the sandbox) the unlocked catpope places are on the map as plain markers (no label);
// Member (the test user, `npm run dev:test-user`):
//   8. a guest's discoveries merge into the account's world state on sign-in (set merge, nothing lost on either side);
//      the member's server state is restored afterwards.
// Builder (creator /map as the test user):
//   9. "+ Ort": a click on the canvas adds a place, the de name sets its id; a drag moves it (x/y follow the drag);
//  10. "Weg zeichnen": the new place + a free point + Enter → a route with 2 waypoints (place, free point);
//  11. Save draft → the stored draft has the place and the route; the published map is restored afterwards (the map must
//      have no unpublished draft before the check, else it refuses to run – someone's work in progress is never touched).
//
// usage: node scripts/check-map.mjs [--viewer=http://localhost:8000] [--creator=http://localhost:10000]
//        [--api=http://localhost:9000/api] [--shots=<dir>] [--only=reader|member|builder] [--verbose]
// Exit code 1 on any failure.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const creator = String(opt.creator || local('creator')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const only = opt.only ? String(opt.only) : null;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));

// ── the test user ───────────────────────────────────────────────────────────────────────────────────────────────────
const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
const worldBefore = await (await fetch(`${api}/progress/scutoons/`, { headers: auth })).json();
const world = Object.fromEntries(await Promise.all(['de', 'en'].map(async (l) => [l, await (await fetch(`${api}/library/map?locale=${l}&toon=catpope`)).json()])));
const bundle = Object.fromEntries(await Promise.all(['de', 'en'].map(async (l) => [l, await (await fetch(`${api}/library/toons/catpope?locale=${l}`)).json()])));
const mapEntity = await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json();

/** a #rrggbb / #rgb colour as the browser reports it */
function rgb(hex) {
	const h = hex.replace('#', '');
	const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h;
	return `rgb(${parseInt(full.slice(0, 2), 16)}, ${parseInt(full.slice(2, 4), 16)}, ${parseInt(full.slice(4, 6), 16)})`;
}

/** the places a reader with these checkpoints has unlocked (@scutoons/content unlocksFromProgress) */
function unlocked(places, checkpoints) {
	return places.filter((p) => {
		const u = p.unlock;
		if (!p.hidden || !u || u.toon !== 'catpope') return false;
		const cp = checkpoints.find((c) => c.uri === `catpope/chapter/${u.chapter}`);
		return !!cp && (cp.isComplete || (cp.position ?? 0) >= (u.at ?? 0));
	}).map((p) => p.id);
}
const now = new Date().toISOString();
const PROGRESS = [
	{ uri: 'catpope/chapter/1', isComplete: true, position: 1, created_at: now, updated_at: now },
	{ uri: 'catpope/chapter/2', isComplete: false, position: 0.7, created_at: now, updated_at: now }
];

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-map-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restoreMap = null;
async function restore() {
	await fetch(`${api}/progress/scutoons/`, { method: 'PUT', headers: auth, body: JSON.stringify({ state: worldBefore?.state ?? {}, updated_at: Date.now() }) }).catch(() => {});
	if (restoreMap) await restoreMap().catch((e) => console.log(`  could not restore the map: ${e}`));
}
async function done(code) {
	await restore();
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	process.exit(code);
}
process.on('uncaughtException', (e) => { console.error(e); done(2); });

let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); let lastNet = Date.now();
const consoleErrors = [];
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	if (m.method === 'Runtime.exceptionThrown') consoleErrors.push(p.exceptionDetails?.exception?.description ?? p.exceptionDetails?.text);
	if (m.method === 'Runtime.consoleAPICalled' && p.type === 'error') consoleErrors.push(p.args?.map((a) => a.value ?? a.description).join(' '));
	if (m.method === 'Page.javascriptDialogOpening') send('Page.handleJavaScriptDialog', { accept: true });
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
async function idle(quietMs = 700, maxMs = 15000) {
	const t0 = Date.now();
	while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); }
}
async function waitFor(js, ms = 15000) {
	const t0 = Date.now();
	let r;
	while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(150); }
	return null;
}
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
const center = (selector) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
async function mouse(type, x, y, extra = {}) { await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1, ...extra }); }
async function click(selector) {
	const p = await center(selector);
	if (!p || p.error) return false;
	await mouse('mousePressed', p.x, p.y);
	await mouse('mouseReleased', p.x, p.y);
	return true;
}
async function clickAt(x, y, clickCount = 1) { await mouse('mousePressed', x, y, { clickCount }); await mouse('mouseReleased', x, y, { clickCount }); }
async function drag(from, to, steps = 8) {
	await mouse('mousePressed', from.x, from.y);
	for (let i = 1; i <= steps; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps, button: 'left', buttons: 1 });
	await mouse('mouseReleased', to.x, to.y);
}
async function typeText(text) { for (const ch of text) await send('Input.dispatchKeyEvent', { type: 'char', text: ch }); }
async function key(k, code = k) { for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: k === 'Enter' ? 13 : 0 }); }

let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
async function setLocale(locale, url) { await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url, path: '/' }); }

const MAP = `(() => {
	const o = document.querySelector('[data-testid="map-overlay"]');
	const w = document.querySelector('[data-testid="map-world"]');
	const t = w ? getComputedStyle(w).transform : null;
	const m = t && t !== 'none' ? t.match(/matrix\\(([^)]+)\\)/)[1].split(',').map(Number) : null;
	const labels = [...document.querySelectorAll('[data-testid^="map-label-"]')].map((l) => { const b = l.getBoundingClientRect(); return { id: l.dataset.testid.slice(10), x0: b.left, y0: b.top, x1: b.right, y1: b.bottom, text: l.textContent }; });
	return { open: !!o?.classList.contains('visible'), places: [...document.querySelectorAll('[data-testid^="map-place-"]')].map((b) => b.dataset.testid.slice(10)),
		newPlaces: [...document.querySelectorAll('.place.new [data-testid^="map-place-"]')].map((b) => b.dataset.testid.slice(10)),
		labels, scale: m ? m[0] : null, tx: m ? m[4] : null, ty: m ? m[5] : null, badge: !!document.querySelector('[data-testid="map-badge"]'),
		card: document.querySelector('[data-testid="map-card"] h2')?.textContent ?? null, cardLink: document.querySelector('[data-testid="map-card-chapter"]')?.textContent.trim() ?? null,
		list: [...document.querySelectorAll('[data-testid="map-list"] li button')].map((b) => b.firstChild?.textContent.trim()),
		world: JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null') };
})()`;

// ── reader ──────────────────────────────────────────────────────────────────────────────────────────────────────────
async function reader(width, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	const data = world[locale];
	console.log(`reader ${width} ${locale}`);
	await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
	await send('Network.clearBrowserCookies');
	await setLocale(locale, viewer);
	await open(`${viewer}${prefix}/catpope`);
	await evaluate('localStorage.clear(), sessionStorage.clear(), true');
	await open(`${viewer}${prefix}/catpope`);

	// 1. fresh guest
	await click('[data-testid="map-toggle"]');
	let s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.scale ? s : null; })()`);
	report('fresh guest: the map opens, no place visible yet (they unlock while reading)', s && s.places.length === 0 ? [] : [JSON.stringify(s && { open: s.open, places: s.places })]);
	{
		// the sea is the toon's colour (owner 2026-10-01: /library/map view.color = tile colour, else the star's)
		const bg = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-overlay"]')).backgroundColor`);
		const want = rgb(data.view?.color ?? '#00ff00');
		report(`the sea is the toon's colour (${data.view?.color ?? 'default'})`, bg === want ? [] : [`${bg} ≠ ${want}`]);
	}
	await click('[data-testid="map-list-toggle"]');
	const empty = await waitFor(`document.querySelector('[data-testid="map-list"] p')?.textContent ?? null`, 3000);
	report('"Orte" says nothing was discovered yet', empty === msg.map.places_empty ? [] : [`"${empty}"`]);
	await shot(`map-${width}-${locale}-1-empty`);

	// 2. progress → unlocks
	const expected = unlocked(data.locations, PROGRESS).sort();
	await evaluate(`localStorage.setItem('scutoons_game_catpope', JSON.stringify({ toonId: 'catpope', variables: {}, progress: ${JSON.stringify(PROGRESS)}, created_at: '${now}', updated_at: '${now}' })), true`);
	await open(`${viewer}${prefix}/catpope`);
	s = await waitFor(`(() => { const s = ${MAP}; return s.badge && s.world ? s : null; })()`, 10000);
	{
		const got = Object.keys(s?.world?.map?.places ?? {}).sort();
		report(`progress unlocks the reached places (${expected.length}: ${expected.join(', ')}) in the world state + badge`, s && JSON.stringify(got) === JSON.stringify(expected) ? [] : [`world state ${JSON.stringify(got)}`, `badge ${s?.badge}`]);
	}
	await click('[data-testid="map-toggle"]');
	s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.places.length ? s : null; })()`);
	report('the opened map shows them, new ones with the halo', s && JSON.stringify([...s.places].sort()) === JSON.stringify(expected) && s.newPlaces.length > 0 ? [] : [JSON.stringify(s && { places: s.places, isNew: s.newPlaces })]);
	await shot(`map-${width}-${locale}-2-unlocked`);
	const seen = await waitFor(`(() => { const s = ${MAP}; return !s.badge && Object.values(s.world.map.places).some((p) => p.seen) ? s : null; })()`, 6000);
	report('places on screen are marked seen, the badge goes', seen ? [] : [JSON.stringify((await evaluate(MAP)).badge)]);

	// 4. labels never overlap
	s = await evaluate(MAP);
	{
		const overlaps = [];
		for (let i = 0; i < s.labels.length; i++) for (let j = i + 1; j < s.labels.length; j++) {
			const a = s.labels[i], b = s.labels[j];
			if (a.x0 < b.x1 - 1 && a.x1 > b.x0 + 1 && a.y0 < b.y1 - 1 && a.y1 > b.y0 + 1) overlaps.push(`${a.text} × ${b.text}`);
		}
		report(`no overlapping labels (${s.labels.length} shown)`, s.labels.length && !overlaps.length ? [] : overlaps.length ? overlaps : ['no label shown']);
	}

	// 3. pan + zoom
	const vp = await center('[data-testid="map-viewport"]');
	const before = await evaluate(MAP);
	await drag({ x: vp.x - 40, y: vp.y + 60 }, { x: vp.x + 20, y: vp.y + 100 });
	let after = await evaluate(MAP);
	report('a drag pans the world by the drag (60, 40 px)', Math.abs(after.tx - before.tx - 60) <= 2 && Math.abs(after.ty - before.ty - 40) <= 2 ? [] : [`moved ${after.tx - before.tx}, ${after.ty - before.ty}`]);
	await click('[data-testid="map-zoom-in"]');
	after = await evaluate(MAP);
	report('"+" zooms ×1.5', Math.abs(after.scale / before.scale - 1.5) < 0.01 ? [] : [`scale ${before.scale} → ${after.scale}`]);
	await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: vp.x, y: vp.y, deltaX: 0, deltaY: 200 });
	await sleep(200);
	const wheeled = await evaluate(MAP);
	report('the wheel zooms out at the cursor', wheeled.scale < after.scale ? [] : [`scale ${after.scale} → ${wheeled.scale}`]);
	await shot(`map-${width}-${locale}-3-zoomed`);

	// 5. card
	const first = data.locations.find((p) => p.id === expected[0]);
	await evaluate(`(() => { document.querySelector('[data-testid="map-place-${first.id}"]').scrollIntoView(); return true; })()`);
	await click('[data-testid="map-list-toggle"]');
	const listed = await waitFor(`(() => { const s = ${MAP}; return s.list.length ? s.list : null; })()`, 3000);
	const names = expected.map((id) => data.locations.find((p) => p.id === id).name).sort((a, b) => a.localeCompare(b));
	report('"Orte" lists the visible places A–Z', JSON.stringify(listed) === JSON.stringify(names) ? [] : [JSON.stringify(listed), JSON.stringify(names)]);
	await evaluate(`(() => { [...document.querySelectorAll('[data-testid="map-list"] li button')].find((b) => b.firstChild?.textContent.trim() === ${JSON.stringify(first.name)})?.click(); return true; })()`);
	s = await waitFor(`(() => { const s = ${MAP}; return s.card ? s : null; })()`, 3000);
	const chapterItem = bundle[locale].seasons[0].chapters.find((c) => c.number === first.chapter?.number);
	const readable = !!chapterItem && ['published', 'draft'].includes(chapterItem.status) && !!chapterItem.isPreview;
	const link = readable ? fill(msg.map.read_chapter, { number: first.chapter.number }) : null;
	report(`a place's card: "${first.name}"${link ? ` + "${link}"` : ' (chapter not readable for a guest: no link)'}`, s?.card === first.name && s?.cardLink === link ? [] : [JSON.stringify({ card: s?.card, link: s?.cardLink })]);
	await shot(`map-${width}-${locale}-4-card`);

	// 7. universe-wide: the sandbox (dev toon) shows catpope's unlocked places as plain markers
	await open(`${viewer}${prefix}/sandbox`);
	const ok = await waitFor(`document.querySelector('[data-testid="map-toggle"]') ? true : null`, 8000);
	if (ok) {
		await click('[data-testid="map-toggle"]');
		s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.places.length ? s : null; })()`, 8000);
		report('universe-wide: another toon (sandbox) shows the discovered places, as markers without labels', s && expected.every((id) => s.places.includes(id)) && s.labels.length === 0 ? [] : [JSON.stringify(s && { places: s.places, labels: s.labels.map((l) => l.text) })]);
	} else report('universe-wide: the sandbox toon page', ['no map toggle on /sandbox']);
}

// ── member: guest discoveries merge on sign-in ──────────────────────────────────────────────────────────────────────
async function member() {
	console.log('member (test user)');
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
	await fetch(`${api}/progress/scutoons/`, { method: 'PUT', headers: auth, body: JSON.stringify({ state: { map: { places: { monastery: { unlocked: 5 } }, routes: {}, positions: {} } }, updated_at: Date.now() }) });
	await send('Network.clearBrowserCookies');
	await setLocale('de', viewer);
	await open(`${viewer}/catpope`);
	await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_game_scutoons', JSON.stringify({ map: { places: { apartment: { unlocked: 7, seen: 8 } }, routes: {}, positions: {} }, created_at: '${now}', updated_at: '${now}' })), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	await open(`${viewer}/catpope`);
	const merged = await waitFor(`(() => { const w = JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null'); return w && w.map.places.monastery && w.map.places.apartment ? w : null; })()`, 10000);
	let server = null;
	for (let i = 0; i < 20; i++) {
		server = await (await fetch(`${api}/progress/scutoons/`, { headers: auth })).json();
		if (server?.state?.map?.places?.apartment) break;
		await sleep(500);
	}
	const places = Object.keys(server?.state?.map?.places ?? {}).sort();
	report('sign-in merges the guest\'s discoveries with the account\'s (both devices keep everything)', merged && places.includes('apartment') && places.includes('monastery') ? [] : [`local ${JSON.stringify(merged?.map?.places)}`, `server ${JSON.stringify(places)}`]);
}

// ── builder ─────────────────────────────────────────────────────────────────────────────────────────────────────────
async function builder() {
	console.log('builder (creator /map)');
	if (mapEntity?.published?.id !== mapEntity?.version?.id) {
		report('the map has no unpublished draft (the check never touches work in progress)', [`newest version ${mapEntity?.version?.id} ≠ published ${mapEntity?.published?.id} – publish or discard it first`]);
		return;
	}
	const original = mapEntity.content;
	restoreMap = async () => {
		const put = await fetch(`${api}/entities/map`, { method: 'PUT', headers: auth, body: JSON.stringify({ content: original, message: 'check-map: restore' }) });
		if (put.ok) await fetch(`${api}/entities/map/publish`, { method: 'POST', headers: auth, body: '{}' });
	};
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
	await send('Network.clearBrowserCookies');
	await setLocale('de', creator);
	await open(`${creator}/`);
	await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	await open(`${creator}/map`);
	const ready = await waitFor(`document.querySelector('[data-testid="map-canvas"]') && document.querySelectorAll('[data-testid^="builder-place-"]').length ? true : null`, 15000);
	report('the builder opens with the world\'s places', ready ? [] : ['no canvas / places']);
	if (!ready) return;
	await shot('builder-1-open');

	// 9. add a place, name it, drag it
	const canvas = await evaluate(`(() => { const b = document.querySelector('[data-testid="map-canvas"]').getBoundingClientRect(); return { x: b.left, y: b.top, w: b.width, h: b.height }; })()`);
	await click('[data-testid="tool-place"]');
	const at = { x: canvas.x + canvas.w * 0.3, y: canvas.y + canvas.h * 0.3 };
	await clickAt(at.x, at.y);
	const idBefore = await waitFor(`document.querySelector('[data-testid="place-id"]')?.value || null`, 5000);
	const name = `Prüfort ${Date.now() % 100000}`;
	await waitFor(`document.activeElement?.dataset?.testid === 'place-name-de' ? true : null`, 3000);
	await typeText(name);
	const placeId = await waitFor(`(() => { const v = document.querySelector('[data-testid="place-id"]')?.value; return v && v !== ${JSON.stringify(idBefore)} ? v : null; })()`, 5000);
	const slug = name.toLowerCase().replace('ü', 'ue').replace(/[^a-z0-9]+/g, '-');
	report(`"+ Ort" adds a place at the click; the de name sets its id (${slug})`, placeId === slug ? [] : [`id ${idBefore} → ${placeId}`]);
	const pos0 = await evaluate(`({ x: Number(document.querySelector('[data-testid="place-x"]').value), y: Number(document.querySelector('[data-testid="place-y"]').value) })`);
	const zoom = (await evaluate(`Number(document.querySelector('[data-testid="zoom-pct"]').textContent.replace(/\\D/g, '')) / 100`)) || 1;
	const marker = await center(`[data-testid="builder-place-${placeId}"]`);
	await drag(marker, { x: marker.x + 60, y: marker.y + 40 });
	const pos1 = await waitFor(`(() => { const x = Number(document.querySelector('[data-testid="place-x"]').value); return x !== ${pos0.x} ? { x, y: Number(document.querySelector('[data-testid="place-y"]').value) } : null; })()`, 3000);
	report('a drag moves the place (world units = px / zoom)', pos1 && Math.abs(pos1.x - pos0.x - 60 / zoom) <= 2 && Math.abs(pos1.y - pos0.y - 40 / zoom) <= 2 ? [] : [`${JSON.stringify(pos0)} → ${JSON.stringify(pos1)} at zoom ${zoom}`]);

	// 10. draw a route: the new place → a free point → Enter
	await click('[data-testid="tool-route"]');
	const from = await center(`[data-testid="builder-place-${placeId}"]`);
	await clickAt(from.x, from.y);
	await clickAt(from.x + 90, from.y + 70);
	await key('Enter');
	const routeId = await waitFor(`document.querySelector('[data-testid="route-id"]')?.value || null`, 5000);
	const waypoints = await evaluate(`[...document.querySelectorAll('[data-testid="route-waypoints"] li')].length`);
	report('"Weg zeichnen": place + free point + Enter → a route with 2 waypoints, selected', routeId && waypoints === 2 ? [] : [`route ${routeId}, ${waypoints} waypoints`]);
	await shot('builder-2-route');

	// 11. save draft → stored
	await click('[data-testid="save-draft"]');
	let stored = null;
	for (let i = 0; i < 20; i++) {
		stored = await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json();
		if (stored?.version?.id !== mapEntity.version.id) break;
		await sleep(400);
	}
	const place = stored?.content?.locations?.find((p) => p.id === placeId);
	const route = stored?.content?.routes?.find((r) => r.id === routeId);
	report('Save draft stores the place (name, moved position) and the route (place + free waypoint)',
		place && place.name?.de === name && place.x === pos1?.x && route && route.waypoints.length === 2 && route.waypoints[0].location === placeId && typeof route.waypoints[1].x === 'number'
			? [] : [JSON.stringify({ place, route })]);

	// 12. a place image (any image of the CDN): it stands on the place's point (ground point 50 / 90 %), size + previews
	await click('[data-testid="tool-select"]');
	await evaluate(`(document.querySelector('[data-testid="nav-place-${placeId}"]')?.click(), true)`);
	await waitFor(`document.querySelector('[data-testid="place-image-path"]') ? true : null`, 3000);
	const image = 'characters/catpope/catpope_avatar-neutral.webp';
	await evaluate(`(() => { const i = document.querySelector('[data-testid="place-image-path"]'); i.value = ${JSON.stringify(image)}; i.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	const img = await waitFor(`(() => {
		const marker = document.querySelector('[data-testid="builder-place-${placeId}"]'); const im = marker?.parentElement.querySelector('.place-image');
		if (!im || !im.complete || !im.naturalWidth) return null;
		const m = marker.getBoundingClientRect(), b = im.getBoundingClientRect();
		return { w: Math.round(b.width), groundX: Math.round(b.left + b.width * 0.5 - (m.left + m.width / 2)), groundY: Math.round(b.top + b.height * 0.9 - (m.top + m.height / 2)), previews: document.querySelectorAll('.previews .preview img').length, anchor: !!document.querySelector('[data-testid="place-anchor"] img') };
	})()`, 8000);
	report('a place image stands on the point (48 px × zoom, ground point bottom centre), with the ground point editor and 3 previews',
		img && Math.abs(img.w - 48 * zoom) <= 2 && Math.abs(img.groundX) <= 2 && Math.abs(img.groundY) <= 2 && img.previews === 3 && img.anchor ? [] : [JSON.stringify(img)]);
	await evaluate(`(() => { const side = document.querySelector('.side'); const f = document.querySelector('[data-testid="place-image"]'); side.scrollTop = f.offsetTop - 20; return true; })()`);
	await shot('builder-3-image');

	// 13. reader view: the sea is the filtered toon's colour (tile colour, else its star's registry colour), else neutral
	const registry = await (await fetch(`${api}/library/characters`)).json();
	const list = await (await fetch(`${api}/library/toons?draft=1`, { headers: auth })).json();
	const colorOf = (t) => list.find((x) => x.id === t)?.tile?.color || registry.characters.find((c) => c.own_toon === t)?.color || null;
	const seas = {};
	await click('[data-testid="reader-view"]');
	for (const t of ['catpope', 'toptoy', 'yellowmouse']) {
		await click(`[data-testid="filter-${t}"]`);
		await sleep(150);
		seas[t] = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-canvas"]')).backgroundColor`);
		await shot(`builder-4-sea-${t}`);
	}
	await click('[data-testid="filter-all"]');
	const neutral = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-canvas"]')).backgroundColor`);
	const wrong = Object.entries(seas).filter(([t, bg]) => colorOf(t) && bg !== (colorOf(t).startsWith('#') ? rgb(colorOf(t)) : bg)).map(([t, bg]) => `${t}: ${bg} ≠ ${colorOf(t)}`);
	report(`reader view: the sea is the filtered toon's colour (${Object.keys(seas).map((t) => `${t} ${colorOf(t)}`).join(', ')}), neon green #00ff00 without a filter`, !wrong.length && neutral === rgb('#00ff00') ? [] : [...wrong, `neutral ${neutral}`]);

	// 14. bidirectional unlock (owner 2026-10-01): the chapter's slice sets "unlocks: <place>", the builder shows it with a
	//     link back – one storage, the place's unlock { toon, chapter, slice: <the slice's image>, at: <its reading point> }
	await restoreMap(); // a clean map again (the builder's test place is not needed here)
	const chapter = await (await fetch(`${api}/library/toons/catpope/chapters/1?draft=1`, { headers: auth })).json();
	const slices = (chapter.groups ?? []).flatMap((g, gi) => (g.slices ?? []).map((s, si) => ({ image: s.image ?? '', group: gi, index: si })));
	const target = slices.find((s) => s.group === 0 && s.index === 1) ?? slices[1];
	const k = slices.indexOf(target);
	const point = k === slices.length - 1 ? 1 : Math.floor(Math.min(0.99, (k + 1) / slices.length) * 10000) / 10000;
	const placeToUnlock = original.locations.find((p) => !p.unlock)?.id ?? original.locations[0].id;
	await open(`${creator}/catpope/chapter/1?group=${target.group + 1}&slice=${target.index + 1}`);
	const tab = await waitFor(`document.querySelector('[data-testid="tab-map"]') ? true : null`, 15000);
	if (!tab) {
		report('chapter editor: the slice inspector has the "Karte" tab', ['no tab-map']);
		return;
	}
	await click('[data-testid="tab-map"]');
	await waitFor(`document.querySelector('[data-testid="slice-unlock-add"]') ? true : null`, 8000);
	await evaluate(`(() => { const s = document.querySelector('[data-testid="slice-unlock-add"]'); s.value = ${JSON.stringify(placeToUnlock)}; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	const listedHere = await waitFor(`document.querySelector('[data-testid="slice-unlock-${placeToUnlock}"]') ? true : null`, 3000);
	report(`chapter editor, slice 2 of chapter 1: "unlocks: ${placeToUnlock}" is set from the slice`, listedHere ? [] : ['not listed']);
	await shot('builder-5-slice-unlocks');
	await click('[data-testid="save-draft"]');
	let unlock = null;
	for (let i = 0; i < 20 && !unlock; i++) {
		const d = await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json();
		unlock = d.content.locations.find((p) => p.id === placeToUnlock)?.unlock?.slice ? d.content.locations.find((p) => p.id === placeToUnlock) : null;
		if (!unlock) await sleep(400);
	}
	report('saving the chapter page stores it in the world map (place: hidden, unlock { toon, chapter, slice image, reading point })',
		unlock && unlock.hidden === true && unlock.unlock.toon === 'catpope' && unlock.unlock.chapter === 1 && unlock.unlock.slice === target.image && unlock.unlock.at === point ? [] : [JSON.stringify(unlock?.unlock), `want slice ${target.image} at ${point}`]);
	await open(`${creator}/map?location=${placeToUnlock}`);
	const where = await waitFor(`(() => { const p = document.querySelector('[data-testid="place-unlock-where"]'); const a = document.querySelector('[data-testid="place-unlock-open"]'); return p && a ? { text: p.textContent, href: a.getAttribute('href'), slice: document.querySelector('[data-testid="place-unlock-slice"]').value } : null; })()`, 10000);
	report('the builder shows "Freigeschaltet in Kapitel 1, Slice 2" with a link to that slice, the slice picked',
		where && where.text.includes('1') && where.href === `/catpope/chapter/1?group=${encodeURIComponent(chapter.groups[target.group]?.label ?? String(target.group + 1))}&slice=${target.index + 1}` && where.slice === target.image ? [] : [JSON.stringify(where)]);
	await shot('builder-6-unlock-where');
}

try {
	if (!only || only === 'reader') {
		await reader(390, 'de');
		await reader(1440, 'en');
	}
	if (!only || only === 'member') await member();
	if (!only || only === 'builder') await builder();
	const errors = consoleErrors.filter((e) => e && !/favicon|ResizeObserver loop/.test(e));
	report('no console errors', errors.length ? errors.slice(0, 5) : []);
} catch (e) {
	console.error(e);
	failed++;
}
console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
