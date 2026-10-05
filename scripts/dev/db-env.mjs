// The local database: the MariaDB container (docker/compose.yml, `npm run dev:db`) – the only one (no SQLite since
// 2026-10-05, CUTOVER 10.30). How PHP and the Node scripts reach it. Shared by scripts/dev/db.mjs, up.mjs,
// test-user.mjs, server/bin/dev-serve.mjs and the check scripts. Docs: server/docs/DATABASE.md "Local MariaDB".
//
// PHP gets QR_DB_HOST/PORT/NAME/USER/PASS (environment, wins over config.local.php) from these scripts.
// SCUTOONS_DB_NAME picks another database in the same container (e.g. a worktree's own: scutoons_wt – its tests then
// use scutoons_wt_test), so parallel checkouts never share data. Local dev values only – never production's.
import { execFileSync, spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const COMPOSE_FILE = resolve(ROOT, 'docker/compose.yml');

const NAME = process.env.SCUTOONS_DB_NAME || 'scutoons';
if (!/^scutoons[a-z0-9_]*$/.test(NAME)) throw new Error(`SCUTOONS_DB_NAME "${NAME}" must start with "scutoons" (letters, digits, _)`);

/** The container's dev credentials (docker/compose.yml); the test database is <name>_test (phpunit.xml.dist) */
export const MARIADB = { host: '127.0.0.1', port: PORTS.mariadb, name: NAME, testName: `${NAME}_test`, user: 'scutoons', pass: 'scutoons', rootPass: 'scutoons-root' };

/** The environment that points the API / console at the local database */
export function phpDbEnv() {
	return {
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

/**
 * SQL on the local dev database through PHP's PDO (no mariadb client needed): each statement with its named
 * parameters (`:name` taken from `params`); returns per statement the rows (SELECT) or the affected row count
 */
export function localSql(statements, params = {}) {
	const php = `$d=new PDO('mysql:host='.getenv('QR_DB_HOST').';port='.getenv('QR_DB_PORT').';dbname='.getenv('QR_DB_NAME').';charset=utf8mb4',getenv('QR_DB_USER'),getenv('QR_DB_PASS'));$d->setAttribute(PDO::ATTR_ERRMODE,PDO::ERRMODE_EXCEPTION);$p=json_decode(getenv('LS_PARAMS'),true);$out=[];foreach(json_decode(getenv('LS_SQL'),true) as $s){$st=$d->prepare($s);$st->execute(array_intersect_key($p,array_flip(array_map(fn($m)=>substr($m,1),(preg_match_all('/:\\w+/',$s,$m)?$m[0]:[])))));$out[]=$st->columnCount()?$st->fetchAll(PDO::FETCH_ASSOC):$st->rowCount();}echo json_encode($out);`;
	const out = execFileSync('php', [...phpMysqlArgs(), '-r', php], {
		env: { ...process.env, ...phpDbEnv(), LS_SQL: JSON.stringify(statements), LS_PARAMS: JSON.stringify(params) },
		encoding: 'utf8'
	});
	return JSON.parse(out);
}

/** The environment of `docker compose` (the ports of shared/dev-ports.json) */
export function composeEnv() {
	return { ...process.env, SCUTOONS_DB_PORT: String(PORTS.mariadb), SCUTOONS_ADMINER_PORT: String(PORTS.adminer) };
}

/** docker compose -f docker/compose.yml <args> */
export function compose(args, options = {}) {
	return spawnSync('docker', ['compose', '-f', COMPOSE_FILE, ...args], { encoding: 'utf8', env: composeEnv(), ...options });
}
