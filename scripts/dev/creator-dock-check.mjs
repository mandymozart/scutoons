#!/usr/bin/env node
// The creator's docked regions (agents/DOCKING.md) in headless Chrome (own profile) as the local test user – nothing is
// saved: at 1280 / 1600 / 1920 the chapter editor's Flow and the season's Problems sit in the AppShell's right dock and
// the content ends where the dock starts; with Ask open too the dock is left of Ask and the sidebar folds to its rail
// (not remembered); the sidebar folds by Ctrl+\, by its chevrons and by dragging its edge narrow, a drag out of the rail
// opens it, a double-click resets it; the dock's Divider drags wider (remembered, ← = 16 px, double-click resets) and
// stops where the content would get narrower than 24rem; Ask's Divider drags it wider; on a phone the dock covers the page.
//   node scripts/dev/creator-dock-check.mjs        (creator :10000, API :9000)
//   DOCK_SHOTS=<dir>: screenshots dock-*.png into <dir>
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const OUT = process.env.DOCK_SHOTS ?? '';
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
if (!['localhost', '127.0.0.1'].includes(new URL(CREATOR).hostname)) {
	console.error(`creator-dock-check: refusing ${CREATOR} – local dev stack only`);
	process.exit(2);
}
const SHOTS = !!OUT;
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9600 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-dock-'));
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
	if (!SHOTS) return;
	writeFileSync(`${OUT}/dock-${name}.png`, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
	console.log(`     shot dock-${name}.png`);
};
const size = (w, h = 1000) => send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile: false });
const go = async (url, ready) => {
	await send('Page.navigate', { url: `${CREATOR}${url}` });
	await until('document.readyState === "complete"');
	return until(ready, 30000);
};
/** rects of the regions (px, rounded) */
const layout = () => js(`(() => { const r = (s) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return { l: Math.round(b.left), r: Math.round(b.right), w: Math.round(b.width), h: Math.round(b.height) }; }; return { sidebar: r('.sc-shell-sidebar'), content: r('.sc-shell-content'), dock: r('.sc-shell-dock'), ask: r('[data-testid="ask-panel"]'), folded: !!document.querySelector('.sc-app-shell.folded'), rail: !!document.querySelector('.sc-shell-rail') }; })()`);
const setAsk = async (open) => {
	const now = await js(`${q('ask-toggle')}?.getAttribute('aria-pressed') === 'true'`);
	if (now !== open) await js(`${q('ask-toggle')}.click(), true`);
	await sleep(400);
};
async function drag(selector, dx) {
	const p = await js(`(() => { const b = document.querySelector('${selector}').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p.x, y: p.y, button: 'left', clickCount: 1 });
	for (let i = 1; i <= 8; i++) {
		await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x + (dx * i) / 8, y: p.y, button: 'left', buttons: 1 });
		await sleep(30);
	}
	return { release: async () => { await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: p.x + dx, y: p.y, button: 'left', clickCount: 1 }); await sleep(250); } };
}

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await size(1600);
	await go('/', 'true');
	await js(`localStorage.clear(); localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);

	for (const w of [1280, 1600, 1920]) {
		await size(w);
		// ── chapter editor: Flow + Ask ──
		await setAsk(false);
		check(await go('/catpope/chapter/1?flow=1', `!!${q('flow-panel')} && !!document.querySelector('.svelte-flow__node')`), `${w}: chapter editor with its Flow open`);
		await sleep(600);
		let g = await layout();
		// the sidebar folds by itself only when the content would get narrower than 24rem next to it
		const crowded = false;
		check(g.dock && g.content.r <= g.dock.l + 1 && g.folded === crowded, `${w}: Flow in the shell's dock, the content ends where it starts, sidebar ${crowded ? 'folded (no room)' : 'open'}`, JSON.stringify(g));
		await shot(`${w}-chapter-flow`);
		await setAsk(true);
		await sleep(500);
		g = await layout();
		check(g.ask && g.dock.r <= g.ask.l + 1 && g.folded && g.rail, `${w}: + Ask – Flow left of Ask, the sidebar folds to its rail`, JSON.stringify(g));
		await shot(`${w}-chapter-flow-ask`);
		await setAsk(false);
		await sleep(400);
		g = await layout();
		check(g.folded === crowded, `${w}: Ask closed – the sidebar as before (folded for the page's sake, not remembered)`, JSON.stringify(g));

		// ── a meta page: Problems ──
		check(await go('/catpope/season/1', `!!${q('problems-toggle')}`), `${w}: season page`);
		await js(`${q('problems-toggle')}.click(), true`);
		await sleep(500);
		g = await layout();
		check(g.dock && g.content.r <= g.dock.l + 1, `${w}: Problems docked, the page makes room`, JSON.stringify(g));
		await shot(`${w}-season-problems`);
	}

	// ── sidebar fold / unfold, the dock divider (1600) ──
	await size(1600);
	await setAsk(false);
	await go('/catpope', `!!document.querySelector('.sc-shell-sidebar')`);
	await sleep(400);
	await shot('1600-toon-sidebar-open');
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: '\\', code: 'Backslash', modifiers: 2, windowsVirtualKeyCode: 220 });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: '\\', code: 'Backslash', modifiers: 2, windowsVirtualKeyCode: 220 });
	await sleep(300);
	let g = await layout();
	check(g.folded && g.rail && g.sidebar.w <= 2.5 * 24, 'Ctrl+\\ folds the sidebar to its rail', JSON.stringify(g));
	await shot('1600-toon-sidebar-folded');
	await js(`document.querySelector('.sc-shell-fold').click(), true`);
	await sleep(300);
	g = await layout();
	check(!g.folded, 'the rail\'s chevrons unfold it');
	// drag the sidebar edge narrow: snaps to the rail; drag out again
	const s0 = g.sidebar.w;
	let d = await drag('.sc-shell-resize', -s0 * 0.6);
	await d.release();
	g = await layout();
	check(g.folded, 'dragging the sidebar edge narrow folds it to the rail', JSON.stringify(g));
	d = await drag('.sc-shell-resize', s0 * 1.2);
	await d.release();
	g = await layout();
	check(!g.folded && g.sidebar.w >= s0, `dragging out of the rail opens it (${g.sidebar.w} px)`, JSON.stringify(g));
	await js(`document.querySelector('.sc-shell-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	await sleep(200);
	g = await layout();
	check(g.sidebar.w === s0, `double-click resets the sidebar (${g.sidebar.w} px)`);

	await size(1920);
	// the toon page's Flow: the dock divider drags it wider (held for a screenshot), the content gives way
	check(await go('/catpope?flow=1', `!!document.querySelector('.sc-shell-dock .svelte-flow__node')`), 'toon page: Flow in the dock');
	await sleep(500);
	const before = await layout();
	d = await drag('.sc-shell-dock ~ .sc-divider', -96);
	await shot('1920-toon-flow-dragging');
	await d.release();
	const after = await layout();
	check(after.dock.w > before.dock.w + 80 && after.content.r <= after.dock.l + 1, `the dock's divider drags it wider (${before.dock.w} → ${after.dock.w} px), the content gives way`);
	const stored = await js(`localStorage.getItem('sc-panel-width:flow')`);
	check(Number(stored) === after.dock.w, `the Flow's width is remembered (${stored})`);
	// keys on the divider
	await js(`document.querySelector('.sc-shell-dock ~ .sc-divider').focus(), true`);
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'ArrowLeft', code: 'ArrowLeft', windowsVirtualKeyCode: 37 });
	await sleep(150);
	const keyed = await layout();
	check(keyed.dock.w === after.dock.w + 16, `← on the divider: 16 px wider (${keyed.dock.w})`);
	await js(`document.querySelector('.sc-shell-dock ~ .sc-divider').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	await sleep(200);
	check((await layout()).dock.w === before.dock.w, 'double-click: the normal width again');
	// the dock never squeezes the content below its minimum: drag far
	d = await drag('.sc-shell-dock ~ .sc-divider', -1200);
	await d.release();
	g = await layout();
	check(g.content.w >= 24 * 24 - 2, `dragged far: the content keeps its minimum or the sidebar folds first (content ${g.content.w} px, folded ${g.folded})`, JSON.stringify(g));
	await shot('1920-toon-flow-wide');
	await js(`document.querySelector('.sc-shell-dock ~ .sc-divider').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);

	// Ask's own divider
	await setAsk(true);
	const a0 = (await layout()).ask.w;
	d = await drag('.ask-edge .sc-divider', -120);
	await d.release();
	const a1 = (await layout()).ask.w;
	check(a1 > a0 + 100, `Ask's divider drags it wider (${a0} → ${a1})`);
	await js(`document.querySelector('.ask-edge .sc-divider').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	await setAsk(false);

	// phones: the dock covers the page
	await size(390, 844);
	await go('/catpope?flow=1', `!!document.querySelector('.sc-shell-dock')`);
	await sleep(500);
	g = await layout();
	check(g.dock && g.dock.r <= 390 && g.dock.l >= 0 && g.dock.h > 600, 'phone: the dock covers the page within the screen', JSON.stringify(g));
	await shot('390-toon-flow');

	check(errors.length === 0, 'no page errors', errors.join(' | '));
} finally {
	ws.close();
	chrome.kill();
}
const failed = results.filter((x) => !x).length;
console.log(`creator-dock-check: ${results.length - failed}/${results.length} ok`);
process.exit(failed ? 1 : 0);
