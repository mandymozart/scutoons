#!/usr/bin/env node
// The local database: the MariaDB container (docker/compose.yml), production's engine and settings. The only local
// database (no SQLite since 2026-10-05). Docs: server/docs/DATABASE.md "Local MariaDB" + "Production seed".
// Needs Docker Desktop; PHP's pdo_mysql is added on the fly.
//
//   npm run dev:db                 = up: start the container (wait until healthy), create the databases, migrate, load
//                                    production's content + catalog (server/db/prod-seed.sql, gitignored – how to get
//                                    it is printed when it is missing), a local promo code, (re)start the API on it,
//                                    the test user. Idempotent: run it any time (an unchanged seed is not loaded again)
//   npm run dev:db -- up --no-api  the same without touching the API dev server / test user (e.g. a second worktree)
//   npm run dev:db seed            load server/db/prod-seed.sql again (--force: even when unchanged – local edits of
//                                  content / catalog / assets are replaced); -- seed --from=<dump.sql> extracts it from
//                                  any phpMyAdmin export first (only the 15 seed tables are kept)
//   npm run dev:db sandbox         rebuild the dev-only sandbox toon from the database's character registry (up does it
//                                  after a seed load; shared/content/scripts/make-sandbox.ts)
//   npm run dev:db media           media missing in the local media/ folder from bunny.net (plan; -- media --apply copies;
//                                  never deletes, never overwrites; -- media --prefix=catpope/ for one folder)
//   npm run dev:db status          container, database, what the API runs on
//   npm run dev:db stop            stop the container (data stays in the volume; `npm run dev` starts it again)
//   npm run dev:db reset           drop this database (+ its test database), then `up` again (fresh from the seed)
//   npm run dev:db test            the PHPUnit suite (database <name>_test, server/phpunit.xml.dist);
//                                  options after `--`: npm run dev:db -- test --filter Wardrobe
//   npm run dev:db -- console <args>  php bin/console on the dev database, e.g. -- console migrate --status
//   npm run dev:db sql             a mariadb shell in the container
//   npm run dev:db adminer         Adminer (web UI) on the "adminer" dev port
//   npm run dev:db version         server version, sql_mode, charset, collation (compare with production)
//
// SCUTOONS_DB_NAME=scutoons_<x> works on another database of the same container (a worktree's own). Local dev
// credentials only (docker/compose.yml).
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { compose, MARIADB, phpDbEnv, phpMysqlArgs, ROOT } from './db-env.mjs';
import { local, PORTS } from './ports.mjs';

const SERVER = resolve(ROOT, 'server');
const API_STATE = resolve(ROOT, `logs/dev/api-server-${PORTS.api}.json`);
const PROMO = 'devpromo'; // qr.code is VARCHAR(8)
const [command = 'up', ...rest] = process.argv.slice(2);

function fail(message) {
	console.error(`dev:db: ${message}`);
	process.exit(1);
}

function run(cmd, args, options = {}) {
	const r = spawnSync(cmd, args, { stdio: 'inherit', ...options });
	if (r.error) fail(`${cmd}: ${r.error.message}`);
	return r.status ?? 1;
}

function requireDocker() {
	const r = spawnSync('docker', ['version', '--format', '{{.Server.Version}}'], { encoding: 'utf8' });
	if (r.error) fail('docker is not installed (Docker Desktop: https://www.docker.com/products/docker-desktop/).');
	if (r.status !== 0) fail('Docker is not running – start Docker Desktop and try again.');
}

function containerUp() {
	requireDocker();
	// a running container is left alone: `up` from another checkout (a worktree) would recreate it with that checkout's
	// bind mounts – same data volume, but a container that breaks once the worktree is gone
	const health = spawnSync('docker', ['inspect', '--format', '{{.State.Health.Status}}', 'scutoons-mariadb-1'], { encoding: 'utf8' });
	if (health.status === 0 && health.stdout.trim() === 'healthy') return;
	console.log(`MariaDB container (docker/compose.yml) on 127.0.0.1:${PORTS.mariadb} …`);
	const r = compose(['up', '-d', '--wait', 'mariadb'], { stdio: 'inherit' });
	if (r.status !== 0) fail('docker compose up failed (docker compose -f docker/compose.yml logs mariadb).');
}

/** SQL as root inside the container (no client needed on Windows) */
function rootSql(sql) {
	const r = compose(['exec', '-T', 'mariadb', 'mariadb', '-uroot', `-p${MARIADB.rootPass}`, '-e', sql], { stdio: ['ignore', 'inherit', 'inherit'] });
	if (r.status !== 0) fail('SQL as root failed');
}

