#!/usr/bin/env node
// Regression check for the creator's Cast page bottom bar (owner 2026-09-30: "Save draft / Publish don't work"):
// headless Chrome (own throw-away profile, CDP, no dependencies) signed in as the local test user
// (agent@scutoons.test, `npm run dev:test-user`) on the running dev stack (creator :10000, API :9000).
//
//   npm run check:creator-cast
//   (another stack, e.g. a worktree's side ports: SCUTOONS_API=http://localhost:19300/api SCUTOONS_CREATOR=http://localhost:20300,
//    like dev:test-user's SCUTOONS_API)
//
// Checks: with nothing changed both buttons answer with a notice (never a silent no-op); the last persona of an
// entry can't be unticked (no tick that changes nothing); a role text, a persona tick, the order and an inline new
// character mark the page dirty; Save draft saves `cast:catpope` (PUT 200, chip "Draft"); Publish publishes both
// `cast:catpope` and `characters` (no chips once published); a reload shows the saved state.
// Afterwards the published versions are restored (the original content saved + published again), also on failure.
// Refuses to run while either entity has an unpublished draft (someone's work in progress is never published here).
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
// the owner's dev stack (ports from shared/dev-ports.json); SCUTOONS_API / SCUTOONS_CREATOR point it at other local servers
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const TOON = 'catpope';
const KEYS = [`cast:${TOON}`, 'characters'];
const NEW_NAME = 'Regression Figur';
const ROLE = 'Regressionstest';

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

// ── baseline: the published versions, no pending drafts ─────────────────────────────────────────────────────
const baseline = {};
for (const key of KEYS) {
	const latest = await api('GET', `/entities/${key}?draft=1&wrap=1`);
	if (!latest.published || latest.published.id !== latest.version.id) {
		console.error(`creator-cast-check: ${key} has an unpublished draft (v${latest.version.id}); not touching it. Publish or restore it first.`);
		process.exit(2);
	}
	baseline[key] = { id: latest.version.id, content: latest.content };
}

