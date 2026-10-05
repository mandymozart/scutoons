#!/usr/bin/env node
// One idempotent start-up routine for the local dev stack (fixed ports: shared/dev-ports.json).
//   node scripts/dev/up.mjs            start every service that is not answering yet, then print the status
//   node scripts/dev/up.mjs --status   only print the status
//   node scripts/dev/up.mjs viewer api  limit to some services
// A service that already answers on its port is never started twice (that race used to make parallel starts fail).
// Logs of services started here: logs/dev/<name>.log (gitignored).
// Database: the MariaDB container (scripts/dev/db-env.mjs) is started first (docker compose) and the API runs on it;
// a fresh setup (migrations + production seed) is `npm run dev:db`. adminer only starts when named.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, openSync, readFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compose, MARIADB } from './db-env.mjs';
import { PORTS } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
// the Ask service (tools/mcp, CUTOVER 10.28) needs the Anthropic key – without one it is not started
const askKey = !!process.env.ANTHROPIC_API_KEY || (existsSync(resolve(ROOT, '.env')) && /^ANTHROPIC_API_KEY=\S/m.test(readFileSync(resolve(ROOT, '.env'), 'utf8')));
const SERVICES = [
	// docker services (docker/compose.yml): mariadb always (the only database), adminer only on request
	{ name: 'mariadb', docker: true, tcp: true },
	{ name: 'adminer', docker: true, path: '/', profile: 'tools', skip: 'off (npm run dev -- adminer, or npm run dev:db adminer)' },
	{ name: 'viewer', path: '/', cmd: 'npm run dev -w viewer' },
	{ name: 'cdn', path: '/', cmd: 'node scripts/dev/media-serve.mjs' },
	{ name: 'api', path: '/api/health/', cmd: 'node server/bin/dev-serve.mjs' },
	{ name: 'qr', path: '/version.json', cmd: `php -S localhost:${PORTS.qr} -t qr/public qr/public/router.php` },
	{ name: 'ask', path: '/api/ask/health', cmd: 'node tools/mcp/bin/scutoons-ask.mjs', skip: !askKey && 'off (no ANTHROPIC_API_KEY in the repo-root .env)' },
	{ name: 'creator', path: '/', cmd: 'npm run dev -w creator' },
	{ name: 'admin', path: '/', cmd: 'npm run dev -w admin' },
	{ name: 'shop', path: '/', cmd: 'npm run dev -w shop' },
	{ name: 'embassy', path: '/', cmd: 'npm run dev -w embassy' }
].map((s) => ({ ...s, port: PORTS[s.name] }));
for (const name of Object.keys(PORTS)) if (!SERVICES.some((s) => s.name === name)) throw new Error(`shared/dev-ports.json: no start command for "${name}" in scripts/dev/up.mjs`);

const args = process.argv.slice(2);
const statusOnly = args.includes('--status');
const only = args.filter((a) => !a.startsWith('--'));
const selected = only.length ? SERVICES.filter((s) => only.includes(s.name)).map((s) => ({ ...s, skip: false })) : SERVICES;

function tcpOpen(port) {
	return new Promise((done) => {
		const socket = createConnection({ host: '127.0.0.1', port, timeout: 2000 });
		socket.on('connect', () => (socket.destroy(), done(true)));
		socket.on('error', () => done(false));
		socket.on('timeout', () => (socket.destroy(), done(false)));
	});
}

async function answers(s) {
	if (s.tcp) return tcpOpen(s.port);
	try {
		const res = await fetch(`http://localhost:${s.port}${s.path}`, { signal: AbortSignal.timeout(3000) });
		return res.status < 500;
	} catch {
		return false;
	}
}

function start(s) {
	if (s.docker) {
		const r = compose([...(s.profile ? ['--profile', s.profile] : []), 'up', '-d', '--wait', s.name], { stdio: ['ignore', 'pipe', 'pipe'] });
		if (r.error || r.status !== 0) {
			const hint = /pipe|daemon|connect/i.test(r.stderr ?? '') ? ' – is Docker Desktop running?' : '';
			console.error(`${s.name}: docker compose failed${hint}\n${(r.stderr || r.error?.message || '').trim()}`);
		}
		return;
	}
	mkdirSync(resolve(ROOT, 'logs/dev'), { recursive: true });
	const log = openSync(resolve(ROOT, `logs/dev/${s.name}.log`), 'a');
	const child = spawn(s.cmd, { cwd: resolve(ROOT, s.cwd ?? '.'), shell: true, detached: true, stdio: ['ignore', log, log], windowsHide: true });
	child.unref();
}

async function waitFor(s, ms = 60000) {
	const until = Date.now() + ms;
	while (Date.now() < until) {
		if (await answers(s)) return true;
		await new Promise((r) => setTimeout(r, 1000));
	}
	return false;
}

const rows = [];
for (const s of selected) {
	const url = s.tcp ? `127.0.0.1:${s.port}` : `http://localhost:${s.port}`;
	let state = (await answers(s)) ? 'running' : 'down';
	if (state === 'down' && s.skip) {
		rows.push([s.name, url, s.skip]);
		continue;
	}
	if (state === 'down' && !statusOnly) {
		start(s);
		state = (await waitFor(s)) ? 'started' : `FAILED (${s.docker ? 'docker compose -f docker/compose.yml logs ' + s.name : `see logs/dev/${s.name}.log`})`;
	}
	rows.push([s.name, url, state]);
}
for (const [name, url, state] of rows) console.log(`${name.padEnd(8)} ${url.padEnd(24)} ${state}`);
console.log(`database: MariaDB container, database ${MARIADB.name} (a fresh setup: npm run dev:db)`);
process.exit(rows.some((r) => r[2].startsWith('FAILED')) ? 1 : 0);
