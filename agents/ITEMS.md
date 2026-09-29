# ITEMS — inventory, items, skills, combining, avatar, accounts

Concept + shared contract for all agents. Written 2026-09-29. Owner decisions from the user are marked (U).

## Concept

Everything a reader can own is an **item**: physical merch (via QR code), things found in the story, things found in
the real world (camera), and things made by combining other items. Items live in the reader's **inventory**
(account-bound when logged in, device-local as guest), can have **skills** (what happens when you *use* them —
often a minigame/tool), can be **combined** into new items (recipes), and wearables can be put on the reader's
**avatar** (one item per body-part slot). (U)

Guiding example (U):
- Every reader starts with a **camera**. A regular camera can only **scan QR codes** of other SCUTOONS items.
- **camera + cucumber → cucumber camera** (one cucumber is used up, the camera turns into the cucumber camera).
- The **cucumber camera** can also scan the real world and recognises **real cucumbers** (on-device ML) → each
  detected real cucumber grants a virtual cucumber. The learning: *items can be combined, and that unlocks logic.*
- Buying a hoodie on Bandcamp → QR code in the package → scan (phone camera or in-app camera) → claim with email →
  the hoodie is in the inventory and can be worn by the avatar.

## Item definition (catalog)

Single source: the server's `config/items.json` (was qr `api/items/items.json`), served at `GET /api/items/` (public). The viewer
fetches it from the QR API; it is not duplicated in the viewer.

```jsonc
{
  "version": 1,
  "slots": ["head", "face", "neck", "torso", "hands", "legs", "feet", "back", "accessory"],
  "items": [
    {
      "id": "camera",                         // stable, snake_case
      "toon": "catpope",                      // null = cross-toon
      "name": { "de": "Kamera", "en": "Camera" },
      "description": { "de": "…", "en": "…" },
      "kind": "virtual",                      // "physical" (bound to a QR code) | "virtual"
      "category": "tool",                     // tool | consumable | wearable | collectible | media
      "icon": "catpope/items/camera.png",     // CDN path, transparent cut-out PNG (512px)
      "stackable": false,
      "starter": true,                        // every new inventory gets one
      "grantable": [],                        // which client-side sources may grant it: "story" | "scan"
      "slot": null,                           // avatar slot for wearables
      "skills": [ { "type": "scan_qr" } ],
      "edition_id": null,                     // qr editions.json id for physical items
      "product": null,                        // viewer shop products.json key
      "variants": null                        // [{ "id": "black", "name": {...}, "icon": "…" }]
    }
  ],
  "recipes": [
    {
      "id": "cucumber_camera",
      "inputs": [ { "item": "camera", "qty": 1 }, { "item": "cucumber", "qty": 1 } ],  // all consumed
      "output": { "item": "cucumber_camera", "qty": 1 }
    }
  ]
}
```

### Skills (what "use" does)

| type | params | behaviour |
|---|---|---|
| `scan_qr` | – | opens camera, decodes QR; `qr.scutoons.com/?c=CODE` → claim flow for that code |
| `scan_object` | `labels: ["cucumber"]`, `grants: "cucumber"`, `cooldown_s` | on-device image classification (TF.js MobileNet, ImageNet class "cucumber, cuke"); hit → grant item (source `scan`) |
| `minigame` | `id` | launches a registered minigame component (registry in viewer, empty stubs ok) |
| `play_audio` | `src` | e.g. the vinyl unlocks/plays a track |
| (implicit) wear | item has `slot` | "Put on" / "Take off" on the avatar |

A skill list is additive: the cucumber camera has `scan_qr` **and** `scan_object`.

### Initial catalog

| id | kind | category | slot | skills | edition / product | notes |
|---|---|---|---|---|---|---|
| camera | virtual | tool | – | scan_qr | – | starter |
| cucumber | virtual | consumable | – | – | – | stackable; grantable story, scan; VN variable `cucumber` |
| cucumber_camera | virtual | tool | – | scan_qr, scan_object(cucumber) | – | from recipe |
| dago_hoodie | physical | wearable | torso | – | `1_catpope_dago` / `hoodie_*` | variants black, beige, grey |
| dago_heavy_t | physical | wearable | torso | – | `1_catpope_dago` / `tshirt_*` | variants black, white |
| scorpion_amulet | physical | wearable | neck | – | `2_catpope_beetles` | variant purple |
| awakening_vinyl | physical | media | – | play_audio | `3_catpope_vinyl` | LE 12" |
| awakening_test_press | physical | media | – | play_audio | (no edition yet) | variants 1_of_2, 2_of_2 |
| kangaroo_cat_hoodie | physical | wearable | torso | – | (no edition yet) | "Love For Women" |

## Codes → items

- A code **always contains items** (U). New table `code_items(code, item_id, variant NULL, qty)`.
- If a code has no `code_items` rows but has an `edition_id`, the edition's catalog items are the **options**: the
  claimer picks one item + variant at claim time (the owner decides per product how codes are printed (U); e.g. the
  Dago edition covers hoodie and tee, so the buyer picks).
