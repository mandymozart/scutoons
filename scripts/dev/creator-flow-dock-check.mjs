#!/usr/bin/env node
// Dock panels across pages (UI-CONCEPTS E.3 b, agents/DOCKING.md "Panels across pages") in headless Chrome (own profile)
// as the local test user – nothing is saved: the toon's Flow, zoomed out, stays the SAME mounted panel (FlowDock's
// element + its mount counter) toon → season → chapter (client-side navigation, as a crumb / sidebar / box does), each
// new level rebuilt at the kept zoom; in the chapter group → group keeps even the graph and its viewport; a box click
// navigates and the panel follows; Problems stay the same panel chapter → season; a page without a Flow (cast) closes
// the held one; the admin's Info stays the same panel codes → assets; Ask gets the crumb trail's place (context + the deepest crumb's address).
//   node scripts/dev/creator-flow-dock-check.mjs        (creator :10000, API :9000)
//   SCUTOONS_FLOW_TOON (default catpope)
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tempProfile } from './chrome-profile.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const TOON = process.env.SCUTOONS_FLOW_TOON ?? 'catpope';
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
if (!['localhost', '127.0.0.1'].includes(new URL(CREATOR).hostname)) {
	console.error(`creator-flow-dock-check: refusing ${CREATOR} – local dev stack only`);
	process.exit(2);
}
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9750 + Math.floor(Math.random() * 150);
const profile = tempProfile('creator-flow-dock-'); // removed on exit (chrome-profile.mjs)
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=1600,1000', 'about:blank'], { stdio: 'ignore' });
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
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description);
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
const q = (id) => `document.querySelector('[data-testid="${id}"]')`;
const go = async (url, ready) => {
	await send('Page.navigate', { url: `${CREATOR}${url}` });
	await until('document.readyState === "complete"');
	return until(ready, 30000);
};
/** client-side navigation (SvelteKit intercepts the click), like a crumb or a sidebar link */
const nav = (href) => js(`(() => { const a = document.createElement('a'); a.href = ${JSON.stringify(href)}; document.body.append(a); a.click(); a.remove(); return true; })()`);
/** the dock's Flow: its element is the one we tagged, its mount counter, the graph's zoom */
const flowState = () =>
	js(`(() => { const d = ${q('flow-dock')}; const vp = document.querySelector('.sc-shell-dock .svelte-flow__viewport'); const z = /scale\\(([\\d.]+)\\)/.exec(vp?.style.transform ?? '')?.[1]; return { here: !!d, same: !!d && d === window.__flowDock, mount: d?.dataset.mount ?? null, zoom: z ? Number(z) : null, graphSame: !!vp && vp === window.__flowViewport, level: document.querySelector('.sc-shell-dock [data-level]')?.dataset.level ?? null, nodes: document.querySelectorAll('.sc-shell-dock .svelte-flow__node').length }; })()`);
