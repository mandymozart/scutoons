# Deploy and cutover

## How it deploys

The API is the target **`api`** of [`deploy/targets.json`](../../deploy/targets.json) (docs: [deploy/README.md](../../deploy/README.md)).
`.github/workflows/server.yml` (monorepo root), path filter `server/**` + `deploy/**`:

1. **test** (every push / pull request touching the server): PHP 8.3, `composer install`, PHPUnit, PHPStan.
2. **deploy** (only `main`, after the tests; `.github/workflows/deploy-app.yml`, environment `production`):
   `deploy/deploy.mjs api` copies the server folder into a staging folder, runs
   `composer install --no-dev --optimize-autoloader` there (the host has no shell, so `vendor/` is uploaded), writes
   `config.php` from the environment's secrets/variables ([CONFIGURATION.md](CONFIGURATION.md), list: root
   `.env.example`) and `version.json` (git sha for `/api/health/`), then FTP-syncs it to **`api/`** on the host.
   Never uploaded and never deleted on the host: `storage/`, `db/`, `logs/`, `config.local.php`, `.htpasswd`.
   Not uploaded: `tests/`, `tools/`, `docs/`, `bin/`, `migrations/`, dev tool configs, README/CHANGELOG.
   Uploaded: `public/`, `src/`, `config/` (+ the generated `config.php`), `templates/` (mails), `vendor/`,
   `composer.*`, the root `.htaccess`, `version.json`.
3. `deploy-all.yml` deploys api → qr → frontends in one run (manual or tag `deploy-*`).

Database changes are never deployed automatically – see [DATABASE.md](DATABASE.md); every deploy run shows a warning.

## Folder and document root

`api.scutoons.com` → **`api/public/`** (recommended; private files physically outside the web root). If the target
directory has to be `api/`, the root `.htaccess` forwards every request into `public/` and denies `src/`, `config/`,
`vendor/`, `migrations/`, `bin/`, `tests/`, `db/`, `logs/`, `storage/`, `templates/`, `docs/`.

This server is **the API only** (+ `/api/health/`, and the legacy `generate/` SPA until the admin app retires it).
`qr.scutoons.com` is the separate qr app (`qr/` → host folder `qr/`, docroot `qr/public/`): it renders the landing
page and the verify page and proxies `/api/validate/`, `/api/keys/keys.php`, `generate/get-signature.php` +
`generate/files/signatures/*` + `/api/certificates/*` (verify page); `PROXY_FULL_API` (whole `/api/*`) is off since 2026-10-05. Both share
`QR_PROXY_SECRET` (the API trusts the client IP / device id the qr app forwards only with it).

## Cutover

The step-by-step cutover of the whole platform (folders, subdomain targets, SSL, secrets, order, checks, rollback)
is in `agents/PLATFORM.md` → "Cutover checklist (all-inkl)". Server-specific data moves in it:

- keypair: old `api/keys/keypair.json` → `api/storage/keys/keypair.json` (upload **before** the first API deploy,
  so `/api/keys/keys.php` never answers 404);
- batch output: old `generate/files/` → `api/storage/files/`;
- `generate/.htpasswd` (Basic auth of the legacy SPA) → next to `api/public/generate/.htaccess`; set `AuthUserFile`
  there to the file's absolute path (KAS shows the account path, e.g. `/www/htdocs/<account>/api/public/generate/.htpasswd`);
- database: the same MySQL database; run the pending `migrations/mysql/*.sql` in phpMyAdmin first.

## Admin role in production

The admin API needs an account with the role `admin`: grant it in the legacy `generate/` SPA ("Users" → admin), or
with a shell `php bin/console roles grant <email> admin`.
