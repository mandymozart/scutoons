#!/usr/bin/env node
// No dead ends for the reader (owner 2026-10-06: "Events queue takeovers have to be safe for accidental dead ends" – by
// logic and validation, no timers; agents/EXPRESSIONS.md "Dead-end safety") on the local dev stack, as the test user:
//   1. Saves that could get a reader stuck are REFUSED (PUT /api/entities → 422 "can get the reader stuck", the server's
//      FlowCheck) and flagged by the creator's validation (@scutoons/content validateNeutral: errors; the Flow graph's
//      "stuck" mark): a question whose choices can all be hidden, a jump to a missing group label / chapter / another
//      toon's label, a divert to a missing point.
//   2. The runtime rules, in headless Chrome (phone size, its own profile) on a draft of sandbox chapter 9 that bypasses
//      the guard (stored through the database – like content saved before the guard): every choice hidden → the script
//      goes on; a jump to a missing label → stays; a slice trigger whose event throws while the script's message waits →
//      the message comes back and answers; a text box with a long `wait` → its tap layer continues at once. After every
//      step: a frozen page always shows something to answer; at the end the page scrolls and the follow-up bar comes.
// Every row the run adds is deleted again, the test user's save game is put back (checked).
//
//   node --experimental-transform-types --no-warnings scripts/dev/vn-safety-check.mjs   (npm run check:vn-safety)
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { local } from './ports.mjs';
import { saveGameRow } from './save-game-row.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { validateNeutral, chapterFlow, resolveChapter } = await import(pathToFileURL(resolve(ROOT, 'shared/content/src/index.ts')).href);
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const { token, email } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const KEY = 'chapter:sandbox:9';
const MESSAGE = 'vn-safety-check (removed again)';
const SHOTS = process.env.VN_SAFETY_SHOTS;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => (s === null ? 'NULL' : `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`);
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	return { status: res.status, json: await res.json().catch(() => null) };
}
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const before = (await api('GET', `/entities/${KEY}?draft=1&wrap=1`)).json;
const maxBefore = Number(sql('SELECT MAX(id) FROM entity_versions'));
const saveGame = saveGameRow(email);
function restore() {
	sql(`DELETE FROM entity_versions WHERE id > ${maxBefore} AND entity_key = ${quote(KEY)} AND message = ${quote(MESSAGE)}`);
	const now = sql(`SELECT MAX(id) FROM entity_versions WHERE entity_key = ${quote(KEY)}`);
	check(Number(now) === before.version.id, `restored ${KEY} (newest version ${now}, was ${before.version.id})`);
	check(saveGame.restore(), "restored the test user's save game (progress row scutoons)");
}

