#!/usr/bin/env node
// Expressions in the creator's text fields (agents/EXPRESSIONS.md §7 step 4, DESIGN.md "Expressions in text fields"),
// in headless Chrome (1280 × 900, its own throw-away profile) on the LOCAL dev stack as the local test user: a test
// group is saved as a DRAFT of sandbox chapter 9 (a slice with one text box + a group script message with an
// expression condition) and edited in the creator; afterwards every version this run added to the toon's entities is
// deleted again (the dev DB is as before; checked).
//   - typing `{coins}` makes a chip; `{` shows "Enter = builder", Enter opens the builder: if … then … else with a
//     sample reader's preview, Apply → a chip with the ink code (Ctrl+Z takes it back, Ctrl+Y again)
//   - `{map.totl}` → a wavy underline (warning colour) + the tooltip with the suggestion `map.total`; `{coins ==}` → red
//   - Backspace after a chip removes it whole; the arrow keys step over a chip; double-click = edit in place (Escape: a
//     chip again); a click opens the builder parsed back
//   - the canvas shows the text box for the toolbar's sample reader (as written / new guest)
//   - the Problems panel: an expression problem jumps to its field and selects the range (the condition's event opens)
//   - Save draft: refused by the server with a syntax error (422), stored with only a warning (200)
//
//   npm run check:creator-expr          (creator :10000, API :9000, MariaDB container)
//   EXPR_SHOTS=<dir>: screenshots builder-*.png
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const SHOTS = process.env.EXPR_SHOTS;
const TOON = 'sandbox';
const KEY = 'chapter:sandbox:9';
const GROUP = 'expr-check';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

for (const url of [API, CREATOR]) {
	const host = new URL(url).hostname;
	if (!['localhost', '127.0.0.1', '[::1]', '::1'].includes(host)) {
		console.error(`creator-expr-check: refusing ${url} – local dev stack only`);
		process.exit(2);
	}
}

execFileSync(process.execPath, [resolve(ROOT, 'scripts/dev/test-user.mjs')], { stdio: ['ignore', 'ignore', 'inherit'] });
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;

// ── the test group (a draft of sandbox chapter 9) ────────────────────────────────────────────────────────────────
const t = (de, en = de) => ({ de, en });
const before = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
if (before.published?.id !== before.version.id) {
	console.error(`creator-expr-check: ${KEY} has an unpublished draft; not touching it.`);
	process.exit(2);
}
/** the newest version id of every entity of the toon before the run: everything newer is this run's */
const maxBefore = Number(sql(`SELECT IFNULL(MAX(id), 0) FROM entity_versions`));
const group = {
	label: GROUP,
	renderer: { type: 'vertical' },
	slices: [{ image: '../catpope/s1/c2/s1c2-slice-9.webp', width: 390, height: 625, dialog: [{ id: 'xc-box', rect: { x: 8, y: 8, width: 84, height: 24 }, variant: 'caption', text: t('Münzen ', 'Coins ') }] }],
	events: [{ id: 'xc-set', type: 'setVariable', params: { variable: 'coins', value: 3 } }, { id: 'xc-m1', type: 'showMessage', if: { expr: 'coins >= 3 and map.seenn > 1' }, params: { text: t('Hallo', 'Hello'), wait: true } }]
};
const draft = await api('PUT', `/entities/${KEY}`, { content: { ...before.content, groups: [...before.content.groups, group] }, base: before.version.id, message: 'creator-expr-check (removed again)' });

async function restore() {
	const added = sql(`SELECT id FROM entity_versions WHERE id > ${maxBefore} AND entity_key REGEXP ${quote(`^[a-z]+:${TOON}(:|$)`)}`)
		.split('\n')
		.filter(Boolean);
	if (added.length) sql(`DELETE FROM entity_versions WHERE id IN (${added.join(',')})`);
	const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	const same = after.version.id === before.version.id && JSON.stringify(after.content) === JSON.stringify(before.content);
	const others = Number(sql(`SELECT COUNT(*) FROM entity_versions WHERE id > ${maxBefore}`));
	console.log(`${same ? 'ok  ' : 'FAIL'} restored: ${added.length} version(s) of this run deleted, ${KEY} at version ${after.version.id} again${others ? ` (${others} newer versions of other toons left alone)` : ''}`);
	if (!same) process.exitCode = 1;
}

// ── headless Chrome ─────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'creator-expr-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=1280,900', 'about:blank'], { stdio: 'ignore' });
const cleanup = () => {
	try {
		chrome.kill();
	} catch {}
	try {
		rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
	} catch {}
};
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try {
		wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl;
	} catch {}
	if (!wsUrl) await sleep(200);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
