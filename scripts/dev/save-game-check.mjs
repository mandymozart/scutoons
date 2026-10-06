#!/usr/bin/env node
// The ONE save game (owner 2026-10-06, agents/DATA.md "The save game") in headless Chrome (its own profile) on the local
// dev stack:
//   - a guest's old per-toon game states (`scutoons_game_<toon>`) are taken into `scutoons_game_scutoons` on load and
//     removed: a variable in two toons → the NEWEST state's value; checkpoints, history and the map kept
//   - signing in: the account's save game (newer) and the device's become one – the account's variables, both sides'
//     checkpoints – and the merge is pushed (the server holds the device's checkpoint too)
//   - one variable across toons (the mama story drafts, agents/EXAMPLES.md): catpope's 3 tips + Top Toy's in toptoy = 4,
//     then catpope offers "Zu Mama!" (needs 4)
// The test user's save game (progress row `scutoons`) is put back as it was.
//
//   node scripts/dev/save-game-check.mjs        (viewer :8000, API :9000, MariaDB container)
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => (s === null ? 'NULL' : `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`);
const userId = sql(`SELECT id FROM users WHERE email = ${quote(email)}`);
const before = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
function restore() {
	if (before) {
		const tab = before.indexOf('\t');
		sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
		sql(`INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (${quote(userId)}, 'scutoons', ${quote(before.slice(tab + 1))}, ${Number(before.slice(0, tab))})`);
	} else sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	const now = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	console.log(`${now === before ? 'ok  ' : 'FAIL'} restored the test user's save game (progress row scutoons)`);
	if (now !== before) process.exitCode = 1;
}

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'save-game-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=390,844', 'about:blank'], { stdio: 'ignore' });
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
const iso = (ms) => new Date(ms).toISOString();
const cp = (uri, at) => ({ uri, isComplete: false, position: 0.5, created_at: iso(at), updated_at: iso(at) });
const SAVE = `JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null')`;
/** the mama story's drafts are in this database (agents/EXAMPLES.md) – else that part is skipped */
async function mamaStory() {
	const res = await fetch(`${API}/entities/chapter:toptoy:1?draft=1`, { headers: { Authorization: `Bearer ${token}` } }).catch(() => null);
	const ok = !!res?.ok && JSON.stringify(await res.json()).includes('tip_hypercity');
	if (!ok) console.log('skip the mama story part (its drafts are not in this database)');
	return ok;
}

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Page.navigate', { url: `${VIEWER}/about` });
	await until('document.readyState === "complete"');
	// ── a guest with the old per-toon states (toptoy older than catpope; the save game the oldest) ──
	const t0 = Date.now() - 3 * 86400_000;
	await js(`(() => {
		localStorage.clear();
		localStorage.setItem('scutoons_game_scutoons', ${JSON.stringify(JSON.stringify({ map: { places: { harbour: { unlocked: 5 } }, routes: {}, positions: {} }, created_at: iso(t0), updated_at: iso(t0) }))});
		localStorage.setItem('scutoons_game_toptoy', ${JSON.stringify(JSON.stringify({ toonId: 'toptoy', variables: { mama_tips: 5, visits_hypercity: 1 }, progress: [cp('toptoy/chapter/1', t0 + 1000)], history: {}, created_at: iso(t0 - 5000), updated_at: iso(t0 + 1000) }))});
		localStorage.setItem('scutoons_game_catpope', ${JSON.stringify(JSON.stringify({ toonId: 'catpope', variables: { mama_tips: 2, visits_harbour: 3 }, progress: [cp('catpope/chapter/2', t0 + 2000)], history: { 'event:echeck1': { n: 2, first: t0, last: t0 + 2000 } }, created_at: iso(t0), updated_at: iso(t0 + 2000) }))});
		return true;
	})()`);
	await send('Page.navigate', { url: `${VIEWER}/about` });
	const guest = await until(`(() => { const s = ${SAVE}; return s && s.variables && Object.keys(s.variables).length ? s : null; })()`, 15000);
	check(guest?.variables?.mama_tips === 2, `a clash: the newest state's value wins (mama_tips = ${guest?.variables?.mama_tips}, catpope's 2 over toptoy's 5)`, JSON.stringify(guest));
	check(guest?.variables?.visits_hypercity === 1 && guest?.variables?.visits_harbour === 3, 'every toon\u2019s variables are in the one save game');
	check(['catpope/chapter/2', 'toptoy/chapter/1'].every((u) => guest?.progress?.some((c) => c.uri === u)), 'both toons\u2019 checkpoints kept (keyed by toon)');
	check(guest?.history?.['event:echeck1']?.n === 2, 'the history kept');
	check(!!guest?.map?.places?.harbour, 'the map kept');
	check(guest?.created_at === iso(t0 - 5000), `created_at = the earliest start (${guest?.created_at})`);
	const left = await js(`Object.keys(localStorage).filter((k) => k.startsWith('scutoons_game_') && k !== 'scutoons_game_scutoons')`);
	check(left.length === 0, `the old keys are removed (${JSON.stringify(left)})`);

	// ── signing in: the account's save game is newer ──
	const t1 = Date.now() + 3600_000;
	sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	sql(`INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (${quote(userId)}, 'scutoons', ${quote(JSON.stringify({ variables: { mama_tips: 9 }, progress: [cp('yellowmouse/chapter/1', t1)], history: {}, map: { places: { market: { unlocked: 7 } }, routes: {}, positions: {} }, created_at: iso(t0 + 9000), updated_at: iso(t1) }))}, ${t1})`);
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	await send('Page.navigate', { url: `${VIEWER}/about` });
	const member = await until(`(() => { const s = ${SAVE}; return s && s.variables.mama_tips === 9 ? s : null; })()`, 15000);
	check(!!member, `the account's newer variables win (mama_tips = ${(await js(SAVE))?.variables?.mama_tips})`);
	check(['catpope/chapter/2', 'toptoy/chapter/1', 'yellowmouse/chapter/1'].every((u) => member?.progress?.some((c) => c.uri === u)), 'checkpoints of both sides');
	check(!!member?.map?.places?.harbour && !!member?.map?.places?.market, 'the map of both sides');
	const pushed = await (async () => {
		const end = Date.now() + 10000;
		while (Date.now() < end) {
			const s = JSON.parse(sql(`SELECT state FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`) || 'null');
			if (s?.progress?.some((c) => c.uri === 'catpope/chapter/2')) return s;
			await sleep(300);
		}
		return null;
	})();
	check(!!pushed && pushed.variables.mama_tips === 9, 'the merge is pushed to the account (the device\u2019s checkpoints on the server, the account\u2019s variables)', JSON.stringify(pushed));

	// \u2500\u2500 one variable across toons: the mama story (drafts, agents/EXAMPLES.md) \u2013 catpope's 3 tips, Top Toy's in toptoy
	// counts on, then catpope offers "Zu Mama!" (needs 4) \u2500\u2500
	if (await mamaStory()) {
		sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
		const fresh = { variables: { mama_tips: 3, tip_hat: 'yes', tip_crown: 'yes', tip_road: 'yes' }, progress: [], history: {}, map: { places: {}, routes: {}, positions: {} }, created_at: iso(Date.now()), updated_at: iso(Date.now()) };
		await js(`(() => { localStorage.setItem('scutoons_game_scutoons', ${JSON.stringify(JSON.stringify(fresh))}); localStorage.setItem('scutoons.draft-preview', '1'); return true; })()`);
		await send('Page.navigate', { url: `${VIEWER}/toptoy/chapter/1?label=mama-hypercity` });
		let said = null;
		for (let i = 0; i < 30 && !said; i++) {
			const text = await until(`document.querySelector('.vn-message.visible .text')?.innerText.trim() || null`, 8000);
			if (/Tipp Nummer|tip number/.test(text ?? '')) said = text;
			else await js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
			await sleep(250);
		}
		check(/\b4\b/.test(said ?? ''), `toptoy counts catpope's tips on: "${said}"`);
		check((await js(SAVE))?.variables?.mama_tips === 4, 'the one save game holds mama_tips = 4');
		await send('Page.navigate', { url: `${VIEWER}/catpope/chapter/2?label=mama-crossroads` });
		// the crossroads' script starts when its first slice is seen: its slices are the chain's last two
		await until(`document.querySelectorAll('[data-slice-index]').length > 2`, 15000);
		await js(`(() => { const s = [...document.querySelectorAll('[data-slice-index]')]; s[s.length - 2].scrollIntoView({ block: 'start' }); return true; })()`);
		let choices = null;
		for (let i = 0; i < 30 && !choices; i++) {
			choices = await js(`(() => { const c = [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()); return c.length ? c : null; })()`);
			if (!choices) {
				await until(`!!document.querySelector('.vn-message.visible .continue-overlay') || document.querySelectorAll('.vn-choice').length > 0`, 8000);
				await js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
				await sleep(250);
			}
		}
		check(choices?.some((c) => /Zu Mama|To Mama/.test(c)), `catpope offers "Zu Mama!" with the tip from toptoy (${JSON.stringify(choices)})`);
	}
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	console.error(error);
	results.push(false);
} finally {
	try {
		await send('Network.clearBrowserCookies');
	} catch {}
	try { chrome.kill(); } catch {}
	await sleep(300);
	restore();
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
const failed = results.filter((r) => !r).length;
console.log(failed ? `save-game-check: ${failed} of ${results.length} FAILED` : `save-game-check: all ${results.length} passed`);
process.exit(failed || process.exitCode ? 1 : 0);