- One code can be claimed by exactly one user; re-claim by the same user is idempotent.
- `n` codes → one account (email) (U).

## Accounts (port of osct `server/api/src/{Auth,LoginMail,Mailer,Progress}.php`)

Lives in qr.scutoons.com (the only backend). Email-based login (U): request → mail with one-time code + magic link →
verify → session token (Bearer, stored by the viewer in localStorage `scutoons_session`). Claiming a code during login
("email + code") attaches the code in the same step. Book progress (the viewer game state per toon) is stored per user
like osct's Progress. Local dev: mail is written to a log file instead of sent.

## HTTP API contract (base `QR_API` = `http://localhost:9000/api` dev, `https://qr.scutoons.com/api` prod)

(Platform P1b: the same paths answer on `https://api.scutoons.com/api` – the target base, `VITE_API_URL` – and stay
on `https://qr.scutoons.com/api` during the transition, `QR_HOST_SERVES_API`; agents/PLATFORM.md "Domain architecture".
Production CORS adds creator, admin and shop origins; dev 8000/10000/11000/12000.)

All JSON. Auth = `Authorization: Bearer <session_token>`. CORS: `https://scutoons.com`, `https://www.scutoons.com`,
`http://localhost:5173`, `http://localhost:5174`, `http://localhost:4173`. Errors: `{ "status": "error", "error": "<code>", "message": "…" }`.

| Method | Path | Body / query | Response |
|---|---|---|---|
| POST | `/auth/request/` | `{ email, code?, locale? }` | `{ status: "sent" }` (always, no user enumeration) |
| POST | `/auth/verify/` | `{ email, otp }` or `{ token }` | `{ status:"success", session_token, user:{ id, email, created_at }, claimed?: [code] }` |
| POST | `/auth/logout/` | – | `{ status: "success" }` |
| GET | `/me/` | – | `{ user, memberships:[{code,type,expires_at}], inventory:[Entry], avatar:{ slots:{ <slot>: entryId|null } } }` |
| GET | `/items/` | – | catalog (above) |
| GET | `/products/` | – | shop catalog, `config/products.json` as stored (`{ version:2, currency, checked_at, source, products:[Product] }`, `@scutoons/content` `ProductCatalog`); CORS `*`, cached 5 min (platform P2b) |
| GET | `/products/featured/` | – | homepage featured products, `config/featured-products.json` as stored (`[{ product, variant? }]`) |
| GET | `/code/?c=CODE` | – | `{ code, type, claimed: bool, claimed_by_me?: bool, items:[{item_id, variant, qty}], options:[{item_id, variants:[…]}], edition:{ id, name, number?, count } }` — never an email |
| POST | `/inventory/claim/` | `{ code, item_id?, variant? }` | `{ status, inventory:[Entry] }` · 409 `already_claimed` · 422 `choose_item` (+options) |
| POST | `/inventory/craft/` | `{ recipe_id }` | `{ status, inventory }` · 422 `missing_inputs` |
| POST | `/inventory/grant/` | `{ item_id, source:"story"\|"scan", ref? }` | `{ status, inventory }` · 403 if not in item's `grantable` · 429 rate limit |
| POST | `/inventory/import/` | `{ items:[{ item_id, qty }] }` | merge guest inventory on first login (grantable items only) |
| PUT | `/avatar/` | `{ slots:{ <slot>: entryId\|null } }` | `{ status, avatar }` · 422 wrong slot/not owned |
| GET/PUT | `/progress/<toonId>/` | `{ state, updated_at }` | last-write-wins by `updated_at` |
| POST/GET | `/validate/` | unchanged input | unchanged shape **minus** `email` and `signature` (backwards compatible for old viewer) |

`Entry = { id, item_id, variant|null, qty, source:"qr"|"story"|"scan"|"craft"|"starter", code|null, edition_number|null, acquired_at }`

## Viewer

- Config: `VITE_QR_API` (fallback prod URL) replaces the hardcoded `QRCODE_API_SERVER`.
- Guest: local inventory in localStorage (starter camera + story items); on login → `/inventory/import/`.
- VN `add` of a variable whose name is a catalog item id also grants the item (source `story`).
- **A code belongs to exactly one account (U, 2026-09-29).** Membership = a valid membership of the logged-in account
  (`/me/` memberships) only; the viewer never calls `/validate/` and grants nothing to guests.
- `?code=` on any page: logged in → claim sheet auto-claims (when there is nothing to choose); guest → claim sheet
  preview (`/code/`) + inline login (`/auth/request/` with `code` → claimed at verify). Claimed by someone else →
  "This code already belongs to another account", nothing granted. The in-app `scan_qr` camera uses the same sheet.
- Legacy `scutoons_membership` (device-local, pre-accounts) is no longer valid; its code is a pending claim: claimed
  silently after sign-in (key cleared on success / already yours / 409 — 409 shown once), guests get a one-time
  "Sign in with your email to keep your membership" prompt that opens the claim sheet.
