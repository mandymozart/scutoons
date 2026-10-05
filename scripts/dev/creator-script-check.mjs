#!/usr/bin/env node
// Check of the creator's script page (the toon screenplay, entity `script:<toon>:<l>`, agents/DATA.md "Script"):
// headless Chrome (own throw-away profile, CDP, no dependencies) signed in as the local test user
// (agent@scutoons.test, `npm run dev:test-user`) on the running dev stack (creator :10000, API :9000 – or
// SCUTOONS_API / SCUTOONS_CREATOR for servers on other ports).
//
//   npm run check:creator-script
//
// Checks on Katzenpapst: the German script loads byte-identical to its published version (and to the screenplay file,
// when SCREENPLAY_DE or the default path exists); the outline lists chapters / scenes / songs and jumps (cursor + scroll);
// read-only until "Edit script" is confirmed; typing stays fast on the ~1800-line text (median keystroke round trip, no long tasks); an edited line → Save draft
// (PUT) → Publish; the history lists both versions; restoring the original gives the identical text again (published
// again); the language switch shows the English script. Afterwards the published versions are as before (restored also
// on failure). Refuses to run while a script has an unpublished draft (someone's work in progress is never published).
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
const LOCALES = ['de', 'en'];
const KEYS = LOCALES.map((l) => `script:${TOON}:${l}`);
const SCREENPLAY_DE = process.env.SCREENPLAY_DE ?? 'C:/Development/cat-pope-agent/external-repos/cat-pope-awakening/de/screenplay.txt';
const EDIT = ' [script-check]';

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

// ── baseline: the published versions, no pending drafts ─────────────────────────────────────────────────────
const baseline = {};
for (const key of KEYS) {
	const latest = await api('GET', `/entities/${key}?draft=1&wrap=1`);
	if (!latest.published || latest.published.id !== latest.version.id) {
		console.error(`creator-script-check: ${key} has an unpublished draft or none published; not touching it. Publish or restore it first.`);
		process.exit(2);
	}
	baseline[key] = { id: latest.version.id, content: latest.content };
}
const source = baseline[KEYS[0]].content.text;
const english = baseline[KEYS[1]].content.text;
if (existsSync(SCREENPLAY_DE)) {
	check(Buffer.from(source, 'utf8').equals(readFileSync(SCREENPLAY_DE)), 'and is byte-identical to the screenplay file');
}

