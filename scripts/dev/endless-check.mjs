#!/usr/bin/env node
// The endless band (BACKLOG 10.100, agents/research/TEXT-ACROSS-SLICES.md §8; viewer renderer/strip-endless.svelte,
// strip-loop.ts, strip-drag.ts) in headless Chrome via CDP (own temp profile, no dependencies; Node 22 WebSocket) as the
// local test user with the draft preview on. The API's chapter answer is rewritten (CDP Fetch) so one horizontal group is
// endless (default catpope chapter 2 group 0, 7 slices). At 390 × 844 (touch) and 1440 × 900, then 1440 × 900 at device
// pixel ratio 1.5 – the plan's prototype checks where they apply:
//   1. the band: the whole width, min(100svh, 100lvh − min(4rem, 10lvh)) high, a grab cursor, touch-action "pan-y pinch-zoom"; every
//      data-slice-index once on the page (no clones), the group's slices all in the band
//   2. frames come in decoded: a frame never shown (the farthest in the track first), brought in by one wheel step, is no
//      ink on its first paint
//   3. a mouse drag (and a touch drag at 390) does nothing in the 0.5rem dead zone, then moves every frame 1:1 (grabbing
//      cursor); resting before letting go: no glide
//   4. a fling (mouse; touch at 390) glides, stops within 4 s and never steps backwards – every rAF recorded, the frames
//      side by side in each one
//   5. a full turn each way in wheel steps: every frame moves exactly by each step, the frames on screen stay side by side
//      in order across the wraps, the same offset mod T looks the same (screenshots compared, animations paused)
//   6. a vertical touch swipe (390) scrolls the page, the band stays; wheel deltaY scrolls the page, deltaX moves the band
//   7. ← → with focus on the band ease to the next / previous frame edge
//   8. html[data-vn-blocking]: drag, touch, wheel and keys do nothing
//   9. reading progress: with the band's top 12 px above the screen a 3 px sliver of the next frame is not read, more than
//      half of it is (the member's save game emptied for this; it is put back exactly at the end)
//  10. an un-rewritten horizontal group on the same page (default the first other one) still pins and moves sideways
//      with the page scroll
//  11. the band never reaches past the page's sides, so the page never gets wider by it (every state read, every recorded
//      rAF); anything else past them is named in a note (the pinned strip's 100vw counts a classic scrollbar); no page errors
//  12. at DPR 1.5: no ink hairline between frames (the band red, the art grey: no red at any frame edge, wrap seam too)
//  13. the drag hint (strip-nudge.ts, owner 2026-10-16 Q10), "Show hints" set by rewriting the API's /me/ answer (off for
//      1–12): with hints off nothing plays; on, the hint line sweeps once across frame 0 (its part on screen) and the
//      screen bounces once (zoomed for the band's width), and never again after scrolling away and back; under html[data-vn-blocking] nothing (it plays once the story
//      lets go); a touch (390) / mouse press (1440) on the band mid-sweep cancels it (line gone, no bounce, no replay);
//      prefers-reduced-motion: the line stands still (fades) and there is no bounce
//   node scripts/dev/endless-check.mjs [--page=/de/catpope/chapter/2] [--group=0] [--pinned=<group index>]
//        [--only=390|1440|1.5] [--shots=<dir>]       (viewer :8000, API :9000 – local only; npm run dev:test-user)
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { local } from './ports.mjs';
import { saveGameRow } from './save-game-row.mjs';
import { ENDLESS_PAGE, NUDGE_PAGE, differ, fmt, gaps, moves, norm, off, png } from './endless-page.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const VIEWER = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const API = String(opt.api || local('api', '/api')).replace(/\/$/, '');
if (![VIEWER, API].every((u) => /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/.test(u))) { console.error('endless-check: local servers only'); process.exit(2); }
const PAGE = String(opt.page || '/de/catpope/chapter/2');
const [, TOON, CHAPTER] = PAGE.match(/\/([^/?]+)\/chapter\/(\d+)/) ?? [];
if (!TOON) { console.error(`--page=${PAGE}: no /<toon>/chapter/<n> in it`); process.exit(2); }
const GROUP = Number(opt.group ?? 0);
const SHOTS = opt.shots ? String(opt.shots) : null;
if (SHOTS) mkdirSync(SHOTS, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test user, the chapter's groups (the draft preview's answer) ──
const sessionFile = join(ROOT, 'logs/dev/test-user.json');
const session = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')) : null;
const auth = { headers: { Authorization: `Bearer ${session?.token}` } };
if (!session?.token || !(await fetch(`${API}/me/`, auth)).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const answer = await (await fetch(`${API}/library/toons/${TOON}/chapters/${CHAPTER}?draft=1&locale=de`, auth)).json();
const groups = answer.groups ?? [];
const slices = (g) => groups[g]?.slices?.length ?? 0;
const horizontal = (g) => groups[g]?.renderer?.type === 'horizontal' && slices(g) >= 2;
if (!horizontal(GROUP)) { console.error(`${TOON} ch${CHAPTER} group ${GROUP} is no horizontal group with ≥ 2 slices`); process.exit(2); }
// the page = the group's chain (linked groups, `continues`); data-slice-index counts from its first group
let start = GROUP;
while (start > 0 && groups[start - 1]?.continues === true) start--;
let end = GROUP;
while (end < groups.length - 1 && groups[end]?.continues === true) end++;
const pageOffset = (g) => groups.slice(start, g).reduce((sum, _, i) => sum + slices(start + i), 0);
const N = slices(GROUP);
const OFFSET = pageOffset(GROUP);
const PINNED = opt.pinned !== undefined ? Number(opt.pinned) : [...Array(end - start + 1).keys()].map((i) => start + i).find((g) => g !== GROUP && horizontal(g) && !groups[g].renderer.params?.endless);
const saveGame = saveGameRow(session.email);
const SAVE_KEY = 'scutoons_game_scutoons'; // SAVE_GAME_KEY, shared/content save-game.ts

// ── Chrome ──
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9600 + Math.floor(Math.random() * 150);
const profile = tempProfile('endless-check-'); // removed on exit (chrome-profile.mjs)
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try { wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl; } catch {}
	if (!wsUrl) await sleep(200);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
let rewritten = 0;
/** the member's "Show hints" in the API's /me/ answer (the test user's own setting would decide otherwise) */
let HINTS = false;
const pending = new Map();
const errors = [];
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) return pending.get(m.id)(m), pending.delete(m.id);
	if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
	if (m.method === 'Fetch.requestPaused') (/\/me\/(\?|$)/.test(m.params.request.url) ? hints : endless)(m.params).catch(() => send('Fetch.continueResponse', { requestId: m.params.requestId }));
});
/** the chapter's answer with the group endless – the format after the 10.100 migration */
async function endless({ requestId, responseStatusCode, responseHeaders }) {
	if (responseStatusCode !== 200) return send('Fetch.continueResponse', { requestId });
	const r = await send('Fetch.getResponseBody', { requestId });
	let body = r.result.base64Encoded ? Buffer.from(r.result.body, 'base64').toString('utf8') : r.result.body;
	try {
		const json = JSON.parse(body);
		const group = (json.groups ?? json.chapter?.groups)?.[GROUP];
		if (group) (group.renderer = { type: 'horizontal', params: { endless: true } }), (body = JSON.stringify(json)), rewritten++;
	} catch {}
	send('Fetch.fulfillRequest', { requestId, responseCode: 200, responseHeaders: responseHeaders.filter((h) => !/^content-(length|encoding)$/i.test(h.name)), body: Buffer.from(body).toString('base64') });
}
/** the /me/ answer with "Show hints" = HINTS */
async function hints({ requestId, responseStatusCode, responseHeaders }) {
	if (responseStatusCode !== 200) return send('Fetch.continueResponse', { requestId });
	const r = await send('Fetch.getResponseBody', { requestId });
	let body = r.result.base64Encoded ? Buffer.from(r.result.body, 'base64').toString('utf8') : r.result.body;
	try {
		const json = JSON.parse(body);
		if (json.user) (json.user.show_hints = HINTS), (body = JSON.stringify(json));
	} catch {}
	send('Fetch.fulfillRequest', { requestId, responseCode: 200, responseHeaders: responseHeaders.filter((h) => !/^content-(length|encoding)$/i.test(h.name)), body: Buffer.from(body).toString('base64') });
}
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
	return r.result?.result?.value;
}
async function until(expression, ms = 20000) {
	const stop = Date.now() + ms;
	while (Date.now() < stop) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(150);
	}
	return null;
}

