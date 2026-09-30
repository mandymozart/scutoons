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

Base `https://api.scutoons.com/api` (target). `https://qr.scutoons.com/api` answers the same during the transition: the
qr app (`../qr`) proxies the whole `/api/*` here byte for byte while its `PROXY_FULL_API` is on (released viewers).
Locally `http://localhost:9000/api` (`VITE_API_URL`; the qr app on :9001). Public on qr forever (qr app): `/?c=`,
`/verify.html`, and – proxied – `/api/validate/`, `/api/keys/keys.php`. The qr app is a **trusted proxy**: with
`X-Scutoons-Proxy-Secret` = `QR_PROXY_SECRET` the API takes the client IP / device id from `X-Scutoons-Client-Ip` /
`X-Scutoons-Device-Id` (rate limits, statistics); without the secret those headers are ignored.

## Endpoints (base `/api`)

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health/` | – | `{ status, db, version, git_sha, built_at }` (`composer.json` version, deploy's `version.json`; locally `git_sha: "dev"`) |
| GET | `/items/` | – | catalog (`config/items.json`); any origin (`*`), cached 5 min |
| GET | `/editions/` (`?toon=`), `/toons/` | – | public lists (`config/editions.json`, `config/toons.json`) |
| GET | `/products/` | – | shop catalog = `config/products.json` byte-for-byte (`{ version:2, currency, checked_at, source, products:[…] }`, type `ProductCatalog` in `@scutoons/content`); any origin (`*`), cached 5 min; other methods 405 |
| GET | `/products/featured/` | – | homepage featured products = `config/featured-products.json` (`[{ product, variant? }]`); same headers |
| POST | `/auth/request/` | – | `{ email, code?, item_id?, variant?, locale?, marketing_opt_in? }` → `{ status:"sent", expires_at }` (always; new accounts only with a signup code) |
| POST | `/auth/verify/` | – | `{ email, otp }` or `{ token }` (+ `marketing_opt_in?`) → session + claims the request's code if claimable · `403 signup_code_required` |
| POST | `/auth/logout/` | Bearer | ends this session |
| GET | `/me/` | Bearer | `{ user, access, memberships, inventory, avatar }` – `user: { id, email, created_at, roles, marketing_opt_in, hide_sensitive }`, `access: { permissions, unlocked, expires_at, codes, signup_code, access_token? }` (owned codes + the valid signup code) |
| PUT | `/me/` | Bearer | `{ marketing_opt_in?: bool, hide_sensitive?: bool }` (at least one) → `{ status, user, avatar }` · `400 invalid_body`. `hide_sensitive` = "Hide sensitive content" (default `true`, agents/ITEMS.md "Sensitive content"); turning it on while the avatar body is a sensitive A-pose look moves the body to the same character's first non-sensitive look (stored; `avatar` in the answer) |
| GET/POST | `/access/` | optional | `{ code }` / `?c=` → device access: `type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token?` (never the owner); per-IP limit; logs `access`; send `X-Device-Id` |
| GET | `/code/?c=CODE` | optional | items/options/edition, `claimable`, `signup` of a code, `claimed_by_me` when signed in; per-IP limit (`&scan=1` from the qr app: logs a `scan`, own per-IP scan limit – see "Rate limits") |
| POST | `/inventory/claim/` | Bearer | `{ code, item_id?, variant? }` |
| POST | `/inventory/craft/` | Bearer | `{ recipe_id }` |
| POST | `/inventory/grant/` | Bearer | `{ item_id, source:"story"\|"scan", ref? }` |
| POST | `/inventory/import/` | Bearer | `{ items:[{ item_id, qty }] }` |
| GET/PUT | `/avatar/` | Bearer | GET → `{ status, avatar: { slots, body, unlocked } }`; PUT `{ slots?:{ <slot instance id>: entryId\|null }, body?: { character, look }\|null }` (at least one) – `body` must be unlocked and have an A-pose image (`422 locked\|no_apose`, `400 invalid_body`) and must not be a look marked `sensitive` while the member's `hide_sensitive` is on (`422 sensitive_hidden { character, look }`; a stored sensitive body then reads as the character's first non-sensitive look); `slots` are keyed by the **slot instances** of the current body (the A-pose look's `slots` in the registry, e.g. `torso-left`/`torso-right` on a duo; a body without slots = one per catalog type, id = type) and GET lists every instance (`null` = empty); `422 invalid_slot` (not an instance of this body, + `slots`) / `wrong_slot` (item type ≠ instance type, + `slot_type`); on a body switch worn items keep the same instance id, else move to the first free instance of their type, else come off; `unlocked` = `[{ character, looks, source }]` (defaults from `AVATAR_DEFAULT_UNLOCKS` first); `body: null` = the first unlocked default |
| GET/PUT | `/progress/<toonId>/` | Bearer | `{ state, updated_at }` last write wins |
| GET | `/library/toons`, `/library/toons/<toon>[/seasons/<n>\|/chapters/<n>\|/cast\|/credits]`, `/events/` | – / creator | the reader's entities composed from the content store – see "Library" |
| GET | `/entities?toon=`, `/entities/<key>[?draft=1\|?version=<id>][&wrap=1]`, `/entities/<key>/versions` | creator | the content store by entity key – see "Entities" |
| PUT | `/entities/<key>` | creator | `{ content, message?, base? }` → new draft version |
| POST | `/entities/<key>/publish`, `/unpublish`, `/restore` | creator | make a version live / nothing live / old version → new draft |
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
| PUT | `/users/{id}/characters/{character}` | unlock an avatar-body character `{ looks?, source?: default\|code\|item\|story\|admin (admin), ref? (admin:<email>) }` → `{ changed, characters }` · `400 unknown_character\|invalid_look\|invalid_source\|invalid_body` · 404 |
| DELETE | `/users/{id}/characters/{character}[/looks/{look}]` | revoke the character (with its looks) or one look → `{ changed, characters }`; a worn body that is no longer unlocked is reset; defaults can't be revoked per account |
| GET | `/events` | activity log (filters code, event list, edition_id, toon, from, to) |
| GET | `/catalog` · `/keys` | items, editions, toons, permissions map (read-only) · the ML-DSA-65 keypair for client-side signing |
| GET | `/content?toon=` | content store overview, every toon: per stored file the newest version (+ status, time), the published version, the version count (read-only; platform P2a) |
| GET/PUT | `/settings` | runtime settings (table `settings`): `GET` → `{ status, items: [{ key, value, default, min, max, stored, updated_at, updated_by }] }`; `PUT { key: int\|null, … }` (or `{ settings: {…} }`) → `{ status, changed, items }` – integer `min…max` (0…1000000), `null` = back to the config default; only changed values are written, `updated_by` = `admin:<email>` · `400 invalid_setting { key, min, max }\|unknown_setting { settings }\|invalid_settings`. Keys: `scan_limit_per_hour`, `lookup_limit_per_hour` (see "Rate limits") |
| GET | `/versions` | the deployed version of every app: `{ status, checked_at, api: { app, version, git_sha, built_at }, apps: [{ app, url, ok, http_status, version?, git_sha?, built_at?, shared?, error? }] }` – each app's `version.json` (`VERSION_URLS`) fetched in parallel, 4 s timeout (platform P3) |

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
`DELETE FROM rate_limits WHERE bucket LIKE 'lookup:%'` or raise the limit (admin app → Settings).

### Rate limits (code lookups, QR scans)

Every attempt counts (also unknown codes – against code enumeration); over the limit → `429 rate_limited
{ retry_after: 3600 }`, and the qr app shows its "Too many codes checked" page (429). Per client IP and hour, in two
buckets of `rate_limits`:

| Bucket | Counts | Limit (runtime setting → config default) |
|---|---|---|
| `lookup:<ip hash>` | direct `/access/` + `/code/` calls (viewer, API) | `lookup_limit_per_hour` → `ACCESS_MAX_PER_IP_HOUR` (120) |
| `qrscan:<ip hash>` | `/code/?scan=1` from the qr app (trusted proxy, forwarded client IP) = QR landing-page scans | `scan_limit_per_hour` → `SCAN_MAX_PER_IP_HOUR` (1000) |

Scans have their own, higher limit because at an event everybody scans over one Wi-Fi (one public IP). `0` = no limit
(nothing is counted). The owner changes both in the admin app (Settings → `PUT /api/admin/settings`); a stored value
applies to the next request. `scan=1` without the proxy secret is an ordinary lookup.

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
- **Events** (`code_events`, append-only, `src/Domain/Events/EventLog.php`): `scan` (landing page: the qr app's `/code/?c=&scan=1`, trusted proxy only, also for expired / revoked codes), `access` (at most
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

## Entities (the content store, creator)

Every toon-related document (toon entry and settings, seasons, chapters, VN scenes, map, cast, credits, characters,
moods, wanted images, people, roles, events) as versions in `entity_versions`, keyed by **entity key** (agents/DATA.md §2):
`toon:<toon>`, `toon-settings:<toon>:<l>`, `season:<toon>:<n>:<l>`, `chapter:<toon>:<n>:<l>`,
`scene:<toon>:<chapter>:<name>:<l>`, `locations:<toon>:<l>`, `routes:<toon>:<l>`, `cast:<toon>`, `credits:<toon>`,
`characters`, `moods`, `asset-requests`, `people`, `roles`, `events`. Every save is a new version; per key at most **one**
version is `published`, the newest is the working copy. Readers use the library (below); everything here needs the role
`creator`.

- **Validation** (minimal, the creator checks the TypeScript types): a valid key (`400 invalid_key`), body JSON object
  or array, `CONTENT_MAX_BYTES` (default 2 MB). JSON is decoded as objects and stored re-encoded (unescaped
  unicode/slashes), so `{}` stays `{}`.

| Method | Path | Body / query → answer |
|---|---|---|
| GET | `/entities?toon=<toon>` | `{ entities: [{ key, kind, toon, locale, latest, published, versions }] }` (`toon=` empty: the platform-wide ones; no `toon`: all) |
| GET | `/entities/<key>` | the published version as raw JSON (`no-store`, `X-Content-Version`/`X-Content-Status`) · 404 when nothing is published |
| GET | same `?draft=1` / `?version=<id>` | the newest version / that version · 404 |
| GET | same `…&wrap=1` | `{ status, version: meta, published: { id, created_at } \| null, content }` |
| PUT | `/entities/<key>` | `{ content, message?, base? }` → `{ status, unchanged, version: meta }`; `base` not the newest → **409 `conflict`** `{ latest: meta }` |
| GET | `/entities/<key>/versions?limit=` | `{ versions: [{ id, status, created_at, author, author_email, message, size }] }` newest first |
| POST | `/entities/<key>/publish` | `{ version? }` (default: newest) → `{ version }` · 404 for a version of another key |
| POST | `/entities/<key>/unpublish` | → `{ unpublished: n }` |
| POST | `/entities/<key>/restore` | `{ id, message? }` → `{ version }` – that version (of this key) as the newest **draft** |

`meta = { id, key, kind, toon, locale, status, created_at, author, author_email, message, size }`. Errors:
`400 invalid_key|invalid_toon|invalid_json|missing_content`, `413 content_too_large`, `422 invalid_content`,
`401 unauthorized`, `403 forbidden` (`role: "creator"`).

**Seeding / interchange** – `php bin/console import-content shared/content/seed [--toon=] [--only=<kinds>] [--db=<sqlite>]
[--update] [--dry-run] [--sql]`, `export-content <dir>` (see [DATABASE.md](DATABASE.md)); the path ↔ key mapping of the
JSON folder layout is `Console\ContentFiles`. Migration `2026-09-30-entity-keys` created `entity_versions` from the
former path-keyed `content_versions` (same ids).

## Library (the reader's entities, agents/DATA.md)

Public GETs composed from the content store's **published** documents (the configuration format of `@scutoons/content`
is the format of every answer). Implementation: `Domain\Library\Library` + `LibraryController`.

| GET | Answer |
|---|---|
| `/library/toons` | `ToonIndex[]`: every `toon:<toon>` entry with `status: published` (creation order) |
| `/library/toons/<toon>?locale=` | `{ entry, toon, seasons: SeasonOverview[], locations, routes }` – the toon layout in one request |
| `/library/toons/<toon>/seasons/<n>?locale=` | `SeasonOverview = { number, season, chapters: ChapterSummary[] }` – listed chapters (readable + coming-soon); a summary is the chapter file without `groups` / `audio` / `status` + the season's `type`, `number`, `status` |
| `/library/toons/<toon>/chapters/<n>?locale=` | the chapter with every `ref` group replaced by its scene file (a missing scene keeps its `ref` group + `X-Content-Missing`) |
| `/library/toons/<toon>/cast[?chapter=<n>]` | `{ registry, cast }`: `characters` reduced to the toon's cast (or the characters the chapter references; legacy ids mapped) + `cast:<toon>`; `null` where missing. Resolve with `resolveCast()` |
| `/library/toons/<toon>/credits` | `{ credits, people, roles }`: `credits:<toon>`, the referenced people (+ `merged_into` targets), the roles · 404 without credits |
| `/library/characters` | the whole character registry (entity `characters`), e.g. the avatar bodies |
| `/events/` | the entity `events` |

- **Locale** `?locale=xx` (default: the toon's `sourceLanguage`): per document the asked locale, else the source
  language, else another language of the toon → `Content-Language` = the served one, `X-Content-Fallback: 1` when it
  differs. Language-neutral answers (cast, credits, events, the list) have no `Content-Language`.
- **Visibility**: index entry + toon settings must be `published` (else `404 not_found`); a chapter's status is its season
  `sequence[].status`: `coming-soon` → `403 coming_soon { chapter: ChapterSummary }`, `draft` or not in any season →
  `403 not_public`. `CONTENT_SHOW_DRAFTS=1` (local development) makes `draft` readable.
- **`?draft=1`** (role `creator`; `401` / `403 forbidden` otherwise): the newest version of every document, no gates,
  `Cache-Control: private, no-store`.
- **Headers**: `Cache-Control: public, max-age=60`, `ETag` (`304` on `If-None-Match`), `Access-Control-Allow-Origin`
  = the allowed origin or `*`, exposed `ETag, Content-Language, X-Content-Fallback, X-Content-Missing`.
- **Server cache** (`CACHE_DRIVER`, `CACHE_PATH`): answers are cached under endpoint + the ids of every published
  version of the toon and of the platform-wide entities – a publish changes the key, so nothing is ever invalidated; drafts are never
  cached. APCu when enabled, else files in `storage/cache/library/` (pruned by age).
- Errors: `400 invalid_toon|invalid_locale|invalid_chapter`, `404 not_found`, `403 coming_soon|not_public`.
- Compare: `tests/entities-compare.php` checks every answer against the interchange files of `shared/content/seed`
  (fresh DB, part of `composer test`; `--report` against production).

## Security notes

- `/api/keys/keys.php` serves the public key only; the private keypair (`storage/keys/keypair.json`) goes to admins only
  (`/api/admin/keys`, legacy `generate/keys.php` with the token).
- `/api/validate/`, `/api/code/`, `/api/access/` never return who claimed a code.
- `config/permissions.json` is server-only; clients only see the resulting `permissions`.
- A code is bound to one account (`qr.claimed_by`); codes claimed with the old email-only form can be taken over only
  by an account with that email.
- Grants are limited to the item's `grantable` sources; scan grants need a detecting tool and respect its cooldown.
- The legacy `GENERATE_PASSWORD` token only unlocks the `generate/*.php` endpoints; an empty password locks them.
