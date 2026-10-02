#!/usr/bin/env node
// Check of the creator's cover editor (`/<toon>/cover`, agents/DESIGN.md "Cover editor", agents/DATA.md "Cover lettering
// layout"): headless Chrome (own throw-away profile, CDP, no dependencies) signed in as the local test user
// (agent@scutoons.test, `npm run dev:test-user`) on the running dev stack (creator :10000, API :9000, CDN :8001 – or
// SCUTOONS_API / SCUTOONS_CREATOR for servers on other ports).
//
//   npm run check:creator-cover [-- --shots=<dir>]
//
// On the sandbox toon (de, en, ko), with the text-free catpope art when the CDN has it: lettering off → no text on the
// art; "Default" → the two default lines; the preview language switches the texts (de / en / ko titles from the text
// fields); dragging a line moves it and turns the default into an own layout; the corner handle makes it bigger, the
// side handle narrower; a long translation shrinks into its box (every language "as readers see it"); an own line with a
// text per language (Korean too); Save draft stores a valid layout (`validateFile`) with the texts NOT in the layout's
// geometry. Afterwards toon:sandbox is as before (restored also on failure). Refuses to run while toon:sandbox has an
// unpublished draft.
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';
import { validateFile } from '../../shared/content/src/index.ts';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const CDN = (process.env.SCUTOONS_CDN ?? local('cdn')).replace(/\/$/, '');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const TEXTLESS = '../catpope/images/s1-cover-square-textless.webp';
const LONG = { de: 'Die unglaublich lange Geschichte vom Erwachen', en: 'Season one', ko: '시즌 1' };
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const shots = opt.shots ? String(opt.shots) : null;
// the sandbox has three languages (de, en, ko); --toon=catpope e.g. for screenshots on the real toon (de, en)
const TOON = String(opt.toon || 'sandbox');
const KEY = `toon:${TOON}`;
if (shots) mkdirSync(shots, { recursive: true });

const results = [];
const check = (ok, label, detail = '') => {
	results.push(ok);
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${ok || !detail ? '' : `  — ${detail}`}`);
	return ok;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── session of the test user ─────────────────────────────────────────────────────────────────────────────
execFileSync(process.execPath, [resolve(ROOT, 'scripts/dev/test-user.mjs')], { stdio: ['ignore', 'ignore', 'inherit'], env: { ...process.env, SCUTOONS_API: API } });
const token = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8')).token;

async function api(method, path, body) {
	const res = await fetch(API + path, {
		method,
		headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path} → ${res.status} ${JSON.stringify(json)?.slice(0, 200)}`);
	return json;
}

// ── baseline: the published toon entry, no pending draft ───────────────────────────────────────────────────
const latest = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
if (!latest.published || latest.published.id !== latest.version.id) {
	console.error(`creator-cover-check: ${KEY} has an unpublished draft or none published; not touching it. Publish or restore it first.`);
	process.exit(2);
}
const baseline = latest.content;
const LOCALES = baseline.languages?.length ? baseline.languages : ['de', 'en'];
const titles = {};
for (const l of LOCALES) {
	titles[l] = {
		toon: (await api('GET', `/entities/toon-settings:${TOON}:${l}`)).title,
		season: (await api('GET', `/entities/season:${TOON}:1:${l}`)).title
	};
}
// the text-free art when the CDN has it (owner 2026-10-02), else the toon's own cover
const textless = (await fetch(`${CDN}/catpope/images/s1-cover-square-textless.webp`, { method: 'HEAD' }).catch(() => null))?.ok;
const square = textless ? TEXTLESS : baseline.cover?.square;
check(!!square, `the art: ${square}${textless ? ' (text-free)' : ''}`);
// start from a draft: the art, lettering off
await api('PUT', `/entities/${KEY}`, { content: { ...baseline, cover: { square } }, base: latest.version.id, message: 'creator-cover-check: start' });

async function restore() {
	const now = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	if (JSON.stringify(now.content) !== JSON.stringify(baseline)) {
		const put = await api('PUT', `/entities/${KEY}`, { content: baseline, base: now.version.id, message: 'creator-cover-check: restore' });
		await api('POST', `/entities/${KEY}/publish`, { version: put.version.id });
	} else if (now.published?.id !== now.version.id) await api('POST', `/entities/${KEY}/publish`, { version: now.version.id });
	const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	check(after.published?.id === after.version.id && JSON.stringify(after.content) === JSON.stringify(baseline), `restored ${KEY} (published content as before, no pending draft)`);
}

