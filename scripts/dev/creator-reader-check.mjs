#!/usr/bin/env node
// The creator's dock Reader and the vertical strip preview (BACKLOG 10.49, UI-CONCEPTS E.6, agents/DOCKING.md "The Reader
// panel") in headless Chrome (own profile, removed on exit) as the local test user:
//   - a vertical group's strip preview: the slices stacked in one reader screen (no sideways track);
//   - the Reader opens in the dock (the viewer's draft preview in a frame, signed in by the shared cookie), says "ready";
//   - group → group, slice and chapter → chapter (another toon too) keep the SAME panel and frame (no remount, no reload),
//     the reader scrolls to the selected group / slice;
//   - a saved draft reloads the frame: a draft of a sandbox chapter is saved through the editor's Save (note
//     "reader-check (removed again)") and its version rows deleted afterwards (the dev DB as before; an interrupted run's
//     rows are swept first);
//   - the viewer ignores a well-formed "show" from another origin (a page on the CDN's origin framing the viewer).
//   node scripts/dev/creator-reader-check.mjs        (creator :10000, viewer :8000, API :9000, cdn :8001, MariaDB container)
//   SCUTOONS_READER_TOON / _CHAPTER (default toptoy 1: vertical groups), SCUTOONS_READER_SAVE (default sandbox 2)
//   READER_SHOTS=<dir>: e6-reader-<toon>-ch<n>.png (1440, the Reader open) + e6-strip-vertical.png
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const OTHER = local('cdn'); // another origin of the same site (the attacker's page in the last step)
for (const url of [CREATOR, VIEWER, API]) {
	if (!['localhost', '127.0.0.1'].includes(new URL(url).hostname)) {
		console.error(`creator-reader-check: refusing ${url} – local dev stack only`);
		process.exit(2);
	}
}
const TOON = process.env.SCUTOONS_READER_TOON ?? 'toptoy';
const CHAPTER = Number(process.env.SCUTOONS_READER_CHAPTER ?? 1);
const [SAVE_TOON, SAVE_CHAPTER] = (process.env.SCUTOONS_READER_SAVE ?? 'sandbox 2').split(' ');
const SAVE_KEY = `chapter:${SAVE_TOON}:${SAVE_CHAPTER}`;
const NOTE = 'reader-check (removed again)';
const SHOTS = process.env.READER_SHOTS;
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

// ── API + DB (the save step's restore) ──
async function api(method, path) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}` } });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
const sweep = () => {
	const ids = sql(`SELECT id FROM entity_versions WHERE message = ${quote(NOTE)}`).split('\n').filter(Boolean).map(Number);
	if (ids.length) sql(`DELETE FROM entity_versions WHERE id IN (${ids.join(',')})`);
	return ids.length;
};
const swept = sweep();
if (swept) console.log(`note an earlier run's leftovers removed: ${swept} draft version(s)`);