- Magic link = `<viewer origin>/?login=<token>`; progress `updated_at` = ms since 1970.
- Inventory UI: grid of item icons (cut-outs), detail sheet (name, description, edition `n / total`, skills as
  actions: Use / Combine / Put on), combine mode (select 2 → recipe preview), avatar view with slots.
- Camera: QR decode with `jsqr` (iOS has no BarcodeDetector); cucumber detection with TF.js MobileNet, lazy-loaded
  only when the cucumber camera is used.

## Backend implementation notes (qr.scutoons.com, 2026-09-29)

Implemented in `C:\Development\qr.scutoons.com` (details: its `docs/ACCOUNTS.md`). Everything above holds; these
are **additive details / clarifications** the viewer can rely on:

- **Ids**: `Entry.id`, `user.id` are UUID **strings**. Timestamps are ISO 8601 strings.
- **Errors**: always `{ status:"error", error, message }` plus extra fields where noted. `401 unauthorized` when the
  Bearer token is missing/invalid/expired (drop `scutoons_session` then). `429 rate_limited` may carry `retry_after`.
- **Catalog**: each variant also has `product` (viewer `products.json` key, e.g. `hoodie_black`); item `product` =
  the default variant's key.
- `POST /auth/request/` also accepts `item_id`, `variant` (choice for the attached `code`); response
  `{ status:"sent", expires_at }`; `429 rate_limited` (5 mails/email/hour, 30/IP/hour), `502 mail_failed`.
  **Magic link**: request from an allowed viewer Origin → `<origin>/?login=<token>` (viewer calls
  `/auth/verify/ { token }`); otherwise (QR landing page) → `https://qr.scutoons.com/?login=<token>&c=<code>`.
- `POST /auth/verify/`: `{ email, otp }` checks the newest request of that address (5 wrong tries → `429
  too_many_attempts`; `410 expired|already_used`; `400 invalid_otp|invalid_token`). Optional `item_id`, `variant`
  override the stored choice. Response adds `created: bool` (new account → starter items granted) and, when the
  attached code could not be claimed, `claimed: []` + `claim_error: { code, error, message, options? }` (e.g.
  `choose_item` → call `/inventory/claim/` with the choice).
- `GET /code/?c=`: also returns `redirect_to`, `expires_at`; `options[].variants` are variant **ids** (names from
  the catalog). `404 not_found`, `410 expired`.
- `POST /inventory/claim/` → `{ status:"success", claimed:"new"|"already_yours", code, inventory }`. If the options
  have exactly one item+variant (vinyl, amulet) no choice is needed. Other errors: `422 invalid_item` (+options),
  `422 not_claimable` (types event/xr/page), `410 expired`, `404 not_found`. A code claimed through the old
  email-only form can be claimed by the account with that same email.
- `POST /inventory/craft/` → `{ status, recipe_id, inventory }`; `422 missing_inputs` has `missing:[{item, qty}]`;
  `404 unknown_recipe`. Crafting removes worn inputs from the avatar.
- `POST /inventory/grant/` → `{ status, granted: 0|1, inventory }`. A repeated `ref` (per user+item) grants nothing
  (`granted: 0`) – send e.g. `"catpope:ch1:scene3:cucumber"` for story grants. Limits: 120 grants/hour
  (`429 rate_limited`); `source:"scan"` additionally needs a tool whose `scan_object` grants the item (`403 no_tool`)
  and respects its `cooldown_s` (`429 cooldown`). `403 not_grantable`, `400 invalid_source`, `404 unknown_item`.
- `POST /inventory/import/` → `{ status, imported:[{item_id,qty}], skipped:[{item_id,reason}], inventory }`.
  Only items with non-empty `grantable`; stored with the item's first grantable source (`story`); qty capped at 99;
  non-stackable items only if not owned yet; 10 imports/hour. Safe to call on every login.
- `GET /avatar/` exists too. `PUT /avatar/` changes only the slots given; `avatar.slots` always lists every catalog
  slot (`null` = empty). Errors `422 invalid_slot|not_owned|wrong_slot` carry `slot`.
- `/progress/<toonId>/`: `GET` → `{ toon_id, state, updated_at }` (both `null` if nothing stored). `PUT
  { state: {…}, updated_at: <ms> }` → `{ status:"success"|"stale", toon_id, state, updated_at }` – the answer is the
  record that **won**; on `stale` the server kept a newer state, adopt it. `413 state_too_large` (> 256 KB).
- `/validate/` additionally drops `claimed_by`, adds `edition_number`, `is_expired:false`; an expired code now answers
  `410 { status:"error", message:"QR code expired" }`.
- Local dev: API `http://localhost:9000/api`, login mails in `qr.scutoons.com/logs/mail.log`, icons from
  `CDN_URL` (`http://localhost:8001/` locally).

## Access model (2026-09-30, user decision — final; replaces the anonymous-account draft)

Two separate things:

1. **Viewing content (device access, no account).** Any existing, non-expired code whose type grants `catalog` in the
   permissions map (today: `membership`, `promo`) unlocks every chapter of every toon **on this device** — also when
   the code is owned by someone else. No login, nothing stored on the server: device access, story items, progress
   and the guest inventory live only in the browser (localStorage).
2. **Account actions (login required).** Owning a code (item in the inventory, edition number), avatar, synced
   progress, crafting on the server, account page → email login (one-time code / magic link). **One owner per code**
   applies here: claiming a code someone else owns → `409 already_claimed` (they can still *view* with it).

- **Public, never gated:** `/`, toon landing `/<toon>`, credits, events, shop, attributions.
- **Gated by access:** chapter views and chapter download. Access = device code access **or** a logged-in account
  that owns an unlocking code.
- **No anonymous accounts.** `users.email` stays NOT NULL.
- Codes claimed by the old email-only form (`qr.email` set, `claimed_by` NULL) can only be claimed by an account with
  that email (`409 email_required`); viewing with them works like any code.

### Permissions map

The server's `config/permissions.json` (was qr `api/permissions.json`; the owner edits this file; no code change):
```json
{
  "version": 1,
  "permissions": { "catalog": "Read every chapter of every toon" },
  "types": {
    "membership": { "claimable": true,  "grants": ["catalog"] },
    "promo":      { "claimable": true,  "grants": ["catalog"] },
    "page":       { "claimable": false, "grants": [] },
    "event":      { "claimable": false, "grants": [] },
    "xr":         { "claimable": false, "grants": [] }
  },
  "default":      { "claimable": false, "grants": [] }
}
```
`claimable` = can be owned by one account (goes into its inventory); `grants` = permissions while not expired, for
anyone holding the code (device) and for its owner (account). Unknown types → `default`. Permission ids are open-ended
(later e.g. `toon:catpope`, `chapter:catpope:2`); the viewer checks `catalog` today. **Server-only** (security
decision 2026-09-30): not served over HTTP, no `/api/permissions/` endpoint – the viewer only sees `permissions` in the
`/access/` and `/me/` answers.

**`access_token`** (implemented 2026-09-30): `/access/` (when the code grants `catalog`) and `/me/` `access` (when
unlocked) also return `access_token` = `base64url(payload) "." base64url(HMAC-SHA256)`, payload
`{ v:1, perms:[…], exp:<unix, min(code expiry, now+30 days)>, sub:"code:<keyed hash>"|"user:<id>" }`, key derived from
`AUTH_SECRET` with its own label. Opaque to the viewer: store it with the device access (`scutoons_access`) and send it
later when a server-side content gate exists (`Permissions::verifyToken()`; nothing checks it yet).
`/me/ access` also carries `unlocked: bool` (= has `catalog`). `/access/` and `/code/` share the per-IP limit
(120/hour, `429 rate_limited`).

### API

| Method | Path | Body | Response |
|---|---|---|---|
| GET/POST | `/access/` | `{ code }` or `?c=` (optional Bearer) | `{ status:"success", code, type, expires_at, permissions:["catalog"], claimable, claimed: bool, claimed_by_me?: bool, access_token? }` for any existing, non-expired code — never who owns it · 404 · 410 · 429 (rate limit per IP against enumeration) |
| POST | `/inventory/claim/` | as before (login) | one owner; claimable types only (`422 not_claimable` otherwise) |
| GET | `/me/` | – | + `access: { permissions, unlocked, expires_at, codes, access_token? }` from owned codes; `memberships` = owned claimable codes (+ `grants`) |
| POST | `/inventory/claim/` | legacy email-claimed code, other account | `409 email_required` |
| GET/POST | `/validate/` | legacy | backwards compatible for the old viewer: success for any existing non-expired code (claimed or not), no email/signature/claimed_by |

### Landing page (qr.scutoons.com)

Claimable codes: item preview + **Continue** → `redirect_to?code=<code>` (the viewer grants device access) and a
secondary "Add to my account" → viewer `/account?code=<code>` (sign in there, then claim). No email step on the landing
page. Non-claimable types keep their direct redirect.

### Viewer

- `?code=` / in-app `scan_qr` / legacy `scutoons_membership` → `/access/` → store device access
  (`scutoons_access`: code, permissions, expires_at, checked_at) → chapters unlocked. Re-check it with `/access/`
  now and then (expiry). Code claimed by someone else still grants viewing.
