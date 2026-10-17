#!/usr/bin/env node
// The endless band (FEATURES 10.100, agents/research/TEXT-ACROSS-SLICES.md §8; viewer renderer/strip-endless.svelte) in
// all three engines – Chromium, Firefox and WebKit through Playwright (playwright-core, a root devDependency pinned to the
// browser builds in %LOCALAPPDATA%\ms-playwright, so nothing downloads). endless-check.mjs stays the deep one in Chrome
// (touch, decode on first paint, reading progress); this one runs the checks where engines differ, with the same page
// helpers (endless-page.mjs) and the same rewrite of the API's answer (catpope chapter 2 group 0 endless), as the local
// test user with the draft preview on, "Show hints" off except in 13. At 390 × 844 and 1440 × 900, then 1440 × 900 at
// device pixel ratio 1.5:
//   1. the band: the whole width, the height formula, a grab cursor, pan-y pinch-zoom, every slice once, side by side
//   3. a mouse drag waits out the 0.5rem dead zone, then moves every frame 1:1 (grabbing); resting before letting go: no glide
//   4. a mouse fling glides, stops within 4 s, never steps backwards (every rAF recorded)
//   5. a full turn each way in wheel steps: 1:1 each step, side by side across the wraps, the same picture mod T
//   6. wheel deltaY scrolls the page and leaves the band; deltaX moves the band 1:1 and leaves the page
//   7. ← → with focus ease to the next / previous frame edge
//   8. html[data-vn-blocking]: keys, wheel and a mouse drag leave the band
//  10. an un-rewritten horizontal group on the page still pins and moves sideways with the page scroll
//  11. the band never reaches past the page's sides, the page never gets wider by it; no page errors
//  12. at DPR 1.5: no ink hairline between frames (the band red, the art grey)
//  13. the drag hint: off – nothing; on – one line across frame 0 and one bounce, not again after scrolling away and
//      back; while the story blocks – nothing, after it once; a mouse press mid-sweep cancels it; reduced motion – a
//      still line, no bounce
// Engine traps (measured 2026-10-17): Playwright scales wheel deltas by the DPR in Chromium – every step is compared with
// the deltaX the page received; a pointer released outside the viewport gets no pointerup in Playwright's Firefox – the
// drags stay inside; the hint's bounce zooms the band for a moment – hints are off (the /me/ answer rewritten) but in 13.
// Not here: touch (Playwright can't swipe in Firefox / WebKit – endless-check.mjs swipes in Chrome; a real phone for the
// rest), a frame decoded before its first paint, reading progress.
// The browsers: `npx playwright-core install chromium firefox webkit` installs the builds of the pinned playwright-core
// (into %LOCALAPPDATA%\ms-playwright) – not `npx playwright install`: that fetches the newest playwright, other builds.
//   node scripts/dev/endless-engines-check.mjs [--engines=chromium,firefox,webkit] [--only=390|1440|1.5]
//        [--page=/de/catpope/chapter/2] [--group=0] [--pinned=<group index>]   (viewer :8000, API :9000 – local only)
import { chromium, firefox, webkit } from 'playwright-core';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ENDLESS_PAGE, NUDGE_PAGE, differ, fmt, gaps, moves, norm, off, png } from './endless-page.mjs';
import { local } from './ports.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const VIEWER = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const API = String(opt.api || local('api', '/api')).replace(/\/$/, '');
if (![VIEWER, API].every((u) => /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/.test(u))) { console.error('endless-engines-check: local servers only'); process.exit(2); }
const PAGE = String(opt.page || '/de/catpope/chapter/2');
const [, TOON, CHAPTER] = PAGE.match(/\/([^/?]+)\/chapter\/(\d+)/) ?? [];
if (!TOON) { console.error(`--page=${PAGE}: no /<toon>/chapter/<n> in it`); process.exit(2); }
const GROUP = Number(opt.group ?? 0);
const ENGINES = { chromium, firefox, webkit };
const engines = String(opt.engines || 'chromium,firefox,webkit').split(',').filter((e) => ENGINES[e]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test user, the chapter's groups (the draft preview's answer) – as endless-check.mjs ──
const sessionFile = join(ROOT, 'logs/dev/test-user.json');
const session = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')) : null;
const auth = { headers: { Authorization: `Bearer ${session?.token}` } };
if (!session?.token || !(await fetch(`${API}/me/`, auth)).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const answer = await (await fetch(`${API}/library/toons/${TOON}/chapters/${CHAPTER}?draft=1&locale=de`, auth)).json();
const groups = answer.groups ?? [];
const slices = (g) => groups[g]?.slices?.length ?? 0;
const horizontal = (g) => groups[g]?.renderer?.type === 'horizontal' && slices(g) >= 2;
if (!horizontal(GROUP)) { console.error(`${TOON} ch${CHAPTER} group ${GROUP} is no horizontal group with ≥ 2 slices`); process.exit(2); }
let start = GROUP;
while (start > 0 && groups[start - 1]?.continues === true) start--;
let end = GROUP;
while (end < groups.length - 1 && groups[end]?.continues === true) end++;
const N = slices(GROUP);
const OFFSET = groups.slice(start, GROUP).reduce((sum, _, i) => sum + slices(start + i), 0);
const PINNED = opt.pinned !== undefined ? Number(opt.pinned) : [...Array(end - start + 1).keys()].map((i) => start + i).find((g) => g !== GROUP && horizontal(g) && !groups[g].renderer.params?.endless);

const results = [];
const check = (name, problems) => { results.push(!problems.length); console.log(`${problems.length ? 'FAIL' : 'ok  '} ${name}${problems.length ? `  — ${problems.join('; ')}` : ''}`); };
const tolOf = (dpr) => 1 / dpr + 0.02;
const CHAPTER_URL = new RegExp(`/library/toons/${TOON}/chapters/${CHAPTER}(\\?|$)`);
const ME_URL = /\/me\/(\?|$)/;

/** one engine, one size: a fresh context (own storage) with the page helpers, the rewrites and the reader's wheel sum */
async function contextFor(browser, size, { hints = false, reducedMotion = 'no-preference', held = false } = {}) {
	const context = await browser.newContext({ viewport: { width: size.w, height: size.h }, deviceScaleFactor: size.dpr, reducedMotion });
	await context.addInitScript((token) => {
		try {
			localStorage.setItem('scutoons_session', token);
			localStorage.setItem('scutoons.draft-preview', '1');
		} catch {}
	}, session.token);
	await context.addInitScript(ENDLESS_PAGE);
	await context.addInitScript(NUDGE_PAGE);
	// the wheel events the page received and their deltaX (Playwright scales wheel deltas by the DPR in Chromium)
	await context.addInitScript(() => {
		window.__wx = 0;
		window.__wn = 0;
		addEventListener('wheel', (e) => ((window.__wx += e.deltaX), window.__wn++), { capture: true, passive: true });
	});
	if (held) await context.addInitScript(() => window.__nudge.hold(true));
	await context.route(CHAPTER_URL, async (route) => {
		const res = await route.fetch();
		if (res.status() !== 200) return route.fulfill({ response: res });
		const json = await res.json();
		const group = (json.groups ?? json.chapter?.groups)?.[GROUP];
		if (group) group.renderer = { type: 'horizontal', params: { endless: true } };
		await route.fulfill({ response: res, json });
	});
	await context.route(ME_URL, async (route) => {
		const res = await route.fetch();
		if (res.status() !== 200) return route.fulfill({ response: res });
		const json = await res.json();
		if (json.user) json.user.show_hints = hints;
		await route.fulfill({ response: res, json });
	});
	return context;
}

/** the checks of one page: everything reads the band through `__endless` (endless-page.mjs) */
function reader(page, engine, size, errors) {
	const name = `${engine} ${size.name}`;
	const js = (expression) => page.evaluate(expression);
	let wide = { page: 0, band: 0 };
	const widen = (s) => (wide = { page: Math.max(wide.page, s.sw - s.cw), band: Math.max(wide.band, -s.band.x, s.band.x + s.band.w - s.cw) });
	/** the band after the next two frames (an input's update drawn) */
	const state = async () => {
		const s = await js('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(window.__endless?.state()))))');
		if (s) widen(s);
		return s;
	};
	async function until(expression, ms = 20000) {
		for (const stop = Date.now() + ms; Date.now() < stop; await sleep(150)) {
			const v = await js(expression).catch(() => null);
			if (v) return v;
		}
		return null;
	}
	const point = async (fx = 0.5, fy = 0.5) => {
		const at = await js(`__endless.point(${fx}, ${fy})`);
		if (!at) throw new Error('something covers the whole band');
		return at;
	};
	/**
	 * a wheel event over `at`; returns the deltaX the page received – once it arrived: under load Playwright's Firefox
	 * returns from `wheel` before the page has the event (a step read 0 while the band moved)
	 */
	const wheel = async (at, deltaX, deltaY = 0) => {
		await page.mouse.move(at.x, at.y);
		const [x0, n0] = await js('[window.__wx, window.__wn]');
		await page.mouse.wheel(deltaX, deltaY);
		return js(`new Promise((r) => { const t0 = performance.now(); const f = () => (window.__wn > ${n0} || performance.now() - t0 > 2000 ? r(window.__wx - ${x0}) : requestAnimationFrame(f)); f(); })`);
	};
	const toBand = async () => (await js(`(() => { const b = __endless.band().getBoundingClientRect(); scrollTo({ top: scrollY + b.top - (innerHeight - b.height) / 2, behavior: 'instant' }); return true; })()`), await sleep(150), { ...(await state()), vh: await js('innerHeight') });
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
	const shot = async (clip) => png(await page.screenshot({ clip, scale: 'device' }));
	const bandClip = (s) => { const y = Math.max(0, s.band.y), b = Math.min(s.vh, s.band.y + s.band.h); return { x: Math.max(0, s.win.x), y, width: Math.min(s.cw, s.win.x + s.win.w) - Math.max(0, s.win.x), height: b - y }; };

	async function open() {
		// the dev server restarts when another session changes its setup – wait for it (30 s)
		for (let i = 0; ; i++) {
			try {
				await page.goto(VIEWER + PAGE, { waitUntil: 'domcontentloaded' });
				break;
			} catch (e) {
				if (i >= 10 || !/CONNECTION_REFUSED|Could not connect/i.test(String(e?.message))) throw e;
				await sleep(3000);
			}
		}
		const ready = await until(`(() => { const s = window.__endless?.state(); const imgs = [...(__endless.band()?.querySelectorAll('img') ?? [])]; return s && s.T > 0 && imgs.length >= ${N} && imgs.every((i) => i.complete && i.naturalWidth > 0) ? true : null; })()`, 40000);
		await sleep(700);
		if (!ready) check(`${name}: the endless band renders`, ['no band with loaded frames within 40 s']);
		return !!ready;
	}

	/** 1 */
	async function render() {
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
		check(`${name}: the band ${s.band.w.toFixed(0)} × ${s.band.h.toFixed(0)}, a grab cursor, pan-y pinch-zoom, ${N} frames, every slice once`, p);
	}

	/** 3 */
	async function drag() {
		const s0 = await toBand();
		const p = [];
		const rem = await js(`parseFloat(getComputedStyle(document.documentElement).fontSize)`);
		const at = await point(0.6);
		await page.mouse.move(at.x, at.y);
		await page.mouse.down();
		await page.mouse.move(at.x - rem * 0.25, at.y);
		await sleep(30);
		const s1 = await state();
		for (let i = 1; i <= 10; i++) await page.mouse.move(at.x - 10 * i, at.y), await sleep(16);
		const s2 = await state();
		const cursor = await js(`getComputedStyle(__endless.band()).cursor`);
		await sleep(120);
		await page.mouse.up();
		await sleep(500);
		const s3 = await state();
		if (off(moves(s0, s1), 0, 0.01)) p.push(`moved in the dead zone: ${fmt(moves(s0, s1))}`);
		if (off(moves(s0, s2), -100, tolOf(size.dpr))) p.push(`a 100 px drag moved the frames ${fmt(moves(s0, s2))}`);
		if (off(moves(s2, s3), 0, 0.01)) p.push(`glided after resting: ${fmt(moves(s2, s3))}`);
		if (cursor !== 'grabbing') p.push(`cursor ${cursor} while dragging`);
		for (const s of [s1, s2, s3]) if (gaps(s)) p.push(gaps(s));
		check(`${name}: a mouse drag waits out the 0.5rem dead zone, then moves 1:1 (${moves(s0, s2)[0].toFixed(2)} for −100), no glide after resting`, p);
	}

	/** 4 – the fling stays inside the viewport (Firefox: no pointerup for a release outside it) */
	async function fling() {
		await toBand();
		const p = [];
		const at = await point(0.8);
		await js('__endless.record()');
		await page.mouse.move(at.x, at.y);
		await page.mouse.down();
		const step = Math.min(50, (at.x - 20) / 6);
		for (let i = 1; i <= 6; i++) await page.mouse.move(at.x - step * i, at.y), await sleep(8);
		await page.mouse.up();
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
		check(`${name}: a mouse fling glides ${(-glide).toFixed(0)} px, stops after ${Math.round(ms)} ms, never backwards (${st.length - r} frames)`, p);
	}

	/** 5 */
	async function turns() {
		let s = await toBand();
		const vh = s.vh;
		const at = await point();
		const p = [];
		// every frame shown once first (their entrances), then everything that animates paused
		for (let k = 0; k < Math.ceil(s.T / 300); k++) await wheel(at, 300), await sleep(40);
		// what the page receives per px sent (Chromium at DPR 1.5: less) – each step is sent so the page receives it
		const scale = (await wheel(at, 60)) / 60 || 1;
		await sleep(1500);
		await js(`(() => { const st = document.createElement('style'); st.id = 'endless-check-still'; st.textContent = '*, *::before, *::after { animation-play-state: paused !important; transition: none !important; caret-color: transparent !important; }'; document.head.append(st); document.getAnimations().forEach((a) => a.pause()); return __endless.bare(true); })()`);
		await sleep(200);
		s = await state();
		const s0 = s;
		const clip = bandClip({ ...s0, vh });
		const a = await shot(clip);
		const pictures = [];
		let steps = 0;
		for (const [step, turnsBy] of [[61, 1], [-67, 2]]) {
			const total = turnsBy * s0.T;
			let done = 0;
			while (Math.abs(done) < total - 0.001) {
				const want = Math.sign(step) * Math.min(Math.abs(step), total - Math.abs(done));
				const before = s;
				const got = await wheel(at, want / scale);
				s = await state();
				steps++;
				done += got;
				const m = moves(before, s);
				if (off(m, -got, tolOf(size.dpr))) p.push(`step ${steps} (${got.toFixed(2)}): ${fmt(m)}`);
				const g = gaps(s);
				if (g) p.push(`step ${steps}: ${g}`);
				if (p.length > 4) break;
			}
			const drift = norm(s.frames[0].l - s0.frames[0].l, s0.T);
			if (Math.abs(drift) > tolOf(size.dpr)) p.push(`after ${turnsBy} turn(s) ${step > 0 ? 'left' : 'right'} frame 0 is ${drift.toFixed(2)} px off`);
			await sleep(250);
			const b = await shot(clip);
			const { share, box } = differ(a, b);
			pictures.push((share * 100).toFixed(3));
			if (share > 0.0005) p.push(`the picture after the turn ${step > 0 ? 'left' : 'right'} differs in ${(share * 100).toFixed(2)} % of its pixels (${box})`);
		}
		await js(`(document.getElementById('endless-check-still')?.remove(), document.getAnimations().forEach((a) => a.play()), __endless.bare(false))`);
		check(`${name}: a full turn each way (${steps} wheel steps): 1:1, side by side across the wraps, the same picture mod T (${pictures.join(' / ')} % differ)`, p.slice(0, 5));
	}

	/** 6 */
	async function pageGestures() {
		const p = [];
		let s0 = await toBand();
		await wheel(await point(), 0, 300);
		await sleep(700);
		let s1 = await state();
		if (s1.y - s0.y < 100) p.push(`wheel deltaY 300 scrolled the page ${Math.round(s1.y - s0.y)} px`);
		if (off(moves(s0, s1), 0, 0.01)) p.push(`wheel deltaY moved the band ${fmt(moves(s0, s1))}`);
		s0 = await toBand();
		await sleep(250);
		const got = await wheel(await point(), 250);
		await sleep(300);
		s1 = await state();
		if (off(moves(s0, s1), -got, tolOf(size.dpr))) p.push(`wheel deltaX ${got} moved the band ${fmt(moves(s0, s1))}`);
		if (Math.abs(s1.y - s0.y) > 0.5) p.push(`wheel deltaX scrolled the page ${Math.round(s1.y - s0.y)} px`);
		check(`${name}: wheel deltaY scrolls the page, the band stays; deltaX moves the band 1:1`, p);
	}

	/** 7 */
	async function keys() {
		const p = [];
		const s = await toBand();
		const near = Math.min(...s.frames.map((f) => f.l).filter((l) => l > 0.5));
		if (near < 100) await wheel(await point(), -150), await sleep(200);
		await js(`(__endless.band().focus({ preventScroll: true }), true)`);
		for (const k of ['ArrowRight', 'ArrowRight', 'ArrowLeft']) {
			const s0 = await state();
			const fs = s0.frames.map((f, i) => ({ ...f, i }));
			const want = k === 'ArrowRight' ? fs.filter((f) => f.l > 0.5).sort((a, b) => a.l - b.l)[0] : fs.filter((f) => f.l < -0.5).sort((a, b) => b.l - a.l)[0];
			await page.keyboard.press(k);
			await sleep(70);
			const mid = await js('window.__endless.state()');
			const s1 = await settle();
			const l = s1.frames[want.i].l;
			const m = mid.frames[want.i].l;
			if (Math.abs(l) > 0.5 / size.dpr + 0.02) p.push(`${k}: frame ${want.i} ends at ${l.toFixed(2)}, not at the window's left`);
			if (!(Math.abs(m) < Math.abs(want.l) - 1 && Math.abs(m) > 0.5)) p.push(`${k}: no ease (frame ${want.i} at ${want.l.toFixed(1)} → ${m.toFixed(1)} after 70 ms → ${l.toFixed(1)})`);
		}
		check(`${name}: ← → with focus ease to the next / previous frame edge`, p);
	}

	/** 8 */
	async function blocked() {
		const s0 = await toBand();
		await js(`(document.documentElement.dataset.vnBlocking = '', __endless.band().focus({ preventScroll: true }), true)`);
		const at = await point(0.6);
		await page.keyboard.press('ArrowRight');
		await sleep(400);
		await wheel(at, 200);
		await sleep(200);
		await page.mouse.move(at.x, at.y);
		await page.mouse.down();
		for (let i = 1; i <= 6; i++) await page.mouse.move(at.x - 20 * i, at.y), await sleep(16);
		await page.mouse.up();
		await sleep(500);
		const s1 = await state();
		await js(`(delete document.documentElement.dataset.vnBlocking, true)`);
		check(`${name}: html[data-vn-blocking] – keys, wheel, a mouse drag leave the band`, off(moves(s0, s1), 0, 0.01) ? [`moved ${fmt(moves(s0, s1))}`] : []);
	}

	/** 10 */
	async function pinned() {
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

	/** 11 */
	async function width() {
		const others = wide.page > 0 ? await js('__endless.over()') : [];
		if (others.length) console.log(`note ${name}: the page is ${wide.page} px wider than the viewport – not the band: ${others.join(', ')}`);
		check(`${name}: the band never reaches past the page's sides, the page never got wider by it (band ${wide.band.toFixed(1)} px, page ${wide.page} px)`, wide.band > 0.5 || (wide.page > 0 && !others.length) ? [`the band reaches ${wide.band.toFixed(1)} px past the page, the page ${wide.page} px wider`] : []);
		check(`${name}: no page errors`, errors.length ? [`${errors.length}: ${[...new Set(errors)].slice(0, 3).join(' | ')}`] : []);
	}

	/** 12 – at a fractional dpr no ink between frames: the band red, the art grey – red at a frame edge is the band */
	async function seams() {
		const p = [];
		await js(`(() => { const st = document.createElement('style'); st.textContent = '[data-endless] { background: #f00 !important } [data-endless] img, [data-endless] video, [data-endless] .slice-layers { filter: grayscale(1) !important }'; document.head.append(st); return __endless.bare(true); })()`);
		let s = await toBand();
		const vh = s.vh;
		const dpr = size.dpr;
		let worst = { red: 0 };
		let edges = 0;
		for (let k = 0; k < 10; k++) {
			await wheel(await point(), k ? 97.37 + k * 13.1 : 0.01);
			s = await state();
			const img = await shot({ x: 0, y: 0, width: s.cw, height: vh });
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

	return { open, render, drag, fling, turns, pageGestures, keys, blocked, pinned, width, seams, toBand, until, point };
}

/** 13 – the drag hint, each case in a fresh context ("Show hints" from the /me/ rewrite) */
async function nudges(browser, engine, size) {
	const name = `${engine} ${size.name}`;
	const PLAYED = 2600; // dwell 700 + the sweep 1100 + the bounce from 78 % of it, 520 – and some slack
	const say = (r) => `${r.sweeps.length} line(s)${r.sweeps.length ? ` (${r.sweeps.map((w) => `frame ${w.frame}, ${w.moving ? 'sweeping' : 'still'}`).join('; ')})` : ''}, ${r.runs.length} bounce(s)`;
	const place = (page, out) => page.evaluate(`(() => { const b = __endless.band().getBoundingClientRect(); scrollTo({ top: scrollY + b.top + (${out} ? b.height + innerHeight : -(innerHeight - b.height) / 2), behavior: 'instant' }); return true; })()`);
	async function run(options, body) {
		const context = await contextFor(browser, size, options);
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', (e) => errors.push(e.message));
		try {
			const r = reader(page, engine, size, errors);
			if (await r.open()) await body(page, r, () => page.evaluate('__nudge.read()'));
		} finally {
			await context.close();
		}
	}

	await run({ hints: false }, async (page, _, read) => {
		await place(page, false);
		await sleep(PLAYED);
		const r = await read();
		const p = [];
		if (r.hints) p.push('html.sc-show-hints is set (the /me/ rewrite did not take)');
		if (r.sweeps.length || r.runs.length) p.push(say(r));
		check(`${name}: "Show hints" off – no hint line, no bounce (${say(r)})`, p);
	});

	await run({ hints: true }, async (page, rd, read) => {
		await place(page, false);
		await rd.until(`(() => { const r = __nudge.read(); return r.sweeps.length && r.sweeps.every((w) => w.gone !== null) && r.runs.length && r.runs.every((b) => b.finished || b.canceled) ? true : null; })()`, 8000);
		const r1 = await read();
		await place(page, true);
		await sleep(400);
		await place(page, false);
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
		check(`${name}: "Show hints" on – one hint line across frame 0, then one bounce${r1.runs[0] && r1.sweeps[0] ? ` (${Math.round(r1.runs[0].t - r1.sweeps[0].t)} ms later, zoom ${r1.runs[0].zoom})` : ''}; not again after scrolling away and back (${say(r2)})`, p);
	});

	// the story blocks the page from the start (html[data-vn-blocking] held by __nudge.hold): nothing; once it lets go: once
	await run({ hints: true, held: true }, async (page, _, read) => {
		await place(page, false);
		await sleep(PLAYED);
		const r1 = await read();
		await page.evaluate('__nudge.hold(false)');
		await sleep(PLAYED);
		const r2 = await read();
		const p = [];
		if (r1.sweeps.length || r1.runs.length) p.push(`while blocking: ${say(r1)}`);
		if (r2.sweeps.length !== 1) p.push(`after the story let go: ${say(r2)}, expected the nudge once`);
		check(`${name}: html[data-vn-blocking] – no hint line, no bounce while it blocks (${say(r1)}); after it: ${say(r2)}`, p);
	});

	// a mouse press on the band mid-sweep cancels it
	await run({ hints: true }, async (page, rd, read) => {
		await place(page, false);
		const caught = await rd.until('(() => { const r = window.__nudge?.read(); return r?.sweeps.length ? r : null; })()', 20000);
		if (!caught) return check(`${name}: a mouse press cancels the hint`, ['no hint line within 20 s']);
		const at = await rd.point(0.5, 0.5);
		const pressed = await page.evaluate('performance.now()');
		await page.mouse.move(at.x, at.y);
		await page.mouse.down();
		await sleep(60);
		await page.mouse.up();
		await sleep(PLAYED);
		const r = await read();
		const w = r.sweeps[0];
		const p = [];
		if (w.gone === null || w.gone - pressed > 250) p.push(`the line stayed ${w.gone === null ? 'on' : `${Math.round(w.gone - pressed)} ms`} after the press`);
		if (r.runs.some((b) => !b.canceled && b.t > pressed)) p.push('a bounce started after the press');
		if (r.runs.some((b) => b.finished)) p.push('a bounce ran to its end');
		if (r.sweeps.length !== 1) p.push(`${r.sweeps.length} lines`);
		check(`${name}: a mouse press on the band ${Math.round(pressed - w.t)} ms into the sweep cancels it – the line gone after ${w.gone === null ? '–' : Math.round(w.gone - pressed)} ms, ${r.runs.length} bounce(s)${r.runs.length ? ' cancelled' : ''}, no replay`, p);
	});

	await run({ hints: true, reducedMotion: 'reduce' }, async (page, _, read) => {
		await place(page, false);
		await sleep(PLAYED);
		const r = await read();
		const p = [];
		if (r.sweeps.length !== 1 || r.sweeps[0].moving) p.push(`${say(r)}, expected one still line`);
		if (r.runs.length) p.push(`${r.runs.length} bounce(s)`);
		check(`${name}: prefers-reduced-motion – the line stands still (it fades), no bounce (${say(r)})`, p);
	});
}

const SIZES = [
	{ name: '390', w: 390, h: 844, dpr: 1 },
	{ name: '1440', w: 1440, h: 900, dpr: 1 },
	{ name: '1440@1.5', w: 1440, h: 900, dpr: 1.5, only: '1.5' }
].filter((s) => !opt.only || String(opt.only) === (s.only ?? s.name));

for (const engine of engines) {
	const browser = await ENGINES[engine].launch().catch((e) => (check(`${engine}: launch`, [String(e.message).split('\n')[0]]), null));
	if (!browser) continue;
	console.log(`\n${engine} ${browser.version()}`);
	try {
		for (const size of SIZES) {
			const context = await contextFor(browser, size);
			const page = await context.newPage();
			const errors = [];
			page.on('pageerror', (e) => errors.push(e.message));
			const r = reader(page, engine, size, errors);
			try {
				if (await r.open()) {
					await r.render();
					if (size.dpr !== 1) await r.seams();
					else {
						await r.drag();
						await r.fling();
						await r.turns();
						await r.pageGestures();
						await r.keys();
						await r.blocked();
						await r.pinned();
					}
					await r.width();
				}
			} catch (e) {
				check(`${engine} ${size.name}: run`, [e?.stack?.split('\n').slice(0, 2).join(' ') ?? String(e)]);
			} finally {
				await context.close();
			}
			if (size.dpr === 1) await nudges(browser, engine, size).catch((e) => check(`${engine} ${size.name}: hints run`, [String(e?.message ?? e)]));
		}
	} finally {
		await browser.close();
	}
}
const passed = results.filter(Boolean).length;
console.log(`\n${passed}/${results.length} passed`);
process.exit(passed === results.length ? 0 : 1);
