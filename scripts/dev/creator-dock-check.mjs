#!/usr/bin/env node
// The creator's docked regions (agents/DOCKING.md) in headless Chrome (own profile) as the local test user – nothing is
// saved: at 1280 / 1600 / 1920 the chapter editor's Flow and the season's Problems sit in the AppShell's right dock and
// the content ends where the dock starts; the sidebar folds by Ctrl+\, by its chevrons and by dragging its edge narrow, a drag out of the rail
// opens it, a double-click resets it; the dock's Divider drags wider (remembered, ← = 16 px, double-click resets) and
// stops where the content would get narrower than 24rem; the dock folds to a rail (its fold button, Enter, dragging it
// narrow; the rail shows the panel's icon + title, a click / a drag opens it; a panel opens
// unfolded); the script outline folds to its rail and drags on its Divider (remembered); the
// slice inspector is a region beside the canvas (at most half the area, below it under 1280px); maximize
// (owner 2026-10-06): the Flow / the slice inspector fill the space right of the sidebar's rail by their button, Esc, Ctrl+Space (the main stays mounted: its unsaved edit, scroll and widths survive), a Flow box click
// restores and navigates, another panel takes the maximize over, closing ends it; on a phone the dock covers the page.
// 10.57 (owner 2026-10-07): maximize = fold everything else to its rail (main, other panels; the ActionBar stays);
// several panels: the room running out folds the least recently used first (a squeezed rail's click brings it back,
// the next folds), Ctrl+Shift+\ folds the main; every rail's button at one height (±1 px). Admin + embassy (Info, SplitView): check:tools-dock.
//   node scripts/dev/creator-dock-check.mjs        (creator :10000, API :9000)
//   DOCK_SHOTS=<dir>: screenshots <DOCK_PREFIX, default dock->*.png into <dir>
import { spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const OUT = process.env.DOCK_SHOTS ?? '';
const PREFIX = process.env.DOCK_PREFIX ?? 'dock-';
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
const profile = tempProfile('creator-dock-'); // removed on exit (chrome-profile.mjs)
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
	// leaving a page with an unsaved edit (the maximize check's): nothing is saved
	if (m.method === 'Page.javascriptDialogOpening') ws.send(JSON.stringify({ id: ++seq, method: 'Page.handleJavaScriptDialog', params: { accept: true } }));
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
	writeFileSync(`${OUT}/${PREFIX}${name}.png`, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
	console.log(`     shot ${PREFIX}${name}.png`);
};
const size = (w, h = 1000) => send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile: false });
const go = async (url, ready) => {
	await send('Page.navigate', { url: `${CREATOR}${url}` });
	await until('document.readyState === "complete"');
	return until(ready, 30000);
};
/** rects of the regions (px, rounded) */
const layout = () => js(`(() => { const r = (s) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return { l: Math.round(b.left), r: Math.round(b.right), w: Math.round(b.width), h: Math.round(b.height) }; }; return { sidebar: r('.sc-shell-sidebar'), content: r('.sc-shell-content'), dock: r('.sc-shell-dock'), folded: !!document.querySelector('.sc-app-shell.folded'), rail: !!document.querySelector('[data-testid="shell-rail"]'), dockRail: !!document.querySelector('[data-testid="dock-rail"]') }; })()`);
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
		// ── chapter editor: Flow ──
		check(await go('/catpope/chapter/1?flow=1', `!!${q('flow-panel')} && !!document.querySelector('.svelte-flow__node')`), `${w}: chapter editor with its Flow open`);
		await sleep(600);
		let g = await layout();
		// the sidebar folds by itself only when the content would get narrower than 24rem next to it
		const crowded = false;
		check(g.dock && g.content.r <= g.dock.l + 1 && g.folded === crowded, `${w}: Flow in the shell's dock, the content ends where it starts, sidebar ${crowded ? 'folded (no room)' : 'open'}`, JSON.stringify(g));
		await shot(`${w}-chapter-flow`);

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
	await go('/catpope', `!!document.querySelector('.sc-shell-sidebar')`);
	await sleep(400);
	await shot('1600-toon-sidebar-open');
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: '\\', code: 'Backslash', modifiers: 2, windowsVirtualKeyCode: 220 });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: '\\', code: 'Backslash', modifiers: 2, windowsVirtualKeyCode: 220 });
	await sleep(300);
	let g = await layout();
	check(g.folded && g.rail && g.sidebar.w <= (await js(`(() => { const p = document.createElement('div'); p.style.width = 'var(--sc-shell-rail-width)'; (document.querySelector('.sc-app-shell') ?? document.body).append(p); const w = Math.round(p.getBoundingClientRect().width); p.remove(); return w; })()`)) + 2, 'Ctrl+\\ folds the sidebar to its rail', JSON.stringify(g));
	await shot('1600-toon-sidebar-folded');
	await js(`document.querySelector('[data-testid="shell-rail"] button').click(), true`);
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

	// ── the right dock folds to a rail (owner 2026-10-06): its fold button, the rail, keys, dragging it narrow ──
	await size(1600);
	check(await go('/catpope/chapter/1?flow=1', `!!${q('flow-panel')} && !!document.querySelector('.svelte-flow__node')`), '1600: chapter Flow for the rail');
	await sleep(500);
	// a rail is as wide as its round unfold button + 0.5rem either side (--sc-shell-rail-width, owner 2026-10-06)
	const railPx = await js(`(() => { const p = document.createElement('div'); p.style.width = 'var(--sc-shell-rail-width)'; (document.querySelector('.sc-app-shell') ?? document.body).append(p); const w = Math.round(p.getBoundingClientRect().width); p.remove(); return w; })()`);
	const enter = async () => {
		await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' });
		await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
		await sleep(300);
	};
	const open0 = await layout();
	await js(`${q('dock-fold')}.click(), true`);
	await sleep(300);
	g = await layout();
	const railText = await js(`document.querySelector('[data-testid="dock-rail"]')?.innerText.trim()`);
	check(g.dockRail && Math.abs(g.dock.w - railPx) <= 2 && g.content.r <= g.dock.l + 1 && g.content.w > open0.content.w + 200 && !!railText, `the dock's fold button folds it to a rail labelled "${railText}" (${g.dock.w} px), the content takes the room`, JSON.stringify(g));
	check(await js(`!!document.querySelector('[data-testid="dock-rail"] .sc-rail-icon svg')`), "the rail shows the panel's icon");
	await shot('1600-dock-rail');
	await js(`document.querySelector('[data-testid="dock-rail"] .sc-rail-label').click(), true`);
	await sleep(300);
	g = await layout();
	check(!g.dockRail && g.dock.w === open0.dock.w && (await js(`!!${q('flow-panel')}`)), `a click on the rail unfolds it (${g.dock.w} px)`, JSON.stringify(g));
	// keyboard: Enter on the fold button, then on the rail's button
	await js(`${q('dock-fold')}.focus(), true`);
	await enter();
	check((await layout()).dockRail, 'keyboard: Enter on the fold button folds the dock');
	await js(`document.querySelector('[data-testid="dock-rail"] button').focus(), true`);
	await enter();
	check(!(await layout()).dockRail, "keyboard: Enter on the rail's button unfolds it");
	d = await drag('.sc-shell-dock-resize', open0.dock.w * 0.6);
	await d.release();
	g = await layout();
	check(g.dockRail, "dragging the dock's edge narrow folds it to the rail", JSON.stringify(g));
	d = await drag('.sc-shell-dock-resize', -open0.dock.w * 1.1);
	await d.release();
	g = await layout();
	check(!g.dockRail && g.dock.w >= open0.dock.w, `dragging out of the rail opens it (${g.dock.w} px)`, JSON.stringify(g));
	await js(`document.querySelector('.sc-shell-dock-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	await sleep(200);
	await js(`${q('dock-fold')}.click(), true`);
	await sleep(200);
	await js(`${q('flow-toggle')}.click(), true`);
	await sleep(300);
	await js(`${q('flow-toggle')}.click(), true`);
	await sleep(400);
	check(!(await layout()).dockRail && (await js(`!!${q('flow-panel')}`)), 'closed and opened again, the panel opens unfolded (the fold is not remembered)');

	// ── maximize (owner 2026-10-06): a dock panel / the slice inspector fills the space right of the sidebar's rail;
	//    the rest is only hidden (widths, folds, scroll positions, unsaved edits stay); button, Esc, Ctrl+Space, a Flow box ──
	const press = async (key, code, vk, modifiers = 0) => {
		await send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, modifiers, windowsVirtualKeyCode: vk });
		await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, modifiers, windowsVirtualKeyCode: vk });
		await sleep(350);
	};
	const ctrlSpace = () => press(' ', 'Space', 32, 2);
	const escape = () => press('Escape', 'Escape', 27);
	const maxState = () =>
		js(`(() => { const b = ${q('dock-maximize')}; const c = document.querySelector('.sc-shell-content'); const bar = document.querySelector('.sc-action-bar'); const br = bar?.getBoundingClientRect(); return { dock: !!document.querySelector('.sc-app-shell.max-dock'), main: !!document.querySelector('.sc-app-shell.max-main'), contentHidden: c ? getComputedStyle(c).visibility === 'hidden' : null, mainRail: !!${q('main-rail')}, bar: !!bar && getComputedStyle(bar).visibility === 'visible' && br.bottom <= innerHeight + 1 && br.height > 10, icon: b?.querySelector('svg')?.dataset.icon ?? null, title: b?.title ?? null, label: b?.getAttribute('aria-label') ?? null }; })()`);
	const field = `document.querySelector('.columns > .main input[type="text"], .columns > .main textarea')`;
	check(await go('/catpope/chapter/1?flow=1', `!!${q('flow-panel')} && !!document.querySelector('.svelte-flow__node') && !!${field}`), '1600: chapter Flow + an editable main for maximize');
	await sleep(500);
	// an unsaved edit in the main, and the main scrolled a bit
	await js(`${field}.focus(), true`);
	await send('Input.insertText', { text: ' MAXTEST' });
	await sleep(300);
	const edited = await js(`${field}.value`);
	await js(`(() => { document.querySelector('.columns > .main').scrollTop = 120; document.activeElement.blur(); return true; })()`);
	const m0 = await maxState();
	const l0 = await layout();
	const scroll0 = await js(`document.querySelector('.columns > .main').scrollTop`);
	check(m0.icon === 'maximize-2' && m0.label === 'Maximize' && m0.title?.startsWith('Maximize') && edited.endsWith(' MAXTEST') && (await js(`!${q('save-draft')}.disabled`)), `the Flow's header has the round maximize button ("${m0.title}", maximize-2); the main has an unsaved edit`, JSON.stringify(m0));
	await shot('1600-flow');
	await js(`${q('dock-maximize')}.click(), true`);
	await sleep(400);
	let mx = await maxState();
	g = await layout();
	check(mx.dock && mx.contentHidden && mx.mainRail && g.folded && g.rail && g.dock.l <= 2 * railPx + 4 && g.dock.r >= 1600 - 2 && !(await js(`!!${q('dock-fold')}`)), `maximized: the Flow fills from the sidebar's and the main's rails to the edge (${g.dock.l}–${g.dock.r} px), the main folded to its rail (laid out, hidden), no fold button`, JSON.stringify({ mx, g }));
	check(mx.bar, 'maximized: the ActionBar stays visible (Save / Publish / undo still act on the main)', JSON.stringify(mx));
	check(mx.icon === 'minimize-2' && mx.label === 'Restore' && mx.title?.startsWith('Restore'), `maximized: the button says "${mx.title}" (minimize-2)`, JSON.stringify(mx));
	check((await js(`${field}.value`)) === edited && (await js(`!!${q('flow-panel')}`)), 'maximized: the hidden main keeps its edit (nothing unmounted)');
	await shot('1600-flow-max');
	await js(`${q('dock-maximize')}.click(), true`);
	await sleep(400);
	mx = await maxState();
	g = await layout();
	const scroll1 = await js(`document.querySelector('.columns > .main').scrollTop`);
	check(!mx.dock && !mx.contentHidden && g.dock.w === l0.dock.w && g.sidebar.w === l0.sidebar.w && g.folded === l0.folded && scroll1 === scroll0, `restored: the same widths (dock ${g.dock.w}, sidebar ${g.sidebar.w} px), the main's scroll (${scroll1})`, JSON.stringify({ l0, g, scroll0, scroll1 }));
	check((await js(`${field}.value`)) === edited && (await js(`!${q('save-draft')}.disabled`)), 'restored: the unsaved edit is still there');
	// Esc
	await js(`${q('dock-maximize')}.click(), document.activeElement.blur(), true`);
	await sleep(300);
	await escape();
	check(!(await maxState()).dock && (await js(`!!${q('flow-panel')}`)), 'Esc restores (the panel stays open)');
	// Ctrl+Space: focus in the dock → maximizes it; again → restores
	await js(`${q('dock-maximize')}.focus(), true`);
	await ctrlSpace();
	check((await maxState()).dock, 'Ctrl+Space in the dock maximizes it');
	await ctrlSpace();
	check(!(await maxState()).dock, 'Ctrl+Space again restores');
	// Ctrl+Space from the main: the open dock panel; focus comes back to the field on restore
	await js(`${field}.focus(), true`);
	await ctrlSpace();
	check((await maxState()).dock && (await js(`document.activeElement === ${q('dock-maximize')}`)), 'Ctrl+Space in the main maximizes the open dock panel, focus goes to its button');
	await escape();
	check(!(await maxState()).dock && (await js(`document.activeElement === ${field}`)), 'Esc restores, focus back in the field');
	await js(`${q('dock-maximize')}.click(), true`);
	await sleep(300);
	// another dock panel opened while one is maximized takes the maximize over (10.57: the Flow stays open, on its rail)
	// Problems sits in the ActionBar's "More" below 2048px
	if (!(await js(`!!${q('problems-toggle')}`))) {
		await js(`[...document.querySelectorAll('.sc-action-bar button')].find((b) => /more|mehr/i.test(b.getAttribute('aria-label') ?? b.textContent)).click(), true`);
		await until(`!!${q('problems-toggle')}`, 3000);
	}
	await js(`${q('problems-toggle')}.click(), true`);
	await sleep(400);
	mx = await maxState();
	check(mx.dock && (await js(`!!document.querySelector('.sc-shell-dock.maxed [data-testid="problems-panel"]') && !!document.querySelector('.sc-shell-dock[data-kind="flow"] [data-testid="dock-rail-maximized"]')`)), 'Problems opened while the Flow is maximized: Problems maximized, the Flow open on its rail', JSON.stringify(mx));
	// closing the maximized Problems ends the maximize; the Flow maximized again
	await js(`${q('problems-toggle')}.click(), true`);
	await sleep(400);
	check(!(await maxState()).dock && (await js(`!!${q('flow-panel')}`)), 'Problems closed: the maximize ends, the Flow shows again');
	await js(`document.querySelector('.sc-shell-dock[data-kind="flow"] [data-testid="dock-maximize"]').click(), true`);
	await until(`!!document.querySelector('.sc-shell-dock .box.kind-group')`);
	await sleep(800);
	check((await maxState()).dock, 'the Flow maximized again');
	// a click on a box restores and opens it in the main
	const box = await js(`(() => { const e = document.querySelector('.sc-shell-dock .box.kind-group'); if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + Math.min(b.height / 2, 20) }; })()`);
	if (box) {
		await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: box.x, y: box.y });
		await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: box.x, y: box.y, button: 'left', clickCount: 1 });
		await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: box.x, y: box.y, button: 'left', clickCount: 1 });
	}
	const navigated = await until(`!document.querySelector('.sc-app-shell.max-dock') && location.search.includes('group=')`, 5000);
	check(box && navigated, `a click on a Flow box restores and opens it in the main (${await js('location.search')})`);
	// closing a maximized panel ends the maximize
	await js(`${q('dock-maximize')}.click(), true`);
	await sleep(300);
	await js(`${q('flow-toggle')}.click(), true`);
	await sleep(400);
	check(await js(`!document.querySelector('.sc-app-shell.max-dock, .sc-app-shell.max-main')`), 'closing the maximized panel ends the maximize');

	// the slice inspector maximizes the same way (beside the canvas, the Flow in the dock)
	check(await go('/catpope/chapter/1?group=1&slice=2&flow=1', `!!${q('inspector-maximize')} && !!document.querySelector('.canvas-scroll') && !!${q('flow-panel')}`), '1600: a slice with its inspector and the Flow');
	await sleep(500);
	const i0m = await js(`Math.round(document.querySelector('[data-testid="inspector-region"]').getBoundingClientRect().width)`);
	await js(`${q('inspector-maximize')}.click(), true`);
	await sleep(400);
	let im = await js(`(() => { const r = (s) => { const b = document.querySelector(s).getBoundingClientRect(); return { l: Math.round(b.left), r: Math.round(b.right), w: Math.round(b.width) }; }; return { region: r('[data-testid="inspector-region"]'), columns: r('.columns'), main: getComputedStyle(document.querySelector('.columns > .main')).visibility, flowRail: !!document.querySelector('.sc-shell-dock[data-kind="flow"] [data-testid="dock-rail-maximized"]'), shell: !!document.querySelector('.sc-app-shell.max-main'), folded: !!document.querySelector('.sc-app-shell.folded'), icon: ${q('inspector-maximize')}.querySelector('svg').dataset.icon, title: ${q('inspector-maximize')}.title }; })()`);
	check(im.shell && im.folded && im.main === 'hidden' && im.flowRail && Math.abs(im.region.l - im.columns.l) <= 2 && im.region.r >= 1600 - railPx - 2 && im.icon === 'minimize-2' && im.title.startsWith('Restore'), `the inspector maximized: it fills from the sidebar's rail to the Flow's (${im.region.w} px), the canvas hidden, the Flow on its rail`, JSON.stringify(im));
	check((await maxState()).bar, 'the inspector maximized: the ActionBar stays visible');
	// Esc in a text field belongs to the field (a field of the maximized inspector)
	if (await js(`!!document.querySelector('[data-testid="inspector-region"] input[type="text"], [data-testid="inspector-region"] textarea')`)) {
		await js(`document.querySelector('[data-testid="inspector-region"] input[type="text"], [data-testid="inspector-region"] textarea').focus(), true`);
		await escape();
		check(await js(`!!document.querySelector('.sc-app-shell.max-main')`), 'Esc in a text field leaves the maximize alone');
	}
	await shot('1600-inspector-max');
	await js(`document.activeElement.blur(), true`);
	await escape();
	im = await js(`(() => ({ w: Math.round(document.querySelector('[data-testid="inspector-region"]').getBoundingClientRect().width), main: getComputedStyle(document.querySelector('.columns > .main')).visibility, shell: !!document.querySelector('.sc-app-shell.max-main') }))()`);
	check(!im.shell && im.main === 'visible' && im.w === i0m, `Esc: the inspector back beside the canvas (${im.w} px)`, JSON.stringify(im));
	await js(`${q('tab-layers')}.focus(), true`);
	await ctrlSpace();
	check(await js(`!!document.querySelector('.sc-app-shell.max-main') && document.activeElement === ${q('tab-layers')}`), 'Ctrl+Space in the inspector maximizes the inspector (focus stays)');
	await ctrlSpace();
	check(await js(`!document.querySelector('.sc-app-shell.max-main, .sc-app-shell.max-dock')`), 'Ctrl+Space restores');

	// ── several panels (10.57): the room running out folds the least recently used first; every rail's button at one
	//    height (the panels' header buttons) ──
	const docks = () => js(`[...document.querySelectorAll('.sc-shell-dock')].map((e) => ({ kind: e.dataset.kind, rail: e.classList.contains('railed'), squeezed: !!e.querySelector('[data-testid="dock-rail-squeezed"]'), w: Math.round(e.getBoundingClientRect().width) }))`);
	const squeezedKinds = async () => (await docks()).filter((d) => d.squeezed).map((d) => d.kind).sort().join();
	// squeezed rails are always the least recently used ones: a prefix of `order` (any order within it)
	const lruPrefix = (kinds, order) => {
		const list = kinds ? kinds.split(',') : [];
		return list.length <= order.length && order.slice(0, list.length).sort().join() === list.slice().sort().join();
	};
	// 1024 squeezes (sidebar + 24rem content + three 12rem floors do not fit), 1440 may or may not
	for (const w of [1024, 1440]) {
		await size(w);
		check(await go('/catpope/chapter/1?flow=1', `!!${q('flow-panel')} && !!document.querySelector('.svelte-flow__node')`), `${w}: chapter with its Flow`);
		await js(`['flow', 'reader', 'chapter'].forEach((k) => localStorage.removeItem('sc-panel-width:' + k)), true`);
		await js(`${q('reader-toggle')}.click(), true`);
		await sleep(400);
		if (!(await js(`!!${q('problems-toggle')}`))) {
			await js(`[...document.querySelectorAll('.sc-action-bar button')].find((b) => /more|mehr/i.test(b.getAttribute('aria-label') ?? b.textContent)).click(), true`);
			await until(`!!${q('problems-toggle')}`, 3000);
		}
		await js(`${q('problems-toggle')}.click(), true`);
		await sleep(700);
		const order = (await docks()).map((d) => d.kind).join();
		check(order === 'problems,flow,reader', `${w}: three panels in their places: ${order}`);
		const sq = await squeezedKinds();
		g = await layout();
		check(lruPrefix(sq, ['flow', 'reader']) && (w > 1024 || sq) && g.content.w >= 24 * (await js('parseFloat(getComputedStyle(document.documentElement).fontSize)')) / 2 - 2, `${w}: Flow, Reader, Problems opened in that order: the least recently used fold to their rails first (${sq || 'none'})`, JSON.stringify(await docks()));
		await shot(`${w}-three-panels`);
		if (sq.includes('flow')) {
			// the Flow's rail: it comes back (now the most recently used), the next least recently used folds instead
			await js(`document.querySelector('.sc-shell-dock[data-kind="flow"] [data-testid="dock-rail-squeezed"] button').click(), true`);
			await sleep(500);
			const sq2 = await squeezedKinds();
			check(lruPrefix(sq2, ['reader', 'problems']) && sq2.split(',').length === sq.split(',').length && !(await docks()).find((d) => d.kind === 'flow').rail, `${w}: a click on the Flow's rail brings it back, ${sq2} on rails now`, JSON.stringify(await docks()));
		}
		// rails at one height (the panels' header buttons): the sidebar's (Ctrl+\), a folded panel's, the main's (Ctrl+Shift+\)
		await js(`${q('dock-fold')}?.click(), true`);
		await sleep(200);
		// the Reader's frame may hold the focus (keys go to the viewer then): the creator's page first – before Ctrl+\
		await js(`document.activeElement?.blur(), true`);
		if (!(await js(`!!${q('shell-rail')}`))) await press('\\', 'Backslash', 220, 2);
		await press('|', 'Backslash', 220, 2 + 8);
		const tops = await js(`(() => { const t = (s) => { const e = document.querySelector(s); return e ? Math.round(e.getBoundingClientRect().top * 10) / 10 : null; }; return { sidebar: t('[data-testid="shell-rail"] button'), main: t('[data-testid="main-rail"] button'), dock: t('[data-testid="dock-rail"] button, [data-testid="dock-rail-squeezed"] button') }; })()`);
		check(tops.sidebar !== null && tops.main !== null && tops.dock !== null && [tops.main, tops.dock].every((t) => Math.abs(t - tops.sidebar) <= 1), `${w}: every rail's button at one height (sidebar ${tops.sidebar}, main ${tops.main}, panel ${tops.dock})`, JSON.stringify(tops));
		await shot(`${w}-rails`);
		await press('|', 'Backslash', 220, 2 + 8);
		check(!(await js(`!!${q('main-rail')}`)), `${w}: Ctrl+Shift+\\ again unfolds the main`);
		if (await js(`!!${q('shell-rail')}`)) await js(`${q('shell-rail')}.querySelector('button').click(), true`);
	}
	await size(1600);

	// ── the script outline: a left region of the page on the shared fold + Divider ──
	check(await go('/catpope/script', `!!${q('script-outline')}`), 'script page with its outline');
	await sleep(500);
	const side = () => js(`(() => { const b = document.querySelector('#script-outline').getBoundingClientRect(); return { w: Math.round(b.width), rail: !!document.querySelector('[data-testid="script-outline-rail"]'), folded: localStorage.getItem('sc-script-outline-folded') }; })()`);
	const o0 = await side();
	await shot('1600-script-outline');
	await js(`${q('script-outline-fold')}.click(), true`);
	await sleep(300);
	let o = await side();
	check(o.rail && Math.abs(o.w - railPx) <= 2 && o.folded === '1', `the outline's fold button folds it to its rail (${o.w} px, remembered)`, JSON.stringify(o));
	await shot('1600-script-outline-folded');
	await js(`document.querySelector('[data-testid="script-outline-rail"] .sc-rail-label').click(), true`);
	await sleep(300);
	o = await side();
	check(!o.rail && o.w === o0.w && o.folded === null, 'a click on its rail unfolds it', JSON.stringify(o));
	d = await drag('.outline-resize', 120);
	await d.release();
	o = await side();
	check(o.w >= o0.w + 100 && Number(await js(`localStorage.getItem('sc-script-outline-width')`)) === o.w, `its Divider drags it wider (${o0.w} → ${o.w} px, remembered)`);
	await js(`document.querySelector('.outline-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
	await sleep(200);
	check((await side()).w === o0.w, 'double-click: its normal width again');
	d = await drag('.outline-resize', -o0.w * 0.6);
	await d.release();
	check((await side()).rail, 'dragging it narrow folds it to the rail');
	d = await drag('.outline-resize', o0.w * 1.1);
	await d.release();
	o = await side();
	check(!o.rail && o.w >= o0.w, `dragging out of the rail opens it (${o.w} px)`);
	await js(`document.querySelector('.outline-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);

	// ── the slice inspector: a right region of the chapter editor's main area ──
	const ins = () => js(`(() => { const r = (s) => { const b = document.querySelector(s).getBoundingClientRect(); return { l: Math.round(b.left), r: Math.round(b.right), t: Math.round(b.top), b: Math.round(b.bottom), w: Math.round(b.width) }; }; return { main: r('.columns > .main'), region: r('[data-testid="inspector-region"]'), columns: r('.columns') }; })()`);
	for (const w of [1280, 1600]) {
		await size(w);
		check(await go('/catpope/chapter/1?group=1&slice=2', `!!${q('inspector-region')} && !!document.querySelector('.canvas-scroll')`), `${w}: a slice with its inspector`);
		await sleep(500);
		const i0 = await ins();
		check(i0.main.r <= i0.region.l + 1 && i0.region.r <= i0.columns.r + 1, `${w}: the inspector beside the canvas (${i0.region.w} px), nothing overlaps`, JSON.stringify(i0));
		await shot(`${w}-slice-inspector`);
		// the selection model (BACKLOG 10.40, DESIGN.md "Selection"): the canvas + inspector are the one live region on
		// load; a click in the slice list makes the list live (only it) and its shown slice takes the live look
		const lives = () => js(`[...document.querySelectorAll('[data-live]')].map((e) => e.dataset.scRegion).join(',')`);
		check((await lives()) === 'slice', `${w}: on load the slice canvas + inspector is the one live region`, await lives());
		// a real click on the shown slice's row (its thumbnail – it stays the shown one)
		const at = await js(`(() => { const b = document.querySelector('[data-testid="slice-list"] > li.sc-selected .thumb'); b.scrollIntoView({ block: 'center' }); const r = b.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
		for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: at.x, y: at.y, button: 'left', clickCount: 1 });
		await sleep(300);
		const shown = await js(`(() => { const li = document.querySelector('[data-testid="slice-list"] > li.sc-selected'); return li ? getComputedStyle(li).backgroundColor : null; })()`);
		check((await lives()) === 'slices' && shown === 'rgb(240, 252, 255)', `${w}: a click in the slice list – the list is the one live region, its slice in the live look`, `${await lives()} ${shown}`);
		d = await drag('.inspector-resize', -2000);
		await d.release();
		const i1 = await ins();
		// half the area – never below its normal width (at 1280 the normal 22rem is already more than half)
		const half = Math.ceil(i1.columns.w / 2);
		check(i1.region.w === Math.max(i0.region.w, half) || Math.abs(i1.region.w - Math.max(i0.region.w, half)) <= 1, `${w}: dragged far it stops at half the editor's area, never below its normal width (${i0.region.w} → ${i1.region.w} of ${i1.columns.w} px)`, JSON.stringify(i1));
		await js(`document.querySelector('.inspector-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })), true`);
		await sleep(200);
		check((await ins()).region.w === i0.region.w, `${w}: double-click resets it`);
	}
	await size(1200);
	await go('/catpope/chapter/1?group=1&slice=2', `!!${q('inspector-region')}`);
	await sleep(500);
	g = await js(`(() => { const m = document.querySelector('.columns > .main').getBoundingClientRect(); const r = document.querySelector('[data-testid="inspector-region"]').getBoundingClientRect(); const d = document.querySelector('.inspector-resize'); return { mainBottom: Math.round(m.bottom), regionTop: Math.round(r.top), divider: d ? getComputedStyle(d).display : 'none' }; })()`);
	check(g.regionTop >= g.mainBottom - 1 && g.divider === 'none', '1200: the inspector below the canvas (no Divider)', JSON.stringify(g));
	await shot('1200-slice-inspector');
	await size(1920);

	// phones: the dock covers the page
	await size(390, 844);
	await go('/catpope?flow=1', `!!document.querySelector('.sc-shell-dock')`);
	await sleep(500);
	g = await layout();
	check(g.dock && g.dock.r <= 390 && g.dock.l >= 0 && g.dock.h > 600, 'phone: the dock covers the page within the screen', JSON.stringify(g));
	await shot('390-toon-flow');
	check(await js(`(() => { const b = ${q('dock-maximize')}; return !b || getComputedStyle(b).display === 'none'; })()`), 'phone: no maximize button');

	check(errors.length === 0, 'no page errors', errors.join(' | '));
} finally {
	ws.close();
	chrome.kill();
}
const failed = results.filter((x) => !x).length;
console.log(`creator-dock-check: ${results.length - failed}/${results.length} ok`);
process.exit(failed ? 1 : 0);
