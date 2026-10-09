#!/usr/bin/env node
// The docked regions of admin + embassy (agents/DOCKING.md) in headless Chrome (own profile) as the local test user –
// nothing is saved. Admin at 1280 / 1600 / 1920: a code list's detail (SplitView) opens in the AppShell's right dock, the
// list ends where it starts, the sidebar stays; the Info panel
// opens next to the detail (10.57: both open, Info outermost); the dock folds to a rail with the detail's title, a click
// unfolds it; it maximizes = everything else on its rail (button; Escape restores first; the ActionBar stays); every
// rail's button at one height (1440 / 1920); Escape closes the detail; the catalog's entry form docks the same way; on a phone
// the detail covers the page. Embassy: the page in the AppShell takes the full width.
//   node scripts/dev/tools-dock-check.mjs        (admin :11000, embassy :13000, API :9000 – `npm run dev -w admin` /
//   `-w embassy` when they aren't running)
//   DOCK_SHOTS=<dir>: screenshots <DOCK_PREFIX, default tools-dock->*.png into <dir>
import { spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const OUT = process.env.DOCK_SHOTS ?? '';
const PREFIX = process.env.DOCK_PREFIX ?? 'tools-dock-';
const ADMIN = (process.env.SCUTOONS_ADMIN ?? local('admin')).replace(/\/$/, '');
const EMBASSY = (process.env.SCUTOONS_EMBASSY ?? local('embassy')).replace(/\/$/, '');
for (const origin of [ADMIN, EMBASSY]) {
	if (!['localhost', '127.0.0.1'].includes(new URL(origin).hostname)) {
		console.error(`tools-dock-check: refusing ${origin} – local dev stack only`);
		process.exit(2);
	}
}
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9600 + Math.floor(Math.random() * 150);
const profile = tempProfile('tools-dock-'); // removed on exit (chrome-profile.mjs)
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=1920,1080', 'about:blank'], { stdio: 'ignore' });
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
	if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description);
	return r.result?.result?.value;
}
async function until(expression, ms = 15000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(200);
	}
	return null;
}
const q = (id) => `document.querySelector('[data-testid="${id}"]')`;
const shot = async (name) => {
	if (!OUT) return;
	writeFileSync(`${OUT}/${PREFIX}${name}.png`, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
	console.log(`     shot ${PREFIX}${name}.png`);
};
const size = (w, h = 1000) => send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile: w < 768 });
const go = async (url, ready) => {
	await send('Page.navigate', { url });
	await until('document.readyState === "complete"');
	return until(ready, 30000);
};
const layout = () =>
	js(`(() => { const r = (s) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return { l: Math.round(b.left), r: Math.round(b.right), w: Math.round(b.width), h: Math.round(b.height) }; }; return { sidebar: r('.sc-shell-sidebar'), content: r('.sc-shell-content'), main: r('main'), dock: r('.sc-shell-dock'), detail: !!document.querySelector('.sc-shell-dock [data-testid="split-detail"]'), info: !!document.querySelector('.sc-shell-dock [data-testid="info-panel"]'), folded: !!document.querySelector('.sc-app-shell.folded'), dockRail: !!document.querySelector('[data-testid="dock-rail"]') }; })()`);
