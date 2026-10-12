// "Continue where I stopped" check (BACKLOG 10.69): headless Chrome via CDP (own temp profile, no dependencies; Node 22
// WebSocket), like scripts/check-next-bar.mjs. As the test user (`npm run dev:test-user`), per width (390, 1440):
//   1. a fresh save game; chapter 1 scrolled slice by slice to about 60 % of its first page → the member's save game
//      (progress row `scutoons`) has the chapter's checkpoint unfinished with the furthest spot read (group / slice –
//      the group's index in the chapter, the slice's in the group);
//   2. the toon page's start button says "continue" and links with `?resume`;
//   3. its click opens the chapter at that spot (the slice is on screen, the page is scrolled down) and `?resume` has left
//      the URL.
// The member's save game is restored afterwards.
//
// usage: node scripts/check-resume.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api] [--shots=<dir>]
//        [--only=390] [--verbose]
// Needs the dev servers (npm run dev) and the test user. Exit code 1 on any failure.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';
import { saveGameRow } from './dev/save-game-row.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const toon = 'catpope';
const chapter = 1;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test user ───────────────────────────────────────────────────────────────────────────────────────────────────
const sessionFile = join(root, 'logs/dev/test-user.json');
const session = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')) : null;
if (!session?.token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${session.token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const saveGame = saveGameRow(session.email);
// the chapter's groups: the first page (chain) and where each group's slices start on it
const groups = (await (await fetch(`${api}/library/toons/${toon}/chapters/${chapter}?locale=de`, { headers: { Authorization: `Bearer ${session.token}` } })).json()).groups ?? [];
const pageOf = (index) => {
	let start = index;
	while (start > 0 && groups[start - 1]?.continues === true) start--;
	let offset = 0;
	for (let i = start; i < index; i++) offset += groups[i]?.slices?.length ?? 0;
	return { start, offset };
};
let firstPage = 0;
for (let i = 0; i < groups.length; i++) { firstPage += groups[i]?.slices?.length ?? 0; if (groups[i]?.continues !== true) break; }
if (firstPage < 4) { console.error(`chapter ${chapter}'s first page has too few slices (${firstPage})`); process.exit(2); }

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-resume-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restored = false;
async function done(code) {
	if (!restored) { restored = true; if (!saveGame.restore()) console.log("  could not restore the member's save game"); }
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
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
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
/** a real click (a user gesture) in the middle of the element */
async function click(selector) {
	const box = await evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	if (!box || box.error) return false;
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: box.x, y: box.y, button: 'left', clickCount: 1 });
	return true;
}

// ── checks ──────────────────────────────────────────────────────────────────────────────────────────────────────────
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
const uri = `${toon}/chapter/${chapter}`;
const checkpoint = () => (saveGame.state()?.progress ?? []).find((c) => c.uri === uri) ?? null;

for (const width of [390, 1440]) {
	if (opt.only && String(opt.only) !== String(width)) continue;
	console.log(`${width}`);
	await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
	saveGame.clear();
	await open(`${viewer}/`);
	await evaluate(`(localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(session.token)}), true)`);

	// 1. read to about 60 % of the first page, slice by slice
	await open(`${viewer}/${toon}/chapter/${chapter}`);
	const target = Math.floor(firstPage * 0.6);
	const ready = await waitFor(`document.querySelector('.chapter-view [data-slice-index="${target}"]') ? true : null`, 30000);
	if (!ready) { report('1. the chapter page renders its slices', ['no slices']); continue; }
	await evaluate(`(async () => { for (let i = 0; i <= ${target}; i++) { const el = document.querySelector('.chapter-view [data-slice-index="' + i + '"]'); if (!el) break; el.scrollIntoView({ block: 'end' }); await new Promise((r) => setTimeout(r, 120)); } return true; })()`);
	// the save game reaches the server after its debounce (world.svelte.ts SYNC_DEBOUNCE_MS)
	let cp = null;
	for (let i = 0; i < 40 && !Number.isInteger((cp = checkpoint())?.group); i++) await sleep(250);
	await sleep(2500);
	cp = checkpoint();
	const p1 = [];
	if (!cp) p1.push(`no checkpoint ${uri} in the save game`);
	else {
		if (cp.isComplete || !(cp.position > 0 && cp.position < 1)) p1.push(`expected an unfinished checkpoint, got ${JSON.stringify(cp)}`);
		if (!Number.isInteger(cp.group) || !Number.isInteger(cp.slice)) p1.push(`no spot: ${JSON.stringify(cp)}`);
		else {
			const index = pageOf(cp.group).offset + cp.slice;
			if (pageOf(cp.group).start !== 0) p1.push(`the spot's group ${cp.group} is not on the first page`);
			if (index < target) p1.push(`the spot (page slice ${index}) is before the slice scrolled to (${target})`);
		}
	}
	report('1. the furthest spot is in the member\'s save game', p1, cp ? `position ${cp.position}, group ${cp.group}, slice ${cp.slice}` : '');
	if (!cp || !Number.isInteger(cp.group)) continue;
	const spotIndex = pageOf(cp.group).offset + cp.slice;

	// 2. the start button continues with ?resume
	await open(`${viewer}/${toon}`);
	const startSel = width >= 1024 ? '[data-testid="hero-start"]' : '[data-testid="start-button"]';
	const start = await waitFor(`(() => { const a = document.querySelector('${startSel}'); return a ? { state: a.dataset.state, href: a.getAttribute('href') } : null; })()`, 20000);
	const p2 = [];
	if (!start) p2.push('no start button');
	else {
		if (start.state !== 'continue') p2.push(`state ${start.state}, expected continue`);
		if (!/\/chapter\/1\?resume$/.test(start.href ?? '')) p2.push(`href ${start.href}, expected …/chapter/1?resume`);
	}
	report('2. the start button says "continue" and links with ?resume', p2, start ? `${start.state} ${start.href}` : '');

	// 3. its click opens the chapter at the spot
	await click(startSel);
	const arrived = await waitFor(`(() => {
		if (!location.pathname.endsWith('/chapter/${chapter}')) return null;
		const el = document.querySelector('.chapter-view [data-slice-index="${spotIndex}"]');
		if (!el || scrollY < 50) return null;
		const b = el.getBoundingClientRect();
		return b.top < innerHeight && b.bottom > 0 ? { top: Math.round(b.top), y: Math.round(scrollY), search: location.search } : null;
	})()`, 20000);
	await sleep(2200); // the arrival settles (SETTLE_MS)
	const after = await evaluate(`(() => { const el = document.querySelector('.chapter-view [data-slice-index="${spotIndex}"]'); const b = el?.getBoundingClientRect(); return { top: b ? Math.round(b.top) : null, bottom: b ? Math.round(b.bottom) : null, y: Math.round(scrollY), h: innerHeight, search: location.search }; })()`);
	await shot(`resume-${width}`);
	const p3 = [];
	if (!arrived) p3.push(`the slice ${spotIndex} never came on screen: ${JSON.stringify(after)}`);
	else if (!(after.top < after.h && after.bottom > 0)) p3.push(`the slice ${spotIndex} left the screen after settling: ${JSON.stringify(after)}`);
	if (/resume/.test(after.search ?? '')) p3.push(`?resume is still in the URL (${after.search})`);
	report('3. "continue" opens the chapter at that spot', p3, JSON.stringify(after));
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