// ── Chrome ──
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9750 + Math.floor(Math.random() * 150);
const profile = tempProfile('creator-reader-'); // removed on exit (chrome-profile.mjs)
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=1440,1000', 'about:blank'], { stdio: 'ignore' });
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try { wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl; } catch {}
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
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
async function js(expression, contextId) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, ...(contextId ? { contextId } : {}) });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
	if (r.error) throw new Error(r.error.message);
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
/** an expression in the viewer's frame (an isolated world: the frame's DOM, location and scroll; made anew each time) */
async function inFrame(expression) {
	const { result } = await send('Page.getFrameTree');
	const child = (result?.frameTree?.childFrames ?? []).map((c) => c.frame).find((f) => f.url.startsWith(VIEWER));
	if (!child) return null;
	const world = await send('Page.createIsolatedWorld', { frameId: child.id, worldName: 'reader-check' });
	const contextId = world.result?.executionContextId;
	return contextId ? js(expression, contextId).catch(() => null) : null;
}
async function untilFrame(expression, ms = 20000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await inFrame(expression);
		if (v) return v;
		await sleep(200);
	}
	return null;
}
const q = (id) => `document.querySelector('[data-testid="${id}"]')`;
const go = async (url, ready) => {
	await send('Page.navigate', { url: url.startsWith('http') ? url : `${CREATOR}${url}` });
	await until('document.readyState === "complete"');
	return until(ready, 30000);
};
/** client-side navigation (SvelteKit intercepts the click), like a crumb or a sidebar link */
const nav = (href) => js(`(() => { const a = document.createElement('a'); a.href = ${JSON.stringify(href)}; document.body.append(a); a.click(); a.remove(); return true; })()`);
async function shot(name) {
	if (!SHOTS) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(SHOTS, name), Buffer.from(r.result.data, 'base64'));
	console.log(`     shot ${name}`);
}
/** the dock's Reader: the same element as tagged, its mount counter, its loads, ready */
const dock = () =>
	js(`(() => { const d = ${q('reader-dock')}; const f = ${q('reader-frame')}; return { here: !!d, same: !!d && d === window.__readerDock, frameSame: !!f && f === window.__readerFrame, mount: d?.dataset.mount ?? null, loads: Number(d?.dataset.loads ?? 0), ready: d?.dataset.ready === 'true' }; })()`);
/** the viewer: where it is, and how far the page slice `index` is from the top of its screen */
const viewerAt = (index) =>
	inFrame(`(() => { const el = document.querySelector('.chapter-view [data-slice-index="${index}"]'); return { path: location.pathname, search: location.search, top: el ? Math.round(el.getBoundingClientRect().top) : null, y: Math.round(scrollY), shows: Number(document.documentElement.dataset.creatorShows ?? 0), mark: document.documentElement.dataset.readerCheck ?? null }; })()`);
/** waits until the slice `index` is at the top of the viewer's screen (a strip's later slice: scrolled into its travel) */
const viewerScrolledTo = (index) => untilFrame(`(() => { const el = document.querySelector('.chapter-view [data-slice-index="${index}"]'); if (!el) return null; const strip = el.closest('.strip'); const top = strip ? el.closest('.frame').getBoundingClientRect().left : el.getBoundingClientRect().top; return Math.abs(top) < 3 && scrollY > 0 ? true : null; })()`, 15000);

/** the page slice `index` at the top: the page's first slice = the page's very top */
const viewerShows = (index) => (index === 0 ? untilFrame(`scrollY === 0 && document.querySelector('.chapter-view [data-slice-index="0"]') ? true : null`, 15000) : viewerScrolledTo(index));