// ── the band, read in the page; the drag hint recorded from the document's start (endless-page.mjs) ──
await send('Page.addScriptToEvaluateOnNewDocument', { source: ENDLESS_PAGE });
await send('Page.addScriptToEvaluateOnNewDocument', { source: NUDGE_PAGE });
/** how far the page got wider than the viewport, and the band past the page's sides, px – over every state read */
let wide = { page: 0, band: 0 };
const widen = (s) => (wide = { page: Math.max(wide.page, s.sw - s.cw), band: Math.max(wide.band, -s.band.x, s.band.x + s.band.w - s.cw) });
async function state() {
	const s = await js('window.__endless?.state()');
	if (s) widen(s);
	return s;
}

// ── input ──
const point = async (fx = 0.5, fy = 0.5) => {
	const at = await js(`__endless.point(${fx}, ${fy})`);
	if (!at) throw new Error('something covers the whole band');
	return at;
};
const mouse = (type, { x, y }, held = type !== 'mouseReleased') => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: held ? 1 : 0, clickCount: 1 });
const touch = (type, at) => send('Input.dispatchTouchEvent', { type, touchPoints: at ? [{ x: at.x, y: at.y, id: 1 }] : [] });
const pointer = (kind) => ({
	down: (at) => (kind === 'touch' ? touch('touchStart', at) : mouse('mousePressed', at)),
	move: (at) => (kind === 'touch' ? touch('touchMove', at) : mouse('mouseMoved', at)),
	up: (at) => (kind === 'touch' ? touch('touchEnd') : mouse('mouseReleased', at))
});
const wheel = (at, deltaX, deltaY = 0) => send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: at.x, y: at.y, deltaX, deltaY });
const key = async (k) => {
	const code = { ArrowLeft: 37, ArrowRight: 39 }[k];
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: k, code: k, windowsVirtualKeyCode: code });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code: k, windowsVirtualKeyCode: code });
};
/** the band in the middle of the screen (its slack shared above and below) */
const toBand = async () => (await js(`(() => { const b = __endless.band().getBoundingClientRect(); scrollTo({ top: scrollY + b.top - (innerHeight - b.height) / 2, behavior: 'instant' }); return true; })()`), await sleep(150), { ...(await state()), vh: await js('innerHeight') });
/** waits until nothing moves (two reads 150 ms apart) */
async function settle(ms = 5000) {
	let a = await state();
	for (const stop = Date.now() + ms; Date.now() < stop; ) {
		await sleep(150);
		const b = await state();
		if (b.x === a.x && b.frames.every((f, i) => f.l === a.frames[i].l)) return b;
		a = b;
	}
	return a;
}

