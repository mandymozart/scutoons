#!/usr/bin/env node
// The local MariaDB container (docker/compose.yml) – the production engine for development and tests.
// Docs: server/docs/DATABASE.md "Local MariaDB (Docker)". Needs Docker Desktop; PHP's pdo_mysql is added on the fly.
//
//   npm run dev:db                 = up: start the container (wait until healthy), create the databases, migrate,
//                                    import the content seed (shared/content/seed/*.mysql.sql, as in phpMyAdmin), a
//                                    local promo code, switch the stack to MariaDB, (re)start the API on it, test user
//   npm run dev:db sqlite          switch the stack back to SQLite (restarts an API this script can restart)
//   npm run dev:db status          container, selected database, what the API runs on
//   npm run dev:db stop            stop the container (data stays in the volume; `npm run dev` starts it again)
//   npm run dev:db reset           delete the container AND its data, then `up` again (fresh database)
//   npm run dev:db test [args]     the PHPUnit suite on MariaDB (server/phpunit.mysql.xml, database scutoons_test)
//   npm run dev:db console <args>  php bin/console on the MariaDB dev database, e.g. console migrate --status
//   npm run dev:db sql             a mariadb shell in the container (database scutoons)
//   npm run dev:db adminer         Adminer (web UI) on the "adminer" dev port
//   npm run dev:db version         server version, sql_mode, charset, collation (compare with production)
//
// The switch (logs/dev/db.json) is what `npm run dev`, the API dev server and `npm run dev:test-user` follow;
// SCUTOONS_DB=sqlite|mariadb in the environment wins over it. Local dev credentials only (docker/compose.yml).
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { compose, MARIADB, phpDbEnv, phpMysqlArgs, ROOT, selectDb, selectedDb } from './db-env.mjs';
import { local, PORTS } from './ports.mjs';

const SERVER = resolve(ROOT, 'server');
const SEED = resolve(ROOT, 'shared/content/seed');
const API_STATE = resolve(ROOT, `logs/dev/api-server-${PORTS.api}.json`);
const PROMO = 'devpromo'; // qr.code is VARCHAR(8) on MariaDB
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
	console.log(`MariaDB container (docker/compose.yml) on 127.0.0.1:${PORTS.mariadb} …`);
	const r = compose(['up', '-d', '--wait', 'mariadb'], { stdio: 'inherit' });
	if (r.status !== 0) fail('docker compose up failed (docker compose -f docker/compose.yml logs mariadb).');
}

/** SQL as root inside the container (no client needed on Windows) */
function rootSql(sql) {
	const r = compose(['exec', '-T', 'mariadb', 'mariadb', '-uroot', `-p${MARIADB.rootPass}`, '-e', sql], { stdio: ['ignore', 'inherit', 'inherit'] });
	if (r.status !== 0) fail('SQL as root failed');
}

/** SQL (a file's text) into the dev database through the mariadb client – the same path as phpMyAdmin's import */
function importSql(sql, label) {
	const r = compose(['exec', '-T', 'mariadb', 'mariadb', '--default-character-set=utf8mb4', `-u${MARIADB.user}`, `-p${MARIADB.pass}`, MARIADB.name], {
		input: sql,
		stdio: ['pipe', 'inherit', 'inherit']
	});
	if (r.status !== 0) fail(`import of ${label} failed`);
}

function phpConsole(args, stdio = 'inherit') {
	return spawnSync('php', [...phpMysqlArgs(), 'bin/console', ...args], { cwd: SERVER, env: { ...process.env, ...phpDbEnv('mariadb') }, stdio, encoding: 'utf8' });
}