let restoreNeeded = false;
let saveBefore = null;
try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1000, deviceScaleFactor: 1, mobile: false });
	await go('/', 'true');
	await js(`localStorage.clear(); localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);

	// the chapter's groups (the draft – what the viewer's draft preview reads): the page offsets of their slices
	const draft = await api('GET', `/entities/chapter:${TOON}:${CHAPTER}?draft=1&wrap=1`);
	const groups = draft.content.groups ?? [];
	const vertical = groups.findIndex((g) => g.renderer?.type === 'vertical' && (g.slices?.length ?? 0) >= 2);
	check(vertical >= 0, `${TOON} ch${CHAPTER}: a vertical group with ≥ 2 slices (#${vertical})`);

	// ── the vertical strip preview ──
	check(await go(`/${TOON}/chapter/${CHAPTER}`, `!!${q(`group-${vertical}`)}`), `chapter editor ${TOON} ch${CHAPTER}`);
	await js(`${q(`group-${vertical}`)}.click(), true`);
	check(await until(`${q('strip-preview')}?.dataset.direction === 'vertical' ? true : null`, 10000), 'a vertical group: the strip preview, vertical');
	const stack = await js(`(() => { const s = ${q('strip-preview-screen')}; const f = [...s.querySelectorAll('.frame')].map((e) => e.getBoundingClientRect()); const r = s.getBoundingClientRect(); return { n: f.length, stacked: f.length > 1 && f.every((b, i) => !i || (b.top >= f[i - 1].bottom - 1 && Math.abs(b.left - f[0].left) < 1)), fill: Math.abs(f[0].width - s.clientWidth) < 2, scrollsY: getComputedStyle(s).overflowY === 'auto' && s.scrollHeight > s.clientHeight, noX: s.scrollWidth <= s.clientWidth + 1, ratio: +(r.width / r.height).toFixed(3) }; })()`);
	check(stack.n === groups[vertical].slices.length && stack.stacked && stack.fill && stack.noX, `phone screen: ${stack.n} slices stacked at the screen's width, no sideways track`, JSON.stringify(stack));
	check(Math.abs(stack.ratio - 390 / 844) < 0.01 && stack.scrollsY, `phone screen 390:844 (${stack.ratio}), scrolled up / down inside`, JSON.stringify(stack));
	await shot('e6-strip-vertical.png');
	await js(`${q('strip-preview-desktop')}.click(), true`);
	await sleep(300);
	const desk = await js(`(() => { const s = ${q('strip-preview-screen')}; const t = s.querySelector('.track').getBoundingClientRect(); const r = s.getBoundingClientRect(); const left = r.left + s.clientLeft; return { ratio: +(r.width / r.height).toFixed(3), column: +(t.width / s.clientWidth).toFixed(3), centred: Math.abs((t.left - left) - (left + s.clientWidth - t.right)) < 3 }; })()`);
	check(Math.abs(desk.ratio - 16 / 9) < 0.02 && Math.abs(desk.column - 500 / 1440) < 0.01 && desk.centred, `desktop screen 16:9: the reading column (500 of 1440), centred`, JSON.stringify(desk));
	await shot('e6-strip-vertical-desktop.png');
	await js(`${q('strip-preview-phone')}.click(), true`);

	// ── the Reader opens ──
	await js(`${q('reader-toggle')}.click(), true`);
	check(await until(`${q('reader-dock')}?.dataset.ready === 'true' ? true : null`, 30000), 'the Reader opens in the dock, the viewer says "ready"');
	await js(`window.__readerDock = ${q('reader-dock')}, window.__readerFrame = ${q('reader-frame')}, true`);
	const d0 = await dock();
	const v0 = await untilFrame(`document.querySelector('.chapter-view [data-slice-index]') ? { path: location.pathname, draft: !!document.querySelector('[data-testid="draft-preview-mark"], .draft-mark') || localStorage.getItem('scutoons.draft-preview') === '1' } : null`, 20000);
	check(v0 && v0.path.endsWith(`/${TOON}/chapter/${CHAPTER}`) && v0.draft, `the frame reads ${TOON} ch${CHAPTER} in the draft preview (signed in by the cookie)`, JSON.stringify(v0));
	const size = await js(`(() => { const f = ${q('reader-frame')}; const d = f.parentElement.getBoundingClientRect(); return { w: f.offsetWidth, h: f.offsetHeight, shownW: Math.round(d.width), shownH: Math.round(d.height) }; })()`);
	check(size.w === 390 && size.h === 844 && size.shownW > 0 && Math.abs(size.shownW / size.shownH - 390 / 844) < 0.01, `a phone screen (390 × 844) scaled to the panel (${size.shownW} × ${size.shownH})`, JSON.stringify(size));

	// ── the selection moves the reader; the panel and the frame stay ──
	const offsets = [];
	{
		// the page = the shown group's chain (linked groups share one page); here: offsets within the whole chapter's chain
		let at = 0;
		for (const g of groups) offsets.push(at), (at += g.slices?.length ?? 0);
	}
	const pageOffset = async (gi) => {
		// the viewer's slice numbering restarts per page: read the group's first slice index from its renderer
		return inFrame(`(() => { const label = ${JSON.stringify(groups[gi].label ?? '')}; const g = label && document.querySelector('.chapter-view [data-group-label="' + label + '"] [data-slice-index]'); return g ? Number(g.dataset.sliceIndex) : null; })()`);
	};
	await js(`${q(`group-${vertical}`)}.click(), true`);
	await sleep(300);
	let first = null;
	for (let i = 0; i < 40 && first === null; i++) (first = await pageOffset(vertical)), first === null && (await sleep(250));
	check(first !== null && (await viewerShows(first)), `group #${vertical} (${groups[vertical].label}): the reader scrolls to its first slice (page slice ${first})`, JSON.stringify(await viewerAt(first ?? 0)));
	await nav(`/${TOON}/chapter/${CHAPTER}?group=${encodeURIComponent(groups[vertical].label ?? String(vertical + 1))}&slice=2`);
	check(await viewerScrolledTo((first ?? 0) + 1), `slice 2 of the group: the reader scrolls to it (page slice ${(first ?? 0) + 1})`, JSON.stringify(await viewerAt((first ?? 0) + 1)));
	const other = groups.findIndex((g, i) => i !== vertical && (g.slices?.length ?? 0) >= 1 && g.label);
	if (other >= 0) {
		await js(`${q(`group-${other}`)}.click(), true`);
		await sleep(300);
		let o = null;
		for (let i = 0; i < 40 && o === null; i++) (o = await pageOffset(other)), o === null && (await sleep(250));
		const ok = await viewerShows(o ?? -1);
		check(o !== null && ok, `group → group #${other} (${groups[other].label}): the reader follows (page slice ${o})`, JSON.stringify(await viewerAt(o ?? 0)));
	}
	const d1 = await dock();
	check(d1.same && d1.frameSame && d1.mount === d0.mount && d1.loads === d0.loads, `group → group / slice: the same panel (mount #${d1.mount}) and frame, not reloaded (${d1.loads} load)`, JSON.stringify({ d0, d1 }));

	// back to the vertical group for the screenshot: the Reader at its second slice
	await nav(`/${TOON}/chapter/${CHAPTER}?group=${encodeURIComponent(groups[vertical].label ?? String(vertical + 1))}&slice=2`);
	await viewerScrolledTo((first ?? 0) + 1);
	await sleep(1500);
	await shot(`e6-reader-${TOON}-ch${CHAPTER}.png`);

	// ── chapter → chapter (another toon's): the same panel, the viewer navigates in place ──
	await nav(`/${SAVE_TOON}/chapter/${SAVE_CHAPTER}`);
	check(await until(`location.pathname === '/${SAVE_TOON}/chapter/${SAVE_CHAPTER}' && ${q('save-draft')} ? true : null`, 30000), `chapter → chapter: ${SAVE_TOON} ch${SAVE_CHAPTER}`);
	const v2 = await untilFrame(`location.pathname.endsWith('/${SAVE_TOON}/chapter/${SAVE_CHAPTER}') && document.querySelector('.chapter-view [data-slice-index]') ? location.pathname : null`, 20000);
	const d2 = await dock();
	check(v2 && d2.same && d2.frameSame && d2.mount === d0.mount && d2.loads === d0.loads, `chapter → chapter: the same panel and frame (mount #${d2.mount}), the viewer moved to ${v2}`, JSON.stringify(d2));

	// ── a saved draft reloads the frame ──
	saveBefore = (await api('GET', `/entities/${SAVE_KEY}?draft=1&wrap=1`)).version.id;
	await inFrame(`document.documentElement.dataset.readerCheck = 'before-save', true`);
	await until(`!!${q('meta-panel')}`, 10000);
	const edited = await js(`(() => { const label = [...document.querySelectorAll('[data-testid="meta-panel"] label')].find((l) => /^\\s*(Author|Autor)/i.test(l.textContent)); const input = label?.querySelector('input') ?? (label?.htmlFor && document.getElementById(label.htmlFor)); if (!input) return false; const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(input, input.value + ' '); input.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	check(edited && (await until(`${q('save-draft')}?.textContent.includes('•') ? true : null`, 5000)), 'an edit (the author + a space) marks the chapter dirty');
	restoreNeeded = true;
	await js(`${q('save-draft')}.click(), true`);
	await until(`!!document.querySelector('dialog.sc-dialog[open] input, dialog.sc-dialog[open] textarea')`, 5000);
	await js(`(() => { const f = document.querySelector('dialog.sc-dialog[open] input, dialog.sc-dialog[open] textarea'); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, ${JSON.stringify(NOTE)}); f.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await sleep(200);
	await js(`document.querySelector('dialog.sc-dialog[open] button.primary').click(), true`);
	check(await until(`!${q('save-draft')}?.textContent.includes('•') ? true : null`, 15000), 'Save: the draft stored');
	check(await until(`Number(${q('reader-dock')}?.dataset.loads) > ${d2.loads} && ${q('reader-dock')}?.dataset.ready === 'true' ? true : null`, 20000), 'the frame reloaded after the save and said "ready" again');
	const v3 = await viewerAt(0);
	const d3 = await dock();
	check(v3 && v3.mark === null && v3.path.endsWith(`/${SAVE_TOON}/chapter/${SAVE_CHAPTER}`) && d3.same && d3.mount === d0.mount, `the reload: a fresh page of the same chapter (the mark gone), the same panel`, JSON.stringify({ v3, d3 }));

	// ── a foreign origin's "show" is ignored ──
	const showsBefore = (await viewerAt(0))?.shows ?? 0;
	check(showsBefore > 0, `the viewer accepted the creator's requests (${showsBefore} after the reload)`);
	await go(`${OTHER}/catpope/avatar.webp`, 'true');
	// a page of the CDN's origin (an image document) frames the viewer and sends it a well-formed "show"
	await js(`window.__got = []; addEventListener('message', (e) => window.__got.push(e.data)); const f = document.createElement('iframe'); f.src = ${JSON.stringify(`${VIEWER}/${TOON}/chapter/${CHAPTER}?draft=1`)}; f.style.width = '390px'; f.style.height = '844px'; (document.body ?? document.documentElement).append(f); true`);
	const foreignReady = await untilFrame(`document.querySelector('.chapter-view [data-slice-index]') ? true : null`, 30000);
	await sleep(1000);
	await js(`document.querySelector('iframe').contentWindow.postMessage({ type: 'scutoons:reader-show', toon: ${JSON.stringify(SAVE_TOON)}, chapter: ${Number(SAVE_CHAPTER)}, group: 0, slice: 1 }, '*'), true`);
	await sleep(1500);
	const vf = await inFrame(`({ path: location.pathname, y: Math.round(scrollY), shows: document.documentElement.dataset.creatorShows ?? null })`);
	check(foreignReady && vf && vf.shows === null && vf.path.endsWith(`/${TOON}/chapter/${CHAPTER}`) && vf.y === 0, `a "show" from ${OTHER}: ignored (no request, the page stays)`, JSON.stringify(vf));
	check((await js('window.__got.length')) === 0, `no "ready" reached the foreign page (sent only to the creator's origin)`);

	if (errors.length) check(false, `page errors: ${errors.length}`, errors.slice(0, 3).join(' | '));
} catch (e) {
	check(false, 'run', e?.stack ?? String(e));
} finally {
	ws.close();
	chrome.kill();
	if (restoreNeeded) {
		const removed = sweep();
		const after = await api('GET', `/entities/${SAVE_KEY}?draft=1&wrap=1`).catch(() => null);
		check(removed >= 1 && after?.version.id === saveBefore, `restored ${SAVE_KEY}: ${removed} draft version(s) removed, newest ${after?.version.id} (was ${saveBefore})`);
	}
}
const passed = results.filter(Boolean).length;
console.log(`\n${passed}/${results.length} passed`);
process.exit(passed === results.length ? 0 : 1);
