# Database, migrations, console

MySQL/MariaDB (≥ 10.3, production 10.11) in production, SQLite locally – or the production engine locally in Docker
("Local MariaDB" below). The same SQL runs on both (positional parameters, PHP-side times, `Database::forUpdate()`).

## Migrations

```
migrations/schema/qr.sqlite.sql, qr.mysql.sql     base schema of a fresh database (applied when `qr` is missing)
migrations/sqlite/<date>-<name>.sqlite.sql        dated changes, applied in name order
migrations/mysql/<date>-<name>.mysql.sql          the same for MySQL/MariaDB
migrations/mysql/legacy/                          two MySQL-only changes from before the account system (applied in
                                                  production long ago; never run)
```

`php bin/console migrate` applies the pending files of the configured driver and records each **file name** in
`schema_migrations` – the same names the old `scripts/migrate.php` recorded (the files only moved from
`db/migrations/`), so existing databases re-run nothing. `--status` lists them, `--db=<file>` works on another SQLite
file. `ALTER TABLE … ADD COLUMN` is skipped when the column exists; everything else must be idempotent
(`IF NOT EXISTS`, `INSERT OR IGNORE` / `INSERT IGNORE`). One statement per `;` + line end (triggers on one line).

New migration: add `migrations/sqlite/<YYYY-MM-DD>-<name>.sqlite.sql` **and** `migrations/mysql/<…>.mysql.sql`, keep
`migrations/schema/*` in sync for fresh databases, run `php bin/console migrate` locally and `composer test`, and the
mysql file on MariaDB: `npm run dev:db` (migrates the container) + `npm run dev:db test` (CI does the latter too).

**Production (no shell):** run the pending `*.mysql.sql` files in phpMyAdmin (SQL tab), in date order, after a backup
(Export). Files with triggers need the delimiter `$$` (see the file header). There is no `schema_migrations` bookkeeping
needed there – every file is idempotent.

## Local MariaDB (Docker)

Production runs MariaDB; SQLite forgives things MariaDB does not (reserved words as aliases, InnoDB locking, strict
column lengths, case-insensitive collations …). The container in `docker/compose.yml` gives the local stack and the
tests the production engine. SQLite stays the default – switching is explicit. Needs Docker Desktop. Local dev values
only (user `scutoons` / `scutoons`, root `scutoons-root`, bound to 127.0.0.1); nothing from production, nothing from
the root `.env`.

From the repository root (PowerShell or any shell):

| Command | What |
|---|---|
| `npm run dev:db` | start the container (port `mariadb` in `shared/dev-ports.json`, 13306), wait until healthy, create the databases `scutoons` + `scutoons_test`, `php bin/console migrate`, import `shared/content/seed/*.mysql.sql` with the mariadb client (the phpMyAdmin path: `content-seed`, `characters-seed`, `credits-seed`, then the dated files), add the promo code `devpromo`, **switch the stack to MariaDB**, (re)start the API dev server on it, create the test user (`npm run dev:test-user`). Idempotent – run it again any time |
| `npm run dev` | as before; with MariaDB selected it starts the container first and prints `database: MariaDB container` |
| `npm run dev:db sqlite` | switch back to SQLite (`config.local.php`, `db/local.sqlite`); restarts a running API dev server |
| `npm run dev:db status` | selected database, container state, what the running API uses |
| `npm run dev:db stop` | stop the container – the data stays in the volume `scutoons_mariadb` |
| `npm run dev:db reset` | delete the container **and its data**, then `up` again (fresh database) |
| `npm run dev:db test` | the PHPUnit suite on MariaDB (see below); with PHPUnit options after `--`: `npm run dev:db -- test --filter Wardrobe` |
| `npm run dev:db -- console <args>` | `php bin/console` on the MariaDB dev database, e.g. `npm run dev:db -- console migrate --status` (the `--` keeps npm from taking the options) |
| `npm run dev:db sql` | a `mariadb` shell in the container (database `scutoons`) |
| `npm run dev:db adminer` | Adminer on http://localhost:13380 (server `mariadb`, user `scutoons`, password `scutoons`) |
| `npm run dev:db version` | `VERSION()`, `sql_mode`, charset, collation of the container |

**How the switch works:** `logs/dev/db.json` (gitignored) holds the choice; `SCUTOONS_DB=sqlite|mariadb` in the
environment wins over it. PHP never reads the file: `server/bin/dev-serve.mjs` (`npm run dev:api`, `composer serve`),
`npm run dev:db` and `npm run dev:test-user` pass `QR_DB_DRIVER=mysql` + `QR_DB_HOST/PORT/NAME/USER/PASS` to PHP (an
environment variable wins over `config.local.php`; `DB_DRIVER=mysql` wins over `DB_SQLITE_PATH` – CONFIGURATION.md),
and `-d extension=pdo_mysql` when `php.ini` does not load it (PHP for Windows ships `php_pdo_mysql.dll`, the line is
commented out by default; enabling it in `php.ini` once is fine too). A `php bin/console …` typed by hand still uses
SQLite – use `npm run dev:db -- console …`. The contract suites (`composer test:contract`) stay on a SQLite copy.

