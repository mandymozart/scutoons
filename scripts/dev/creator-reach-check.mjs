#!/usr/bin/env node
// The creator's reachability labels and the "next group needs a label" reminder (owner 2026-10-06, @scutoons/content
// reach.ts, agents/DATA.md "Linked groups") in headless Chrome as the local test user: a test chain is saved as a DRAFT
// of sandbox chapter 8, the chapter editor shows it, and the draft rows are removed again afterwards (the dev DB is as
// before). Nothing is saved from the editor.
//   - group A's script always jumps and continues into group B (no label): the reminder + "Give it a label"; B is
//     labelled "unreachable" in the tree; A's event after the jump is "unreachable"; a test of a variable set nowhere
//     in the toon is "never true"; a divert loop with no way out is "endless loop" – each also a Problems warning
//   - the Flow panel (read-only graph) follows the selection (group level), and on the chapter level shows the same
//     statuses, the jump's edge, and a box opens its group; its first view is 1:1, edge labels don't overlap, no intro line
//   - the season's and the toon's Flow dock on the right and push the page; the dead things are words
//
//   node scripts/dev/creator-reach-check.mjs        (creator :10000, API :9000, MariaDB container)
//   REACH_SHOT=<file.png>: a screenshot of the group section
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
for (const url of [API, CREATOR]) {
	if (!['localhost', '127.0.0.1'].includes(new URL(url).hostname)) {
		console.error(`creator-reach-check: refusing ${url} – local dev stack only`);
		process.exit(2);
	}
}
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const KEY = 'chapter:sandbox:8';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const t = (de, en = de) => ({ de, en });
const slice = (n) => ({ image: `../catpope/s1/c2/s1c2-slice-${n}.webp`, width: 390, height: 625 });
const msg = (text, extra = {}) => ({ type: 'showMessage', params: { text: t(text), wait: true }, ...extra });
// A jumps to C (so C – labelled, something leads there – is reachable)
const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '8', label: 'reach-c' } } };
const content = (base) => ({
	...base,
	groups: [
		{
			label: 'reach-a',
			continues: true,
			renderer: { type: 'vertical' },
			slices: [slice(9)],
			events: [msg('A1', { if: { variable: 'reach_check_set_nowhere', op: 'set' } }), jump, msg('after the jump')]
		},
		{ renderer: { type: 'vertical' }, slices: [slice(10)] },
		{
			label: 'reach-c',
			renderer: { type: 'vertical' },
			slices: [slice(11)],
			events: [{ type: 'label', params: { name: 'round' } }, msg('C1'), { type: 'divert', params: { to: 'round' } }]
		}
	]
});

