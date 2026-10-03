#!/usr/bin/env node
// Smoke check of the creator's chapter editor as it behaves today (agents/COMPONENTS.md 4.3 step 0: the safety net for
// the component refactor). Headless Chrome (own throw-away profile, CDP, no dependencies, like creator-cast-check)
// signed in as the local test user (agent@scutoons.test, `npm run dev:test-user`) on the running LOCAL dev stack
// (creator :10000, API :9000). Refuses any API / creator that is not on localhost – never production.
//
//   npm run check:creator-chapter
//   (another local stack, e.g. a worktree's side ports: SCUTOONS_API=http://localhost:19420/api
//    SCUTOONS_CREATOR=http://localhost:20420, like dev:test-user's SCUTOONS_API)
//
// Checks (catpope chapter 1, its first group, desktop 1440 × 900):
//   tree:    Audio / Meta select their section; a click on the group opens it (slice list), a slice opens the slice
//            editor (canvas + inspector) and the URL follows (?group=…&slice=2)
//   tabs:    each inspector tab activates and picks its canvas tool; each tool opens its tab
//   add:     "+ Add text" shows the presets; a preset adds a text box (canvas box, list entry, inspector, cursor in the
//            text field); typing marks the page dirty
//   snap:    snap on (default 30 px): an arrow moves the selected box onto the grid; snap off: 1 px, Shift: 10 px
//   undo:    outside a field, Ctrl+Z undoes the last nudge, Ctrl+Shift+Z / Ctrl+Y redo it; the Undo / Redo buttons too
//   keys:    Escape lets the selection go; Delete removes the selected box
//   slices:  next / previous slice (bar buttons) move the slice editor and the URL
//   save:    Save draft (note dialog skipped) stores the chapter draft with the new text; the published version stays
//   VN:      chapter 2's scene-14: each event type's line opens its form (+ "Only if"); "+ setVariable" adds one, open,
//            the cursor in it; a slice event has its trigger (never saved)
// Afterwards every entity the run changed is restored (content as before, published again if it was), also on failure.
// Refuses to run while the chapter has an unpublished draft (someone's work in progress is never touched).
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const TOON = 'catpope';
const CHAPTER = 1;
const GROUP = 0;
const SLICE = 1; // the first story slice (0 is the key visual)
const LOCALES = ['de', 'en'];
const CHAPTER_KEYS = LOCALES.map((l) => `chapter:${TOON}:${CHAPTER}:${l}`);
const TEXT = 'Regressionstext';
/** a chapter-2 scene with a group script and slice events (catpope) */
const VN_GROUP = 'scene-14';

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);
for (const url of [API, CREATOR]) {
	let host = '';
	try {
		host = new URL(url).hostname;
	} catch {}
	if (!LOCAL_HOSTS.has(host)) {
		console.error(`creator-chapter-check: refusing ${url} – local dev stack only`);
		process.exit(2);
	}
}

