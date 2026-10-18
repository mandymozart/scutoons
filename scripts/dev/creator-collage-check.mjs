#!/usr/bin/env node
// The score collage stage (agents/research/COLLAGE.md §4.4, §8 step 6) in headless Chrome (own profile, removed on
// exit) as the local test user (agent@scutoons.test, `npm run dev:test-user`) on the running LOCAL dev stack:
//   - the catalog editor: a score item has "Collage" under More → /catalog/items/<id>/collage;
//   - the stage: the readers' score in a phone screen (390 × 844, scaled to the room), the score view live inside; its
//     header, sheet (= Verovio's page) and footer as high as the reader's own score view on that screen (the viewer's
//     lab, plain look: a 390 × 844 touch phone, a 1440 × 810 desktop), its Play label shown / hidden as there;
//   - a shape (disc black) via "Add a piece …" lands in the middle of the page shown; a drag re-anchors it onto bar 10,
//     staff 1 (its box says data-bar 10, the drawn piece sits on the staff lines), one undo step (also when the drag
//     pauses 1.5 s half-way: one Ctrl+Z goes back to its start); the look Negative (difference) reads as the paper's
//     negative (pixels: dark where the paper is, bright where the ink is); the corner scales it; dragged over the page's
//     bottom edge, the leaf does not scroll (the layers keep the music's width, the handle stays on the page), dragged
//     back it is on its note again;
//   - Phone → Desktop (1440 × 810) → zoom + twice: the piece stays on bar 10, staff 1 each time (the lines break anew);
//   - a character picture (Characters tab, a mood portrait) dragged onto bar 12, staff 2: on it at zoom 30 and 35;
//     Behind = under the notes (z 0, the music z 1); Page keeps it where it is (and back);
//   - Save draft (note "collage-check (removed again)"): the item document holds score.collage { file = the score's
//     MusicXML, both pieces – CDN-root paths, their bars }; a reload shows them at their bars; the keys on the live
//     region only (the focus on a sidebar link: Backspace / Delete / Esc leave the piece); Delete + Ctrl+Z; Esc;
//   - 375 × 812: the side region below the stage.
// Afterwards the saved versions are deleted again (by their note; an interrupted run's are swept first) – the item's
// document as before. Refuses to run while the item has an unpublished draft (someone's work in progress).
//   npm run check:creator-collage        (creator :10000, viewer :8000, API :9000, cdn :8001, MariaDB container)
//   SCUTOONS_COLLAGE_ITEM (default cloud9_en: a published score, full score); COLLAGE_SHOTS=<dir>: collage-*.png
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { png } from './endless-page.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
for (const url of [CREATOR, API, VIEWER]) {
	if (!['localhost', '127.0.0.1'].includes(new URL(url).hostname)) {
		console.error(`creator-collage-check: refusing ${url} – local dev stack only`);
		process.exit(2);
	}
}
const ITEM = process.env.SCUTOONS_COLLAGE_ITEM ?? 'cloud9_en';
const KEY = `item:${ITEM}`;
const NOTE = 'collage-check (removed again)';
const SHOTS = process.env.COLLAGE_SHOTS;
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`), !!ok);

// ── the test user, the API, the restore ──
execFileSync(process.execPath, [resolve(ROOT, 'scripts/dev/test-user.mjs')], { stdio: ['ignore', 'ignore', 'inherit'] });
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
async function api(method, path) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}` } });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw Object.assign(new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)?.slice(0, 200)}`), { status: res.status });
	return json;
}
const draftOf = () => api('GET', `/entities/${KEY}?draft=1&wrap=1`).catch((e) => (e.status === 404 ? null : Promise.reject(e)));
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
const sweep = () => {
	const ids = sql(`SELECT id FROM entity_versions WHERE entity_key = ${quote(KEY)} AND message = ${quote(NOTE)}`).split('\n').filter(Boolean).map(Number);
	if (ids.length) sql(`DELETE FROM entity_versions WHERE id IN (${ids.join(',')})`);
	return ids.length;
};
const swept = sweep();
if (swept) console.log(`note: an earlier run's leftovers removed: ${swept} draft version(s)`);
const before = await draftOf();
if (before && before.published?.id !== before.version.id) {
	console.error(`creator-collage-check: ${KEY} has an unpublished draft; not touching it. Publish or restore it first.`);
	process.exit(2);
}
const entry = (await api('GET', '/catalog/docs/items')).entries.find((e) => e.id === ITEM);
const MUSICXML = (before?.content ?? entry?.content)?.score?.musicxml;
if (!entry || !MUSICXML) {
	console.error(`creator-collage-check: ${ITEM} is no score item with a MusicXML file here`);
	process.exit(2);
}

