#!/usr/bin/env node
// Expressions in the reader (agents/EXPRESSIONS.md §7 step 2), played in headless Chrome (phone size, its own profile) on
// the local dev stack as the local test user: a test group with a VN script and a text box using ink's inline syntax is
// saved as a DRAFT of sandbox chapter 9, read with the draft preview over several reloads, and afterwards the draft rows
// are removed and the test user's sandbox progress row is put back as it was (the dev DB is as before).
//   - VN messages: `{coins}`, `{coins == 1: coin|coins}`, `{clock(t)}`, a sequence `{Hello.|You again?|Go away!}`
//   - a text box: `{map.total - map.unlocked}`, the same sequence, `{clock(t)}` (live: t is set by the script after the
//     box is drawn), a choice label `{coins + 1}`
//   - visits persist across reloads (the game state's `history`): Hello. → You again? → Go away! → Go away!
//
//   node scripts/dev/expr-check.mjs        (viewer :8000, API :9000, MariaDB container)
//   EXPR_SHOTS=<dir>: phone screenshots expr-*.png
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const { token, email } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const KEY = 'chapter:sandbox:9';
const SHOTS = process.env.EXPR_SHOTS;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test group ──────────────────────────────────────────────────────────────────────────────────────────────
const t = (de, en = de) => ({ de, en });
const slice = (n, dialog) => ({ image: `../catpope/s1/c2/s1c2-slice-${n}.webp`, width: 390, height: 625, ...(dialog ? { dialog } : {}) });
const msg = (text, wait = true) => ({ type: 'showMessage', params: { text: t(text), wait } });
const set = (variable, value) => ({ type: 'setVariable', params: { variable, value, op: 'set' } });
const BOX = ['BOX {map.total - map.unlocked} to go', 'SEQ {Hello.|You again?|Go away!}', 'CLK {clock(t)}'];
const doc = (base) => ({
	...base,
	groups: [
		{
			label: 'expr-a',
			renderer: { type: 'vertical' },
			slices: [slice(9, [{ id: 'expr-box', rect: { x: 8, y: 8, width: 70, height: 30 }, text: t(BOX), variant: 'caption', size: 4 }]), slice(10)],
			events: [
				set('coins', 1),
				msg('M1 {coins} {coins == 1: coin|coins}'),
				set('t', 330),
				msg('M2 {Hello.|You again?|Go away!}'),
				{ type: 'addChoice', params: { label: t('take {coins + 1}'), onSelect: { set: { variable: 'expr_check', value: 'took' } } } },
				{ type: 'showChoices' },
				msg('M3 {clock(t)}', false)
			]
		}
	]
});

// ── API + DB ────────────────────────────────────────────────────────────────────────────────────────────────────
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
const userId = sql(`SELECT id FROM users WHERE email = ${quote(email)}`);
const progressBefore = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'sandbox'`);

const before = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
const created = [];
async function restore() {
	if (created.length) {
		sql(`DELETE FROM entity_versions WHERE entity_key = ${quote(KEY)} AND id IN (${created.join(',')})`);
		const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
		const same = after.version.id === before.version.id && JSON.stringify(after.content) === JSON.stringify(before.content);
		console.log(`${same ? 'ok  ' : 'FAIL'} restored ${KEY} (newest version ${after.version.id}, was ${before.version.id})`);
		if (!same) process.exitCode = 1;
		created.length = 0;
	}
	// the test user's sandbox progress (the synced game state with the visits) as it was
	if (progressBefore) {
		const tab = progressBefore.indexOf('\t');
		sql(`UPDATE progress SET state = ${quote(progressBefore.slice(tab + 1))}, updated_at = ${Number(progressBefore.slice(0, tab))} WHERE user_id = ${quote(userId)} AND toon_id = 'sandbox'`);
	} else sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'sandbox'`);
	const now = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'sandbox'`);
	console.log(`${now === progressBefore ? 'ok  ' : 'FAIL'} restored the test user's sandbox progress`);
	if (now !== progressBefore) process.exitCode = 1;
}
const saved = await api('PUT', `/entities/${KEY}`, { content: doc(before.content), base: before.version.id, message: 'expr-check (removed again)' });
created.push(saved.version.id);

