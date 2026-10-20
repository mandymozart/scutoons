#!/usr/bin/env node
// Smoke check of the creator's chapter editor as it behaves today (agents/COMPONENTS.md 4.3 step 0: the safety net for
// the component refactor). Headless Chrome (own throw-away profile, CDP, no dependencies, like creator-cast-check)
// signed in as the local test user (agent@scutoons.test, `npm run dev:test-user`) on the running LOCAL dev stack
// (creator :10000, API :9000). Refuses any API / creator that is not on localhost – never production.
//
//   npm run check:creator-chapter      (SCUTOONS_CHAPTER_TOON / SCUTOONS_CHAPTER: another toon / chapter, e.g. while
//                                        catpope's chapter 1 has someone's draft; SCUTOONS_CHAPTER_GROUP=2
//                                        SCUTOONS_CHAPTER_SLICE=2: the group / slice as the URL numbers them)
//   npm run check:creator-chapter -- --pieces   only the collage pieces part (the phone first; + save, reload, restore) – e.g. on
//                                        SCUTOONS_CHAPTER=3 SCUTOONS_CHAPTER_GROUP=2, a group with inline slices
//   (another local stack, e.g. a worktree's side ports: SCUTOONS_API=http://localhost:19420/api
//    SCUTOONS_CREATOR=http://localhost:20420, like dev:test-user's SCUTOONS_API)
//   The run reads inline slices (a group that is a scene ref – every sandbox chapter's – it can't) and wants a slice with
//   a text box or item (layers) and a group of ≥ 3 slices (reorder) – catpope 1's first group; catpope 3
//   (SCUTOONS_CHAPTER=3 SCUTOONS_CHAPTER_GROUP=2 SCUTOONS_CHAPTER_SLICE=1: inline slices, a next slice) has neither, so
//   those 2 checks fail there and the rest runs
//
// Checks (catpope chapter 1, its first group, desktop 1440 × 900):
//   tree:    Audio / Meta select their section; a click on the group opens it (slice list), a slice opens the slice
//            editor (canvas + inspector) and the URL follows (?group=…&slice=2)
//   tabs:    each inspector tab activates and picks its canvas tool; each tool opens its tab (text + item: Layers)
//   add:     "+ Add text" shows the presets; a preset adds a text box (canvas box, list entry, inspector, cursor in the
//            text field); typing marks the page dirty
//   layers:  the Layers tab's list (text boxes + placed items, BACKLOG 10.43, + click / hover areas, 10.54) is the canvas's
//            drawn and pointer order, the new box on
//            top; Alt+ArrowDown on its grip moves it one layer back in the list and on the canvas, one undo brings it back
//   snap:    snap on (default 30 px): an arrow moves the selected box onto the grid; snap off: 1 px, Shift: 10 px
//   undo:    outside a field, Ctrl+Z undoes the last nudge, Ctrl+Shift+Z / Ctrl+Y redo it; the Undo / Redo buttons too
//   keys:    Escape lets the selection go; Delete removes the selected box; the keys act on the live region only
//            (BACKLOG 10.40): with the slice list live, Delete / arrows / Esc leave the canvas selection alone
//   slices:  next / previous slice (bar buttons) move the slice editor and the URL
//   reorder: the shared Sortable (BACKLOG 10.53) – dragging a slice's grip moves it, one undo brings it back; Alt+ArrowDown
//            on a grip moves it one down (the focus follows), one undo again; the same for the groups when there are two
//   E.1:     no unsaved bar / file-status row in the flow; the Save button's dot; no not-live chip – Publish carries
//            the dot once the draft is saved; Saved = a toast
//   save:    Save draft (the note dialog's suggestion) stores the chapter's language-neutral draft (chapter:catpope:1) with the
//            new text; the published version stays; Publish asks first – its checklist names the chapter, ticked (when
//            the chapter's content has errors, the "Publish anyway?" step comes first: the run prints them and goes on)
//   phone:   375 × 812 touch, the inspector sheet closed: the item tool and the piece tool – a tap on the slice shows
//            the picker on screen (not a 0 × 0 modal that freezes the page), a tap on a card places it; both undone
//   pieces:  the collage piece tool (research/COLLAGE.md §3): the tool → a click → the picker → a shape on the picture,
//            the look Negative = the picture's negative there (a pixel), the Fade ramp opaque in its middle, the ramp's
//            dot, one undo per step, Ctrl+C / Ctrl+V, Layers (pieces under the stack; a reorder, its undo / redo keep the
//            moved piece selected), Delete; saved with the draft; the preset grid previews every look on the piece's
//            file, "Save as preset …" keeps its pixels, the toast's Undo leaves no presets document to save (nor its
//            local autosave); an unsaved preset left by a reload is offered at load ("Collage presets" next to the
//            chapter), Restore on both brings the piece's preset back (--pieces only)
//   VN:      a group with a script + slice events (catpope's, else the sandbox's): each event type's line opens its form (+ "Only if"); "+ setVariable" adds one, open,
//            the cursor in it; "+ event" on the chapter's slice adds one line; a slice event has its trigger (never saved)
// Afterwards every entity the run changed is restored (content as before, published again if it was), also on failure.
// Refuses to run while the chapter has an unpublished draft (someone's work in progress is never touched).
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';
import { png } from './endless-page.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const TOON = process.env.SCUTOONS_CHAPTER_TOON ?? 'catpope';
const CHAPTER = Number(process.env.SCUTOONS_CHAPTER ?? 1);
const GROUP = Number(process.env.SCUTOONS_CHAPTER_GROUP ?? 1) - 1;
const SLICE = Number(process.env.SCUTOONS_CHAPTER_SLICE ?? 2) - 1; // the first story slice (1 is the key visual)
/** only the collage pieces part (it saves, reloads and restores too) */
const ONLY_PIECES = process.argv.includes('--pieces');
/** the chapter's one language-neutral document (agents/NEUTRAL.md) */
const CHAPTER_KEYS = [`chapter:${TOON}:${CHAPTER}`];
const TEXT = 'Regressionstext';

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

/**
 * The VN part's group: one with a group script and a slice with events – catpope's chapters first, else the sandbox's
 * (generated, always has them; the dev content changes). Groups are inline in their chapter since 2026-10-19 (step 3 of
 * agents/research/TEXT-ACROSS-SLICES.md – no scene documents any more)
 */
