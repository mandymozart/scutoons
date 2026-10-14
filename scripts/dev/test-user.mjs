#!/usr/bin/env node
// A permanent LOCAL test account for agents (owner, 2026-09-30: "create a test user for yourself, you can debug better").
// Agents debug as agent@scutoons.test – never with the owner's account.
//
//   npm run dev:test-user                          ensure the account + roles + Cat Pope unlock, sign in, write the session
//   npm run dev:test-user -- --fresh               sign in again even if the stored session still works
//   npm run dev:test-user -- --reset               also reset inventory (starter items only), reading progress and avatar
//   npm run dev:test-user -- --browser <app url>   print a one-line JS snippet that signs a browser tab of that app in
//
// How (the proper way, no raw SQL for the account itself):
//   1. POST /api/auth/request/ { email, code: <signup code> } – a local promo code (signup: true) from the local DB
//   2. the 6-digit OTP from server/logs/mail.log (MAIL_TRANSPORT=log) → POST /api/auth/verify/
//   3. roles creator + admin via `php server/bin/console roles grant` (member comes with the signup)
//   4. Cat Pope (every look with an A-pose) via the admin API PUT /api/admin/users/<id>/characters/catpope
// Session: logs/dev/test-user.json { email, token, created } (gitignored). Refuses anything but a localhost API.
// --reset is local-only maintenance on the dev DB (there is no API to empty an inventory).
// Database: the local MariaDB container (scripts/dev/db-env.mjs, npm run dev:db).
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { localSql as sql, phpDbEnv, phpMysqlArgs } from './db-env.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const EMAIL = 'agent@scutoons.test';
const ROLES = ['member', 'creator', 'admin'];
const CHARACTER = 'catpope';
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const SESSION_FILE = resolve(ROOT, 'logs/dev/test-user.json');
const MAIL_LOG = resolve(ROOT, 'server/logs/mail.log');
const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const browserIdx = args.indexOf('--browser');
const browserUrl = browserIdx >= 0 ? args[browserIdx + 1] : null;

function fail(msg) {
	console.error(`test-user: ${msg}`);
	process.exit(1);
}
function isLocal(url) {
	try {
		return LOCAL_HOSTS.has(new URL(url).hostname);
	} catch {
		return false;
	}
}

if (!isLocal(API)) fail(`refusing: the API ${API} is not local (dev only).`);
if (browserIdx >= 0 && (!browserUrl || !isLocal(browserUrl))) fail('--browser needs a local app URL, e.g. http://localhost:10000');