/** The databases exist, latin1 by default like production's server: every table must declare utf8mb4 itself */
function createDatabases() {
	rootSql(
		[MARIADB.name, MARIADB.testName]
			.map((name) => `CREATE DATABASE IF NOT EXISTS \`${name}\` CHARACTER SET latin1 COLLATE latin1_swedish_ci; GRANT ALL PRIVILEGES ON \`${name}\`.* TO '${MARIADB.user}'@'%';`)
			.join(' ')
	);
}

function phpConsole(args, stdio = 'inherit') {
	return spawnSync('php', [...phpMysqlArgs(), 'bin/console', ...args], { cwd: SERVER, env: { ...process.env, ...phpDbEnv() }, stdio, encoding: 'utf8' });
}

/** The dev-only sandbox toon from the local character registry (make-sandbox.ts) */
function sandbox() {
	return run('node', ['--experimental-transform-types', '--no-warnings', resolve(ROOT, 'shared/content/scripts/make-sandbox.ts')], { env: { ...process.env, SCUTOONS_DB_NAME: MARIADB.name } }) === 0;
}

/** db:prod-seed; a missing file is not an error (the command prints how to get it) */
function seed(args = []) {
	const status = phpConsole(['db:prod-seed', ...args]).status;
	if (status !== 0 && status !== 2) fail('db:prod-seed failed');
	return status === 0;
}

async function apiAnswers() {
	try {
		return (await fetch(local('api', '/api/health/'), { signal: AbortSignal.timeout(3000) })).status < 500;
	} catch {
		return false;
	}
}

function apiState() {
	try {
		return JSON.parse(readFileSync(API_STATE, 'utf8'));
	} catch {
		return null;
	}
}

/** Is `pid` still server/bin/dev-serve.mjs (and not a process that got its number later)? */
function isDevServe(pid) {
	const r =
		process.platform === 'win32'
			? spawnSync('powershell', ['-NoProfile', '-Command', `(Get-CimInstance Win32_Process -Filter 'ProcessId=${Number(pid)}').CommandLine`], { encoding: 'utf8' })
			: spawnSync('ps', ['-o', 'args=', '-p', String(Number(pid))], { encoding: 'utf8' });
	return /dev-serve\.mjs/.test(r.stdout ?? '');
}

/** Makes the API dev server run on this database: start it, or restart a dev-serve.mjs that runs on another one */
async function apiOn(db) {
	if (await apiAnswers()) {
		const state = apiState();
		if (state?.db === db) return true;
		if (!state?.pid || !isDevServe(state.pid)) {
			console.warn(`The API on :${PORTS.api} was not started by server/bin/dev-serve.mjs – restart it yourself to use ${db}.`);
			return false;
		}
		console.log(`Restarting the API dev server (pid ${state.pid}, on ${state.db ?? 'an older setup'}) …`);
		if (process.platform === 'win32') spawnSync('taskkill', ['/PID', String(state.pid), '/T', '/F'], { stdio: 'ignore' });
		else {
			try {
				process.kill(state.pid, 'SIGTERM');
			} catch {
				/* gone */
			}
		}
		for (let i = 0; i < 40 && (await apiAnswers()); i++) await new Promise((r) => setTimeout(r, 250));
	}
	return run('node', [resolve(ROOT, 'scripts/dev/up.mjs'), 'api']) === 0;
}

async function up(noApi = rest.includes('--no-api')) {
	containerUp();
	createDatabases();
	console.log(`\nMigrations (php bin/console migrate) on ${MARIADB.name} …`);
	if (phpConsole(['migrate']).status !== 0) fail('migrate failed');
	console.log('\nProduction seed (php bin/console db:prod-seed) …');
	if (seed()) {
		console.log('\nSandbox toon (dev only, from the character registry) …');
		if (!sandbox()) console.warn('  the sandbox could not be built (npm run dev:db sandbox to retry)');
	}
	// a promo code to sign up with (npm run dev:test-user, the apps' sign-up form)
	const r = compose(['exec', '-T', 'mariadb', 'mariadb', '--default-character-set=utf8mb4', `-u${MARIADB.user}`, `-p${MARIADB.pass}`, MARIADB.name], {
		input: `INSERT IGNORE INTO qr (code, redirect_to, type, email) VALUES ('${PROMO}', 'https://scutoons.com/catpope', 'promo', '');\n`,
		stdio: ['pipe', 'inherit', 'inherit']
	});
	if (r.status !== 0) fail('the promo code could not be added');
	console.log(`  promo code ${PROMO} (sign-up)`);
	if (noApi) {
		console.log(`\nDone: ${MARIADB.name} on 127.0.0.1:${PORTS.mariadb} (API and test user left alone: --no-api).`);
		return;
	}
	if (!(await apiOn(MARIADB.name))) fail('the API did not start on the database (logs/dev/api.log)');
	console.log('\nTest user (npm run dev:test-user) …');
	if (run('node', [resolve(ROOT, 'scripts/dev/test-user.mjs')]) !== 0) fail('test user failed');
	console.log(`\nDone: API ${local('api')} on ${MARIADB.name} (127.0.0.1:${PORTS.mariadb}). npm run dev starts the rest.`);
}

