// Which database the local stack uses – SQLite (server/db/local.sqlite, the default) or the MariaDB container
// (docker/compose.yml, `npm run dev:db`) – and how PHP is pointed at it. Shared by scripts/dev/db.mjs, up.mjs,
// test-user.mjs and server/bin/dev-serve.mjs. Docs: server/docs/DATABASE.md "Local MariaDB (Docker)".
//
// The switch is explicit: `npm run dev:db` selects MariaDB, `npm run dev:db sqlite` goes back. It is stored in
// logs/dev/db.json (gitignored); the environment variable SCUTOONS_DB=sqlite|mariadb wins over the file.
// PHP never reads the file: the Node scripts pass QR_DB_DRIVER=mysql + QR_DB_* (environment, wins over
// config.local.php) to the API workers and the console. Local dev values only – never production's.
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const SWITCH_FILE = resolve(ROOT, 'logs/dev/db.json');
export const COMPOSE_FILE = resolve(ROOT, 'docker/compose.yml');

/** The container's dev credentials (docker/compose.yml); the test database is scutoons_test (phpunit.mysql.xml) */
export const MARIADB = { host: '127.0.0.1', port: PORTS.mariadb, name: 'scutoons', testName: 'scutoons_test', user: 'scutoons', pass: 'scutoons', rootPass: 'scutoons-root' };

/** 'mariadb' | 'sqlite' */
export function selectedDb() {
	const env = (process.env.SCUTOONS_DB ?? '').toLowerCase();
	if (env === 'mariadb' || env === 'mysql') return 'mariadb';
	if (env === 'sqlite') return 'sqlite';
	try {
		return JSON.parse(readFileSync(SWITCH_FILE, 'utf8')).db === 'mariadb' ? 'mariadb' : 'sqlite';
	} catch {
		return 'sqlite';
	}
}

export function selectDb(db) {
	mkdirSync(dirname(SWITCH_FILE), { recursive: true });
	writeFileSync(SWITCH_FILE, JSON.stringify({ db, changed: new Date().toISOString() }, null, '\t') + '\n');
}

/** The environment that points the API / console at the selected database (empty for SQLite = config.local.php) */
export function phpDbEnv(db = selectedDb()) {
	if (db !== 'mariadb') return {};
	return {
		QR_DB_DRIVER: 'mysql',
		QR_DB_HOST: MARIADB.host,
		QR_DB_PORT: String(MARIADB.port),
		QR_DB_NAME: MARIADB.name,
		QR_DB_USER: MARIADB.user,
		QR_DB_PASS: MARIADB.pass
	};
}

let pdoMysqlLoaded = null;
/** `-d extension=pdo_mysql` when the php.ini does not load it (PHP for Windows ships the DLL, commented out) */
export function phpMysqlArgs(php = process.env.PHP_BINARY || 'php') {
	if (pdoMysqlLoaded === null) {
		pdoMysqlLoaded = spawnSync(php, ['-r', "echo extension_loaded('pdo_mysql') ? 1 : 0;"], { encoding: 'utf8' }).stdout?.trim() === '1';
	}
	return pdoMysqlLoaded ? [] : ['-d', 'extension=pdo_mysql'];
}

/** The environment of `docker compose` (the ports of shared/dev-ports.json) */
export function composeEnv() {
	return { ...process.env, SCUTOONS_DB_PORT: String(PORTS.mariadb), SCUTOONS_ADMINER_PORT: String(PORTS.adminer) };
}

/** docker compose -f docker/compose.yml <args> */
export function compose(args, options = {}) {
	return spawnSync('docker', ['compose', '-f', COMPOSE_FILE, ...args], { encoding: 'utf8', env: composeEnv(), ...options });
}
