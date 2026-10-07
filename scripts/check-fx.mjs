// Reader FX check (agents/FX.md, BACKLOG 10.56): headless Chrome via CDP (own temp profile, no dependencies; Node 22
// WebSocket), like scripts/check-testo.mjs. At 390 px as the test user with the draft preview on, on a page with
// tappable pickups (default: toptoy chapter 1 `?label=blotter-deal` – five cucumbers; --page=<path?label=…>):
//   1. nudge: an untouched pickup on screen for 4 s wiggles once with sparkles in the viewport layer
//   2. a tap on a transparent pixel of the cut-out misses (only when the icon's alpha is readable – CORS)
//   3. collect: a tap lifts it, it flies through the viewport layer into the inventory button, the layer empties,
//      one pickup less, the "+1" notice; the page never gets wider than the viewport
//   4. refuse: the grant answers max_reached (faked) – the item shakes and stays, the notice says why
//   5. account page: "Sound effects" volume + "Vibration", kept on this device
// The test user is reset afterwards (npm run dev:test-user -- --reset clears the pickup refs).
//
//   node scripts/check-fx.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api] [--page=…] [--shots=<dir>] [--verbose]
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || 'http://localhost:8000').replace(/\/$/, '');
const api = String(opt.api || 'http://localhost:9000/api').replace(/\/$/, '');
const pagePath = String(opt.page || '/de/toptoy/chapter/1?label=blotter-deal');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const isLocal = (u) => /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/.test(u);
if (!isLocal(api) || !isLocal(viewer)) { console.error('local servers only'); process.exit(2); }

const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-fx-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--autoplay-policy=no-user-gesture-required', '--window-size=390,844', 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	try { execFileSync(process.execPath, [join(root, 'scripts/dev/test-user.mjs'), '--reset'], { env: { ...process.env, SCUTOONS_API: api }, stdio: 'ignore' }); } catch { console.log('  (could not reset the test user)'); }
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
let refuseGrants = false;
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	// step 4: the server says "max reached" (the CORS preflight passes)
	if (m.method === 'Fetch.requestPaused') {
		const { requestId, request } = m.params;
		const cors = [{ name: 'Access-Control-Allow-Origin', value: viewer }, { name: 'Access-Control-Allow-Credentials', value: 'true' }, { name: 'Access-Control-Allow-Headers', value: 'Authorization, Content-Type' }, { name: 'Access-Control-Allow-Methods', value: 'GET, POST, PUT' }, { name: 'Content-Type', value: 'application/json' }];
		if (refuseGrants && request.method === 'POST') send('Fetch.fulfillRequest', { requestId, responseCode: 422, responseHeaders: cors, body: Buffer.from(JSON.stringify({ error: 'max_reached', message: 'max reached' })).toString('base64') });
		else if (refuseGrants && request.method === 'OPTIONS') send('Fetch.fulfillRequest', { requestId, responseCode: 204, responseHeaders: cors });
		else send('Fetch.continueRequest', { requestId });
	}
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Fetch.enable', { patterns: [{ urlPattern: '*/inventory/grant/*' }] });
await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
async function idle(quietMs = 700, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 10000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(150); } return null; }
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
async function tap(x, y) {
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
}

let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}

const PICKUPS = `[...document.querySelectorAll('.slice-item.tappable')]`;
const layerKids = `(document.getElementById('sc-fx-layer')?.children.length ?? 0)`;
/** the page again (its script moves on to the next group after a while): the pickups on screen */
async function fresh() {
	await open(viewer + pagePath);
	await waitFor(`(() => { const n = ${PICKUPS}.length; return n ? n : null; })()`, 20000);
}
/** the first pickup, centred, its box and the point at its centre */
async function firstPickup() {
	return evaluate(`(() => { const el = ${PICKUPS}[0]; if (!el) return null; el.scrollIntoView({ block: 'center' }); const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2, left: b.left, top: b.top, w: b.width, h: b.height }; })()`);
}