// ── pixels (png: endless-page.mjs) ──
async function shot(clip, name) {
	const r = await send('Page.captureScreenshot', { format: 'png', ...(clip ? { clip: { ...clip, scale: 1 } } : {}) });
	const buf = Buffer.from(r.result.data, 'base64');
	if (SHOTS && name) writeFileSync(join(SHOTS, `${name}.png`), buf);
	return png(buf);
}
/** share of ink-coloured pixels in a rect (device px) */
function inkShare(img, r, ink) {
	let n = 0, hit = 0;
	for (let y = Math.max(0, Math.round(r.y)); y < Math.min(img.h, Math.round(r.y + r.h)); y += 2) for (let x = Math.max(0, Math.round(r.x)); x < Math.min(img.w, Math.round(r.x + r.w)); x += 2) {
		const p = img.at(x, y);
		n++;
		if (Math.abs(p[0] - ink[0]) + Math.abs(p[1] - ink[1]) + Math.abs(p[2] - ink[2]) < 24) hit++;
	}
	return n ? hit / n : 0;
}
const bandClip = (s) => { const y = Math.max(0, s.band.y), b = Math.min(s.vh, s.band.y + s.band.h); return { x: Math.max(0, s.win.x), y, width: Math.min(s.cw, s.win.x + s.win.w) - Math.max(0, s.win.x), height: b - y }; };

// ── checks ──
const results = [];
const check = (name, problems) => { results.push(!problems.length); console.log(`${problems.length ? 'FAIL' : 'ok  '} ${name}${problems.length ? `  — ${problems.join('; ')}` : ''}`); };
const tolOf = (dpr) => 1 / dpr + 0.02;

async function openPage(name, navigate = true) {
	if (navigate) await send('Page.navigate', { url: VIEWER + PAGE });
	const ready = await until(`(() => { const s = window.__endless?.state(); const imgs = [...(__endless.band()?.querySelectorAll('img') ?? [])]; return s && s.T > 0 && imgs.length >= ${N} && imgs.every((i) => i.complete && i.naturalWidth > 0) ? true : null; })()`, 40000);
	await sleep(700); // the band decodes its frames once they loaded
	if (!ready) check(`${name}: the endless band renders (the API's answer rewritten ${rewritten}×)`, ['no band with loaded frames within 40 s']);
	return ready;
}

async function render(name, dpr) {
	const s = await toBand();
	const p = [];
	const probe = await js(`(() => { const d = document.createElement('div'); d.style.cssText = 'position:absolute;height:calc(100vh - min(4rem, 10vh));height:min(100svh, calc(100lvh - min(4rem, 10lvh)))'; document.body.append(d); const h = d.getBoundingClientRect().height; d.remove(); const band = __endless.band(); const cs = getComputedStyle(band); const all = [...document.querySelectorAll('.chapter-view [data-slice-index]')].map((e) => Number(e.dataset.sliceIndex)); const own = [...band.querySelectorAll('[data-slice-index]')].map((e) => Number(e.dataset.sliceIndex)); return { h, cursor: cs.cursor, touch: cs.touchAction, all, own }; })()`);
	if (Math.abs(s.band.x) > 0.5 || Math.abs(s.band.w - s.cw) > 0.5) p.push(`band ${s.band.x.toFixed(1)} + ${s.band.w.toFixed(1)}, the page ${s.cw} wide`);
	if (Math.abs(s.band.h - probe.h) > 0.5) p.push(`band ${s.band.h.toFixed(1)} high, expected ${probe.h.toFixed(1)}`);
	if (probe.cursor !== 'grab') p.push(`cursor ${probe.cursor}`);
	if (probe.touch !== 'pan-y pinch-zoom') p.push(`touch-action ${probe.touch}`);
	if (new Set(probe.all).size !== probe.all.length) p.push(`a data-slice-index twice on the page (${probe.all.length} slices, ${new Set(probe.all).size} indices)`);
	const expect = [...Array(N).keys()].map((i) => OFFSET + i);
	if (probe.own.join() !== expect.join()) p.push(`band slices ${probe.own.join(',')}, expected ${expect.join(',')}`);
	if (s.frames.length !== N) p.push(`${s.frames.length} frames`);
	const g = gaps(s);
	if (g) p.push(g);
	check(`${name}: the band ${s.band.w.toFixed(0)} × ${s.band.h.toFixed(0)}, a grab cursor, pan-y pinch-zoom, ${N} frames, every slice once (dpr ${dpr})`, p);
	await shot(null, SHOTS && `endless-${name}`);
	return s;
}

/**
 * 2 – a frame never shown, brought in by one wheel step: no ink on its first paint (decode() once the band nears) – the
 * farthest one in the track first, beyond what the browser rasterised ahead (the prototype's black frame was 3 away)
 */