// ── headless Chrome over CDP ──────────────────────────────────────────────────────────────────────────────
const CHROME =
	process.env.CHROME ||
	[
		'C:/Program Files/Google/Chrome/Application/chrome.exe',
		'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
		'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
		'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
		'/usr/bin/google-chrome'
	].find((p) => existsSync(p));
if (!CHROME) {
	console.error('creator-cover-check: no Chrome/Edge found (set CHROME)');
	process.exit(2);
}
const port = 9600 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-cover-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=1440,1000', 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => {
	try {
		chrome.kill();
	} catch {}
	try {
		rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
	} catch {}
});

let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try {
		wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl;
	} catch {}
	if (!wsUrl) await sleep(200);
}
if (!wsUrl) throw new Error('chrome did not start');
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
const errors = [];
ws.addEventListener('message', (e) => {
	const msg = JSON.parse(e.data);
	if (msg.id && pending.has(msg.id)) {
		pending.get(msg.id)(msg);
		pending.delete(msg.id);
		return;
	}
	if (msg.method === 'Page.javascriptDialogOpening') send('Page.handleJavaScriptDialog', { accept: true });
	else if (msg.method === 'Runtime.exceptionThrown') errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
});
const send = (method, params = {}) =>
	new Promise((r) => {
		const i = ++seq;
		pending.set(i, r);
		ws.send(JSON.stringify({ id: i, method, params }));
	});
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(`${expression.slice(0, 80)}: ${r.result.exceptionDetails.exception?.description}`);
	return r.result?.result?.value;
}
async function until(expression, ms = 15000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		if (await js(expression).catch(() => false)) return true;
		await sleep(150);
	}
	return false;
}
const q = (testid) => `document.querySelector('[data-testid="${testid}"]')`;
const centre = (selector) => js(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
async function mouse(type, x, y) {
	await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1 });
}
async function click(selector) {
	const [x, y] = await centre(selector);
	await mouse('mouseMoved', x, y);
	await mouse('mousePressed', x, y);
	await mouse('mouseReleased', x, y);
	await sleep(250);
}
/** a real mouse drag from the element's centre by (dx, dy), in steps */
async function drag(selector, dx, dy) {
	const [x, y] = await centre(selector);
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await mouse('mousePressed', x, y);
	for (let i = 1; i <= 8; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + (dx * i) / 8, y: y + (dy * i) / 8, button: 'left', buttons: 1 });
	await mouse('mouseReleased', x + dx, y + dy);
	await sleep(300);
}
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
const num = (testid) => js(`Number(${q(testid)}?.value)`);
/** the stage's lettering lines: text, and whether each fits its box */
const stageLines = (root = q('cover-stage')) =>
	js(`[...${root}.querySelectorAll('.line')].map((l) => { const t = l.querySelector('.text'); return { text: t.textContent, fits: t.offsetWidth <= l.clientWidth + 1, fit: Number(getComputedStyle(t).getPropertyValue('--fit') || 1), lang: l.closest('[lang]')?.lang }; })`);

await send('Runtime.enable');
await send('Page.enable');