// ── 1. refused saves + the creator's validation ──────────────────────────────────────────────────────────────────
const t = (de, en = de) => ({ de, en });
const IMG = (n) => `../catpope/s1/c2/s1c2-slice-${n}.webp`;
const slice = (n, extra = {}) => ({ image: IMG(n), width: 390, height: 625, ...extra });
const group = (events, extra = {}) => ({ label: 'safety-a', renderer: { type: 'vertical' }, slices: [slice(9), slice(10), slice(11)], events, ...extra });
const msg = (id, text, wait = true) => ({ id, type: 'showMessage', params: { text: t(text), wait } });
const hidden = [
	{ id: 'evs-c1', type: 'addChoice', params: { label: t('A') }, if: { variable: 'nope', op: 'set' } },
	{ id: 'evs-c2', type: 'addChoice', params: { label: t('B') }, if: { expr: 'y > 5' } },
	{ id: 'evs-q1', type: 'showChoices', params: {} }
];
const jump = (id, target) => ({ id, type: 'jumpTo', params: { target } });
const broken = {
	'a question whose choices can all be hidden': [msg('evs-m1', 'Hi'), ...hidden],
	'a jump to a group label that is not in the chapter': [jump('evs-j1', { type: 'chapter', value: '9', label: 'nowhere' })],
	'a jump to a chapter that does not exist': [jump('evs-j2', { type: 'chapter', value: '999' })],
	"a jump to another toon's missing label": [jump('evs-j3', { type: 'chapter', value: '1', toon: 'toptoy', label: 'nope' })],
	'a divert to a missing point': [{ id: 'evs-d1', type: 'divert', params: { to: 'missing' } }]
};
const languages = { source: before.content.language ?? 'de', languages: ['de', 'en'] };
for (const [what, events] of Object.entries(broken)) {
	const content = { ...before.content, groups: [group(events)] };
	const res = await api('PUT', `/entities/${KEY}`, { content, base: before.version.id, message: MESSAGE });
	check(res.status === 422 && /stuck|no point/.test(JSON.stringify(res.json)), `refused on save (422): ${what}`, `${res.status} ${JSON.stringify(res.json).slice(0, 300)}`);
	const ctx = {
		file: 'chapter9.json',
		targets: { toon: 'sandbox', toons: ['sandbox', 'catpope', 'toptoy', 'yellowmouse'], chapters: (toon) => (toon === 'sandbox' ? [8, 9] : toon === 'toptoy' ? [1] : undefined), labels: (toon, n) => (toon === 'sandbox' && n === 9 ? ['safety-a'] : toon === 'toptoy' && n === 1 ? ['mama-hypercity'] : undefined) }
	};
	const errors = validateNeutral('chapter', content, languages, ctx).filter((p) => p.level === 'error' && p.path.includes('events'));
	check(errors.length > 0, `flagged by the creator's validation (an error): ${what}`, JSON.stringify(validateNeutral('chapter', content, languages, ctx).slice(0, 4)));
}
{
	const resolved = resolveChapter({ ...before.content, groups: [group([msg('evs-m1', 'Hi'), ...hidden])] }, languages.source, languages);
	check(chapterFlow(resolved.groups).nodes.some((n) => n.kind === 'choices' && n.status === 'stuck'), 'the Flow graph marks the question "may get stuck"');
}
// one unconditional choice: saved
{
	const content = { ...before.content, groups: [group([msg('evs-m1', 'Hi'), ...hidden.slice(0, 2), { id: 'evs-c3', type: 'addChoice', params: { label: t('Weiter') } }, hidden[2]])] };
	const res = await api('PUT', `/entities/${KEY}`, { content, base: before.version.id, message: MESSAGE });
	check(res.status === 200, 'one unconditional choice: the save goes through', `${res.status} ${JSON.stringify(res.json).slice(0, 200)}`);
}