async function restore() {
	for (const key of KEYS) {
		const latest = await api('GET', `/entities/${key}?draft=1&wrap=1`);
		const same = JSON.stringify(latest.content) === JSON.stringify(baseline[key].content);
		if (!same) {
			const put = await api('PUT', `/entities/${key}`, { content: baseline[key].content, base: latest.version.id, message: 'creator-script-check: restore' });
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
	console.error('creator-script-check: no Chrome/Edge found (set CHROME)');
	process.exit(2);
}
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-script-'));
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
const area = q('script-text');
/** an in-page dialog (confirmDialog / askDialog), when one opens: click its button */
async function answerDialog(button) {
	if (await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000)) await click(`dialog.sc-dialog[open] ${button}`);
}
/** the header's text-language switch (the only language switch) */
const setLanguage = (l) => js(`(() => { const s = ${q('text-language-select')}; s.value = ${JSON.stringify(l)}; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
const value = () => js(`${area}?.value ?? null`);
const writesSince = (n) => writes.slice(n);
const historyButton = `[...document.querySelectorAll('button')].find((b) => ['History', 'Verlauf'].includes(b.textContent.trim()))`;

await send('Runtime.enable');
await send('Network.enable');
await send('Page.enable');

let failed = false;
try {
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	const t0 = Date.now();
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/script` });
	check(await until(`(${area}?.value.length ?? 0) > 1000 && !!${q('file-status')}`), `script page loads (${Date.now() - t0} ms)`);

	// 1. the whole German script, formatting intact
	let text = await value();
	check(text === source, 'the editor shows the German script byte-identical (every space, blank line, trailing space)', `${text?.length} vs ${source.length}`);
	const stats = await js(`${q('script-stats')}?.innerText ?? ''`);
	check(/1834\b/.test(stats), `line count shown (${stats.trim()})`);
	const font = await js(`getComputedStyle(${area}).fontFamily + ' / ' + getComputedStyle(${area}).whiteSpace + ' / ' + ${area}.wrap`);
	check(/IBM Plex Mono/.test(font) && /pre/.test(font) && /off/.test(font), `monospace, no wrapping (${font})`);

	// 2. the outline: chapters, scenes, songs; a click jumps to the line
	const items = await js(`[...document.querySelectorAll('[data-testid="script-outline-item"]')].map((b) => [b.closest('li').className.split(' ')[0], +b.dataset.line, b.innerText.replace(/\\s+/g, ' ').trim()])`);
	const kinds = (k) => items.filter((i) => i[0] === k).length;
	check(kinds('chapter') === 6 && kinds('song') === 6 && kinds('scene') === 39, `outline: ${kinds('chapter')} chapters, ${kinds('scene')} scenes, ${kinds('song')} songs`, JSON.stringify(items.slice(0, 5)));
	const target = items.find((i) => /KATHEDRALE - TAG/.test(i[2]));
	await click(`[data-testid="script-outline-item"][data-line="${target[1]}"]`);
	const jumped = await js(`(() => { const el = ${area}; const line = el.value.slice(0, el.selectionStart).split('\\n').length; const lh = parseFloat(getComputedStyle(el).lineHeight); const first = Math.floor(el.scrollTop / lh) + 1; const rows = Math.floor(el.clientHeight / lh); return { line, first, rows, focused: document.activeElement === el, at: el.value.split('\\n')[line - 1] }; })()`);
	check(jumped.line === target[1] && jumped.focused && jumped.first <= target[1] && target[1] < jumped.first + jumped.rows && /KATHEDRALE/.test(jumped.at),
		`outline jumps: "${target[2]}" → line ${target[1]}, visible from row ${jumped.first}`, JSON.stringify(jumped));
	const currentItem = await js(`document.querySelector('[data-testid="script-outline-item"].current')?.dataset.line`);
	check(Number(currentItem) === target[1], 'the outline marks the scene the cursor is in');

	// read-only until confirmed (owner 2026-10-03): typing asks first, "Edit script" + the dialog's confirm unlocks
	check(await js(`${area}.readOnly`), 'the script is read-only until confirmed');
	await click('[data-testid="script-edit"]');
	await click('dialog.sc-dialog[open] button[type="submit"]');
	check(await until(`!${area}.readOnly`), 'confirming "Edit script" makes it editable');

	// 3. performance on the big text: keystrokes at the end of a line
	await js(`(() => { window.__long = 0; new PerformanceObserver((l) => { window.__long += l.getEntries().filter((e) => e.duration > 50).length; }).observe({ type: 'longtask' }); })()`);
	await js(`(() => { const el = ${area}; const end = el.value.indexOf('\\n', el.selectionStart); el.setSelectionRange(end, end); })()`);
	const times = [];
	for (const ch of EDIT) {
		const start = performance.now();
		await send('Input.insertText', { text: ch });
		await js('new Promise((r) => requestAnimationFrame(() => r(1)))');
		times.push(performance.now() - start);
	}
	await sleep(600); // the debounced write into the doc
	times.sort((a, b) => a - b);
	const median = times[Math.floor(times.length / 2)];
	const long = await js('window.__long');
	check(median < 50 && long <= 1, `typing on ${source.split('\n').length - 1} lines: median keystroke ${median.toFixed(1)} ms, ${long} long tasks`);
	const edited = source.split('\n');
	edited[target[1] - 1] += EDIT;
	text = await value();
	check(text === edited.join('\n'), 'the edit is exactly one changed line');
	check(await until('!!document.querySelector(".sc-bar.is-dirty")'), 'the page is dirty after the debounce');

	// 4. save draft, publish → the history has both versions
	let n = writes.length;
	await click('[data-testid="save-draft"]');
	// Save draft asks for a note (askDialog): save without one
	await answerDialog('button.secondary');
	await until(`!document.querySelector('.sc-bar.is-dirty') && !${q('save-draft')}.disabled`);
	const saved = writesSince(n);
	check(saved.length === 1 && saved[0].method === 'PUT' && saved[0].url === `/entities/${KEYS[0]}` && saved[0].status === 200, 'Save draft: PUT script:catpope:de 200', JSON.stringify(saved));
	const draft = await api('GET', `/entities/${KEYS[0]}?draft=1&wrap=1`);
	check(draft.content.text === edited.join('\n') && draft.published.id === baseline[KEYS[0]].id, 'the draft is stored exactly, the original stays published');
	n = writes.length;
	await click('[data-testid="publish"]');
	await answerDialog('button[type="submit"]'); // Publish confirms in-page
	await until(`!document.querySelector('[data-testid="file-status"] .sc-chip')`);
	const published = writesSince(n).map((w) => `${w.method} ${w.url} ${w.status}`);
	const en = await api('GET', `/entities/${KEYS[1]}?draft=1&wrap=1`);
	check(published.includes(`POST /entities/${KEYS[0]}/publish 200`) && en.version.id === baseline[KEYS[1]].id && en.published.id === baseline[KEYS[1]].id,
		'Publish publishes the edited German script; the English one stays the same version', JSON.stringify(published));
	await js(`${historyButton}?.click()`);
	check(await until(`!!document.querySelector('[data-testid="history-panel"]') && !!document.querySelector('[data-testid="restore-de-${baseline[KEYS[0]].id}"]')`), 'the history lists the edited and the original version');
	const listed = await js(`[...document.querySelectorAll('[data-testid="history-panel"] li')].length`);
	check(listed >= 3, `history entries: ${listed} (de + en)`);

	// 5. restore the original → identical text, published again
	await click(`[data-testid="restore-de-${baseline[KEYS[0]].id}"]`);
	await answerDialog('button[type="submit"]'); // Restore confirms in-page (one question)
	check(await until(`${area}.value === ${JSON.stringify(source)}`), 'restore: the editor shows the original text again');
	const restored = await api('GET', `/entities/${KEYS[0]}?draft=1&wrap=1`);
	check(restored.content.text === source && JSON.stringify(restored.content) === JSON.stringify(baseline[KEYS[0]].content), 'restore: the stored draft is byte-identical to the original');
	await click('[data-testid="publish"]');
	await answerDialog('button[type="submit"]'); // Publish confirms in-page
	await until(`!document.querySelector('[data-testid="file-status"] .sc-chip') && !document.querySelector('.sc-bar.is-dirty')`);
	const live = await api('GET', `/entities/${KEYS[0]}`);
	check(live.text === source, 'the original is published again');

	// 6. the language switch
	await setLanguage('en');
	check(await until(`${area}.value === ${JSON.stringify(english)}`), `en: the English script (${english.split('\n').length - 1} lines)`);
	const enOutline = await js(`document.querySelectorAll('[data-testid="script-outline-item"]').length`);
	check(enOutline > 5, `en outline (${enOutline} items, "Chapter" / "Song" markers)`);
	await setLanguage('de');
	check(await until(`${area}.value === ${JSON.stringify(source)}`), 'back to de');

	// 7. a toon without a script: an empty page, nothing dirty
	await send('Page.navigate', { url: `${CREATOR}/sandbox/script` });
	const empty = await until(`!!${area} && !!${q('file-status')}`);
	check(empty && (await value()) === '' && !(await js('!!document.querySelector(".sc-bar.is-dirty")')), 'a toon without a script: empty editor, not dirty');
	check(errors.length === 0, 'no page errors', errors.join(' | '));
} catch (e) {
	failed = true;
	check(false, 'run', e?.stack ?? String(e));
} finally {
	await restore().catch((e) => check(false, 'restore', String(e)));
}
const bad = results.filter((r) => !r).length;
console.log(bad || failed ? `creator-script-check: ${bad} failed` : `creator-script-check: all ${results.length} passed`);
process.exit(bad || failed ? 1 : 0);
