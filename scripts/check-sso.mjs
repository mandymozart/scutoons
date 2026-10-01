// Single sign-on check (owner 2026-10-01, server/docs/API.md "Single sign-on", shared/auth): headless Chrome via CDP (own
// temp profile, no dependencies), like scripts/check-landing.mjs.
//
//   1. viewer /account: sign in as the test user (agent@scutoons.test: member, creator, admin) through the shared login
//      form (email → the 6-digit code from the mail log); the session is the HttpOnly cookie, no token in localStorage
//   2. creator, admin, embassy and the shop open signed in – no second sign-in
//   3. a throw-away account with only `member` (normal signup through the API with a local promo code, deleted at the
//      end): signed in on the viewer, the admin, creator and embassy show "Dein Konto (x) hat keinen Zugang zu dieser
//      App." → "Mit anderem Konto anmelden" → the admin's sign-in screen, and the viewer is signed out too
//      then the same account gets `admin` (no `creator`): it gets into the creator (admins edit content "for now")
//   4. sign in on the admin's own screen (the test user) → the creator is signed in; a sign-out elsewhere reaches the open
//      creator on focus (no reload); sign in on the creator's screen, sign out there → the
//      viewer, admin and embassy show their sign-in
//   5. migration: a token in localStorage `scutoons_session` (an app from before single sign-on) is moved into the
//      cookie once and deleted from localStorage
//
// usage: node scripts/check-sso.mjs [--viewer=http://localhost:8000] [--creator=http://localhost:10000]
//        [--admin=http://localhost:11000] [--shop=http://localhost:12000] [--embassy=http://localhost:13000]
//        [--api=http://localhost:9000/api] [--mail-log=server/logs/mail.log] [--db=server/db/local.sqlite] [--verbose]
// Local only (refuses a non-localhost API). --db is the API's SQLite file (the throw-away account is removed from it).
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const url = (name, fallback) => String(opt[name] || fallback).replace(/\/$/, '');
const apps = {
	viewer: url('viewer', 'http://localhost:8000'),
	creator: url('creator', 'http://localhost:10000'),
	admin: url('admin', 'http://localhost:11000'),
	shop: url('shop', 'http://localhost:12000'),
	embassy: url('embassy', 'http://localhost:13000')
};
const api = url('api', 'http://localhost:9000/api');
const mailLog = resolve(root, String(opt['mail-log'] || 'server/logs/mail.log'));
const db = resolve(root, String(opt.db || process.env.QR_DB_SQLITE_PATH || 'server/db/local.sqlite'));
const TEST_USER = 'agent@scutoons.test';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const LOCAL = new Set(['localhost', '127.0.0.1', '[::1]']);
if (!LOCAL.has(new URL(api).hostname)) { console.error(`refusing: ${api} is not local`); process.exit(2); }
if (!existsSync(db)) { console.error(`SQLite database not found: ${db} (--db)`); process.exit(2); }

