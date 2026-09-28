# Accounts, items and inventory

Backend for SCUTOONS reader accounts, the item catalog and inventories. The shared contract (concept, catalog
format, HTTP API) is `C:\Development\scutoons\agents\ITEMS.md` – including the section "Backend implementation
notes" with the exact response details. This file covers setup, configuration and operations.

Email login is a port of osct `server/api/src/{Auth,LoginMail,Mailer,Smtp,Progress}.php`: a login request mails a
6-digit one-time code and a magic link; confirming either creates the account (first time, with the starter items)
and returns a session token (Bearer). Tokens and codes are only stored as hashes.

## Layout

| Path | What |
|---|---|
| `api/index.php` | Front controller for every `/api/` path that is not an existing file/folder |
| `api/lib/` | Classes: `Config`, `Http` (JSON, CORS, errors), `Db`, `Catalog`, `Auth`, `LoginMail`, `Mailer`, `Smtp`, `Users`, `Codes`, `Inventory`, `Avatar`, `Progress`, `RateLimit`, `bootstrap.php` (denied over HTTP) |
| `api/items/items.json` | The item catalog (single source), served by `api/items/index.php` |
| `api/validate/` | Membership check for the signed-in owner of a code only (no email/signature) |
| `index.php` + `assets/js/claim.js` | QR landing page and claim flow |
| `db/migrations/2026-09-29-accounts-inventory.{mysql,sqlite}.sql` | New tables + `qr.edition_number`, `qr.claimed_by` |
| `scripts/` | CLI: `migrate.php`, `backfill-edition-numbers.php`, `report-orphan-codes.php` |
| `tests/api-smoke.php` (`.sh`) | End-to-end API test against a running server |
| `router.php` | `php -S` router (local only) |
| `logs/` | `mail.log` (dev login mails), `error.log` (API errors) – gitignored, denied over HTTP |

## Local setup

```bash
cp config.local.php.example config.local.php   # set AUTH_SECRET: php -r "echo bin2hex(random_bytes(32));"
cp db/local.sqlite db/local.sqlite.bak-$(date +%F)   # keep a backup before migrating
php scripts/migrate.php
php scripts/backfill-edition-numbers.php
php -S localhost:9000 router.php
bash tests/api-smoke.sh                         # all checks must pass
```

Login mails land in `logs/mail.log` (`MAIL_TRANSPORT = log`); the smoke test reads the OTP from there.
The smoke test claims one unclaimed vinyl and one Dago code of the local DB per run – local copies only.