async function decoded(name, s0) {
	const p = [];
	const ink = (await js(`getComputedStyle(__endless.band()).backgroundColor`)).match(/\d+/g).slice(0, 3).map(Number);
	const unseen = new Set(s0.frames.map((f, i) => (f.l >= s0.win.w || f.l + f.w <= 0 ? i : -1)).filter((i) => i >= 0));
	let s = s0;
	const tested = [];
	for (let jump = 0; jump < 2; jump++) {
		const next = s.frames.map((f, i) => ({ ...f, i })).filter((f) => f.l >= s.win.w - 0.5 && unseen.has(f.i)).sort((a, b) => b.l - a.l)[0];
		if (!next) break;
		await wheel(await point(), next.l);
		const after = await state();
		const now = after.frames.map((f, i) => ({ ...f, i })).filter((f) => unseen.has(f.i) && Math.min(f.l + f.w, after.win.w) - Math.max(f.l, 0) >= Math.min(f.w, after.win.w) * 0.5);
		const clip = bandClip({ ...after, vh: s0.vh });
		const first = await shot(clip);
		await sleep(400);
		const later = await shot(clip);
		for (const f of now) {
			const r = { x: (Math.max(f.l, 0) + after.win.x - clip.x) * s0.dpr, y: 0, w: (Math.min(f.l + f.w, after.win.w) - Math.max(f.l, 0)) * s0.dpr, h: clip.height * s0.dpr };
			const a = inkShare(first, r, ink), b = inkShare(later, r, ink);
			tested.push(`${f.i}: ${a.toFixed(3)}/${b.toFixed(3)}`);
			if (a > b + 0.02) p.push(`frame ${f.i} ink on its first paint (${(a * 100).toFixed(1)} % ink, ${(b * 100).toFixed(1)} % 400 ms later)`);
			unseen.delete(f.i);
		}
		s = after;
	}
	if (!tested.length) p.push('no frame was off screen at the start');
	check(`${name}: frames never shown come in painted (ink share first / later: ${tested.join(', ')})`, p);
}

/** 3 – dead zone, then 1:1; resting before letting go: no glide */
async function drag(name, kind, dpr) {
	const s0 = await toBand();
	const p = [];
	const rem = await js(`parseFloat(getComputedStyle(document.documentElement).fontSize)`);
	const go = pointer(kind);
	const at = await point(0.6);
	await go.down(at);
	await go.move({ x: at.x - rem * 0.25, y: at.y });
	await sleep(30);
	const s1 = await state();
	for (let i = 1; i <= 10; i++) await go.move({ x: at.x - 10 * i, y: at.y }), await sleep(16);
	const s2 = await state();
	const cursor = await js(`getComputedStyle(__endless.band()).cursor`);
	await sleep(120);
	await go.up({ x: at.x - 100, y: at.y });
	await sleep(500);
	const s3 = await state();
	if (off(moves(s0, s1), 0, 0.01)) p.push(`moved in the dead zone: ${fmt(moves(s0, s1))}`);
	if (off(moves(s0, s2), -100, tolOf(dpr))) p.push(`a 100 px drag moved the frames ${fmt(moves(s0, s2))}`);
	if (off(moves(s2, s3), 0, 0.01)) p.push(`glided after resting: ${fmt(moves(s2, s3))}`);
	if (kind === 'mouse' && cursor !== 'grabbing') p.push(`cursor ${cursor} while dragging`);
	for (const s of [s1, s2, s3]) if (gaps(s)) p.push(gaps(s));
	check(`${name}: a ${kind} drag waits out the 0.5rem dead zone, then moves 1:1 (${moves(s0, s2)[0].toFixed(2)} for −100), no glide after resting`, p);
}

/** 4 – a fling glides, stops, never steps backwards */
async function fling(name, kind) {
	const s0 = await toBand();
	const p = [];
	const go = pointer(kind);
	const at = await point(0.8);
	await js('__endless.record()');
	await go.down(at);
	for (let i = 1; i <= 6; i++) await go.move({ x: at.x - 50 * i, y: at.y }), await sleep(8);
	await go.up({ x: at.x - 300, y: at.y });
	await until('window.__rec?.done', 10000);
	const rec = await js('window.__rec');
	const st = rec.states;
	const r = Math.max(1, rec.release);
	let glide = 0, back = 0, last = r;
	for (let k = r; k < st.length; k++) {
		const d = norm(st[k].frames[0].l - st[k - 1].frames[0].l, st[k].T);
		glide += d;
		if (d > 0.01) back++;
		if (d !== 0) last = k;
	}
	const ms = st[last].t - st[r - 1].t;
	if (rec.release < 0) p.push('no pointerup seen');
	if (glide > -100) p.push(`glided only ${glide.toFixed(1)} px`);
	if (back) p.push(`${back} step(s) backwards`);
	if (ms > 4000 || !rec.done) p.push(`still moving after ${Math.round(ms)} ms`);
	const apart = st.map((s) => gaps(s)).find(Boolean);
	if (apart) p.push(`during the glide: ${apart}`);
	st.forEach(widen);
	check(`${name}: a ${kind} fling glides ${(-glide).toFixed(0)} px, stops after ${Math.round(ms)} ms, never backwards (${st.length - r} frames)`, p);
}

