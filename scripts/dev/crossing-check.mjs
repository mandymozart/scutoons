#!/usr/bin/env node
// Text boxes across slice edges (agents/research/TEXT-ACROSS-SLICES.md §3, §7 – the reader's paint rule, step 1) in all
// three engines through Playwright (playwright-core, the pinned builds – see endless-engines-check.mjs). The API's chapter
// answer is rewritten: boxes are added that reach over a seam, forward (into the next slice) and backward (into the
// previous one), in a vertical group, in a pinned strip and – the same strip rewritten – in the endless band; a later
// slice's box lies on one of them (a collision). At 390 × 844 and 1440 × 900 per engine:
//   1. vertical: the crossing part of the forward and the backward box is on top (hit test at points over the neighbour's
//      picture); the later slice's box wins where they overlap
//   2. the pinned strip: the same at a frame seam, both directions (the track moved so the seam is on screen)
//   3. the endless band: the same, the wrap seam too (frame N's box reaching right over frame 1)
//   4. a box's text size is % of its slice's width (cqw from .slice-layers): the box's font size / the slice's width / the
//      layout's --fit is the stored size, in every layout
//   5. an emulated old engine (Safari ≤ 18: `container-type` = layout containment, a stacking context – here forced on
//      .slice-layers, where the container is now): 1–3 still hold; and the OLD rule (containment on .slice) fails the
//      forward box – the check can tell
// Not here: a real iPhone (iOS 17 / 18) – the plan's one device pass.
//   npm run check:crossing -- … | node scripts/dev/crossing-check.mjs [--engines=chromium,firefox,webkit] [--only=390|1440] [--page=/de/catpope/chapter/2]
//        (viewer :8000, API :9000 – local only; npm run dev:test-user)
import { chromium, firefox, webkit } from 'playwright-core';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const VIEWER = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const API = String(opt.api || local('api', '/api')).replace(/\/$/, '');
if (![VIEWER, API].every((u) => /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/.test(u))) { console.error('crossing-check: local servers only'); process.exit(2); }
const PAGE = String(opt.page || '/de/catpope/chapter/2');
const [, TOON, CHAPTER] = PAGE.match(/\/([^/?]+)\/chapter\/(\d+)/) ?? [];
const ENGINES = { chromium, firefox, webkit };
const engines = String(opt.engines || 'chromium,firefox,webkit').split(',').filter((e) => ENGINES[e]);
const SIZES = [{ name: '390', w: 390, h: 844 }, { name: '1440', w: 1440, h: 900 }].filter((s) => !opt.only || s.name === String(opt.only));

