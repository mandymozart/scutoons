# Accounts, items and inventory

Backend for SCUTOONS reader accounts, the item catalog and inventories. The shared contract (concept, catalog
format, HTTP API) is `C:\Development\scutoons\agents\ITEMS.md` – including the section "Backend implementation
notes" with the exact response details. This file covers setup, configuration and operations.

Email login is a port of osct `server/api/src/{Auth,LoginMail,Mailer,Smtp,Progress}.php`: a login request mails a
6-digit one-time code and a magic link; confirming either creates the account (first time, with the `member` role
and the starter items) and returns a session token (Bearer). Tokens and codes are only stored as hashes.
**New accounts only with a signup code** (a SCUTOONS product or a promo code) – see "Signup rule".

## Layout

| Path | What |
|---|---|
| `api/index.php` | Front controller for every `/api/` path that is not an existing file/folder |
| `api/lib/` | Classes: `Config`, `Http` (JSON, CORS, errors), `Db`, `Catalog`, `Permissions`, `Events`, `Auth`, `LoginMail`, `Mailer`, `Smtp`, `Users`, `Roles`, `Codes`, `Inventory`, `Avatar`, `Progress`, `RateLimit`, `bootstrap.php` (denied over HTTP) |
| `api/permissions.json` | What each code type grants (server-only, denied over HTTP) – see "Permissions map" |
| `api/items/items.json` | The item catalog (single source), served by `api/items/index.php` |
| `api/validate/` | Legacy membership check, backwards compatible (no email/signature/claimed_by) |
| `index.php` | QR landing page: preview + Continue + "Add to my account" (no email step); promo: Continue + "Create account" |
| `generate/stats.php`, `code-events.php`, `revoke.php` + `app/views/stats.js` | Admin "Stats" view: activity per code, revoke |
| `generate/users.php`, `user-roles.php` + `app/views/users.js` | Admin "Users" view: search by email, roles, signup code, opt-in |
| `db/migrations/2026-09-29-accounts-inventory.{mysql,sqlite}.sql` | New tables + `qr.edition_number`, `qr.claimed_by` |
| `db/migrations/2026-09-30-code-owner-guard.*`, `2026-09-30-revoke-events.*` | Owner triggers; `qr.revoked_at`/`revoked_reason`, `code_events` |
| `db/migrations/2026-09-30-roles-signup.*` | `roles`, `user_roles` (+ backfill `member`), `users.marketing_opt_in`/`_at`/`signup_code`, `login_requests.marketing_opt_in` |
| `scripts/` | CLI: `migrate.php`, `backfill-edition-numbers.php`, `report-orphan-codes.php`, `roles.php` |
| `tests/api-smoke.php` (`.sh`), `tests/claim-race.php` | End-to-end API test against a running server (incl. signup rule, roles, opt-in); concurrent claim test |
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
| POST | `/auth/request/` | – | `{ email, code?, item_id?, variant?, locale?, marketing_opt_in? }` → `{ status:"sent" }` (always; new accounts only with a signup code) |
| POST | `/auth/verify/` | – | `{ email, otp }` or `{ token }` (+ `marketing_opt_in?`) → session + claims the request's code if claimable · `403 signup_code_required` |
| POST | `/auth/logout/` | Bearer | ends this session |
| GET | `/me/` | Bearer | `{ user, access, memberships, inventory, avatar }` – `user: { id, email, created_at, roles, marketing_opt_in }`, `access: { permissions, unlocked, expires_at, codes, access_token? }` |
| PUT | `/me/` | Bearer | `{ marketing_opt_in: bool }` → `{ status, user }` · `400 invalid_body` |
| GET/POST | `/access/` | optional | `{ code }` / `?c=` → device access: `type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token?` (never the owner); per-IP limit; logs `access`; send `X-Device-Id` |
| GET | `/code/?c=CODE` | optional | items/options/edition, `claimable`, `signup` of a code, `claimed_by_me` when signed in |
| POST | `/inventory/claim/` | Bearer | `{ code, item_id?, variant? }` |
| POST | `/inventory/craft/` | Bearer | `{ recipe_id }` |
| POST | `/inventory/grant/` | Bearer | `{ item_id, source:"story"\|"scan", ref? }` |
| POST | `/inventory/import/` | Bearer | `{ items:[{ item_id, qty }] }` |
| GET/PUT | `/avatar/` | Bearer | `{ slots:{ <slot>: entryId\|null } }` |
| GET/PUT | `/progress/<toonId>/` | Bearer | `{ state, updated_at }` last write wins |
| GET/POST | `/validate/` | – | legacy, backwards compatible: success for any existing, unexpired, unrevoked code (claimed or not) |

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
| `ACCESS_MAX_PER_IP_HOUR` | 120 | `/access/` + `/code/` lookups per IP and hour |
| `PERMISSIONS_PATH` | `api/permissions.json` | another permissions map (tests, staging) |

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
   Then `db/migrations/2026-09-30-revoke-events.mysql.sql` (idempotent), then
   `db/migrations/2026-09-30-roles-signup.mysql.sql` (idempotent; backfills `member` for every account).
   Grant `creator` afterwards in the admin SPA (Users) – there is no shell on the host.
