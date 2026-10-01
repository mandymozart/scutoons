#!/usr/bin/env node
// One idempotent start-up routine for the local dev stack (fixed ports: shared/dev-ports.json).
//   node scripts/dev/up.mjs            start every service that is not answering yet, then print the status
//   node scripts/dev/up.mjs --status   only print the status
//   node scripts/dev/up.mjs viewer api  limit to some services
// A service that already answers on its port is never started twice (that race used to make parallel starts fail).
// Logs of services started here: logs/dev/<name>.log (gitignored).
import { spawn } from 'node:child_process';
import { mkdirSync, openSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SERVICES = [
	{ name: 'viewer', path: '/', cmd: 'npm run dev -w viewer' },
	{ name: 'cdn', path: '/', cmd: `python server.py ${PORTS.cdn}`, cwd: 'cdn', python: true },
	{ name: 'api', path: '/api/health/', cmd: 'node server/bin/dev-serve.mjs' },
	{ name: 'qr', path: '/version.json', cmd: `php -S localhost:${PORTS.qr} -t qr/public qr/public/router.php` },
	{ name: 'creator', path: '/', cmd: 'npm run dev -w creator' },
	{ name: 'admin', path: '/', cmd: 'npm run dev -w admin' },
	{ name: 'shop', path: '/', cmd: 'npm run dev -w shop' },
	{ name: 'embassy', path: '/', cmd: 'npm run dev -w embassy' }
].map((s) => ({ ...s, port: PORTS[s.name] }));
for (const name of Object.keys(PORTS)) if (!SERVICES.some((s) => s.name === name)) throw new Error(`shared/dev-ports.json: no start command for "${name}" in scripts/dev/up.mjs`);

const args = process.argv.slice(2);
const statusOnly = args.includes('--status');
const only = args.filter((a) => !a.startsWith('--'));
const selected = only.length ? SERVICES.filter((s) => only.includes(s.name)) : SERVICES;

async function answers(s) {
	try {
		const res = await fetch(`http://localhost:${s.port}${s.path}`, { signal: AbortSignal.timeout(3000) });
		return res.status < 500;
	} catch {
		return false;
	}
}

function start(s) {
	mkdirSync(resolve(ROOT, 'logs/dev'), { recursive: true });
	const log = openSync(resolve(ROOT, `logs/dev/${s.name}.log`), 'a');
	const cmd = s.python && process.platform === 'win32' ? s.cmd.replace(/^python /, `${resolve(ROOT, 'cdn/.venv/Scripts/python.exe')} `) : s.cmd;
	const child = spawn(cmd, { cwd: resolve(ROOT, s.cwd ?? '.'), shell: true, detached: true, stdio: ['ignore', log, log], windowsHide: true });
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
	let state = (await answers(s)) ? 'running' : 'down';
	if (state === 'down' && !statusOnly) {
		start(s);
		state = (await waitFor(s)) ? 'started' : `FAILED (see logs/dev/${s.name}.log)`;
	}
	rows.push([s.name, `http://localhost:${s.port}`, state]);
}
for (const [name, url, state] of rows) console.log(`${name.padEnd(8)} ${url.padEnd(24)} ${state}`);
process.exit(rows.some((r) => r[2].startsWith('FAILED')) ? 1 : 0);
