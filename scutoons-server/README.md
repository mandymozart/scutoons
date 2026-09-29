# qr.scutoons.com

QR landing page (`/?c=CODE`), the admin generator (`/generate/`) and the SCUTOONS accounts / items API (`/api/`).
Plain PHP 8 + PDO: MySQL/MariaDB in production, SQLite locally.

## Local dev server

```bash
cp config.local.php.example config.local.php   # set AUTH_SECRET (32+ random chars)
php scripts/migrate.php                          # create/upgrade db/local.sqlite
php -S localhost:9000 router.php
```

`router.php` does locally what `.htaccess` does on Apache (clean `/api/...` paths, denied files).
Login mails are written to `logs/mail.log` (`MAIL_TRANSPORT = log`).

- Landing page: http://localhost:9000/?c=CODE
- API: http://localhost:9000/api/ (see `docs/ACCOUNTS.md`, contract in scutoons `agents/ITEMS.md`)
- Smoke test of the whole API flow: `bash tests/api-smoke.sh` (server running on :9000)

## Scripts (CLI only)

- `scripts/migrate.php` – apply pending `db/migrations/*.{sqlite,mysql}.sql`
- `scripts/backfill-edition-numbers.php` – fill `qr.edition_number` from `generate/files/batch/*.json` (`--sql` for production)
- `scripts/report-orphan-codes.php` – read-only list of edition codes that are in no batch manifest

## Deploy

Push to `main` deploys via FTP (`.github/workflows/main.yml`). Other branches do not deploy.
Database changes are not deployed automatically – see `docs/ACCOUNTS.md` ("Production").