3. Edition numbers: copy the server's `generate/files/batch/*.json` locally (FTP), then
   `php scripts/backfill-edition-numbers.php --sql --manifests=<that folder> > backfill.sql` and run `backfill.sql`
   in phpMyAdmin. Only codes listed in a manifest get a number.
4. Review `php scripts/report-orphan-codes.php` against a copy of the production DB (codes that are in no manifest –
   locally 37 Dago and 20 beetles duplicates). Nothing is deleted automatically.
5. Set the GitHub secrets/variables above (at least `AUTH_SECRET` and the SMTP settings), then merge to `main`.
6. After the deploy: `GET https://qr.scutoons.com/api/health/`, then one real login + claim with a test code.
7. `generate/.htpasswd` is no longer in git and is excluded from the FTP sync, so the server copy stays.
   Keep a backup of it (and of `api/keys/keypair.json`) outside the repo.

## Access model: viewing vs. owning

(ITEMS.md "Access model", final, 2026-09-30.)

- **Viewing is device-level.** Anyone holding an existing, unexpired, unrevoked code whose type grants `catalog`
  may read everything on that device – also when the code is owned by someone else. The viewer asks `/api/access/`
  and keeps the answer (and its `access_token`) in the browser. Nothing is stored on the server except the
  activity event. Codes are meant to be shareable (no device limits).
- **Owning is an account action.** Claiming (item in the inventory, edition number), avatar, synced progress need an
  email login. One owner per code applies only here.

### One owner per code (ownership)

1. **API** – `Codes::claim` runs in a transaction (`BEGIN IMMEDIATE` on SQLite, `SELECT … FOR UPDATE` on MariaDB)
   and binds with `UPDATE qr SET claimed_by = ? … WHERE claimed_by IS NULL`; a second account gets 409
   `already_claimed` (it can still read with the code). Codes claimed by the old email-only form can only be claimed
   by an account with that email (409 `email_required`). The owner claiming again is a no-op (`already_yours`).
2. **Database** – triggers (`2026-09-30-code-owner-guard`) refuse re-assigning `qr.claimed_by` to another account
   and refuse inventory rows whose `code` belongs to someone else, even for writes outside the API.
   Releasing a code (support case) = set `claimed_by` to NULL and delete its inventory rows.

Tests: `tests/api-smoke.php` and `tests/claim-race.php [n]` (n processes claim one code at once → exactly one owner).

## Permissions map – how to change what a code type unlocks

`api/permissions.json` (server-only, never served over HTTP):