const sessionFile = join(ROOT, 'logs/dev/test-user.json');
const session = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')) : null;
const auth = { headers: { Authorization: `Bearer ${session?.token}` } };
if (!session?.token || !(await fetch(`${API}/me/`, auth)).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const answer = await (await fetch(`${API}/library/toons/${TOON}/chapters/${CHAPTER}?draft=1&locale=de`, auth)).json();
const groups = answer.groups ?? [];
const VERTICAL = groups.findIndex((g) => g?.renderer?.type !== 'horizontal' && (g?.slices?.length ?? 0) >= 4);
const STRIP = groups.findIndex((g) => g?.renderer?.type === 'horizontal' && !g.renderer.params?.endless && (g?.slices?.length ?? 0) >= 4);
if (VERTICAL < 0 || STRIP < 0) { console.error(`${TOON} ch${CHAPTER}: needs a vertical and a pinned horizontal group with ≥ 4 slices`); process.exit(2); }
const SIZE = 6; // the boxes' text size (% of the slice width)

/** the boxes added (marked by their text): forward from slice 1 into 2, backward from slice 3 into 2, a collider on 2 */
const box = (text, rect) => ({ id: `xc-${text}`, text, rect, fit: 'fixed', variant: 'caption', size: SIZE });
function inject(json, axis) {
	const all = json.groups ?? json.chapter?.groups;
	const g = all?.[axis === 'vertical' ? VERTICAL : STRIP];
	if (!g) return;
	if (axis === 'endless') g.renderer = { type: 'horizontal', params: { endless: true } };
	const s = g.slices;
	const add = (i, d) => (s[i].dialog = [...(s[i].dialog ?? []), d]);
	if (axis === 'vertical') {
		add(1, box('FWD', { x: 10, y: 88, width: 50, height: 20 }));
		add(3, box('BACK', { x: 40, y: -12, width: 50, height: 20 }));
		add(2, box('ONTOP', { x: 30, y: 1, width: 20, height: 4 }));
	} else {
		add(1, box('FWD', { x: 80, y: 30, width: 40, height: 10 }));
		add(3, box('BACK', { x: -20, y: 60, width: 40, height: 10 }));
		add(2, box('ONTOP', { x: 1, y: 32, width: 8, height: 4 }));
		// the wrap seam (endless): the last frame's box reaching right over frame 1
		const last = s.length - 1;
		add(last, box('WRAP', { x: 85, y: 75, width: 30, height: 10 }));
	}
}

const results = [];
const check = (name, problems) => { results.push(!problems.length); console.log(`${problems.length ? 'FAIL' : 'ok  '} ${name}${problems.length ? `  — ${problems.join('; ')}` : ''}`); };

/** in the page: a box by its text – the points of its part over the neighbour slice and what lies on top there */
const PROBE = `window.__xc = {
	box(text) { return [...document.querySelectorAll('.dialog')].find((d) => d.textContent.trim() === text) ?? null; },
	slice(el) { return el?.closest('[data-slice-index]') ?? el?.closest('.slice'); },
	/** the crossing part: the box's points outside its own slice (a grid), which element is on top at each */
	crossing(text) {
		const d = this.box(text);
		if (!d) return { missing: true };
		// the boxes are pointer-events: none (a tap goes to the picture) – elementFromPoint would look through them: for the
		// paint order, every box takes the hit test
		for (const x of document.querySelectorAll('.dialog')) x.style.pointerEvents = 'auto';
		const b = d.getBoundingClientRect();
		const own = this.slice(d).getBoundingClientRect();
		const pts = [];
		for (let i = 1; i < 6; i++) for (let j = 1; j < 4; j++) {
			const x = b.left + (b.width * i) / 6, y = b.top + (b.height * j) / 4;
			if (x < 0 || y < 0 || x > innerWidth || y > innerHeight) continue;
			if (x >= own.left && x <= own.right && y >= own.top && y <= own.bottom) continue;
			pts.push([x, y]);
		}
		// 'box' = itself; 'later' = a later slice's box (the collision rule: the later slice paints on top – real content may
		// lie there, more so with WebKit's taller text); anything else (a picture, an earlier box) is a paint bug
		const mine = this.slice(d);
		const hits = pts.map(([x, y]) => {
			const e = document.elementFromPoint(x, y);
			if (e && d.contains(e)) return 'box';
			const other = e?.closest('.dialog');
			if (other && mine.compareDocumentPosition(this.slice(other)) & Node.DOCUMENT_POSITION_FOLLOWING) return 'later';
			return other?.textContent.trim().slice(0, 24) ?? e?.tagName.toLowerCase() ?? 'none';
		});
		return { points: pts.length, hits };
	},
	/** the collider (a later slice's box lying on FWD): on top at its centre */
	ontop() {
		const d = this.box('ONTOP');
		if (!d) return 'missing';
		for (const x of document.querySelectorAll('.dialog')) x.style.pointerEvents = 'auto';
		const r = d.getBoundingClientRect();
		const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
		return d.contains(e) ? 'ok' : e?.closest('.dialog')?.textContent.trim().slice(0, 24) ?? e?.tagName.toLowerCase() ?? 'none';
	},
	/** a box's font size / its slice's width / the layout's fit scale (--fit: a strip fits its screen) – the stored size / 100 */
	scale(text) {
		const d = this.box(text);
		if (!d) return null;
		const fs = parseFloat(getComputedStyle(d.querySelector('.box') ?? d).fontSize);
		const fit = parseFloat(getComputedStyle(d).getPropertyValue('--fit')) || 1;
		return fs / fit / this.slice(d).getBoundingClientRect().width;
	},
	/** the box (and its neighbour) on screen: the page scrolled to it; a strip's / band's track moved by hand */
	show(text) {
		const d = this.box(text);
		if (!d) return false;
		const track = d.closest('.track');
		if (!track) { d.scrollIntoView({ block: 'center' }); return true; }
		const section = track.closest('section, .strip, [data-endless]') ?? track.parentElement;
		section.scrollIntoView({ block: 'start' });
		const frame = d.closest('.frame');
		// the box centred on screen (its seam with it – forward or backward)
		const x = frame.offsetLeft + d.offsetLeft + d.offsetWidth / 2 - innerWidth / 2;
		track.style.transform = 'translate3d(' + (-x) + 'px, 0, 0)';
		return true;
	}
};`;

async function run(browser, engine, size, axis, emulate) {
	const context = await browser.newContext({ viewport: { width: size.w, height: size.h } });
	await context.addInitScript((token) => {
		try {
			localStorage.setItem('scutoons_session', token);
			localStorage.setItem('scutoons.draft-preview', '1');
		} catch {}
	}, session.token);
	await context.addInitScript(PROBE);
	await context.route(new RegExp(`/library/toons/${TOON}/chapters/${CHAPTER}(\\?|$)`), async (route) => {
		const res = await route.fetch();
		if (res.status() !== 200) return route.fulfill({ response: res });
		const json = await res.json();
		inject(json, axis);
		await route.fulfill({ response: res, json });
	});
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (e) => errors.push(e.message));
	// not networkidle: Firefox never reaches it on the Vite dev server (endless-engines-check.mjs does the same)
	await page.goto(VIEWER + PAGE, { waitUntil: 'domcontentloaded' });
	await page.waitForFunction(() => window.__xc.box('FWD') && [...document.querySelectorAll('.slice img')].slice(0, 8).every((i) => i.complete), null, { timeout: 40000 });
	await page.waitForTimeout(500);
	if (emulate === 'new') await page.addStyleTag({ content: '.slice-layers { contain: layout !important; }' });
	if (emulate === 'old') await page.addStyleTag({ content: '.slice { contain: layout !important; }' });
	// emulated: the real boxes hidden – a later slice's real box lying on FWD would hide whether the picture covers it
	if (emulate) await page.evaluate(() => { for (const d of document.querySelectorAll('.dialog')) if (!/^(FWD|BACK|ONTOP|WRAP)$/.test(d.textContent.trim())) d.style.visibility = 'hidden'; });
	const texts = axis === 'endless' ? ['FWD', 'BACK', 'WRAP'] : ['FWD', 'BACK'];
	const out = {};
	for (const t of texts) {
		await page.evaluate((x) => window.__xc.show(x), t);
		await page.waitForTimeout(250);
		out[t] = await page.evaluate((x) => window.__xc.crossing(x), t);
		out[`${t}:scale`] = await page.evaluate((x) => window.__xc.scale(x), t);
		if (t === 'FWD') out.ontop = await page.evaluate(() => window.__xc.ontop());
	}
	await context.close();
	return { out, errors };
}