// ── headless Chrome ─────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'expr-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=375,812', 'about:blank'], { stdio: 'ignore' });
const cleanup = () => {
	try { chrome.kill(); } catch {}
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
};
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
async function until(expression, ms = 10000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(150);
	}
	return null;
}
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));
const shot = async (name) => {
	if (SHOTS) writeFileSync(join(SHOTS, `expr-${name}.png`), Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
};

const STATE = `(() => ({
	text: document.querySelector('.vn-message.visible .text')?.innerText.trim() ?? null,
	choices: [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()),
	box: [...document.querySelectorAll('.slice .dialog .line')].map((l) => l.innerText.trim()),
	history: Object.keys(JSON.parse(localStorage.getItem('scutoons_game_sandbox') ?? '{}').history ?? {})
}))()`;
const state = () => js(STATE);
const waitText = (re, ms = 10000) => until(`(() => { const s = ${STATE}; return ${re}.test(s.text ?? '') ? s : null; })()`, ms);
const click = () => js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
const GREETINGS = ['Hello.', 'You again?', 'Go away!', 'Go away!'];

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 2, mobile: true });
	await send('Page.navigate', { url: `${VIEWER}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await js(`localStorage.setItem('scutoons.draft-preview', '1')`);
	// a fresh reader for this toon (the account's sandbox state would come down with the sync): no history yet
	await api('PUT', '/progress/sandbox/', { state: { toonId: 'sandbox', variables: {}, progress: [], created_at: new Date().toISOString() }, updated_at: Date.now() }).catch(() => null);

	for (let visit = 0; visit < GREETINGS.length; visit++) {
		await send('Page.navigate', { url: `${VIEWER}/sandbox/chapter/9` });
		let s = await waitText(/^M1 /, 15000);
		check(s?.text === 'M1 1 coin', `visit ${visit + 1}: {coins} {coins == 1: coin|coins} → "${s?.text}"`, JSON.stringify(await state()));
		if (visit === 0) {
			// the box before the script set t: the clock line is empty; map.total - map.unlocked once the map is loaded
			const box = await until(`(() => { const s = ${STATE}; return /^BOX -?\\d+ to go$/.test(s.box[0] ?? '') ? s : null; })()`, 15000);
			check(box, `text box: {map.total - map.unlocked} → "${box?.box?.[0] ?? (await state()).box[0]}"`, JSON.stringify(await state()));
			await shot('vn-message');
		}
		s = await state();
		check(s.box[1] === `SEQ ${GREETINGS[visit]}`, `visit ${visit + 1}: text box sequence → "${s.box[1]}"`, JSON.stringify(s));
		await click();
		s = await waitText(/^M2 /, 5000);
		check(s?.text === `M2 ${GREETINGS[visit]}`, `visit ${visit + 1}: VN message sequence → "${s?.text}"`, JSON.stringify(await state()));
		s = await until(`(() => { const s = ${STATE}; return s.box[2] === 'CLK 5:30' ? s : null; })()`, 5000);
		check(s, 'the box re-renders when t is set: {clock(t)} → "CLK 5:30"', JSON.stringify(await state()));
		await click();
		s = await until(`(${STATE}).choices.length ? (${STATE}) : null`, 5000);
		check(s?.choices?.[0] === 'take 2', `choice label {coins + 1} → "${s?.choices?.[0]}"`, JSON.stringify(await state()));
		if (visit === 0) await shot('text-box');
		await js(`document.querySelector('.vn-choice')?.click(), true`);
		s = await waitText(/^M3 /, 5000);
		check(s?.text === 'M3 5:30', `{clock(t)} in a message → "${s?.text}"`, JSON.stringify(await state()));
		const box = (await state()).box[1];
		check(box === `SEQ ${GREETINGS[visit]}`, `the box stays on its visit's element while re-rendering ("${box}")`);
		if (visit === GREETINGS.length - 1) await shot('go-away');
		await sleep(300);
	}
	const s = await state();
	check(s.history.some((k) => k.startsWith('text:sandbox/9/box/expr-box/1#')) && s.history.some((k) => k.startsWith('text:sandbox/9/vn/expr-a/script/3#')), 'the visits are in the game state (scutoons_game_sandbox.history)', JSON.stringify(s.history));
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	check(false, 'the run', String(error?.stack ?? error));
} finally {
	await sleep(2500); // the debounced progress push of the last page, before the row is put back
	await restore().catch((error) => check(false, 'restore', String(error)));
	cleanup();
}
const bad = results.filter((r) => !r).length;
console.log(bad ? `expr-check: ${bad} failed` : `expr-check: all ${results.length} passed`);
process.exit(bad || process.exitCode ? 1 : 0);