/** 5 – a full turn each way in wheel steps: 1:1 every step, side by side across the wraps, the same picture mod T */
async function turns(name, dpr) {
	let s = await toBand();
	const vh = s.vh;
	const at = await point();
	const p = [];
	// every frame shown once first (their entrances), then everything that animates paused
	for (let k = 0; k < Math.ceil(s.T / 300); k++) await wheel(at, 300), await sleep(40);
	await sleep(1500);
	await js(`(() => { const st = document.createElement('style'); st.id = 'endless-check-still'; st.textContent = '*, *::before, *::after { animation-play-state: paused !important; transition: none !important; caret-color: transparent !important; }'; document.head.append(st); document.getAnimations().forEach((a) => a.pause()); document.querySelectorAll('video').forEach((v) => v.pause()); return __endless.bare(true); })()`);
	await sleep(200);
	s = await state();
	const s0 = s;
	const clip = bandClip({ ...s0, vh });
	const a = await shot(clip, SHOTS && `endless-${name}-turn-a`);
	const pictures = [];
	let steps = 0;
	for (const [step, turnsBy] of [[61, 1], [-67, 2]]) {
		const total = turnsBy * s0.T;
		let done = 0;
		while (Math.abs(done) < total - 0.001) {
			const d = Math.sign(step) * Math.min(Math.abs(step), total - Math.abs(done));
			const before = s;
			await wheel(at, d);
			s = await state();
			steps++;
			done += Math.sign(step) * Math.abs(d);
			const m = moves(before, s);
			if (off(m, -d, tolOf(dpr))) p.push(`step ${steps} (${d.toFixed(2)}): ${fmt(m)}`);
			const g = gaps(s);
			if (g) p.push(`step ${steps}: ${g}`);
			if (p.length > 4) break;
		}
		const drift = norm(s.frames[0].l - s0.frames[0].l, s0.T);
		if (Math.abs(drift) > tolOf(dpr)) p.push(`after ${turnsBy} turn(s) ${step > 0 ? 'left' : 'right'} frame 0 is ${drift.toFixed(2)} px off`);
		await sleep(250);
		const b = await shot(clip, SHOTS && `endless-${name}-turn-${step > 0 ? 'left' : 'right'}`);
		const { share, box } = differ(a, b);
		pictures.push((share * 100).toFixed(3));
		if (share > 0.0005) p.push(`the picture after the turn ${step > 0 ? 'left' : 'right'} differs in ${(share * 100).toFixed(2)} % of its pixels (${box})`);
	}
	await js(`(document.getElementById('endless-check-still')?.remove(), document.getAnimations().forEach((a) => a.play()), __endless.bare(false))`);
	check(`${name}: a full turn each way (${steps} wheel steps): 1:1, side by side across the wraps, the same picture mod T (${pictures.join(' / ')} % differ)`, p.slice(0, 5));
}

/** 6 – the page keeps its vertical gestures */
async function pageGestures(name, touchToo, dpr) {
	const p = [];
	let s0 = await toBand();
	await wheel(await point(), 0, 300);
	await sleep(700);
	let s1 = await state();
	if (s1.y - s0.y < 100) p.push(`wheel deltaY 300 scrolled the page ${Math.round(s1.y - s0.y)} px`);
	if (off(moves(s0, s1), 0, 0.01)) p.push(`wheel deltaY moved the band ${fmt(moves(s0, s1))}`);
	s0 = await toBand();
	await sleep(250);
	await wheel(await point(), 250);
	await sleep(300);
	s1 = await state();
	if (off(moves(s0, s1), -250, tolOf(dpr))) p.push(`wheel deltaX 250 moved the band ${fmt(moves(s0, s1))}`);
	if (Math.abs(s1.y - s0.y) > 0.5) p.push(`wheel deltaX scrolled the page ${Math.round(s1.y - s0.y)} px`);
	if (touchToo) {
		s0 = await toBand();
		const at = await point(0.5, 0.7);
		await touch('touchStart', at);
		for (let i = 1; i <= 15; i++) await touch('touchMove', { x: at.x, y: at.y - (200 * i) / 15 }), await sleep(16);
		await touch('touchEnd');
		await sleep(900);
		s1 = await state();
		if (s1.y - s0.y < 100) p.push(`a vertical swipe scrolled the page ${Math.round(s1.y - s0.y)} px`);
		if (off(moves(s0, s1), 0, 0.01)) p.push(`a vertical swipe moved the band ${fmt(moves(s0, s1))}`);
	}
	check(`${name}: ${touchToo ? 'a vertical swipe and ' : ''}wheel deltaY scroll the page, the band stays; deltaX moves the band 1:1`, p);
}

/** 7 – ← → with focus ease to the next / previous frame edge */
async function keys(name, dpr) {
	const p = [];
	const s = await toBand();
	const near = Math.min(...s.frames.map((f) => f.l).filter((l) => l > 0.5));
	if (near < 100) await wheel(await point(), -150), await sleep(200);
	await js(`(__endless.band().focus({ preventScroll: true }), true)`);
	for (const k of ['ArrowRight', 'ArrowRight', 'ArrowLeft']) {
		const s0 = await state();
		const fs = s0.frames.map((f, i) => ({ ...f, i }));
		const want = k === 'ArrowRight' ? fs.filter((f) => f.l > 0.5).sort((a, b) => a.l - b.l)[0] : fs.filter((f) => f.l < -0.5).sort((a, b) => b.l - a.l)[0];
		await key(k);
		await sleep(70);
		const mid = await state();
		const s1 = await settle();
		const l = s1.frames[want.i].l;
		const m = mid.frames[want.i].l;
		if (Math.abs(l) > 0.5 / dpr + 0.02) p.push(`${k}: frame ${want.i} ends at ${l.toFixed(2)}, not at the window's left`);
		if (!(Math.abs(m) < Math.abs(want.l) - 1 && Math.abs(m) > 0.5)) p.push(`${k}: no ease (frame ${want.i} at ${want.l.toFixed(1)} → ${m.toFixed(1)} after 70 ms → ${l.toFixed(1)})`);
	}
	check(`${name}: ← → with focus ease to the next / previous frame edge`, p);
}

/** 8 – html[data-vn-blocking]: nothing moves the band */
async function blocked(name, touchToo) {
	const s0 = await toBand();
	await js(`(document.documentElement.dataset.vnBlocking = '', __endless.band().focus({ preventScroll: true }), true)`);
	const at = await point(0.6);
	await key('ArrowRight');
	await sleep(400);
	await wheel(at, 200);
	await sleep(200);
	for (const kind of touchToo ? ['mouse', 'touch'] : ['mouse']) {
		const go = pointer(kind);
		await go.down(at);
		for (let i = 1; i <= 6; i++) await go.move({ x: at.x - 20 * i, y: at.y }), await sleep(16);
		await go.up({ x: at.x - 120, y: at.y });
	}
	await sleep(500);
	const s1 = await state();
	await js(`(delete document.documentElement.dataset.vnBlocking, true)`);
	check(`${name}: html[data-vn-blocking] – keys, wheel, ${touchToo ? 'mouse and touch drags' : 'a mouse drag'} leave the band`, off(moves(s0, s1), 0, 0.01) ? [`moved ${fmt(moves(s0, s1))}`] : []);
}