- After a successful scan, offer (not force) "Add to my account" → login → `/inventory/claim/` (409 → "already in
  someone's account, you can still read with it").
- Guest: local inventory/progress only; logged in: server inventory/progress; login imports the guest's grantable
  items (as before). Device access stays after logout.
- Blocker (no access): "Scan your code", "Sign in", "Buy access".
- Implemented 2026-09-30 (viewer): `/account?code=` opens the same sheet and goes straight to sign-in → claim;
  `/access/` `access_token` is stored with the device access (account: `/me/` `access.access_token`) and exposed as
  `membership.accessToken` for a future content gate (not sent yet); `X-Device-Id` on every QR_API request;
  re-check every 6 h, `410` → device access dropped + one-time "This code is no longer valid" notice.

## Revoke + activity statistics (2026-09-30, user decision)

- **No device limits** — codes are shareable for virality. Danger → the owner revokes that one code.
- `qr.revoked_at` (+ `revoked_reason`): revoked codes grant nothing — `/access/`, `/validate/`, `/inventory/claim/`
  → `410 revoked`; `/me` access ignores them. Viewer: on `410 revoked`/`expired` during a re-check → drop device access.
- **`code_events`** (append-only): `id, code, event, device_hash NULL, user_id NULL, ip_hash NULL, toon NULL, created_at`.
  Events: `scan` (landing page `/?c=`), `access` (at most once per code+device per UTC day), `claim`,
  `claim_refused`, `login` (one row per owned code at sign-in), `revoke`, `unrevoke`.
  Privacy: viewer sends a random per-browser id `X-Device-Id` (localStorage `scutoons_device`); server stores
  `hmac(device id)` only. IP → `hmac(ip + daily salt)`. No raw IP, no email, no user agent string in events.
- **Admin stats** in the generate SPA (admin token): per code — edition, number, type, scans, unique devices,
  accesses, first/last seen, claimed (at), revoked; sort by spread; filter edition/type; daily chart scans/unlocks;
  CSV export; revoke/unrevoke with reason. Endpoints under `generate/` (admin token), never public.
- **Implemented (qr backend, 2026-09-30):** as specified. Details for the viewer: `X-Device-Id` must match
  `[A-Za-z0-9_-]{8,128}` (otherwise the access is deduped per daily IP hash); CORS allows the header. `/access/`
  also accepts an optional `toon` (body or `?toon=`) stored with the event. `410 revoked` also comes from `/code/`.
  Old magic links `qr.scutoons.com/?login=<token>` are forwarded to `APP_URL/?login=<token>`.

## Roles, signup rule, promo codes (2026-09-30, user decision)

### Roles
- Tables: `roles (id PK, name, description)` seeded with `member`, `admin`, `creator`;
  `user_roles (user_id, role_id, granted_at, granted_by NULL)`, PK (user_id, role_id).
- Every new account gets `member`. `/me/` → `user.roles: ["member", …]`.
- Grant/revoke: admin SPA (generate) "Users" view (search by email, toggle roles) via admin-token endpoints, and a CLI
  `php scripts/roles.php grant|revoke <email> <role>`.
- `creator` → may use `scutoons.com/creator` and the content API; `admin` → reserved (later: admin UI in the viewer).

### Accounts only with a product or a PR code
- **A new account can only be created with a code whose type has `"signup": true`** in `config/permissions.json` (server):
  - `membership` — physical product: signup + claim of that item (one owner, as before).
  - `promo` — PR code: **not claimable** (no item, no owner, reusable by many people), grants `catalog` and allows
    signup. Account = save progress across devices + optional promotional updates about new releases.
- `page`, `event`, `xr` → no signup.
- `auth/request` for an email **without** an account and **without** a signup code → still `{status:"sent"}` (no user
  enumeration), but the mail says there is no account yet and how to get one (a code from a SCUTOONS product / promo).
  Existing accounts can always sign in without a code.
- `users.marketing_opt_in` (bool) + `marketing_opt_in_at`, `users.signup_code` (the code used). Opt-in is an unchecked
  checkbox at signup ("Send me news about new releases") and toggleable on the account page. Event `signup`.
- permissions.json becomes:
  `membership { claimable:true, signup:true, grants:["catalog"] }`, `promo { claimable:false, signup:true, grants:["catalog"] }`,
  `page|event|xr { claimable:false, signup:false, grants:[] }`, `default` like page.

### Viewer copy for promo codes (claim sheet)
"Unlocked on this device. Enjoy reading!" then: **no item to claim** — "This is a promo code – there's no item to
collect. You can still create an account to save your progress across devices and get news about new releases."
Buttons: "Continue reading" · "Create account" (email → code → opt-in checkbox). For membership codes keep
"Add to my account".

**Implemented (2026-09-30):** qr backend + viewer as specified (details: qr `docs/ACCOUNTS.md` "Signup rule" and
"Roles"). Contract additions: `/access/` and `/code/` return `signup` (`/code/` also `claimable`); `/auth/request/` and
`/auth/verify/` accept `marketing_opt_in` (verify wins; `true` at sign-in also opts an existing account in);
`/auth/verify/` returns `created` and `403 signup_code_required` when a new account has no usable signup code;
`claimed`/`claim_error` only for claimable codes; `/me/ user` = `{ id, email, created_at, roles, marketing_opt_in }`;
`PUT /me/ { marketing_opt_in }` → `{ status, user }`. For other endpoints: `Roles::requireRole('creator')` (401/403
`forbidden`). A membership code already owned by someone else cannot create an account.

## Admin API (2026-09-30, platform P1b — final for the admin app)

Server-side replacement of the `generate/*.php` endpoints for the admin app (admin.scutoons.com) plus KPIs and lists.
Implemented in the server (`src/Http/Controllers/Admin/*`, `src/Domain/Admin/*`); tested by `tests/admin-smoke.php`.

- **Base** `{API}/admin` (`VITE_API_URL` + `/admin`, e.g. `http://localhost:9000/api/admin`, prod
  `https://api.scutoons.com/api/admin`). All JSON (UTF-8, unescaped), `Cache-Control: no-store`.
- **Auth**: the normal account login (`/auth/request/`, `/auth/verify/`) → `Authorization: Bearer <session_token>`;
  the account needs the role `admin` (`/me/ user.roles`). No session → `401 unauthorized`; without the role →
  `403 forbidden { role: "admin" }`. Grant: `PUT /admin/users/{id}/roles/admin`, legacy SPA "Users", or
  `bin/console roles grant <email> admin`.
- **CORS**: origins in `ALLOWED_ORIGINS` (admin: `https://admin.scutoons.com`, dev `http://localhost:11000`); methods
  `GET, POST, PUT, DELETE, OPTIONS`.
- **Errors**: `{ status:"error", error, message, …extra }` as everywhere; unknown path `404 not_found`.
- **Success** answers carry `status: "success"`; creates answer `201`.
- **Lists** (URL-driven): `?page=1&page_size=50&sort=<field>|-<field>&q=<text>&<filter>=<value>` →
  `{ status, items: [...], pagination: { total, page, pageSize } }`. `page` from 1; `page_size` clamped to 1…500;
  `-field` = descending; unknown sort field → `400 invalid_sort { sortable }`.
- **Privacy**: code, event and statistics answers never contain emails or user ids (`claimed`/`account` booleans,
  `device` = first 10 characters of the device hash). Emails only in the users endpoints.
- **Times**: `created_at`, `claimed_at`, `expires_at`, `revoked_at` of codes are the `qr` table's server-local
  `YYYY-MM-DD HH:MM:SS`; everything else ISO 8601 (UTC) as in the public API.

### Dashboard

`GET /admin/dashboard?days=30&edition_id=<id>|none` (days 1–366, default 30; period ends today UTC) →
```jsonc
{ "status": "success", "days": 30, "from": "2026-09-01", "to": "2026-09-30", "edition_id": null,
  "totals": { "codes", "codes_claimed", "codes_revoked", "codes_signed", "users", "users_opted_in", "users_new" },
  "period": { "scan", "access", "claim", "signup", "login", "claim_refused", "revoke", "unrevoke", "unique_devices" },
  "editions": [ { "id": "3_catpope_vinyl"|null, "name", "toon", "edition_count", "codes", "claimed", "revoked", "signed",
                  "scan", "access", "claim", "signup", "login", "claim_refused", "revoke", "unrevoke" } ],
  "daily": [ { "day": "2026-09-01", "scan", "access", "claim", "signup", "login", "claim_refused", "revoke", "unrevoke" } ],
  "daily_by_edition": [ { "day", "edition_id", …same counts } ] }
```
`editions`: codes claimed/total per edition (all time) + activity in the period; `id: null` = codes without edition.
`daily` has every day of the period (zeros included); `daily_by_edition` only rows with events. `400 invalid_days`.

### Codes

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/codes` | list; sort `code` (default), `type`, `edition_id`, `edition_number`, `created_at`, `claimed_at`, `expires_at`, `revoked_at`, `scans`, `accesses`, `devices`, `logins`, `refused`, `first_seen`, `last_seen`; filters `edition_id` (`none` = without), `type`, `status` = `claimed`\|`unclaimed`\|`revoked`\|`expired`\|`active`\|`signed`\|`unsigned`; `q` = part of the code | items `{ code, type, edition_id, edition_number, created_at, claimed, claimed_at, expires_at, revoked_at, revoked_reason, scans, accesses, devices, logins, refused, first_seen, last_seen, signed }` · `400 invalid_status` |
| GET | `/admin/codes?format=csv&…` | same filters, no paging | `text/csv` attachment (header row; booleans yes/no) |
| POST | `/admin/codes` | `{ redirect_to, type, code?, email?, expires_at?, edition_id? }` – `code` omitted → a fresh nanoid(8); `email` = old email-only claim (normally empty) | `201 { status, code: { code, redirect_to, type, email, expires_at, edition_id } }` · `400 invalid_redirect_to\|invalid_type\|invalid_email\|invalid_code\|invalid_expires_at` · `409 code_exists` |
| GET | `/admin/codes/{code}` | – | `{ status, code: { code, type, edition_id, edition_number, redirect_to, created_at, claimed_at, expires_at, revoked_at, revoked_reason, claimed }, items: [{ item_id, variant, qty }], options: [{ item_id, variants }], edition: { id, name, number, count }\|null, stats: { scans, accesses, devices, logins, refused, first_seen, last_seen }, signed, expired, revoked }` · `404 not_found` |
| GET | `/admin/codes/{code}/events?limit=1000` | limit ≤ 5000 | `{ status, items: [{ event, created_at, device, account, toon }] }` newest first · 404 |
| POST | `/admin/codes/{code}/revoke` | `{ reason? }` (≤ 255) | `{ status, code, revoked_at, revoked_reason }` (logged as event `revoke`) · 404 |
| POST | `/admin/codes/{code}/unrevoke` | – | same, `revoked_at: null` · 404 |
| GET | `/admin/codes/{code}/signature` | – | `{ status, signature: { signature, publicKey, algorithm, signedData, timestamp }, fingerprint, qr_data, redirect_to }` · `404 not_found\|no_signature` |
| PUT | `/admin/codes/{code}/signature` | `{ signature: { signature, publicKey, algorithm, signedData, timestamp } }` (made client-side) | `{ status, code }` · `400 invalid_signature` · 404 |

### Batches and files (port of the generate "Batch" mode)

A batch = a manifest `BATCH-<UTC timestamp>-<edition id>.json`:
`{ filename, edition, editionId, productname, edition_number, edition_count, excluded_number, creators, redirectUrl,
total, created_at, codes: [{ n, code, status: "pending"|"running"|"done"|"error", completedSteps: ["db","svg","pdf","sig"] }] }`.
Pipeline per code (client-side, like generate): sign (keys below) → register (`POST /admin/codes` with `code`,
`redirect_to: redirectUrl`, `type: "membership"`, `edition_id`) → QR SVG → certificate PDF → store files → store the
signature (`PUT /admin/codes/{code}/signature` + the `signatures` file) → `PUT` the manifest with the progress.

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/batches` | list; sort `created_at` (default `-created_at`), `filename`, `edition`, `total`, `done`, `errors`; filters `edition_id`, `complete` (1/0); `q` in filename + edition | items `{ filename, edition, editionId, total, done, errors, created_at, complete }` |
| POST | `/admin/batches` | `{ edition_id, count? (default edition_count, 1–2000), start_number? (default 1), force? }` | `201 { status, batch: manifest }` – codes are unique (DB + all manifests) and reserved in the manifest only, **not** inserted · `404 unknown_edition` · `400 invalid_count\|invalid_start_number` · `409 batch_complete { batch }` (a complete batch of this edition exists; `force: true` to make more) |
| GET | `/admin/batches/{filename}` | – | `{ status, batch: manifest }` · 404 |
| PUT | `/admin/batches/{filename}` | `{ manifest }` (or the manifest itself) – must have a `codes` list, `filename` must match | `{ status, batch }` · `422 invalid_manifest` · 404 (create with POST) |
| GET | `/admin/batches/{filename}/zip` | – | `application/zip` attachment: `manifest.json` + `svgs/`, `certificates/`, `signatures/` of every code with status `done` |
| PUT | `/admin/files/{type}/{filename}` | `{ doc }` – SVG markup, a PDF as data URI (`data:application/pdf;base64,…`) or bytes, JSON text (stored pretty-printed UTF-8) | `{ status, type, filename }` · `400 invalid_type\|invalid_filename\|invalid_file` |
| GET | `/admin/files/{type}/{filename}` | – | the file (`image/svg+xml`, `application/pdf`, `application/json`, `application/zip`) as attachment · 404 |

`type` = `svgs` (`SCUTOONS-QR-MEMBERSHIP-<code>.svg`), `certificates` (`SCUTOONS-COA-<code>.pdf`), `signatures`
(`SIGNATURE-<code>.json`), `batch`, `zips`; `filename` = `[A-Za-z0-9_.-]{1,200}.(pdf|svg|json|zip)`, no leading dot.

### Users and roles

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/users` | list; sort `created_at` (default `-created_at`), `email`, `codes`; filters `role` (name), `opt_in` (1/0); `q` = part of the email | items `{ id, email, created_at, roles, signup_code, marketing_opt_in, marketing_opt_in_at, codes }` (`codes` = number owned) |
| GET | `/admin/users/{id}` | – | `{ status, user: { id, email, created_at, roles, signup_code, marketing_opt_in, marketing_opt_in_at, locale, codes: [{ code, type, edition_id, edition_number, claimed_at, revoked_at }], inventory_entries, sessions, last_seen_at } }` · 404 |
| PUT | `/admin/users/{id}/roles/{role}` | – | `{ status, changed, user }` (grant; `changed: false` when it had it; `granted_by` = `admin:<your email>`) · `400 unknown_role { roles }` · 404 |
| DELETE | `/admin/users/{id}/roles/{role}` | – | `{ status, changed, user }` (revoke) |
| GET | `/admin/roles` | – | `{ status, items: [{ id, name, description }] }` (member, admin, creator) |

### Events, catalog, keys

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/events` | list (default page_size 100); sort `created_at` (default `-created_at`), `code`, `event`; filters `code`, `event` (comma list of scan, access, claim, claim_refused, login, signup, revoke, unrevoke), `edition_id` (`none`), `toon`, `from`, `to` (`YYYY-MM-DD`, UTC); `q` = part of the code | items `{ id, code, event, created_at, device, account, toon, edition_id }` · `400 invalid_event\|invalid_date` |
| GET | `/admin/catalog` | – | `{ status, catalog (= GET /items/), editions, toons, permissions (the server-only map, read-only) }` |
| GET | `/admin/keys` | – | `{ status, algorithm, publicKey, privateKey, generated }` – the ML-DSA-65 keypair for client-side signing · `404 keys_missing` · `500 keys_invalid` |
| GET | `/admin/versions` | – | `{ status, checked_at, api: { app:"api", version, git_sha, built_at }, apps: [{ app, url, ok, http_status, version?, git_sha?, built_at?, shared?: { "@scutoons/api": "0.1.0", … }, error? }] }` – every deployed app's `version.json` (server setting `VERSION_URLS`: viewer, admin, creator, shop, qr) fetched in parallel (4 s timeout), next to the API's own; for the dashboard's out-of-sync check (platform P3) |

Also platform P3: `GET /health/` returns `{ status, db, version, git_sha, built_at }`; `GET /code/?c=&scan=1` logs a
`scan` event only for the qr app (trusted proxy: `X-Scutoons-Proxy-Secret` = `QR_PROXY_SECRET`, which also lets it
forward `X-Scutoons-Client-Ip` / `X-Scutoons-Device-Id`); for everyone else `scan` is ignored.

### Settings (runtime, 2026-09-30)

Values the owner changes without a deploy (table `settings`: `key`, `value` JSON, `updated_at`, `updated_by`). A stored
value wins; without a row the config value is the default. Read once per request (`Domain\Settings\RuntimeSettings`).

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/settings` | – | `{ status, items: [{ key, value, default, min, max, stored, updated_at, updated_by }] }` (`updated_at` ISO 8601 UTC, `updated_by` = `admin:<email>` of the last change, both null while nothing is stored) |
| PUT | `/admin/settings` | `{ <key>: int\|null, … }` or `{ settings: { … } }` – integer `min…max`, `null` = back to the default (row removed) | `{ status, changed: [keys], items }` – all validated first, only changed values written · `400 invalid_setting { key, min, max }\|unknown_setting { settings }\|invalid_settings` (empty body) |

| Key | Default (config) | What |
|---|---|---|
| `scan_limit_per_hour` | `SCAN_MAX_PER_IP_HOUR` = 1000 | QR scans per client IP and hour: `GET /code/?c=&scan=1` from the qr app (trusted proxy, forwarded IP), own bucket `qrscan:<ip hash>`. Events share one IP – keep it high. 0 = no limit |
| `lookup_limit_per_hour` | `ACCESS_MAX_PER_IP_HOUR` = 120 | direct `/code/` + `/access/` lookups per IP and hour (bucket `lookup:<ip hash>`). 0 = no limit |

Over a limit: `429 rate_limited { retry_after: 3600 }`; the qr app shows its rate-limited page (429). Range 0…1000000.

### Content (read-only overview, platform P2a)

The admin app lists the content store without the `creator` role; editing stays in the creator.

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/content` | optional `toon` (`[a-z0-9_-]{1,64}`) | `{ status, toons: [{ id, name }], files: [{ toon, locale, path, latest, latest_status: "draft"\|"published", latest_at, published: id\|null, published_at, versions }] }` – `toons` = `config/toons.json` + toons that only exist in the store; files ordered by toon, path, locale; times are the store's `YYYY-MM-DD HH:MM:SS` (UTC) · `400 invalid_toon` |

The legacy `generate/*.php` endpoints (static `X-Generate-Token`) keep working unchanged for the `generate/` SPA until the
admin app has parity; then `generate/` and `GENERATE_PASSWORD` are removed.

## Reader avatar bodies & character unlocks (2026-09-30, owner — follow-up, not built yet)

- The reader avatar body is a **character's A-pose** (character level, not persona) in one of its A-pose looks:
  `swim` (default, swim pants) or `naked`. Data lives in the character registry (`character.apose.looks[]` with
  path + wear anchors per slot); helper `avatarBodies(registry)` in `@scutoons/content`.
- **Members can only pick characters they have unlocked.** Server tables (to build):
  `member_characters (user_id, character_id, source: default|code|item|story|admin, ref?, unlocked_at)` and
  `member_character_looks (user_id, character_id, look_id, source, unlocked_at)` for A-pose looks beyond the default.
  A default-unlocked set is configurable (permissions/settings), unlocks can be granted by codes/items (catalog),
  story events (VN `add`/grant) and admins. `avatar` gets `body: { character, look }` (server, per account; guests
  per device).
- **No look creator:** outfitting (putting items on the body) happens only in the reader's inventory avatar view;
  the creator only stores A-pose images and anchors as data.