const label = { vertical: 'vertical group', strip: 'pinned strip', endless: 'endless band' };
for (const engine of engines) {
	const browser = await ENGINES[engine].launch();
	for (const size of SIZES) {
		for (const axis of ['vertical', 'strip', 'endless']) {
			const { out, errors } = await run(browser, engine, size, axis, null);
			const problems = [];
			for (const [t, r] of Object.entries(out)) {
				if (t === 'ontop') {
					if (r !== 'ok') problems.push(`the later slice's box (ONTOP) not on top of FWD: ${r}`);
					continue;
				}
				if (t.endsWith(':scale')) {
					if (r === null || Math.abs(r * 100 - SIZE) > 0.15) problems.push(`${t.split(':')[0]} text size ${r === null ? '–' : (r * 100).toFixed(2)} % of the slice (stored ${SIZE})`);
					continue;
				}
				if (r.missing) problems.push(`${t}: not drawn`);
				else if (!r.points) problems.push(`${t}: no crossing part on screen`);
				else {
					const wrong = r.hits.filter((h) => h !== 'box' && h !== 'later');
					if (wrong.length) problems.push(`${t}: under ${[...new Set(wrong)].join(', ')} at ${wrong.length} / ${r.points} points`);
				}
			}
			if (errors.length) problems.push(`page errors: ${errors.join(' | ').slice(0, 200)}`);
			check(`${engine} ${size.name} ${label[axis]}: forward / backward${axis === 'endless' ? ' / wrap seam' : ''} on top, cqw of the slice`, problems);
		}
		// an old engine, emulated: the containment where the container now is – still fine; on the slice – the old bug
		const now = await run(browser, engine, size, 'vertical', 'new');
		const was = await run(browser, engine, size, 'vertical', 'old');
		const under = (r) => (r.missing || !r.points ? null : r.hits.filter((h) => h !== 'box' && h !== 'later').length);
		const p = [];
		if (under(now.out.FWD) !== 0 || under(now.out.BACK) !== 0) p.push(`containment on .slice-layers: forward ${under(now.out.FWD)}, backward ${under(now.out.BACK)} points covered`);
		if (!(under(was.out.FWD) > 0)) p.push(`containment on .slice (the old rule) did not cover the forward box – the emulation tells nothing (${JSON.stringify(was.out.FWD)})`);
		check(`${engine} ${size.name} old engine emulated (layout containment): the new rule holds, the old one fails`, p);
	}
	await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`\n${results.length - failed} / ${results.length} ok`);
process.exit(failed ? 1 : 0);