/** 9 – reading progress: a 3 px sliver of a frame is not read while the band's top is above the screen */
async function sliver(name, dpr) {
	const p = [];
	const spot = async () => {
		const g = await js(`(() => { try { return JSON.parse(localStorage.getItem('${SAVE_KEY}')); } catch { return null; } })()`);
		const c = (g?.progress ?? []).find((x) => x.uri === `${TOON}/chapter/${CHAPTER}`);
		return Number.isInteger(c?.group) ? pageOffset(c.group) + c.slice : -1;
	};
	await toBand();
	await js(`(() => { const b = __endless.band().getBoundingClientRect(); scrollTo({ top: scrollY + b.top + 12, behavior: 'instant' }); return true; })()`);
	await sleep(400);
	const s0 = await state();
	const read0 = await spot();
	const next = s0.frames.map((f, i) => ({ ...f, i })).filter((f) => f.l >= s0.win.w - 0.5).sort((a, b) => a.l - b.l)[0];
	const index = OFFSET + next.i;
	const at = await point();
	await wheel(at, next.l - (s0.win.w - 3));
	await sleep(600);
	const s1 = await state();
	const read1 = await spot();
	await wheel(at, next.w * 0.6);
	await sleep(600);
	const read2 = await spot();
	const shown = s1.win.w - s1.frames[next.i].l;
	if (s0.band.y > -11 || s0.band.y < -13) p.push(`the band's top at ${s0.band.y.toFixed(1)}`);
	if (Math.abs(shown - 3) > tolOf(dpr)) p.push(`the sliver is ${shown.toFixed(2)} px`);
	if (read0 >= index) p.push(`slice ${index} read before it showed (${read0})`);
	if (read1 >= index) p.push(`the 3 px sliver of slice ${index} counted as read (the spot: slice ${read1})`);
	if (read2 !== index) p.push(`more than half of slice ${index} shown: the spot is slice ${read2}`);
	check(`${name}: reading progress – a 3 px sliver of frame ${next.i} with the band's top 12 px above the screen not read (spot ${read0} → ${read1}), half of it is (→ ${read2})`, p);
}

/** 10 – the pinned strip of an un-rewritten horizontal group pins and moves with the page scroll */
async function pinned(name) {
	if (PINNED === undefined) return check(`${name}: an un-rewritten horizontal group on the page`, ['none on this page (--pinned=<group index>)']);
	const label = groups[PINNED].label;
	const at = (top) => js(`(async () => { const s = document.querySelector('.chapter-view .strip[data-group-label=${JSON.stringify(label ?? '')}]'); if (!s) return null; scrollTo({ top: scrollY + s.getBoundingClientRect().top + ${top} * innerHeight, behavior: 'instant' }); await new Promise((r) => setTimeout(r, 300)); const pin = s.querySelector('.pin'), track = pin.querySelector('.track'); return { section: s.getBoundingClientRect().top, pin: pin.getBoundingClientRect().top, x: new DOMMatrix(getComputedStyle(track).transform).m41, range: s.offsetHeight - pin.offsetHeight }; })()`);
	const a = await at(0.5);
	const b = a && (await at(1));
	const p = [];
	if (!a) p.push(`no pinned strip for group ${PINNED} (${label})`);
	else {
		if (a.range < 100) p.push(`no pinned range (${a.range} px)`);
		if (Math.abs(a.pin) > 0.5 || Math.abs(b.pin) > 0.5) p.push(`the pin's top ${a.pin.toFixed(1)} / ${b.pin.toFixed(1)}, not 0`);
		if (!(b.x < a.x - 50)) p.push(`the track ${a.x.toFixed(1)} → ${b.x.toFixed(1)} for half a screen of scroll`);
	}
	check(`${name}: group ${PINNED} (${label}, not rewritten) still pins and moves sideways with the page scroll${a && b ? ` (${a.x.toFixed(0)} → ${b.x.toFixed(0)} px)` : ''}`, p);
}

/** 12 – at a fractional dpr no ink between frames: the band red, the art grey – red at a frame edge is the band */
async function seams(name, dpr) {
	const p = [];
	await js(`(() => { const st = document.createElement('style'); st.textContent = '[data-endless] { background: #f00 !important } [data-endless] img, [data-endless] video, [data-endless] .slice-layers { filter: grayscale(1) !important }'; document.head.append(st); return __endless.bare(true); })()`);
	let s = await toBand();
	const vh = s.vh;
	let worst = { red: 0 };
	let edges = 0;
	for (let k = 0; k < 10; k++) {
		await wheel(await point(), k ? 97.37 + k * 13.1 : 0.01);
		await sleep(80);
		s = await state();
		const img = await shot(null, SHOTS && k === 0 && `endless-${name}-seams`);
		const ys = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9].map((t) => Math.round((Math.max(0, s.band.y) + t * (Math.min(vh, s.band.y + s.band.h) - Math.max(0, s.band.y))) * dpr));
		s.frames.forEach((f, i) => {
			if (f.l < 3 || f.l > s.win.w - 3) return;
			edges++;
			const cx = Math.round((s.win.x + f.l) * dpr);
			for (let dx = -3; dx <= 3; dx++) for (const y of ys) {
				const [r, g, b] = img.at(Math.min(img.w - 1, Math.max(0, cx + dx)), Math.min(img.h - 1, y));
				const red = r - Math.max(g, b);
				if (red > worst.red) worst = { red, seam: i === 0 ? `wrap ${N - 1} | 0` : `${i - 1} | ${i}`, x: cx + dx, y };
			}
		});
	}
	if (!edges) p.push('no frame edge on screen');
	if (worst.red > 40) p.push(`red ${worst.red} at the seam ${worst.seam} (device px ${worst.x}, ${worst.y})`);
	check(`${name}: no ink hairline between frames at dpr ${dpr} (${edges} edges, the worst red excess ${worst.red}${worst.seam ? ` at ${worst.seam}` : ''})`, p);
}