async function drag(selector, dx) {
	const p = await js(`(() => { const b = document.querySelector('${selector}').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p.x, y: p.y, button: 'left', clickCount: 1 });
	for (let i = 1; i <= 8; i++) {
		await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x + (dx * i) / 8, y: p.y, button: 'left', buttons: 1 });
		await sleep(30);
	}
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: p.x + dx, y: p.y, button: 'left', clickCount: 1 });
	await sleep(250);
}
const firstRow = `document.querySelector('.sc-shell-content tbody tr.clickable')`;

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await size(1600);
	for (const origin of [ADMIN, EMBASSY]) {
		await go(`${origin}/`, 'true');
		await js(`localStorage.clear(); localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	}

	// ── admin: the list detail (SplitView) in the right dock ──
	for (const w of [1280, 1600, 1920]) {
		await size(w);
		check(await go(`${ADMIN}/codes`, `!!${firstRow}`), `admin ${w}: codes list`);
		await js(`${firstRow}.click(), true`);
		check(await until(`!!document.querySelector('.sc-shell-dock [data-testid="split-detail"]') && location.search.includes('open=')`), `admin ${w}: a row opens its detail in the dock`);
		await sleep(400);
		let g = await layout();
		check(g.dock && g.content.r <= g.dock.l + 1 && !g.folded, `admin ${w}: the list ends where the dock starts, the sidebar stays`, JSON.stringify(g));
		await shot(`admin-${w}-codes-detail`);
	}

	await size(1600);
	// Info opens next to the detail (10.57: several panels side by side; Info outermost)
	check(await go(`${ADMIN}/codes`, `!!${firstRow} && !!${q('info-toggle')}`), 'admin: codes list with Info');
	await js(`${firstRow}.click(), true`);
	await until(`!!document.querySelector('[data-testid="split-detail"]')`);
	await js(`${q('info-toggle')}.click(), true`);
	await sleep(500);
	let g = await layout();
	const pair = await js(`[...document.querySelectorAll('.sc-shell-dock')].map((e) => { const b = e.getBoundingClientRect(); return { kind: e.dataset.kind, l: Math.round(b.left), r: Math.round(b.right) }; })`);
	check(g.info && g.detail && (await js(`location.search.includes('open=')`)) && pair.length === 2 && pair[0].kind === 'detail' && pair[1].kind === 'info' && pair[0].r <= pair[1].l + 1 && g.content.r <= pair[0].l + 1, 'admin: Info opens next to the detail (both open, the detail left, Info outermost, the list makes room)', JSON.stringify({ g, pair }));
	await shot('admin-1600-codes-info');
	await js(`${q('info-toggle')}.click(), true`);
	await sleep(300);
	g = await layout();
	check(g.detail && !g.info && (await js(`${q('info-toggle')}.getAttribute('aria-expanded') === 'false'`)), 'admin: Info closed – the detail stays', JSON.stringify(g));
	// the dock folds to a rail with the detail's title
	const before = g;
	await js(`${q('dock-fold')}.click(), true`);
	await sleep(300);
	g = await layout();
	const railText = await js(`document.querySelector('[data-testid="dock-rail"]')?.innerText.trim()`);
	check(g.dockRail && g.content.w > before.content.w + 200 && !!railText, `admin: the detail folds to a rail "${railText}"`, JSON.stringify(g));
	await shot('admin-1600-codes-rail');
	await js(`document.querySelector('[data-testid="dock-rail"] .sc-rail-label').click(), true`);
	await sleep(300);
	check((await layout()).detail, 'admin: a click on the rail unfolds it');
	// the dock's divider drags it wider
	const w0 = (await layout()).dock.w;
	await drag('.sc-shell-dock-resize', -120);
	const w1 = (await layout()).dock.w;
	check(w1 > w0 + 100 && Number(await js(`localStorage.getItem('sc-panel-width:detail')`)) === w1, `admin: the detail drags wider (${w0} → ${w1} px, remembered)`);
	await js(`document.querySelector('.sc-shell-dock-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	// maximize (owner 2026-10-06): the detail fills from the sidebar's rail to the edge, the list only hidden; Escape in
	// the maximized detail restores first (it doesn't close it)
	const scrollY0 = await js(`window.scrollY`);
	const m0 = await layout();
	await js(`${q('dock-maximize')}.click(), true`);
	await sleep(400);
	g = await layout();
	const mx = await js(`(() => { const b = ${q('dock-maximize')}; const rail = document.querySelector('[data-testid="main-rail"]')?.getBoundingClientRect(); const bar = document.querySelector('.sc-action-bar'); return { max: !!document.querySelector('.sc-app-shell.max-dock'), hidden: getComputedStyle(document.querySelector('.sc-shell-content')).visibility, mainRail: rail ? Math.round(rail.right) : null, bar: !bar || getComputedStyle(bar).visibility === 'visible', icon: b.querySelector('svg').dataset.icon, title: b.title }; })()`);
	check(mx.max && mx.hidden === 'hidden' && mx.mainRail !== null && g.folded && g.dock.r >= 1600 - 2 && g.dock.l <= mx.mainRail + 4 && mx.bar && mx.icon === 'minimize-2' && /^Restore|^Wiederherstellen/.test(mx.title), `admin: the detail maximizes (${g.dock.l}–${g.dock.r} px, "${mx.title}"), the list on its rail (hidden), the ActionBar stays`, JSON.stringify({ g, mx }));
	await shot('admin-1600-codes-max');
	await js(`${q('dock-maximize')}.focus(), true`);
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	await sleep(400);
	g = await layout();
	check(!(await js(`!!document.querySelector('.sc-app-shell.max-dock')`)) && g.detail && g.dock.w === m0.dock.w && g.sidebar?.w === m0.sidebar?.w && (await js(`window.scrollY`)) === scrollY0, 'admin: Escape restores the maximized detail (still open, same widths and scroll)', JSON.stringify({ m0, g }));
	// Escape closes it
	await js(`document.querySelector('[data-testid="split-detail"] button').focus(), true`);
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	check(await until(`!document.querySelector('.sc-shell-dock') && !location.search.includes('open=')`, 5000), 'admin: Escape closes the detail');

	// every rail's button at one height (owner 2026-10-07): the sidebar's, the folded detail's, the main's (Ctrl+Shift+\)
	for (const w of [1440, 1920]) {
		await size(w);
		check(await go(`${ADMIN}/codes`, `!!${firstRow}`), `admin ${w}: codes list for the rails`);
		await js(`${firstRow}.click(), true`);
		await until(`!!document.querySelector('[data-testid="split-detail"]')`);
		await js(`${q('dock-fold')}.click(), document.activeElement?.blur(), true`);
		await sleep(300);
		const key = async (key, modifiers) => {
			for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key, code: 'Backslash', modifiers, windowsVirtualKeyCode: 220 });
			await sleep(300);
		};
		if (!(await js(`!!${q('shell-rail')}`))) await key('\\', 2);
		await key('|', 2 + 8);
		const tops = await js(`(() => { const t = (s) => { const e = document.querySelector(s); return e ? Math.round(e.getBoundingClientRect().top * 10) / 10 : null; }; return { sidebar: t('[data-testid="shell-rail"] button'), main: t('[data-testid="main-rail"] button'), dock: t('[data-testid="dock-rail"] button') }; })()`);
		check(Object.values(tops).every((t) => t !== null) && [tops.main, tops.dock].every((t) => Math.abs(t - tops.sidebar) <= 1), `admin ${w}: every rail's button at one height (${JSON.stringify(tops)})`, JSON.stringify(tops));
		await shot(`admin-${w}-rails`);
		await key('|', 2 + 8);
		if (await js(`!!${q('shell-rail')}`)) await js(`${q('shell-rail')}.querySelector('button').click(), true`);
	}
	await size(1600);

	// the catalog's entry form
	check(await go(`${ADMIN}/catalog/items`, `!!${firstRow}`), 'admin: catalog items');
	await js(`${firstRow}.click(), true`);
	check(await until(`!!document.querySelector('.sc-shell-dock [data-testid="split-detail"] form')`), 'admin: a catalog entry opens its form in the dock');
	await sleep(400);
	g = await layout();
	check(g.content.r <= g.dock.l + 1, 'admin: the catalog list makes room', JSON.stringify(g));
	await shot('admin-1600-catalog-entry');
	// the selection model (BACKLOG 10.40, DESIGN.md "Selection"): the open row is sc-selected; the table is the one live
	// region (live on load), so the row shows the live look – a cyan tint, the cells framed in the primary colour
	const sel = await js(`(() => { const r = document.querySelector('tr.sc-selected'); const live = [...document.querySelectorAll('[data-live]')].map((e) => e.dataset.scRegion); return { live, rows: document.querySelectorAll('tr.sc-selected').length, bg: r && getComputedStyle(r).backgroundColor, frame: r && getComputedStyle(r.querySelector('td')).boxShadow }; })()`);
	check(sel.live.length === 1 && sel.live[0].startsWith('table') && sel.rows === 1 && sel.bg === 'rgb(240, 252, 255)' && /0, 209, 255/.test(sel.frame ?? ''), 'admin: the open catalog row is the live selection (one live region: the table)', JSON.stringify(sel));

	// phones: the detail covers the page
	await size(390, 844);
	check(await go(`${ADMIN}/codes`, `!!${firstRow}`), 'admin phone: codes list');
	await js(`${firstRow}.click(), true`);
	await until(`!!document.querySelector('[data-testid="split-detail"]')`);
	await sleep(500);
	g = await layout();
	check(g.dock && g.dock.l >= 0 && g.dock.r <= 390 && g.dock.h > 600, 'admin phone: the detail covers the page', JSON.stringify(g));
	await shot('admin-390-codes-detail');

	// ── embassy: AppShell, the page takes the full width ──
	for (const w of [1280, 1600, 1920]) {
		await size(w);
		check(await go(`${EMBASSY}/`, `!!document.querySelector('.sc-app-shell main')`), `embassy ${w}: Today in the AppShell`);
		g = await layout();
		check(g.content.r >= w - 1, `embassy ${w}: the page takes the full width`, JSON.stringify(g));
		await shot(`embassy-${w}-today`);
	}
	// the selection model: at most one live region (the embassy has none of its own – its nav is "you are here")
	check((await js(`document.querySelectorAll('[data-live]').length`)) <= 1, 'embassy: at most one live selection region');
	await size(390, 844);
	check(await go(`${EMBASSY}/`, `!!document.querySelector('.sc-app-shell main')`), 'embassy phone: Today');
	g = await layout();
	check(g.content.l <= 0 && g.content.r >= 389, 'embassy phone: the page takes the full width', JSON.stringify(g));
	await shot('embassy-390-today');

	check(errors.length === 0, 'no page errors', errors.join(' | '));
} finally {
	ws.close();
	chrome.kill();
}
const failed = results.filter((x) => !x).length;
console.log(`tools-dock-check: ${results.length - failed}/${results.length} ok`);
process.exit(failed ? 1 : 0);
