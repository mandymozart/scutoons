# Deploy and cutover

## How it deploys

`.github/workflows/server.yml` (monorepo root), path filter `scutoons-server/**`:

1. **test** (every push / pull request touching the server): PHP 8.3, `composer install`, PHPUnit, PHPStan.
2. **web-deploy** (only `main`, after the tests): `composer install --no-dev --optimize-autoloader` (the host has no
   shell, so `vendor/` is uploaded), writes `config.php` from the GitHub secrets/variables (same names as the old
   qr.scutoons.com workflow, list in [CONFIGURATION.md](CONFIGURATION.md); `ALLOWED_ORIGINS` = scutoons.com, www,
   creator, admin, shop), then FTP-syncs the server folder (`SamKirkland/FTP-Deploy-Action`, `server-dir` =
   variable `SERVER_FTP_DIR`, default the FTP account root as before).
   Never uploaded (and never deleted on the server): `db*/`, `logs/`, `storage/`, `tests/`, `tools/`, `docs/`,
   `bin/`, `migrations/`, `.git*`, `.htpasswd`, `config.local.php*`, dev tool configs.
   Uploaded: `public/`, `src/`, `config/` (+ the generated `config.php`), `templates/`, `vendor/`, `composer.*`,
   the root `.htaccess`.

Database changes are never deployed automatically – see [DATABASE.md](DATABASE.md). **This refactor changes no
database schema.**

## One folder, two subdomains, document root `public/`

Target (agents/PLATFORM.md "Domain architecture"): **one server folder** on all-inkl; **both** `qr.scutoons.com` and
`api.scutoons.com` point at **`<server folder>/public/`** (all-inkl KAS → Domains / Subdomains → target directory).
The app decides by host what answers (`HostRouting`, `QR_HOST` / `API_HOST`):

- qr: `/?c=` landing page, `/verify.html`, `/api/validate/`, `/api/keys/keys.php`, assets, legacy `generate/`, and –
  while `QR_HOST_SERVES_API` is on (default) – the full `/api/*` for released viewers.
- api: `/api/*` only (static pages of `public/` answer 404 there, `public/.htaccess`).

**The document root question.** Two layouts work with the same upload:

| Layout | Document root | How | Notes |
|---|---|---|---|
| A (old, works immediately) | `<server folder>/` | the root `.htaccess` forwards every request into `public/` and denies `src/`, `config/`, `vendor/`, `migrations/`, `bin/`, `tests/`, `db/`, `logs/`, `storage/`, `templates/`, `docs/` | relies on mod_rewrite + `.htaccess`; private files are inside the web root (denied) |
| **B (recommended)** | `<server folder>/public/` | `public/.htaccess` only | private files are physically outside the web root; the root `.htaccess` is not used |

All printed URLs are identical in both layouts (checked by `tests/public-urls.php`).

## Cutover from the old plain-PHP layout (qr.scutoons.com)

The old repository (`mandymozart/qr.scutoons.com`) deployed the same folder with the same FTP action. Its files:
`index.php`, `db.php`, `config.php` (generated), `router.php`, `verify.html`, `assets/`, `api/…` (incl. the private
`api/keys/keypair.json`), `generate/…` (incl. `generate/.htpasswd` and the batch output `generate/files/`).

1. **Preconditions.** `platform` merged into `main` of the monorepo; the GitHub secrets/variables exist in the
   monorepo (same names as before, [CONFIGURATION.md](CONFIGURATION.md)); production runs the account-system schema
   (the `*.mysql.sql` migrations up to `2026-09-30-content-versions` – [DATABASE.md](DATABASE.md)). Nothing else in
   the database changes.
2. **Stop the old deploy**: disable Actions in the old repository (or archive it), so it never uploads the old layout
   over the new one.
3. **Back up** (FTP download + phpMyAdmin export): `generate/files/`, `api/keys/keypair.json`, `generate/.htpasswd`,
   `config.php`, and the database.
4. **Keys first (no gap for `/api/keys/keys.php`)**: create `storage/keys/` in the server folder and upload a copy of
   `api/keys/keypair.json` there as `storage/keys/keypair.json`.
5. **Deploy**: push/merge to `main` (or run the workflow). The FTP action compares with its sync state file in the
   server folder (written by the old workflow) and deletes the old files it had uploaded (`index.php`, `db.php`,
   `api/…`, `generate/*.php`, …). If that state file is missing it deletes nothing – then remove the old files by hand
   (they are unreachable anyway: the root `.htaccess` forwards everything into `public/`).
6. **Batch output**: rename `generate/files/` → `storage/files/` (FTP rename, instant). Until then the legacy SPA's
   batch list is empty; the verify page is not affected (signatures come from the database). Then delete the old
   `api/keys/keypair.json` (keep your backup).
7. **Check** (layout A is live now):
   `curl https://qr.scutoons.com/api/health/` → `{"status":"ok","db":"ok"}`; open `https://qr.scutoons.com/?c=<real code>`
   for a membership, a promo and a page code; `/?c=<code>&skip=1` redirects; `/verify.html` with a real certificate
   code; `/api/validate/?c=<code>`; `/api/keys/keys.php`; one login + claim with a test code in the viewer; `/generate/`
   (Basic auth + password) → Stats and Users load.
   From a machine with a copy of the production DB: `php tests/public-urls.php https://qr.scutoons.com --db=<copy>`.
8. **Document root → `public/` (layout B)**: in KAS set the target directory of `qr.scutoons.com` to
   `<server folder>/public/`. Re-check step 7. **Basic auth of `generate/`**: `public/generate/.htaccess` still says
   `AuthUserFile .htpasswd`; Apache resolves a relative path against its server root, so set it to the absolute path of
   the existing file (KAS shows the account path, e.g. `/www/htdocs/<account>/<folder>/generate/.htpasswd`) or move the
   file next to it and use that absolute path – then check `/generate/` asks for the password again.
9. **api.scutoons.com**: create the subdomain in KAS with the same target `<server folder>/public/` and SSL. Check
   `https://api.scutoons.com/api/health/` (200) and `https://api.scutoons.com/?c=x` (404). Then switch the frontends'
   `VITE_API_URL` to `https://api.scutoons.com/api`.
10. **Later**: when every app uses api.scutoons.com and the stats show no more `/api/*` traffic on qr, add
    `define('QR_HOST_SERVES_API', '0');` to the generated config in `server.yml`. Printed URLs (`/?c=`, `/verify.html`,
    `/api/validate/`, `/api/keys/keys.php`) stay on qr forever.

**Rollback**: re-upload the backup of the old folder (or re-enable and re-run the old repository's workflow), move
`storage/files/` back to `generate/files/` and the keypair back to `api/keys/`. The database needs no rollback.

## Admin role in production

The admin API needs an account with the role `admin`: grant it in the legacy `generate/` SPA ("Users" → admin), or
with a shell `php bin/console roles grant <email> admin`.