function seedFiles() {
	const first = ['content-seed.mysql.sql', 'characters-seed.mysql.sql', 'credits-seed.mysql.sql'];
	const dated = readdirSync(SEED).filter((f) => f.endsWith('.mysql.sql') && !first.includes(f)).sort();
	return [...first.filter((f) => existsSync(resolve(SEED, f))), ...dated];
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

/** Makes the API dev server run on `db`: start it, or restart a dev-serve.mjs that runs on the other database */
async function apiOn(db) {
	if (await apiAnswers()) {
		const state = apiState();
		if (state?.db === db || (!state?.db && db === 'sqlite')) return true;
		if (!state?.pid) {
			console.warn(`The API on :${PORTS.api} was not started by server/bin/dev-serve.mjs – restart it yourself to use ${db}.`);
			return false;
		}
		if (!isDevServe(state.pid)) {
			console.warn(`The API on :${PORTS.api} is not the dev server of logs/dev/api-server-${PORTS.api}.json (pid ${state.pid}) – restart it yourself to use ${db}.`);
			return false;
		}
		console.log(`Restarting the API dev server (pid ${state.pid}, on ${state.db ?? 'sqlite'}) …`);
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

async function up() {
	containerUp();
	rootSql(
		[MARIADB.name, MARIADB.testName]
			.map((name) => `CREATE DATABASE IF NOT EXISTS \`${name}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci; GRANT ALL PRIVILEGES ON \`${name}\`.* TO '${MARIADB.user}'@'%';`)
			.join(' ')
	);
	console.log('\nMigrations (php bin/console migrate) …');
	if (phpConsole(['migrate']).status !== 0) fail('migrate failed');
	console.log('\nContent seed (shared/content/seed/*.mysql.sql, as imported in phpMyAdmin) …');
	for (const file of seedFiles()) {
		importSql(readFileSync(resolve(SEED, file)), file);
		console.log(`  imported ${file}`);
	}
	// a promo code to sign up with (npm run dev:test-user, the apps' sign-up form); the SQLite dev DB has its own
	importSql(`INSERT IGNORE INTO qr (code, redirect_to, type, email) VALUES ('${PROMO}', 'https://scutoons.com/catpope', 'promo', '');\n`, 'the promo code');
	console.log(`  promo code ${PROMO} (sign-up)`);
	selectDb('mariadb');
	console.log('\nThe local stack now uses MariaDB (npm run dev:db sqlite switches back).');
	if (!(await apiOn('mariadb'))) fail('the API did not start on MariaDB (logs/dev/api.log)');
	console.log('\nTest user (npm run dev:test-user) …');
	if (run('node', [resolve(ROOT, 'scripts/dev/test-user.mjs')]) !== 0) fail('test user failed');
	console.log(`\nDone: API ${local('api')} on MariaDB 127.0.0.1:${PORTS.mariadb}/${MARIADB.name}. npm run dev starts the rest.`);
}

async function status() {
	const db = selectedDb();
	const ps = compose(['ps', '--format', '{{.Service}} {{.State}} {{.Status}}'], { stdio: ['ignore', 'pipe', 'pipe'] });
	console.log(`selected database: ${db}${process.env.SCUTOONS_DB ? ' (SCUTOONS_DB)' : ''}`);
	console.log(`containers:        ${ps.status === 0 ? ps.stdout.trim().replace(/\n/g, ' · ') || 'none running' : 'docker not available'}`);
	const state = apiState();
	const running = await apiAnswers();
	console.log(`API ${local('api')}: ${running ? `running on ${state?.db ?? 'unknown (not dev-serve.mjs)'}` : 'down'}`);
	if (running && state?.db && state.db !== db) console.log('  → it runs on the other database: npm run dev:db (or npm run dev:db sqlite) restarts it');
}

switch (command) {
	case 'up':
		await up();
		break;
	case 'sqlite':
		selectDb('sqlite');
		console.log('The local stack uses SQLite again (server/db/local.sqlite via config.local.php). The container keeps running: npm run dev:db stop.');
		if (await apiAnswers()) await apiOn('sqlite');
		break;
	case 'status':
		await status();
		break;
	case 'stop':
		requireDocker();
		process.exit(compose(['--profile', 'tools', 'stop'], { stdio: 'inherit' }).status ?? 1);
		break;
	case 'reset':
		requireDocker();
		console.log('Removing the container and its data (volume scutoons_mariadb) …');
		if (compose(['--profile', 'tools', 'down', '-v'], { stdio: 'inherit' }).status !== 0) fail('docker compose down failed');
		await up();
		break;
	case 'test': {
		containerUp();
		rootSql(`CREATE DATABASE IF NOT EXISTS \`${MARIADB.testName}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci; GRANT ALL PRIVILEGES ON \`${MARIADB.testName}\`.* TO '${MARIADB.user}'@'%';`);
		const env = { ...process.env, TEST_DB_PORT: String(PORTS.mariadb) };
		process.exit(run('php', [...phpMysqlArgs(), 'vendor/bin/phpunit', '-c', 'phpunit.mysql.xml', ...rest], { cwd: SERVER, env }));
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
		fail(`unknown command "${command}" – up | sqlite | status | stop | reset | test | console | sql | adminer | version`);
}