// ── API + DB helpers (the throw-away account) ───────────────────────────────────────────────────────────────────────
async function call(method, path, body, headers = {}) {
	const r = await fetch(api + path, { method, headers: { 'Content-Type': 'application/json', ...headers }, body: body === undefined ? undefined : JSON.stringify(body) });
	const text = await r.text();
	let json = null;
	try { json = JSON.parse(text); } catch { /* not JSON */ }
	return { status: r.status, json, text };
}
/** The newest one-time code mailed to `email` at or after `since` (ms) – the subject starts with the 6 digits */
async function otpFor(email, since) {
	for (let i = 0; i < 40; i++) {
		if (existsSync(mailLog)) {
			const blocks = readFileSync(mailLog, 'utf8').split(/^=== /m);
			for (let j = blocks.length - 1; j >= 0; j--) {
				const b = blocks[j];
				if (!(Date.parse(b.slice(0, b.indexOf(' ==='))) >= since - 2000)) continue;
				if (!new RegExp(`^To: ${email.replace(/[.+]/g, '\\$&')}\\s*$`, 'mi').test(b)) continue;
				const m = b.match(/^Subject: (\d{6})\b/m);
				if (m) return m[1];
			}
		}
		await sleep(250);
	}
	throw new Error(`no login mail for ${email} in ${mailLog}`);
}
function sql(statements, params = {}) {
	const php = `$d=new PDO('sqlite:'.getenv('SSO_DB'));$d->setAttribute(PDO::ATTR_ERRMODE,PDO::ERRMODE_EXCEPTION);$p=json_decode(getenv('SSO_PARAMS'),true);$out=[];foreach(json_decode(getenv('SSO_SQL'),true) as $s){$st=$d->prepare($s);$st->execute(array_intersect_key($p,array_flip(array_map(fn($m)=>substr($m,1),(preg_match_all('/:\\w+/',$s,$m)?$m[0]:[])))));$out[]=$st->columnCount()?$st->fetchAll(PDO::FETCH_ASSOC):$st->rowCount();}echo json_encode($out);`;
	return JSON.parse(execFileSync('php', ['-r', php], { env: { ...process.env, SSO_DB: db, SSO_SQL: JSON.stringify(statements), SSO_PARAMS: JSON.stringify(params) }, encoding: 'utf8' }));
}
const [testUser] = sql(['SELECT id FROM users WHERE email = :email'], { email: TEST_USER });
if (!testUser.length) { console.error(`${TEST_USER} does not exist in ${db} – run npm run dev:test-user first`); process.exit(2); }
const [promoRows] = sql([`SELECT code FROM qr WHERE type = 'promo' AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > datetime('now')) ORDER BY (code = 'promo_test') DESC, created_at LIMIT 1`]);
if (!promoRows.length) { console.error('no local promo (signup) code in the DB'); process.exit(2); }
const memberEmail = `sso-member-${Date.now().toString(36)}@scutoons.test`;
/** The throw-away account: the normal signup (promo code → one-time code → verify), role member only */
async function createMember() {
	const since = Date.now();
	const req = await call('POST', '/auth/request/', { email: memberEmail, code: promoRows[0].code, locale: 'de' });
	if (req.status !== 200) throw new Error(`signup request ${req.status} ${req.text}`);
	const ver = await call('POST', '/auth/verify/', { email: memberEmail, otp: await otpFor(memberEmail, since) });
	if (ver.status !== 200 || !ver.json?.created) throw new Error(`signup verify ${ver.status} ${ver.text}`);
	return ver.json.user;
}
function deleteMember() {
	const [rows] = sql(['SELECT id FROM users WHERE email = :email'], { email: memberEmail });
	if (!rows.length) return 0;
	const id = rows[0].id;
	const tables = ['sessions', 'user_roles', 'inventory', 'progress', 'avatar_slots', 'avatar_body', 'member_characters', 'member_character_looks', 'code_events'];
	sql([...tables.map((t) => `DELETE FROM ${t} WHERE user_id = :id`), 'DELETE FROM login_requests WHERE email = :email', 'DELETE FROM users WHERE id = :id'], { id, email: memberEmail });
	return 1;
}

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-sso-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1280,800', 'about:blank'], { stdio: 'ignore' });
let memberCreated = false;
async function done(code) {
	if (memberCreated) {
		try { console.log(`\n  cleanup: throw-away account ${deleteMember() ? 'deleted' : 'not found'} (${memberEmail})`); } catch (e) { console.error('cleanup failed', e); code = code || 1; }
	}
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	process.exit(code);
}
process.on('uncaughtException', (e) => { console.error(e); done(2); });
process.on('unhandledRejection', (e) => { console.error(e); done(2); });
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); let lastNet = Date.now();
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
async function idle(quietMs = 600, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 15000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(200); } return null; }
async function open(target) { lastNet = Date.now(); await send('Page.navigate', { url: target }); await sleep(300); await idle(); }
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
const q = (s) => JSON.stringify(s);
/** the visible state of the current app: sign-in form, denied notice, signed-in markers */
const STATE = `(() => {
	const t = (s) => document.querySelector(s)?.textContent.replace(/\\s+/g, ' ').trim() ?? null;
	return {
		path: location.pathname,
		form: document.querySelector('[data-testid="login-form"]')?.dataset.step ?? null,
		gate: document.querySelector('[data-testid$="-sign-in"]')?.dataset.testid ?? null,
		denied: t('[data-testid="auth-denied"]'),
		signedInAs: t('[data-testid="signed-in-as"] .email'),
		creatorBar: !!document.querySelector('[data-testid="creator-bar"]'),
		topbarEmail: t('.topbar .email'),
		storedToken: localStorage.getItem('scutoons_session')
	};
})()`;
const state = () => evaluate(STATE);
/** Fill an input the Svelte way (bind:value listens to input events) */
const fill = (selector, value) => evaluate(`(() => { const el = document.querySelector(${q(selector)}); if (!el) return false; const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(el, ${q(value)}); el.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
const click = (selector) => evaluate(`(() => { const el = document.querySelector(${q(selector)}); if (!el) return false; el.click(); return true; })()`);
async function cookie() {
	const r = await send('Network.getCookies', { urls: [api + '/'] });
	return (r.result?.cookies ?? []).find((c) => c.name === 'scutoons_sid') ?? null;
}
/** The shared login form on the current page: email → code from the mail log → signed in */
async function signIn(email) {
	const problems = [];
	if (!(await waitFor(`document.querySelector('[data-testid="login-form"] input[name="email"]') && true`))) return ['no login form'];
	const since = Date.now();
	await fill('[data-testid="login-form"] input[name="email"]', email);
	await click('[data-testid="login-form"] button[type="submit"]');
	if (!(await waitFor(`document.querySelector('[data-testid="login-form"]')?.dataset.step === 'otp'`))) return ['the code step did not come'];
	await fill('[data-testid="login-form"] input[name="otp"]', await otpFor(email, since));
	await click('[data-testid="login-form"] button[type="submit"]');
	if (!(await waitFor(`!document.querySelector('[data-testid="login-form"]')`))) problems.push('still on the login form after the code');
	await idle();
	return problems;
}

console.log(`SSO check · API ${api} · viewer ${apps.viewer} · creator ${apps.creator} · admin ${apps.admin} · shop ${apps.shop} · embassy ${apps.embassy}\n`);
try {
	// ── 1. viewer: sign in as the test user ────────────────────────────────────────────────────────────────────────
	await open(`${apps.viewer}/de/account`);
	{
		const problems = await signIn(TEST_USER);
		const s = await waitFor(`document.querySelector('[data-testid="signed-in-as"] .email')?.textContent.trim() === ${q(TEST_USER)} && true`);
		if (!s) problems.push(`viewer /account does not say "Angemeldet als ${TEST_USER}": ${JSON.stringify(await state())}`);
		const c = await cookie();
		if (!c) problems.push('no scutoons_sid cookie');
		else {
			if (!c.httpOnly) problems.push('cookie is not HttpOnly');
			if (c.sameSite !== 'Lax') problems.push(`cookie SameSite ${c.sameSite}`);
			if (c.path !== '/api') problems.push(`cookie path ${c.path}`);
		}
		const st = await state();
		if (st.storedToken) problems.push('the token was stored in localStorage');
		const js = await evaluate(`document.cookie.includes('scutoons_sid')`);
		if (js) problems.push('JavaScript can read the session cookie');
		report('viewer: sign in with the shared form → HttpOnly cookie, no token in localStorage', problems, c ? `${c.name} httpOnly=${c.httpOnly} sameSite=${c.sameSite} path=${c.path} domain=${c.domain}` : '');
	}

	// ── 2. the other apps open signed in ───────────────────────────────────────────────────────────────────────────
	for (const [app, ok] of [
		['creator', `document.querySelector('[data-testid="creator-bar"]') && true`],
		['admin', `document.querySelector('.topbar .email')?.textContent.trim() === ${q(TEST_USER)}`],
		['embassy', `document.querySelector('.topbar .email')?.textContent.includes(${q(TEST_USER)})`],
		['shop', `document.querySelector('[data-testid="signed-in-as"] .email')?.textContent.trim() === ${q(TEST_USER)}`]
	]) {
		await open(`${apps[app]}/`);
		const signed = await waitFor(`(${ok}) || null`);
		const st = await state();
		report(`${app}: signed in without signing in again`, signed && !st.gate && !st.form ? [] : [JSON.stringify(st)]);
	}

	// ── 3. an account without the role: the notice + "switch account" ─────────────────────────────────────────────
	await open(`${apps.viewer}/de/account`);
	await click('[data-testid="signed-in-as"] [data-testid="sign-out"]');
	await waitFor(`document.querySelector('[data-testid="login-form"]') && true`);
	await createMember();
	memberCreated = true;
	report('viewer: sign in as a member-only account (normal signup with a promo code)', await signIn(memberEmail));
	for (const app of ['creator', 'embassy', 'admin']) {
		await open(`${apps[app]}/`);
		const text = await waitFor(`document.querySelector('[data-testid="auth-denied"]')?.textContent.trim() || null`);
		const problems = [];
		const expected = [`Dein Konto (${memberEmail}) hat keinen Zugang zu dieser App.`, `Your account (${memberEmail}) does not have access to this app.`];
		if (!expected.includes(text)) problems.push(`notice: ${JSON.stringify(text)} – expected ${JSON.stringify(expected[0])}`);
		const button = await evaluate(`document.querySelector('[data-testid="auth-switch-account"]')?.textContent.trim() ?? null`);
		if (!['Mit anderem Konto anmelden', 'Switch account'].includes(button)) problems.push(`switch button: ${JSON.stringify(button)}`);
		report(`${app}: "no access" notice for the member account + switch button`, problems, text ?? '');
	}
	{
		// on the admin now: switch account → signed out everywhere → the admin's sign-in screen
		await click('[data-testid="auth-switch-account"]');
		const gate = await waitFor(`document.querySelector('[data-testid="admin-sign-in"] [data-testid="login-form"]') && true`);
		const problems = gate ? [] : [`admin after "switch account": ${JSON.stringify(await state())}`];
		if (await cookie()) problems.push('the cookie is still there');
		await open(`${apps.viewer}/de/account`);
		if (!(await waitFor(`document.querySelector('[data-testid="login-form"]') && true`))) problems.push(`viewer still signed in: ${JSON.stringify(await state())}`);
		report('switch account: signed out everywhere, the app shows its sign-in screen', problems);
	}

	// ── 3b. the same account with `admin` (no `creator`) gets into the creator – "for now" (owner 2026-10-01) ─────
	{
		sql([`INSERT INTO user_roles (user_id, role_id, granted_at, granted_by) SELECT u.id, r.id, datetime('now'), 'check-sso' FROM users u, roles r WHERE u.email = :email AND r.name = 'admin'`], { email: memberEmail });
		await open(`${apps.creator}/`);
		const problems = await signIn(memberEmail);
		if (!(await waitFor(`document.querySelector('[data-testid="creator-bar"]') && true`))) problems.push(`creator: ${JSON.stringify(await state())}`);
		if (!(await waitFor(`document.querySelector('[data-testid^="toon-"]') && true`))) problems.push('no toons listed (the draft library refused the admin?)');
		report('creator: an admin-only account (member + admin, no creator) gets in and sees the toons', problems);
		await click('[data-testid="sign-out"]');
		await waitFor(`document.querySelector('[data-testid="creator-sign-in"]') && true`);
	}

	// ── 4. sign in on the admin's own screen, sign out in the creator ──────────────────────────────────────────────
	await open(`${apps.admin}/`);
	{
		const problems = await signIn(TEST_USER);
		if (!(await waitFor(`document.querySelector('.topbar .email')?.textContent.trim() === ${q(TEST_USER)} || null`))) problems.push(`admin not signed in: ${JSON.stringify(await state())}`);
		await open(`${apps.creator}/`);
		if (!(await waitFor(`document.querySelector('[data-testid="creator-bar"]') && true`))) problems.push(`creator not signed in: ${JSON.stringify(await state())}`);
		report('admin: sign in there → the creator is signed in too', problems);
	}
	{
		// another app signs out while the creator stays open: it notices when its tab gets the focus back (no reload)
		const problems = [];
		const c = await cookie();
		const r = await fetch(`${api}/auth/logout/`, { method: 'POST', headers: { Origin: apps.viewer, Cookie: `scutoons_sid=${c?.value}`, 'Content-Type': 'application/json' }, body: '{}' });
		if (!r.ok) problems.push(`logout from "another app" answered ${r.status}`);
		await sleep(2100); // the re-check runs at most every 2 s
		await evaluate(`window.dispatchEvent(new Event('focus')), true`);
		if (!(await waitFor(`document.querySelector('[data-testid="creator-sign-in"]') && true`))) problems.push(`the open creator still shows the app: ${JSON.stringify(await state())}`);
		report('signed out in another app → the open creator shows its sign-in on focus, without a reload', problems);
		report('creator: sign in on its own screen', await signIn(TEST_USER));
	}
	{
		const problems = [];
		if (!(await waitFor(`document.querySelector('[data-testid="sign-out"]') && true`))) problems.push('no sign-out button in the creator');
		await click('[data-testid="sign-out"]');
		if (!(await waitFor(`document.querySelector('[data-testid="creator-sign-in"]') && true`))) problems.push(`creator after sign-out: ${JSON.stringify(await state())}`);
		for (const [app, gate] of [['viewer', 'login-form'], ['admin', 'admin-sign-in'], ['embassy', 'embassy-sign-in'], ['shop', null]]) {
			await open(app === 'viewer' ? `${apps.viewer}/de/account` : `${apps[app]}/`);
			if (gate) {
				if (!(await waitFor(`document.querySelector('[data-testid="${gate}"]') && true`))) problems.push(`${app} still signed in: ${JSON.stringify(await state())}`);
			} else {
				await sleep(800);
				if ((await state()).signedInAs) problems.push(`${app} still shows the account`);
			}
		}
		report('sign out in the creator → viewer, admin, embassy and shop are signed out', problems);
	}

	// ── 5. migration: a token stored by a build from before single sign-on ─────────────────────────────────────────
	{
		const problems = [];
		const since = Date.now();
		await call('POST', '/auth/request/', { email: TEST_USER, locale: 'de' });
		const ver = await call('POST', '/auth/verify/', { email: TEST_USER, otp: await otpFor(TEST_USER, since) }); // no Origin: no cookie
		const token = ver.json?.session_token;
		if (!token) problems.push(`login over the API failed: ${ver.status}`);
		await open(`${apps.creator}/`);
		await evaluate(`localStorage.setItem('scutoons_session', ${q(token)}), true`);
		await open(`${apps.creator}/`);
		if (!(await waitFor(`document.querySelector('[data-testid="creator-bar"]') && true`))) problems.push(`creator with a stored token: ${JSON.stringify(await state())}`);
		if ((await state()).storedToken) problems.push('the stored token is still in localStorage');
		const c = await cookie();
		if (!c || c.value !== token) problems.push('the stored token was not moved into the cookie');
		await open(`${apps.viewer}/de/account`);
		if (!(await waitFor(`document.querySelector('[data-testid="signed-in-as"] .email')?.textContent.trim() === ${q(TEST_USER)} || null`))) problems.push('viewer not signed in after the move');
		report('migration: a stored token moves into the cookie once and leaves localStorage', problems);
		await click('[data-testid="signed-in-as"] [data-testid="sign-out"]');
		await waitFor(`document.querySelector('[data-testid="login-form"]') && true`);
	}
} catch (e) {
	console.error(e);
	failed++;
}

console.log(`\n${passed} / ${passed + failed} checks passed`);
await done(failed ? 1 : 0);
