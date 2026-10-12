# API – accounts, codes, items, content, admin

The shared contract (concept, catalog format, every request/response) is `agents/ITEMS.md` in the monorepo – including
"Backend implementation notes" and the "Admin API" section. It must not change without updating that file. This page
describes how the server implements it. Setup: [../README.md](../README.md); code structure:
[ARCHITECTURE.md](ARCHITECTURE.md); settings: [CONFIGURATION.md](CONFIGURATION.md).

Email login (a port of osct `Auth/LoginMail/Mailer/Smtp/Progress`; mails go through the email module, see "Emails" below): a login request mails a 6-digit one-time code and a
magic link; confirming either creates the account (first time, with the `member` role and the starter items) and returns
a session token (Bearer) – and, for the SCUTOONS apps, the same session as the HttpOnly cookie `scutoons_sid`: one
sign-in for every app (see "Single sign-on"). Tokens and codes are only stored as hashes. **New accounts only with a signup code** (a
SCUTOONS product or a promo code) – see "Signup rule".

## Hosts

Base `https://api.scutoons.com/api`. `https://qr.scutoons.com/api` is NOT an alias any more: the qr app's `PROXY_FULL_API`
(the old viewer's transition) is off since 2026-10-05 – only the paths below are proxied there.
Locally `http://localhost:9000/api` (`VITE_API_URL`; the qr app on :9001). Public on qr forever (qr app): `/?c=`,
`/verify.html`, and – proxied – `/api/validate/`, `/api/keys/keys.php`. The qr app is a **trusted proxy**: with
`X-Scutoons-Proxy-Secret` = `QR_PROXY_SECRET` the API takes the client IP / device id from `X-Scutoons-Client-Ip` /
`X-Scutoons-Device-Id` (rate limits, statistics); without the secret those headers are ignored.

## Endpoints (base `/api`)

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health/` | – | `{ status, db, version, git_sha, built_at }` (`composer.json` version, deploy's `version.json`; locally `git_sha: "dev"`) |
| GET | `/items/` | – | item catalog `{ version, slots, items, recipes, albums, stats }` (tables `catalog_items` / `_recipes` / `_albums`; `slots` / `stats` from code – see "Catalog"); any origin (`*`), cached 5 min |
| GET | `/editions/` (`?toon=`), `/toons/` | – | public lists (tables `catalog_editions`, `toons`) |
| GET | `/products/` | – | shop catalog (`{ version:2, currency, checked_at, source, products:[…] }`, type `ProductCatalog` in `@scutoons/content`; table `catalog_products` + meta `products`, the shape of the old `config/products.json`); any origin (`*`), cached 5 min; other methods 405 |
| GET | `/products/featured/` | – | the landing page's editions (`[{ product, variant? }]`, table `catalog_featured`, chosen in the admin app: `/admin/featured`); same headers but `max-age=60` |
| GET | `/stockists/?product=&country=` | – | the public "Where to buy" list (scutoons.com/stores): the Embassy's listed stores (`store`, `record_shop`) carrying ≥ 1 product, public fields only → `{ stores: [{ id, name, type, independent, city, country, address, website, opening, carries: [product id] }], products: { <product id>: <stores> }, countries: [ISO] }` (independent first, then country, city, name; `products` / `countries` over all listed stores, unfiltered) · any origin (`*`), `max-age=300` + ETag (304), 600 requests / IP / hour · `400 invalid_country\|invalid_product` – see "Embassy" → "Stores" |
| POST | `/auth/request/` | – | `{ email, code?, item_id?, variant?, locale?, marketing_opt_in?, dispute? }` → `{ status:"sent", expires_at }` (always; new accounts only with a signup code). `dispute: true` + a product code registered to someone else = an account to file an ownership claim (nothing claimed; `/auth/verify/` answers `dispute: true`; `503 not_available` before migration `2026-10-02-ownership-signup`) |
| POST | `/auth/verify/` | – | `{ email, otp }` or `{ token }` (+ `marketing_opt_in?`) → session + claims the request's code if claimable · `403 signup_code_required`; from an `ALLOWED_ORIGINS` app also the session cookie |
| POST | `/auth/logout/` | session | `{ all?: true }` → ends this session (= every app; `all`: every session of the account) and clears the cookie |
| GET | `/auth/session/` | optional | `{ status, user \| null, via: "cookie" \| "bearer" \| null }` – who is signed in; renews the cookie |
| POST | `/auth/session/` | Bearer | the same session as the cookie (an app moves its stored token once) → `{ status, user, via: "cookie" }` · `400 bearer_required` · `403 csrf_rejected` |
| GET | `/me/` | session | `{ user, access, memberships, inventory, avatar, stats, effects, received }` – `received` = gifts / codes moved here not seen yet (`[{ id, kind: gift\|claim\|admin, item_id, variant, qty, code, from (the giver's email, gifts only), created_at }]`; `[]` before the ownership migration) – `user: { id, email, created_at, roles, marketing_opt_in, hide_sensitive, media_quality }`, `access: { permissions, unlocked, expires_at, codes, signup_code, access_token? }` (owned codes + the valid signup code) |
| PUT | `/me/` | session | `{ marketing_opt_in?: bool, hide_sensitive?: bool, media_quality?: "auto"\|"low"\|"medium"\|"high" }` (at least one) → `{ status, user, avatar }` · `400 invalid_body`. `media_quality` = "Media quality" (default `auto`, agents/DESIGN.md "Media quality"; the reader picks the video file per level). `hide_sensitive` = "Hide sensitive content" (default `true`, agents/ITEMS.md "Sensitive content"; it blurs and mutes slices marked sensitive) |
| GET/POST | `/access/` | optional | `{ code }` / `?c=` → device access: `type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token?` (never the owner); per-IP limit; logs `access`; send `X-Device-Id` |
| GET | `/code/?c=CODE` | optional | items/options/edition, `claimable`, `signup` of a code, `claimed_by_me` when signed in; per-IP limit (`&scan=1` from the qr app: logs a `scan`, own per-IP scan limit – see "Rate limits") |
| POST | `/inventory/claim/` | Bearer | `{ code, item_id?, variant? }` |
| POST | `/inventory/craft/` | Bearer | `{ recipe_id }` |
| POST | `/inventory/grant/` | Bearer | `{ item_id, source:"story"\|"scan", ref?, respawn?, visit? }` – `respawn` (a placed item that reappears, `Domain\Catalog\ItemRespawn`): checked against the item's range, the server appends the period to the ref (`@m<minutes>:<bucket>` / `@v:<visit>`) · `422 respawn_out_of_range\|respawn_not_allowed`, `400 invalid_respawn\|visit_required\|ref_required` |
| POST | `/inventory/import/` | Bearer | `{ items:[{ item_id, qty }] }` |
| POST | `/inventory/gift/` | session | `{ entry_id \| item_id (+ variant), qty?, to: email }` → `{ status, gift: { item_id, variant, qty, code, to, transfer_id }, inventory }` – a code-bound piece moves its whole code · `400 invalid_email\|self_gift\|invalid_qty\|invalid_entry` · `403 not_giftable` · `404 recipient_not_found\|not_found` · `409 worn\|cooldown { retry_after }\|max_reached { max }` · `422 missing` · `429 daily_limit\|rate_limited` · `503 not_available` (see "Ownership, claims, gifting") |
| POST | `/inventory/received/seen/` | session | `{ ids? }` → `{ status, seen }` – marks `/me/ received` notices seen (all without ids) |
| POST | `/claims/` | session | `{ code, statement }` → `201 { status, claim }` – "This is mine" on a code registered to someone else · `400 statement_required\|invalid_code` · `404` · `409 not_registered\|already_yours\|claim_open { claim }` · `422 not_claimable` · `429 rate_limited` |
| GET | `/claims/` | session | `{ status, items: [claim] }` my claims, newest first – `claim = { id, code, status: open\|transferred\|rejected\|withdrawn, statement, files: [{ id, name, type, size }], created_at, decided_at }` (never the owner) |
| POST | `/claims/{id}/files?name=` | session | the raw file as the body (JPEG, PNG, WebP or PDF – checked by its first bytes) → `{ status, claim }` · `400 empty_file` · `409 claim_closed\|too_many_files` · `413 file_too_large` · `415 unsupported_type` |
| POST | `/claims/{id}/withdraw` | session | → `{ status, claim }` · `409 claim_closed` |
| GET/PUT | `/avatar/` | Bearer | GET → `{ status, avatar: { slots, body, unlocked } }`; PUT `{ slots?:{ <slot instance id>: entryId\|null }, body?: { character, look }\|null }` (at least one) – `body` must be unlocked and have an A-pose image (`422 locked\|no_apose`, `400 invalid_body`); the removed `naked` look (and any look still marked `sensitive: true` in the registry) is ignored – a body stored with it reads as the character's default look; every slot may be empty; `slots` are keyed by the **slot instances** of the current body (the A-pose look's `slots` in the registry, e.g. `torso-left`/`torso-right` on a duo; a body without slots = one per catalog type, id = type) and GET lists every instance (`null` = empty); `422 invalid_slot` (not an instance of this body, + `slots`) / `wrong_slot` (item type ≠ instance type, + `slot_type`); on a body switch worn items keep the same instance id, else move to the first free instance of their type, else come off; `unlocked` = `[{ character, looks, source }]` (defaults from `AVATAR_DEFAULT_UNLOCKS` first); `body: null` = the first unlocked default |
| GET/PUT | `/progress/<toonId>/` | Bearer | `{ state, updated_at }` last write wins |
| GET | `/library/toons`, `/library/toons/<toon>[/seasons/<n>\|/chapters/<n>\|/cast\|/credits]`, `/events/` | – / creator | the reader's entities composed from the content store – see "Library" |
| GET | `/entities?toon=`, `/entities/<key>[?draft=1\|?version=<id>][&wrap=1]`, `/entities/<key>/versions` | creator | the content store by entity key – see "Entities" |
| PUT | `/entities/<key>` | creator | `{ content, message?, base? }` → new draft version |
| POST | `/entities/<key>/publish`, `/unpublish`, `/restore` | creator | make a version live / nothing live / old version → new draft · publish / unpublish `403 review_required` when the account needs a review for the toon – see "Reviews" |
| GET/POST | `/reviews`, `/reviews/publish`, `/reviews/return` | creator | review before publish – see "Reviews" |
| GET/POST/DELETE | `/me/tokens`, `/me/tokens/{id}` | session (members) | the member's AI connections (personal tokens `sct_…`) – see "AI connections" |
| GET/POST | `/validate/` | – | PUBLIC URL (printed): legacy, backwards compatible – success for any existing, unexpired, unrevoked code (claimed or not); CORS `*` |
| GET | `/keys/keys.php` | – | PUBLIC URL (printed): the public key of the certificate signatures |
| * | `/admin/*` | admin | the admin API – see "Admin API" |

Errors: `{ status:"error", error:"<code>", message }` (+ `options`, `missing`, `slot`, `retry_after`, `role` where
noted); unknown paths `404 not_found` "Unknown endpoint.". CORS: `ALLOWED_ORIGINS`, with credentials (the session
cookie). "Bearer" / "session" in the table = the Bearer header or the session cookie.

## Single sign-on (2026-10-01)

One session for every app on `*.scutoons.com` (viewer, creator, admin, shop, embassy): signed in on one = signed in on
all (as far as the account's roles allow, each app checks them – `@scutoons/auth` `APP_ROLES`), signed out on one =
signed out on all. Code: `Http\SessionCookie`, `Http\Middleware\Session`, `Http\CurrentSession`, `AuthController`;
browser side `shared/auth` (`@scutoons/auth`).

- **The cookie**: `scutoons_sid=<session token>; Path=/api; Max-Age=<SESSION_TTL_DAYS>; HttpOnly; SameSite=Lax; Secure;
  Domain=scutoons.com` – `Domain` only when the API host is `SESSION_COOKIE_DOMAIN` or below it (api.scutoons.com and,
  during the transition, qr.scutoons.com/api); other hosts get a host-only cookie (localhost: cookies ignore ports, so
  the dev apps on :8000…:13000 share it). `Secure` always except plain http on a loopback host. `Path=/api` keeps it
  off page, asset and CDN requests of the subdomains. The value is a normal session token (one `sessions` row).
- **Set** by `POST /auth/verify/` when the request's `Origin` is in `ALLOWED_ORIGINS` (a form on another site cannot
  sign a browser into its account). The answer still carries `session_token` (old clients); the new apps check the
  cookie with `GET /auth/session/` and then forget the token (they keep it in localStorage only if the browser refused
  the cookie). `GET /auth/session/` renews the cookie's lifetime.
- **Accepted** next to Bearer everywhere; the Bearer header wins when both are sent (old builds keep working unchanged).
- **CSRF**: `SameSite=Lax` keeps the cookie off cross-site subrequests and cross-site POSTs, but all subdomains of
  scutoons.com are *same-site*. So a cookie on `POST`/`PUT`/`DELETE`/`PATCH` counts only when the `Origin` header is one
  of `ALLOWED_ORIGINS`; otherwise the request is anonymous (`user()` = null) and endpoints that need a user answer
  `403 csrf_rejected` (logout too). Public endpoints that only look at the user optionally (`/access/`, the printed
  `/validate/`) keep working for everybody. Reads (`GET`) are left to CORS: a foreign origin never gets
  `Access-Control-Allow-Origin`, so the browser hides the answer. Bearer requests need no Origin (the token is not
  ambient). The apps' requests are JSON with `X-Device-Id`, i.e. CORS-preflighted, which a foreign origin fails anyway.
- **Logout** (`POST /auth/logout/`) deletes the session row and clears the cookie → every app is signed out (the open
  ones notice on the next request or when their tab gets the focus). `{ all: true }` = every session of the account.
- **Migration** (`POST /auth/session/` with Bearer, from an app Origin): a token an app stored before single sign-on
  becomes the cookie (same session, no new row); the app checks `GET /auth/session/` and deletes its copy.
- **qr app**: its proxy forwards only `scutoons_sid` of the browser's cookies (transition: apps on
  `qr.scutoons.com/api`). The QR landing page itself needs no session – "Add to my account" goes to
  `<viewer>/account?code=`, where the viewer has the shared session.
- **Browsers**: first-party cookies (api. and the apps are the same site), so no third-party cookie blocking. Safari ITP
  caps cookies that *JavaScript* writes to 7 days, not HttpOnly server cookies – but it also caps server cookies to 7
  days when the API's IP address differs from the page's (CNAME/IP check, Safari 16.4+): keep api.scutoons.com on the
  same host as the apps (all-inkl) or expect a weekly sign-in on Safari if the apps move to a CDN. Installed iOS
  home-screen apps (PWA) have their own cookie jar: signed in once inside the PWA (the 6-digit code; a magic link from
  the mail opens Safari, not the PWA). localStorage tokens were also wiped by ITP after 7 days without a visit – the
  cookie is the better store.

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
| GET | `/codes/{code}/history` | ownership history (table `item_transfers`), newest first: `{ items: [{ id, kind: claim\|gift\|admin, code, item_id, variant, qty, from, to, by, claim_id, note, created_at }] }` – accounts as `{ id, email }` (admin only) |
| POST | `/codes/{code}/transfer` | `{ to: email \| account id, note, item_id?, variant? }` – move the code to another account outside a claim (kind `admin`); mails the previous and the new owner → `{ code, from, to, entries, history }` · `400 note_required\|invalid_recipient` · `404 not_found\|unknown_user` · `409 already_owner` · `422 not_claimable\|choose_item` |
| GET | `/claims` | ownership claims, list: sort `created_at` (default `-created_at`), `decided_at`, `status`, `code`; filters `status` (open\|transferred\|rejected\|withdrawn), `code`, `user` (claimant or owner at filing); `q` = part of the code or the claimant's email → items `{ id, code, status, claimant, owner, owner_changed, edition_id, edition_number, files, statement (140 chars), created_at, decided_at }` + `open` (count) · `400 invalid_status` |
| GET | `/claims/{id}` | `{ claim: { …, statement, claimant, owner, admin, admin_note, files }, code (= GET /codes/{code}), current_owner, legacy_registration, certificate (the certificate check of the code), history, other_claims }` |
| POST | `/claims/{id}/transfer` · `/claims/{id}/reject` | `{ note }` (required; `item_id?`, `variant?` for a never-registered code with options) → the detail · transfer moves the code to the claimant (mails both owners), reject keeps it (mails the claimant; the note stays internal) · `400 note_required` · `409 claim_closed\|already_owner` |
| GET | `/claims/{id}/files/{fileId}` | an evidence file, inline, `X-Content-Type-Options: nosniff`, sandbox CSP, `no-store` |
| GET/POST | `/batches` | list · create `{ edition_id, count?, start_number?, force? }` (reserves codes, `409 batch_complete`) |
| GET/PUT | `/batches/{filename}` · GET `/batches/{filename}/zip` | manifest · progress · ZIP |
| GET/PUT | `/files/{type}/{filename}` | batch output (svgs, certificates, signatures, batch, zips) |
| GET | `/users` · `/users/{id}` · `/roles` | accounts (emails shown here), detail (incl. `reading`: the member's chapter checkpoints, last read first – `{ toon, chapter, position, finished, group, slice, started_at, read_at }` – and the sizes of their save game; null = none on the server), roles |
| GET | `/reading?toon=` | where the members are (BACKLOG 10.69): per chapter `{ toon, chapter, started, finished, reading, stops[10], last_read_at }` + `readers` (members with a save game; guests are not on the server) · `400 invalid_toon` |
| PUT/DELETE | `/users/{id}/roles/{role}` | grant / revoke |
| PUT | `/users/{id}/characters/{character}` | unlock an avatar-body character `{ looks?, source?: default\|code\|item\|story\|admin (admin), ref? (admin:<email>) }` → `{ changed, characters }` · `400 unknown_character\|invalid_look\|invalid_source\|invalid_body` · 404 |
| DELETE | `/users/{id}/characters/{character}[/looks/{look}]` | revoke the character (with its looks) or one look → `{ changed, characters }`; a worn body that is no longer unlocked is reset; defaults can't be revoked per account |
| GET | `/events` | activity log (filters code, event list, edition_id, toon, from, to) |
| GET | `/proposals?status=open\|applied\|dismissed` | changes an admin's own AI proposed – see "Proposals" below |
| GET | `/catalog` · `/keys` | items, editions, toons, permissions map in one answer (read-only; editing: "Catalog" below) · the ML-DSA-65 keypair for client-side signing |
| GET | `/content?toon=` | content store overview, every toon: per stored file the newest version (+ status, time), the published version, the version count (read-only; platform P2a) |
| GET/PUT | `/settings` | runtime settings (table `settings`): `GET` → `{ status, items: [{ key, value, default, min, max, stored, updated_at, updated_by }] }`; `PUT { key: int\|null, … }` (or `{ settings: {…} }`) → `{ status, changed, items }` – integer `min…max` (0…1000000), `null` = back to the config default; only changed values are written, `updated_by` = `admin:<email>` · `400 invalid_setting { key, min, max }\|unknown_setting { settings }\|invalid_settings`. Keys: `scan_limit_per_hour`, `lookup_limit_per_hour` (see "Rate limits") |
| GET/PUT | `/featured` | the landing page's editions (`Domain\Catalog\FeaturedProducts`, table `catalog_featured`; one `catalog_history` row `replace` per change): `GET` → `{ status, items, updated_at, updated_by, max: 12 }`; `PUT { items: [{ product, variant? }] }` → `{ status, changed, …same }` – products / variants of the shop catalog, no duplicates, ≤ 12, the order is the page's; `[]` = nothing featured (there is no default list since 10.30) · `400 invalid_featured\|unknown_product { product, products }\|unknown_variant { product, variant, variants }\|duplicate_featured` |
| GET | `/emails/templates` | the email module's registry: `{ status, locales: ["de","en"], transport, items: [{ id, audience: member\|admin, description, vars: { name: type }, sample }] }` |
| GET | `/emails/templates/{id}/preview?locale=de\|en` | `{ status, id, locale, subject, text, html }` rendered with the template's sample vars · `404 not_found` · `400 invalid_locale` |
| GET | `/emails/log` | the sent-mail log (table `mail_log`), list: sort `created_at` (default `-created_at`), `template`, `status`, `recipient`; filters `template`, `status` (`sent`\|`failed`), `recipient`, `from`, `to` (`YYYY-MM-DD`, UTC); `q` = part of the address → items `{ id, template, recipient, locale, status, transport, error, created_at }` · `400 invalid_template\|invalid_status\|invalid_date\|invalid_sort`. Recipient addresses are shown here (admin only, like `/users`) |
| GET | `/server-errors` | the newest entries of a PHP error log, newest first (BACKLOG 10.11): `source` = `api` (`LOG_PATH`, default) \| `qr` (`QR_LOG_PATH`, the qr app's `qr/logs/error.log`), `day` (`YYYY-MM-DD`), `q` (text), `limit` (≤ 500, default 200) → `{ source, sources: [{ id, configured }], exists, size, truncated, days, entries: [{ at, day, level, message, detail }] }` (level `fatal`\|`error`\|`warning`\|`notice`) · `400 invalid_source\|invalid_day`. Only the last 512 KB are read; messages only – e-mail addresses, tokens / keys / hashes, passwords and header values are masked, the server's folder is cut from paths |
| GET/PUT | `/emails/recipients` | admin notification addresses (setting `mail_admin_recipients`, default `MAIL_ADMIN_TO`): `GET` → `{ status, items, default, stored, updated_at, updated_by, max: 10 }`; `PUT { items: [email] \| null }` → `{ status, changed, …same }` (lower-cased, deduplicated; `[]` = nobody; `null` = back to the default) · `400 invalid_recipients` |
| GET | `/versions` | the deployed version of every app: `{ status, checked_at, api: { app, version, git_sha, built_at }, apps: [{ app, url, ok, http_status, version?, git_sha?, built_at?, shared?, error? }] }` – each app's `version.json` (`VERSION_URLS`) fetched in parallel, 4 s timeout (platform P3) |

The legacy `generate/` SPA and its static-token endpoints were removed 2026-10-09 (the admin app does all of it). Two
of its URLs stay, public and unchanged, because the verify page and printed certificates use them:
`GET /generate/get-signature.php?code=` and `GET /generate/files/signatures/SIGNATURE-<code>.json`
(`Legacy/SignatureController`).

### Proposals (2026-10-09, migration `2026-10-09-mcp-drafts`)

Changes an admin's own AI proposes (MCP `propose_changes`: catalog fields, calendar entries, an asset swapped in place);
the admin opens `/proposals/<id>` in the admin app, reads before → after and applies them there through the normal admin
API (catalog history + undo, the calendar as a draft) – or dismisses. Nothing changes here but the proposal's status.
Code: `Domain\Admin\Proposals` (table `admin_proposals`). The only admin write a token may do is `POST /proposals`.

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/proposals?status=open\|applied\|dismissed` | `{ status, items: [{ id, title, count, status, author, created_at, … }] }` · `400 invalid_status` |
| POST | `/proposals` | `{ title, changes, calendar_base?, source? }` → `201 { status, proposal, url }` · `400 title_required\|title_too_long\|changes_required\|invalid_change` · `413 too_large` |
| GET | `/proposals/{id}` | `{ status, proposal, url }` · 404 |
| PUT | `/proposals/{id}` | `{ status: applied \| dismissed }` → `{ status, proposal }` · `409 decided` (not open any more) · `400 invalid_status` |

Before the migration the endpoints answer `503 not_available`.

### Catalog (2026-10-04, migration `2026-10-04-catalog` – agents/CATALOG.md)

The catalog lives in database tables, one row per entry with its definition as JSON in the shape of the old
`server/config/*.json` files (`data`), its order (`position`) and lookup columns (`toon`, `item_id`, `edition_id`,
`product_id`, `variant_id`). Production is the catalog; a local database gets it with the production seed
(docs/DATABASE.md "Production seed"), tests from `tests/fixtures/catalog`. An item's respawn rule is its `respawn`
field (edited with `PUT /catalog/items/<id>`).
`slots` and `stats` are code (`Domain\Avatar\AvatarSlots`, `Domain\Reader\ReaderStats` = `@scutoons/content`
`AVATAR_SLOTS`, `READER_STATS`) and are still served in `GET /api/items/`.

`<kind>` = `items` · `recipes` · `albums` · `products` · `editions` · `featured` (`{ product, variant? }`, id
`<product>[:<variant>]`) · `permissions` (data = the description string) · `code-types` (`{ claimable, signup,
grants }`; id `default` = types not listed) · `toons` (`{ id, name, redirectUrl }`) · `meta` (`items` `{ version }`,
`products` `{ version, currency, checked_at, source }`, `permissions` `{ version }`).

| Method | Path | What |
|---|---|---|
| GET | `/catalog/<kind>` | `{ status, kind, items: [{ id, position, data, updated_at, updated_by }] }` in order |
| GET | `/catalog/<kind>/<id>` | `{ status, entry }` · `404 not_found` |
| PUT | `/catalog/<kind>/<id>` | `{ data, note? }` → create (201) or update (200) `{ status, created, changed, entry, history_id }` – validated (`Domain\Catalog\CatalogValidator`: id pattern, `data.id` = the id, required fields, slots / stats, item ↔ product ↔ edition, recipe items, scan_object grants, respawn rules, album ↔ format, featured product / variant (≤ 12), code type grants ↔ permissions, toon id / redirectUrl) · `422 invalid_catalog { problems, kind, id }` · `400 missing_data`. Objects stay objects (`{}`). A new toon also gets its content index entry `toon:<id>` (draft) when the creator has none – it then shows in the creator's toon list |
| DELETE | `/catalog/<kind>/<id>` | `{ status, deleted, history_id }` · `409 in_use { uses }` while anything refers to it (recipes, products, featured, scan grants, items' edition / product / album, code types' grants, codes / inventory rows, a toon's content; meta never) |
| POST | `/catalog/<kind>/reorder` | `{ ids: [...] }` – those first, the rest after them in their order → `{ status, changed, ids, history_id }` · `400 invalid_order` |
| GET | `/catalog/history?kind=&id=&limit=50` | `{ status, items: [{ id, kind, entry_id, action: create\|update\|delete\|reorder\|replace, before, after, position, undo_of, note, by, at }] }` newest first (≤ 500); `entry_id` `''` = the whole kind (reorder / replace) |
| POST | `/catalog/history/<n>/undo` | `{ force? }` – the entry (or the kind's order / list) goes back to that change's `before`; recorded as a change itself (`undo_of`, so an undo can be undone) → `{ status, undone, history_id, entry? / ids? }` · `409 changed_since { current }` (changed again since – undo the later changes first, or `force`) · the same validation / `in_use` checks as a write · `404 not_found` |

Writes refresh the asset usage index (source `catalog`, ref `<kind>:<id>` with the singular kind: `item:camera`,
`product:dago_hoodie`, `album:awakening` …); an asset relink / rename updates the rows (validated, with history).

## Access model: viewing vs. owning

(ITEMS.md "Access model", final, 2026-09-30.)

- **Viewing is device-level.** Anyone holding an existing, unexpired, unrevoked code whose type grants `catalog` may
  read everything on that device – also when the code is owned by someone else. The viewer asks `/api/access/` and
  keeps the answer (and its `access_token`) in the browser. Nothing is stored on the server except the activity
  event. Codes are meant to be shareable (no device limits).
- **Owning is an account action.** Claiming (item in the inventory, edition number), avatar, synced progress need an
  email login. One owner per code applies only here.

### One owner per code (ownership)

1. **API** – `CodeService::claim` runs in a transaction (`SELECT … FOR UPDATE`)
   and binds with `UPDATE qr SET claimed_by = ? … WHERE claimed_by IS NULL`; a second account gets 409
   `already_claimed` (it can still read with the code). Codes claimed by the old email-only form can only be claimed by
   an account with that email (409 `email_required`). The owner claiming again is a no-op (`already_yours`).
2. **Database** – triggers (`2026-09-30-code-owner-guard`) refuse re-assigning `qr.claimed_by` to another account and
   refuse inventory rows whose `code` belongs to someone else, even for writes outside the API. Releasing a code
   (support case) = set `claimed_by` to NULL and delete its inventory rows.

Tests: `CodeServiceTest`, `WardrobeTest::testTwoSignupsAtOnceDoNotBlockEachOther` (InnoDB locking). `/access/`
+ `/code/` share a per-IP limit (120/hour) – against the dev DB use
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

The tables `catalog_permissions` + `catalog_code_types` (server-only, never served to readers; since 2026-10-04 –
until then `config/permissions.json`). `Domain\Access\Permissions::map()` builds the old file's shape:

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
- To change it: the admin API (`PUT /api/admin/catalog/permissions/<id>` with the description, `PUT
  /api/admin/catalog/code-types/<type>` with `{ claimable, signup, grants }` – grants must be known permissions; a
  permission still granted or a type with codes can't be deleted; every change in `catalog_history`, undoable). No
  deploy, no migration. Tests may point `Permissions::useFile()` at a map file instead of the tables.

## Signup rule, promo codes

(ITEMS.md "Roles, signup rule, promo codes", 2026-09-30.)

- **A new account needs a signup code**: a code whose type has `"signup": true` (today `membership` and `promo`),
  existing, not expired, not revoked. A claimable code (membership = a product) must also still be free (or registered
  to this email by the old form) – it is claimed in the same step. `CodeService::signupRefusal()` decides.
- `POST /auth/request/` for an email **without** an account and **without** a usable signup code still answers
  `{ status:"sent" }` (no user enumeration), but the mail (template `signup_without_code`, de/en, texts in
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
creator app and the content API, 4 `liaison` and 5 `ambassador` – the Embassy app and `/api/embassy/*`, 6 `developer` –
the migrations, `/api/dev/*`; above admin: only a developer grants or revokes it, the last one stays – `409
last_developer`) and `user_roles (user_id, role_id, granted_at, granted_by)`. `granted_by`: `signup`,
`migration`, `admin` (legacy SPA), `admin:<email>` (admin API), `cli`. `/me/` and `/auth/verify/` return `user.roles`.

**Content editors – for now creator OR admin** (owner 2026-10-01): one rule, `Roles::CONTENT_EDITORS` = `creator`,
`admin`, checked by `Roles::canEditContent()` / `requireContentEditor()` for every content endpoint (`/entities/*`,
library `?draft=1`); the 403 keeps saying `role: "creator"`. The creator app gates with `@scutoons/auth`
`APP_ROLES.creator` (same two roles). To narrow it later, change those two lists only.

In code (`src/Domain/Roles/Roles.php`): `has($user, 'creator')`, `of($userId)`, `require($user, 'creator')`
(401 / 403 `forbidden { role }`), `grant($userId, $role, $by)`, `revoke($userId, $role)`; routes use the
`RequireRole` middleware. Granting: admin API `PUT|DELETE /api/admin/users/{id}/roles/{role}`, the legacy SPA "Users"
view (`generate/users.php`, `generate/user-roles.php`), or `php bin/console roles grant|revoke|list <email> [role]`.

## Embassy (`/api/embassy/*`, roles `liaison` / `ambassador`)

The partner network app (embassy.scutoons.com, agents/EMBASSY.md). Domain rules: `src/Domain/Embassy/Embassy.php`,
endpoint list: `src/Http/Controllers/EmbassyController.php`. Migration `2026-10-01-embassy` (tables `embassy_orgs`,
`embassy_people`, `embassy_leads`, `embassy_activities`; roles 4 `liaison`, 5 `ambassador`).

- Access: `ambassador` or `admin` = everything; `liaison` = read all, edit organisations and people, work own and
  unowned leads (`403 not_your_lead` otherwise). Import, deleting and assigning a lead to somebody else need an
  ambassador (`403 forbidden { role: "ambassador" }`). No role → `403 forbidden { role: "liaison" }`.
- One lead per organisation × track (`live`, `vinyl`, `booking`, `promo`): `stage` (`research`, `ready`, `contacted`,
  `in_conversation`, `won`, `not_now`, `no`, `dormant`), `owner_id`, `next_at` (YYYY-MM-DD, local date).
- `POST /orgs/{id}/activities` with a `track` moves the lead on (and anything but a note makes the user owner of an unowned lead): `mail_out` → contacted + call in 6 days; `call`
  `reached` / `mail_in` / `meeting` → in conversation + 10 days; `voicemail` / `no_answer` → 3 days (weekend dates move to Monday); `call_back` needs
  `next_at`; `declined` → stage `no`, no date. `stage` / `next_at` in the body override.
- `GET /orgs?quiet=1` (or `quiet=<days>`, default 90): "answered before, quiet now" – organisations with a lead (or an
  entry without a track) that had a reply, a meeting or a reached call, then no entry for that many days, no next date
  ahead, stage not `no`, organisation not do-not-contact. Nothing is ever sent automatically (owner 2026-10-04).
- `GET /today?owner=me|all|none|<id>`: leads due today or earlier (not do-not-contact), with organisation, people and
  the last activity; `unscheduled` = leads in research/ready without a date.
- Duplicates: same normalized name (`Embassy::nameKey`) and same city (or country): `POST /orgs` → `409 duplicate
  { id }` unless `force: true`; `POST /import` merges (fills empty fields, unites lists, adds new people and tracks,
  never overwrites). `dry_run: true` reports without writing.
- Activities (in `GET /orgs/{id}`, `/today`, the log answers): `{ id, org_id, lead_id, track, person_id, kind, outcome,
  note, by_email, at, subject, mail_from, mail_to, message_id }` – the last four only for mails sent from the app / read
  back from the inbox (else `null`). `GET /me` adds `mail: { from, inbox }` (the sender address; `inbox` = the sync is
  configured) and `pace` (see "Pace": `null` = not onboarded yet, or the migration is missing).

### Mail (2026-10-04, migration `2026-10-04-ask-embassy-mail`)

People send one-to-one mails from the app (owner 2026-10-04): from the shared mailbox `EMBASSY_MAIL_FROM`
(hello@scutoons.com, name `EMBASSY_MAIL_FROM_NAME`), its own SMTP account (`EMBASSY_SMTP_USER` / `_PASS`, else
`SMTP_USER` / `_PASS`; server and `MAIL_TRANSPORT` as for every mail – `log` locally). Every Embassy role may send (no
approval step). The member's AI only drafts ("Mail drafts" below) – a token is refused here (`403 not_for_ai`); the
browser calls this when the person clicks Send.
Code: `Domain\Embassy\EmbassyMail`.

**`POST /embassy/orgs/{id}/mail`** `{ to, person_id?, track?, subject, text, reply_to_activity_id?, language?, draft_id? }` →
`201 { status, activity, lead }` (like `POST /orgs/{id}/activities`; `lead` null without a `track`)

- `to` = the organisation's address or one of its people's (case-insensitive) – else `400 invalid_recipient`.
  `person_id` (else the person `to` belongs to) must be one of the organisation's people (`400 invalid_person`).
- Do-not-contact on the organisation, the person or the owner of `to` → `409 do_not_contact` (checked before every send).
- `subject` 1–300 characters (line breaks become spaces; `400 subject_required|subject_too_long`), `text` 1–20000
  (`400 text_required|text_too_long`), `track` one of the tracks (`400 invalid_track`); a liaison on somebody else's
  lead → `403 not_your_lead` – all checked before anything goes out.
- The server always ends the text with the opt-out line (unless the text has it already), German or English: `language`
  (`de` | `en`) if given, else the person's `language`, else German for organisations in DE / AT / CH / LI, else English:
  `—` + "Keine weiteren Nachrichten von uns? Einfach mit „Stopp“ antworten." / "Rather not hear from us? Just reply
  “stop”.".
- Plain text, UTF-8, quoted-printable; `Reply-To` = the sender; our own `Message-ID: <uuid@scutoons.com>`;
  `reply_to_activity_id` = a mail activity (`mail_in` / `mail_out`) of this organisation → `In-Reply-To` / `References`
  (`400 invalid_reply_to` otherwise).
- Then: a `mail_log` row (template `embassy`, admin app → Emails), the `mail_out` activity (note = the text as sent,
  `subject`, `mail_from`, `mail_to`, `message_id`, by the user; cadence as above: contacted + call in 6 days) and – when
  `EMBASSY_IMAP_SENT` is set – a copy in that IMAP folder (best effort; a failure there never fails the send).
- The transport refused it → `502 mail_failed` (message = the short reason), logged as `failed` in `mail_log`, nothing on
  the organisation. Before the migration the mail is sent and logged without the mail fields.

### Mail drafts (2026-10-09, migration `2026-10-09-mcp-drafts`)

The member's own AI (MCP `draft_mail`) saves a draft; the member opens its link, edits and sends it (agents/MCP.md
"Draft → link"). Code: `Domain\Embassy\EmbassyDrafts` (table `embassy_mail_drafts`). Every Embassy member sees an
organisation's drafts; only the author sends one; the author or an ambassador discards it. Nothing is sent from here.

| Method | Path | Notes |
| --- | --- | --- |
| POST | `/embassy/orgs/{id}/drafts` | `{ to, person_id?, track?, subject, text, reply_to_activity_id?, source? }` → `201 { status, draft, url }` – checked like a send (recipient of the organisation, do-not-contact, subject, text, track, reply-to: the same errors as "Mail"), never sent; `url` = `<EMBASSY_URL>/orgs/<org>?draft=<id>`; `source` `agent` \| `person` |
| GET | `/embassy/drafts` | `{ status, items }` – every open draft |
| GET | `/embassy/drafts/{id}` | `{ status, draft, url }` · 404 |
| DELETE | `/embassy/drafts/{id}` | `{ status }` – its author or an ambassador (`403 forbidden`) · 404 |
| GET | `/embassy/orgs/{id}` | also returns `drafts` (the organisation's open drafts) |
| POST | `/embassy/orgs/{id}/mail` | with `draft_id`: only the draft's author (`403 forbidden`; `400 invalid_draft` for another organisation's draft); the draft is deleted once the mail went out |

Before the migration the draft endpoints answer `503 not_available`.

### Inbox (2026-10-04, migration `2026-10-04-ask-embassy-mail`)

Replies are read back by IMAP from the INBOX of the hello@ mailbox – the same login (`EMBASSY_SMTP_USER` / `_PASS`) on the
same server (`SMTP_HOST`); `EMBASSY_IMAP_*` only override that for an unusual mailbox (`Domain\Embassy\EmbassyInbox`,
`Infrastructure\Mail\Imap` – no ext-imap needed; `bin/console embassy:inbox` for a cron job). Where it stopped is kept
per folder in `embassy_mail_state` (UIDVALIDITY + last UID; a new UIDVALIDITY starts over, already logged mails are
not logged twice). The first run reads the last 30 days; at most 200 mails per run.

- Matching: In-Reply-To / References name one of our `mail_out` mails → its organisation, track and person (the sender
  when they are another person of that organisation); else the sender's address is a person's or an organisation's →
  that one, track = the organisation's latest `mail_out`. **Unknown senders are skipped and nothing about them is
  stored** (only counted); so are our own address, automatic replies (`Auto-Submitted`, `X-Autoreply`, `Precedence:
  bulk|list|junk|auto_reply`) and bounces (`mailer-daemon@`, `postmaster@`). A Message-ID already logged → skipped.
- A match → a `mail_in` activity (by `inbox`, no user; `at` = the mail's Date in UTC; `subject`, `mail_from`,
  `message_id`, `in_reply_to`; note = the reply text without the quoted mail – `>` lines, "On … wrote:", "Am …
  schrieb …:", "-----Original Message-----", Outlook's "Von: / Gesendet:" block – at most 4000 characters). The cadence
  moves the lead to in conversation + 10 days: the sequence stops. No lead is created by the sync.
- First line of the reply just `stop` / `stopp` / `unsubscribe` / `abmelden` (any case, punctuation ok) → do-not-contact
  on the person (or, from the organisation's address, on the organisation + lead stage `no`), no next date, noted in the
  activity. Respected forever: every later send answers `409 do_not_contact`.

| Method | Path | Body → answer |
|---|---|---|
| GET | `/embassy/inbox` | `{ status, configured, synced_at, error }` (`synced_at` ISO UTC of the last attempt, `error` its error or null) |
| POST | `/embassy/inbox/sync` | `{ force? }` → `{ status, configured: true, synced_at, matched, skipped, new_activities, error, throttled }` – runs unless the last run is younger than 2 minutes (then `throttled: true`, counts 0); `force: true` skips that for ambassadors / admins. Not configured (no `EMBASSY_IMAP_HOST`) → `{ status, configured: false }`. A mailbox error is answered in `error` (200) and stored. `503 not_available` before the migration |

### Pace (2026-10-05, migration `2026-10-05-embassy-pace`)

The Embassy work is done by part-time liaisons at their own pace (owner 2026-10-05). Each person sets how many contacts a
week they take on and a reminder mail **to themselves**; every 4 weeks the app asks whether the pace is right.
Ambassadors / admins see everyone's pace, answers and numbers. Nothing here ever mails a contact. Code:
`Domain\Embassy\EmbassyPace` (pace, portion, team), `Domain\Embassy\EmbassyReminders` (cron + mails). Tables
`embassy_pace` (one row per person: `per_week`, `reminder`, `onboarded_at`, `asked_at`, `reminded_at`) and
`embassy_checkins` (every answer with the pace before / after). Before the migration: `GET /me` says `pace: null`, every
endpoint below answers `503 not_available`, the cron URL runs only the inbox sync (`reminders: null`).

`pace` = `{ per_week, reminder, onboarded_at, asked_at, checkin_due }` – `per_week` 1–100 (preset 10), `reminder`
`weekly` (Monday mail, the preset) | `daily` (every working day Mon–Fri) | `off`, times ISO UTC, `checkin_due` = now ≥
(`asked_at` ?? `onboarded_at`) + 28 days. `null` = not onboarded: the app shows the onboarding.

| Method | Path | Body → answer |
|---|---|---|
| PUT | `/embassy/pace` | `{ per_week?, reminder? }` → `{ status, pace }` – the first call is the onboarding (`onboarded_at` = `asked_at` = now; missing values = 10 / `weekly`), later calls change what is given. `per_week` a whole number 1–100 (`400 invalid_pace { field, min, max }`), `reminder` (`400 invalid_reminder { field, allowed }`) |
| POST | `/embassy/pace/checkin` | `{ answer: less \| fine \| more }` → `{ status, pace }` – per week −5 / 0 / +5 (floor 1, cap 100), `asked_at` = now, the answer stored in `embassy_checkins`. `400 invalid_answer`, `409 not_onboarded` |
| GET | `/embassy/portion?owner=me\|<user id>` | the person's weekly portion (below). Somebody else's only for ambassadors / admins (`403 forbidden { role }`; unknown user `404`) |
| GET | `/embassy/team/pace` | ambassadors / admins (`403 forbidden` otherwise) → `{ status, items: [{ user: { id, email, access }, pace \| null, owned, weeks: [{ start, done }] (the last 4 weeks, oldest first, this week last), last_checkin: { at, answer, per_week_before, per_week_after } \| null }] }` – everyone with an Embassy role, by email |
| GET, POST | `/embassy/cron?key=…` | the host's daily job (no session) → `{ status, inbox, reminders }` (below) |

**The weekly portion** (`GET /embassy/portion`):

```
{ status, today: "YYYY-MM-DD",
  week: { start: "YYYY-MM-DD" (Monday), end (Sunday), pace, done, left },
  today_count, today_done,
  items: [{ group: "due" | "quiet" | "first", hint: { kind, … }, lead, org, people, last_activity }],
  more: { due, quiet, first },
  owned }
```

- The week is Monday–Sunday in the API's time zone (like `next_at`). `pace` = the person's `per_week` (10 when not
  onboarded). `done` = distinct organisations the person contacted this week (`mail_out`, `call` of any outcome, `dm`,
  `meeting` logged by them – mails sent from the app count; notes and replies don't). `left` = max(0, pace − done).
- `items`: at most `left`, **one per organisation** (an organisation with several leads shows once, with its most urgent
  lead), never a do-not-contact organisation, only the person's own leads, in this order: (1) `due` – next date today or
  earlier, oldest first; (2) `quiet` – "answered before, quiet now" (the `quiet` list filter, 90 days); (3) `first` –
  stage research / ready, never contacted (no mail_out / call / dm / meeting on the lead), no date ahead; tier A first.
  `lead`, `org`, `people` (those that may be contacted), `last_activity` = the same shape as `GET /today`'s items.
  `more` = what did not fit this week, per group (counted once per organisation).
- `today_count` = how many of `items` (from the top) are today's slice: what was left this morning (pace − done before
  today), spread over the working days left incl. today (Mon–Fri; ceil), minus what was done today (`today_done`) –
  so the slice shrinks as the person works and does not refill the same day. Saturday / Sunday: everything left. Capped
  at the number of items.
- `owned` = all leads the person owns (0 → the app says "add your first contacts, or take unowned ones").
- `hint` – why the item is on the list, for the app to say in de / en (the reminder mail renders the same):

| `kind` | fields | when |
|---|---|---|
| `due` | `date` (the next date) | due, no more specific reason (no activity yet, a reached call, a meeting, a DM, a note) |
| `mail_no_answer` | `days`, `subject?` | the last entry is our mail (`subject` when sent from the app) |
| `voicemail` | `days` | the last entry is a call → voicemail |
| `no_answer` | `days` | the last entry is a call → no answer |
| `call_back` | `date` | the last entry is a call → call back on `date` |
| `replied` | `days`, `subject?` | the last entry is their reply |
| `answered_then_quiet` | `date`, `days` | group `quiet`: their last reply / meeting / reached call |
| `no_contact_yet` | – | group `first` |

`days` = whole days from that entry's local date to today.

**Reminders + cron.** `GET|POST /api/embassy/cron?key=<EMBASSY_CRON_KEY>` – or the key as the HTTP Basic password (the
host's cron calls a URL and cannot set headers; compared in constant time). `EMBASSY_CRON_KEY` empty → `404` (off);
missing / wrong key → `403 forbidden`. All-inkl: KAS → Tools → Cronjobs → URL
`https://api.scutoons.com/api/embassy/cron?key=…`, daily at 07:00. Locally: `php bin/console embassy:cron` (same job).
It runs (a) the inbox sync (`inbox` = like `POST /inbox/sync`, no throttle; `null` when no IMAP host is configured) and
(b) the due reminder mails: `reminders: { sent, skipped, failed }` (`null` before the migration).

- `weekly`: on Monday – or the first run of the week when Monday's was missed; `daily`: Mon–Fri; `off`: never. At most
  one mail per local day (daily) / week (weekly) (`reminded_at`; a failed send is tried on the next run). Only people
  who still have an Embassy role. No mail when the portion has no items and no check-in is due (`skipped`).
- The mail: template `embassy_portion` (admin app → Emails; from `MAIL_FROM`, in the account's language de / en, a
  `mail_log` row): "This week: X of Y contacts done", the items as "• organisation · person – hint" (weekly: the week's
  items; daily: today's slice), "+N more if you have time", the check-in line when due, a button to the Today page
  (`EMBASSY_URL`), and how to change the reminder (on Today).

### Stores (2026-10-02, migration `2026-10-02-embassy-stores`)

Organisation type `store` (new) and `record_shop` are **stockists** (`Embassy::STOCKIST_TYPES`): they sell to readers.
Fields on `embassy_orgs` (in every organisation answer, edited with `POST/PUT /orgs`): `listed` (bool, "show on
scutoons.com"), `independent` (bool, small / independent vendor – listed first), `address`, `opening` (free text ≤ 255),
`carries` (product ids of the shop catalog; unknown → `400 invalid_product { allowed }`; the import unites them).
`GET /embassy/me` adds `stockist_types`. The public read is `GET /api/stockists/` (`Domain\Embassy\Stockists`,
endpoint table above): only `listed` stockists with at least one product still in the catalog, only `id, name, type,
independent, city, country, address, website (http(s) only), opening, carries` – never people, email, phone, notes, tags,
tier, source, leads or activities. Before the migration ran it answers an empty list. Local fake data:
`php bin/console sample-stores [--remove]` (local dev database only).

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
version is `published`, the newest is the working copy. Readers use the library (below); everything here needs a
content editor: role `creator` – or, **for now** (owner 2026-10-01), `admin` (`Roles::CONTENT_EDITORS`, see "Roles");
otherwise `401` / `403 forbidden { role: "creator" }`.

- **Validation** (minimal, the creator checks the TypeScript types): a valid key (`400 invalid_key`), body JSON object
  or array, `CONTENT_MAX_BYTES` (default 2 MB). JSON is decoded as objects and stored re-encoded (unescaped
  unicode/slashes), so `{}` stays `{}`.

| Method | Path | Body / query → answer |
|---|---|---|
| GET | `/entities?toon=<toon>` | `{ entities: [{ key, kind, toon, locale, latest, published, versions, latest_at?, latest_message?, latest_author? }] }` (`toon=` empty: the platform-wide ones; no `toon`: all; `latest_*` only for a key whose newest version is not published – the creator's "publish down the tree", BACKLOG 10.60) |
| GET | `/entities/<key>` | the published version as raw JSON (`no-store`, `X-Content-Version`/`X-Content-Status`) · 404 when nothing is published |
| GET | same `?draft=1` / `?version=<id>` | the newest version / that version · 404 |
| GET | same `…&wrap=1` | `{ status, version: meta, published: { id, created_at } \| null, content }` |
| PUT | `/entities/<key>` | `{ content, message?, base? }` → `{ status, unchanged, version: meta }`; `base` not the newest → **409 `conflict`** `{ latest: meta }`; a chapter / scene that adds placements of a catalog item over its `placements` (every toon together) → **422 `item_limit`** `{ problems }` (BACKLOG 10.39) |
| GET | `/catalog/items/<id>/placements` | (creator) `{ status, item, limits: { placements, max_world }, placements: [{ key, toon, count }], total, owners }` – where the item is placed (each chapter / scene with the more of its newest and its published version; neutral documents only) and how many accounts own it · 404 `unknown_item` |
| GET | `/entities/<key>/versions?limit=` | `{ versions: [{ id, status, created_at, author, author_email, message, size }] }` newest first |
| POST | `/entities/<key>/publish` | `{ version? }` (default: newest) → `{ version }` · 404 for a version of another key |
| POST | `/entities/<key>/unpublish` | → `{ unpublished: n }` |
| POST | `/entities/<key>/restore` | `{ id, message? }` → `{ version }` – that version (of this key) as the newest **draft** |

`meta = { id, key, kind, toon, locale, status, created_at, author, author_email, message, size }`. Errors:
`400 invalid_key|invalid_toon|invalid_json|missing_content`, `413 content_too_large`, `422 invalid_content`,
`401 unauthorized`, `403 forbidden` (`role: "creator"`).

**Seeding / interchange** – a local database gets production's content from the production seed (docs/DATABASE.md);
`php bin/console import-content <folder> [--toon=] [--only=<kinds>] [--update] [--dry-run] [--sql]`, `export-content <dir>` (see [DATABASE.md](DATABASE.md)); the path ↔ key mapping of the
JSON folder layout is `Console\ContentFiles`. Migration `2026-09-30-entity-keys` created `entity_versions` from the
former path-keyed `content_versions` (same ids).

## Flow index (the creator's SCUTOONS-wide Flow, 2026-10-06)

`GET /api/creator/flow-index` (role creator, `Domain\Content\FlowIndex`) – from the NEWEST version of every chapter /
scene / season / toon document (drafts included; a legacy per-locale chapter only when it has no neutral document;
ref groups read through their scenes):
`{ toons: [{ id, title, status, seasons: [{ number, chapters: [{ number, status }] }] }], edges: [{ source: { toon,
chapter, group }, target: { type, value, label?, anyOf? }, kind: 'jump' | 'choice' | 'end', label? }], dead: { <toon>:
{ unread, unset, points, labels } } }`. `edges` = every jumpTo event, choice that jumps and authored group end (group
scripts, slice triggers, item taps); `dead` = @scutoons/content `deadThings`' rules (variables set but never read / read
but never set, script points and group labels nothing jumps to). Cached under the version ids it was built from (the
library's cache, nothing to invalidate).

## AI connections (personal access tokens, 2026-10-09 – migration `2026-10-09-api-tokens`)

A member's own AI (Claude Desktop / Claude Code running the SCUTOONS MCP server, tools/mcp/README.md, agents/MCP.md §3)
signs in with a personal token from the account page ("AI connections"). Code: `Domain\Auth\ApiTokens`,
`ApiTokensController`, `Http\ApiTokenScope`.

- `Authorization: Bearer sct_…` – taken like a session (`CurrentSession`): the account's CURRENT roles, so every role
  check applies unchanged. Only the SHA-256 is stored; the token is shown once. `last_used_at` at most once a minute.
- **Members only** (owner 2026-10-09): creating one needs creator / liaison / ambassador / admin (`403 no_member_role`);
  a token stops working (401) once its account has none of those roles left. At most 20 per account (`409 too_many`).
- **Refused for a token** (`403 not_for_ai` – the member does it in the app): `POST /entities/<key>/publish|unpublish`,
  `POST /reviews/publish|return`, `POST /embassy/orgs/{id}/mail`, `DELETE /embassy/orgs/{id}`, `POST /embassy/import`,
  every `/admin/*` call but GET / HEAD and `POST /admin/proposals`, `POST /auth/session|logout`, `/me/tokens*`.
- A save through a token is signed "(AI: <connection name>)" in the version message.
- **Six months** (owner 2026-10-09, migration `2026-10-09-tokens-expire`): `expires_at` = made / last renewed + 182 days;
  expired → 401. Renewing (`POST /me/tokens/{id}/renew`, a real sign-in) moves the SAME token six months on – nothing to
  set up again. Used within its last 14 days, its member gets ONE mail per period (`ai_token_expiring`, `reminded_at`) –
  no cron: a token nobody uses needs no reminder.
- Before the migration the endpoints answer `503 not_available`.

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/me/tokens` | `{ status, member, items: [{ id, name, created_at, last_used_at, expires_at, expired }] }` – `member`: the account may have tokens (readers can't; `items` empty then) |
| POST | `/me/tokens` | `{ name }` → `201 { status, token, item }` – `token` only here · `400 name_required` · `403 no_member_role` · `409 too_many` |
| POST | `/me/tokens/{id}/renew` | → `{ status, item }` – six more months from now (also an expired one) |
| DELETE | `/me/tokens/{id}` | → `{ status }` – revoked: the next call with it answers 401 |

A real session only (cookie or a session Bearer); CORS allows DELETE there.

## Developer API (`/api/dev/*`, role `developer` – BACKLOG 10.61)

The migrations through the API instead of phpMyAdmin (docs/DATABASE.md "Migrations"): a session or the developer's AI
connection (their agents over MCP). Only the files that came with the deploy run – never SQL from a request.
`DevController`, `Domain\Dev\Migrations`.

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/dev/migrations` | `{ status, database, base_schema, pending, files: [{ name, applied_at, sha256 }], runs: [the latest 10] }` |
| POST | `/dev/migrations/plan` | the dry run, stored → `201 { status, run }` · `409 busy` · `409 no_schema` |
| GET | `/dev/migrations/runs/{id}` | `{ status, run }` – `run.plan`: per file `{ name, sha256, kind, dry_run, error, statements: [{ sql, kind, rows, skipped, backup }] }` |
| POST | `/dev/migrations/runs/{id}/apply` | → `{ status, run }` with `run.status` applied / failed, `log`, `error`, `backup` · `409 not_planned` · `409 plan_stale` (older than a day, or the pending files changed) · `409 busy` |
| POST | `/dev/migrations/run` | plan + apply in one go → `{ status, run }`; `run: null` = nothing pending |
| POST | `/deploy/migrate` | **no session**: header `X-Migrate-Key: <MIGRATE_KEY>` (the deploy) – like `/dev/migrations/run`; HTTP 500 when the run failed · `403` wrong key · `404` MIGRATE_KEY not set |

## Reviews (review before publish, 2026-10-04 – migration `2026-10-04-reviews`)

Owner 2026-10-04 (agents/CUTOVER.md 10.28 "Review levels"): who may publish without a review is **not a role** but a
relation per toon, set by an admin like GitHub reviewers (`content_review_rules`, `toon` = a toon id or `*` = every toon):

- `needs_review` – the account may save drafts, restore and submit, but `POST /entities/<key>/publish|unpublish` of
  that toon answers **`403 review_required { toon }`**. Without the rule an account publishes directly (as before).
- `reviewer` – gets the `review_requested` mail, publishes a submitted version or sends it back. Never their own submit.

Reviewed levels: every **toon-level** entity (a toon in its key: `toon`, `toon-settings`, `season`, `chapter`, `scene`,
`cast`, `credits`, `script`, …). Platform-wide entities (`characters`, `moods`, `people`, `roles`, `events`, `map`,
`asset-requests`) publish directly for every creator. The API decides, so the creator app and agents (the MCP server
calls with the creator's token) get the same answer. `Domain\Content\Reviews`.

A submit (`content_reviews`) is **pinned to one version**: the reviewer publishes exactly that version, newer drafts stay
drafts (`newest` > `version` shows them). A newer submit for the same key **annuls** the open one. Several files go in one
call (a chapter = every language + its scenes) and every person gets one mail per call.

| Method | Path | Body → answer |
|---|---|---|
| GET | `/reviews` | `{ needs_review: [toon\|*], reviewer_of: [toon\|*], to_review: [review], mine: [review] }` – to_review = open submits of the toons this account reviews (not its own), mine = its last 50 |
| POST | `/reviews` | `{ items: [{ key, version? }], note? }` (version default the newest) → `{ reviews, reviewers: [email] }`; mails `review_requested` to each reviewer · `400 invalid_items` · `404 not_found` · `409 already_published` (that version is live) · **`409 no_reviewer { toon }`** (nobody reviews that toon – ask an admin) · `422 review_not_needed` (platform-wide) |
| POST | `/reviews/publish` | `{ ids, note? }` → `{ reviews, keys }` – publishes each pinned version; mails `review_published` to each author |
| POST | `/reviews/return` | `{ ids, note? }` → `{ reviews, keys }` – sends them back (entities unchanged); mails `review_returned` |

`/reviews/publish|return`: `400 invalid_ids` · `404 not_found` · `409 review_closed { state }` · `403 not_reviewer { toon }`
(not a reviewer of that toon, or the own submit); all-or-nothing. A reviewer who publishes the submitted version through
`/entities/<key>/publish` closes the review too (no mail). `review = { id, key, toon, locale, version, newest, state
(pending | published | returned | annulled), note, author: { id, email }, created_at, decided_by, decided_at, decision_note }`.

Admin (`/api/admin`): `GET /review-rules` → `{ items: [{ toon, rule, user: { id, email }, created_at, created_by }] }`,
`PUT|DELETE /users/{id}/review-rules/{toon}/{rule}` → `{ changed, review_rules }` (`400 invalid_toon|invalid_rule`);
`GET /users/{id}` includes `review_rules`. Mails name the toon (its catalog name) and link to the creator dashboard `CREATOR_URL/` ("To review" / "Your submissions").

## Library (the reader's entities, agents/DATA.md)

Public GETs composed from the content store's **published** documents (the configuration format of `@scutoons/content`
is the format of every answer). Implementation: `Domain\Library\Library` + `LibraryController`.

| GET | Answer |
|---|---|
| `/library/toons` | `ToonIndex[]`: every `toon:<toon>` entry with `status: published` + every unpublished one whose toon settings say `announce: true` (marked `announced: true`; its pages stay 404) (creation order); each with the settings' `tile` when set. Drafts only with `CONTENT_SHOW_DRAFTS` / `?draft=1`; the dev-only sandbox never publicly |
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
- **`?draft=1`** (a content editor: `creator`, for now also `admin`; `401` / `403 forbidden` otherwise): the newest version of every document, no gates,
  `Cache-Control: private, no-store`.
- **Headers**: `Cache-Control: public, max-age=60`, `ETag` (`304` on `If-None-Match`), `Access-Control-Allow-Origin`
  = the allowed origin or `*`, exposed `ETag, Content-Language, X-Content-Fallback, X-Content-Missing`.
- **Server cache** (`CACHE_DRIVER`, `CACHE_PATH`): answers are cached under endpoint + the ids of every published
  version of the toon and of the platform-wide entities – a publish changes the key, so nothing is ever invalidated; drafts are never
  cached. APCu when enabled, else files in `storage/cache/library/` (pruned by age).
- Errors: `400 invalid_toon|invalid_locale|invalid_chapter`, `404 not_found`, `403 coming_soon|not_public`.

## Ownership, claims, gifting (2026-10-02)

Contract and rules: agents/ITEMS.md "Ownership, claims, gifting". Code: `Domain\Ownership\OwnershipTransfers` (the
transfer core + history + admin transfer + `received` notices + the two ownership mails), `OwnershipClaims`,
`Gifts`. Tables (migration `2026-10-02-ownership`): `ownership_claims`, `ownership_claim_files` (files under
`FILES_PATH/claims/`, never served directly), `item_transfers`; `login_requests.dispute` (`2026-10-02-ownership-signup`).

- **Transfer core** (`moveCode`, one transaction): the owner guard `qr_owner_immutable` forbids A → B in one UPDATE,
  so the code goes A → NULL → B; every inventory row with that code moves (source `qr`, `gift` for gifts), the
  previous owner's avatar slots wearing them are cleared, `qr.email` / `claimed_at` follow. A never-registered code
  (or one registered by the old email form) is claimed for the new owner instead (its items are added). The previous
  owner keeps reading with the code (device access is not ownership).
- **A deleted account** (`DELETE FROM users`; no API for it): the trigger `users_release_codes` (migration
  `2026-10-07-account-deletion-frees-codes`) frees its codes (`claimed_by` NULL, `email` '') and withdraws its open
  claims; the foreign keys remove its inventory. The next person claims the code as usual (ITEMS.md "Item limits").
- **Before the migrations ran** (production runs the code first): `GET /me/` (`received: []`), the code history and
  the admin claims list keep working; gift / claim / transfer answer `503 not_available`; a `dispute` sign-in too.
- Settings: `CLAIM_MAX_PER_DAY` (5), `CLAIM_FILES_MAX` (5), `CLAIM_FILE_MAX_BYTES` (5 MB), `GIFT_MAX_PER_DAY` (10),
  `GIFT_LOOKUPS_PER_DAY` (30), `GIFT_COOLDOWN_HOURS` (24), `ADMIN_URL` (link in the admin mail).

## Emails (the email module, 2026-10-02)

The API is the only sender of email (the qr app, the frontends and scripts send none). Every platform mail goes through
`Domain\Mail\Mailer::send($templateId, $to, $locale, $vars)`; the one exception are the mails people write in the Embassy
app (`Domain\Embassy\EmbassyMail`, "Embassy" → "Mail": same transport, logged in `mail_log` as template `embassy`):

- **Registry** `Domain\Mail\MailTemplates` – template ids with audience, description, variables, sample values (the
  admin preview), the body as blocks (`p`, `button`, `code`, conditional with `if`) and the fine print. Registered:
  `login_code` (sign-in link + one-time code), `signup_without_code` (no account and no usable signup code), and the
  ownership mails (agents/ITEMS.md "Ownership, claims, gifting"): `ownership_claim_received` (claimant),
  `ownership_claim_admin` (admins), `ownership_claim_rejected` (claimant), `ownership_transferred_from` /
  `ownership_transferred_to` (previous / new owner), `gift_received` (recipient).
- **Texts** `templates/mail/texts.php` – `[locale][template id][key]`, de + en, `{var}` placeholders, `subject`
  required; `_layout.signature`. **HTML frame** `templates/mail/layout.html.php` (one layout for every mail).
  `MailRenderer` builds subject, text and HTML from the same blocks.
- **Transport** `Infrastructure\Mail\Transport` (`MAIL_TRANSPORT` = `smtp` | `log` → `MAIL_LOG_PATH` | `mail`); the log
  file format is unchanged (the contract suites and `scripts/dev/test-user.mjs` read one-time codes there).
- **Sent-mail log** table `mail_log` (`MailLog`): template, recipient, locale, `sent` | `failed`, transport, error,
  time – never the subject, text or variables. Kept `MAIL_LOG_DAYS` (180). A failed send is logged and throws
  `MailFailed` (the login request answers `502 mail_failed` as before).
- **Admin recipients** `MailRecipients` (setting `mail_admin_recipients`, default `MAIL_ADMIN_TO`) for
  `Mailer::sendToAdmins($templateId, $vars)` in `MAIL_ADMIN_LOCALE`.
- Admin app → **Emails** (`/emails` log, `/emails/templates` previews + recipients); admin API above.

## Newsletter (2026-10-07, migration `2026-10-07-newsletter` – agents/NEWSLETTER.md)

Mass mails to members and contacts, sent in batches through the SMTP account (`Domain\Newsletter`). Not a
`MailTemplates` template: the text is written in the admin; same HTML frame, logged in `mail_log` as `newsletter`.

Public (no session):

| Method | Path | |
|---|---|---|
| GET | `/newsletter/unsubscribe?t=<token>` | HTML page in the token's language with one "Unsubscribe" button (a GET changes nothing – mail scanners open links); invalid token → a page that says so |
| POST | `/newsletter/unsubscribe?t=<token>` | unsubscribes (`mail_suppressions` reason `unsubscribe`, a member's `marketing_opt_in` → 0) → HTML "done"; also the mail client's one-click (`List-Unsubscribe-Post`, RFC 8058); 400 for an invalid token |
| GET / POST | `/newsletter/cron?key=<NEWSLETTER_CRON_KEY>` | one batch → `{ result }` (below); key also as the HTTP Basic password; empty key → 404, wrong → 403 |

Token = `base64url("<locale>|<email>") . "." . base64url(HMAC-SHA256 truncated to 16 bytes)`, key derived from
`AUTH_SECRET` (label `newsletter-unsubscribe`); nothing stored, never expires.

Admin (`/api/admin/newsletter/*`, role `admin`):

| Method | Path | |
|---|---|---|
| GET | `status` | `{ transport, cron, per_hour, sent_last_hour, queued }` |
| GET | `audiences[?selected=a,b]` | `{ items: [{ id, kind, source, count, missing_locale }], selected?: { count, missing_locale, locales } }` – ids `members_opted_in`, `members_all`, `contacts`, `contacts:<source>`; counts without suppressed / invalid addresses, each address once |
| GET | `contacts?q=&source=&locale=de\|en\|none&sort=&page=&page_size=` | list + `sources: [{ source, count, missing_locale }]`; a row: `id, email, name, locale, source, consented_at, created_at, created_by, member, suppressed` |
| POST | `contacts/import` | `{ text, source, consented_at?, locale?, dry_run? }` → `{ rows: [{ line, email, name, locale, status }], counts, added }` – status `new` · `exists` · `member` · `suppressed` · `duplicate` · `invalid`; only `new` is added |
| PUT | `contacts` | `{ ids, locale?, source? }` → `{ changed }` |
| PUT | `contacts/{id}` | `{ name?, locale? }` → `{ item }` |
| POST | `contacts/delete` | `{ ids }` → `{ deleted }` |
| GET | `suppressions?q=&reason=` | list `{ email, reason: unsubscribe\|bounce\|manual, detail, created_at }` |
| POST | `suppressions` · `suppressions/remove` | `{ email }` – block by hand · mail again |
| GET / POST | `campaigns` | `{ items }` · create `{ title, audiences?, subject_de?, subject_en?, body_de?, body_en? }` → 201 `{ item }` |
| GET / PUT / DELETE | `campaigns/{id}` | a draft changes everything, a paused one only subject / body (409 `invalid_status` otherwise); delete: draft or done |
| POST | `campaigns/{id}/preview` | `{ locale, subject?, body? }` (unsaved text) → `{ subject, text, html }` |
| POST | `campaigns/{id}/test` | `{ locale }` → the saved text to the signed-in admin, subject `[Test] …` (502 `mail_failed`) |
| POST | `campaigns/{id}/start` | draft → sending, recipients fixed (snapshot); 409 `no_audience` · `no_recipients` · `missing_locale` (`count`, `emails`) · `missing_text` (`locale`) |
| POST | `campaigns/{id}/pause` · `resume` | sending ⇄ paused |
| GET | `campaigns/{id}/recipients?status=&locale=&source=&q=` | `{ email, locale, name, source: member\|contact, status: queued\|sent\|failed\|skipped, attempts, next_attempt_at, sent_at, error }` |
| POST | `send` | one batch now → `{ result, items }` |

A newsletter item: `id, title, audiences, subject_de, subject_en, body_de, body_en, status (draft | sending | paused |
done), last_error, created_at/by, updated_at/by, started_at, finished_at, progress { queued, sent, failed, skipped,
total }`. A batch result: `{ status: idle | sent | limit | busy | error, sent, failed, retry, skipped, queued, per_hour,
sent_last_hour, error }`.

## Assets (the media asset layer, 2026-10-02 – agents/ASSETS.md)

Content keeps referencing CDN paths; this API manages the registry over the storage (local `media/` or the bunny.net
zone). Content editors (creator, admin):

| Method | Path | |
|---|---|---|
| GET | `/assets?q=&folder=&kind=&tag=&state=used\|unused&page=&page_size=&sort=` | `{ items: [asset summary], pagination, folders, cdn }` – current versions with their renditions and use counts; `folder` = a top-level folder, or with a `/` a path prefix (`characters/catpope` = the character and its persona folders) |
| GET | `/assets/lookup?path=` | `{ asset: detail \| null, rendition }` – the asset a CDN path belongs to |
| GET / PUT | `/assets/{id}` | detail `{ asset, versions: [{ …, renditions }], usages, jobs }` · PUT `{ title?, tags? }` |
| PUT | `/assets/upload?path=&title=&tags=` | raw body = the file → 201 detail + `uploaded`; 409 `exists`, 422 `invalid_path` / `invalid_type` / `content_mismatch` / `unsafe_svg` / `empty_file`, 413 `file_too_large` (`ASSET_UPLOAD_MAX_BYTES`) |
| GET | `/assets/missing` · `/assets/orphans` | uses of unknown files · current files nothing uses |

Admin (`/api/admin/assets/*`): `GET storage[?stats=1]` (drivers, settings status – never secrets –, totals, 30-day
bandwidth), `POST scan`, `POST relink { from, to, publish? }` (or `{ map: { old: new, … }, publish? }` – several files in one pass, one new draft per touched document; nothing written when a target is missing), `PUT {id}/file?path=&note=` (raw body: replace in place →
`replaced, archived_as, purged, stale_renditions`), `POST {id}/rename { slug, publish? }`, `GET {id}/delete-check`,
`DELETE {id}[?force=1]` (409 `in_use` + `usages` while referenced; `force` only when drafts are the only uses),
`POST {id}/jobs { kind, params }`, `GET jobs`, `GET unused` (cleanup list per asset on a freshly rebuilt index:
`files[{ path, what: current|old|archive, size }]`, `all_unused`, `totals`), `POST cleanup { paths }` (re-checks every path
against a fresh index; referenced ones are `skipped`, never deleted; an asset with every current file listed goes as a
whole → `deleted, assets_deleted, bytes, skipped, failed`). Without the migration: 503 `not_available`. A creator save / publish /
restore re-indexes the entity's uses.

## Security notes

- Sessions: HttpOnly cookie for the apps (JavaScript never stores the token; it only sees it once in the verify answer),
  CSRF by Origin for cookie writes, credentials-CORS only for `ALLOWED_ORIGINS` (never `*`) – "Single sign-on".

- `/api/keys/keys.php` serves the public key only; the private keypair (`storage/keys/keypair.json`) goes to admins only
  (`/api/admin/keys`, legacy `generate/keys.php` with the token).
- `/api/validate/`, `/api/code/`, `/api/access/` never return who claimed a code.
- The permissions map (tables `catalog_permissions` / `catalog_code_types`) is server-only (admin API only); clients only see the resulting `permissions`.
- A code is bound to one account (`qr.claimed_by`); codes claimed with the old email-only form can be taken over only
  by an account with that email.
- Grants are limited to the item's `grantable` sources; scan grants need a detecting tool and respect its cooldown.