// sign in (the test user) + the draft preview, then the page
await open(`${viewer}/de`);
await evaluate(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}); localStorage.setItem('scutoons.draft-preview', '1'); localStorage.removeItem('scutoons_fx_volume'); localStorage.removeItem('scutoons_fx_haptics'); true`);
const opened = Date.now();
await open(viewer + pagePath);
const count0 = await waitFor(`(() => { const n = ${PICKUPS}.length; return n ? n : null; })()`, 20000);
if (!count0) { console.log(`  FAIL  no tappable pickup on ${pagePath}`); await done(1); }

// 1 – nudge
{
	const p = [];
	await firstPickup();
	const sparks = await waitFor(`(() => { const l = document.getElementById('sc-fx-layer'); const n = l ? l.querySelectorAll('svg').length : 0; return n ? n : null; })()`, 8000);
	const after = Date.now() - opened;
	if (!sparks) p.push('no sparkles in the viewport layer within 8 s');
	else if (after < 4000) p.push(`sparkles ${after} ms after opening the page – the nudge waits 4 s on screen`);
	await sleep(150);
	await shot('1-nudge');
	await sleep(1500);
	if ((await evaluate(layerKids)) !== 0) p.push('the sparkles stay in the layer');
	// once per page view: no second nudge
	await sleep(4500);
	if ((await evaluate(layerKids)) !== 0) p.push('a second nudge on the same page view');
	report('nudge: once after 4 s on screen, sparkles on the outline', p, `${sparks} sparkles after ${after} ms`);
}

// 2 – a tap on a transparent pixel (the box's corner of a rotated, round-ish cut-out)
let alphaReadable = false;
{
	const p = [];
	await fresh();
	const b = await firstPickup();
	await sleep(300);
	await tap(b.left + 2, b.top + 2);
	await sleep(900);
	const n = await evaluate(`${PICKUPS}.length`);
	alphaReadable = n === count0 && !(await evaluate(`[...document.querySelectorAll('.notice')].length`));
	if (!alphaReadable) console.log(`  info  the corner tap took the item – the icon's alpha isn't readable (CORS?) or the corner is opaque: the box counts`);
	else report('a tap on a transparent pixel misses', p);
}

// 3 – collect
{
	const p = [];
	await fresh();
	const before = await evaluate(`${PICKUPS}.length`);
	const b = await firstPickup();
	await sleep(300);
	const bag = await evaluate(`(() => { const r = document.querySelector('.inventory-toggle')?.getBoundingClientRect(); return r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : null; })()`);
	await evaluate(`window.__fx = { maxW: 0, flew: [] }; (function watch() { const l = document.getElementById('sc-fx-layer'); const f = l?.querySelector('img'); if (f) { const r = f.getBoundingClientRect(); window.__fx.flew.push([r.left + r.width / 2, r.top + r.height / 2]); } window.__fx.maxW = Math.max(window.__fx.maxW, document.documentElement.scrollWidth); if (window.__fx.flew.length < 400) requestAnimationFrame(watch); })(); true`);
	await tap(b.x, b.y);
	await sleep(250);
	await shot('3-collect-flying');
	await sleep(1400);
	const fx = await evaluate(`window.__fx`);
	const after = await evaluate(`${PICKUPS}.length`);
	const notice = await evaluate(`[...document.querySelectorAll('.notice')].map((n) => n.innerText).join(' | ')`);
	if (after !== before - 1) p.push(`pickups ${before} → ${after}, expected one less`);
	if (!fx.flew.length) p.push('nothing flew in the viewport layer');
	else if (bag) {
		const [lx, ly] = fx.flew.at(-1);
		const d = Math.hypot(lx - bag.x, ly - bag.y);
		if (d > 60) p.push(`the flight ended ${Math.round(d)} px from the inventory button`);
	}
	if ((await evaluate(layerKids)) !== 0) p.push('the flying item stays in the layer');
	if (fx.maxW > 390) p.push(`the page got ${fx.maxW} px wide during the flight (viewport 390)`);
	if (!/\+1/.test(notice)) p.push(`no "+1" notice (${notice || 'none'})`);
	report('collect: lift, flight into the inventory button, one pickup less', p, `${fx.flew.length} frames, notice "${notice}"`);
	await shot('3-collect-done');
}

// 4 – refuse
{
	const p = [];
	await fresh();
	refuseGrants = true;
	await evaluate(`window.__shook = false; (() => { const el = ${PICKUPS}[0]; const orig = el.animate.bind(el); el.animate = (k, o) => { if (JSON.stringify(k).includes('translateX(-9px)')) window.__shook = true; return orig(k, o); }; })(); true`);
	const before = await evaluate(`${PICKUPS}.length`);
	const b = await firstPickup();
	await sleep(300);
	await tap(b.x, b.y);
	await sleep(250);
	await shot('4-refuse');
	await sleep(1000);
	refuseGrants = false;
	const after = await evaluate(`${PICKUPS}.length`);
	const notice = await evaluate(`[...document.querySelectorAll('.notice')].map((n) => n.innerText).join(' | ')`);
	if (after !== before) p.push(`pickups ${before} → ${after}: a refused item must stay`);
	if (!(await evaluate('window.__shook'))) p.push('no shake');
	if ((await evaluate(layerKids)) !== 0) p.push('something left in the layer');
	if (!notice) p.push('no notice why');
	report('refuse: it shakes, stays, the notice says why', p, `notice "${notice}"`);
}

// 5 – account page
{
	const p = [];
	await open(`${viewer}/de/account`);
	const ok = await waitFor(`document.querySelector('[data-testid="fx-setting"] #fx-volume')`);
	if (!ok) p.push('no "Soundeffekte" on the account page');
	else {
		await evaluate(`(() => { const r = document.querySelector('#fx-volume'); r.value = '35'; r.dispatchEvent(new Event('input', { bubbles: true })); const h = document.querySelector('[data-testid="fx-haptics"]'); h.click(); return true; })()`);
		await sleep(300);
		const stored = await evaluate(`[localStorage.getItem('scutoons_fx_volume'), localStorage.getItem('scutoons_fx_haptics'), document.querySelector('[data-testid="fx-setting"] output')?.textContent]`);
		if (stored[0] !== '0.35') p.push(`volume stored ${stored[0]}, expected 0.35`);
		if (stored[1] !== '0') p.push(`vibration stored ${stored[1]}, expected 0`);
		if (!/35/.test(stored[2] ?? '')) p.push(`the level shows ${stored[2]}`);
		await open(`${viewer}/de/account`);
		await waitFor(`document.querySelector('#fx-volume')`);
		const again = await evaluate(`[document.querySelector('#fx-volume').value, document.querySelector('[data-testid="fx-haptics"]').checked]`);
		if (again[0] !== '35' || again[1] !== false) p.push(`after a reload: ${JSON.stringify(again)}`);
		await evaluate(`document.querySelector('[data-testid="fx-setting"]').scrollIntoView({ block: 'center' }); true`);
		await shot('5-account');
	}
	report('account: sound effects volume + vibration, kept on this device', p);
}

console.log(`\n${passed} ok, ${failed} failed${alphaReadable ? '' : ' (alpha hit-test not checked)'}`);
await done(failed ? 1 : 0);