**Tests on MariaDB:** `npm run dev:db test` (root), or in `server/`:
`php -d extension=pdo_mysql vendor/bin/phpunit -c phpunit.mysql.xml`. `phpunit.mysql.xml` sets `TEST_DB_DRIVER=mysql`
and the container's `scutoons_test` (real environment variables win, e.g. `TEST_DB_PORT`); the suite drops every table
of that database, migrates it with `migrations/mysql` once per run and restores that state before each test
(`tests/Unit/MysqlTestDatabase.php`). A few tests run only there (InnoDB locking, the mysql ↔ sqlite schema check) and
are skipped on SQLite. CI runs both (`.github/workflows/server.yml`, jobs `test` and `test-mariadb`; the latter starts
the same `docker/compose.yml`).

**Matching production:** run in phpMyAdmin (SQL tab) on the production database:

```sql
SELECT VERSION();
SELECT @@sql_mode, @@character_set_server, @@collation_server;
```

and set what it shows as environment variables before `npm run dev:db reset` (or change the defaults in
`docker/compose.yml` and commit them): `MARIADB_VERSION` (image tag, e.g. `10.11` or `10.11.9`; default `10.11`),
`MARIADB_SQL_MODE` (default `STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_AUTO_CREATE_USER,NO_ENGINE_SUBSTITUTION`,
MariaDB 10.11's own default), `MARIADB_CHARSET` / `MARIADB_COLLATION` (default `utf8mb4` / `utf8mb4_general_ci`, what the
production `qr` table uses). `npm run dev:db version` shows the container's values for comparison.

## Console (`php bin/console …`, local or on a host with a shell)

| Command | What |
|---|---|
| `migrate [--status] [--db=<sqlite>]` | apply pending migrations |
| `roles grant\|revoke <email> <role>` · `roles list [email]` | account roles (member, admin, creator); `granted_by` = `cli` |
| `import-content <folder> [--toon=] [--db=] [--update] [--dry-run] [--sql] [--only=<kinds>]` | the interchange files (`shared/content/seed`: `toons.json`, `events.json`, `<toon>/<l>/…`, `<toon>/_/…`, `scutoons/_/…`) into `entity_versions` as published versions, keyed by entity (`Console\ContentFiles`; idempotent, `--update` for changed files). `--sql` prints guarded MySQL INSERTs for phpMyAdmin (the production seed, committed as `shared/content/seed/content-seed.mysql.sql`) |
| `export-content <dir> [--toon=] [--draft] [--db=]` | the store back into the file layout (`toons.json`, `events.json`, `<toon>/<locale>/<path>`): backup, diff, seed for another environment (agents/DATA.md) |
| `backfill-edition-numbers [--dry-run] [--sql] [--force] [--manifests=<dir>]` | `qr.edition_number` from the batch manifests (`--sql` for production) |
| `report-orphan-codes [--csv] [--manifests=<dir>]` | read-only: edition codes in no manifest |

Production equivalents without a shell: roles → admin app / `generate/` "Users"; content seed → `import-content --sql`
locally (or the committed `shared/content/seed/content-seed.mysql.sql`), import the file in phpMyAdmin after `2026-09-30-entity-keys`; edition numbers → copy `storage/files/batch/*.json` from the server, run
`backfill-edition-numbers --sql --manifests=<copy>` locally, import the output.

## Production data steps still open from the account system (docs/DEPLOY.md "Cutover")

In phpMyAdmin, in this order (all idempotent): `2026-09-29-accounts-inventory.mysql.sql`,
`2026-09-30-code-owner-guard.mysql.sql` (delimiter `$$`), `2026-09-30-revoke-events.mysql.sql`,
`2026-09-30-roles-signup.mysql.sql`, `2026-09-30-content-versions.mysql.sql`, `2026-09-30-settings.mysql.sql`,
`2026-09-30-sensitive-content.mysql.sql` (`users.hide_sensitive`, default 1), `2026-10-01-media-quality.mysql.sql`
(`users.media_quality`, default `auto` – with the `media-quality` branch, after the release), `2026-10-02-mail-log.mysql.sql`
(the sent-mail log of the email module; without it mails are still sent, just not logged), `2026-10-02-ownership.mysql.sql`
(ownership claims, claim files, transfer history – agents/ITEMS.md "Ownership, claims, gifting"; until it ran gifts and
claims answer 503, everything else works), `2026-10-02-assets.mysql.sql` (the media asset registry: assets, versions, renditions, usages, jobs –
agents/ASSETS.md; without it only the asset manager answers 503), `2026-10-02-ownership-signup.mysql.sql` (`login_requests.dispute`: signing up
to file a claim); then the content seed and the
edition-number backfill above; review `report-orphan-codes` against a copy of the production DB (nothing is deleted
automatically).
