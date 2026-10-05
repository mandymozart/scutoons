# Database, migrations, console

MariaDB everywhere: production (10.11 at all-inkl) and locally the same engine in Docker ("Local MariaDB" below). No
SQLite since 2026-10-05 (CUTOVER 10.30). Queries use positional parameters and PHP-side times; row locks are
`SELECT … FOR UPDATE` inside `Database::transaction()`.

## Migrations

```
migrations/schema/qr.mysql.sql                    base schema of a fresh database (applied when `qr` is missing)
migrations/mysql/<date>-<name>.mysql.sql          dated changes, applied in name order
migrations/mysql/legacy/                          two changes from before the account system (never run;
                                                  qr.template_data never reached production – 2026-10-05 – and is
                                                  not in the base schema)
```

`php bin/console migrate` applies the pending files and records each **file name** in `schema_migrations` – the same
names the old `scripts/migrate.php` recorded, so existing databases re-run nothing. `--status` lists them.
`ALTER TABLE … ADD COLUMN` is skipped when the column exists; everything else must be idempotent (`IF NOT EXISTS`,
`INSERT IGNORE`). One statement per `;` + line end (triggers on one line).

New migration: add `migrations/mysql/<YYYY-MM-DD>-<name>.mysql.sql`, keep `migrations/schema/qr.mysql.sql` in sync for
fresh databases, run `npm run dev:db` (migrates the local database) and `npm run dev:db test` (CI runs the same suite).

**Production (no shell):** run the pending `*.mysql.sql` files in phpMyAdmin (SQL tab, database `d0487143` selected),
in date order, after a backup (Export). Files with triggers need the delimiter `$$` (see the file header). There is no
`schema_migrations` bookkeeping needed there – every file is idempotent.

## Local MariaDB (Docker)

The container in `docker/compose.yml` is the local database of the stack and the tests: production's engine and
settings. Needs Docker Desktop. Local dev values only (user `scutoons` / `scutoons`, root `scutoons-root`, bound to
127.0.0.1). A fresh, complete local setup is one command: `npm run dev:db`.

From the repository root (PowerShell or any shell):

| Command | What |
|---|---|
| `npm run dev:db` | start the container (port `mariadb` in `shared/dev-ports.json`, 13306; a running one is left alone), create the databases `scutoons` + `scutoons_test`, `php bin/console migrate`, load the **production seed** (below; skipped when unchanged), rebuild the dev-only sandbox toon from the character registry (`shared/content/scripts/make-sandbox.ts` → `logs/dev/sandbox`), add the promo code `devpromo`, (re)start the API dev server on it, create the test user (`npm run dev:test-user`). Idempotent – run it any time. `-- up --no-api` leaves the API and the test user alone |
| `npm run dev` | starts the container first, then the apps |
| `npm run dev:db seed` | load `server/db/prod-seed.sql` again (`-- seed --force`: even when unchanged; `-- seed --from=<dump.sql>`: extract it from a phpMyAdmin export first) |
| `npm run dev:db sandbox` | rebuild + import the sandbox toon alone |
| `npm run dev:db media` | the media files missing in the local `media/` folder, from bunny.net (production's storage; needs the `BUNNY_*` keys of the repository-root `.env`): a plan; `-- media --apply` copies them, `-- media --prefix=catpope/` one folder. Never deletes, never overwrites (a file with another size stays – it may be local work) |
| `npm run dev:db status` | the database, container state, what the running API uses |
| `npm run dev:db stop` | stop the container – the data stays in the volume `scutoons_mariadb` |
| `npm run dev:db reset` | drop this database and its test database, then `up` again (fresh from the seed) |
| `npm run dev:db test` | the PHPUnit suite (see below); with PHPUnit options after `--`: `npm run dev:db -- test --filter Wardrobe` |
| `npm run dev:db -- console <args>` | `php bin/console` on the dev database, e.g. `npm run dev:db -- console migrate --status` (the `--` keeps npm from taking the options) |
| `npm run dev:db sql` | a `mariadb` shell in the container |
| `npm run dev:db adminer` | Adminer on http://localhost:13380 (server `mariadb`, user `scutoons`, password `scutoons`) |
| `npm run dev:db version` | `VERSION()`, `sql_mode`, charset, collation of the container |

**How PHP finds it:** `server/bin/dev-serve.mjs` (`npm run dev:api`, `composer serve`), `npm run dev:db`, the test user
and the check scripts pass `QR_DB_HOST/PORT/NAME/USER/PASS` (an environment variable wins over `config.local.php`) and
`-d extension=pdo_mysql` when `php.ini` does not load it (PHP for Windows ships `php_pdo_mysql.dll`, the line is
commented out by default; enabling it in `php.ini` once is fine too). `config.local.php.example` carries the same values,
so a plain `php bin/console …` works as well. Node scripts reach the database with `localSql()` of
`scripts/dev/db-env.mjs`.

**Another database (a worktree):** `SCUTOONS_DB_NAME=scutoons_<x>` makes every `npm run dev:db …` work on that database
of the same container (its tests on `scutoons_<x>_test`) – parallel checkouts never share data. Combine with
`-- up --no-api` when the other checkout's API dev server should keep running.

**Tests:** `npm run dev:db test` (root), or in `server/`: `php -d extension=pdo_mysql vendor/bin/phpunit`.
`phpunit.xml.dist` points at the container's `scutoons_test` (`TEST_DB_HOST/PORT/NAME/USER/PASS`; real environment
variables win); the suite drops every table of that database, migrates it once per run and restores that state before
each test (`tests/Unit/MysqlTestDatabase.php`; it refuses a database whose name does not end in `test`). The migrated
test database holds the 2026-10-04 catalog snapshot of `2026-10-04-catalog` = the committed fixture
`tests/fixtures/catalog`; the TS tests of `@scutoons/content` read that fixture and `tests/fixtures/content`. CI
(`.github/workflows/server.yml`) starts the same `docker/compose.yml`.