const results = [];
const check = (ok, label, detail = '') => {
	results.push(ok);
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${ok || !detail ? '' : `  — ${detail}`}`);
	return ok;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── session of the test user ─────────────────────────────────────────────────────────────────────────────
execFileSync(process.execPath, [resolve(ROOT, 'scripts/dev/test-user.mjs')], { stdio: ['ignore', 'ignore', 'inherit'] });
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

// ── baseline: every stored entity of the toon + the platform-wide ones; the chapter without pending drafts ──────
async function index() {
	const out = {};
	for (const toon of [TOON, '']) for (const e of (await api('GET', `/entities?toon=${toon}`)).entities) out[e.key] = e;
	return out;
}
const before = await index();
for (const key of CHAPTER_KEYS) {
	const e = before[key];
	if (!e || e.published !== e.latest) {
		console.error(`creator-chapter-check: ${key} has an unpublished draft or none published; not touching it. Publish or restore it first.`);
		process.exit(2);
	}
}
const baseline = {};
for (const key of CHAPTER_KEYS) baseline[key] = await api('GET', `/entities/${key}?draft=1&wrap=1`);
const slice = baseline[CHAPTER_KEYS[0]].content.groups[GROUP].slices[SLICE];
const sliceWidth = slice.width > 0 ? slice.width : 390;
const dialogsBefore = (slice.dialog ?? []).length;
const baselineGroupKey = baseline[CHAPTER_KEYS[0]].content.groups[GROUP].label ?? String(GROUP + 1);
if ((slice.events ?? []).length) console.log(`note: slice ${SLICE + 1} has events – the empty-slice event check is weaker`);

/** every entity whose newest version changed during the run gets its old content back (published again if it was) */
async function restore() {
	const after = await index();
	for (const [key, e] of Object.entries(after)) {
		const old = before[key];
		if (old && old.latest === e.latest) continue;
		if (!old) {
			check(false, `restore ${key}`, 'created during the run – remove it by hand');
			continue;
		}
		const was = await api('GET', `/entities/${key}?version=${old.latest}&wrap=1`);
		const now = await api('GET', `/entities/${key}?draft=1&wrap=1`);
		let version = now.version.id;
		if (JSON.stringify(now.content) !== JSON.stringify(was.content)) {
			version = (await api('PUT', `/entities/${key}`, { content: was.content, base: now.version.id, message: 'creator-chapter-check: restore' })).version.id;
		}
		if (old.published === old.latest && now.published?.id !== version) await api('POST', `/entities/${key}/publish`, { version });
		const done = await api('GET', `/entities/${key}?draft=1&wrap=1`);
		check(
			JSON.stringify(done.content) === JSON.stringify(was.content) && (old.published !== old.latest || done.published?.id === done.version.id),
			`restored ${key} (content as before${old.published === old.latest ? ', published, no pending draft' : ''})`
		);
	}
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
	console.error('creator-chapter-check: no Chrome/Edge found (set CHROME)');
	process.exit(2);
}
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-chapter-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
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
const writes = []; // non-GET API requests: { method, url, status }
const errors = [];
ws.addEventListener('message', (e) => {
	const msg = JSON.parse(e.data);
	if (msg.id && pending.has(msg.id)) {
		pending.get(msg.id)(msg);
		pending.delete(msg.id);
		return;
	}
	const p = msg.params;
	if (msg.method === 'Page.javascriptDialogOpening') send('Page.handleJavaScriptDialog', { accept: true });
	else if (msg.method === 'Runtime.exceptionThrown') errors.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text);
	else if (msg.method === 'Network.requestWillBeSent' && p.request.url.startsWith(API) && !['GET', 'OPTIONS'].includes(p.request.method))
		writes.push({ id: p.requestId, method: p.request.method, url: p.request.url.slice(API.length), status: null });
	else if (msg.method === 'Network.responseReceived') {
		const w = writes.find((x) => x.id === p.requestId);
		if (w) w.status = p.response.status;
	}
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
/** a real mouse click in the middle of the element */
async function click(selector) {
	const [x, y] = await js(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
	await sleep(250);
}
const tid = (testid) => `[data-testid="${testid}"]`;
const KEYS = {
	ArrowLeft: 37,
	ArrowUp: 38,
	ArrowRight: 39,
	ArrowDown: 40,
	Escape: 27,
	Delete: 46,
	z: 90,
	y: 89
};
/** a real key press; mods: 'ctrl', 'shift' */
async function press(key, ...mods) {
	const modifiers = (mods.includes('ctrl') ? 2 : 0) | (mods.includes('shift') ? 8 : 0);
	const code = key.length === 1 ? `Key${key.toUpperCase()}` : key;
	const base = { key: mods.includes('shift') && key.length === 1 ? key.toUpperCase() : key, code, windowsVirtualKeyCode: KEYS[key], modifiers };
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	await sleep(200);
}
/** an in-page dialog (confirmDialog / askDialog), when one opens: click its button */
async function answerDialog(button) {
	if (await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000)) await click(`dialog.sc-dialog[open] ${button}`);
}
const search = () => js('location.search');
const dialogCount = () => js(`document.querySelectorAll('[data-testid^="box-dialog-"]').length`);
/** the box's position in px of the slice's 1x size (from its left / top in %) */
const boxAt = (i) =>
	js(`(() => { const b = ${q(`box-dialog-${i}`)}; if (!b) return null; const c = ${q('slice-canvas')}.getBoundingClientRect(); return { x: parseFloat(b.style.left), y: parseFloat(b.style.top), w: c.width, h: c.height }; })()`).then((r) =>
		r ? { x: (r.x / 100) * SLICE_W, y: (r.y / 100) * SLICE_W * (r.h / r.w), xPct: r.x, yPct: r.y } : null
	);
const SLICE_W = sliceWidth;
const near = (a, b, eps = 0.15) => Math.abs(a - b) <= eps;
const writesSince = (n) => writes.slice(n);
/** a bar button; below the bar's breakpoint the secondary ones sit in "More" (opened first) */
async function barClick(testid) {
	if (!(await js(`!!${q(testid)}`))) await click('.sc-action-bar .more-toggle');
	await click(tid(testid));
}

await send('Runtime.enable');
await send('Network.enable');
await send('Page.enable');

let failed = false;
try {
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/${CHAPTER}` });
	check(await until(`!!${q('chapter-tree')} && !!${q(`group-${GROUP}`)}`, 30000), 'chapter editor loads signed in as the test user (tree with groups)');

	// ── tree ─────────────────────────────────────────────────────────────────────────────────────────────
	await click(tid('tree-audio'));
	check((await js(`${q('tree-audio')}.classList.contains('is-current')`)) && (await search()) === '?section=audio', 'tree: Audio selects the audio section (?section=audio)');
	await click(tid('tree-meta'));
	check(
		await until(`${q('tree-meta')}.classList.contains('is-current') && !${q('tree-audio')}.classList.contains('is-current') && location.search === ''`, 3000),
		'tree: Meta selects the meta section and clears the URL (Audio → Meta used to keep ?section=audio)',
		await search()
	);
	await click(tid(`group-${GROUP}`));
	check(
		(await until(`${q(`group-${GROUP}`)}.getAttribute('aria-expanded') === 'true' && !!${q('slice-list')}`)) && /[?&]group=/.test(await search()),
		'tree: a click on the group opens it (slice list, ?group=)',
		await search()
	);
	await click(tid(`slice-${SLICE}`));
	check(
		(await until(`!!${q('slice-canvas')} && !!${q('tab-text')}`)) && new URLSearchParams(await search()).get('slice') === String(SLICE + 1),
		`tree: a slice opens the slice editor (canvas + inspector, slice=${SLICE + 1})`,
		await search()
	);
	check((await dialogCount()) === dialogsBefore, `the canvas shows the slice's ${dialogsBefore} text box(es)`);

	// ── tabs ↔ tools ─────────────────────────────────────────────────────────────────────────────────────
	const pairs = [
		['tab-items', 'tool-item'],
		['tab-audio', 'tool-sample'],
		['tab-events', 'tool-event'],
		['tab-text', 'tool-text']
	];
	for (const [tab, tool] of pairs) {
		await click(tid(tab));
		check(
			(await js(`${q(tab)}.classList.contains('active') && ${q(tool)}.getAttribute('aria-pressed') === 'true'`)) === true,
			`tabs: ${tab} activates and picks ${tool}`
		);
	}
	await click(tid('tab-background'));
	check((await js(`${q('tab-background')}.classList.contains('active') && !!${q('slice-map')}`)) === true, 'tabs: the Slice tab shows the slice media + map');
	for (const [tab, tool] of [...pairs].reverse()) {
		await click(tid(tool));
		check((await js(`${q(tool)}.getAttribute('aria-pressed') === 'true' && ${q(tab)}.classList.contains('active')`)) === true, `tools: ${tool} opens ${tab}`);
	}

	await click(tid('tool-text'));

	// ── add a text ───────────────────────────────────────────────────────────────────────────────────────
	await click(tid('add-text'));
	check(await until(`!!${q('text-presets')}`, 3000), 'add: "+ Add text" shows the presets');
	await click(tid('text-preset-speech'));
	const added = dialogsBefore;
	check(
		(await until(`!!${q(`box-dialog-${added}`)} && !!${q(`dialog-${added}`)} && !!${q('dialog-inspector')}`, 3000)) && !(await js(`!!${q('text-presets')}`)),
		'add: a preset adds a text box (canvas box, list entry, inspector open, presets closed)'
	);
	check(await until(`document.activeElement?.dataset?.testid === 'text-de'`, 3000), 'add: the cursor is in the new text field');
	await send('Input.insertText', { text: TEXT });
	await sleep(300);
	check(await until(`!!document.querySelector('.dirty-bar') && ${q('save-draft')}.textContent.includes('•')`, 3000), 'add: typing marks the page dirty');

	// ── snap + nudge (outside a field: the list entry has the focus) ──────────────────────────────────────
	await click(tid(`dialog-${added}`));
	check((await js(`document.activeElement?.dataset?.testid`)) === `dialog-${added}` && (await js(`${q(`box-dialog-${added}`)}.classList.contains('selected')`)), 'the list entry selects the box (focus outside a field)');
	check((await js(`${q('snap-toggle')}.getAttribute('aria-pressed') === 'true' && ${q('grid-size')}.value === '30'`)) === true, 'snap: on by default, 30 px grid');
	await sleep(1200); // separate undo steps (checkpoints within ~1 s merge)
	const p0 = await boxAt(added);
	await press('ArrowRight');
	const p1 = await boxAt(added);
	const onGrid = (v) => near(v / 30, Math.round(v / 30), 0.02);
	check(p1.x > p0.x && onGrid(p1.x) && near(p1.y, p0.y, 0.01), 'snap: ArrowRight moves the box onto the 30 px grid', JSON.stringify({ p0, p1 }));
	await press('ArrowDown');
	const p2 = await boxAt(added);
	check(p2.y > p1.y && near(p2.x, p1.x, 0.01), 'nudge: ArrowDown moves the box down', JSON.stringify({ p1, p2 }));
	await click(tid('snap-toggle'));
	check((await js(`${q('snap-toggle')}.getAttribute('aria-pressed') === 'false' && ${q('grid-size')}.disabled`)) === true, 'snap: the toggle switches it off (grid size disabled)');
	await click(tid(`dialog-${added}`));
	await sleep(1200);
	await press('ArrowRight');
	const p3 = await boxAt(added);
	check(near(p3.x - p2.x, 1), 'snap off: ArrowRight moves 1 px', JSON.stringify({ p2, p3 }));
	await sleep(1200);
	await press('ArrowLeft', 'shift');
	const p4 = await boxAt(added);
	check(near(p3.x - p4.x, 10), 'snap off: Shift+ArrowLeft moves 10 px', JSON.stringify({ p3, p4 }));
	await click(tid('snap-toggle'));
	check((await js(`${q('snap-toggle')}.getAttribute('aria-pressed') === 'true'`)) === true, 'snap: the toggle switches it on again');

	// ── undo / redo outside a field ─────────────────────────────────────────────────────────────────────
	await click(tid(`dialog-${added}`));
	await press('z', 'ctrl');
	const u1 = await boxAt(added);
	check(near(u1.x, p3.x) && near(u1.y, p3.y), 'undo: Ctrl+Z undoes the last nudge', JSON.stringify({ p3, u1 }));
	await press('z', 'ctrl', 'shift');
	const r1 = await boxAt(added);
	check(near(r1.x, p4.x), 'redo: Ctrl+Shift+Z redoes it', JSON.stringify({ p4, r1 }));
	await press('z', 'ctrl');
	await press('y', 'ctrl');
	const r2 = await boxAt(added);
	check(near(r2.x, p4.x), 'redo: Ctrl+Y redoes it too', JSON.stringify({ p4, r2 }));
	await barClick('undo');
	const u2 = await boxAt(added);
	await barClick('redo');
	const r3 = await boxAt(added);
	check(near(u2.x, p3.x) && near(r3.x, p4.x), 'undo / redo: the bar buttons do the same', JSON.stringify({ u2, r3 }));
	check((await js(`document.querySelector('[data-testid="text-de"]')?.value ?? ''`)) === TEXT || (await js(`${q(`dialog-${added}`)}.textContent.includes(${JSON.stringify(TEXT)})`)), 'undo of a nudge keeps the typed text');

	// ── Escape / Delete ─────────────────────────────────────────────────────────────────────────────────
	await click(tid(`dialog-${added}`));
	await press('Escape');
	check((await js(`!document.querySelector('[data-testid^="box-dialog-"].selected') && !${q('dialog-inspector')}`)) === true, 'Escape lets the selection go (no selected box, no inspector)');
	const n0 = await dialogCount();
	await press('Delete');
	check((await dialogCount()) === n0, 'Delete without a selection removes nothing');
	await click(tid(`dialog-${added}`));
	await press('Delete');
	check((await dialogCount()) === n0 - 1 && !(await js(`!!${q(`dialog-${added}`)}`)), 'Delete removes the selected text box');
	await barClick('undo');
	check((await dialogCount()) === n0, 'undo brings the deleted box back');

	// ── prev / next slice ───────────────────────────────────────────────────────────────────────────────
	await click(tid('next-slice'));
	check(new URLSearchParams(await search()).get('slice') === String(SLICE + 2) && (await js(`document.querySelector('.slice-nav.in-end')?.textContent.includes('${SLICE + 2} /')`)), 'next slice: the slice editor and the URL move on', await search());
	check((await dialogCount()) === (baseline[CHAPTER_KEYS[0]].content.groups[GROUP].slices[SLICE + 1].dialog ?? []).length, 'next slice: the canvas shows that slice');
	await click(tid('prev-slice'));
	check(new URLSearchParams(await search()).get('slice') === String(SLICE + 1) && (await dialogCount()) === n0, 'previous slice: back, with the new text box', await search());

	// ── save draft ──────────────────────────────────────────────────────────────────────────────────────
	const n = writes.length;
	await click(tid('save-draft'));
	await answerDialog('button.secondary'); // the note dialog: save without a note
	await until(`!document.querySelector('.dirty-bar') && !${q('save-draft')}.textContent.includes('•')`);
	const saved = writesSince(n).map((w) => `${w.method} ${w.url} ${w.status}`);
	check(saved.includes(`PUT /entities/${CHAPTER_KEYS[0]} 200`) && !saved.some((w) => /publish/.test(w)), 'Save draft: PUT the chapter (de) 200, nothing published', JSON.stringify(saved));
	const draft = await api('GET', `/entities/${CHAPTER_KEYS[0]}?draft=1&wrap=1`);
	const savedDialogs = draft.content.groups[GROUP].slices[SLICE].dialog ?? [];
	check(
		savedDialogs.length === dialogsBefore + 1 && JSON.stringify(savedDialogs[dialogsBefore]).includes(TEXT) && draft.published?.id === baseline[CHAPTER_KEYS[0]].version.id,
		'the draft has the new text box; the published version stays',
		JSON.stringify({ n: savedDialogs.length, published: draft.published })
	);

	// ── VN events (step 8: one form per type): chapter 2's first scene – its group script, then a slice's events ──
	// edits here are never saved (the page is left with them; nothing is written)
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/2?group=${VN_GROUP}` });
	check(await until(`!!${q('vn-events')} && !!${q('event-line-0')}`, 30000), `VN: the group script of ${VN_GROUP} lists its events`);
	const typeIndex = await js(`(() => { const out = {}; document.querySelectorAll('[data-testid^="event-line-"]').forEach((b) => { const t = b.querySelector('.type')?.textContent; if (t && !(t in out)) out[t] = Number(b.dataset.testid.slice(11)); }); return out; })()`);
	/** what each type's form shows once its line is opened */
	const FORMS = {
		showMessage: (i) => `!!document.querySelector('[data-testid="event-${i}"] textarea') && !!document.querySelector('[data-testid="event-${i}"] .sc-fields')`,
		joinScene: (i) => `!!document.querySelector('[data-testid="event-${i}"] [data-testid="character-ref-open"]')`,
		addChoice: (i) => `!!document.querySelector('[data-testid="choice-actions-${i}"]')`,
		setVariable: (i) => `document.querySelectorAll('[data-testid="event-${i}"] .sc-fields input, [data-testid="event-${i}"] .sc-fields select').length >= 3`,
		label: (i) => `!!document.querySelector('[data-testid="event-${i}"] .sc-fields input')`,
		divert: (i) => `!!document.querySelector('[data-testid="event-${i}"] .sc-fields select')`,
		jumpTo: (i) => `document.querySelectorAll('[data-testid="event-${i}"] .sc-fields select').length >= 1`
	};
	for (const [type, ready] of Object.entries(FORMS)) {
		const i = typeIndex[type];
		if (i === undefined) continue;
		await click(tid(`event-line-${i}`));
		check(
			(await until(`${q(`event-line-${i}`)}.getAttribute('aria-expanded') === 'true' && (${ready(i)}) && (!!${q(`add-condition-${i}`)} || !!document.querySelector('[data-testid="event-${i}"] details.only-if[open]'))`, 3000)) === true,
			`VN: a ${type} line opens its form (+ "Only if")`
		);
		await click(tid(`event-line-${i}`));
	}
	const lines = await js(`document.querySelectorAll('[data-testid^="event-line-"]').length`);
	await click(`[data-testid="vn-events"] .add button:last-child`); // + setVariable
	check(
		(await until(`document.querySelectorAll('[data-testid^="event-line-"]').length === ${lines + 1} && ${q(`event-line-${lines}`)}.getAttribute('aria-expanded') === 'true'`, 3000)) &&
			(await js(`document.activeElement?.closest('[data-testid="event-${lines}"]') !== null && document.activeElement?.tagName === 'INPUT'`)),
		'VN: "+ setVariable" adds an event, opened, the cursor in its first field'
	);
	// a slice without events: "+ event" shows the new one (it used to land outside the reactive list)
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/${CHAPTER}?group=${encodeURIComponent(baselineGroupKey)}&slice=${SLICE + 1}` });
	await until(`!!${q('tab-events')}`, 30000);
	await click(tid('tab-events'));
	await until(`!!${q('vn-events')}`, 5000);
	await click(`[data-testid="vn-events"] .add button:last-child`);
	check(await until(`!!${q('event-line-0')} && ${q('tab-events')}.textContent.includes('(1)')`, 3000), 'VN: "+ event" on a slice without events shows the new event');
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/2?group=${VN_GROUP}&slice=1` });
	await until(`!!${q('tab-events')}`, 30000);
	await click(tid('tab-events'));
	check(await until(`!!${q('event-line-0')} && !!${q('vn-events')}`, 5000), 'VN: the slice events tab lists the slice\'s events');
	await click(tid('event-line-0'));
	check(await until(`!!${q('event-whole-0')}`, 3000), 'VN: a slice event has its trigger (whole slice / area)');
	check(errors.length === 0, 'no page errors', errors.join(' | '));
} catch (e) {
	failed = true;
	check(false, 'run', e?.stack ?? String(e));
} finally {
	await restore().catch((e) => check(false, 'restore', String(e)));
}
const bad = results.filter((r) => !r).length;
console.log(bad || failed ? `creator-chapter-check: ${bad} failed` : `creator-chapter-check: all ${results.length} passed`);
process.exit(bad || failed ? 1 : 0);
