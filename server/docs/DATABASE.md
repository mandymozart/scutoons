# Database, migrations, console

MySQL/MariaDB (≥ 10.3, production 10.11) in production, SQLite locally. The same SQL runs on both (positional
parameters, PHP-side times, `Database::forUpdate()`).

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
`migrations/schema/*` in sync for fresh databases, run `php bin/console migrate` locally and `composer test`.

**Production (no shell):** run the pending `*.mysql.sql` files in phpMyAdmin (SQL tab), in date order, after a backup
(Export). Files with triggers need the delimiter `$$` (see the file header). There is no `schema_migrations` bookkeeping
needed there – every file is idempotent.

## Console (`php bin/console …`, local or on a host with a shell)

| Command | What |
|---|---|
| `migrate [--status] [--db=<sqlite>]` | apply pending migrations |
| `roles grant\|revoke <email> <role>` · `roles list [email]` | account roles (member, admin, creator); `granted_by` = `cli` |
| `import-content <viewer/static/api> [--toon=] [--db=] [--update] [--dry-run] [--sql]` | seed the content store from the viewer's static configs as published versions (idempotent). `--sql` prints guarded MySQL INSERTs for phpMyAdmin (the production seed) |
| `backfill-edition-numbers [--dry-run] [--sql] [--force] [--manifests=<dir>]` | `qr.edition_number` from the batch manifests (`--sql` for production) |
| `report-orphan-codes [--csv] [--manifests=<dir>]` | read-only: edition codes in no manifest |

Production equivalents without a shell: roles → admin app / `generate/` "Users"; content seed → `import-content --sql`
locally, import the file in phpMyAdmin; edition numbers → copy `storage/files/batch/*.json` from the server, run
`backfill-edition-numbers --sql --manifests=<copy>` locally, import the output.

## Production data steps still open from the account system (docs/DEPLOY.md "Cutover")

In phpMyAdmin, in this order (all idempotent): `2026-09-29-accounts-inventory.mysql.sql`,
`2026-09-30-code-owner-guard.mysql.sql` (delimiter `$$`), `2026-09-30-revoke-events.mysql.sql`,
`2026-09-30-roles-signup.mysql.sql`, `2026-09-30-content-versions.mysql.sql`, `2026-09-30-settings.mysql.sql`; then the content seed and the
edition-number backfill above; review `report-orphan-codes` against a copy of the production DB (nothing is deleted
automatically).