// ── Chrome ──
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9900 + Math.floor(Math.random() * 90);
const profile = tempProfile('creator-collage-');
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try {
		wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl;
	} catch {}
	if (!wsUrl) await sleep(200);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
const errors = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) return pending.get(m.id)(m), pending.delete(m.id);
	if (m.method === 'Page.javascriptDialogOpening') ws.send(JSON.stringify({ id: ++seq, method: 'Page.handleJavaScriptDialog', params: { accept: true } }));
	if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
});
const send = (method, params = {}) =>
	new Promise((r) => {
		const i = ++seq;
		pending.set(i, r);
		ws.send(JSON.stringify({ id: i, method, params }));
	});
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(`${expression.slice(0, 80)}: ${r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text}`);
	return r.result?.result?.value;
}
async function until(expression, ms = 20000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(150);
	}
	return null;
}
const q = (testid) => `document.querySelector('[data-testid="${testid}"]')`;
const tid = (testid) => `[data-testid="${testid}"]`;
async function go(url, ready) {
	await send('Page.navigate', { url: `${CREATOR}${url}` });
	await until('document.readyState === "complete"');
	return until(ready, 40000);
}
async function shot(name) {
	if (!SHOTS) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(SHOTS, `collage-${name}.png`), Buffer.from(r.result.data, 'base64'));
}
async function mouse(type, x, y, buttons = 0) {
	await send('Input.dispatchMouseEvent', { type, x, y, button: type === 'mouseMoved' && !buttons ? 'none' : 'left', buttons, clickCount: type === 'mouseMoved' ? 0 : 1 });
}
/** a real mouse click in the middle of the element */
async function click(selector) {
	const at = await js(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	if (!at) throw new Error(`no ${selector}`);
	await mouse('mouseMoved', ...at);
	await mouse('mousePressed', ...at, 1);
	await mouse('mouseReleased', ...at);
	await sleep(250);
}
/** a real mouse drag from one viewport point to another, in steps; `pause` ms = held still half-way */
async function drag([x, y], [tx, ty], pause = 0) {
	await mouse('mouseMoved', x, y);
	await mouse('mousePressed', x, y, 1);
	for (let i = 1; i <= 12; i++) {
		if (i === 7 && pause) await sleep(pause);
		await mouse('mouseMoved', x + ((tx - x) * i) / 12, y + ((ty - y) * i) / 12, 1);
	}
	await sleep(100);
	await mouse('mouseReleased', tx, ty);
	await sleep(400);
}
/** types into an input like a person: the value set and an input event */
const typeInto = (testid, text) =>
	js(`(() => { const f = ${q(testid)}; f.focus(); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, ${JSON.stringify(text)}); f.dispatchEvent(new Event('input', { bubbles: true })); f.blur(); return true; })()`);
const KEYS = { Backspace: 8, Escape: 27, Delete: 46, z: 90, y: 89 };
async function press(key, ...mods) {
	const modifiers = (mods.includes('ctrl') ? 2 : 0) | (mods.includes('shift') ? 8 : 0);
	const base = { key: mods.includes('shift') && key.length === 1 ? key.toUpperCase() : key, code: key.length === 1 ? `Key${key.toUpperCase()}` : key, windowsVirtualKeyCode: KEYS[key], modifiers };
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	await sleep(250);
}

// ── the stage, read from the page ──
const FRAME = tid('score-frame');
const pickerOpen = `!!document.querySelector('[data-testid="collage-picker-sheet"]')?.closest('dialog')?.open`;
const rows = () => js(`[...document.querySelectorAll('[data-testid="collage-list"] > li')].map((li) => li.dataset.testid.replace('collage-row-', ''))`);
const selectedId = () => js(`document.querySelector('[data-testid="piece-inspector"]')?.dataset.piece ?? null`);
/** the page the screen shows (its leaf in the pages' scroll box) */
const shownPage = () => js(`(() => { const box = document.querySelector('${FRAME} [data-testid="score-pages"]')?.getBoundingClientRect(); if (!box) return 0; const leaf = [...document.querySelectorAll('${FRAME} .leaf')].find((l) => { const r = l.getBoundingClientRect(); return r.width && Math.abs(r.left + r.width / 2 - (box.left + box.width / 2)) < r.width / 2; }); return leaf ? Number(leaf.dataset.page) : 0; })()`);
/** bar n's staff s: its five lines' box (client px) on a rendered page, and that page – null when not rendered */
const staffBox = (n, s = 1) =>
	js(`(() => { for (const leaf of document.querySelectorAll('${FRAME} .leaf')) { const bar = leaf.querySelector('g.measure[data-n="${n}"]'); if (!bar) continue; const staff = bar.querySelector('g.staff[data-n="${s}"]'); if (!staff) return null; const lines = [...staff.querySelectorAll(':scope > path')].slice(0, 5).map((l) => l.getBoundingClientRect()); if (lines.length < 5) return null; const x = Math.min(...lines.map((r) => r.left)), y = Math.min(...lines.map((r) => r.top)), r = Math.max(...lines.map((r) => r.right)), b = Math.max(...lines.map((r) => r.bottom)); return { page: Number(leaf.dataset.page), x, y, w: r - x, h: b - y }; } return null; })()`);
/** the drawn piece (in the score view's pieces layer) and its overlay box */
const drawn = (id) =>
	js(`(() => { const p = document.querySelector('${FRAME} .score-pieces .piece[data-piece="${id}"]'); const b = document.querySelector('${FRAME} [data-testid="stage-piece-${id}"]'); if (!p || !b) return null; const r = p.getBoundingClientRect(); const s = getComputedStyle(p); return { x: r.left, y: r.top, w: r.width, h: r.height, cx: r.left + r.width / 2, cy: r.top + r.height / 2, bar: b.dataset.bar, z: s.zIndex, blend: s.mixBlendMode, behind: b.classList.contains('behind'), selected: b.classList.contains('selected'), page: Number(p.closest('.leaf')?.dataset.page ?? 0), src: p.querySelector('img')?.getAttribute('src') ?? p.querySelector('.fill')?.style.getPropertyValue('--src') ?? '' }; })()`);
/** the piece's centre on bar n's staff s (inside its lines, ± half the staff's height up and down) */
async function onStaff(id, n, s) {
	const [p, box] = [await drawn(id), await staffBox(n, s)];
	const ok = !!p && !!box && p.bar === String(n) && p.page === box.page && p.cx >= box.x && p.cx <= box.x + box.w && p.cy >= box.y - box.h / 2 && p.cy <= box.y + box.h * 1.5;
	return { ok, detail: JSON.stringify({ piece: p && { cx: Math.round(p.cx), cy: Math.round(p.cy), bar: p.bar, page: p.page }, staff: box && { x: Math.round(box.x), y: Math.round(box.y), w: Math.round(box.w), h: Math.round(box.h), page: box.page } }) };
}
/** waits for a new layout to settle on bar n (rendered) with the piece's box drawn on it */
const settled = (id, n, s) => until(`(async () => { const a = JSON.stringify(document.querySelector('${FRAME} .score-pieces .piece[data-piece="${id}"]')?.getBoundingClientRect()); await new Promise((r) => setTimeout(r, 400)); const b = JSON.stringify(document.querySelector('${FRAME} .score-pieces .piece[data-piece="${id}"]')?.getBoundingClientRect()); return a && a === b && !!document.querySelector('${FRAME} .leaf g.measure[data-n="${n}"] g.staff[data-n="${s}"]') && document.querySelector('${FRAME} [data-testid="stage-piece-${id}"]')?.dataset.bar === '${n}'; })()`, 30000);
/** turns pages until bar n is on the page shown */
async function turnTo(n) {
	for (let i = 0; i < 12; i++) {
		const [box, shown] = [await staffBox(n), await shownPage()];
		if (box && box.page === shown) return true;
		await click(`${FRAME} [data-testid="${box && box.page < shown ? 'score-prev' : 'score-next'}"]`);
		await sleep(700);
	}
	return false;
}
/** a score view's chrome under `root`: its size, its header, sheet (= Verovio's page) and footer heights in its own CSS
 *  px, whether the Play label shows and whether the shown page fits its sheet (no scrolling) – null until a page is drawn */
const chromeOf = (root) =>
	`(() => { const v = document.querySelector('${root} [data-testid="score-view"]'); const [h, s, f, w] = ['header', '.sheet', 'footer', '.play .wide'].map((c) => v?.querySelector(c)); const leaf = v?.querySelector('.leaf:has(svg)'); if (!h || !s || !f || !w || !leaf) return null; return { width: v.offsetWidth, height: v.offsetHeight, header: h.offsetHeight, sheet: s.offsetHeight, footer: f.offsetHeight, label: getComputedStyle(w).display !== 'none', fits: leaf.scrollHeight <= leaf.clientHeight && leaf.clientWidth === leaf.offsetWidth }; })()`;
/** the reader's own score view (the viewer's lab, plain look, the full score) on a screen of that size; a touch screen
 *  = a coarse pointer (not `mobile`: that would widen the layout viewport to the lab page's own content) */
async function readerChrome(width, height, touch) {
	await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
	await send('Emulation.setTouchEmulationEnabled', { enabled: touch, maxTouchPoints: touch ? 5 : 1 });
	await send('Page.navigate', { url: `${VIEWER}/account/lab/score?score=cloud9-en&part=full&look=plain` });
	await until('document.readyState === "complete"');
	return until(chromeOf('body'), 40000);
}
const sameChrome = (a, b) => !!a && !!b && ['width', 'height', 'header', 'sheet', 'footer'].every((k) => Math.abs(a[k] - b[k]) <= 1) && a.label === b.label && a.fits && b.fits;
/** a share of bright / dark pixels inside the middle of a box (a screenshot of it, device px = CSS px) */
async function pixels(r) {
	const inset = 0.25;
	const clip = { x: r.x + r.w * inset, y: r.y + r.h * inset, width: Math.max(2, r.w * (1 - 2 * inset)), height: Math.max(2, r.h * (1 - 2 * inset)), scale: 1 };
	const shotData = await send('Page.captureScreenshot', { format: 'png', clip });
	const img = png(Buffer.from(shotData.result.data, 'base64'));
	let dark = 0, bright = 0, n = 0;
	for (let y = 0; y < img.h; y++)
		for (let x = 0; x < img.w; x++) {
			const i = (y * img.w + x) * img.ch;
			const l = 0.2126 * img.px[i] + 0.7152 * img.px[i + 1] + 0.0722 * img.px[i + 2];
			n++;
			if (l < 60) dark++;
			else if (l > 190) bright++;
		}
	return { dark: dark / n, bright: bright / n };
}

let saved = false;
try {
	await send('Runtime.enable');
	await send('Page.enable');
	// the reader's own score view first: what the stage's screens must equal
	const reader = { phone: await readerChrome(390, 844, true), desktop: await readerChrome(1440, 810, false) };
	await send('Emulation.setTouchEmulationEnabled', { enabled: false, maxTouchPoints: 1 });
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
	await go('/', 'true');
	await js(`localStorage.clear(); localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);

	// ── the catalog editor: Collage on a score item ──
	check(await go(`/catalog/items?id=${ITEM}`, `!!document.querySelector('[data-testid="catalog-detail"] input')`), `catalog editor: ${ITEM} opens in its form${before ? '' : ' (no document yet: the table\'s entry)'}`);
	await click(tid('phone-bar-more'));
	const collageButton = await until(`!!${q('catalog-collage')}`, 5000);
	check(collageButton, 'a score item has "Collage" under More');
	if (collageButton) await click(tid('catalog-collage'));
	else await go(`/catalog/items/${ITEM}/collage`, 'true');
	check(await until(`location.pathname === '/catalog/items/${ITEM}/collage' && !!${q('score-stage')}`, 20000), '"Collage" opens the stage');

	// ── the stage: the score in a phone screen ──
	const ready = await until(`!!document.querySelector('${FRAME} .leaf svg') && !${q('stage-add-piece')}.disabled`, 40000);
	const frame = await js(`(() => { const f = document.querySelector('${FRAME}'); const r = f.getBoundingClientRect(); return { w: f.offsetWidth, h: f.offsetHeight, shown: Math.round(r.width), screen: f.dataset.screen, crumbs: document.querySelector('nav[aria-label] ol, .crumbs')?.textContent ?? '' }; })()`);
	check(ready && frame.screen === 'phone' && frame.w === 390 && frame.h === 844 && frame.shown > 100, `the stage: the score live in a phone screen (390 × 844, shown ${frame.shown} px wide)`, JSON.stringify(frame));
	const phoneChrome = await until(chromeOf(FRAME), 10000);
	check(sameChrome(phoneChrome, reader.phone), `the phone screen's score view = the reader's on a phone (header / sheet / footer ${phoneChrome?.header} / ${phoneChrome?.sheet} / ${phoneChrome?.footer} px, Play label ${phoneChrome?.label ? 'shown' : 'hidden'}, the page fits its sheet)`, JSON.stringify({ stage: phoneChrome, reader: reader.phone }));
	await shot('phone');

	// ── a shape: added in the middle of the page shown, dragged onto bar 10, staff 1 ──
	const BAR = 10;
	check(await turnTo(BAR), `bar ${BAR} is on the page shown (page ${await shownPage()})`);
	await click(tid('stage-add-piece'));
	check(await until(`${pickerOpen} && !!document.querySelector('[data-testid^="collage-shape-"]')`, 20000), '"Add a piece …" opens the collage picker (Shapes)');
	await js(`${q('collage-shape-search')}.focus()`);
	await send('Input.insertText', { text: 'disc black' });
	await until(`!!${q('collage-shape-disc-black')}`, 5000);
	await click(tid('collage-shape-disc-black'));
	await until(`!${pickerOpen} && document.querySelectorAll('[data-testid="collage-list"] > li').length === 1 && !!${q('piece-inspector')}`, 10000);
	const shape = await selectedId();
	const added = shape && (await drawn(shape));
	check(added && added.selected && added.page === (await shownPage()), 'the shape lands on the page shown, drawn and selected, the inspector open', JSON.stringify(added));
	const target = await staffBox(BAR, 1);
	if (added && target) await drag([added.cx, added.cy], [target.x + target.w * 0.35, target.y + target.h / 2]);
	let at = await onStaff(shape, BAR, 1);
	check(at.ok && (await js(`${q('piece-bar')}?.value`)) === String(BAR), `a drag re-anchors it: on bar ${BAR}, staff 1 (its box data-bar ${BAR}, the inspector's bar ${BAR})`, at.detail);
	await press('z', 'ctrl');
	const undone = await js(`${q('piece-bar')}?.value`);
	await press('z', 'ctrl', 'shift');
	check(undone !== String(BAR) && (await js(`${q('piece-bar')}?.value`)) === String(BAR), `the drag is one undo step (undo: bar ${undone}, redo: bar ${BAR})`);

	// the look Negative: difference with a white fill – the paper under it reads black, the ink white
	await click(tid('piece-look-negative'));
	await sleep(400);
	const neg = await drawn(shape);
	const px = neg && (await pixels(neg));
	check(neg?.blend === 'difference' && px && px.dark > 0.4 && px.bright > 0.01, `Negative: difference reaches the paper (dark ${Math.round((px?.dark ?? 0) * 100)} %, bright ink ${Math.round((px?.bright ?? 0) * 100)} %)`, JSON.stringify({ blend: neg?.blend, px }));
	await shot('negative');

	// the corner handle scales it (never stretched), one undo step
	const small = await drawn(shape);
	const corner = await js(`(() => { const r = ${q('stage-piece-scale')}?.getBoundingClientRect(); return r ? [r.x + r.width / 2, r.y + r.height / 2] : null; })()`);
	if (small && corner) await drag(corner, [corner[0] + small.w / 4, corner[1] + small.h / 4]);
	const big = await drawn(shape);
	await press('z', 'ctrl');
	const again = await drawn(shape);
	check(!!big && Math.abs(big.w / small.w - 1.5) < 0.1 && Math.abs(big.w / big.h - small.w / small.h) < 0.02 && Math.abs(again.w - small.w) < 1 && big.bar === String(BAR), `the corner scales it evenly (× ${(big?.w / small?.w).toFixed(2)}), still on bar ${BAR}; Ctrl+Z takes it back`, JSON.stringify({ small, big, again }));

	// ── over the page's bottom edge (review fix 2026-10-18): the overlay clips to the page, so the leaf never scrolls (a
	//    scroll bar narrowed it, the pieces' frame with it – every piece off its note, unlike readers'); the selected
	//    piece's handle stays on the page; dragged back, the piece is on its note again ──
	const home = await drawn(shape);
	const pageBottom = await js(`document.querySelector('${FRAME} [data-testid="stage-piece-${shape}"]')?.closest('.score-overlay')?.getBoundingClientRect().bottom ?? null`);
	const grabAt = home && [home.cx, home.cy - home.h / 4];
	// its centre onto the page's bottom edge: half the piece hangs over it
	if (home && pageBottom) await drag(grabAt, [home.cx, pageBottom - home.h / 4]);
	const edge = await js(`(() => { const b = document.querySelector('${FRAME} [data-testid="stage-piece-${shape}"]'); const leaf = b?.closest('.leaf'); if (!leaf) return null; const r = (sel) => leaf.querySelector(sel)?.getBoundingClientRect(); const [l, page, box, h] = [leaf.getBoundingClientRect(), r('.score-overlay'), b.getBoundingClientRect(), document.querySelector('[data-testid="stage-piece-scale"]')?.getBoundingClientRect()]; return { over: box.bottom - page.bottom, pastLeaf: box.bottom - l.bottom, scroll: leaf.scrollHeight - leaf.clientHeight, bar: leaf.offsetWidth - leaf.clientWidth, svg: r(':scope > svg').width, pieces: r('.score-pieces').width, overlay: page.width, handle: !!h && h.left >= page.left - 0.5 && h.right <= page.right + 0.5 && h.top >= page.top - 0.5 && h.bottom <= page.bottom + 0.5 }; })()`);
	check(edge && edge.over > home.h / 3 && edge.pastLeaf > 1 && edge.scroll <= 0 && edge.bar === 0 && Math.abs(edge.pieces - edge.svg) < 0.5 && Math.abs(edge.overlay - edge.svg) < 0.5 && edge.handle, `a piece ${Math.round(edge?.over ?? 0)} px over the page's bottom edge: the leaf does not scroll (no scroll bar), the pieces' layer and the overlay as wide as the music, the handle on the page`, JSON.stringify(edge));
	await shot('edge');
	// the handle held on the page scales from where it is (its offset from the box's corner kept: no jump), one undo step
	const held = await js(`(() => { const r = ${q('stage-piece-scale')}?.getBoundingClientRect(); return r ? [r.x + r.width / 2, r.y + r.height / 2] : null; })()`);
	const over = await drawn(shape);
	if (held && over) await drag(held, [held[0] + over.w / 8, held[1]]);
	const grown = await drawn(shape);
	const expect = over ? Math.hypot((over.w * 5) / 8, over.h / 2) / Math.hypot(over.w / 2, over.h / 2) : 0;
	await press('z', 'ctrl');
	const ungrown = await drawn(shape);
	check(!!grown && Math.abs(grown.w / over.w - expect) < 0.03 && Math.abs(ungrown.w - over.w) < 0.5, `the handle held on the page scales from where it is (× ${(grown?.w / over?.w).toFixed(2)}, its corner's × ${expect.toFixed(2)}); Ctrl+Z takes it back`, JSON.stringify({ held, over, grown, ungrown }));
	if (over && grabAt) await drag([over.cx, over.cy - over.h / 4], grabAt);
	const back = await drawn(shape);
	at = await onStaff(shape, BAR, 1);
	const off = back && home ? Math.hypot(back.cx - home.cx, back.cy - home.cy) : Infinity;
	check(at.ok && off < 1.5 && Math.abs(back.w - home.w) < 0.5, `dragged back: on bar ${BAR}, staff 1, where it was (${off.toFixed(2)} px off, as wide)`, `${at.detail} ${JSON.stringify({ home, back })}`);
	// a drag held still half-way for 1.5 s (past doc.edit's 1.2 s key window) is still one undo step (review fix 2026-10-18)
	if (back) await drag([back.cx, back.cy], [back.cx + back.w, back.cy], 1500);
	const paused = await drawn(shape);
	await press('z', 'ctrl');
	const unpaused = await drawn(shape);
	const left = back && unpaused ? Math.hypot(unpaused.cx - back.cx, unpaused.cy - back.cy) : Infinity;
	check(!!paused && paused.cx - back.cx > back.w / 2 && left < 1.5, `a drag that pauses 1.5 s is one undo step (one Ctrl+Z: ${left.toFixed(2)} px from its start)`, JSON.stringify({ back, paused, unpaused }));

	// ── Phone → Desktop → zoom + twice: still on bar 10, staff 1 ──
	await click(tid('stage-desktop'));
	await until(`document.querySelector('${FRAME}')?.dataset.screen === 'desktop'`, 5000);
	await settled(shape, BAR, 1);
	const desk = await js(`(() => { const f = document.querySelector('${FRAME}'); return [f.offsetWidth, f.offsetHeight]; })()`);
	at = await onStaff(shape, BAR, 1);
	check(at.ok && desk[0] === 1440 && desk[1] === 810, `Desktop (1440 × 810): still on bar ${BAR}, staff 1`, at.detail);
	const deskChrome = await until(chromeOf(FRAME), 10000);
	check(sameChrome(deskChrome, reader.desktop), `the desktop screen's score view = the reader's on a desktop (header / sheet / footer ${deskChrome?.header} / ${deskChrome?.sheet} / ${deskChrome?.footer} px, Play label ${deskChrome?.label ? 'shown' : 'hidden'}, the page fits its sheet)`, JSON.stringify({ stage: deskChrome, reader: reader.desktop }));
	await shot('desktop');
	for (const zoom of [45, 50]) {
		await click(`${FRAME} [data-testid="score-zoom-in"]`);
		await sleep(300);
		await settled(shape, BAR, 1);
		at = await onStaff(shape, BAR, 1);
		check(at.ok, `Desktop, zoom ${zoom}: still on bar ${BAR}, staff 1`, at.detail);
	}
	await shot('desktop-50');

	// ── a character picture onto bar 12, staff 2 – on the phone at zoom 30 and 35 ──
	const BAR2 = 12;
	await click(tid('stage-phone'));
	await until(`document.querySelector('${FRAME}')?.dataset.screen === 'phone'`, 5000);
	await settled(shape, BAR, 1);
	check(await turnTo(BAR2), `bar ${BAR2} is on the page shown`);
	await click(tid('stage-add-piece'));
	await until(pickerOpen, 10000);
	await click(tid('collage-tab-characters'));
	const card = await until(`document.querySelector('[data-testid="collage-picker-sheet"] [data-testid^="picker-character-"] img') ? document.querySelector('[data-testid="collage-picker-sheet"] [data-testid^="picker-character-"]').dataset.testid : null`, 15000);
	if (card) await click(tid(card));
	const mood = await until(`document.querySelector('[data-testid^="picker-mood-"]:not(.is-missing)')?.dataset.testid ?? null`, 15000);
	if (mood) await click(tid(mood));
	await until(`!${pickerOpen} && document.querySelectorAll('[data-testid="collage-list"] > li').length === 2`, 15000);
	const figure = await selectedId();
	const fig = figure && figure !== shape ? await until(`(() => { const p = document.querySelector('${FRAME} .score-pieces .piece[data-piece="${figure}"]'); return p ? true : null; })()`, 10000) : null;
	const figDrawn = fig && (await drawn(figure));
	check(figDrawn && /characters\//.test(figDrawn.src) && !!(await js(`!!${q('piece-picture')}`)), `a character picture (${card?.replace('picker-character-', '')} · ${mood?.replace('picker-mood-', '')}) lands on the page shown, its picture row in the inspector`, JSON.stringify(figDrawn));
	const target2 = await staffBox(BAR2, 2);
	if (figDrawn && target2) await drag([figDrawn.cx, figDrawn.cy], [target2.x + target2.w * 0.6, target2.y + target2.h / 2]);
	at = await onStaff(figure, BAR2, 2);
	check(at.ok, `dragged onto bar ${BAR2}, staff 2 (zoom 30)`, at.detail);
	await click(`${FRAME} [data-testid="score-zoom-in"]`);
	await sleep(300);
	await settled(figure, BAR2, 2);
	at = await onStaff(figure, BAR2, 2);
	check(at.ok, `zoom 35: still on bar ${BAR2}, staff 2`, at.detail);
	await click(`${FRAME} [data-testid="score-zoom-out"]`);
	await settled(figure, BAR2, 2);
	await shot('character');

	// Behind: under the notes; Page keeps it where it is (and back to the bar)
	await js(`(() => { const d = ${q('piece-advanced')}; if (d && !d.open) d.open = true; return true; })()`);
	await click(tid('piece-behind'));
	await sleep(300);
	const behind = await drawn(figure);
	const music = await js(`getComputedStyle(document.querySelector('${FRAME} .leaf > svg')).zIndex`);
	check(behind?.z === '0' && behind.behind && music === '1', `Behind: under the notes (piece z ${behind?.z}, the music z ${music}, its box dotted)`);
	const was = await drawn(figure);
	await click(tid('piece-at-page'));
	await sleep(400);
	const paged = await drawn(figure);
	await click(tid('piece-at-bar'));
	await sleep(400);
	const barred = await drawn(figure);
	const same = (a, b) => !!a && !!b && Math.abs(a.cx - b.cx) < 1 && Math.abs(a.cy - b.cy) < 1 && Math.abs(a.w - b.w) < 1;
	check(same(was, paged) && same(was, barred) && paged.bar === String(BAR2) && barred.bar === String(BAR2), 'Page keeps the piece where it is on its page, and back to its bar', JSON.stringify({ was, paged, barred }));

	// a bar the file lacks: the list warns, readers skip it (no box, nothing drawn); back to bar 12
	await typeInto('piece-bar', '999');
	const lost = await until(`!!${q(`collage-missing-${figure}`)} && !document.querySelector('${FRAME} .score-pieces .piece[data-piece="${figure}"]')`, 5000);
	await typeInto('piece-bar', String(BAR2));
	await settled(figure, BAR2, 2);
	check(lost && !(await js(`!!${q(`collage-missing-${figure}`)}`)) && (await onStaff(figure, BAR2, 2)).ok, `bar 999 (not in the file): ⚠ in the list, not drawn; bar ${BAR2} again: back on staff 2`);

	// ── save, reload ──
	await click(tid('save-draft'));
	await until(`!!document.querySelector('dialog.sc-dialog[open] input, dialog.sc-dialog[open] textarea')`, 5000);
	await js(`(() => { const f = document.querySelector('dialog.sc-dialog[open] input, dialog.sc-dialog[open] textarea'); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, ${JSON.stringify(NOTE)}); f.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await sleep(200);
	saved = true;
	await js(`document.querySelector('dialog.sc-dialog[open] button.primary').click(), true`);
	check(await until(`!${q('save-draft')}?.textContent.includes('•') ? true : null`, 15000), 'Save: the draft stored');
	const stored = await draftOf();
	const sheet = stored?.content?.score?.collage;
	const byId = (id) => sheet?.pieces?.find((p) => p.id === id);
	check(
		sheet?.file === MUSICXML && sheet.pieces?.length === 2 && byId(shape)?.src === 'catpope/vectors/disc-black.svg' && byId(shape)?.at?.bar === String(BAR) && byId(shape)?.preset === 'negative' && /^characters\//.test(byId(figure)?.src ?? '') && !!byId(figure)?.figure && byId(figure)?.at?.bar === String(BAR2) && byId(figure)?.at?.staff === 2 && byId(figure)?.behind === true && !('height' in (byId(shape)?.rect ?? {})),
		`the item document: score.collage { file: the score's MusicXML, 2 pieces – CDN-root paths, bars ${BAR} / ${BAR2}, the look, the figure, behind }`,
		JSON.stringify(sheet)?.slice(0, 600)
	);
	await go(`/catalog/items/${ITEM}/collage`, `document.querySelectorAll('[data-testid="collage-list"] > li').length === 2 && !!document.querySelector('${FRAME} .leaf svg')`);
	await click(tid(`collage-select-${shape}`));
	await settled(shape, BAR, 1);
	const back1 = await onStaff(shape, BAR, 1);
	await click(tid(`collage-select-${figure}`));
	await settled(figure, BAR2, 2);
	const back2 = await onStaff(figure, BAR2, 2);
	check(back1.ok && back2.ok, `a reload: both pieces at their bars (a click in the list turns to the page)`, `${back1.detail} ${back2.detail}`);

	// ── Delete + Ctrl+Z, Esc – on the live region only (review fix 2026-10-18): with the focus outside the stage (a link
	//    of the creator's sidebar) Backspace / Delete / Esc leave the piece ──
	await click(tid(`collage-select-${shape}`));
	await settled(shape, BAR, 1);
	const live = await js(`${q('score-stage')}.hasAttribute('data-live') && !!${q(`collage-row-${shape}`)}?.matches('[data-live] .sc-selected')`);
	const outside = await js(`(() => { const a = [...document.querySelectorAll('a[href]')].find((el) => !el.closest('[data-testid="score-stage"]') && el.offsetParent); a?.focus(); return a && document.activeElement === a ? a.textContent.trim() || a.getAttribute('href') : null; })()`);
	for (const key of ['Backspace', 'Delete', 'Escape']) await press(key);
	const kept = { rows: await rows(), selected: await selectedId(), path: await js('location.pathname') };
	check(live && outside && kept.rows.length === 2 && kept.selected === shape && kept.path === `/catalog/items/${ITEM}/collage`, `the stage is the live region (its list row in the live look); the focus on "${outside}" outside it: Backspace / Delete / Esc leave the piece`, JSON.stringify({ live, outside, kept }));
	await js(`document.activeElement?.blur(), true`);
	await press('Delete');
	const afterDelete = await rows();
	await press('z', 'ctrl');
	const afterUndo = await rows();
	check(afterDelete.length === 1 && !afterDelete.includes(shape) && afterUndo.length === 2 && afterUndo.includes(shape), 'Delete removes the selected piece, Ctrl+Z brings it back', JSON.stringify({ afterDelete, afterUndo }));
	await click(tid(`collage-select-${figure}`));
	await press('Escape');
	check(!(await js(`!!${q('piece-inspector')}`)), 'Esc lets the piece go (no inspector)');

	// ── a phone-sized creator: the side region below the stage ──
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 1, mobile: true });
	await go(`/catalog/items/${ITEM}/collage`, `!!document.querySelector('${FRAME} .leaf svg')`);
	const narrow = await js(`(() => { const s = ${q('score-stage')}.querySelector('.main').getBoundingClientRect(); const a = ${q('score-stage-side')}.getBoundingClientRect(); const f = document.querySelector('${FRAME}').getBoundingClientRect(); return { mainBottom: Math.round(s.bottom), sideTop: Math.round(a.top), frame: Math.round(f.width), scroll: document.documentElement.scrollWidth }; })()`);
	check(narrow.sideTop >= narrow.mainBottom - 1 && narrow.frame > 50 && narrow.scroll <= 376, '375 × 812: the score screen on top, the side region below it, no sideways scroll', JSON.stringify(narrow));
	await shot('narrow');

	if (errors.length) check(false, `page errors: ${errors.length}`, errors.slice(0, 3).join(' | '));
} catch (e) {
	check(false, 'run', e?.stack ?? String(e));
} finally {
	ws.close();
	chrome.kill();
	if (saved) {
		const removed = sweep();
		const after = await draftOf();
		check(removed >= 1 && (after?.version.id ?? null) === (before?.version.id ?? null), `restored ${KEY}: ${removed} draft version(s) removed, newest ${after?.version.id ?? 'none'} (was ${before?.version.id ?? 'none'})`);
	}
}
const passed = results.filter(Boolean).length;
console.log(`\n${passed}/${results.length} passed`);
process.exit(passed === results.length ? 0 : 1);