const errors = [];
const writes = [];
ws.addEventListener('message', (e) => {
	const msg = JSON.parse(e.data);
	if (msg.id && pending.has(msg.id)) return pending.get(msg.id)(msg), pending.delete(msg.id);
	const p = msg.params;
	if (msg.method === 'Runtime.exceptionThrown') errors.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text);
	else if (msg.method === 'Network.requestWillBeSent' && p.request.url.startsWith(API) && !['GET', 'OPTIONS'].includes(p.request.method)) writes.push({ id: p.requestId, method: p.request.method, url: p.request.url.slice(API.length), status: null });
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
async function until(expression, ms = 10000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(120);
	}
	return null;
}
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));
const shot = async (name) => {
	if (SHOTS) writeFileSync(join(SHOTS, `builder-${name}.png`), Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
};
const q = (testid) => `document.querySelector('[data-testid="${testid}"]')`;
async function mouse(x, y, clickCount = 1) {
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount });
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount });
}
async function centre(selector) {
	return js(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const r = el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
}
async function click(selector) {
	const at = await centre(selector);
	if (!at) throw new Error(`no ${selector}`);
	await mouse(...at);
	await sleep(200);
}
const KEYS = { Enter: 13, Backspace: 8, Delete: 46, Escape: 27, ArrowLeft: 37, ArrowRight: 39, End: 35, z: 90, y: 89 };
async function press(key, ...mods) {
	const modifiers = (mods.includes('ctrl') ? 2 : 0) | (mods.includes('shift') ? 8 : 0);
	const code = key.length === 1 ? `Key${key.toUpperCase()}` : key;
	const base = { key, code, windowsVirtualKeyCode: KEYS[key], modifiers };
	await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
	if (key === 'Enter' && !modifiers) await send('Input.dispatchKeyEvent', { type: 'char', ...base, text: '\r' });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	await sleep(150);
}
const type = async (text) => {
	for (const ch of text) await send('Input.insertText', { text: ch });
	await sleep(100);
};

const FIELD = 'text-de';
const value = () => js(`${q(FIELD)}.value`);
/** the chips the field draws: their text and state classes */
const chips = () => js(`[...${q(`${FIELD}-field`)}.querySelectorAll('.backdrop .chip.first')].map((s) => { let t = '', n = s; for (;;) { t += n.textContent; if (n.classList.contains('last')) break; n = n.nextElementSibling; if (!n) break; } return { text: t, error: s.classList.contains('error'), warning: s.classList.contains('warning') }; })`);
/** the caret at the end of the field */
const toEnd = () => js(`(() => { const a = ${q(FIELD)}; a.focus(); a.setSelectionRange(a.value.length, a.value.length); return true; })()`);
/** a chip's hit area (the layer over the textarea) by its text */
const hitOf = (text) => `[...document.querySelectorAll('[data-testid="${FIELD}-field"] .hits .chip-hit')].find((s) => s.textContent === ${JSON.stringify(text)})`;

await send('Runtime.enable');
await send('Network.enable');
await send('Page.enable');
let crashed = null;
try {
	await send('Page.navigate', { url: `${CREATOR}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await send('Page.navigate', { url: `${CREATOR}/${TOON}/chapter/9?group=${GROUP}&slice=1` });
	check(await until(`!!${q('slice-canvas')} && !!${q('box-dialog-0')}`, 30000), 'the creator opens the test group’s slice (canvas with the text box)');
	await click('[data-testid="box-dialog-0"]');
	check(await until(`!!${q(FIELD)} && !!${q(`${FIELD}-field`)}`, 5000), 'the text box’s text field is the expression field');

	// ── typing {coins} → a chip ───────────────────────────────────────────────────────────────────────────────────
	await toEnd();
	await type('{coins}');
	let c = await chips();
	check((await value()) === 'Münzen {coins}' && c.length === 1 && c[0].text === '{coins}', `typing {coins} makes a chip (${JSON.stringify(c)})`);
	const mirror = await js(`${q(`${FIELD}-field`)}.querySelector('.backdrop').textContent === ${q(FIELD)}.value + '\\u200b'`);
	check(mirror, 'the mirror holds exactly the field’s text (layout in step with the textarea)');

	// ── `{` + Enter → the builder: if … then … else with a sample reader's preview ─────────────────────────────────
	await type(' {');
	check(await until(`!!${q('expr-hint')}`, 2000), '`{` shows the hint “Enter = builder”');
	await press('Enter');
	check(await until(`!!${q('expr-builder')} && document.activeElement === ${q('expr-left')}`, 3000), 'Enter opens the builder at the cursor (the value search focused)');
	await click('[data-testid="expr-kind-cond"]');
	await js(`(() => { const i = ${q('expr-left')}; i.focus(); return true; })()`);
	await type('coi');
	await sleep(200);
	const hits = await js(`[...document.querySelectorAll('[data-testid="expr-hit"]')].map((b) => b.textContent)`);
	check(hits.some((h) => /^coins/.test(h)) && hits.length > 0, `the value search offers the story variable coins (${hits.slice(0, 4).join(' / ')})`);
	await js(`[...document.querySelectorAll('[data-testid="expr-hit"]')].find((b) => /^coins/.test(b.textContent))?.click(), true`);
	await js(`(() => { const s = ${q('expr-op')}; s.value = '>='; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	await until(`!!${q('expr-right')}`, 2000);
	for (const [id, text] of [['expr-right', '3'], ['expr-then', 'reich'], ['expr-else', 'arm']]) {
		await js(`(() => { const i = ${q(id)}; i.focus(); i.value = ${JSON.stringify(text)}; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	}
	const sample = async (id) => {
		await js(`(() => { const s = ${q('expr-sample')}; s.value = '${id}'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
		await sleep(150);
		return js(`${q('expr-preview')}.textContent.trim()`);
	};
	const guest = await sample('guest');
	const halfway = await sample('halfway');
	const code = await js(`${q('expr-code')}.textContent`);
	check(guest === 'arm' && halfway === 'reich', `the preview: new guest → "${guest}", halfway → "${halfway}"`);
	check(code === '{coins >= 3: reich|arm}', `the resulting code is shown: ${code}`);
	await shot('builder');
	await sample('guest');
	await js(`${q('expr-apply')}.click(), true`);
	await sleep(200);
	c = await chips();
	check(!(await js(`!!${q('expr-builder')}`)) && (await value()) === 'Münzen {coins} {coins >= 3: reich|arm}' && c.length === 2, `Apply writes the chip with the ink code in place of the \`{\` (${await value()})`);
	await press('z', 'ctrl');
	const undone = await value();
	await press('y', 'ctrl');
	check(undone === 'Münzen {coins} {' && (await value()) === 'Münzen {coins} {coins >= 3: reich|arm}', `Ctrl+Z takes the builder’s edit back (“${undone}”), Ctrl+Y again`);

	// ── problems: a wavy underline + the tooltip with the suggestion ──────────────────────────────────────────────
	await toEnd();
	await type(' {map.totl}');
	await sleep(500);
	c = await chips();
	const totl = c.find((x) => x.text === '{map.totl}');
	check(totl?.warning && !totl.error, `{map.totl}: a wavy underline in the warning colour (a doubt) ${JSON.stringify(totl)}`);
	const style = await js(`(() => { const s = ${q(`${FIELD}-field`)}.querySelector('.backdrop .chip.warning'); const cs = getComputedStyle(s); return cs.textDecorationStyle + ' ' + cs.textDecorationLine; })()`);
	check(/wavy/.test(style) && /underline/.test(style), `… drawn as text-decoration ${style}`);
	const title = await js(`${hitOf('{map.totl}')}?.getAttribute('title') ?? ${hitOf('{map.totl}')}?.dataset.scTitle`);
	check(/map\.totl/.test(title ?? '') && /map\.total/.test(title ?? '') && /\d/.test(title ?? ''), `the tooltip names the problem, its place and the suggestion: “${title}”`);
	const at = await js(`(() => { const r = ${hitOf('{map.totl}')}.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0], y: at[1] });
	const tip = await until(`(() => { const t = document.querySelector('.sc-tooltip'); return t && !t.hidden ? t.textContent : null; })()`, 3000);
	check(/map\.total/.test(tip ?? ''), `hovering the chip shows it in the one tooltip style (“${tip}”)`);
	await shot('underline');
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 5, y: 5 });

	// ── Backspace removes a chip whole; arrows step over it ────────────────────────────────────────────────────
	await toEnd();
	await press('Backspace');
	check((await value()) === 'Münzen {coins} {coins >= 3: reich|arm} ', `Backspace after a chip removes it whole (“${await value()}”)`);
	await press('Backspace');
	await js(`(() => { const a = ${q(FIELD)}; a.setSelectionRange(14, 14); return true; })()`); // after {coins}
	await press('ArrowLeft');
	const left = await js(`${q(FIELD)}.selectionStart`);
	await press('ArrowRight');
	const right = await js(`${q(FIELD)}.selectionStart`);
	check(left === 7 && right === 14, `the arrow keys step over a chip as one (14 → ${left} → ${right})`);

	// ── double-click edits in place, Escape makes it a chip again ────────────────────────────────────────────────
	const pos = await js(`(() => { const r = ${hitOf('{coins}')}.getBoundingClientRect(); return [r.x + r.width * 0.6, r.y + r.height / 2]; })()`);
	await mouse(pos[0], pos[1], 1);
	await mouse(pos[0], pos[1], 2);
	await sleep(400);
	c = await chips();
	const editingCaret = await js(`${q(FIELD)}.selectionStart`);
	check(!c.some((x) => x.text === '{coins}') && editingCaret > 7 && editingCaret < 14 && !(await js(`!!${q('expr-builder')}`)), `double-click: {coins} is plain text with the caret in it (caret ${editingCaret}, no builder)`);
	await shot('edit-in-place');
	await type('X');
	await press('Escape');
	c = await chips();
	check((await value()).startsWith('Münzen {coinsX}') || c.some((x) => /^\{coin/.test(x.text)), `typing edits it; Escape makes it a chip again (${JSON.stringify(c.map((x) => x.text))})`);
	await press('z', 'ctrl');
	check((await value()).startsWith('Münzen {coins} '), `undo restores {coins} (“${await value()}”)`);

	// ── a click opens the builder for the chip, parsed back ─────────────────────────────────────────────────────
	await click(`[data-testid="${FIELD}-field"] .hits .chip-hit[data-start="15"]`);
	check(await until(`!!${q('expr-builder')}`, 2000), 'a click on a chip opens the builder');
	const parsed = await js(`({ left: ${q('expr-left')}?.value, then: ${q('expr-then')}?.value, else: ${q('expr-else')}?.value, kind: document.querySelector('[data-testid="expr-kind-cond"]')?.getAttribute('aria-pressed') })`);
	check(parsed.left === 'coins' && parsed.then === 'reich' && parsed.else === 'arm' && parsed.kind === 'true', `… parsed back into if / then / else (${JSON.stringify(parsed)})`);
	await press('Escape');
	check(!(await js(`!!${q('expr-builder')}`)), 'Escape closes the builder');

	// ── the canvas as a sample reader sees it ────────────────────────────────────────────────────────────────────
	const boxText = () => js(`${q('box-dialog-0')}?.closest('[data-testid="slice-canvas"]')?.querySelector('.dialog, .sc-dialog, [class*="dialog"]')?.innerText ?? ''`);
	const canvasText = () => js(`[...${q('slice-canvas')}.querySelectorAll('*')].filter((e) => !e.children.length && /Münzen/.test(e.textContent)).map((e) => e.textContent).join(' | ')`);
	check(await until(`!!${q('canvas-sample')}`, 2000), 'the canvas toolbar offers the sample readers (the slice has a text with {…})');
	await js(`(() => { const s = ${q('canvas-sample')}; s.value = 'halfway'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	await sleep(300);
	const asHalfway = await canvasText();
	await js(`(() => { const s = ${q('canvas-sample')}; s.value = 'written'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	await sleep(300);
	const asWritten = await canvasText();
	check(/Münzen 3 reich/.test(asHalfway) && /\{coins\}/.test(asWritten), `canvas: halfway “${asHalfway}”, as written “${asWritten}”`);
	await js(`(() => { const s = ${q('canvas-sample')}; s.value = 'guest'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	void boxText;
	await shot('canvas');

	// ── save: a syntax error is refused (422), a warning is stored ──────────────────────────────────────────────
	await toEnd();
	await type(' {coins ==}');
	await sleep(500);
	c = await chips();
	check(c.find((x) => x.text === '{coins ==}')?.error, '{coins ==}: a red wavy underline (a syntax error)');
	let n = writes.length;
	await click('[data-testid="save-draft"]');
	if (await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000)) await click('dialog.sc-dialog[open] button.primary');
	await until(`${JSON.stringify(n)} < 0 || false`, 1500);
	let saved = writes.slice(n).filter((w) => w.url.startsWith(`/entities/${KEY}`));
	check(saved.length && saved.every((w) => w.status === 422), `Save draft with a syntax error: refused (${saved.map((w) => `${w.method} ${w.status}`).join(', ')})`);
	const notice = await until(`[...document.querySelectorAll('.sc-bar')].map((b) => b.textContent).find((t) => /expr:/.test(t)) ?? null`, 3000);
	check(notice && (await js(`!!${q(FIELD)}`)), `… the reason is a bar, the field stays editable (“${(notice ?? '').trim().slice(0, 90)}…”)`);
	await toEnd();
	await press('Backspace'); // the chip, whole
	await press('Backspace'); // the space
	await type(' {map.totl}');
	await sleep(500);

	// ── the Problems panel: an expression problem jumps to its field and selects the range ────────────────────────
	const viaMore = !(await js(`!!${q('problems-toggle')}`));
	if (viaMore) await click('.sc-action-bar .more-toggle');
	await click('[data-testid="problems-toggle"]');
	if (viaMore && (await js(`document.querySelector('.sc-action-bar .more-toggle')?.getAttribute('aria-expanded') === 'true'`))) await click('.sc-action-bar .more-toggle');
	await until(`!!${q('problems-panel')}`, 3000);
	const rows = await js(`[...${q('problems-panel')}.querySelectorAll('li')].map((li) => li.textContent.trim())`);
	const totlRow = rows.findIndex((r) => /map\.totl/.test(r));
	check(totlRow >= 0, `the Problems panel lists {map.totl} in the creator’s words (“${rows[totlRow]}”)`);
	await js(`${q('problems-panel')}.querySelectorAll('li')[${totlRow}].querySelector('.goto').click(), true`);
	const sel = await until(`(() => { const a = ${q(FIELD)}; return document.activeElement === a && a.selectionEnd > a.selectionStart ? a.value.slice(a.selectionStart, a.selectionEnd) : null; })()`, 3000);
	check(sel === 'map.totl', `a click on it focuses the field and selects the range (“${sel}”)`);
	await shot('problems-jump');

	// the condition's expression (the group script's message): its row opens the event's form and selects the range
	const condRow = rows.findIndex((r) => /map\.seenn/.test(r));
	check(condRow >= 0, `the condition’s problem is listed (“${rows[condRow]}”)`);
	await js(`${q('problems-panel')}.querySelectorAll('li')[${condRow}].querySelector('.goto').click(), true`);
	const condSel = await until(`(() => { const a = document.querySelector('textarea[data-testid^="condition-expr-"]'); return a && document.activeElement === a && a.selectionEnd > a.selectionStart ? a.value.slice(a.selectionStart, a.selectionEnd) : null; })()`, 4000);
	check(condSel === 'map.seenn', `… the event opens, its condition field takes the focus with the range selected (“${condSel}”)`);
	const condMark = await js(`!!document.querySelector('[data-testid$="-field"] .backdrop .warning') && document.querySelectorAll('textarea[data-testid^="condition-expr-"]').length === 1`);
	check(condMark, 'the condition field draws the warning as a wavy underline');
	await shot('condition');
	// Enter in the condition opens the builder on the whole expression (code: it has `and`)
	await press('End');
	await press('Enter');
	const condBuilder = await until(`${q('expr-builder')} && ${q('expr-left')}?.value`, 2000);
	check(condBuilder === 'coins >= 3 and map.seenn > 1' || condBuilder === 'coins >= 3 and map.seenn', `Enter in a condition opens the builder on the expression (value “${condBuilder}”)`);
	await press('Escape');

	n = writes.length;
	await click('[data-testid="save-draft"]');
	if (await until(`!!document.querySelector('dialog.sc-dialog[open]')`, 3000)) await click('dialog.sc-dialog[open] button.primary');
	await until(`!document.querySelector('.sc-bar.is-dirty')`, 5000);
	saved = writes.slice(n).filter((w) => w.url.startsWith(`/entities/${KEY}`));
	const stored = (await api('GET', `/entities/${KEY}?draft=1&wrap=1`)).content.groups.find((g) => g.label === GROUP)?.slices[0].dialog[0].text.de;
	check(saved.some((w) => w.status === 200) && stored === 'Münzen {coins} {coins >= 3: reich|arm} {map.totl}', `Save draft with only a warning: stored (“${stored}”)`);

	check(!errors.length, 'no uncaught exceptions on the page', errors.join(' | '));
} catch (e) {
	crashed = e;
	check(false, 'the run finished', e.stack ?? String(e));
} finally {
	await shot('last').catch(() => {});
	cleanup();
	await restore();
}
const failed = results.filter((x) => !x).length;
console.log(`\n${results.length - failed}/${results.length} ok`);
if (failed || crashed) process.exitCode = 1;
void draft;