```json
{
  "version": 1,
  "permissions": { "catalog": "Read every chapter of every toon" },
  "types": {
    "membership": { "claimable": true,  "signup": true,  "grants": ["catalog"] },
    "promo":      { "claimable": false, "signup": true,  "grants": ["catalog"] },
    "page":       { "claimable": false, "signup": false, "grants": [] },
    "event":      { "claimable": false, "signup": false, "grants": [] },
    "xr":         { "claimable": false, "signup": false, "grants": [] }
  },
  "default":      { "claimable": false, "signup": false, "grants": [] }
}
```

- `claimable`: the code can be owned by one account (goes into its inventory). `grants`: permission ids for anyone
  holding the code (device) and for its owner (account) while it is neither expired nor revoked. `signup`: a new
  account can be created with a code of this type (see "Signup rule").
- Types not listed use `default`. The viewer checks `catalog` today; ids like `toon:catpope` or
  `chapter:catpope:2` can be added later without code changes on this server.
- To change it: edit the file (valid JSON!), deploy (push to `main`) or upload it over FTP. No restart, no migration.
  Check locally with `php tests/api-smoke.php` (it also proves an override map is applied). A broken file makes the
  code endpoints answer 500, so validate the JSON before uploading.
- `PERMISSIONS_PATH` points to another file (staging, tests).

## Signup rule, promo codes

(ITEMS.md "Roles, signup rule, promo codes", 2026-09-30.)

- **A new account needs a signup code**: a code whose type has `"signup": true` (today `membership` and `promo`),
  existing, not expired, not revoked. A claimable code (membership = a product) must also still be free (or
  registered to this email by the old form) – it is claimed in the same step. `Codes::signupRefusal()` decides.
- `POST /auth/request/` for an email **without** an account and **without** a usable signup code still answers
  `{ status:"sent" }` (no user enumeration), but the mail (de/en, `LoginMail::composeNoAccount`) has no link and no
  one-time code: it says there is no account yet and that one needs a code from a SCUTOONS product or a promo code
  (and names the code sent, if it was not usable). The request is stored only for the hourly limits (hashes of values
  nobody knows – it can never be confirmed).
- `POST /auth/verify/` checks the rule again for a new account (the code may have been revoked meanwhile) →
  `403 signup_code_required`, nothing created. Existing accounts sign in with or without a code.
- At signup: `users.signup_code` = the code, `users.marketing_opt_in` (+ `marketing_opt_in_at`) from
  `marketing_opt_in` in the request body (default false; `verify` may override it), role `member`, event `signup`.
  An existing account that sends `marketing_opt_in: true` while signing in is opted in (never opted out there).
- **Promo codes are never owned**: no item, no owner, reusable for any number of signups. Claim → `422 not_claimable`;
  `/auth/verify/` with a promo code only creates/signs in (no `claimed`/`claim_error`). Landing page: "Promo code – no
  item to collect" with **Continue** and **Create account** (→ `<viewer>/account?code=`).
- `PUT /me/ { marketing_opt_in }` toggles the opt-in (account page); `marketing_opt_in_at` = last change.
- Login requests get time-ordered ids (`Db::uuid7()`), so "the newest request of this address" is exact also when
  several mails are requested within one second.

## Roles

Tables `roles` (seeded: 1 `member` – every account, 2 `admin` – reserved, 3 `creator` – scutoons.com/creator and
the content API) and `user_roles (user_id, role_id, granted_at, granted_by)`. `granted_by`: `signup`, `migration`,
`admin` (SPA), `cli`. `/me/` and `/auth/verify/` return `user.roles` (e.g. `["member", "creator"]`).

For other endpoints (`api/lib/Roles.php`):

```php
$user = Roles::requireRole(Roles::CREATOR); // signed-in user, else 401 unauthorized / 403 forbidden { role }
Roles::has($user, 'creator');               // bool; $user = users row or id
Roles::of($userId);                         // ['member', 'creator']
Roles::grant($userId, 'creator', 'admin');  // true when newly granted
Roles::revoke($userId, 'creator');          // true when removed
```