## Production seed (CUTOVER 10.30)

Production is the source of truth for content and catalog (owner 2026-10-05); the repository carries no content or
catalog data. A local database gets them from a phpMyAdmin export:

1. phpMyAdmin → database **d0487143** → Export → Custom → format SQL, **data only**, character set utf8mb4, tables
   `entity_versions`, `toons`, `catalog_meta`, `catalog_items`, `catalog_recipes`, `catalog_albums`,
   `catalog_editions`, `catalog_products`, `catalog_featured`, `catalog_permissions`, `catalog_code_types`, `assets`,
   `asset_versions`, `asset_renditions`, `asset_usages` – nothing else (no users, qr, sessions, inventory, progress,
   ownership, mail, embassy, agent, settings, catalog_history).
2. Save it as `server/db/prod-seed.sql` (**gitignored**: `entity_versions.author` names creators). A full export is fine
   too: `npm run dev:db -- seed --from=<dump.sql>` writes `server/db/prod-seed.sql` with only those tables' INSERTs.
3. `npm run dev:db`.

`php bin/console db:prod-seed [--from=<dump>] [--file=] [--force]` (`Infrastructure\Database\ProdSeed`): in one
transaction it empties those 15 tables – the migrations' own starter rows (e.g. the `2026-10-04-catalog` snapshot) go
too – and runs the file's INSERTs; other statements and tables of the file are ignored. The file's SHA-256 is kept in
the setting `dev_prod_seed`: the same file again is a no-op (`--force` reloads it; local edits of content, catalog and
the asset registry are then replaced). Without the file the command explains how to get it and the database keeps the
migrations' starter rows. Never run it against production.

**Matching production:** the container's defaults are production's (phpMyAdmin, 2026-10-03:
`10.11.14-MariaDB-0ubuntu0.24.04.1-log`, `@@sql_mode = NO_ENGINE_SUBSTITUTION`, `@@character_set_server = latin1`,
`@@collation_server = latin1_swedish_ci`): `MARIADB_VERSION` 10.11.14, `MARIADB_SQL_MODE` `NO_ENGINE_SUBSTITUTION`,
`MARIADB_CHARSET` / `MARIADB_COLLATION` latin1 / latin1_swedish_ci (override them as environment variables, then
`npm run dev:db reset`). The local databases get the default latin1 too, so a table that does not declare its charset
would end up latin1 like in production: every `CREATE TABLE` in `migrations/mysql/` declares
`DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci` (`ADD COLUMN`s inherit it), and
`InfrastructureTest::testEveryTextColumnIsUtf8mb4` checks every column on MariaDB. To re-read production:

```sql
SELECT VERSION();
SELECT @@sql_mode, @@character_set_server, @@collation_server;
-- the database default and every text column's charset + collation (expect utf8mb4 everywhere;
-- utf8mb4_bin for the code columns after 2026-10-03-code-case, utf8mb4_general_ci for the rest)
SELECT DEFAULT_CHARACTER_SET_NAME, DEFAULT_COLLATION_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = DATABASE();
SELECT TABLE_NAME, COLUMN_NAME, CHARACTER_SET_NAME, COLLATION_NAME FROM information_schema.COLUMNS
WHERE TABLE_SCHEMA = DATABASE() AND CHARACTER_SET_NAME IS NOT NULL ORDER BY TABLE_NAME, ORDINAL_POSITION;
```

**Session sql_mode (`DB_SQL_MODE`):** production's server mode is not strict, so MariaDB would silently cut a too long
value (a 9-character code into `VARCHAR(8)`) or coerce a bad one. The API therefore sets its own session mode on every
connection: `DB_SQL_MODE`, default `STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION` – such a
write fails instead, the same on every host. `DB_SQL_MODE=server` keeps the server's mode. The tests run with the
default; `TEST_DB_SQL_MODE=server npm run dev:db test` runs them with the server's (non-strict) mode. Migrations in
phpMyAdmin run under the server's mode (they only create / alter tables).

**Codes are case-sensitive** (owner 2026-10-03): every column holding a code is `utf8mb4_bin` (migration
`2026-10-03-code-case`) and every lookup is an exact `code = ?` – there is no
case-insensitive lookup anywhere (also not for certificates / the verify page). Only the admin lists' search box
(`q`) ignores case (`LOWER(code) LIKE`).

## Console (`php bin/console …`, local or on a host with a shell)

| Command | What |
|---|---|
| `migrate [--status]` | apply pending migrations |
| `db:prod-seed [--from=<dump>] [--force]` | production's content + catalog into the local database ("Production seed") |
| `roles grant\|revoke <email> <role>` · `roles list [email]` | account roles (member, admin, creator); `granted_by` = `cli` |
| `import-content <folder> [--toon=] [--update] [--dry-run] [--sql] [--only=<kinds>]` | interchange files (`toons.json`, `events.json`, `<toon>/<l>/…`, `<toon>/_/…`, `scutoons/_/…` – an `export-content` folder, the sandbox) into `entity_versions` as published versions, keyed by entity (`Console\ContentFiles`; idempotent, `--update` for changed files). `--sql` prints guarded INSERTs for phpMyAdmin |
| `export-content <dir> [--toon=] [--draft]` | the store back into the file layout (`toons.json`, `events.json`, `<toon>/<locale>/<path>`): backup, diff (agents/DATA.md) |
| `backfill-edition-numbers [--dry-run] [--sql] [--force] [--manifests=<dir>]` | `qr.edition_number` from the batch manifests (`--sql` for production) |
| `report-orphan-codes [--csv] [--manifests=<dir>]` | read-only: edition codes in no manifest |

Production equivalents without a shell: roles → admin app / `generate/` "Users"; content → the creator (production is the
source); edition numbers → copy `storage/files/batch/*.json` from the server, run
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
to file a claim), `2026-10-03-code-case.mysql.sql` (codes case-sensitive: the code columns → `utf8mb4_bin`; safe before
or after the deploy of the matching API), `2026-10-04-catalog.mysql.sql` (the catalog moves from server/config/*.json into
tables with today's data, history; folds and removes the settings `item_respawn` / `featured_products` – agents/CATALOG.md
"Status"; ran 2026-10-05 – the API needs the tables since 10.30),
`2026-10-04-agent-chats.mysql.sql` (the creator's Ask panel chat history; without it the panel works but keeps no history –
`/api/agent-chats` answers 503), `2026-10-04-agent-usage.mysql.sql` (the Ask panel's usage log – one row per AI call,
the admin's overview per creator and service; without it the Ask service keeps logging to its own file),
`2026-10-04-ask-embassy-mail.mysql.sql` (after those two and the Embassy ones: `app` on agent chats / usage, the mail
fields of `embassy_activities`, `embassy_mail_state` for the inbox sync; without it chats and usage are the creator's
only, Embassy mails are sent and logged without subject / addresses, the inbox sync answers 503); then the edition-number backfill
above; review `report-orphan-codes` against a copy of the production DB (nothing is deleted
automatically).