const near = (a, b) => a != null && b != null && Math.abs(a - b) < 0.02;
/** waits until the dock's Flow shows `level` with boxes and a settled zoom */
const flowAt = (level) => until(`(() => { const vp = document.querySelector('.sc-shell-dock .svelte-flow__viewport'); return document.querySelector('.sc-shell-dock [data-level="${level}"]') && document.querySelector('.sc-shell-dock .svelte-flow__node') && /scale/.test(vp?.style.transform ?? '') ? true : null; })()`, 30000).then(() => sleep(600));

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
	await go('/', 'true');
	await js(`localStorage.clear(); localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);

	// ── the toon's Flow, zoomed out twice ──
	check(await go(`/${TOON}?flow=1`, `!!${q('flow-dock')} && !!document.querySelector('.sc-shell-dock .svelte-flow__node')`), 'toon page: the Flow in the dock');
	await flowAt('toon');
	await js(`window.__flowDock = ${q('flow-dock')}, true`);
	const s0 = await flowState();
	await js(`${q('flow-zoom-out')}.click(), true`);
	await sleep(400);
	await js(`${q('flow-zoom-out')}.click(), true`);
	await sleep(600);
	const s1 = await flowState();
	check(s1.zoom != null && s1.zoom < (s0.zoom ?? 1) - 0.05, `zoomed out ${s0.zoom} → ${s1.zoom}`, JSON.stringify({ s0, s1 }));

	// ── toon → season → chapter: the same panel, the zoom kept ──
	await nav(`/${TOON}/season/1`);
	await flowAt('season');
	const s2 = await flowState();
	check(s2.same && s2.mount === s0.mount, `toon → season: the same Flow panel (mount #${s2.mount}, not remounted)`, JSON.stringify(s2));
	check(near(s2.zoom, s1.zoom), `toon → season: the season's graph at the kept zoom (${s2.zoom})`, JSON.stringify(s2));
	check(!(await js('location.search.includes("flow=1")')), 'season: no ?flow=1 needed in the address');

	await nav(`/${TOON}/chapter/1`);
	await flowAt('chapter');
	const s3 = await flowState();
	check(s3.same && s3.mount === s0.mount, `season → chapter: the same Flow panel (mount #${s3.mount})`, JSON.stringify(s3));
	check(near(s3.zoom, s1.zoom), `season → chapter: the chapter's graph at the kept zoom (${s3.zoom})`, JSON.stringify(s3));
	// a chapter with two groups (chapter → chapter: the same panel too)
	let chapter = 1;
	for (let n = 2; n <= 9 && !(await until(`${q('group-1')} ? true : null`, 4000)); n++) {
		await nav(`/${TOON}/chapter/${n}`);
		await flowAt('chapter');
		chapter = n;
	}
	const s3b = await flowState();
	check(s3b.same && s3b.mount === s0.mount && near(s3b.zoom, s1.zoom), `chapter → chapter ${chapter} (two groups): the same panel, zoom kept`, JSON.stringify(s3b));

	// ── in the chapter: chapter → group (a level: rebuilt, zoom kept), group → group (the graph itself stays) ──
	await js(`${q('group-0')}.click(), true`);
	await flowAt('group');
	const s4 = await flowState();
	check(s4.same && near(s4.zoom, s1.zoom), `chapter → group level: the same panel, zoom kept (${s4.zoom})`, JSON.stringify(s4));
	await js(`window.__flowViewport = document.querySelector('.sc-shell-dock .svelte-flow__viewport'), true`);
	await js(`${q('flow-zoom-in')}.click(), true`);
	await sleep(600);
	const s5 = await flowState();
	await js(`${q('group-1')}.click(), true`);
	await until(`location.search.includes('group=') && !location.search.includes('group=1&') ? true : null`, 3000);
	await sleep(800);
	const s6 = await flowState();
	check(s6.same && s6.graphSame && s6.level === 'group' && near(s6.zoom, s5.zoom), `group → group: the same panel AND graph, the zoom stays (${s5.zoom} → ${s6.zoom})`, JSON.stringify({ s5, s6 }));

	// ── a box navigates; the panel follows (chapter level → a box of the next chapter is not always there: back up) ──
	await nav(`/${TOON}`);
	await flowAt('toon');
	const s7 = await flowState();
	check(s7.same && s7.mount === s0.mount, `chapter → toon (a crumb): still the same panel (mount #${s7.mount})`, JSON.stringify(s7));
	const box = await js(`(() => { const n = [...document.querySelectorAll('.sc-shell-dock .svelte-flow__node')].find((e) => /kind-chapter/.test(e.querySelector('[class*="kind-"]')?.className ?? '')); if (!n) return null; n.querySelector('[class*="kind-"]').click(); return n.dataset.id; })()`);
	if (box) {
		await until(`location.pathname.includes('/chapter/') ? true : null`, 10000);
		await flowAt('chapter');
		const s8 = await flowState();
		check(s8.same && s8.mount === s0.mount, `a chapter box (${box}) opens its chapter, the panel follows (mount #${s8.mount})`, JSON.stringify(s8));
	} else check(false, 'a chapter box in the toon Flow');

	// ── a page without a Flow: the held one closes ──
	await nav(`/${TOON}/cast`);
	await until(`location.pathname.endsWith('/cast') && !${q('flow-dock')} ? true : null`, 15000);
	check(!(await js(`!!${q('flow-dock')}`)), 'the cast page (no Flow): the held Flow closes');

	// ── Problems: chapter → season the same panel ──
	// the bar's start (Problems) may sit in "More" (the chapter editor's ActionBar breakpoint)
	check(await go(`/${TOON}/chapter/1`, `!!${q('problems-toggle')} || !!document.querySelector('.sc-action-bar .more-toggle')`), 'chapter editor');
	await js(`(${q('problems-toggle')} ?? (document.querySelector('.sc-action-bar .more-toggle').click(), null)) ? true : null`);
	await until(`!!${q('problems-toggle')}`);
	await js(`${q('problems-toggle')}.click(), true`);
	await until(`!!${q('problems-panel')}`);
	await js(`window.__problems = document.querySelector('.problems-dock'), true`);
	await nav(`/${TOON}/season/1`);
	await until(`location.pathname.includes('/season/') && document.querySelector('[data-testid="season-meta"]') ? true : null`, 20000);
	await sleep(500);
	check(await js(`!!document.querySelector('.problems-dock') && document.querySelector('.problems-dock') === window.__problems`), 'chapter → season: Problems stay the same panel (the season\'s list)');

	// ── Ask reads the crumbs: context = the trail's names, ref = the deepest crumb's address ──
	check(await go(`/${TOON}/chapter/${chapter}`, `!!${q('group-1')}`), 'chapter editor for Ask');
	await js(`${q('group-1')}.click(), true`);
	await sleep(500);
	await js(`window.__askBody = null; const f = window.fetch; window.fetch = (u, o) => { if (o?.body && String(o.body).includes('"context"')) { window.__askBody = JSON.parse(o.body); return Promise.resolve(new Response('{}', { status: 500 })); } return f(u, o); }; true`);
	await js(`${q('ask-toggle')}.click(), true`);
	await until(`!!document.querySelector('[data-testid="ask-panel"] textarea')`);
	const crumbs = await js(`[...document.querySelectorAll('nav [href]')].filter((a) => a.closest('nav')?.getAttribute('aria-label')).map((a) => a.textContent.trim())`);
	await js(`(() => { const t = document.querySelector('[data-testid="ask-panel"] textarea'); t.value = 'where am I'; t.dispatchEvent(new Event('input', { bubbles: true })); t.form ? t.form.requestSubmit() : t.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return true; })()`);
	const body = await until('window.__askBody', 5000);
	check(body && /Season|Staffel/.test(body.context) && body.context.split(' · ').length >= 4 && new RegExp(`/chapter/${chapter}[?]group=`).test(body.ref ?? ''), `Ask: context "${body?.context}" (the trail with the season), ref ${body?.ref}`, JSON.stringify({ body, crumbs }));

	// ── the admin's Info (the ActionBar's, kind 'info'): codes → assets the same panel, the next page's help ──
	const ADMIN = (process.env.SCUTOONS_ADMIN ?? local('admin')).replace(/\/$/, '');
	await send('Page.navigate', { url: `${ADMIN}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), localStorage.removeItem('sc-info-open'), true`);
	await send('Page.navigate', { url: `${ADMIN}/codes` });
	check(await until(`!!${q('info-toggle')}`, 30000), 'admin codes page with Info');
	await js(`${q('info-toggle')}.click(), true`);
	await until(`!!${q('info-panel')}`);
	await js(`window.__info = ${q('info-panel')}, window.__infoText = ${q('info-panel')}.textContent, true`);
	await nav('/assets');
	await until(`location.pathname === '/assets' && ${q('info-panel')}?.textContent !== window.__infoText ? true : null`, 20000);
	await sleep(500);
	check(await js(`!!${q('info-panel')} && ${q('info-panel')} === window.__info`), 'admin codes → assets: Info stays the same panel, with the assets’ help');
	await js(`${q('info-toggle')}.click(), true`);

	if (errors.length) check(false, `page errors: ${errors.length}`, errors.slice(0, 3).join(' | '));
} catch (e) {
	check(false, 'run', e?.stack ?? String(e));
} finally {
	ws.close();
	chrome.kill();
}
const passed = results.filter(Boolean).length;
console.log(`\n${passed}/${results.length} passed`);
process.exit(passed === results.length ? 0 : 1);
