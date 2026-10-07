#!/usr/bin/env node
// A LOCAL account for the owner to test with in any browser (owner 2026-10-08: "create a test admin account for me …
// admin creator ambassador and member"): signs the email up on the local API (a local promo code, the OTP from
// server/logs/mail.log), throws the session away and grants the roles with the documented CLI. Local dev only – the
// local database has no users from production (server/docs/DATABASE.md "Production seed").
// Signing in afterwards: the account page, the email, the 6-digit code from server/logs/mail.log (MAIL_TRANSPORT=log).
//
//   node scripts/dev/local-account.mjs <email> [role …]      roles default: admin creator ambassador (member comes with the signup)
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { localSql as sql, phpDbEnv, phpMysqlArgs } from './db-env.mjs';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = local('api', '/api').replace(/\/$/, '');
const MAIL_LOG = resolve(ROOT, 'server/logs/mail.log');
const [email, ...rolesArg] = process.argv.slice(2);
const roles = rolesArg.length ? rolesArg : ['admin', 'creator', 'ambassador'];

const fail = (msg) => (console.error(`local-account: ${msg}`), process.exit(1));
if (!email || !/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) fail('usage: node scripts/dev/local-account.mjs <email> [role …]');
if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(API).hostname)) fail(`refusing: ${API} is not local`);

const post = async (path, body) => {
	const res = await fetch(API + path, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
	return { status: res.status, json: await res.json().catch(() => null) };
};
function otpSince(since) {
	if (!existsSync(MAIL_LOG)) return null;
	const blocks = readFileSync(MAIL_LOG, 'utf8').split(/^=== /m);
	for (let i = blocks.length - 1; i >= 0; i--) {
		const b = blocks[i];
		if (!(Date.parse(b.slice(0, b.indexOf(' ==='))) >= since - 2000)) continue;
		if (!new RegExp(`^To: ${email.replace(/[.+]/g, '\\$&')}\\s*$`, 'mi').test(b)) continue;
		const m = b.match(/^Subject: (\d{6})\b/m);
		if (m) return m[1];
	}
	return null;
}

const [users] = sql(['SELECT id FROM users WHERE email = :email'], { email });
if (!users.length) {
	const [codes] = sql([`SELECT code FROM qr WHERE type = 'promo' AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > :now) ORDER BY (code = 'promo_test') DESC, created_at LIMIT 1`], { now: new Date().toISOString().slice(0, 19).replace('T', ' ') });
	const code = codes[0]?.code;
	if (!code) fail('no usable local promo (signup) code – create one in the admin app (Codes → type promo)');
	const since = Date.now();
	const req = await post('/auth/request/', { email, locale: 'de', code });
	if (req.status !== 200) fail(`/auth/request/ → ${req.status} ${JSON.stringify(req.json)}`);
	let otp = null;
	for (let i = 0; i < 20 && !otp; i++) (otp = otpSince(since)) || (await new Promise((r) => setTimeout(r, 250)));
	if (!otp) fail('no login mail in server/logs/mail.log (MAIL_TRANSPORT = log?)');
	const ver = await post('/auth/verify/', { email, otp });
	if (ver.status !== 200) fail(`/auth/verify/ → ${ver.status} ${JSON.stringify(ver.json)}`);
	console.log(`created ${email} locally (signup code ${code}); the session was not kept`);
} else console.log(`${email} exists locally`);

for (const role of roles) execFileSync('php', [...phpMysqlArgs(), 'bin/console', 'roles', 'grant', email, role], { cwd: resolve(ROOT, 'server'), env: { ...process.env, ...phpDbEnv() }, stdio: 'inherit' });
execFileSync('php', [...phpMysqlArgs(), 'bin/console', 'roles', 'list', email], { cwd: resolve(ROOT, 'server'), env: { ...process.env, ...phpDbEnv() }, stdio: 'inherit' });
console.log('\nSign in: the account page → this email → the 6-digit code from server/logs/mail.log.');