Granting/revoking:
- Admin SPA → **Users**: search by email, checkboxes per role, signup code, news opt-in. Endpoints (admin token):
  `GET generate/users.php?q=<email part>` → `{ success, roles, total, users:[{ id, email, created_at, roles,
  signup_code, marketing_opt_in, marketing_opt_in_at }] }` (newest 50 when `q` is empty) and
  `POST generate/user-roles.php { user_id | email, role, action:"grant"|"revoke" }` → `{ success, changed, user }`.
- CLI: `php scripts/roles.php grant|revoke <email> <role>`, `php scripts/roles.php list [email]` (without email:
  every account with a role other than member).

The Users view shows emails (it is the place to manage accounts); the Stats view still shows none.

## Access tokens

`/access/` (when the code grants `catalog`) and `/me/` `access` (when unlocked) include `access_token`:
`base64url(payload) "." base64url(HMAC-SHA256(payload part))`, payload `{ v:1, perms, exp, sub }`,
`exp = min(code expiry, now + 30 days)`, `sub = "code:<keyed hash of the code>"` or `"user:<id>"`. The key is
derived from `AUTH_SECRET` with its own label; rotating `AUTH_SECRET` invalidates all tokens.
`Permissions::verifyToken($token)` returns the payload or null – for a later server-side content gate (nothing
checks the token yet).

## Revoke + activity statistics

- **Revoke** one code in the admin SPA ("Stats" → code → Revoke, with a reason). A revoked code answers
  `410 revoked` in `/access/`, `/code/`, `/validate/` and claims; `/me/` access ignores it; the landing page says
  "This code is no longer valid". Unrevoke restores it. Both are logged as events.
- **Events** (`code_events`, append-only, `api/lib/Events.php`): `scan` (landing page), `access` (at most once per
  code + device per UTC day), `claim`, `claim_refused`, `login` (one per owned code at sign-in), `signup` (account
  created with this code), `revoke`, `unrevoke`. Logging never breaks a request.
- **Admin endpoints** (admin token, under `generate/`, never public): `stats.php` (per-code table, `?format=csv`,
  `?days=` for the daily chart), `code-events.php?code=` (timeline), `revoke.php` (POST).

### Privacy

What is stored per event: the code, the event type, the time (UTC), optionally the toon, the account id for
`claim`/`claim_refused`/`login`/`signup`/signed-in `access`, `device_hash` = HMAC of the browser's random `X-Device-Id`
(localStorage `scutoons_device`, not linked to a person), `ip_hash` = HMAC of the IP with a **daily** salt derived
from `AUTH_SECRET` + UTC date (hashes of different days can't be linked; no salt is stored). Never stored: raw IP,
email, user agent, the device id itself. The Stats views show no emails or user ids (only "account: yes").

Accounts store the email, the signup code and the news opt-in (`marketing_opt_in`, unchecked by default, with the
time of the last change as consent record). Mention the opt-in in the privacy policy; export the opted-in addresses
with `SELECT email FROM users WHERE marketing_opt_in = 1` when sending news.

Retention suggestion: aggregate what you need and delete events older than 13 months
(`DELETE FROM code_events WHERE created_at < NOW() - INTERVAL 13 MONTH` in phpMyAdmin, e.g. yearly), and mention
the anonymous usage statistics in the privacy policy.

## Security notes

- `/api/keys/keys.php` serves the public key only; the admin SPA gets the keypair from `generate/keys.php`
  (admin token + Basic auth). `api/keys/.htaccess` denies everything else in that folder.
- `/api/validate/`, `/api/code/`, `/api/access/` never return who claimed a code.
- `api/permissions.json` is server-only (denied in `.htaccess` and `router.php`); clients only see the resulting
  `permissions`. Access tokens are HMAC-signed with a key derived from `AUTH_SECRET` (own label).
- A code is bound to one account (`qr.claimed_by`); codes claimed with the old email-only form can be taken over
  only by an account with that email.
- Grants are limited to the item's `grantable` sources; scan grants need a detecting tool and respect its cooldown.