/** 13 – the drag hint: off / once / not while blocked / a touch cancels / reduced motion */
async function nudges(name, mobile) {
	const PLAYED = 2600; // dwell 700 + the sweep 1100 + the bounce from 78 % of it, 520 – and some slack
	const read = () => js('__nudge.read()');
	const fresh = async (hints) => {
		HINTS = hints;
		await send('Page.navigate', { url: 'about:blank' });
		await sleep(200);
	};
	/** the band in the middle of the screen, or (out) below it, off screen – no state read (the bounce is past the sides) */
	const place = (out) => js(`(() => { const b = __endless.band().getBoundingClientRect(); scrollTo({ top: scrollY + b.top + (${out} ? b.height + innerHeight : -(innerHeight - b.height) / 2), behavior: 'instant' }); return true; })()`);
	const say = (r) => `${r.sweeps.length} line(s)${r.sweeps.length ? ` (${r.sweeps.map((w) => `frame ${w.frame}, ${w.moving ? 'sweeping' : 'still'}`).join('; ')})` : ''}, ${r.runs.length} bounce(s)`;

	// off
	await fresh(false);
	if (await openPage(`${name} hints off`)) {
		await place(false);
		await sleep(PLAYED);
		const r = await read();
		const p = [];
		if (r.hints) p.push('html.sc-show-hints is set (the /me/ rewrite did not take)');
		if (r.sweeps.length || r.runs.length) p.push(say(r));
		check(`${name}: "Show hints" off – no hint line, no bounce (${say(r)})`, p);
	}

	// on: once, and not again after scrolling away and back
	await fresh(true);
	if (await openPage(`${name} hints on`)) {
		await place(false);
		await until(`(() => { const r = __nudge.read(); return r.sweeps.length && r.sweeps.every((w) => w.gone !== null) && r.runs.length && r.runs.every((b) => b.finished || b.canceled) ? true : null; })()`, 8000);
		const r1 = await read();
		await place(true);
		await sleep(400);
		await place(false);
		await sleep(PLAYED);
		const r2 = await read();
		const p = [];
		if (!r1.hints) p.push('html.sc-show-hints not set (the /me/ rewrite did not take)');
		if (r1.sweeps.length !== 1 || !r1.sweeps[0].moving || r1.sweeps[0].frame !== 0) p.push(`first view: ${say(r1)}, expected one sweeping line on frame 0`);
		if (r1.sweeps[0] && !r1.sweeps[0].inside) p.push('the line reaches past the window (it spans the part of the slice on screen)');
		if (r1.runs[0] && !r1.runs[0].fits) p.push(`the bounce zooms ${r1.runs[0].zoom}, more than the band's width needs`);
		if (r1.runs.length !== 1 || r1.runs[0].canceled) p.push(`first view: ${r1.runs.length} bounce(s)${r1.runs[0]?.canceled ? ', cancelled' : ''}, expected one`);
		else if (r1.sweeps[0] && !(r1.runs[0].t > r1.sweeps[0].t + 300)) p.push(`the bounce ${Math.round(r1.runs[0].t - r1.sweeps[0].t)} ms after the line, expected near its end`);
		if (r2.sweeps.length !== r1.sweeps.length || r2.runs.length !== r1.runs.length) p.push(`after scrolling away and back: ${say(r2)}`);
		check(`${name}: "Show hints" on – one hint line across frame 0 (on screen), then one bounce${r1.runs[0] && r1.sweeps[0] ? ` (${Math.round(r1.runs[0].t - r1.sweeps[0].t)} ms later, zoom ${r1.runs[0].zoom})` : ''}; not again after scrolling away and back (${say(r2)})`, p);
	}

	// the story blocks the page from the start: nothing; once it lets go: the nudge
	await fresh(true);
	await send('Page.navigate', { url: VIEWER + PAGE });
	await until(`location.href.startsWith('${VIEWER}') && window.__nudge && __nudge.hold(true)`, 5000);
	if (await openPage(`${name} blocked`, false)) {
		await place(false);
		await sleep(PLAYED);
		const r1 = await read();
		await js('__nudge.hold(false)');
		await sleep(PLAYED);
		const r2 = await read();
		const p = [];
		if (r1.sweeps.length || r1.runs.length) p.push(`while blocking: ${say(r1)}`);
		if (r2.sweeps.length !== 1) p.push(`after the story let go: ${say(r2)}, expected the nudge once`);
		check(`${name}: html[data-vn-blocking] – no hint line, no bounce while it blocks (${say(r1)}); after it: ${say(r2)}`, p);
	}

	// a touch / press on the band mid-sweep cancels it
	await fresh(true);
	await send('Page.navigate', { url: VIEWER + PAGE });
	let caught = null;
	for (const stop = Date.now() + 20000; Date.now() < stop && !caught; await sleep(30)) {
		const r = await js('window.__nudge?.read()').catch(() => null);
		if (r?.sweeps.length) caught = r;
	}
	if (!caught) check(`${name}: a ${mobile ? 'touch' : 'mouse press'} cancels the hint`, ['no hint line within 20 s']);
	else {
		const at = await point(0.5, 0.5);
		const go = pointer(mobile ? 'touch' : 'mouse');
		const pressed = await js('performance.now()');
		await go.down(at);
		await sleep(60);
		await go.up(at);
		await sleep(PLAYED);
		const r = await read();
		const w = r.sweeps[0];
		const p = [];
		if (w.gone === null || w.gone - pressed > 250) p.push(`the line stayed ${w.gone === null ? 'on' : `${Math.round(w.gone - pressed)} ms`} after the press`);
		if (r.runs.some((b) => !b.canceled && b.t > pressed)) p.push('a bounce started after the press');
		if (r.runs.some((b) => b.finished)) p.push('a bounce ran to its end');
		if (r.sweeps.length !== 1) p.push(`${r.sweeps.length} lines`);
		check(`${name}: a ${mobile ? 'touch' : 'mouse press'} on the band ${Math.round(pressed - w.t)} ms into the sweep cancels it – the line gone after ${w.gone === null ? '–' : Math.round(w.gone - pressed)} ms, ${r.runs.length} bounce(s)${r.runs.length ? ' cancelled' : ''}, no replay`, p);
	}

	// reduced motion: a still line, no bounce
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
	await fresh(true);
	if (await openPage(`${name} reduced motion`)) {
		await place(false);
		await sleep(PLAYED);
		const r = await read();
		const p = [];
		if (r.sweeps.length !== 1 || r.sweeps[0].moving) p.push(`${say(r)}, expected one still line`);
		if (r.runs.length) p.push(`${r.runs.length} bounce(s)`);
		check(`${name}: prefers-reduced-motion – the line stands still (it fades), no bounce (${say(r)})`, p);
	}
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
	HINTS = false;
}