async function api(method, path, body, token) {
	const res = await fetch(API + path, {
		method,
		headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	const text = await res.text();
	let json = null;
	try {
		json = JSON.parse(text);
	} catch {
		/* not JSON */
	}
	return { status: res.status, json, text };
}

function readSession() {
	try {
		return JSON.parse(readFileSync(SESSION_FILE, 'utf8'));
	} catch {
		return null;
	}
}

/** The newest OTP mailed to EMAIL at or after `since` (ms). */
function otpFromMailLog(since) {
	if (!existsSync(MAIL_LOG)) return null;
	const blocks = readFileSync(MAIL_LOG, 'utf8').split(/^=== /m);
	for (let i = blocks.length - 1; i >= 0; i--) {
		const b = blocks[i];
		const at = Date.parse(b.slice(0, b.indexOf(' ===')));
		if (!(at >= since - 2000)) continue;
		if (!new RegExp(`^To: ${EMAIL.replace(/[.]/g, '\\.')}\\s*$`, 'mi').test(b)) continue;
		const m = b.match(/^Subject: (\d{6})\b/m); // "<otp> is your SCUTOONS code" / "<otp> ist dein SCUTOONS-Code"
		if (m) return m[1];
	}
	return null;
}

function signupCode() {
	const [rows] = sql([
		`SELECT code FROM qr WHERE type = 'promo' AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > :now)
		 ORDER BY (code = 'promo_test') DESC, created_at LIMIT 1`
	], { now: new Date().toISOString().slice(0, 19).replace('T', ' ') });
	return rows[0]?.code ?? null;
}

async function login() {
	const [users] = sql(['SELECT id FROM users WHERE email = :email'], { email: EMAIL });
	const exists = users.length > 0;
	const code = exists ? null : signupCode();
	if (!exists && !code) fail('no usable local promo (signup) code in the dev DB; create one in the admin app (Codes → type promo).');
	const since = Date.now();
	const req = await api('POST', '/auth/request/', { email: EMAIL, locale: 'en', ...(code ? { code } : {}) });
	if (req.status !== 200) fail(`/auth/request/ answered ${req.status}: ${req.text.slice(0, 200)}`);
	let otp = null;
	for (let i = 0; i < 20 && !otp; i++) {
		otp = otpFromMailLog(since);
		if (!otp) await new Promise((r) => setTimeout(r, 250));
	}
	if (!otp) fail(`no login mail for ${EMAIL} in server/logs/mail.log (is MAIL_TRANSPORT = log?)`);
	const ver = await api('POST', '/auth/verify/', { email: EMAIL, otp });
	if (ver.status !== 200 || !ver.json?.session_token) fail(`/auth/verify/ answered ${ver.status}: ${ver.text.slice(0, 200)}`);
	if (ver.json.created) console.log(`created ${EMAIL} (signup code ${code})`);
	return ver.json.session_token;
}

async function main() {
	const health = await api('GET', '/health/').catch(() => null);
	if (!health || health.status !== 200) fail(`the API ${API} is not answering – start it with npm run dev`);

	let session = readSession();
	let token = session?.email === EMAIL && !flag('--fresh') ? session.token : null;
	let me = token ? await api('GET', '/me/', undefined, token) : null;
	if (!me || me.status !== 200) {
		token = await login();
		session = { email: EMAIL, token, created: new Date().toISOString() };
		mkdirSync(dirname(SESSION_FILE), { recursive: true });
		writeFileSync(SESSION_FILE, JSON.stringify(session, null, '\t') + '\n');
		me = await api('GET', '/me/', undefined, token);
	}
	const user = me.json.user;

	// roles (member comes with the signup): the documented CLI
	for (const role of ROLES) {
		if (user.roles.includes(role)) continue;
		execFileSync('php', [...phpMysqlArgs(), 'bin/console', 'roles', 'grant', EMAIL, role], { cwd: resolve(ROOT, 'server'), env: { ...process.env, ...phpDbEnv() }, stdio: 'inherit' });
	}

	// Cat Pope with every look that has an A-pose (`<persona>/<look>`): admin API
	const reg = await api('GET', '/library/characters');
	const registry = reg.json?.characters ?? reg.json;
	const entry = Array.isArray(registry) ? registry.find((c) => c.id === CHARACTER) : registry?.[CHARACTER];
	const looks = (entry?.personas ?? []).flatMap((p) => (p?.looks ?? []).filter((l) => l?.apose?.path).map((l) => `${p.id}/${l.id}`));
	let unlock = await api('PUT', `/admin/users/${user.id}/characters/${CHARACTER}`, { looks, source: 'admin', ref: 'dev:test-user' }, token);
	// the library's registry may list a look the server's (published) registry doesn't have yet: the looks it knows
	if (unlock.status === 400 && unlock.json?.error === 'invalid_look' && Array.isArray(unlock.json.looks)) {
		unlock = await api('PUT', `/admin/users/${user.id}/characters/${CHARACTER}`, { looks: looks.filter((l) => unlock.json.looks.includes(l)), source: 'admin', ref: 'dev:test-user' }, token);
	}
	if (unlock.status !== 200) fail(`Cat Pope unlock answered ${unlock.status}: ${unlock.text.slice(0, 200)}`);

	if (flag('--reset')) {
		// known state = a fresh signup: only the starter items (qty 1), no progress, no avatar choices, codes released.
		// The default wearables (starter items with a slot) are removed too: the next GET /me/ grants them again and puts
		// them on, exactly like a signup (agents/ITEMS.md "Wardrobe")
		const catalog = (await api('GET', '/items/')).json;
		const wearables = (catalog?.items ?? []).filter((i) => i.starter && i.slot && /^[a-z0-9_]+$/.test(i.id)).map((i) => `'${i.id}'`);
		const [released, inv, , prog, slots, body] = sql(
			[
				'UPDATE qr SET claimed_by = NULL, claimed_at = NULL WHERE claimed_by = :id',
				`DELETE FROM inventory WHERE user_id = :id AND (source <> 'starter'${wearables.length ? ` OR item_id IN (${wearables.join(', ')})` : ''})`,
				"UPDATE inventory SET qty = 1 WHERE user_id = :id AND source = 'starter'",
				'DELETE FROM progress WHERE user_id = :id',
				'DELETE FROM avatar_slots WHERE user_id = :id',
				'DELETE FROM avatar_body WHERE user_id = :id'
			],
			{ id: user.id }
		);
		// reader stats + effects (agents/ITEMS.md "Substances", tables of 2026-10-01-reader-state when migrated), the
		// starter items a recipe used up (the camera in the cucumber / testo camera) – back like a fresh signup
		const [tables] = sql([
			"SELECT TABLE_NAME AS name FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN ('member_stats', 'member_effects')"
		]);
		const clear = tables.map((t) => `DELETE FROM ${t.name} WHERE user_id = :id`);
		// story pickups are granted once per place (a ref per user + item): a fresh reader can pick them up again
		clear.push('DELETE FROM rate_limits WHERE bucket LIKE :grantref');
		const [owned] = sql(['SELECT DISTINCT item_id FROM inventory WHERE user_id = :id'], { id: user.id });
		const have = new Set(owned.map((r) => r.item_id));
		const missing = (catalog?.items ?? []).filter((i) => i.starter && !i.slot && !have.has(i.id) && /^[a-z0-9_]+$/.test(i.id));
		const now = new Date().toISOString().slice(0, 19).replace('T', ' ');
		const restore = missing.map((i) => `INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES ('${randomUUID()}', :id, '${i.id}', NULL, 1, 'starter', NULL, NULL, '${now}', '${now}')`);
		if (clear.length + restore.length) sql([...clear, ...restore], { id: user.id, grantref: `grantref:${user.id}:%` });
		const pref = await api('PUT', '/me/', { hide_sensitive: true }, token);
		if (pref.status !== 200) fail(`PUT /me/ answered ${pref.status}`);
		console.log(`reset: ${inv} inventory rows, ${prog} progress rows, ${slots + body} avatar rows removed, ${released} codes released; hide_sensitive on`);
	}

	const final = (await api('GET', '/me/', undefined, token)).json;
	console.log(`${EMAIL}  roles: ${final.user.roles.join(', ')}  session: logs/dev/test-user.json (${session.created})`);
	console.log(`  unlocked: ${(final.avatar?.unlocked ?? []).map((u) => `${u.character}[${u.looks.join(', ')}]`).join(', ') || '–'}  inventory: ${final.inventory?.length ?? 0}`);

	if (browserUrl) {
		const origin = new URL(browserUrl).origin;
		console.log(`\n// run in a tab on ${origin} (e.g. javascript_tool), then it reloads signed in:`);
		console.log(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}); location.reload();`);
	}
}

main().catch((e) => fail(e?.stack ?? String(e)));