let failed = false;
try {
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}); localStorage.setItem('scutoons_creator_text_language', 'all')`);
	// the toon settings link to the editor (the state of the lettering instead of the old checkbox)
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/toon` });
	check(await until(`/Off|Aus/.test(${q('toon-cover-lettering')}?.innerText ?? '')`), 'toon settings: "Lettering: Off" + the link to the cover editor');
	await shot('0-toon-settings');
	await click('[data-testid="toon-cover-editor"]');
	check(await until(`location.pathname === '/${TOON}/cover'`), 'the link opens /<toon>/cover');
	check(await until(`!!${q('cover-stage')} && ${q('cover-stage')}.querySelector('img')?.complete && ${q('cover-stage')}.querySelector('img')?.naturalWidth > 0`), 'the cover page shows the art');
	check(await js(`!!document.querySelector('[data-testid="nav-cover"][aria-current="page"]')`), 'the sidebar marks "Cover"');

	// 1. off → no lettering; Default → the two default lines
	check(await js(`${q('cover-mode-off')}.getAttribute('aria-pressed') === 'true' && !${q('cover-stage')}.querySelector('.line')`), 'lettering off: the art without text');
	await shot('1-off');
	await click('[data-testid="cover-mode-default"]');
	check(await until(`${q('cover-stage')}.querySelectorAll('.line').length === 2`), 'Default: two lines over the art');
	let lines = await stageLines();
	check(lines[0]?.text === titles.de.toon && lines[1]?.text === `"${titles.de.season}"`, `de: "${lines[0]?.text}" over ${lines[1]?.text} (from the text fields)`, JSON.stringify(lines));
	check(await until('!!document.querySelector(".dirty-bar")'), 'the page is dirty');
	await shot('2-default-de');

	// 2. the preview language
	for (const l of [...LOCALES.slice(1), LOCALES[0]]) {
		await click(`[data-testid="cover-lang-${l}"]`);
		await until(`${q('cover-stage')}.querySelector('.line .text')?.textContent === ${JSON.stringify(titles[l].toon)}`, 3000);
		lines = await stageLines();
		check(lines[0]?.text === titles[l].toon && lines[1]?.text === `"${titles[l].season}"` && lines[0]?.lang === l, `preview ${l}: ${lines.map((x) => x.text).join(' / ')}`, JSON.stringify(lines));
		await shot(`3-default-${l}`);
	}

	// 3. drag the season line: it moves, the default becomes an own layout
	await centre('[data-testid="cover-box-1"]');
	const before = await js(`(() => { const r = ${q('cover-box-1')}.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	await drag('[data-testid="cover-box-1"]', 40, 90);
	const after = await js(`(() => { const r = ${q('cover-box-1')}.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	check(Math.abs(after[0] - before[0] - 40) < 4 && Math.abs(after[1] - before[1] - 90) < 4, `drag: the line moved by (${(after[0] - before[0]).toFixed(0)}, ${(after[1] - before[1]).toFixed(0)}) px`);
	check(await js(`${q('cover-mode-custom')}.getAttribute('aria-pressed') === 'true'`), 'dragging made the default an own layout');
	const y = await num('cover-y');
	check(y > 31, `the inspector shows the selected line: y ${y} %`);
	// the lettering itself follows the box
	const textCentre = await js(`(() => { const r = ${q('cover-stage')}.querySelector('[data-line="1"]').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	check(Math.abs(textCentre[0] - after[0]) < 3 && Math.abs(textCentre[1] - after[1]) < 3, 'the rendered line sits in its box');

	// 4. the corner handle makes it bigger, the side handle narrower
	const size0 = await num('cover-size');
	await drag('[data-testid="cover-handle-size"]', 40, 20);
	const size1 = await num('cover-size');
	check(size1 > size0, `size handle: ${size0} → ${size1}`);
	const width0 = await num('cover-maxWidth');
	await drag('[data-testid="cover-handle-width"]', -80, 0);
	const width1 = await num('cover-maxWidth');
	check(width1 < width0, `width handle: ${width0} → ${width1} %`);
	lines = await stageLines();
	check(lines.every((x) => x.fits), `every line fits its box (fit ${lines.map((x) => x.fit.toFixed(2)).join(', ')})`, JSON.stringify(lines));
	await shot('4-dragged');

	// 5. an own line with a text per language (all languages shown: i18n "all", set before the page loaded)
	await click('[data-testid="cover-add-line"]');
	check(await until(`!!${q(`cover-custom-${LOCALES.at(-1)}`)}`), 'Add line: an own text, one field per language');
	for (const l of LOCALES) {
		if (!LONG[l]) continue;
		await js(`(() => { const el = ${q(`cover-custom-${l}`)}; el.focus(); el.value = ${JSON.stringify(LONG[l])}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`);
	}
	await sleep(400);
	for (const l of LOCALES) {
		const shown = await js(`[...${q(`cover-reader-${l}`)}.querySelectorAll('.line .text')].map((t) => t.textContent)`);
		if (LONG[l]) check(shown.includes(LONG[l]), `as readers see it, ${l}: ${shown.join(' / ')}`);
		const fits = await stageLines(q(`cover-reader-${l}`));
		check(fits.every((x) => x.fits), `${l}: every line fits its box (fit ${fits.map((x) => x.fit.toFixed(2)).join(', ')})`, JSON.stringify(fits));
	}
	const longFit = (await stageLines(q('cover-reader-de'))).find((x) => x.text === LONG.de)?.fit ?? 1;
	const shortFit = (await stageLines(q('cover-reader-en'))).find((x) => x.text === LONG.en)?.fit ?? 1;
	check(longFit < shortFit, `the long German line shrinks (${longFit.toFixed(2)}), the short English one does not (${shortFit.toFixed(2)})`, JSON.stringify(await stageLines(q('cover-reader-en'))));
	await shot('5-own-line');
	await js(`${q('cover-readers')}.scrollIntoView({ block: 'center' })`);
	await shot('6-readers');

	// 6. Save draft → a valid layout in the store, texts only in the custom line
	await click('[data-testid="save-draft"]');
	await until(`!document.querySelector('.dirty-bar')`);
	const stored = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	const layout = stored.content.cover?.lettering?.square;
	check(Array.isArray(layout) && layout.length === 3 && layout[0].text === 'toon' && layout[1].text === 'season' && layout[2].text === 'custom', 'Save draft: cover.lettering.square = 3 lines (toon, season, own)', JSON.stringify(stored.content.cover));
	check(JSON.stringify(layout?.[2]?.custom) === JSON.stringify(Object.fromEntries(LOCALES.filter((l) => LONG[l]).map((l) => [l, LONG[l]]))), 'the own line keeps its text per language');
	check(!JSON.stringify(layout?.slice(0, 2)).includes(titles.de.season), 'the title lines carry no text (it comes from the text fields)');
	const problems = validateFile('index', stored.content, 'de', { file: KEY });
	check(!problems.some((p) => p.level === 'error' && p.path[0] === 'cover'), 'the stored layout validates', JSON.stringify(problems));
	check(stored.content.cover.square === square, 'the art is the same one path for every language');

	// 7. Publish → the reader's toon page (phone) sets the layout in its language (when the viewer runs)
	const viewerUp = (await fetch(VIEWER).catch(() => null))?.ok;
	if (viewerUp) {
		await click('[data-testid="publish"]');
		await until(`/Published|Veröffentlicht/.test(${q('file-status')}?.innerText ?? '')`);
		const live = await api('GET', `/entities/${KEY}`);
		check(Array.isArray(live.cover?.lettering?.square), 'Publish: the layout is live');
		await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
		for (const l of LOCALES.filter((x) => x === 'de' || x === 'en')) {
			await send('Page.navigate', { url: `${VIEWER}/${l}/${TOON}` });
			const shown = await until(`document.querySelectorAll('.toon-header [data-testid="cover-lettering"] .line').length === 3`, 20000);
			await sleep(500);
			const lines = shown ? await stageLines(`document.querySelector('.toon-header [data-testid="cover-art"]')`) : [];
			check(shown && lines[0]?.text === titles[l].toon && lines[1]?.text === `"${titles[l].season}"` && lines[2]?.text === LONG[l] && lines.every((x) => x.fits && x.lang === l),
				`reader ${l} (390 px): ${lines.map((x) => `${x.text} (${x.fit.toFixed(2)})`).join(' / ')}`, JSON.stringify(lines));
			await shot(`7-reader-${l}`);
		}
		await send('Emulation.clearDeviceMetricsOverride');
		await send('Page.navigate', { url: `${CREATOR}/${TOON}/cover` });
		await until(`!!${q('cover-stage')}`);
	} else console.log(`(the reader at ${VIEWER} is not running: its toon page is not checked)`);

	// 8. back to off
	await click('[data-testid="cover-mode-off"]');
	check(await until(`!${q('cover-stage')}.querySelector('.line')`), 'Off: the lettering is gone again');
	check(errors.length === 0, 'no page errors', errors.join(' | '));
} catch (e) {
	failed = true;
	check(false, 'run', e?.stack ?? String(e));
} finally {
	await js(`localStorage.removeItem('scutoons_creator_text_language')`).catch(() => {});
	await restore().catch((e) => check(false, 'restore', String(e)));
}
const bad = results.filter((r) => !r).length;
console.log(bad || failed ? `creator-cover-check: ${bad} failed` : `creator-cover-check: all ${results.length} passed`);
process.exit(bad || failed ? 1 : 0);