async function findVnScene() {
	for (const toon of [TOON, 'sandbox']) {
		const keys = (await api('GET', `/entities?toon=${toon}`)).entities.map((e) => e.key).filter((k) => /^chapter:[^:]+:\d+$/.test(k)).sort();
		for (const key of keys) {
			const doc = (await api('GET', `/entities/${key}?draft=1&wrap=1`)).content;
			for (const [gi, g] of (doc?.groups ?? []).entries()) {
				const slice = (g?.slices ?? []).findIndex((s) => (s?.events ?? []).length > 0);
				if ((g?.events ?? []).length && slice >= 0) return { key, toon, chapter: Number(key.split(':')[2]), group: g.label ?? String(gi + 1), slice: slice + 1 };
			}
		}
	}
	return null;
}
const VN = await findVnScene();
if (!VN) console.log('note: no group with a script and slice events – the VN part is skipped');

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
	y: 89,
	c: 67,
	v: 86
};
/** a real key press; mods: 'ctrl', 'shift' */
async function press(key, ...mods) {
	const modifiers = (mods.includes('alt') ? 1 : 0) | (mods.includes('ctrl') ? 2 : 0) | (mods.includes('shift') ? 8 : 0);
	const code = key.length === 1 ? `Key${key.toUpperCase()}` : key;
	const base = { key: mods.includes('shift') && key.length === 1 ? key.toUpperCase() : key, code, windowsVirtualKeyCode: KEYS[key], modifiers };
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	await sleep(200);
}
/** a real mouse drag of a Sortable grip onto the upper quarter of the target (it lands before it) */
async function dragGrip(grip, target) {
	const [x, y] = await js(`(() => { const el = document.querySelector(${JSON.stringify(grip)}); el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	const ty = await js(`(() => { const r = document.querySelector(${JSON.stringify(target)}).getBoundingClientRect(); return r.y + r.height / 4; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
	for (let i = 1; i <= 8; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y: y + ((ty - y) * i) / 8, button: 'left', buttons: 1 });
	await sleep(100);
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y: ty, button: 'left', buttons: 0, clickCount: 1 });
	await sleep(300);
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
/** a bar button; the grouped bar (UI-CONCEPTS E.10, every width since 2026-10-12) keeps undo / redo in its History
 * group and the rest in More – the group is opened first (undo / redo keep it open). The main row's own cells (Save,
 * Publish, Problems, the desktop's slice stepper) are hidden while a group is open: its ‹ goes back first */
async function barClick(testid) {
	if (!(await js(`!!${q(testid)}`)) && (await js(`!!${q('phone-bar-back')}`))) await click(tid('phone-bar-back'));
	if (!(await js(`!!${q(testid)}`))) await click(tid(['undo', 'redo'].includes(testid) ? 'phone-bar-history' : 'phone-bar-more'));
	await click(tid(testid));
}

// ── collage pieces (research/COLLAGE.md §3, §8 step 4) ─────────────────────────────────────────────────────
const PIECES = '[data-testid="slice-canvas"] .slice-media .slice-pieces .piece';
const pieceCount = () => js(`document.querySelectorAll(${JSON.stringify(PIECES)}).length`);
const pieceIds = () => js(`[...document.querySelectorAll(${JSON.stringify(PIECES)})].map((p) => p.dataset.piece)`);
/** the file a catpope shape is stored as in this toon (toonAssetRef) */
const DISC = TOON === 'catpope' ? 'vectors/disc-black.svg' : '../catpope/vectors/disc-black.svg';
const frames = () => js('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))');
/** the colour at a viewport point (CSS px, DPR 1), the canvas overlay and the lettering hidden (and the pieces: `pieces` false) */
async function pixelAt(x, y, pieces = true) {
	const hide = `[data-testid="slice-canvas"] .overlay, [data-testid="slice-canvas"] .slice-layers${pieces ? '' : ', [data-testid="slice-canvas"] .slice-pieces'}`;
	await js(`(() => { const s = document.createElement('style'); s.id = 'probe-hide'; s.textContent = ${JSON.stringify(`${hide} { visibility: hidden !important }`)}; document.head.append(s); })()`);
	await frames();
	const [sx, sy] = await js('[scrollX, scrollY]');
	const shot = await send('Page.captureScreenshot', { format: 'png', clip: { x: Math.round(x + sx) - 1, y: Math.round(y + sy) - 1, width: 3, height: 3, scale: 1 } });
	await js(`document.getElementById('probe-hide')?.remove()`);
	return png(Buffer.from(shot.result.data, 'base64')).at(1, 1);
}
const off = (a, b) => Math.max(...a.map((v, i) => Math.abs(v - b[i])));
const negative = (c) => c.map((v) => 255 - v);
/** polls the pixel until it is within `tol` of `want` (a mask / picture still loading); the last reading */
async function pixelUntil(x, y, want, tol, ms = 4000) {
	const end = Date.now() + ms;
	let c = await pixelAt(x, y);
	while (off(c, want) > tol && Date.now() < end) {
		await sleep(200);
		c = await pixelAt(x, y);
	}
	return c;
}
/**
 * a spot of the canvas without a box on it (a click there places, never selects) – the brightest of a few, so the
 * dark disc stands out against the picture (the ramp check)
 */
async function emptySpot() {
	const spots = await js(`(() => {
		const o = document.querySelector('[data-testid="slice-canvas"] .overlay');
		const r = o.getBoundingClientRect();
		const out = [];
		for (const fy of [0.5, 0.35, 0.65]) for (const fx of [0.5, 0.35, 0.65]) {
			const x = r.x + r.width * fx, y = r.y + r.height * fy;
			if (document.elementFromPoint(x, y) === o) out.push([x, y]);
		}
		return out;
	})()`);
	let best = null;
	let light = -1;
	for (const spot of spots) {
		const c = await pixelAt(...spot);
		const l = c[0] + c[1] + c[2];
		if (l > light) [best, light] = [spot, l];
	}
	return best;
}
async function mouseAt(x, y) {
	for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
	await sleep(250);
}
/** a real mouse drag from one viewport point to another */
async function mouseDrag([x, y], [tx, ty], steps = 8) {
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
	for (let i = 1; i <= steps; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + ((tx - x) * i) / steps, y: y + ((ty - y) * i) / steps, button: 'left', buttons: 1 });
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: tx, y: ty, button: 'left', buttons: 0, clickCount: 1 });
	await sleep(300);
}
/** Ctrl+C / Ctrl+V as the browser's copy / paste commands (the clipboard runtime listens to the DOM events) */
async function clipKey(key) {
	const base = { key, code: `Key${key.toUpperCase()}`, windowsVirtualKeyCode: KEYS[key], modifiers: 2 };
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base, commands: [key === 'c' ? 'copy' : 'paste'] });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	await sleep(400);
}
const blendOf = (i) => js(`(() => { const p = document.querySelectorAll(${JSON.stringify(PIECES)})[${i}]; return p ? getComputedStyle(p).mixBlendMode : null; })()`);
/** the collage picker sheet is open (the Sheet's <dialog> stays in the page) */
const pickerOpen = `!!document.querySelector('[data-testid="collage-picker-sheet"]')?.closest('dialog')?.open`;
const layerTestids = () => js(`[...document.querySelectorAll('[data-testid="layers"] > li')].map((li) => li.dataset.testid)`);

/**
 * Art over the slice (pieces are often large – COLLAGE.md §3; the reviewer's case): the selected piece `i` (the text
 * tool on) scaled past the slice's width by its handle; picking the text tool lets it go, a text-tool drag over it draws
 * a new text box and leaves the piece where it is (undone); the piece tool still selects it there. Leaves `i` selected.
 */
