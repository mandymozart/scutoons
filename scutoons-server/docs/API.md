# API – accounts, codes, items, content, admin

The shared contract (concept, catalog format, every request/response) is `agents/ITEMS.md` in the monorepo – including
"Backend implementation notes" and the "Admin API" section. It must not change without updating that file. This page
describes how the server implements it. Setup: [../README.md](../README.md); code structure:
[ARCHITECTURE.md](ARCHITECTURE.md); settings: [CONFIGURATION.md](CONFIGURATION.md).

Email login (a port of osct `Auth/LoginMail/Mailer/Smtp/Progress`): a login request mails a 6-digit one-time code and a
magic link; confirming either creates the account (first time, with the `member` role and the starter items) and returns
a session token (Bearer). Tokens and codes are only stored as hashes. **New accounts only with a signup code** (a
SCUTOONS product or a promo code) – see "Signup rule".

## Hosts

Base `https://api.scutoons.com/api` (target) – and, during the transition (`QR_HOST_SERVES_API`, default on), the same
paths on `https://qr.scutoons.com/api` (released viewers). Locally `http://localhost:9000/api` (`VITE_API_URL`).
Public on qr forever: `/?c=`, `/verify.html`, `/api/validate/`, `/api/keys/keys.php`.

## Endpoints (base `/api`)

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health/` | – | `{ status, db }` |
| GET | `/items/` | – | catalog (`config/items.json`); any origin (`*`), cached 5 min |
| GET | `/editions/` (`?toon=`), `/toons/` | – | public lists (`config/editions.json`, `config/toons.json`) |
| POST | `/auth/request/` | – | `{ email, code?, item_id?, variant?, locale?, marketing_opt_in? }` → `{ status:"sent", expires_at }` (always; new accounts only with a signup code) |
| POST | `/auth/verify/` | – | `{ email, otp }` or `{ token }` (+ `marketing_opt_in?`) → session + claims the request's code if claimable · `403 signup_code_required` |
| POST | `/auth/logout/` | Bearer | ends this session |
| GET | `/me/` | Bearer | `{ user, access, memberships, inventory, avatar }` – `user: { id, email, created_at, roles, marketing_opt_in }`, `access: { permissions, unlocked, expires_at, codes, signup_code, access_token? }` (owned codes + the valid signup code) |
| PUT | `/me/` | Bearer | `{ marketing_opt_in: bool }` → `{ status, user }` · `400 invalid_body` |
| GET/POST | `/access/` | optional | `{ code }` / `?c=` → device access: `type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token?` (never the owner); per-IP limit; logs `access`; send `X-Device-Id` |
| GET | `/code/?c=CODE` | optional | items/options/edition, `claimable`, `signup` of a code, `claimed_by_me` when signed in; per-IP limit |
| POST | `/inventory/claim/` | Bearer | `{ code, item_id?, variant? }` |
| POST | `/inventory/craft/` | Bearer | `{ recipe_id }` |
| POST | `/inventory/grant/` | Bearer | `{ item_id, source:"story"\|"scan", ref? }` |
| POST | `/inventory/import/` | Bearer | `{ items:[{ item_id, qty }] }` |
| GET/PUT | `/avatar/` | Bearer | `{ slots:{ <slot>: entryId\|null } }` |
| GET/PUT | `/progress/<toonId>/` | Bearer | `{ state, updated_at }` last write wins |
| GET | `/content/<toon>/<locale>/<path>.json` | – / creator | published config (raw JSON); `?draft=1`, `?version=`, `&wrap=1` need `creator` – see "Content API" |
| PUT | `/content/<toon>/<locale>/<path>.json` | creator | `{ content, message?, base? }` → new draft version |
| GET | `/content/versions`, `/content/index` | creator | history of a file / all stored files of a toon |
| POST | `/content/publish`, `/content/unpublish`, `/content/restore` | creator | make a version live / nothing live / old version → new draft |
| GET/POST | `/validate/` | – | PUBLIC URL (printed): legacy, backwards compatible – success for any existing, unexpired, unrevoked code (claimed or not); CORS `*` |
| GET | `/keys/keys.php` | – | PUBLIC URL (printed): the public key of the certificate signatures |
| * | `/admin/*` | admin | the admin API – see "Admin API" |

Errors: `{ status:"error", error:"<code>", message }` (+ `options`, `missing`, `slot`, `retry_after`, `role` where
noted); unknown paths `404 not_found` "Unknown endpoint.". CORS (Bearer header, no cookies): `ALLOWED_ORIGINS`.

## Admin API (`/api/admin/*`, role `admin`)

Full contract: agents/ITEMS.md "Admin API". Session (Bearer) + `RequireRole(admin)` → `401 unauthorized` /
`403 forbidden { role:"admin" }`. Lists follow the admin-app convention `?page=&page_size=&sort=&q=&<filter>=` →
`{ status, items, pagination:{ total, page, pageSize } }`.

| Method | Path | What |
|---|---|---|
| GET | `/dashboard?days=30&edition_id=` | KPIs: totals, period counts, per edition (codes, claimed, revoked, signed + activity), daily scan/access/claim/signup/… (zero-filled), daily per edition |
| GET | `/codes` (`?format=csv`) | codes + statistics; filters `edition_id` (`none`), `type`, `status` (claimed, unclaimed, revoked, expired, active, signed, unsigned), `q`; CSV of the filtered list |
| POST | `/codes` | create one code `{ redirect_to, type, code?, email?, expires_at?, edition_id? }` → 201 · `409 code_exists` |
| GET | `/codes/{code}` · `/codes/{code}/events` | detail (no email/owner id) · timeline |
| POST | `/codes/{code}/revoke` `{ reason? }` · `/codes/{code}/unrevoke` | |
| GET/PUT | `/codes/{code}/signature` | read / store the certificate signature |
| GET/POST | `/batches` | list · create `{ edition_id, count?, start_number?, force? }` (reserves codes, `409 batch_complete`) |
| GET/PUT | `/batches/{filename}` · GET `/batches/{filename}/zip` | manifest · progress · ZIP |
| GET/PUT | `/files/{type}/{filename}` | batch output (svgs, certificates, signatures, batch, zips) |
| GET | `/users` · `/users/{id}` · `/roles` | accounts (emails shown here), detail, roles |
| PUT/DELETE | `/users/{id}/roles/{role}` | grant / revoke |
| GET | `/events` | activity log (filters code, event list, edition_id, toon, from, to) |
| GET | `/catalog` · `/keys` | items, editions, toons, permissions map (read-only) · the ML-DSA-65 keypair for client-side signing |
| GET | `/content?toon=` | content store overview, every toon: per stored file the newest version (+ status, time), the published version, the version count (read-only; platform P2a) |

The legacy `generate/*.php` endpoints (static token `X-Generate-Token`) stay for the `generate/` SPA until the admin
app has parity; they call the same services.

## Access model: viewing vs. owning

(ITEMS.md "Access model", final, 2026-09-30.)

- **Viewing is device-level.** Anyone holding an existing, unexpired, unrevoked code whose type grants `catalog` may
  read everything on that device – also when the code is owned by someone else. The viewer asks `/api/access/` and
  keeps the answer (and its `access_token`) in the browser. Nothing is stored on the server except the activity
  event. Codes are meant to be shareable (no device limits).
- **Owning is an account action.** Claiming (item in the inventory, edition number), avatar, synced progress need an
  email login. One owner per code applies only here.

### One owner per code (ownership)

1. **API** – `CodeService::claim` runs in a transaction (`BEGIN IMMEDIATE` on SQLite, `SELECT … FOR UPDATE` on MariaDB)
   and binds with `UPDATE qr SET claimed_by = ? … WHERE claimed_by IS NULL`; a second account gets 409
   `already_claimed` (it can still read with the code). Codes claimed by the old email-only form can only be claimed by
   an account with that email (409 `email_required`). The owner claiming again is a no-op (`already_yours`).
2. **Database** – triggers (`2026-09-30-code-owner-guard`) refuse re-assigning `qr.claimed_by` to another account and
   refuse inventory rows whose `code` belongs to someone else, even for writes outside the API. Releasing a code
   (support case) = set `claimed_by` to NULL and delete its inventory rows.

Tests: `tests/api-smoke.php` and `tests/claim-race.php [n] [claim|signup|both]` (n processes claim one code at once →
exactly one owner; n fresh emails sign up with one product code at once → exactly one account, owning it). `/access/`
+ `/code/` share a per-IP limit (120/hour) – `composer test` works on a fresh DB copy each run; against the dev DB use
`DELETE FROM rate_limits WHERE bucket LIKE 'lookup:%'` or raise `ACCESS_MAX_PER_IP_HOUR` in config.local.php.

## Permissions map – how to change what a code type unlocks

`config/permissions.json` (server-only, never served over HTTP):

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
- Types not listed use `default`. The viewer checks `catalog` today; ids like `toon:catpope` or `chapter:catpope:2` can
  be added later without code changes on this server.
- To change it: edit the file (valid JSON!) and deploy. No migration. A broken file makes the code endpoints answer 500,
  so validate the JSON first (`composer test` proves an override map is applied). `PERMISSIONS_PATH` points to another
  file (staging, tests). The admin API shows it read-only (`/api/admin/catalog`).

## Signup rule, promo codes

(ITEMS.md "Roles, signup rule, promo codes", 2026-09-30.)

- **A new account needs a signup code**: a code whose type has `"signup": true` (today `membership` and `promo`),
  existing, not expired, not revoked. A claimable code (membership = a product) must also still be free (or registered
  to this email by the old form) – it is claimed in the same step. `CodeService::signupRefusal()` decides.
- `POST /auth/request/` for an email **without** an account and **without** a usable signup code still answers
  `{ status:"sent" }` (no user enumeration), but the mail (de/en, `LoginMail::composeNoAccount`, texts in
  `templates/mail/texts.php`) has no link and no one-time code: it says there is no account yet and how to get one.
  The request is stored only for the hourly limits (hashes of values nobody knows – it can never be confirmed).
- `POST /auth/verify/` checks the rule again for a new account → `403 signup_code_required`, nothing created. Existing
  accounts sign in with or without a code.
- **Signup with a product code is atomic** (`LoginService::confirm`, `CodeService::claimInTransaction`): creating the
  account and claiming its code happen in one transaction. If the claim fails, nothing is created and the login request
  stays unused: `409 already_claimed`, `422 choose_item` (+ `options`; confirm again with the same one-time code plus
  `item_id`/`variant`), `422 invalid_item`.
- **Promo accounts read on every device**: `/me/ access` also counts the grants of the account's `signup_code` while it
  exists, is not expired or revoked and its type grants something – without owning it. `access.signup_code` names it
  when it contributes (else `null`); revoking the promo code ends that access.
- At signup: `users.signup_code`, `users.marketing_opt_in` (+ `marketing_opt_in_at`), role `member`, event `signup`. An
  existing account that sends `marketing_opt_in: true` while signing in is opted in (never opted out there).
- **Promo codes are never owned**: claim → `422 not_claimable`; `/auth/verify/` with a promo code only creates/signs in.
  Landing page: "Promo code – no item to collect" with **Continue** and **Create account** (→ `<viewer>/account?code=`).
- Login requests get time-ordered ids (`Support\Ids::uuid7()`), so "the newest request of this address" is exact also
  within one second.

## Roles

Tables `roles` (seeded: 1 `member` – every account, 2 `admin` – the admin app and `/api/admin/*`, 3 `creator` – the
creator app and the content API) and `user_roles (user_id, role_id, granted_at, granted_by)`. `granted_by`: `signup`,
`migration`, `admin` (legacy SPA), `admin:<email>` (admin API), `cli`. `/me/` and `/auth/verify/` return `user.roles`.

In code (`src/Domain/Roles/Roles.php`): `has($user, 'creator')`, `of($userId)`, `require($user, 'creator')`
(401 / 403 `forbidden { role }`), `grant($userId, $role, $by)`, `revoke($userId, $role)`; routes use the
`RequireRole` middleware. Granting: admin API `PUT|DELETE /api/admin/users/{id}/roles/{role}`, the legacy SPA "Users"
view (`generate/users.php`, `generate/user-roles.php`), or `php bin/console roles grant|revoke|list <email> [role]`.

## Access tokens

`/access/` (when the code grants `catalog`) and `/me/` `access` (when unlocked) include `access_token`:
`base64url(payload) "." base64url(HMAC-SHA256(payload part))`, payload `{ v:1, perms, exp, sub }`,
`exp = min(code expiry, now + 30 days)`, `sub = "code:<keyed hash of the code>"` or `"user:<id>"`. The key is derived
from `AUTH_SECRET` with its own label; rotating `AUTH_SECRET` invalidates all tokens. `AccessTokens::verify($token)`
returns the payload or null – for a later server-side content gate (nothing checks the token yet).

## Revoke + activity statistics

- **Revoke** one code (admin API `POST /api/admin/codes/{code}/revoke`, or the legacy SPA "Stats"): a revoked code
  answers `410 revoked` in `/access/`, `/code/`, `/validate/` and claims; `/me/` access ignores it; the landing page says
  "This code is no longer valid". Unrevoke restores it. Both are logged as events.
- **Events** (`code_events`, append-only, `src/Domain/Events/EventLog.php`): `scan` (landing page), `access` (at most
  once per code + device per UTC day), `claim`, `claim_refused`, `login` (one per owned code at sign-in), `signup`,
  `revoke`, `unrevoke`. Logging never breaks a request.
- **Admin views**: `/api/admin/dashboard`, `/api/admin/codes`, `/api/admin/events` (and the legacy `generate/stats.php`,
  `code-events.php`, `revoke.php`).

### Privacy

Per event: the code, the event type, the time (UTC), optionally the toon, the account id for `claim`/`claim_refused`/
`login`/`signup`/signed-in `access`, `device_hash` = HMAC of the browser's random `X-Device-Id`, `ip_hash` = HMAC of the
IP with a **daily** salt derived from `AUTH_SECRET` + UTC date. Never stored: raw IP, email, user agent, the device id
itself. Code views and statistics show no emails or user ids (only "account: yes"); the users views are the place for
emails.

Accounts store the email, the signup code and the news opt-in (`marketing_opt_in`, unchecked by default, with the time
of the last change as consent record). Export the opted-in addresses with
`SELECT email FROM users WHERE marketing_opt_in = 1` when sending news. Retention suggestion: delete events older than
13 months (`DELETE FROM code_events WHERE created_at < NOW() - INTERVAL 13 MONTH`).

## Content API (creator editor)

Store for the chapter / season / toon configs and VN scene files (the viewer reads the store first and falls back to its
static files).

- **Table `content_versions`** `(id, toon, locale, path, json, author → users.id | NULL, created_at UTC, status, message)`.
  Every save is a new row. Per file (toon, locale, path) at most **one** row is `published`; all others are `draft`. The
  newest row is the working copy.
- **Validation** (minimal, the editor checks the TypeScript types): toon `^[a-z0-9_-]{1,64}$`, locale `^[a-z]{2}$`, path
  `^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$`, body JSON object or array, `CONTENT_MAX_BYTES` (default 2 MB). JSON is decoded
  as objects and stored re-encoded (unescaped unicode/slashes), so `{}` stays `{}`.

| Method | Path | Role | Body / query → answer |
|---|---|---|---|
| GET | `/content/<toon>/<locale>/<path>` | – | the published version as raw JSON (`ETag: "v<id>"`, `Cache-Control: no-cache`, 304 on `If-None-Match`, headers `X-Content-Version`/`X-Content-Status`) · 404 when nothing is published |
| GET | same `?draft=1` | creator | the newest version (raw JSON, `no-store`) · 401 / 403 `forbidden` |
| GET | same `?version=<id>` | creator | that version of this file · 404 |
| GET | same `…&wrap=1` | creator | `{ status, version: meta, published: { id, created_at } \| null, content }` |
| PUT | `/content/<toon>/<locale>/<path>` | creator | `{ content, message?, base? }` → `{ status, unchanged, version: meta }`; `base` not the newest → **409 `conflict`** `{ latest: meta }` |
| GET | `/content/versions?toon=&locale=&path=&limit=` | creator | `{ versions: [{ id, status, created_at, author, author_email, message, size }] }` newest first |
| GET | `/content/index?toon=` | creator | `{ files: [{ locale, path, latest, published, versions }] }` |
| POST | `/content/publish` | creator | `{ toon, locale, path, version? }` (default: newest) → `{ version }` |
| POST | `/content/unpublish` | creator | `{ toon, locale, path }` → `{ unpublished: n }` |
| POST | `/content/restore` | creator | `{ id, message? }` → `{ version }` – that version as the newest **draft** |

`meta = { id, toon, locale, path, status, created_at, author, author_email, message, size }`. Errors:
`400 invalid_toon|invalid_locale|invalid_path|invalid_json|missing_content`, `413 content_too_large`,
`422 invalid_content`, `401 unauthorized`, `403 forbidden` (`role: "creator"`).

**Seeding** – `php bin/console import-content <monorepo>/viewer/static/api [--toon=] [--db=<sqlite>] [--update]
[--dry-run] [--sql]` (see [DATABASE.md](DATABASE.md)).

## Security notes

- `/api/keys/keys.php` serves the public key only; the private keypair (`storage/keys/keypair.json`) goes to admins only
  (`/api/admin/keys`, legacy `generate/keys.php` with the token).
- `/api/validate/`, `/api/code/`, `/api/access/` never return who claimed a code.
- `config/permissions.json` is server-only; clients only see the resulting `permissions`.
- A code is bound to one account (`qr.claimed_by`); codes claimed with the old email-only form can be taken over only
  by an account with that email.
- Grants are limited to the item's `grantable` sources; scan grants need a detecting tool and respect its cooldown.
- The legacy `GENERATE_PASSWORD` token only unlocks the `generate/*.php` endpoints; an empty password locks them.