async function restore() {
	for (const key of KEYS) {
		const latest = await api('GET', `/entities/${key}?draft=1&wrap=1`);
		const same = JSON.stringify(latest.content) === JSON.stringify(baseline[key].content);
		if (!same) {
			const put = await api('PUT', `/entities/${key}`, { content: baseline[key].content, base: latest.version.id, message: 'creator-cast-check: restore' });
			await api('POST', `/entities/${key}/publish`, { version: put.version.id });
		} else if (latest.published?.id !== latest.version.id) {
			await api('POST', `/entities/${key}/publish`, { version: latest.version.id });
		}
		const after = await api('GET', `/entities/${key}?draft=1&wrap=1`);
		check(
			after.published?.id === after.version.id && JSON.stringify(after.content) === JSON.stringify(baseline[key].content),
			`restored ${key} (published content as before, no pending draft)`
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
	console.error('creator-cast-check: no Chrome/Edge found (set CHROME)');
	process.exit(2);
}
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-cast-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=1400,900', 'about:blank'], { stdio: 'ignore' });
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
async function type(selector, text) {
	await click(selector);
	await send('Input.insertText', { text });
	await sleep(250);
}
/** an in-page dialog (confirmDialog / askDialog), when one opens: click its button */
async function answerDialog(button) {
	if (await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000)) await click(`dialog.sc-dialog[open] ${button}`);
}
const state = () =>
	js(`({
		dirty: !!document.querySelector('.dirty-bar'),
		chips: ${CHIPS},
		notice: document.querySelector('.note-bar')?.innerText.replace('✕', '').trim() ?? '',
		saveDisabled: ${q('save-draft')}?.disabled,
		publishDisabled: ${q('publish')}?.disabled,
		members: [...document.querySelectorAll('[data-testid^="cast-member-"]')].map((e) => e.dataset.testid.slice(12))
	})`);
const writesSince = (n) => writes.slice(n);
/** the status chips' words: the colour chips carry them as their accessible name / tooltip (owner 2026-10-02) */
const CHIPS = `[...(${q('file-status')}?.querySelectorAll('.file-status, .badge[aria-label]') ?? [])].map((e) => e.getAttribute('aria-label') ?? e.getAttribute('title') ?? e.dataset.scTitle ?? e.innerText).join(' ').replace(/\\s+/g, ' ').trim()`;
// (the file status only shows what is not published – a clean page has none)
const loaded = () => until(`!!${q('cast-editor')} && document.querySelectorAll('[data-testid^="cast-member-"]').length > 0`);

await send('Runtime.enable');
await send('Network.enable');
await send('Page.enable');

let failed = false;
try {
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/cast` });
	check(await loaded(), 'cast page loads signed in as the test user (creator)');

	// 1. nothing changed: both buttons answer
	let s = await state();
	check(!s.dirty && s.chips === '', 'starts clean, cast + characters published (no chips: only what is not published shows)', JSON.stringify(s));
	check(s.saveDisabled === false && s.publishDisabled === false, 'Save draft and Publish are clickable', JSON.stringify(s));
	let n = writes.length;
	await click('[data-testid="save-draft"]');
	s = await state();
	check(s.notice.length > 0 && writesSince(n).length === 0, 'Save draft without changes says so (no request)', JSON.stringify(s));
	await click('[data-testid="publish"]');
	s = await state();
	check(s.notice.length > 0 && writesSince(n).length === 0, 'Publish with everything published says so (no request)', JSON.stringify(s));

	// 2. the only persona of an entry can't be unticked
	const lone = await js(`[...document.querySelectorAll('[data-testid^="cast-personas-"]')].map((el) => [...el.querySelectorAll('input')]).find((l) => l.filter((i) => i.checked).length === 1)?.find((i) => i.checked)?.disabled`);
	check(lone === true || lone === undefined, 'the last ticked persona is disabled (no tick that changes nothing)');
	// 2b. a character with only one persona shows no persona row (nothing to choose)
	const single = await js(`(() => { const rows = [...document.querySelectorAll('[data-testid^="cast-personas-"]')]; return rows.every((el) => el.querySelectorAll('input').length > 1); })()`);
	check(single === true, 'persona rows only for characters with more than one persona');

	// 3. a role text → Save draft saves the cast
	const [first, second] = s.members;
	const last = s.members[s.members.length - 1];
	await type(`[data-testid="cast-role-${last}-de"]`, ROLE);
	s = await state();
	check(s.dirty && s.saveDisabled === false, 'a role text marks the page dirty', JSON.stringify(s));
	n = writes.length;
	await click('[data-testid="save-draft"]');
	await answerDialog('button.secondary'); // the note dialog: save without a note
	await until(`!document.querySelector('.dirty-bar') && !${q('save-draft')}.disabled`);
	s = await state();
	const saved = writesSince(n);
	check(saved.length === 1 && saved[0].method === 'PUT' && saved[0].url === `/entities/cast:${TOON}` && saved[0].status === 200, 'Save draft: PUT cast 200', JSON.stringify(saved));
	check(!s.dirty && /Draft/i.test(s.chips), 'the cast chip shows the draft', JSON.stringify(s));

	// 4. persona tick + order + inline character → Publish publishes both entities
	const tick = await js(`(() => { const box = [...document.querySelectorAll('[data-testid="cast-personas-${first}"] input')].find((i) => !i.checked); if (!box) return null; box.dataset.check = '1'; return true; })()`);
	if (tick) {
		await click('[data-testid="cast-personas-' + first + '"] input[data-check="1"]');
		check((await state()).dirty, 'a persona tick marks the page dirty');
	}
	// the inline character first, then move it up one place (works for a cast of any size, also a single member)
	await type('[data-testid="cast-new-name"]', NEW_NAME);
	await click('[data-testid="cast-new-create"]');
	s = await state();
	const newId = s.members.find((m) => ![first, second, last].includes(m) && m.startsWith('regression'));
	check(!!newId && /Characters\s+\S.*(Unsaved|Ungespeichert|•)/i.test(s.chips), 'an inline new character joins the cast and marks the registry dirty', JSON.stringify(s));
	await click(`[data-testid="cast-member-${newId}"] .order button`);
	s = await state();
	check(s.members.indexOf(newId) === s.members.length - 2 && s.members.indexOf(last) === s.members.length - 1, 'moving up reorders the cast', JSON.stringify(s.members));
	n = writes.length;
	await click('[data-testid="publish"]');
	await answerDialog('button[type="submit"]'); // Publish confirms in-page
	await until(`${CHIPS} === '' && !document.querySelector('.dirty-bar')`);
	s = await state();
	const published = writesSince(n).map((w) => `${w.method} ${w.url} ${w.status}`);
	check(
		[`PUT /entities/cast:${TOON} 200`, 'PUT /entities/characters 200', `POST /entities/cast:${TOON}/publish 200`, 'POST /entities/characters/publish 200'].every((w) => published.includes(w)),
		'Publish saves and publishes cast + characters',
		JSON.stringify(published)
	);
	check(!s.dirty && s.chips === '', 'both published: no chips left', JSON.stringify(s));

	// 5. a reload shows the saved state
	await send('Page.reload');
	await sleep(500);
	await loaded();
	s = await state();
	const role = await js(`${q(`cast-role-${last}-de`)}?.value`);
	const ticked = tick ? await js(`[...document.querySelectorAll('[data-testid="cast-personas-${first}"] input')].every((i) => i.checked)`) : true;
	check(s.members.includes(newId) && s.members.indexOf(newId) < s.members.indexOf(last) && role === ROLE && ticked && !s.dirty, 'after a reload: role, persona, order and the new character are there', JSON.stringify({ s, role, ticked }));
	check(errors.length === 0, 'no page errors', errors.join(' | '));
} catch (e) {
	failed = true;
	check(false, 'run', e?.stack ?? String(e));
} finally {
	await restore().catch((e) => check(false, 'restore', String(e)));
}
const bad = results.filter((r) => !r).length;
console.log(bad || failed ? `creator-cast-check: ${bad} failed` : `creator-cast-check: all ${results.length} passed`);
process.exit(bad || failed ? 1 : 0);