async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
const sql = (query) => execFileSync('docker', ['exec', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N ${DB} -e "${query}"`]).toString().trim();
const before = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
const created = [];
const saved = await api('PUT', `/entities/${KEY}`, { content: content(before.content), base: before.version.id, message: 'creator-reach-check (removed again)' });
created.push(saved.version.id);

const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-reach-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
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

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await send('Page.navigate', { url: `${CREATOR}/sandbox/chapter/8?group=reach-a` });
	check(await until(`!!${q('chapter-tree')} && !!${q('vn-events')}`, 30000), 'the chapter editor opens group reach-a (its script)');
	// the toon's documents load in the background: the "never true" label needs them
	const chips = await until(`(() => { const t = (s) => [...document.querySelectorAll(s)].map((x) => x.innerText.trim()); const c = { tree1: t('[data-testid="group-1"] .sc-chip'), tree2: t('[data-testid="group-2"] .sc-chip'), events: [0, 1, 2].map((i) => t('[data-testid="event-' + i + '"] .sc-chip')) }; return c.events[0].length ? c : null; })()`, 20000);
	check(chips?.tree1.some((x) => /unreachable|unerreichbar/i.test(x)) && !chips?.tree2.length, 'tree: group 2 (after A, which always jumps; no label) is labelled unreachable, C (A jumps there) is not', JSON.stringify(chips));
	check(chips?.events[0].some((x) => /never true|nie wahr/i.test(x)), 'event 1: its test of a variable set nowhere is "never true"', JSON.stringify(chips?.events));
	check(chips?.events[1].length === 0, 'event 2 (the jump) has no label');
	check(chips?.events[2].some((x) => /unreachable|unerreichbar/i.test(x)), 'event 3 (after the jump) is "unreachable"', JSON.stringify(chips?.events));
	check(!!(await js(`!!${q('next-label-hint')}`)), 'the reminder: the next group needs a label');
	if (process.env.REACH_SHOT) writeFileSync(process.env.REACH_SHOT, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
	await js(`${q('next-label-add')}.click(), true`);
	const labelled = await until(`!${q('next-label-hint')} ? [...document.querySelectorAll('[data-testid="group-1"]')].map((b) => b.innerText).join('') : null`, 5000);
	check(labelled && /scene-2/.test(labelled), `"Give it a label": the next group is named, the reminder goes (${labelled?.trim().split(/\s+/)[0]})`);
	// group C: the loop
	await js(`${q('group-2')}.click(), true`);
	const loop = await until(`(() => { const c = [...document.querySelectorAll('[data-testid="event-2"] .sc-chip')].map((x) => x.innerText.trim()); return c.length ? c : null; })()`, 8000);
	check(loop?.some((x) => /endless loop|endlosschleife/i.test(x)), 'group C: the divert back with no way out is "endless loop"', JSON.stringify(loop));
	// the Flow panel (owner 2026-10-06, read-only): the same statuses as the chips; a box opens its group
	await js(`${q('flow-toggle')}.click(), true`);
	// the level follows the selection (group C open: its group level, every event of its script); then up to the chapter
	const groupLevel = await until(`document.querySelector('[data-testid="flow-panel"]')?.dataset.level === 'group' && document.querySelector('[data-testid="flow-node-g2e1"]')?.className.includes('kind-run') ? true : null`, 20000);
	check(groupLevel, 'Flow panel: group C open – its group level (the message as a run)');
	await js(`${q('flow-level-chapter')}.click(), true`);
	const flow = await until(`(() => { const c = (id) => document.querySelector('[data-testid="flow-node-' + id + '"]')?.className ?? ''; return c('g0') && c('g2e2') ? { a: c('g0'), b: c('g1'), c: c('g2'), jump: c('g0e1'), loop: c('g2e2') } : null; })()`, 20000);
	check(/is-reachable/.test(flow?.a) && /is-unreachable/.test(flow?.b) && /is-reachable/.test(flow?.c) && /is-reachable/.test(flow?.jump), 'Flow panel: A and C reachable, group 2 unreachable (like the tree)', JSON.stringify(flow));
	check(/is-loop/.test(flow?.loop), 'Flow panel: the divert back in C is an endless loop', JSON.stringify(flow));
	const edge = await js(`[...document.querySelectorAll('.svelte-flow__edge')].some((e) => e.getAttribute('data-id')?.startsWith('g0e1>g2'))`);
	check(edge, 'Flow panel: the jump leads to group C');
	await js(`${q('group-0')}.click(), true`);
	await sleep(500);
	await js(`document.querySelector('[data-testid="flow-node-g2"]').click(), true`);
	check(await until(`location.search.includes('group=reach-c') || null`, 5000), 'Flow panel: a box opens its group in the editor');
	// labels of edges never overlap (dagre lays them out like boxes); the first view is readable (1:1); no intro line
	await js(`${q('group-0')}.click(), true`);
	await sleep(300);
	await js(`${q('flow-level-chapter')}?.click(), true`);
	const view = await until(`(() => { const vp = document.querySelector('.svelte-flow__viewport'); const labels = [...document.querySelectorAll('.svelte-flow__edge-label')].map((l) => l.getBoundingClientRect()); if (!vp || !document.querySelector('.svelte-flow__node')) return null; let overlaps = 0; for (let i = 0; i < labels.length; i++) for (let j = i + 1; j < labels.length; j++) { const a = labels[i], b = labels[j]; if (a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom) overlaps++; } return { transform: vp.style.transform, overlaps, hint: !!document.querySelector('[data-testid="flow-panel"] p.hint:not(.error)') }; })()`, 10000);
	check(view && /scale\(1\)/.test(view.transform) && view.overlaps === 0 && !view.hint, 'Flow panel: the first view at 1:1, no edge labels on top of each other, no intro line', JSON.stringify(view));
	// the season's and the toon's Flow dock on the right and push the page (like the chapter's Flow and the Ask panel)
	const pushed = (sel) => `(() => { const dock = document.querySelector('[data-testid="toon-flow"]')?.closest('aside'); const page = document.querySelector('${sel}'); if (!dock || !page || !document.querySelector('.svelte-flow__node')) return null; const d = dock.getBoundingClientRect(), p = page.getBoundingClientRect(); return { dockLeft: Math.round(d.left), pageRight: Math.round(p.right), zoom: document.querySelector('.svelte-flow__viewport')?.style.transform, dead: document.querySelector('[data-testid="flow-dead"] summary')?.textContent ?? '' }; })()`;
	for (const [what, url, sel] of [['season', `${CREATOR}/sandbox/season/1?flow=1`, '.meta-page .content'], ['toon', `${CREATOR}/sandbox?flow=1`, '.creator-page']]) {
		await send('Page.navigate', { url });
		const g = await until(pushed(sel), 30000);
		check(g && g.pageRight <= g.dockLeft + 1 && /scale\(1\)/.test(g.zoom), `${what} Flow: docked, the page makes room (page ends ${g?.pageRight}, dock starts ${g?.dockLeft}), first view 1:1`, JSON.stringify(g));
		// the dead things in words ("Unused: 1 variable · 9 labels"), never bare numbers
		check(!g?.dead || !/:\s*\d+\s*·\s*\d+/.test(g.dead), `${what} Flow: the dead-things line in words`, g?.dead);
	}
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	check(false, 'the run', String(error?.stack ?? error));
} finally {
	try { chrome.kill(); } catch {}
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	sql(`DELETE FROM entity_versions WHERE entity_key = '${KEY}' AND id IN (${created.join(',')})`);
	const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	check(after.version.id === before.version.id && JSON.stringify(after.content) === JSON.stringify(before.content), `restored ${KEY} (newest version ${after.version.id}, was ${before.version.id})`);
}
const bad = results.filter((r) => !r).length;
console.log(bad ? `creator-reach-check: ${bad} failed` : `creator-reach-check: all ${results.length} passed`);
process.exit(bad ? 1 : 0);