const SIZES = [
	{ name: '390', w: 390, h: 844, dpr: 1, mobile: true },
	{ name: '1440', w: 1440, h: 900, dpr: 1, mobile: false },
	{ name: '1440@1.5', w: 1440, h: 900, dpr: 1.5, mobile: false, only: '1.5' }
].filter((s) => !opt.only || String(opt.only) === (s.only ?? s.name));

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setFocusEmulationEnabled', { enabled: true });
	await send('Fetch.enable', { patterns: [{ urlPattern: `*/library/toons/${TOON}/chapters/${CHAPTER}?*`, requestStage: 'Response' }, { urlPattern: `*/library/toons/${TOON}/chapters/${CHAPTER}`, requestStage: 'Response' }, { urlPattern: '*/me/', requestStage: 'Response' }, { urlPattern: '*/me/?*', requestStage: 'Response' }] });
	await send('Page.navigate', { url: `${VIEWER}/favicon-16x16.png` });
	await sleep(500);
	await js(`(localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(session.token)}), localStorage.setItem('scutoons.draft-preview', '1'), true)`);
	for (const size of SIZES) {
		console.log(`\n${size.name} (${size.w} × ${size.h}, dpr ${size.dpr})`);
		await send('Emulation.setDeviceMetricsOverride', { width: size.w, height: size.h, deviceScaleFactor: size.dpr, mobile: size.mobile });
		await send('Emulation.setTouchEmulationEnabled', { enabled: size.mobile, maxTouchPoints: size.mobile ? 5 : 1 });
		wide = { page: 0, band: 0 };
		if (!(await openPage(size.name))) continue;
		const s0 = { ...(await render(size.name, size.dpr)), dpr: size.dpr };
		if (size.dpr !== 1) {
			await seams(size.name, size.dpr);
		} else {
			await decoded(size.name, s0);
			await drag(size.name, 'mouse', size.dpr);
			if (size.mobile) await drag(size.name, 'touch', size.dpr);
			await fling(size.name, 'mouse');
			if (size.mobile) await fling(size.name, 'touch');
			await turns(size.name, size.dpr);
			await pageGestures(size.name, size.mobile, size.dpr);
			await keys(size.name, size.dpr);
			await blocked(size.name, size.mobile);
			await pinned(size.name);
			// a fresh reader: the save game emptied (in the browser and on the server), the chapter opened anew
			await send('Page.navigate', { url: 'about:blank' });
			await sleep(300);
			saveGame.clear();
			await send('Page.navigate', { url: `${VIEWER}/favicon-16x16.png` });
			await sleep(300);
			await js(`(localStorage.removeItem('${SAVE_KEY}'), true)`);
			if (await openPage(`${size.name} fresh`)) await sliver(size.name, size.dpr);
		}
		// the band's own box is all it adds to the page (it clips its frames); anything else past the sides is named
		const others = wide.page > 0 ? await js('__endless.over()') : [];
		if (others.length) console.log(`note ${size.name}: the page is ${wide.page} px wider than the viewport – not the band: ${others.join(', ')}`);
		check(`${size.name}: the band never reaches past the page's sides, the page never got wider by it (band ${wide.band.toFixed(1)} px, page ${wide.page} px)`, wide.band > 0.5 || (wide.page > 0 && !others.length) ? [`the band reaches ${wide.band.toFixed(1)} px past the page, the page ${wide.page} px wider`] : []);
		// last: the bounce moves the whole view sideways (zoomed) – past the sides by design
		if (size.dpr === 1) await nudges(size.name, size.mobile);
	}
	check('no page errors', errors.length ? [`${errors.length}: ${errors.slice(0, 3).join(' | ')}`] : []);
} catch (e) {
	check('run', [e?.stack ?? String(e)]);
} finally {
	ws.close();
	chrome.kill();
	if (!saveGame.restore()) console.log("     could not restore the member's save game");
}
const passed = results.filter(Boolean).length;
console.log(`\n${passed}/${results.length} passed`);
process.exit(passed === results.length ? 0 : 1);