async function status() {
	const ps = compose(['ps', '--format', '{{.Service}} {{.State}} {{.Status}}'], { stdio: ['ignore', 'pipe', 'pipe'] });
	console.log(`database:   ${MARIADB.name}${process.env.SCUTOONS_DB_NAME ? ' (SCUTOONS_DB_NAME)' : ''} on 127.0.0.1:${PORTS.mariadb}`);
	console.log(`containers: ${ps.status === 0 ? ps.stdout.trim().replace(/\n/g, ' · ') || 'none running' : 'docker not available'}`);
	const state = apiState();
	const running = await apiAnswers();
	console.log(`API ${local('api')}: ${running ? `running on ${state?.db ?? 'unknown (not dev-serve.mjs)'}` : 'down'}`);
	if (running && state?.db && state.db !== MARIADB.name) console.log(`  → it runs on another database: npm run dev:db restarts it on ${MARIADB.name}`);
}

/** assets:sync bunny → local media/ (adds missing files only); a plan unless --apply */
function media() {
	const apply = rest.includes('--apply');
	const prefix = rest.find((a) => a.startsWith('--prefix='));
	const args = ['assets:sync', '--from=bunny', '--to=local', '--list=10', ...(prefix ? [prefix] : []), ...(apply ? ['--apply'] : [])];
	const status = phpConsole(args).status;
	if (!apply && status === 0) console.log('\nA plan only – npm run dev:db -- media --apply copies the missing files (never deletes, never overwrites).');
	process.exit(status ?? 1);
}

switch (command) {
	case 'up':
		await up();
		break;
	case 'seed': {
		containerUp();
		process.exit(seed(rest) ? 0 : 2);
		break;
	}
	case 'sandbox':
		containerUp();
		process.exit(sandbox() ? 0 : 1);
		break;
	case 'media':
		media();
		break;
	case 'status':
		await status();
		break;
	case 'stop':
		requireDocker();
		process.exit(compose(['--profile', 'tools', 'stop'], { stdio: 'inherit' }).status ?? 1);
		break;
	case 'reset':
		containerUp();
		console.log(`Dropping ${MARIADB.name} and ${MARIADB.testName} …`);
		rootSql(`DROP DATABASE IF EXISTS \`${MARIADB.name}\`; DROP DATABASE IF EXISTS \`${MARIADB.testName}\`;`);
		await up();
		break;
	case 'test': {
		containerUp();
		createDatabases();
		const env = { ...process.env, TEST_DB_PORT: String(PORTS.mariadb), TEST_DB_NAME: process.env.TEST_DB_NAME || MARIADB.testName };
		process.exit(run('php', [...phpMysqlArgs(), 'vendor/bin/phpunit', ...rest], { cwd: SERVER, env }));
		break;
	}
	case 'console':
		containerUp();
		process.exit(phpConsole(rest).status ?? 1);
		break;
	case 'sql':
		requireDocker();
		process.exit(compose(['exec', 'mariadb', 'mariadb', '--default-character-set=utf8mb4', `-u${MARIADB.user}`, `-p${MARIADB.pass}`, MARIADB.name], { stdio: 'inherit' }).status ?? 1);
		break;
	case 'adminer':
		requireDocker();
		if (compose(['--profile', 'tools', 'up', '-d', 'adminer'], { stdio: 'inherit' }).status !== 0) fail('docker compose up adminer failed');
		console.log(`Adminer: ${local('adminer')} – server mariadb, user ${MARIADB.user}, password ${MARIADB.pass}, database ${MARIADB.name}`);
		break;
	case 'version':
		requireDocker();
		rootSql('SELECT VERSION(); SELECT @@sql_mode, @@character_set_server, @@collation_server;');
		console.log('Production: run the same two SELECTs in phpMyAdmin (SQL tab) and compare – docs/DATABASE.md.');
		break;
	default:
		fail(`unknown command "${command}" – up | seed | sandbox | media | status | stop | reset | test | console | sql | adminer | version`);
}