## Endpoints (base `/api`)

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health/` | – | `{ status, db }` |
| GET | `/items/` | – | catalog |
| POST | `/auth/request/` | – | `{ email, code?, item_id?, variant?, locale? }` → `{ status:"sent" }` |
| POST | `/auth/verify/` | – | `{ email, otp }` or `{ token }` → session + claims the request's code |
| POST | `/auth/logout/` | Bearer | ends this session |
| GET | `/me/` | Bearer | `{ user, memberships, inventory, avatar }` |
| GET | `/code/?c=CODE` | optional | items/options/edition of a code, `claimed_by_me` when signed in |
| POST | `/inventory/claim/` | Bearer | `{ code, item_id?, variant? }` |
| POST | `/inventory/craft/` | Bearer | `{ recipe_id }` |
| POST | `/inventory/grant/` | Bearer | `{ item_id, source:"story"\|"scan", ref? }` |
| POST | `/inventory/import/` | Bearer | `{ items:[{ item_id, qty }] }` |
| GET/PUT | `/avatar/` | Bearer | `{ slots:{ <slot>: entryId\|null } }` |
| GET/PUT | `/progress/<toonId>/` | Bearer | `{ state, updated_at }` last write wins |
| GET/POST | `/validate/` | `{ code }` + Bearer | owner only: 401 `login_required` · 403 `not_claimed` · 409 `already_claimed` · 200 legacy shape |

Errors: `{ status:"error", error:"<code>", message }` (+ `options`, `missing`, `slot`, `retry_after` where noted).
CORS (credentials via Bearer header, no cookies): `ALLOWED_ORIGINS`.

Apache: `.htaccess` sends unknown `/api/` paths to `api/index.php` and passes the `Authorization` header to PHP.

## Configuration

Lookup order: environment `QR_<KEY>` → PHP constant `<KEY>` (`config.local.php` locally, `config.php` written by
the deploy workflow in production) → default in `api/lib/Config.php`. An empty constant falls back to the default.

| Key | Default | Notes |
|---|---|---|
| `AUTH_SECRET` | – | **required**, ≥ 32 chars; HMAC for login codes and IP hashes. Changing it invalidates open login mails only |
| `QR_URL` | `https://qr.scutoons.com` | magic links from the landing page |
| `APP_URL` | `https://scutoons.com` | "Open SCUTOONS" after a code-less magic link |
| `CDN_URL` | `https://cdn.scutoons.com/` | item icons (`<CDN_URL><icon>`); locally `http://localhost:8001/` |
| `ALLOWED_ORIGINS` | scutoons.com, www, localhost:5173/5174/4173 | comma separated |
| `MAIL_TRANSPORT` | `smtp` | `smtp` \| `log` (dev) \| `mail` |
| `MAIL_FROM`, `MAIL_FROM_NAME` | `no-reply@scutoons.com`, `SCUTOONS` | |
| `MAIL_LOG_PATH` | `logs/mail.log` | for `log` |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_USER`, `SMTP_PASS`, `SMTP_TIMEOUT` | –, 587, `tls`, –, –, 15 | `SMTP_SECURE`: `tls` (STARTTLS) / `ssl` (465) |
| `LOGIN_TTL_MINUTES` | 30 | validity of link + code |
| `LOGIN_MAX_ATTEMPTS` | 5 | wrong codes per mail |
| `LOGIN_MAX_PER_EMAIL_HOUR`, `LOGIN_MAX_PER_IP_HOUR` | 5, 30 | mails per hour |
| `SESSION_TTL_DAYS` | 365 | sessions end after this long unused |
| `GRANT_MAX_PER_HOUR`, `IMPORT_MAX_PER_HOUR`, `IMPORT_MAX_QTY` | 120, 10, 99 | |
| `PROGRESS_MAX_BYTES` | 262144 | |
| `LOG_PATH` | `logs/error.log` | API error log; empty = the host's log |

GitHub settings used by `.github/workflows/main.yml` (names only): secrets `AUTH_SECRET`, `SMTP_USER`, `SMTP_PASS`
(plus the existing `GENERATE_PASSWORD`, `DB_USER`, `DB_PASS`, `FTP_USERNAME`, `FTP_PASSWORD`); variables
`MAIL_FROM`, `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE` (plus `DB_HOST`, `DB_NAME`, `FTP_SERVER`). Values must not
contain a single quote (they are written into PHP string literals).

## Production (MariaDB) – steps

The deploy (push to `main`) uploads code only; `db/` and `scripts/` are not deployed.

1. **Back up** the database (phpMyAdmin → Export).
2. In phpMyAdmin run `db/migrations/2026-09-29-accounts-inventory.mysql.sql` (idempotent: `IF NOT EXISTS`
   everywhere, safe to run twice). Requires MariaDB ≥ 10.3 (production is 10.11).
   Then run `db/migrations/2026-09-30-code-owner-guard.mysql.sql` (triggers; SQL tab with delimiter `$$`, see the
   file header). Before that, check no inventory row points at a code owned by someone else (none can exist yet).
3. Edition numbers: copy the server's `generate/files/batch/*.json` locally (FTP), then
   `php scripts/backfill-edition-numbers.php --sql --manifests=<that folder> > backfill.sql` and run `backfill.sql`
   in phpMyAdmin. Only codes listed in a manifest get a number.
4. Review `php scripts/report-orphan-codes.php` against a copy of the production DB (codes that are in no manifest –
   locally 37 Dago and 20 beetles duplicates). Nothing is deleted automatically.
5. Set the GitHub secrets/variables above (at least `AUTH_SECRET` and the SMTP settings), then merge to `main`.
6. After the deploy: `GET https://qr.scutoons.com/api/health/`, then one real login + claim with a test code.
7. `generate/.htpasswd` is no longer in git and is excluded from the FTP sync, so the server copy stays.
   Keep a backup of it (and of `api/keys/keypair.json`) outside the repo.

## One owner per code

Every code that goes through qr.scutoons.com belongs to at most one account, enforced in three layers:

1. **API** – `Codes::claim` runs in a transaction (`BEGIN IMMEDIATE` on SQLite, `SELECT … FOR UPDATE` on MariaDB)
   and binds with `UPDATE qr SET claimed_by = ? … WHERE claimed_by IS NULL`; a second account gets 409
   `already_claimed`. Codes claimed by the old email-only form can only be claimed by an account with that email.
   The owner claiming again is a no-op (`already_yours`).
2. **No anonymous use** – `/api/validate/` only answers success for the signed-in owner. A photographed or shared
   code unlocks nothing on another device; the viewer no longer grants device-only memberships.
3. **Database** – triggers (`2026-09-30-code-owner-guard`) refuse re-assigning `qr.claimed_by` to another account
   and refuse inventory rows whose `code` belongs to someone else, even for writes outside the API.
   Releasing a code (support case) = set `claimed_by` to NULL and delete its inventory rows, then it can be claimed again.

Not covered (by design): `event`, `xr` and `page` codes carry no items and only redirect, so they stay shareable.

Tests: `tests/api-smoke.php` (owner/guest/other/unclaimed validate, DB guard) and `tests/claim-race.php [n]`
(n processes claim one code at the same moment → exactly one owner).

## Security notes

- `/api/keys/keys.php` serves the public key only; the admin SPA gets the keypair from `generate/keys.php`
  (admin token + Basic auth). `api/keys/.htaccess` denies everything else in that folder.
- `/api/validate/` and `/api/code/` never return who claimed a code; `/api/validate/` answers only the owner.
- A code is bound to one account (`qr.claimed_by`); codes claimed with the old email-only form can be taken over
  only by an account with that email.
- Grants are limited to the item's `grantable` sources; scan grants need a detecting tool and respect its cooldown.