async function artCase(i) {
	const box = q(`box-piece-${i}`);
	const rectOf = () => js(`(() => { const s = ${box}?.style; return s ? [s.left, s.top, s.width, s.height].join(' ') : null; })()`);
	const handle = await js(`(() => { const h = ${box}?.querySelector('[data-testid="handle-scale"]'); if (!h) return null; const r = h.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	if (!check(!!handle, 'art: the selected piece shows its scale handle (the text tool on)')) return;
	// out to the slice's visible bottom-right corner
	const corner = await js(`(() => { const r = ${q('slice-canvas')}.getBoundingClientRect(); return [Math.min(r.right, innerWidth) - 4, Math.min(r.bottom, innerHeight) - 4]; })()`);
	await mouseDrag(handle, corner);
	const width = await js(`parseFloat(${box}.style.width)`);
	if (!check(width > 100, `art: the scale handle makes the piece wider than the slice (${width} %)`)) return;
	await click(tid('tool-text'));
	check(await js(`!document.querySelector('[data-testid="slice-canvas"] .box.selected') && !${q('piece-inspector')}`), 'art: picking the text tool lets the piece go');
	// a spot on the piece with nothing but the canvas under it (other piece boxes don't count), room for the drag after it
	const spot = await js(`(() => {
		const o = document.querySelector('[data-testid="slice-canvas"] .overlay');
		const r = ${box}.getBoundingClientRect();
		const c = o.getBoundingClientRect();
		const [x0, y0] = [Math.max(r.left, c.left), Math.max(r.top, c.top)];
		const [x1, y1] = [Math.min(r.right, c.right, innerWidth) - 70, Math.min(r.bottom, c.bottom, innerHeight) - 50];
		for (const fy of [0.5, 0.4, 0.6, 0.3, 0.7]) for (const fx of [0.3, 0.5, 0.2, 0.7, 0.4]) {
			const [x, y] = [x0 + (x1 - x0) * fx, y0 + (y1 - y0) * fy];
			const below = document.elementsFromPoint(x, y).filter((el) => !el.closest('.box.kind-piece'));
			if (below[0] === o) return [x, y];
		}
		return null;
	})()`);
	if (!check(!!spot, 'art: the piece has a spot without a text box / item on it')) return;
	const n = await dialogCount();
	const r0 = await rectOf();
	await mouseDrag(spot, [spot[0] + 60, spot[1] + 40]);
	const drawn = { boxes: await dialogCount(), rect: await rectOf(), selected: await js(`[...document.querySelectorAll('[data-testid="slice-canvas"] .box.selected')].map((b) => b.dataset.testid)`) };
	check(drawn.boxes === n + 1 && drawn.rect === r0 && drawn.selected.length === 1 && drawn.selected[0].startsWith('box-dialog-'), 'art: a text-tool drag over the piece draws a new text box – the piece stays where it is', JSON.stringify({ n, r0, ...drawn }));
	// the new text box's field holds the focus (Ctrl+Z in a field is the browser's): out of it first
	await js('document.activeElement?.blur()');
	await press('z', 'ctrl');
	check((await dialogCount()) === n && (await rectOf()) === r0, 'art: undo takes the text box away, the piece as it was', JSON.stringify({ boxes: await dialogCount(), rect: await rectOf() }));
	await click(tid('tool-piece'));
	await mouseAt(...spot);
	check((await until(`${box}?.classList.contains('selected') && !!${q('piece-inspector')}`, 2000)) && !(await js(pickerOpen)) && (await rectOf()) === r0, 'art: the piece tool still selects the piece there (no picker, nothing moved)', JSON.stringify({ rect: await rectOf(), picker: await js(pickerOpen) }));
}

/**
 * The piece tool on the open slice: the tool → a click → the picker → a shape (disc-black) → it lies on the picture,
 * under the lettering, centred at the click; the look Negative blends it (difference – the picture's negative there);
 * the look Fade's ramp keeps the disc opaque in its middle; the ramp's dot moves its end; one undo per step back to no
 * piece (and redo); Ctrl+C / Ctrl+V = a second piece with a new id, an offset; Layers: pieces reorder among themselves
 * (the selected piece stays selected – through undo / redo too), never above the stack; Delete removes the selected one;
 * a piece scaled over the slice lets the text tool draw over it (artCase).
 * Leaves the two new pieces on the slice (unsaved).
 */
async function pieceCase() {
	const n0 = await pieceCount();
	await click(tid('tool-piece'));
	check((await js(`${q('tool-piece')}.getAttribute('aria-pressed') === 'true' && ${q('tab-layers')}.classList.contains('active')`)) === true, 'pieces: the piece tool is picked and opens the Layers tab');
	const spot = await emptySpot();
	if (!check(!!spot, 'pieces: the slice has a spot without a box')) return;
	await mouseAt(...spot);
	check(await until(`${pickerOpen} && !!${q('collage-shape-search')}`, 5000), 'pieces: a click on the slice opens the collage picker (Shapes)');
	await until(`!!document.querySelector('[data-testid^="collage-shape-"]')`, 15000);
	await js(`${q('collage-shape-search')}.select()`);
	await send('Input.insertText', { text: 'disc black' });
	check(await until(`!!${q('collage-shape-disc-black')}`, 5000), 'pieces: the search finds the shape "disc black"');
	await click(tid('collage-shape-disc-black'));
	const added = await until(`document.querySelectorAll(${JSON.stringify(PIECES)}).length === ${n0 + 1} && !!${q('piece-inspector')} && !${pickerOpen} && ${q(`box-piece-${n0}`)}?.classList.contains('selected')`, 5000);
	if (!check(added, 'pieces: a shape pick adds the piece – drawn on the picture, its box selected, the inspector open, the sheet closed', JSON.stringify(await js(`({ pieces: document.querySelectorAll(${JSON.stringify(PIECES)}).length, inspector: !!${q('piece-inspector')}, sheet: ${pickerOpen}, box: ${q(`box-piece-${n0}`)}?.className ?? null, tab: document.querySelector('[data-testid="tab-layers"]')?.className, sel: [...document.querySelectorAll('.box.selected')].map((b) => b.dataset.testid) })`)))) return;
	const box = await js(`(() => { const p = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}]; const o = ${q('slice-canvas')}.getBoundingClientRect(); const r = p.getBoundingClientRect(); return { width: p.style.width, translate: p.style.translate, cx: (r.x + r.width / 2 - o.x) / o.width * 100, sx: ${spot[0]}, ox: o.x, ow: o.width, cy: (r.y + r.height / 2 - o.y) / o.height * 100, sy: ${spot[1]}, oy: o.y, oh: o.height, under: (() => { const media = p.closest('.slice-media'); const layers = document.querySelector('[data-testid="slice-canvas"] .slice-layers'); return !!media && (!layers || (getComputedStyle(layers).zIndex === '1' && !!(media.compareDocumentPosition(layers) & Node.DOCUMENT_POSITION_FOLLOWING))); })() }; })()`);
	// the click snaps to the grid (30 px): within a grid step of the spot
	const step = (30 / sliceWidth) * 100;
	check(
		box.width === '30%' && /-50%/.test(box.translate) && Math.abs(box.cx - ((box.sx - box.ox) / box.ow) * 100) <= step && Math.abs(box.cy - ((box.sy - box.oy) / box.oh) * 100) <= step && box.under,
		'pieces: centred at the click (anchor c, 30 % wide), in the picture group under the lettering',
		JSON.stringify(box)
	);
	const rows = await layerTestids();
	const firstPiece = rows.findIndex((r) => r.startsWith('layer-piece-'));
	check(firstPiece >= 0 && rows[firstPiece] === `layer-piece-${n0}` && rows.slice(firstPiece).every((r) => r.startsWith('layer-piece-')), 'pieces: the Layers list shows it under the stack (the front piece first)', JSON.stringify(rows));
	// the anchor picker keeps the piece where it is (its drawn height comes from the canvas – the ratio), undo takes it back
	const rectOf = () => js(`(() => { const r = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}].getBoundingClientRect(); return [r.x, r.y, r.width, r.height].map((v) => Math.round(v)); })()`);
	const r0 = await rectOf();
	await js(`${q('piece-advanced')}.open = true`);
	await click(tid('anchor-tl'));
	const r1 = await rectOf();
	const moved = await js(`document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}].style.translate`);
	check(r1.every((v, i) => Math.abs(v - r0[i]) <= 1) && !moved, 'pieces: another anchor (top-left) keeps the piece in place', JSON.stringify({ r0, r1, moved }));
	// the radio holds the focus (Ctrl+Z in a field is the browser's): out of it first
	await js('document.activeElement?.blur()');
	await press('z', 'ctrl');
	check(/-50%/.test(await js(`document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}].style.translate`)), 'pieces: undo puts the anchor back (centre)');
	await js(`${q('piece-advanced')}.open = false`);

	// pixels at the piece's centre: the picture alone (P), the disc as drawn (A)
	const [cx, cy] = await js(`(() => { const r = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}].getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	await until(`(() => { const img = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}]?.querySelector('img'); return !!img && img.complete && img.naturalWidth > 0; })()`, 5000);
	const P = await pixelAt(cx, cy, false);
	const A = await pixelAt(cx, cy);
	await sleep(1200); // separate undo steps (the looks share one key)
	await click(tid('piece-look-negative'));
	check((await blendOf(n0)) === 'difference', 'pieces: the look Negative blends the piece (computed mix-blend-mode difference)', String(await blendOf(n0)));
	const N = await pixelUntil(cx, cy, negative(P), 6);
	check(off(N, negative(P)) <= 6, 'pieces: under the negative disc the picture is its negative', JSON.stringify({ P, N }));
	await sleep(1200);
	await click(tid('piece-look-fade'));
	const F = await pixelUntil(cx, cy, A, 8);
	if (off(A, P) < 40) console.log(`note: the picture is about as dark as the disc there (${JSON.stringify({ A, P })}) – the ramp check is weak`);
	check(off(F, A) <= 8, 'pieces: the look Fade keeps the disc opaque in the middle of its ramp (the bands add up to 1)', JSON.stringify({ A, F, P }));
	// the ramp's dots: the end dot dragged up
	const dots = await until(`!!${q('piece-blur-dot-from')} && !!${q('piece-blur-dot-to')}`, 3000);
	check(dots, 'pieces: a ramp shows its two dots on the selected piece');
	if (dots) {
		const [dx, dy] = await js(`(() => { const r = ${q('piece-blur-dot-to')}.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
		await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: dx, y: dy, button: 'left', clickCount: 1 });
		for (let i = 1; i <= 5; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: dx, y: dy - (30 * i) / 5, button: 'left', buttons: 1 });
		await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: dx, y: dy - 30, button: 'left', clickCount: 1 });
		await sleep(250);
		const top = await js(`parseFloat(${q('piece-blur-dot-to')}.style.top)`);
		check(top < 95 && top > 40 && (await pieceCount()) === n0 + 1, `pieces: dragging the end dot moves the ramp's end (${top} %), the piece stays`);
	}
	// one undo per step: the dot, Fade → Negative, Negative → as drawn, the piece
	await sleep(300);
	await press('z', 'ctrl');
	const back1 = dots ? await js(`parseFloat(${q('piece-blur-dot-to')}?.style.top)`) : 100;
	await press('z', 'ctrl');
	const back2 = await blendOf(n0);
	await press('z', 'ctrl');
	const back3 = [await blendOf(n0), await pieceCount()];
	await press('z', 'ctrl');
	const back4 = await pieceCount();
	check(back1 === 100 && back2 === 'difference' && back3[0] === 'normal' && back3[1] === n0 + 1 && back4 === n0, 'pieces: Ctrl+Z steps back – the dot, the look, the look, then the piece is gone', JSON.stringify({ back1, back2, back3, back4 }));
	for (let i = 0; i < 4; i++) await press('z', 'ctrl', 'shift');
	check((await pieceCount()) === n0 + 1 && (await blendOf(n0)) === 'normal' && (await js(`!!document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}]?.classList.contains('ramp')`)), 'pieces: redo brings the piece back with its ramp');

	// the preset grid (research/COLLAGE.md §12.4): every tile previews this piece's own file in its look; Save as preset …
	// keeps its pixels (the piece names the new preset, no own values); the toast's Undo takes both back – no presets
	// document is left to save (this run never stores `colorations`; check:creator-collage saves and removes one)
	await click(tid(`slice-piece-${n0}`));
	await until(`!!${q('piece-looks')}`, 3000);
	const fileOf = (css) => String(css ?? '').replace(/^url\("?|"?\)$/g, '');
	const fileJs = (el) => `(${el} ? (${el}.querySelector('img')?.getAttribute('src') ?? ${el}.querySelector('.fill')?.style.getPropertyValue('--src') ?? '') : null)`;
	const file = await js(`(() => { const p = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}]; return ${fileJs('p')}; })()`);
	const tiles = await js(`[...document.querySelectorAll('[data-testid="piece-looks"] button')].map((b) => { const p = b.querySelector('.look-preview .piece'); return { id: b.dataset.testid.replace('piece-look-', ''), file: ${fileJs('p')}, blend: p ? getComputedStyle(p).mixBlendMode : null, ramp: !!p?.classList.contains('ramp'), pressed: b.getAttribute('aria-pressed') === 'true' }; })`);
	const tileOf = (id) => tiles.find((t) => t.id === id);
	check(
		tiles.length >= 11 && tiles.every((t) => fileOf(t.file) === fileOf(file)) && tileOf('negative')?.blend === 'difference' && tileOf('fade')?.ramp && tileOf('fade')?.pressed,
		`presets: the grid previews each of its ${tiles.length} looks on the piece's own file (Negative blends, Fade's ramp), Fade chosen`,
		JSON.stringify({ file, tiles })
	);
	const pressedLook = () => js(`document.querySelector('[data-testid="piece-looks"] button[aria-pressed="true"]')?.dataset.testid.replace('piece-look-', '') ?? null`);
	const opacity = () => js(`document.querySelector('[data-testid="piece-opacity"] input')?.value ?? null`);
	const presetLine = () => js(`[...document.querySelectorAll('[data-testid="preset-line"] button')].map((b) => b.dataset.testid)`);
	// typed like a person: the fold opened first, then the value and its blur (it commits; a page without the window's focus
	// sends none by itself)
	await js(`${q('piece-advanced')}.open = true`);
	await sleep(150);
	await js(`(() => { const f = document.querySelector('[data-testid="piece-opacity"] input'); f.focus(); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, '50'); f.dispatchEvent(new Event('input', { bubbles: true })); f.dispatchEvent(new FocusEvent('blur')); f.blur(); return true; })()`);
	await until(`getComputedStyle(document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}]).opacity === '0.5'`, 3000);
	await sleep(300);
	// the piece's centre now (a click's scrollIntoView may have moved the canvas since cx / cy)
	const centre = () => js(`(() => { const r = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0}].getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	const own = await pixelAt(...(await centre()));
	const ownLine = await presetLine();
	await click(tid('preset-save'));
	await until(`!!document.querySelector('dialog.sc-dialog[open] input')`, 3000);
	await js(`(() => { const f = document.querySelector('dialog.sc-dialog[open] input'); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, 'Night'); f.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await answerDialog('button.primary');
	await sleep(300);
	const named = { look: await pressedLook(), line: await presetLine(), opacity: await opacity(), px: await pixelUntil(...(await centre()), own, 4) };
	check(
		JSON.stringify(ownLine) === '["preset-save"]' && !!named.look && !['fade', 'none'].includes(named.look) && JSON.stringify(named.line) === '["preset-more"]' && named.opacity === '' && off(named.px, own) <= 4,
		`presets: Fade + its own opacity → "Save as preset …" "Night": the piece names "${named.look}" without own values, its pixels unchanged`,
		JSON.stringify({ ownLine, named, own })
	);
	// the editor autosaves the presets document while it is dirty (1 s on); the Undo takes that local copy away too – a
	// later load would offer it back as a Recover bar that drops what was made since
	const COPY = `localStorage.getItem('scutoons_creator:colorations')`;
	const copied = await until(`${COPY} !== null`, 3000);
	await js(`(() => { const t = [...document.querySelectorAll('.toast')].find((t) => t.textContent.includes('Night')); t?.querySelector('[data-testid="toast-action"]')?.click(); return true; })()`);
	await sleep(300);
	const undone = { look: await pressedLook(), opacity: await opacity(), night: await js(`!!${q(`piece-look-${named.look}`)}`) };
	check(undone.look === 'fade' && undone.opacity === '50' && !undone.night, "presets: the toast's Undo – the piece Fade + its opacity again, the preset gone", JSON.stringify(undone));
	await sleep(1300); // past the autosave's second
	const copy = await js(COPY);
	check(copied && copy === null, "presets: the toast's Undo leaves no local autosave of the presets document (there while it was dirty)", JSON.stringify({ copied, copy }));

	// copy / paste: a second piece, a new id, one step off
	await click(tid(`slice-piece-${n0}`));
	await clipKey('c');
	await clipKey('v');
	const ids = await pieceIds();
	const pasted = await js(`(() => { const [a, b] = [...document.querySelectorAll(${JSON.stringify(PIECES)})].slice(${n0}); return a && b ? [parseFloat(b.style.left) - parseFloat(a.style.left), parseFloat(b.style.top) - parseFloat(a.style.top)] : null; })()`);
	check(ids.length === n0 + 2 && new Set(ids).size === ids.length && !!pasted && Math.abs(pasted[0] - 2) < 0.01 && Math.abs(pasted[1] - 2) < 0.01 && (await js(`${q(`box-piece-${n0 + 1}`)}?.classList.contains('selected')`)), 'pieces: Ctrl+C / Ctrl+V adds a second piece – a new id, 2 % right / down, selected', JSON.stringify({ ids, pasted }));

	// Layers: the pieces reorder among themselves, never above the stack
	const before = await pieceIds();
	// the selected (pasted, front) piece is the one that moves: its box, its row and the inspector go with it
	const movedId = before[n0 + 1];
	const selectedPiece = () =>
		js(`({ ids: [...document.querySelectorAll(${JSON.stringify(PIECES)})].map((p) => p.dataset.piece), boxes: [...document.querySelectorAll('[data-testid="slice-canvas"] .box.selected')].map((b) => b.dataset.testid), rows: [...document.querySelectorAll('[data-testid="layers"] > li.sc-selected')].map((li) => li.dataset.testid), inspector: ${q('piece-inspector')}?.dataset.piece ?? null })`);
	/** the piece `id` is the selected one: the one selected box and row at its index, the inspector on it */
	const follows = (s, id) => {
		const at = s.ids.indexOf(id);
		return at >= 0 && JSON.stringify(s.boxes) === JSON.stringify([`box-piece-${at}`]) && JSON.stringify(s.rows) === JSON.stringify([`layer-piece-${at}`]) && s.inspector === id;
	};
	await js(`document.querySelector('[data-testid="layer-piece-${n0 + 1}"] [data-sort-grip]').focus()`);
	await press('ArrowDown', 'alt');
	const swapped = await pieceIds();
	check(JSON.stringify(swapped.slice(n0)) === JSON.stringify([...before.slice(n0)].reverse()), 'pieces: Alt+ArrowDown on the front piece row puts it behind the other piece', JSON.stringify({ before, swapped }));
	const s1 = await selectedPiece();
	check(s1.ids[n0] === movedId && follows(s1, movedId), 'pieces: the moved piece stays selected after the reorder – its box, its row, the inspector', JSON.stringify({ movedId, ...s1 }));
	await press('z', 'ctrl');
	const s2 = await selectedPiece();
	check(JSON.stringify(s2.ids) === JSON.stringify(before) && follows(s2, movedId), 'pieces: one undo brings the order back – the piece stays selected', JSON.stringify({ movedId, ...s2 }));
	await press('z', 'ctrl', 'shift');
	const s3 = await selectedPiece();
	check(JSON.stringify(s3.ids) === JSON.stringify(swapped) && follows(s3, movedId), 'pieces: redo moves it behind again – still selected', JSON.stringify({ movedId, ...s3 }));
	await press('z', 'ctrl');
	if (!(await layerTestids()).some((r) => !r.startsWith('layer-piece-'))) {
		// a slice without text boxes / items: one text box to be above
		await click(tid('add-text'));
		await click(tid('text-preset-speech'));
		await until(`[...document.querySelectorAll('[data-testid="layers"] > li')].some((li) => li.dataset.testid.startsWith('layer-dialog-'))`, 3000);
	}
	{
		const rows0 = await layerTestids();
		const top = rows0.find((r) => r.startsWith('layer-piece-'));
		await js(`document.querySelector('[data-testid="${top}"] [data-sort-grip]').focus()`);
		await press('ArrowUp', 'alt');
		check(JSON.stringify(await layerTestids()) === JSON.stringify(rows0) && (await until(`[...document.querySelectorAll('.sc-toast-stack .toast')].some((t) => /Pieces always lie|Elemente liegen immer/.test(t.textContent))`, 2000)), 'pieces: a piece row never moves above the text boxes / items (a hint says so)', JSON.stringify(rows0));
	}

	// Delete removes the selected piece; undo brings it back
	await click(tid(`slice-piece-${n0 + 1}`));
	await press('Delete');
	const afterDelete = await pieceCount();
	await press('z', 'ctrl');
	check(afterDelete === n0 + 1 && (await pieceCount()) === n0 + 2, 'pieces: Delete removes the selected piece, undo brings it back', JSON.stringify({ afterDelete }));
	// "+ Add a piece" (the text tool on): the picker, the piece in the middle of the slice, selected (the slice already has
	// pieces), scaled over the slice (artCase), then removed
	await click(tid('tool-text'));
	await click(tid('add-slice-piece'));
	await until(`${pickerOpen} && !!${q('collage-shape-disc-black')}`, 5000);
	await click(tid('collage-shape-disc-black'));
	const third = await until(`document.querySelectorAll(${JSON.stringify(PIECES)}).length === ${n0 + 3} && !!${q(`box-piece-${n0 + 2}`)}?.classList.contains('selected') && !!${q('piece-inspector')}`, 5000);
	const middle = await js(`(() => { const p = document.querySelectorAll(${JSON.stringify(PIECES)})[${n0 + 2}]; return p ? [p.style.left, p.style.top] : null; })()`);
	check(third && middle?.[0] === '50%' && middle?.[1] === '50%', 'pieces: "+ Add a piece" places the pick in the middle of the slice, selected', JSON.stringify(middle));
	if (third) await artCase(n0 + 2);
	await click(tid('piece-remove'));
	check((await pieceCount()) === n0 + 2, 'pieces: the inspector’s Remove takes it away');
	await click(tid('tool-text'));
}

/**
 * A preset left unsaved by a reload or a crash (research/COLLAGE.md §12.4; the final review's case): the saved piece `i`
 * with its own opacity → "Save as preset …" "Crash test"; the editor keeps both local copies (the chapter's and the
 * presets'); a reload offers both – "Collage presets" next to the chapter, at load; Restore on both: the piece names the
 * preset again (its tile chosen, drawn with it) and Problems has no "unknown preset". Nothing is saved: the run never
 * stores `colorations` (its copies live in the throw-away profile).
 */
async function presetsRecoverCase(i) {
	const PIECE = `document.querySelectorAll(${JSON.stringify(PIECES)})[${i}]`;
	const pressed = () => js(`(() => { const b = document.querySelector('[data-testid="piece-looks"] button[aria-pressed="true"], [data-testid="piece-inspector"] button[data-testid^="piece-look-"][aria-pressed="true"]'); return b ? { id: b.dataset.testid.replace('piece-look-', ''), name: b.querySelector('.name')?.textContent ?? '' } : null; })()`);
	const open = async () => {
		await click(tid('tool-piece'));
		await click(tid(`slice-piece-${i}`));
		return until(`!!${q('piece-looks')}`, 3000);
	};
	if (!check(await open(), 'recover: the saved piece opens in the inspector')) return;
	await js(`${q('piece-advanced')}.open = true`);
	await sleep(150);
	await js(`(() => { const f = document.querySelector('[data-testid="piece-opacity"] input'); f.focus(); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, '40'); f.dispatchEvent(new Event('input', { bubbles: true })); f.dispatchEvent(new FocusEvent('blur')); f.blur(); return true; })()`);
	await until(`getComputedStyle(${PIECE}).opacity === '0.4'`, 3000);
	await sleep(300);
	await click(tid('preset-save'));
	await until(`!!document.querySelector('dialog.sc-dialog[open] input')`, 3000);
	await js(`(() => { const f = document.querySelector('dialog.sc-dialog[open] input'); const set = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(f), 'value').set; set.call(f, 'Crash test'); f.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await answerDialog('button.primary');
	await sleep(300);
	const named = await pressed();
	const copies = [`scutoons_creator:${CHAPTER_KEYS[0]}`, 'scutoons_creator:colorations'];
	const kept = await until(copies.map((k) => `localStorage.getItem(${JSON.stringify(k)}) !== null`).join(' && '), 4000);
	check(named?.name === 'Crash test' && kept, `recover: "Save as preset …" "Crash test" – the piece names "${named?.id}"; both local copies kept (the chapter's, the presets')`, JSON.stringify({ named, kept }));
	// a reload as a crash would leave it (the browser's leave question answered)
	await js('window.__beforeReload = true');
	await send('Page.reload');
	const BARS = `[...document.querySelectorAll('[data-testid="recover"]')]`;
	await until(`!window.__beforeReload && ${BARS}.length >= 2`, 30000);
	const bars = await js(`${BARS}.map((b) => b.textContent.trim())`);
	check(bars?.length === 2 && /^Chapter \d+:/.test(bars[0]) && /^Collage presets:/.test(bars[1]), 'recover: a reload offers both copies at load – "Collage presets" next to the chapter', JSON.stringify(bars));
	await js(`${BARS}.forEach((b) => b.querySelector('button.secondary')?.click()), true`);
	await sleep(500);
	if (!(await open())) return void check(false, 'recover: the piece opens after Restore');
	const back = { look: await pressed(), opacity: await js(`getComputedStyle(${PIECE}).opacity`), bars: await js(`${BARS}.length`) };
	await click(tid('problems-toggle'));
	await until(`!!${q('problems-panel')}`, 5000);
	back.unknown = await js(`/unknown preset/.test(${q('problems-panel')}?.textContent ?? '')`);
	await click(tid('problems-toggle'));
	check(back.look?.id === named?.id && back.look?.name === 'Crash test' && back.opacity === '0.4' && back.bars === 0 && back.unknown === false, 'recover: Restore on both – the piece names "Crash test" again (its tile chosen, drawn with it), no bar left, Problems has no "unknown preset"', JSON.stringify(back));
	const presetsNow = (await index()).colorations;
	check((presetsNow?.latest ?? null) === (before.colorations?.latest ?? null), 'recover: nothing stored – the presets document as before', JSON.stringify(presetsNow ?? null));
}

/** the stored pieces of the slice in the saved draft: the two new ones (the toon's path, a ratio, the look, distinct ids) */
/** a real finger tap (touch emulation on): touchStart + touchEnd at a viewport point */
async function tapAt(x, y) {
	await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
	await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
	await sleep(300);
}
async function tap(selector) {
	const [x, y] = await js(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	await tapAt(x, y);
}
/** a canvas tool by finger: its click comes a moment after the touch; the first touch right after the switch to the
 *  phone layout is sometimes lost (the strip still settling) – one more tap then (the strip is a radio group: a tap
 *  never turns a tool off) */
async function tapTool(tool) {
	const pressed = `${q(tool)}.getAttribute('aria-pressed') === 'true'`;
	// the phone's bottom row (E3): another kind's row is open – ‹ back to the kinds first
	const inRow = `!!document.querySelector('[data-testid="phone-kinds"] [data-testid="${tool}"]')`;
	if ((await js(`!!${q('phone-kinds')}`)) && !(await js(inRow)) && (await js(`!!${q('phone-kind-back')}`))) await tap(tid('phone-kind-back'));
	await tap(tid(tool));
	if (await until(pressed, 1500)) return true;
	await tap(tid(tool));
	return until(pressed, 3000);
}
/** the phone's inspector sheet closed: its content is display:none */
const inspectorHidden = `getComputedStyle(document.querySelector('[data-testid="inspector-region"] .inspector') ?? document.body).display === 'none'`;
/** the picker (its testid) open AND on screen – a modal <dialog> inside a display:none parent opens as an invisible
 *  0 × 0 modal that makes the page inert: the middle of the dialog has to hit the dialog */
const pickerShown = (testid) =>
	`(() => { const d = ${q(testid)}?.closest('dialog'); if (!d?.open) return false; const r = d.getBoundingClientRect(); return r.width > 100 && r.height > 100 && !!document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('dialog'); })()`;
const openDialogs = () => js(`JSON.stringify([...document.querySelectorAll('dialog[open]')].map((d) => [Math.round(d.getBoundingClientRect().width), Math.round(d.getBoundingClientRect().height)]))`);
const itemBoxes = `document.querySelectorAll('[data-testid="slice-canvas"] [data-testid^="box-item-"]').length`;
/**
 * The phone (375 × 812, touch) with the inspector sheet closed (owner 2026-10-18, the live creator on the phone: the
 * Pieces tool, then a tap on the slice did nothing and more taps froze it – "It should work exactly like the items"):
 * the item tool and the piece tool each – a finger tap on the slice shows its picker on screen, a tap on a card places
 * it on the slice; both undone again (the slice as before), back to the desktop.
 */
async function phoneCase() {
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 1, mobile: true });
	await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
	try {
		check(await until(`!!${q('phone-bar-more')} && ${inspectorHidden}`, 5000), 'phone: 375 × 812 – the phone bar, the inspector sheet closed (its content hidden)');
		await frames();
		const i0 = await js(itemBoxes);
		const p0 = await pieceCount();
		check(await tapTool('tool-item'), 'phone: a tap picks the item tool');
		let spot = await emptySpot();
		if (!check(!!spot, 'phone: the slice has a spot without a box')) return;
		await tapAt(...spot);
		// the tap's own click lands where the picker opens: it must not pick the card under the finger
		if (check(await until(`${pickerShown('item-picker-sheet')} && ${itemBoxes} === ${i0}`, 5000), 'phone: the item tool + a tap on the slice shows the item picker on screen, nothing placed yet (its own click picks no card)', `${await js(itemBoxes)} item boxes (${i0} before), dialogs ${await openDialogs()}`)) {
			await until(`!!document.querySelector('[data-testid^="item-card-"]:not([disabled])')`, 15000);
			await tap('[data-testid^="item-card-"]:not([disabled])');
			check(await until(`${itemBoxes} === ${i0 + 1} && !${q('item-picker-sheet')}?.closest('dialog')?.open`, 5000), 'phone: a tap on an item card places it on the slice, the picker closes', `${await js(itemBoxes)} item boxes, dialogs ${await openDialogs()}`);
		} else await press('Escape');
		check(await tapTool('tool-piece'), 'phone: a tap picks the piece tool');
		spot = await emptySpot();
		if (!check(!!spot, 'phone: the slice still has a spot without a box')) return;
		await tapAt(...spot);
		if (check(await until(`${pickerShown('collage-picker-sheet')} && document.querySelectorAll(${JSON.stringify(PIECES)}).length === ${p0}`, 5000), 'phone: the piece tool + a tap on the slice shows the collage picker on screen (like the items), nothing placed yet', `${await pieceCount()} pieces (${p0} before), dialogs ${await openDialogs()}`)) {
			await until(`!!document.querySelector('[data-testid^="collage-shape-"]')`, 15000);
			// the picker keeps its last search: typed over
			await js(`${q('collage-shape-search')}.select()`);
			await send('Input.insertText', { text: 'disc black' });
			await until(`!!${q('collage-shape-disc-black')}`, 5000);
			await tap(tid('collage-shape-disc-black'));
			check(await until(`document.querySelectorAll(${JSON.stringify(PIECES)}).length === ${p0 + 1} && !${pickerOpen}`, 5000), 'phone: a tap on a shape places the piece on the slice, the picker closes', `${await pieceCount()} pieces, dialogs ${await openDialogs()}`);
		} else await press('Escape');
		for (let i = 0; i < 2 && ((await js(itemBoxes)) > i0 || (await pieceCount()) > p0); i++) await press('z', 'ctrl');
		check((await js(itemBoxes)) === i0 && (await pieceCount()) === p0, 'phone: two undos take the item and the piece off again');
	} finally {
		await send('Emulation.setTouchEmulationEnabled', { enabled: false });
		await send('Emulation.clearDeviceMetricsOverride');
		await until(`!(${inspectorHidden})`, 5000);
	}
}
function savedPieces(draft, before) {
	const pieces = draft.content.groups[GROUP].slices[SLICE].pieces ?? [];
	const fresh = pieces.slice(before);
	return {
		ok: fresh.length === 2 && fresh.every((p) => p.src === DISC && p.ratio > 0 && p.preset === 'fade' && p.anchor === 'c' && p.rect?.width === 30 && p.rect.height === undefined) && new Set(pieces.map((p) => p.id)).size === pieces.length,
		detail: JSON.stringify(fresh)
	};
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
	if (ONLY_PIECES) {
		const piecesBefore = (slice.pieces ?? []).length;
		await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/${CHAPTER}?group=${encodeURIComponent(baselineGroupKey)}&slice=${SLICE + 1}` });
		check(await until(`!!${q('slice-canvas')} && !!${q('tool-piece')} && !!document.querySelector('[data-testid="slice-canvas"] img')`, 30000), `the slice editor opens on group ${GROUP + 1}, slice ${SLICE + 1}`);
		await phoneCase();
		await pieceCase();
		await click(tid('save-draft'));
		await answerDialog('button.primary');
		await until(`!document.querySelector('[data-testid="save-draft"]')?.textContent.includes('•')`);
		const saved = savedPieces(await api('GET', `/entities/${CHAPTER_KEYS[0]}?draft=1&wrap=1`), piecesBefore);
		check(saved.ok, 'pieces: Save draft stores both pieces (the toon\'s path, the ratio, the look, distinct ids, no height)', saved.detail);
		const presetsNow = (await index()).colorations;
		check((presetsNow?.latest ?? null) === (before.colorations?.latest ?? null), 'presets: the Save stored no presets document (the Undo left none to save)', JSON.stringify(presetsNow ?? null));
		// the old page still draws them until the reload lands (Page.reload answers at once): a mark on it tells them apart
		await js('window.__beforeReload = true');
		await send('Page.reload');
		check(await until(`!window.__beforeReload && document.querySelectorAll(${JSON.stringify(PIECES)}).length === ${piecesBefore + 2}`, 30000), 'pieces: after a reload the draft draws them again');
		await presetsRecoverCase(piecesBefore);
		throw Object.assign(new Error('done'), { done: true });
	}

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
		(await until(`!!${q('slice-canvas')} && !!${q('tab-layers')}`)) && new URLSearchParams(await search()).get('slice') === String(SLICE + 1),
		`tree: a slice opens the slice editor (canvas + inspector, slice=${SLICE + 1})`,
		await search()
	);
	check((await dialogCount()) === dialogsBefore, `the canvas shows the slice's ${dialogsBefore} text box(es)`);

	// ── tabs ↔ tools ─────────────────────────────────────────────────────────────────────────────────────
	// the Layers tab (BACKLOG 10.43) holds text boxes AND placed items: it picks the text tool, the item tool opens it too
	const pairs = [
		['tab-audio', 'tool-sample'],
		['tab-events', 'tool-event'],
		['tab-layers', 'tool-text']
	];
	for (const [tab, tool] of pairs) {
		await click(tid(tab));
		check(
			(await js(`${q(tab)}.classList.contains('active') && ${q(tool)}.getAttribute('aria-pressed') === 'true'`)) === true,
			`tabs: ${tab} activates and picks ${tool}`
		);
	}
	await click(tid('tab-background'));
	// the Slice tab's map section left with the place unlocks (e0bc49e5, 2026-10-11: places unlock by unlockLocation events)
	check((await js(`${q('tab-background')}.classList.contains('active') && !!${q('slice-media')}`)) === true, 'tabs: the Slice tab shows the slice media');
	for (const [tab, tool] of [...pairs, ['tab-layers', 'tool-item']].reverse()) {
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
	check(await until(`!!document.querySelector('[data-testid="save-draft"]')?.textContent.includes('•')`, 3000), 'add: typing marks the page dirty');
	// leaving with unsaved changes asks in-page (step 14: no browser confirm()); "Cancel" stays
	const toonLink = await js(`!!document.querySelector('a[href="/${TOON}"]')`);
	if (toonLink) {
		await js(`document.querySelector('a[href="/${TOON}"]').click()`);
		const asked = await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000);
		const askedText = asked ? await js(`document.querySelector('dialog.sc-dialog[open]').innerText`) : '';
		await answerDialog('button.ghost');
		await sleep(400);
		check(asked && (await js(`location.pathname`)) === `/${TOON}/chapter/${CHAPTER}`, 'leaving with unsaved changes asks in-page; Cancel stays on the chapter', askedText);
	} else check(false, 'leaving: a link to the toon page', 'no a[href="/catpope"]');
	await js(`${q('text-de')}.focus()`);
	// undo inside a field is the browser's (owner 2026-10-03, COMPONENTS.md 4.4.1): it undoes the typing, never the file
	await sleep(1200);
	await send('Input.insertText', { text: ' x' });
	await sleep(300);
	await press('z', 'ctrl');
	await sleep(300);
	const inField = await js(`({ value: ${q('text-de')}?.value ?? null, focus: document.activeElement?.dataset?.testid, boxes: document.querySelectorAll('[data-testid^="box-dialog-"]').length })`);
	check(inField.value === TEXT && inField.focus === 'text-de' && inField.boxes === dialogsBefore + 1, 'undo in a field: Ctrl+Z undoes the typing there, the text box stays (the file is not undone)', JSON.stringify(inField));

	// ── layers: one stack of text boxes + placed items (BACKLOG 10.43) + click / hover areas (10.54, rows without a drawn
	// layer on the canvas – the canvas's boxes show them) – the new box is the top text box / item row (in front) ──
	const layerRows = () => js(`[...document.querySelectorAll('[data-testid="layers"] > li')].map((li) => li.dataset.testid)`);
	/** the rows the canvas draws in its `.slice-layers` (text boxes, items) */
	const drawnRows = (rows) => rows.filter((r) => /^layer-(dialog|item)-/.test(r));
	/** the canvas's drawn stack, back to front (what the reader draws the same way) */
	const drawn = () => js(`[...document.querySelectorAll('[data-testid="slice-canvas"] .slice-layers > *')].map((el) => (el.classList.contains('dialog') ? 'layer-dialog-' : 'layer-item-') + el.dataset.index)`);
	/** the canvas boxes in pointer order, back to front, as stack rows (an event area's boxes = one row) */
	const boxRows = () =>
		js(`[...document.querySelectorAll('[data-testid="slice-canvas"] .box[data-testid^="box-"]:not(.inactive)')].map((el) => el.dataset.testid.replace('box-', 'layer-'))`);
	const l0 = await layerRows();
	const at = l0.indexOf(`layer-dialog-${added}`);
	if (l0.length >= 2 && at >= 0 && at < l0.length - 1) {
		check(drawnRows(l0)[0] === `layer-dialog-${added}` && JSON.stringify([...(await drawn())].reverse()) === JSON.stringify(drawnRows(l0)), 'layers: the new text box is the top text box / item row and drawn in front; the list = the canvas order', JSON.stringify({ l0, drawn: await drawn() }));
		await js(`document.querySelector('[data-testid="layer-dialog-${added}"] [data-sort-grip]').focus()`);
		await press('ArrowDown', 'alt');
		const l1 = await layerRows();
		const d1 = await drawn();
		check(l1[at + 1] === l0[at] && l1[at] === l0[at + 1] && JSON.stringify([...d1].reverse()) === JSON.stringify(drawnRows(l1)), 'layers: Alt+ArrowDown on its row moves the box one layer back (list + canvas)', JSON.stringify({ l1, d1 }));
		// the canvas's pointer order (its boxes on the Layers tab; the selected one is lifted) = the list, areas included
		const boxes = [...new Set(await boxRows())].reverse().filter((r) => r !== `layer-dialog-${added}` && l1.includes(r));
		check(JSON.stringify(boxes) === JSON.stringify(l1.filter((r) => r !== `layer-dialog-${added}` && boxes.includes(r))) && boxes.length === l1.length - 1, 'layers: the canvas boxes take the pointer in the list order (areas too)', JSON.stringify({ boxes, l1 }));
		await barClick('undo');
		check(JSON.stringify(await layerRows()) === JSON.stringify(l0), 'layers: one undo brings the order back');
	} else check(false, 'layers: the slice has a row under the new text box to reorder against', JSON.stringify(l0));

	// ── snap + nudge (outside a field: the list entry has the focus) ──────────────────────────────────────
	await click(tid(`dialog-${added}`));
	check((await js(`document.activeElement?.dataset?.testid`)) === `dialog-${added}` && (await js(`${q(`box-dialog-${added}`)}.classList.contains('selected')`)), 'the list entry selects the box (focus outside a field)');
	check((await js(`${q('snap-toggle')}.getAttribute('aria-pressed') === 'true' && ${q('grid-size')}.textContent.trim() === '30'`)) === true, 'snap: on by default, 30 px grid');
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
	check((await js(`${q('snap-toggle')}.getAttribute('aria-pressed') === 'false'`)) === true, 'snap: the grid button switches it off');
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

	// ── E.4 the tool strip + zoom (UI-CONCEPTS E.4): two square groups left of the canvas, no toolbar row ──────────
	check(
		(await js(`(() => { const v = ${q('tools-view')}.getBoundingClientRect(), c = ${q('tools-content')}.getBoundingClientRect(), w = ${q('canvas-wrap')}.getBoundingClientRect(); return !document.querySelector('.canvas-tools') && v.right <= w.left && c.right <= w.left && c.top >= v.bottom && getComputedStyle(${q('tools-content')}).flexDirection === 'column'; })()`)) === true,
		'E.4: View + Content are vertical groups left of the canvas (the toolbar row is gone)'
	);
	await click(tid('grid-size'));
	check(await until(`!!${q('grid-sizes')} && ${q('grid-size-30')}.getAttribute('aria-checked') === 'true'`, 2000), 'E.4: the corner number opens the grid sizes (30 checked)');
	await click(tid('grid-size-15'));
	check(await until(`!${q('grid-sizes')} && ${q('grid-size')}.textContent.trim() === '15' && ${q('snap-toggle')}.getAttribute('aria-pressed') === 'true'`, 2000), 'E.4: picking 15 closes the sizes, the corner says 15');
	await js(`${q('snap-toggle')}.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }))`);
	check(await until(`!!${q('grid-sizes')}`, 2000), 'E.4: a right-click on the grid opens the sizes');
	await click(tid('grid-size-30'));
	const wrapW = () => js(`${q('canvas-wrap')}.getBoundingClientRect().width`);
	await click(tid('canvas-zoom'));
	check(await until(`!!${q('canvas-zoom-flyout')}`, 2000), 'E.4: the zoom button opens its flyout');
	await click(tid('canvas-zoom-1x'));
	check(await until(`Math.abs(${q('canvas-wrap')}.getBoundingClientRect().width - ${SLICE_W}) < 1 && ${q('canvas-zoom-value')}.textContent.trim() === '100 %'`, 2000), 'E.4: 1:1 shows the slice at its own width (100 %)', String(await wrapW()));
	await click(tid('canvas-zoom-in'));
	check(await until(`Math.abs(${q('canvas-wrap')}.getBoundingClientRect().width - ${SLICE_W * 1.25}) < 1`, 2000), 'E.4: zoom in = the next step (125 %)', String(await wrapW()));
	const z0 = await wrapW();
	const [wx, wy] = await js(`(() => { const r = ${q('canvas-area')}.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 3]; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: wx, y: wy, deltaX: 0, deltaY: -200, modifiers: 2 });
	check(await until(`${q('canvas-wrap')}.getBoundingClientRect().width > ${z0 + 1}`, 2000), 'E.4: Ctrl + wheel over the canvas zooms in', `${z0} → ${await wrapW()}`);
	await click(tid('canvas-zoom-fit'));
	check(await until(`${q('canvas-zoom-fit')}.getAttribute('aria-pressed') === 'true'`, 2000), 'E.4: fit');
	await press('Escape');
	check(await until(`!${q('canvas-zoom-flyout')}`, 2000), 'E.4: Escape closes the flyout');
	await click(tid(`dialog-${added}`));

	// ── drag on the canvas (lib/editor/canvas-geometry.ts): move onto the grid, the rotate handle (15° steps with snap) ──
	const dragFrom = async (selector, ddx, ddy) => {
		const [x, y] = await js(`(() => { const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
		await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
		for (let i = 1; i <= 5; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + (ddx * i) / 5, y: y + (ddy * i) / 5, button: 'left', buttons: 1 });
		await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + ddx, y: y + ddy, button: 'left', clickCount: 1 });
		await sleep(250);
	};
	await click(tid(`dialog-${added}`));
	const m0 = await boxAt(added);
	await dragFrom(`[data-testid="box-dialog-${added}"]`, 60, 0);
	const m1 = await boxAt(added);
	check(m1.x > m0.x && onGrid(m1.x), 'drag: the box moves with the pointer, onto the grid', JSON.stringify({ m0, m1 }));
	await dragFrom('[data-testid="handle-rotate"]', 80, 40);
	const turn = await js(`(() => { const t = ${q(`box-dialog-${added}`)}.style.transform; const m = t.match(/rotate\\((-?[\\d.]+)deg\\)/); return m ? Number(m[1]) : 0; })()`);
	check(turn !== 0 && turn % 15 === 0, `drag: the rotate handle turns the box in 15° steps (${turn}°)`);
	await barClick('undo');
	await barClick('undo');
	const back = await boxAt(added);
	check(near(back.x, m0.x) && !(await js(`${q(`box-dialog-${added}`)}.style.transform.includes('rotate')`)), 'undo: one step per drag (rotate, then move) brings it back');

	// the inspector's edge drags it wider, a double-click resets it (@scutoons/ui resizable, step 13)
	const inspectorWidth = () => js(`Math.round(document.querySelector('.inspector').getBoundingClientRect().width)`);
	const w0 = await inspectorWidth();
	await dragFrom('.inspector-resize', -100, 0);
	const w1 = await inspectorWidth();
	await js(`document.querySelector('.inspector-resize').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))`);
	await sleep(300);
	check(w1 > w0 && (await inspectorWidth()) === w0, `resize: the inspector drags wider (${w0} → ${w1} px), a double-click resets it`);

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

	// ── the selection model (BACKLOG 10.40, DESIGN.md "Selection"): the keys act on the LIVE region only ──────────
	const lives = () => js(`[...document.querySelectorAll('[data-live]')].map((e) => e.dataset.scRegion).join(',')`);
	await click(`[data-testid="box-dialog-${added}"]`);
	check((await lives()) === 'slice' && (await js(`!!${q(`box-dialog-${added}`)}.querySelector('.handle') && ${q(`layer-dialog-${added}`)}.classList.contains('sc-selected')`)), 'selection: the canvas is the one live region – the box with handles, its Layers row sc-selected', await lives());
	await click(`[data-testid="slice-grip-${SLICE}"]`);
	check((await lives()) === 'slices' && !(await js(`!!${q(`box-dialog-${added}`)}.querySelector('.handle')`)), 'selection: a click in the slice list makes it live – the box loses its handles (selected, not live)', await lives());
	const n1 = await dialogCount();
	await press('Delete');
	check((await dialogCount()) === n1, 'selection: Delete with the slice list live leaves the canvas box alone');
	await press('Escape');
	check(await js(`!!document.querySelector('[data-testid="box-dialog-${added}"].selected')`), 'selection: Esc with the slice list live keeps the canvas selection');

	// ── prev / next slice (the desktop's stepper: a cell of the bar's main row since e3c9c63b, 2026-10-12) ──────────
	await barClick('next-slice');
	check(new URLSearchParams(await search()).get('slice') === String(SLICE + 2) && (await js(`document.querySelector('.slice-nav')?.textContent.includes('${SLICE + 2} /')`)), 'next slice: the slice editor and the URL move on', await search());
	check((await dialogCount()) === (baseline[CHAPTER_KEYS[0]].content.groups[GROUP].slices[SLICE + 1].dialog ?? []).length, 'next slice: the canvas shows that slice');
	await barClick('prev-slice');
	check(new URLSearchParams(await search()).get('slice') === String(SLICE + 1) && (await dialogCount()) === n0, 'previous slice: back, with the new text box', await search());

	// ── reorder: the shared Sortable (BACKLOG 10.53) ────────────────────────────────────────────────────
	const sliceNames = () => js(`[...document.querySelectorAll('[data-testid="slice-list"] > li .thumb .name')].map((e) => e.textContent)`);
	const s0 = await sliceNames();
	if (s0.length >= 3) {
		await dragGrip(tid(`slice-grip-${SLICE + 1}`), `[data-testid="slice-list"] > li:nth-child(${SLICE + 1})`);
		const s1 = await sliceNames();
		check(s1[SLICE] === s0[SLICE + 1] && s1[SLICE + 1] === s0[SLICE] && s1.length === s0.length, 'reorder: dragging a slice grip up one place moves the slice', JSON.stringify({ s0, s1 }));
		await barClick('undo');
		check(JSON.stringify(await sliceNames()) === JSON.stringify(s0), 'reorder: one undo brings the order back');
		await js(`${q(`slice-grip-${SLICE}`)}.focus()`);
		await press('ArrowDown', 'alt');
		const s2 = await sliceNames();
		const focused = await js(`document.activeElement?.dataset.testid ?? ''`);
		check(s2[SLICE + 1] === s0[SLICE] && focused === `slice-grip-${SLICE + 1}`, 'reorder: Alt+ArrowDown on a slice grip moves it down, the focus follows', JSON.stringify({ s2, focused }));
		await barClick('undo');
		check(JSON.stringify(await sliceNames()) === JSON.stringify(s0), 'reorder: undo again');
	} else check(false, 'reorder: the group has at least 3 slices', JSON.stringify(s0));
	const groupNames = () => js(`[...document.querySelectorAll('[data-testid="chapter-tree"] .group-row .name')].map((e) => e.textContent)`);
	const g0 = await groupNames();
	if (g0.length >= 2) {
		await js(`${q('group-grip-0')}.focus()`);
		await press('ArrowDown', 'alt');
		const g1 = await groupNames();
		check(g1[0] === g0[1] && g1[1] === g0[0], 'reorder: Alt+ArrowDown on a group grip moves the group down', JSON.stringify({ g0, g1 }));
		await barClick('undo');
		check(JSON.stringify(await groupNames()) === JSON.stringify(g0), 'reorder: one undo brings the groups back');
		// the open group follows a move (chapter-tree moveGroups) but not its undo: the run's group is opened again, or
		// the pieces part below lands on another group's slice (ISSUES 10.109)
		const openGroup = await js(`[...document.querySelectorAll('[data-testid^="group-"][aria-expanded="true"]')].map((e) => e.dataset.testid).join(',')`);
		if (openGroup !== `group-${GROUP}`) {
			console.log(`note: after the undo the editor shows ${openGroup || 'no group'}, not group-${GROUP} – opened again`);
			await click(tid(`group-${GROUP}`));
			await until(`!!${q(`slice-${SLICE}`)}`, 3000);
		}
		await click(tid(`slice-${SLICE}`));
	}

	// ── E.1 notifications (BACKLOG 10.44): no bars in the flow, the state on Save / Publish (their dots – the not-live
	// chip left the ActionBar with the grouped bar, e3c9c63b 2026-10-12) ──
	if (await js(`!!${q('phone-bar-back')}`)) await click(tid('phone-bar-back'));
	check(await js(`!document.querySelector('.sc-bar.is-dirty') && !document.querySelector('.editor > .files')`), 'E.1: no unsaved bar, no file-status row above the editor');
	check(await js(`document.querySelector('[data-testid="save-draft"]').textContent.includes('•')`), 'E.1: the Save button carries the dot');
	check(await js(`!document.querySelector('[data-testid="file-status"]') && !${q('publish')}.textContent.includes('•')`), 'E.1: no not-live chip; Publish without a dot while nothing is saved (no draft)');

	// ── collage pieces (left on the slice for the save below) ──────────────────────────────────────────
	const piecesBefore = (slice.pieces ?? []).length;
	await phoneCase();
	await pieceCase();

	// ── save draft ──────────────────────────────────────────────────────────────────────────────────────
	const n = writes.length;
	await barClick('save-draft');
	// the note dialog suggests what changed (version-note.ts, owner 2026-10-18), selected – its primary saves with it
	const suggested = (await until(`!!document.querySelector('dialog.sc-dialog[open] #sc-dialog-input')`, 3000))
		? await js(`(() => { const f = document.querySelector('dialog.sc-dialog[open] #sc-dialog-input'); return { value: f.value, selected: f.selectionStart === 0 && f.selectionEnd === f.value.length }; })()`)
		: null;
	check(!!suggested?.value && suggested.selected && suggested.value.length <= 90 && /Text ?box/i.test(suggested.value), 'Save draft: the note dialog suggests what changed (the new text box), selected', JSON.stringify(suggested));
	await answerDialog('button.primary');
	await until(`!document.querySelector('[data-testid="save-draft"]')?.textContent.includes('•')`);
	const saved = writesSince(n).map((w) => `${w.method} ${w.url} ${w.status}`);
	check(saved.includes(`PUT /entities/${CHAPTER_KEYS[0]} 200`) && !saved.some((w) => /publish/.test(w)), 'Save draft: PUT the chapter (one neutral document) 200, nothing published', JSON.stringify(saved));
	check(await until(`!!document.querySelector('.sc-toast-stack .toast.success')`, 3000), 'E.1: "Saved" is a toast (bottom-right, not a bar in the flow)');
	check(await until(`${q('publish')}.textContent.includes('•')`, 3000), 'E.1: Publish carries the dot once a draft is not live');
	const draft = await api('GET', `/entities/${CHAPTER_KEYS[0]}?draft=1&wrap=1`);
	const savedDialogs = draft.content.groups[GROUP].slices[SLICE].dialog ?? [];
	check(
		savedDialogs.length === dialogsBefore + 1 && JSON.stringify(savedDialogs[dialogsBefore]).includes(TEXT) && draft.published?.id === baseline[CHAPTER_KEYS[0]].version.id,
		'the draft has the new text box; the published version stays',
		JSON.stringify({ n: savedDialogs.length, published: draft.published })
	);
	const savedNew = savedPieces(draft, piecesBefore);
	check(savedNew.ok, 'pieces: the draft has the two new pieces (the toon\'s path, the ratio, the look, distinct ids, no height)', savedNew.detail);

	// Publish asks first with one checklist for every level (c258c037, 2026-10-11: the files with a draft, ticked; the
	// language-neutral chapter has no languages to name) – cancelled here
	await barClick('publish');
	const dialogText = async () =>
		(await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000))
			? await js(`(() => { const d = document.querySelector('dialog.sc-dialog[open]'); return [d.querySelector('.message')?.textContent ?? '', ...[...d.querySelectorAll('.checklist .line')].map((l) => (l.querySelector('input')?.checked ? '[x] ' : '[ ] ') + (l.querySelector('.text > span')?.textContent ?? ''))].join(' | '); })()`)
			: '';
	let confirmText = await dialogText();
	// errors in the problem list ask once more first (the chapter's content, not this run's): name them, go on to the confirm
	if (/problem list|Problemliste/i.test(confirmText)) {
		await answerDialog('button.ghost');
		await barClick('problems-toggle');
		await until(`!!${q('problems-panel')}`, 5000);
		console.log(`note: the chapter has errors – ${(await js(`${q('problems-panel')}?.innerText ?? ''`)).replace(/\s+/g, ' ').slice(0, 400)}`);
		await barClick('problems-toggle');
		await barClick('publish');
		await dialogText();
		await answerDialog('button.primary');
		await sleep(300);
		confirmText = await dialogText();
	}
	const asked = await js(`!!document.querySelector('dialog.sc-dialog[open] .checklist')`);
	await answerDialog('button.ghost');
	check(asked && new RegExp(`\\[x\\] (Chapter|Kapitel) ${CHAPTER}(?!\\d)`).test(confirmText), `Publish asks first: its checklist names chapter ${CHAPTER}, ticked`, confirmText);
	check((await writesSince(n).filter((w) => /publish/.test(w.url)).length) === 0, 'cancelled: nothing published');

	// ── VN events (step 8: one form per type): a scene with a script (findVnScene) – its group script, then a slice's events ──
	// edits here are never saved (the page is left with them; nothing is written)
	if (VN) {
		await send('Page.navigate', { url: `${CREATOR}/${VN.toon}/chapter/${VN.chapter}?group=${encodeURIComponent(VN.group)}` });
		check(await until(`!!${q('vn-events')} && !!${q('event-line-0')}`, 30000), `VN: the group script of ${VN.key} · ${VN.group} lists its events`);
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
			(await js(`document.activeElement?.closest('[data-testid="event-${lines}"]') !== null && ['INPUT', 'SELECT'].includes(document.activeElement?.tagName)`)),
		'VN: "+ setVariable" adds an event, opened, the cursor in its first field'
	);
	// a slice without events: "+ event" shows the new one (it used to land outside the reactive list)
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/${CHAPTER}?group=${encodeURIComponent(baselineGroupKey)}&slice=${SLICE + 1}` });
	await until(`!!${q('tab-events')}`, 30000);
	await click(tid('tab-events'));
	await until(`!!${q('vn-events')}`, 5000);
	// the chapter's slice may already have events (catpope 1 slice 2 does): the new one is one line more either way
	const sliceLines = await js(`document.querySelectorAll('[data-testid="vn-events"] [data-testid^="event-line-"]').length`);
	await click(`[data-testid="vn-events"] .add button:last-child`);
	check(
		await until(`document.querySelectorAll('[data-testid="vn-events"] [data-testid^="event-line-"]').length === ${sliceLines + 1} && ${q('tab-events')}.querySelector('.count')?.textContent === '${sliceLines + 1}'`, 3000),
		`VN: "+ event" on a slice${sliceLines ? ` with ${sliceLines} event(s)` : ' without events'} shows the new event`,
		JSON.stringify(await js(`({ lines: document.querySelectorAll('[data-testid="vn-events"] [data-testid^="event-line-"]').length, count: ${q('tab-events')}?.querySelector('.count')?.textContent })`))
	);
	await send('Page.navigate', { url: `${CREATOR}/${VN.toon}/chapter/${VN.chapter}?group=${encodeURIComponent(VN.group)}&slice=${VN.slice}` });
	await until(`!!${q('tab-events')}`, 30000);
	await click(tid('tab-events'));
	check(await until(`!!${q('event-line-0')} && !!${q('vn-events')}`, 5000), 'VN: the slice events tab lists the slice\'s events');
	await click(tid('event-line-0'));
	check(await until(`!!${q('event-whole-0')}`, 3000), 'VN: a slice event has its trigger (whole slice / area)');
	}
	check(errors.length === 0, 'no page errors', errors.join(' | '));
} catch (e) {
	if (e?.done) check(errors.length === 0, 'no page errors', errors.join(' | '));
	else {
		failed = true;
		check(false, 'run', e?.stack ?? String(e));
	}
} finally {
	await restore().catch((e) => check(false, 'restore', String(e)));
}
const bad = results.filter((r) => !r).length;
console.log(bad || failed ? `creator-chapter-check: ${bad} failed` : `creator-chapter-check: all ${results.length} passed`);
process.exit(bad || failed ? 1 : 0);