// ── 2. the runtime rules on content from before the guard (stored through the database) ───────────────────────────
const legacy = {
	...before.content,
	groups: [
		group(
			[
				msg('evs-r1', 'S1 Hallo'),
				...hidden,
				msg('evs-r2', 'S2 after the hidden question'),
				jump('evs-r3', { type: 'chapter', value: '9', label: 'nowhere' }),
				msg('evs-r4', 'S3 stayed'),
				{ id: 'evs-r5', type: 'showTextBox', params: { dialog: 'evs-box', wait: true, duration: 9000 } },
				msg('evs-r6', 'S4 after the box'),
				msg('evs-r7', 'END', false)
			],
			{
				slices: [
					slice(9, {
						dialog: [{ id: 'evs-box', rect: { x: 10, y: 10, width: 60 }, text: t('BOX') }],
						// a slice trigger whose first event throws (no params) – the run fails on its way
						events: [
							{ id: 'evs-t1', type: 'setVariable', params: null, trigger: { type: 'click', rect: { x: 0, y: 0, width: 100, height: 100 } } },
							{ id: 'evs-t2', type: 'showMessage', params: { text: t('NEVER'), wait: true }, trigger: { type: 'click', rect: { x: 0, y: 0, width: 100, height: 100 } } }
						]
					}),
					slice(10),
					slice(11)
				]
			}
		)
	]
};
sql(`INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (${quote(KEY)}, 'chapter', 'sandbox', NULL, ${quote(JSON.stringify(legacy))}, NULL, NOW(), 'draft', ${quote(MESSAGE)})`);

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'vn-safety-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=375,812', 'about:blank'], { stdio: 'ignore' });
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
const shot = async (name) => {
	if (SHOTS) writeFileSync(join(SHOTS, `vn-safety-${name}.png`), Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
};
/** the page's state: frozen, and what on screen answers */
const STATE = `(() => {
	const vis = (el) => !!el && el.getClientRects().length > 0 && getComputedStyle(el).visibility !== 'hidden';
	const answerable = {
		click: vis(document.querySelector('.vn-message.visible .continue-overlay')),
		choices: [...document.querySelectorAll('.vn-choices.visible .vn-choice')].filter(vis).map((b) => b.innerText.trim()),
		ask: vis(document.querySelector('[data-testid="vn-ask"]')),
		tap: vis(document.querySelector('.text-box-wait'))
	};
	return {
		frozen: document.documentElement.dataset.vnBlocking !== undefined,
		text: document.querySelector('.vn-message.visible .text')?.innerText.trim() ?? null,
		answerable,
		any: answerable.click || answerable.choices.length > 0 || answerable.ask || answerable.tap,
		next: !!document.querySelector('[data-testid="next-bar"]')
	};
})()`;
const state = () => js(STATE);
/** THE invariant, checked at every step: frozen only with something to answer */
let invariantOk = true;
async function settled(label) {
	await sleep(400);
	const s = await state();
	if (s.frozen && !s.any) {
		invariantOk = false;
		console.log(`FAIL frozen with nothing to answer at "${label}": ${JSON.stringify(s)}`);
	}
	return s;
}
const waitText = (re, ms = 10000) => until(`(() => { const s = ${STATE}; return ${re}.test(s.text ?? '') ? s : null; })()`, ms);
const next = () => js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 2, mobile: true });
	await send('Page.navigate', { url: `${VIEWER}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await js(`localStorage.setItem('scutoons.draft-preview', '1')`);
	saveGame.clear(); // a fresh reader
	await send('Page.navigate', { url: `${VIEWER}/sandbox/chapter/9` });

	let s = await waitText(/^S1 /, 15000);
	check(!!s && s.frozen && s.answerable.click, 'S1 waits for its click (frozen, the click answers)', JSON.stringify(s));
	await shot('s1');
	// a slice trigger that throws while S1 waits: the run ends, S1 comes back and answers
	await js(`(() => { const el = document.querySelector('.event-trigger--click'); const r = el.getBoundingClientRect(); el.parentElement.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: r.left + 20, clientY: r.top + 20 })); return true; })()`);
	s = await settled('after the throwing trigger');
	check(s.text === 'S1 Hallo' && s.answerable.click, `a trigger that throws gives the parked message back: "${s.text}", click ${s.answerable.click}`, JSON.stringify(s));
	check(s.text !== 'NEVER', 'the failed run showed nothing more');
	await next();
	s = await waitText(/^S2 /, 8000);
	s = await settled('after the hidden question');
	check(s?.text === 'S2 after the hidden question' && s.answerable.choices.length === 0, `every choice hidden: no question, the script goes on → "${s?.text}"`, JSON.stringify(s));
	await next();
	s = await waitText(/^S3 /, 8000);
	s = await settled('after the missing jump');
	check(s?.text === 'S3 stayed' && (await js('location.pathname + location.search')) === '/sandbox/chapter/9', `a jump to a missing label stays on the page → "${s?.text}"`, JSON.stringify(s));
	await next();
	s = await until(`(() => { const s = ${STATE}; return s.answerable.tap ? s : null; })()`, 8000);
	s = await settled('the long text box wait');
	check(!!s?.answerable.tap && s.frozen, 'a text box with a long wait: frozen, its tap layer answers', JSON.stringify(s));
	await shot('box-wait');
	const tapped = Date.now();
	await js(`document.querySelector('.text-box-wait')?.click(), true`);
	s = await waitText(/^S4 /, 4000);
	check(!!s && Date.now() - tapped < 4000, `the tap continues at once (not after the box's 9 s) → "${s?.text}"`);
	await next();
	s = await waitText(/^END$/, 8000);
	s = await settled('the end');
	check(s?.text === 'END' && !s.frozen, 'the end: not frozen any more', JSON.stringify(s));
	// the page scrolls, the follow-up bar comes after the last slice
	await js(`(() => { const all = document.querySelectorAll('[data-slice-index]'); all[all.length - 1].scrollIntoView({ block: 'end' }); return true; })()`);
	const bar = await until(`!!document.querySelector('[data-testid="next-bar"]')`, 10000);
	check(!!bar, 'the follow-up bar comes');
	await shot('end');
	check(invariantOk, 'at every step: frozen only with something to answer');
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	console.error(error);
	results.push(false);
} finally {
	try { chrome.kill(); } catch {}
	await sleep(300);
	restore();
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
const failed = results.filter((r) => !r).length;
console.log(failed ? `vn-safety-check: ${failed} of ${results.length} FAILED` : `vn-safety-check: all ${results.length} passed`);
process.exit(failed ? 1 : 0);
