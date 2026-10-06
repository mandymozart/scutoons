#!/usr/bin/env node
// Events' repeat rules, askText and expression conditions in the reader (agents/EXPRESSIONS.md §7 step 3), played in
// headless Chrome (phone size, its own profile) on the local dev stack as the local test user: a test group is saved as
// a DRAFT of sandbox chapter 9 and read with the draft preview over three page views (the third with the page's clock
// 61 minutes ahead); afterwards the draft rows, the test user's save game, name, inventory and grant refs are put
// back as they were (the dev DB is as before).
//   - slice click triggers: once for good, once per page view, every 10 minutes, every time (counters in game variables,
//     the history's event:<id>)
//   - a placed item's tap events (once per page view); a pickup with respawn every hour – gone until the next period,
//     the server's grant stays the authority (a second grant in its real period: already taken)
//   - askText: a story variable (typed + Enter / ✓) and the reader's name (Skip → the default), both shown in later
//     messages; each asked only while unanswered (`if: { expr: 'not hero' }`); a message whose condition is an expression
//   - the account page: the member's name field (shows the askText answer, edits it); a guest's name on the device
//
//   node scripts/dev/events-check.mjs        (viewer :8000, API :9000, MariaDB container)
//   EVENTS_SHOTS=<dir>: phone screenshots step3-*.png
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
const SHOTS = process.env.EVENTS_SHOTS;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test group ──────────────────────────────────────────────────────────────────────────────────────────────
const t = (de, en = de) => ({ de, en });
const IMG = (n) => `../catpope/s1/c2/s1c2-slice-${n}.webp`;
const add = (id, variable, trigger) => ({ id, type: 'setVariable', params: { variable, value: 1, op: 'add' }, ...(trigger ? { trigger } : {}) });
const click = (x, y, repeat) => ({ type: 'click', rect: { x, y, width: 50, height: 50 }, ...(repeat ? { repeat } : {}) });
const msg = (id, text, extra = {}) => ({ id, type: 'showMessage', params: { text: t(text), wait: true }, ...extra });
const doc = (base) => ({
	...base,
	groups: [
		{
			label: 'events-a',
			renderer: { type: 'vertical' },
			slices: [
				{
					image: IMG(9),
					width: 390,
					height: 625,
					events: [
						add('evc-once', 'c_once', click(0, 0, { mode: 'once' })),
						add('evc-view', 'c_view', click(50, 0, { mode: 'view' })),
						add('evc-int', 'c_int', click(0, 50, { mode: 'interval', every: { minutes: 10 } })),
						add('evc-every', 'c_every', click(50, 50))
					]
				},
				{
					image: IMG(10),
					width: 390,
					height: 625,
					items: [
						{ id: 'evc-item', item: 'cucumber', rect: { x: 10, y: 20, width: 25 }, action: 'events', repeat: { mode: 'view' }, events: [add('evc-item-e1', 'c_item')] },
						{ id: 'evc-pick', item: 'cucumber', rect: { x: 60, y: 20, width: 25 }, action: 'pickup', respawn: { hours: 1 } }
					]
				},
				{ image: IMG(11), width: 390, height: 625 }
			],
			events: [
				{ id: 'evc-ask1', type: 'askText', if: { expr: 'not hero' }, params: { text: t('Q1 Wie heißt du?', 'Q1 Your name?'), variable: 'hero', placeholder: t('Name'), maxLength: 12 } },
				msg('evc-m1', 'HERO {hero}'),
				{ id: 'evc-ask2', type: 'askText', if: { expr: 'not reader.name' }, params: { text: t('Q2 Und überall?', 'Q2 And everywhere?'), scope: 'reader', default: t('Ada') } },
				msg('evc-m2', 'READER {reader.name}'),
				msg('evc-m3', 'COND yes', { if: { expr: 'hero == "Zed" and count("evc-every") == 0' } }),
				msg('evc-m4', 'COND no', { if: { expr: 'count("evc-every") > 0' } }),
				msg('evc-m5', 'END', { params: { text: t('END'), wait: false } })
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
const quote = (s) => (s === null ? 'NULL' : `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`);
const userId = sql(`SELECT id FROM users WHERE email = ${quote(email)}`);
const progressBefore = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
const nameBefore = sql(`SELECT IFNULL(display_name, CHAR(0)) FROM users WHERE id = ${quote(userId)}`);
const inventoryBefore = sql(`SELECT CONCAT(id, CHAR(9), qty, CHAR(9), updated_at) FROM inventory WHERE user_id = ${quote(userId)}`).split('\n').filter(Boolean).map((l) => l.split('\t'));
const limitsBefore = new Set(sql(`SELECT id FROM rate_limits WHERE bucket LIKE ${quote(`grant%:${userId}%`)}`).split('\n').filter(Boolean));

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
	if (progressBefore) {
		const tab = progressBefore.indexOf('\t');
		sql(`UPDATE progress SET state = ${quote(progressBefore.slice(tab + 1))}, updated_at = ${Number(progressBefore.slice(0, tab))} WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	} else sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	const now = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	console.log(`${now === progressBefore ? 'ok  ' : 'FAIL'} restored the test user's save game (progress row scutoons)`);
	if (now !== progressBefore) process.exitCode = 1;
	// the name, the pickup's inventory rows and grant refs (the server's rate_limits rows of this run)
	sql(`UPDATE users SET display_name = ${quote(nameBefore === '\0' ? null : nameBefore)} WHERE id = ${quote(userId)}`);
	const ids = inventoryBefore.map(([id]) => quote(id));
	sql(`DELETE FROM inventory WHERE user_id = ${quote(userId)}${ids.length ? ` AND id NOT IN (${ids.join(',')})` : ''}`);
	for (const [id, qty, updated] of inventoryBefore) sql(`UPDATE inventory SET qty = ${Number(qty)}, updated_at = ${quote(updated)} WHERE id = ${quote(id)}`);
	const limitsNow = sql(`SELECT id FROM rate_limits WHERE bucket LIKE ${quote(`grant%:${userId}%`)}`).split('\n').filter(Boolean);
	const fresh = limitsNow.filter((id) => !limitsBefore.has(id));
	if (fresh.length) sql(`DELETE FROM rate_limits WHERE id IN (${fresh.map(quote).join(',')})`);
	const nameNow = sql(`SELECT IFNULL(display_name, CHAR(0)) FROM users WHERE id = ${quote(userId)}`);
	const invNow = sql(`SELECT CONCAT(id, CHAR(9), qty, CHAR(9), updated_at) FROM inventory WHERE user_id = ${quote(userId)}`).split('\n').filter(Boolean);
	const okAccount = nameNow === nameBefore && invNow.sort().join('\n') === inventoryBefore.map((r) => r.join('\t')).sort().join('\n');
	console.log(`${okAccount ? 'ok  ' : 'FAIL'} restored the test user's name, inventory and grant refs (${fresh.length} ref rows removed)`);
	if (!okAccount) process.exitCode = 1;
}
const saved = await api('PUT', `/entities/${KEY}`, { content: doc(before.content), base: before.version.id, message: 'events-check (removed again)' });
created.push(saved.version.id);

// ── headless Chrome ─────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'events-'));
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
	if (SHOTS) writeFileSync(join(SHOTS, `step3-${name}.png`), Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
};

const STATE = `(() => {
	const game = JSON.parse(localStorage.getItem('scutoons_game_scutoons') ?? '{}');
	return {
		text: document.querySelector('.vn-message.visible .text')?.innerText.trim() ?? null,
		asking: !!document.querySelector('[data-testid="vn-ask"]'),
		vars: game.variables ?? {},
		history: game.history ?? {},
		items: [...document.querySelectorAll('.slice-item.tappable')].length
	};
})()`;
const state = () => js(STATE);
const waitText = (re, ms = 10000) => until(`(() => { const s = ${STATE}; return ${re}.test(s.text ?? '') ? s : null; })()`, ms);
const next = () => js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
/** a click at the middle of a click trigger's area (the slice's handlers check the area) */
const tapTrigger = (i) =>
	js(`(() => {
		const el = document.querySelectorAll('.event-trigger--click')[${i}];
		if (!el) return false;
		const r = el.getBoundingClientRect();
		el.parentElement.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }));
		return true;
	})()`);
/** a tap on a placed item (index 0: the tap-events cucumber, 1: the pickup) */
const tapItem = (id) => js(`document.querySelector('.slice-item.tappable[data-index="${id === 'evc-item' ? 0 : 1}"]')?.click(), true`);
async function answer(text) {
	await js(`(() => { const i = document.querySelector('[data-testid="vn-ask-input"]'); i.value = ${JSON.stringify(text)}; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await js(`document.querySelector('[data-testid="vn-ask-ok"]').click(), true`);
}

/**
 * The page's clock starts 30 s into a fresh 10-minute period (periods are fixed UTC buckets – recurrence.ts `periodOf`):
 * page views 1 and 2 then share one period (else a run that straddles a boundary sees "every 10 min" run again).
 */
const CLOCK_BASE = Math.ceil(Date.now() / 600_000) * 600_000 + 30_000 - Date.now();

/** one page view: the script (questions while unanswered), then every trigger / item twice */
async function pageView(view, offsetMinutes) {
	await js(`localStorage.setItem('evc-clock', ${String(CLOCK_BASE + offsetMinutes * 60_000)})`);
	await send('Page.navigate', { url: `${VIEWER}/sandbox/chapter/9` });
	if (view === 1) {
		let s = await waitText(/^Q1 /, 15000);
		check(s?.asking, 'askText: the question with its input line', JSON.stringify(await state()));
		const frozen = await js(`document.documentElement.dataset.vnBlocking !== undefined && getComputedStyle(document.documentElement).overflow === 'hidden'`);
		check(frozen, 'the page does not scroll while the question waits');
		await shot('ask-text');
		await answer('  Zed\n ');
		s = await waitText(/^HERO /, 5000);
		check(s?.text === 'HERO Zed' && s.vars.hero === 'Zed', `toon scope: trimmed into the story variable → "${s?.text}" (hero = ${JSON.stringify(s?.vars.hero)})`);
		await next();
		s = await waitText(/^Q2 /, 5000);
		check(s?.asking, 'askText with scope reader asks (no name yet)', JSON.stringify(await state()));
		await js(`document.querySelector('[data-testid="vn-ask-skip"]').click(), true`);
		s = await waitText(/^READER /, 5000);
		check(s?.text === 'READER Ada', `reader scope: the answer button stores the default, {reader.name} shows it → "${s?.text}"`);
		await shot('reader-name');
		await sleep(400);
		const stored = sql(`SELECT IFNULL(display_name, '') FROM users WHERE id = ${quote(userId)}`);
		check(stored === 'Ada', `the member's name is on the account (users.display_name = "${stored}")`);
		await next();
		s = await waitText(/^COND /, 5000);
		check(s?.text === 'COND yes', `an expression condition: hero == "Zed" and count("evc-every") == 0 → "${s?.text}"`);
		await next();
	} else {
		const s = await waitText(/^HERO /, 15000);
		check(s && !s.asking, `page view ${view}: answered questions are not asked again (if: not hero / not reader.name) → "${s?.text}"`, JSON.stringify(await state()));
		await next();
		const r = await waitText(/^READER /, 5000);
		check(r?.text === 'READER Ada', `page view ${view}: {reader.name} → "${r?.text}"`);
		await next();
		const c = await waitText(/^COND /, 5000);
		check(c?.text === 'COND no', `page view ${view}: the expression conditions flip (count("evc-every") > 0) → "${c?.text}"`);
		await next();
	}
	await waitText(/^END$/, 5000);
	await until(`document.querySelectorAll('.event-trigger--click').length === 4`, 5000);
	for (let i = 0; i < 4; i++) for (let k = 0; k < 2; k++) {
		await tapTrigger(i);
		await sleep(250);
	}
	await until(`document.querySelectorAll('.slice-item.tappable').length >= 1`, 5000);
	for (let k = 0; k < 2; k++) {
		await tapItem('evc-item');
		await sleep(250);
	}
	await sleep(300);
	return state();
}

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 2, mobile: true });
	// the page's clock, set ahead per page view (localStorage evc-clock = ms): repeat periods without waiting
	await send('Page.addScriptToEvaluateOnNewDocument', {
		source: `(() => { const real = Date.now.bind(Date); const off = () => Number(localStorage.getItem('evc-clock') || 0); Date.now = () => real() + off(); })();`
	});
	await send('Page.navigate', { url: `${VIEWER}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await js(`localStorage.setItem('scutoons.draft-preview', '1')`);
	// a fresh reader: no save game yet (one for every toon – put back afterwards)
	sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	await api('PUT', '/me/', { display_name: null });

	// page view 1: everything runs once / twice
	let s = await pageView(1, 0);
	const counts = (v) => ['c_once', 'c_view', 'c_int', 'c_every', 'c_item'].map((k) => v[k] ?? 0).join(' ');
	check(counts(s.vars) === '1 1 1 2 1', `page view 1: once for good / per page view / every 10 min / every time / item tap → ${counts(s.vars)} (want 1 1 1 2 1)`);
	check(['evc-once', 'evc-view', 'evc-int', 'evc-every', 'evc-item-e1'].every((id) => s.history[`event:${id}`]), 'the runs are in the save game (history event:<id>)', JSON.stringify(Object.keys(s.history)));
	check(s.history['event:evc-every']?.n === 2, `count("evc-every") = ${s.history['event:evc-every']?.n}`);
	// the pickup (respawn every hour): taken, gone for this period
	await tapItem('evc-pick');
	s = await until(`(() => { const s = ${STATE}; return s.items === 1 ? s : null; })()`, 5000);
	check(s, 'the pickup is taken (gone from the slice)', JSON.stringify(await state()));
	check(Number(sql(`SELECT IFNULL(SUM(qty), 0) FROM inventory WHERE user_id = ${quote(userId)} AND item_id = 'cucumber'`)) >= 1, 'the server granted it (inventory)');
	await shot('page-view-1');
	await sleep(2500); // the debounced progress push

	// page view 2 (a reload): per page view again, once for good not, the same 10-minute bucket not
	s = await pageView(2, 0);
	check(counts(s.vars) === '1 2 1 4 2', `page view 2 (reload): → ${counts(s.vars)} (want 1 2 1 4 2)`);
	check(s.items === 1, 'the pickup stays gone in its period');

	// page view 3, 61 minutes later (the page's clock): the 10-minute rule and the pickup are due again
	s = await pageView(3, 61);
	check(counts(s.vars) === '1 3 2 6 3', `page view 3 (+61 min): → ${counts(s.vars)} (want 1 3 2 6 3)`);
	check(s.items === 2, 'the pickup is back on the slice after its hour (respawn as before)');
	await tapItem('evc-pick');
	await sleep(1500);
	s = await state();
	check(s.items === 1, "a second grant in the server's real hour: refused as taken (the server stays the authority) – hidden again");
	await shot('page-view-3');
	await sleep(2500);

	// the account page: the member's name (the askText answer), edited there
	await js(`localStorage.setItem('evc-clock', '0')`);
	await send('Page.navigate', { url: `${VIEWER}/account` });
	const field = await until(`document.querySelector('[data-testid="reader-name-input"]')?.value || null`, 15000);
	check(field === 'Ada', `account page: "Your name" shows the askText answer → "${field}"`);
	await js(`(() => { const i = document.querySelector('[data-testid="reader-name-input"]'); i.value = ' Bea  '; i.dispatchEvent(new Event('input', { bubbles: true })); i.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	await sleep(1200);
	const changed = sql(`SELECT IFNULL(display_name, '') FROM users WHERE id = ${quote(userId)}`);
	check(changed === 'Bea', `account page: editing saves it on the account (trimmed) → "${changed}"`);
	await js(`document.querySelector('[data-testid="reader-name"]').scrollIntoView({ block: 'center' }), true`);
	await sleep(300);
	await shot('account-name');
	// a guest: the name stays on the device. Signed out for real: the session lives in the HttpOnly cookie scutoons_sid
	// since single sign-on (@scutoons/auth moves the stored token into it on the first page) – removing the token alone
	// left this browser signed in, and the "guest's" name went to the account whenever the session check was done first
	await send('Network.clearBrowserCookies');
	await js(`localStorage.removeItem('scutoons_session'), true`);
	await send('Page.navigate', { url: `${VIEWER}/account` });
	await until(`!!document.querySelector('[data-testid="reader-name-input"]')`, 15000);
	const asGuest = await until(`(() => { const t = document.querySelector('[data-testid="reader-name"] small')?.textContent ?? ''; return /device|Gerät/.test(t) ? t : null; })()`, 15000);
	check(!!asGuest, `signed out: the field saves on the device ("${asGuest}")`);
	await js(`(() => { const i = document.querySelector('[data-testid="reader-name-input"]'); i.value = 'Gus'; i.dispatchEvent(new Event('input', { bubbles: true })); i.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
	await sleep(500);
	const guest = await js(`localStorage.getItem('scutoons_reader_name')`);
	check(guest === 'Gus', `a guest's name is on the device (localStorage scutoons_reader_name = "${guest}")`);
	const account = sql(`SELECT IFNULL(display_name, '') FROM users WHERE id = ${quote(userId)}`);
	check(account === 'Bea', `… and not on the account (users.display_name stays "${account}")`);
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	check(false, 'the run', String(error?.stack ?? error));
} finally {
	await sleep(1000);
	await restore().catch((error) => check(false, 'restore', String(error)));
	cleanup();
}
const bad = results.filter((r) => !r).length;
console.log(bad ? `events-check: ${bad} failed` : `events-check: all ${results.length} passed`);
process.exit(bad || process.exitCode ? 1 : 0);
