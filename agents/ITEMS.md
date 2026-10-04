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
- The **cucumber camera only scans cucumbers** (U, 2026-09-30): it recognises **real cucumbers** in the real world
  (on-device ML) → each detected real cucumber grants a virtual cucumber, but it can no longer read QR codes. Crafting is
  a real trade-off. The learning: *items can be combined, and that unlocks (and costs) logic.*
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
| ~~`play_audio`~~ | `src` | **replaced 2026-10-02** by albums + players ("Players, albums, related items"); still read gracefully (plays without a player), no catalog item uses it |
| (implicit) play | item has `album` + `format` | "Abspielen" only with a player whose `plays` has the format; else "Zum Abspielen brauchst du einen Plattenspieler" + the album download |
| (implicit) wear | item has `slot` | "Put on" / "Take off" on the avatar |

A camera session has one mode: an item with `scan_object` opens straight in object detection (no QR decoding, no mode
toggle); `scan_qr` opens the QR scanner. The cucumber camera has **only** `scan_object` (U, 2026-09-30) – crafting
consumes the starter camera, so the reader trades QR scanning for cucumber detection (the membership blocker's "Scan
your code" still opens the QR scanner without any item).

### Initial catalog

| id | kind | category | slot | skills | edition / product | notes |
|---|---|---|---|---|---|---|
| camera | virtual | tool | – | scan_qr | – | starter |
| cucumber | virtual | food | – | apply ("Essen") | – | stackable; grantable story, scan; VN `add cucumber` grants it; eaten: appetite −5 (still a recipe ingredient) |
| cucumber_camera | virtual | tool | – | scan_object(cucumber) | – | from recipe; no QR scanning |
| dago_hoodie | physical | wearable | torso | – | `1_catpope_dago` / `hoodie_*` | variants black, beige, grey |
| dago_heavy_t | physical | wearable | torso | – | `1_catpope_dago` / `tshirt_*` | variants black, white |
| scorpion_amulet | physical | wearable | neck | – | `2_catpope_beetles` | variant purple |
| awakening_vinyl | physical | media | – | – (album `awakening`, format `vinyl`) | `3_catpope_vinyl` | LE 12" |
| awakening_test_press | physical | media | – | – (album `awakening`, format `vinyl`) | (no edition yet) | variants 1_of_2, 2_of_2 |
| record_player | virtual | tool | – | – (`plays: ["vinyl"]`) | – | from the recipe `record_player_repair` (`grantable: []`) |
| broken_record_player | virtual | tool | – | – (plays nothing) | – | grantable story: chapter 2's first scene; recipe input |
| repair_coupon | virtual | collectible | – | – | – | grantable story: chapter 2's last slice; recipe input (spent) |
| tape_player | virtual | tool | – | – (`plays: ["tape"]`) | – | not obtainable yet (`grantable: []`) |
| cd_player | virtual | tool | – | – (`plays: ["cd"]`) | – | not obtainable yet (`grantable: []`) |
| kangaroo_cat_hoodie | physical | wearable | torso | – | (no edition yet) | "Love For Women" |
| scutoons_tshirt | virtual | wearable | torso | – | – | starter, basic dress (`default_worn`), `wear` overlays ("Wardrobe") |
| scutoons_swim_pants | virtual | wearable | legs | – | – | starter in the inventory, the cover rule's first choice (sensitive-content safety) |
| scutoons_shorts | virtual | wearable | legs | – | – | starter, basic dress (owner may still move it to a story find) |
| scutoons_sandals | virtual | wearable | feet | – | – | starter, basic dress |

## Collectables removed – items are the only concept (U, 2026-09-30)

The old VN inventory ("collectables": `<toon>/<locale>/collectables.json`, `Collectable`/`CollectableRef` types, the
inventory showing game variables that matched a collectable id) is gone everywhere (files, types, loaders, UI, import /
seed, library bundle, docs). Everything a reader can own is a catalog item. VN variables stay story logic (`{cucumber}`
interpolation, conditions); the VN → item bridge (`set`/`add` of a variable whose name is a catalog item id grants the
item, source `story`, ref per place) does the collecting. The old art `catpope/images/collectables/cucumber.png` is
unreferenced (left on the CDN).

## Codes → items

- A code **always contains items** (U). New table `code_items(code, item_id, variant NULL, qty)`.
- If a code has no `code_items` rows but has an `edition_id`, the edition's catalog items are the **options**: the
  claimer picks one item + variant at claim time (the owner decides per product how codes are printed (U); e.g. the
  Dago edition covers hoodie and tee, so the buyer picks).
- One code can be claimed by exactly one user; re-claim by the same user is idempotent. Ownership moves between accounts only through the
  transfer core ("Ownership, claims, gifting": a claim decided by an admin, an admin transfer, a gift).
- `n` codes → one account (email) (U).

## Accounts (port of osct `server/api/src/{Auth,LoginMail,Mailer,Progress}.php`)

Lives in qr.scutoons.com (the only backend). Email-based login (U): request → mail with one-time code + magic link →
verify → session token (Bearer, stored by the viewer in localStorage `scutoons_session`). Claiming a code during login
("email + code") attaches the code in the same step. Book progress (the viewer game state per toon) is stored per user
like osct's Progress. Local dev: mail is written to a log file instead of sent. (2026-10-02: every mail goes through
the server's email module, agents/PLATFORM.md "Emails".)

## HTTP API contract (base `QR_API` = `http://localhost:9000/api` dev, `https://qr.scutoons.com/api` prod)

(Platform P1b: the same paths answer on `https://api.scutoons.com/api` – the target base, `VITE_API_URL` – and stay
on `https://qr.scutoons.com/api` during the transition, `QR_HOST_SERVES_API`; agents/PLATFORM.md "Domain architecture".
Production CORS adds creator, admin and shop origins; dev 8000/10000/11000/12000.)

All JSON. Auth = `Authorization: Bearer <session_token>` **or** (single sign-on, 2026-10-01) the HttpOnly cookie
`scutoons_sid` that `/auth/verify/` sets for the apps (Bearer wins when both come; server docs/API.md "Single sign-on").
CORS: `ALLOWED_ORIGINS` (with `Access-Control-Allow-Credentials: true`). Errors: `{ "status": "error", "error": "<code>", "message": "…" }`.

| Method | Path | Body / query | Response |
|---|---|---|---|
| POST | `/auth/request/` | `{ email, code?, locale? }` | `{ status: "sent" }` (always, no user enumeration) |
| POST | `/auth/verify/` | `{ email, otp }` or `{ token }` | `{ status:"success", session_token, user:{ id, email, created_at }, claimed?: [code] }` |
| POST | `/auth/logout/` | `{ all?: true }` | `{ status: "success" }` – ends the session (the shared one: every app) and clears the cookie; `all` = every session of the account |
| GET | `/auth/session/` | – | `{ status, user \| null, via: "cookie"\|"bearer"\|null }` – who is signed in; renews the cookie |
| POST | `/auth/session/` | Bearer | `{ status, user, via: "cookie" }` + the cookie for that same session (an app's stored token moves into the cookie once) · 400 `bearer_required` · 403 `csrf_rejected` (Origin not an app) |
| GET | `/me/` | – | `{ user, memberships:[{code,type,expires_at}], inventory:[Entry], avatar:{ slots:{ <slot>: entryId|null } } }` – `user.hide_sensitive` ("Sensitive content") |
| PUT | `/me/` | `{ marketing_opt_in?, hide_sensitive? }` | `{ status, user, avatar }` – turning `hide_sensitive` on moves a worn sensitive body to the character's first non-sensitive look · 400 `invalid_body` |
| GET | `/items/` | – | catalog (above) |
| GET | `/products/` | – | shop catalog, `config/products.json` as stored (`{ version:2, currency, checked_at, source, products:[Product] }`, `@scutoons/content` `ProductCatalog`); CORS `*`, cached 5 min (platform P2b) |
| GET | `/products/featured/` | – | the landing page's editions `[{ product, variant? }]`: chosen in the admin app ("Startseite", `GET/PUT /admin/featured`, setting `featured_products`), default `config/featured-products.json` as stored; cached 1 min |
| GET | `/stockists/?product=&country=` | – | "Where to buy": the Embassy's listed stores carrying products (public fields only, independent first; `{ stores, products: { id: count }, countries }`), CORS `*`, cached 5 min + ETag, 600 / IP / hour – agents/EMBASSY.md "Stores" |
| GET | `/library/toons[/<toon>[/seasons/<n>\|/chapters/<n>\|/cast\|/credits]]`, `/library/characters`, `/events/` | – (`?draft=1`: creator) | the reader's entities from the content store (agents/DATA.md §3, server docs/API.md "Library"): toons list, toon bundle, season overview, chapter with scenes inlined, cast, credits, events; `?locale=`, ETag, 403 `coming_soon`/`not_public` |
| GET/PUT/POST | `/entities/<key>…` | creator | the content store by entity key (`chapter:catpope:2:de`, `characters` …): read, save draft, versions, publish, unpublish, restore – server docs/API.md "Entities" (replaced `/content/*`) |
| GET | `/code/?c=CODE` | – | `{ code, type, claimed: bool, claimed_by_me?: bool, items:[{item_id, variant, qty}], options:[{item_id, variants:[…]}], edition:{ id, name, number?, count } }` — never an email |
| POST | `/inventory/claim/` | `{ code, item_id?, variant? }` | `{ status, inventory:[Entry] }` · 409 `already_claimed` · 422 `choose_item` (+options) |
| POST | `/inventory/craft/` | `{ recipe_id }` | `{ status, inventory }` · 422 `missing_inputs` |
| POST | `/inventory/grant/` | `{ item_id, source:"story"\|"scan", ref? }` | `{ status, inventory }` · 403 if not in item's `grantable` · 429 rate limit |
| POST | `/inventory/import/` | `{ items:[{ item_id, qty }] }` | merge guest inventory on first login (grantable items only) |
| PUT | `/avatar/` | `{ slots:{ <slot instance id>: entryId\|null }, body? }` | `{ status, avatar, restored }` · 422 invalid_slot/wrong_slot/not owned ("Slots per avatar body") · 422 `sensitive_hidden` ("Sensitive content") · `restored` = instances the cover rule refilled ("Wardrobe") |
| GET/PUT | `/progress/<toonId>/` | `{ state, updated_at }` | last-write-wins by `updated_at` |
| POST/GET | `/validate/` | unchanged input | unchanged shape **minus** `email` and `signature` (backwards compatible for old viewer) |
| GET | `/certificates/<CODE>` (or `?code=`) | – | certificate check of a code, server-side ("Certificates" below): `{ status, valid, reason, source:"code", code, registered, revoked, certificate }` – never the owner; CORS `*`; lookup limit |
| POST | `/certificates/verify` | a `SIGNATURE-<code>.json` (or its inner `signature` object) | same shape, `source:"file"` · 400 `invalid_signature_file` |

`Entry = { id, item_id, variant|null, qty, source:"qr"|"story"|"scan"|"craft"|"starter"|"gift", code|null, edition_number|null, acquired_at }` (`gift`: "Ownership, claims, gifting")

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
- Inventory UI (catalog items only – collectables were removed 2026-09-30): grid of item icons (cut-outs), detail sheet (name, description, edition `n / total`, skills as
  actions: Use / Combine / Put on), combine mode (select 2 → recipe preview), avatar view with slots.
- Camera: QR decode with `jsqr` (iOS has no BarcodeDetector); cucumber detection with TF.js MobileNet, lazy-loaded
  only when the cucumber camera is used (that session never loads jsQR).

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
- `GET /avatar/` exists too. `PUT /avatar/` changes only the slots given; `avatar.slots` always lists every slot
  instance of the current body (since 2026-09-30 – "Slots per avatar body"; before: every catalog slot) (`null` = empty). Errors `422 invalid_slot|not_owned|wrong_slot` carry `slot`.
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

Since 2026-10-04 the tables `catalog_permissions` + `catalog_code_types`, edited in the admin API (agents/CATALOG.md); the
shape below is the seed `config/permissions.json` (was qr `api/permissions.json`) and what `Permissions::map()` builds:
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

### Emails (2026-10-02, the email module – agents/PLATFORM.md "Emails")

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/emails/templates` | – | `{ status, locales: ["de","en"], transport, items: [{ id, audience: "member"\|"admin", description, vars: { name: type }, sample }] }` |
| GET | `/admin/emails/templates/{id}/preview` | `?locale=de\|en` (default en) | `{ status, id, locale, subject, text, html }` rendered with the sample vars · `404 not_found` · `400 invalid_locale` |
| GET | `/admin/emails/log` | list; sort `created_at` (default `-created_at`), `template`, `status`, `recipient`; filters `template`, `status` (`sent`\|`failed`), `recipient`, `from`, `to` (`YYYY-MM-DD`, UTC); `q` = part of the address | items `{ id, template, recipient, locale, status, transport, error, created_at }` · `400 invalid_template\|invalid_status\|invalid_date` |
| GET | `/admin/emails/recipients` | – | `{ status, items, default (MAIL_ADMIN_TO), stored, updated_at, updated_by, max: 10 }` |
| PUT | `/admin/emails/recipients` | `{ items: [email, …] \| null }` | `{ status, changed, …same }` – lower-cased, deduplicated, `[]` = nobody, `null` = back to the default · `400 invalid_recipients` |

Privacy: the log shows recipient addresses (admin only, like the users endpoints), never a mail's content.

### Featured editions on the landing page (2026-10-01, owner)

Admin page **"Startseite" / "Homepage"** (`admin/src/routes/homepage`, nav group Data): which products of the shop
catalog, in which colour (= which images), in which order the landing page shows. Since 2026-10-04 the table
`catalog_featured` (agents/CATALOG.md; one `catalog_history` row per change); until then the setting `featured_products`,
folded into the table by the migration 2026-10-04-catalog. The default (`null`) is the seed's list
`server/config/featured-products.json` (dago_hoodie beige, dago_heavy_t white).

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/featured` | – | `{ status, items: [{ product, variant? }], default, stored, updated_at, updated_by, max: 12 }` |
| PUT | `/admin/featured` | `{ items: [{ product, variant? }] \| null }` | `{ status, changed, …as GET }` · `400 invalid_featured\|unknown_product\|unknown_variant\|duplicate_featured` |

The public `GET /products/featured/` returns the list, `max-age=60`.
Contract: `admin-smoke` "Featured editions" (401/403, validation, store, public read, back to the default).

### Content (read-only overview, platform P2a)

The admin app lists the content store without the `creator` role; editing stays in the creator.

| Method | Path | Request | Response |
|---|---|---|---|
| GET | `/admin/content` | optional `toon` (`[a-z0-9_-]{1,64}`) | `{ status, toons: [{ id, name }], files: [{ toon, locale, path, latest, latest_status: "draft"\|"published", latest_at, published: id\|null, published_at, versions }] }` – `toons` = `config/toons.json` + toons that only exist in the store; files ordered by toon, path, locale; times are the store's `YYYY-MM-DD HH:MM:SS` (UTC) · `400 invalid_toon` |

The legacy `generate/*.php` endpoints (static `X-Generate-Token`) keep working unchanged for the `generate/` SPA until the
admin app has parity; then `generate/` and `GENERATE_PASSWORD` are removed.

## Reader avatar bodies & character unlocks (2026-09-30, owner — built 2026-09-30)

- The reader avatar body is a **character's A-pose** (character level, not persona) in one of its A-pose looks:
  `swim` (default, swim pants) or `naked`. Data lives in the character registry (`character.apose.looks[]` with
  path + wear slots, see "Slots per avatar body"); helper `avatarBodies(registry)` in `@scutoons/content`.
- **Members can only pick characters they have unlocked.** Server tables (to build):
  `member_characters (user_id, character_id, source: default|code|item|story|admin, ref?, unlocked_at)` and
  `member_character_looks (user_id, character_id, look_id, source, unlocked_at)` for A-pose looks beyond the default.
  A default-unlocked set is configurable (permissions/settings) — **owner decision 2026-09-30: only `catpope` (both A-pose looks swim + naked) is unlocked for every member by default**; unlocks can be granted by codes/items (catalog),
  story events (VN `add`/grant) and admins. `avatar` gets `body: { character, look }` (server, per account; guests
  per device).
- **No look creator:** outfitting (putting items on the body) happens only in the reader's inventory avatar view;
  the creator only stores A-pose images and wear slots as data.

**Implemented (2026-09-30):**
- Server: migration `2026-09-30-member-characters.{mysql,sqlite}.sql` (idempotent; after `settings`): `member_characters
  (user_id, character_id, source, ref, unlocked_at)`, `member_character_looks (user_id, character_id, look_id, source, ref,
  unlocked_at)`, `avatar_body (user_id, character_id, look_id, updated_at)`. `Domain\Characters\CharacterUnlocks`: the
  default unlocks are **one setting** `AVATAR_DEFAULT_UNLOCKS` (config/settings.php, JSON `{"catpope":["swim","naked"]}`,
  overridable per install like every setting) – no rows per member; a `member_characters` row unlocks the character with
  its **default** A-pose look, `member_character_looks` adds further looks. The registry is the published store file
  `scutoons/_/characters.json` (which looks exist and have an image). Sources `default|code|item|story|admin` + `ref` are
  stored; no code/item/story flows yet.
- API: `/me/ avatar` = `{ slots, body: { character, look }|null, unlocked: [{ character, looks, source }] }`;
  `PUT /avatar/ { body }` (with or without `slots`) → `422 locked` (not unlocked) / `no_apose` (no A-pose image),
  `400 invalid_body`; `body: null` = default. Admin: `PUT /admin/users/{id}/characters/{character} { looks?, source?, ref? }`,
  `DELETE /admin/users/{id}/characters/{character}[/looks/{look}]` → `{ changed, characters }`; `GET /admin/users/{id}`
  has `characters` (defaults + rows). Admin app user page: section "Avatar bodies" (list, unlock by id + extra looks,
  revoke). Contract: admin-smoke "Avatar bodies & character unlocks" (21 checks), PHPUnit `CharacterUnlocksTest`.
- Viewer: `components/avatar/avatar.svelte` = `unlockedBodies(avatarBodies(registry, locale), inventory.unlocked)`
  (registry via `fetchRegistry` of `@scutoons/api`, once per page load); thumbnails (A-pose + name + look label), locked
  characters hidden; worn items at the body's wear slots (since 2026-09-30 slot instances – "Slots per avatar body").
  Members save with `PUT /avatar/ { body }`; guests get `DEFAULT_AVATAR_UNLOCKS` (@scutoons/content, same list as the
  server default) and store `{ character, look }` in localStorage `scutoons_avatar_body` (old values sphynx /
  sphynx_nude / mouse are mapped; a locked choice falls back to the default).

## Slots per avatar body (2026-09-30, owner — built 2026-09-30)

Owner: "there is no rule for two… define slots per avatar, so an avatar can have the same slot twice; very useful for
twins or triplets."

- **Model** (`@scutoons/content` characters.ts): an A-pose look has `slots: AvatarSlotInstance[]` =
  `[{ id, slot, x, y, size, label? }]` (replaces the old `anchors: { <slot>: { x, y, size } }`). `slot` = the slot type
  (the catalog's `head, face, neck, torso, hands, legs, feet, back, accessory`), `id` = unique per body
  (`^[a-z0-9][a-z0-9-]{0,31}$`), `x/y/size` = anchor in image px (768 × 1152; size 0 = not drawn), `label` = the tile
  label per locale (none = the type's label). Single figures: one instance per type, `id` = type (catpope, yellowmouse).
  Toptoy (the duo): one instance per type and figure, `<type>-left` / `<type>-right`, labels "Kopf links" / "Head left" …
  (measured on the normalized A-pose; `hands` = a figure's outer hand, `accessory` = its inner hand).
- Helpers: `bodySlots(look.slots)` (a body without slots = one unplaced instance per type), `avatarBodies()` →
  `AvatarBody.slots`, `remapAvatarSlots(worn, target, typeOf)`. Validation: unique ids, id pattern, known slot types
  (error), numbers ≥ 0, the old `anchors` field is an error. Source of the values: `migrate-characters.ts`
  (`SPHYNX_SLOTS`, `MOUSE_SLOTS`, `TOPTOY_SLOTS` via `single()` / `twins()`); the creator shows them read-only (A-pose →
  "Technical: wear slots": id, type, x, y, size).
- **Server** (`AvatarService`): worn items are stored by **slot instance id** in `avatar_slots.slot` (no schema change,
  hence no migration: for single figures the instance id equals the old type value, so existing rows stay valid; rows
  that don't fit the current body – e.g. a legacy `torso` row on Toptoy – are read by type and rewritten on the next
  save). `GET /me/` / `GET /avatar/` → `avatar.slots` = every instance of the current body (chosen body, else the first
  unlocked default – `CharacterUnlocks::defaultBody`) → entry id | null. `PUT /avatar/ { slots: { <instance id>: entryId|null } }`:
  `422 invalid_slot` (no such instance on this body, + `slots`), `wrong_slot` (item type ≠ instance type, + `slot_type`),
  `not_owned`; one entry is worn once. **Body switch** (`PUT { body }`): an item stays on the instance with the same id,
  otherwise it goes to the first free instance of its type, otherwise it comes off (same rule in `remapAvatarSlots` for
  guests). `CharacterUnlocks::slotInstances(character, look)` reads the registry.
- **Viewer**: the inventory context loads the registry lazily (`loadRegistry()`), exposes `bodies`, `currentBody`,
  `slots` and the worn map by instance (`equippedEntry(id)`, `equip(entry, id?)` – without an id the first free instance
  of the item's type –, `unequip(id)`, `unequipEntry(entry)`); guests store instance ids in `scutoons_inventory` and are
  remapped on a body switch. `avatar.svelte`: one tile per instance (a multi-figure body puts each figure's tiles in its
  column by anchor x, smaller tiles), the picker filters by the instance's type, worn items at the instance anchors.
- Contract: admin-smoke "Slots per avatar body" (11 checks: invalid_slot, twin torsos, wrong_slot, one entry once,
  switching single ↔ duo, a legacy type row); PHPUnit `CharacterUnlocksTest::testSlotInstancesAndTheDefaultBody`; content
  tests "A-pose slot instances".

## Sensitive content (2026-09-30, owner — built 2026-09-30)

Owner: "Make it a member-wide setting. Hide sensitive content is on by default."

- **Data** (generic on purpose, only A-pose looks use it so far): `sensitive?: boolean` = `SensitiveFlag` in
  `@scutoons/content` (`sensitive.ts`: `isSensitive`, `hidesSensitive(user)`, `withoutSensitive(items, hide)`,
  `HIDE_SENSITIVE_DEFAULT`). `AposeLook extends SensitiveFlag`; **every `naked` look is `sensitive: true`**
  (`migrate-characters.ts`, `APOSE_LOOKS`, the creator's "start A-pose"); schema + validation (not a boolean = error);
  `AvatarBody.sensitive`. Other content can take the same flag later (not built).
- **Setting `hide_sensitive`, default on.** Members: column `users.hide_sensitive` (migration
  `2026-09-30-sensitive-content`, MySQL + SQLite, idempotent – the existing member-setting pattern of `marketing_opt_in`,
  no preferences table exists), `GET /me/ user.hide_sensitive`, changed with `PUT /me/ { hide_sensitive }` (alone or with
  `marketing_opt_in`; the answer adds `avatar`). **Guests: always on, no toggle** (a missing user counts as "hide").
- **Server:** `PUT /avatar/ { body }` with a sensitive look while on → `422 sensitive_hidden { character, look }`.
  Turning the setting on while wearing a sensitive look moves the body to the same character's first non-sensitive,
  unlocked look with an image (its default look first; worn items move like on any body switch; none → the default
  body) – `AvatarService::hideSensitiveBody`. A sensitive body stored earlier reads as that safe look while the setting
  is on (`AvatarService::body`); the default body skips sensitive looks.
- **Viewer:** the inventory context filters `bodies` with `withoutSensitive(…, hideSensitive)` (replaces the hard-coded
  swim-only filter and its "move to swim" effect of 5e2b10d); `currentBody` falls back to the same character's visible
  look. The picker shows only with more than one body; the look label under the name only when a character has more
  than one visible look. (Superseded 2026-10-01 by "Wardrobe": one tile per character, the naked base comes from taking the swim pants off.) Account page: checkbox "Sensible Inhalte ausblenden" / "Hide sensitive content" + one-line hint,
  under the news opt-in (plain `.opt-in` style).
- Tests: admin-smoke "(Hide sensitive content)" (default on, 422 sensitive_hidden, 400, off → naked, on → moved to swim
  and stored, a stored naked body reads as swim), api-smoke (new accounts: on), PHPUnit
  `CharacterUnlocksTest::testSensitiveLooksFollowHideSensitive`, content test "sensitive content".

## Cast & "featuring" (2026-09-30, owner — concept, build after data refactor Phase C)

The cast is an **editorial list**, not derived: non-VN chapters (plain vertical scroll) reference no characters, so a
cast computed from scenes would be empty there.
- **Season cast** (stored, as today): characters + the personas that appear. The toon's **star** (the character with
  `own_toon` = this toon, e.g. catpope → Cat Pope) is always in it — added automatically, cannot be removed.
- **Chapter "featuring"** (new, optional, small): list of season-cast character ids that appear in this chapter;
  pre-filled as a suggestion from the chapter's VN scene refs, set by hand for non-VN chapters.
- **Reader:** season page "Staffel 1 · mit Katzenpapst, …" / "Season 1 · featuring Cat Pope, …" (small avatars);
  chapter list item + chapter start show the chapter's "featuring".
  **Removed from the reader 2026-09-30 (owner: "the reader frontend is supposed to be super logical and simple"):** no
  featuring line, avatars or chapter-start band any more (`toon/featuring.svelte` deleted); the data, the creator field
  and `/cast?chapter=` stay for later (DESIGN.md "Toon page").
- **Creator:** character picker shows the season cast first; picking someone outside adds them to the season cast
  (undo); problem list suggests missing cast/featuring entries from VN usage. Cast page = the season cast editor (no
  "+ Season" there); chapter "featuring" is edited on the chapter page.
- **Implemented (2026-09-30):** `Chapter.featuring?: string[]` (@scutoons/content type + schema + validation: a list
  of ids, duplicates / ids outside the season cast are warnings); helpers `toonStar`, `ensureStar`,
  `chapterSceneCharacters`, `suggestFeaturing` (star + VN scene refs, first appearance), `featuredCharacters`,
  `featuringSuggestions` (info problems). Star: added in front of every season by the creator (cast page on load +
  save, chapter editor when it saves the cast; `castWithStar`), the characters generator (`migrate-characters.ts`) and
  the library `/cast` answer (`Library::withStar`, for documents saved before); locked in the cast editor ("Star" badge,
  no remove). Creator chapter page (Meta): featuring chips (order, remove), "Add …" (season cast first, others add to
  the cast with the undo notice), "Suggest from the scenes" (adds, never removes). Library: the season overview and the
  chapter pass `featuring` through; `/cast?chapter=N` includes the chapter's featuring. Reader: season line, chapter
  list items, chapter start (DESIGN.md "Featuring"). Data: catpope chapters 1 + 2 `featuring: ["catpope"]` (chapter 1:
  the star only until the owner names more; chapter 2: its scenes use no characters (narrator only) → the star).


## Wardrobe (2026-10-01, owner — built 2026-10-01 on branch `wardrobe`)

Owner: "clothing becomes real, exchangeable items that overlay the character's A-pose."

- **Default wearables** (starter items with a slot): `scutoons_tshirt` (torso), `scutoons_swim_pants` (legs),
  `scutoons_shorts` (legs), `scutoons_sandals` (feet). Every reader owns all four.
- **The basic dress** (owner 2026-10-01: "T-shirt + short pants + sandals") = the items marked `"default_worn": true`
  (T-shirt, shorts, sandals), put on when the wearables are handed out, per slot type, where nothing of that type is worn
  (`basicDress` in `@scutoons/content`, `AvatarService::basicDress` / `wearBasicDress`). **Rule per body:** for each slot
  type the `default_worn` item with an overlay for the body's character; if it has none, another starter of that type with
  one – so **yellowmouse and Top Toy (no shorts overlay) wear the swim pants** and their base never looks wrong; if no
  starter of the type has one, the `default_worn` item anyway (drawn as an icon: the T-shirt and sandals there). The body
  is the one the reader has at that moment (signup / guests: the default body, Cat Pope). The **swim pants** stay a starter
  in the inventory: the sensitive-content safety (cover rule below). Members: granted at signup (`Users::create` →
  `grantStarterWearables` + `wearBasicDress`), accounts older than the items get them lazily on `GET /me/` /
  `GET|PUT /avatar/` (`AvatarService::ensureWardrobe`: only items the member owns none of – wearables are never consumed,
  so no extra table). Guests: `giveGuestStarters` (once per item id, as before) dresses the same way.
- **Overlays** (`ItemDef.wear`, type + schema + `validateCatalog` in `@scutoons/content` `wardrobe.ts`):
  `{ "<character>": "<path>", "<character>/<slot instance>": "<path>" }` – a full-body image aligned to the character's
  A-pose (same 768 × 1152 canvas, transparent WebP). Lookup `<character>/<instance>` first, then `<character>`
  (`wearOverlay`). A body without an overlay for the worn item draws the item's **icon at the slot anchor** (as before;
  hoodies, tee, amulet keep that). Validation: keys must be a known character / slot instance of the item's type, `.webp`
  /`.png`, only wearables, a warning for one overlay on a multi-figure body, a warning when no starter covers `legs`.
- **Draw order** (`WEAR_ORDER`, bottom → top): legs → feet → torso → back → hands → accessory → neck → head → face; then
  the icon-only items (old order back … face) above all overlays.
- **Swap**: putting an item on an occupied instance takes the old one off; it stays an inventory entry (nothing is ever
  lost). The viewer says so ("SCUTOONS Badehose ist zurück im Inventar." – toast, a status line in the avatar view and
  in the item detail). **Worn items are not listed in the Items tab** (owner 2026-10-01; replaces the "Am Avatar" /
  "Im Inventar" sections): they are seen and managed in the Avatar tab's slot tiles only; taking one off makes it appear
  in Items again; a stackable entry with more pieces than are worn shows the rest (an entry is worn at most once).
- **Sensitive rule** ("Hide sensitive content" on = members by default, guests always): the cover slots
  (`SENSITIVE_COVER = ['legs']`) are never left empty – every save (`PUT /avatar/` slots or body, `PUT /me/
  { hide_sensitive: true }`) refills each empty legs instance with the first unworn legs item in catalog order (swim
  pants before shorts); the answer's `restored` lists them, the viewer notes "„Sensible Inhalte ausblenden“ ist an:
  SCUTOONS Badehose angezogen." So taking off the swim pants is effectively blocked, taking off the shorts puts the swim
  pants back. Same rule for guests in the inventory context (`coverSensitive`). With the setting off, legs may be empty
  (naked allowed).
- **Base image** (`wardrobeBase`): the character's sensitive (naked) look when it is unlocked and either the setting is
  off or every legs instance wears an item **with an overlay** for this body; otherwise the safe look (swim, pants painted
  in). While hidden the naked base is only shown once the legs overlays have loaded, on top of the safe look (a failed
  layer can never reveal a naked body). Twins with one pair of pants (or an item without an overlay) → safe base.
- **Bodies picker**: one tile per character (nakedness is the pants coming off, not a look). `body { character, look }`
  stays in the API; the look ids `swim`/`naked` stay in the registry (the safe base and the naked base).

### Files the owner replaces later (same names; placeholders made by `cdn/tools/make_wear_overlays.py`)

Convention: `characters/<character>/wear/<item>.webp` (single figure) or `…/<item>-<slot instance>.webp` (multi-figure
body), 768 × 1152 transparent WebP aligned to that character's A-pose; icon `scutoons/items/<item>.png` (512 px cut-out).

| File (CDN root) | Size | Source / method |
|---|---|---|
| `characters/catpope/wear/scutoons_swim_pants.webp` | 8 KB | swim − naked difference |
| `characters/catpope/wear/scutoons_tshirt.webp` | 14 KB | the reference `catpope_apose-example-dressed.webp`, colour segmentation (low saturation) |
| `characters/catpope/wear/scutoons_shorts.webp` | 16 KB | the same reference, colour segmentation (denim blue) |
| `characters/catpope/wear/scutoons_sandals.webp` | 10 KB | the same reference: brown/cork below the ankles, each sandal moved + scaled onto the naked foot |
| `characters/yellowmouse/wear/scutoons_swim_pants.webp` | 9 KB | swim − naked difference |
| `characters/toptoy/wear/scutoons_swim_pants-legs-left.webp` | 16 KB | swim − naked difference, left half |
| `characters/toptoy/wear/scutoons_swim_pants-legs-right.webp` | 14 KB | swim − naked difference, right half |
| `scutoons/items/scutoons_tshirt.png` | 122 KB | the catpope overlay, trimmed, 512 px |
| `scutoons/items/scutoons_swim_pants.png` | 116 KB | the catpope overlay, trimmed, 512 px |
| `scutoons/items/scutoons_shorts.png` | 190 KB | the catpope overlay, trimmed, 512 px |
| `scutoons/items/scutoons_sandals.png` | 96 KB | the catpope overlay, the pair side by side, 512 px |

**Missing overlays (asset requests, the icon is drawn meanwhile – `missingWearOverlays(catalog, registry)` lists them):**
`characters/yellowmouse/wear/scutoons_tshirt.webp`, `characters/yellowmouse/wear/scutoons_shorts.webp`,
`characters/toptoy/wear/scutoons_tshirt-torso-left.webp`, `…-torso-right.webp`,
`characters/toptoy/wear/scutoons_shorts-legs-left.webp`, `…-legs-right.webp`,
`characters/yellowmouse/wear/scutoons_sandals.webp`, `characters/toptoy/wear/scutoons_sandals-feet-left.webp`,
`…-feet-right.webp` – plus every physical wearable (Dago hoodie /
heavy T, kangaroo hoodie, scorpion amulet) on every body. They are not stored as `asset-requests` entries (that would
change the release seed); add them in the creator's wanted images if the owner wants them listed there.

**Extraction quality:** swim pants = clean (the two A-poses share one transform; fur/skin texture noise averaged out by
blurring the images before the difference, thin attachments like a tail drawn differently dropped); the Top Toy pants
carry a 2–4 px fringe of the figure's own fur (invisible over the same body). T-shirt/shorts from the dressed reference:
the upper body lines up with the naked A-pose within 1–3 px, the shirt fits; the reference's legs stand wider than the
A-pose's, so the shorts hang a little wide over the thighs (reads as loose shorts). The sandals: the reference's feet
stand wider, turned out and 20 px higher than the naked A-pose's, so each sandal (brown/cork mask below y 950, the holes
between the straps filled – the reference's toes come along and cover the cat's claws) is moved and scaled onto the naked
foot (width = foot + 24 px, sole on the sole); it reads as a foot in a sandal in a slight 3/4 view over a front-facing
foot, the sole edge keeps a few ragged shadow pixels.
Placeholders only – the owner's branded artwork replaces them.

**Tests:** content `test/wardrobe.test.ts` (catalog vs registry, validation errors, overlay lookup, the basic dress per
body, cover rule, base image), PHPUnit `WardrobeTest` (new account wears the basic dress, basic dress per body, lazy grant
once – the swim pants on Top Toy –, swap keeps the entry, cover rule, setting off/on, twins), api-smoke (a new account wears
the basic dress, the swim pants wait), admin-smoke "Wardrobe" (9 checks), `npm run check:wardrobe` (headless, 390 + 1440,
guest + test user, 24 checks: the basic dress, swim pants on → shorts back + the Items tab lists them and none of the worn items, swim pants stay on while hidden,
item detail swap, setting off → naked, Top Toy twins, back to Cat Pope). `npm run dev:test-user -- --reset` removes the
default wearables so the next `/me/` hands them out and puts the basic dress on, like a signup.

**Merge note (after the 2026-10-04 release):** no migration, no seed change. Deploy order: CDN first (`cdn/sync.py` the
`characters/*/wear/` folders and `scutoons/items/` – run `cdn/tools/make_wear_overlays.py` on the owner's `cdn/root`
first, the branch could not write there), then the API (items.json + services), then the viewer. Existing members get
the four wearables on their next `/me/` with the basic dress (T-shirt, shorts, sandals – the swim pants on a body
without a shorts overlay) worn where those slots were free. The bodies picker no longer
offers "nackt" as a separate look.

## Substances, reader stats, testo (2026-10-01, owner — built 2026-10-01 on branch `wardrobe`)

Owner: "There is no male/female. Instead a testo value (a slider): the more testo, the more [gender-]read
characteristics." Then: "other drugs too, taken at any time" → one small **substances** model, not a one-off. Game
mechanics only: no dosing, no real-world use instructions, no claims about real effects in any text.

- **No male/female anywhere.** There was no gender/sex field in the types, seed, creator or people (checked); none added.
  Characters carry an optional `testo` level 0–100 (`RegistryCharacter.testo`, per-persona override
  `CharacterPersona.testo`; validation 0–100; creator character page: a slider next to the colour + "✕" = unset, same
  for the persona). **No values are set in the content** (unset = undefined). `characterTesto(c, persona)` reads it,
  `testoBand(level)` → `testo-low | testo-mid | testo-high` (< 34 / < 67 / ≥ 67) is the hook for future A-pose / portrait
  variants – **no visual change yet**. **Direction (owner 2026-10-01): more testo → the character reads more male, less
  testo → more female** (`testo-low` = female-read, `testo-mid` = ambiguous, `testo-high` = male-read); the art variants
  follow that later.
- **Character tendencies** (`RegistryCharacter.tendencies: string[]`, creator field "Neigungen"): story/workflow
  reference only, no runtime effect. Top Toy: `["psychedelics"]` (owner: "tends to take a lot of psychedelics";
  `migrate-characters.ts`, the characters seed, the characters statement of both seed SQL files).
- **Reader stats** (`catalog.stats` in `server/config/items.json`): `testo` and `appetite` (a placeholder stat, no
  gameplay yet), each `{ label, baseline: 50, half_life_days: 3 }`. **Neutral start 50.** **Decay:** the level drifts back
  to the baseline, `level(t) = baseline + (stored − baseline) · 0.5^(days / half_life_days)` – computed lazily on read from
  `{ level, updated_at }`, no cron (one constant per stat, in the catalog). Members: table `member_stats`; guests:
  localStorage `scutoons_reader`. `GET /me/ stats` = `{ <stat>: { level, updated_at } }` = the level **as of** `updated_at`
  (= now, decayed, 0.1 steps), so the reader keeps decaying it from there.
- **Substance items** (`category: "substance"`, consumable, stackable, skill `{ "type": "apply" }` = "Benutzen"):
  `effects: [{ stat, delta }]` (current level + delta, clamped 0–100) and/or `effect: { kind, duration_min }` (a timed
  reader effect; a running one of the same kind is replaced). Validation: known stats, delta ≠ 0, duration > 0, a
  substance needs `apply` and `apply` needs effects.

  | id (placeholder names, owner may rename) | what | grantable |
  |---|---|---|
  | `testo` | testo +20 | story, scan |
  | `psilocybin` ("Zauberpilz") | effect `psychedelic` 20 min | story |
  | `lsd` ("Bunte Pappe") | effect `psychedelic` 45 min | story |
  | `semaglutide` ("Semaglutid-Pen", generic name only) | appetite −20 | story |
  | `weed` ("Gras" / "Weed") | appetite +25 (no reader effect: kept simple, owner may add a mild one later) | story, scan |
  | `testo_camera` (tool) | `scan_object` labels `lotion`, `sunscreen`, `pill bottle` → grants `testo`, cooldown 10 s | – |

  **Food** (owner 2026-10-01: "some drugs like weed make you want to eat; both degrade"): `category: "food"`, the same
  `apply` skill + `effects` with a negative appetite delta; the button says **"Essen" / "Eat"** instead of "Benutzen"
  (`items.eat`), the toast "Gegessen: Gurke". `cucumber` −5 (eaten one at a time, still a recipe ingredient),
  `tuna_can` ("Thunfischdose" / "Can of tuna", consumable, stackable, grantable story + scan) −30. Clamped 0–100 like
  testo; both directions drift back to 50 (the existing decay). No item places them yet (story/scan grantable; no camera
  maps them – ImageNet has no cannabis class).

  Recipe `testo_camera` = `camera` + `testo` (like the cucumber camera: the starter camera's QR scan is traded for it).
- **API:** `POST /inventory/use/ { item_id }` → `{ status, item_id, stats, effects, inventory }` – consumes one piece and
  applies it in **one transaction** · `422 missing` (none owned), `not_usable`, `404 unknown_item`.
  `POST /effects/end/ { kind }` → `{ status, ended, effects }` ("Effekt beenden"; a POST, so the reader CORS methods stay
  GET/POST/PUT). `GET /me/` adds `stats` and `effects: [{ kind, until }]` (running only). Server:
  `Domain\Reader\ReaderState`.
- **Psychedelic effect (viewer):** `components/items/reader-effect.svelte` (toon layout): `html[data-effect=psychedelic]`
  makes the slice images drift in hue and wobble gently (CSS); **off with `prefers-reduced-motion: reduce`**; a small bar
  "Farbrausch · noch N min · Effekt beenden" ends it at once. Stat bars: `reader-stats.svelte` (`include` / `exclude`, `compact`): **testo and
  appetite sit small at the right of the inventory's tab row** (seen on both tabs; on ≈ 320 px phones they wrap below
  the tabs, still right) and **at the right of the account page's "Avatar" heading** (owner 2026-10-03: "less massive",
  replaces 2026-10-01's big testo bar under the body picker): "Testo 62" + a thin 2.5rem bar, no gendered wording.
- **Story pickup:** one `testo` in the sandbox, chapter 2 (the Top Toy encounter), slice 6: a click on the stone ledge
  right of the figure (the art has no tube drawn in) → "Eine Tube Testo!" / "Einsammeln" (`add testo 1` → the VN item
  bridge grants it, source story, once per place). `make-sandbox.ts` (`TESTOS`), the sandbox seed (dev only; not in the
  production seed SQL).
- **Testo camera detection – the limits, honestly:** the camera uses MobileNet v2 (ImageNet 1000 classes) on the device,
  like the cucumber camera. ImageNet has no "testosterone" or "gel tube" class; the closest are `lotion`, `sunscreen` and
  `pill bottle`, so the testo camera reacts to **tubes and small bottles in general** (hand cream, sunscreen, pill jars),
  not to testo specifically, and only when the object fills the centre square (threshold 0.35, 3 frames). Game fiction:
  "every find is a tube of testo in the game".
- **Check It camera** (`checkit_camera`, "Check It Kamera" / "Check It camera"; owner 2026-10-01 – the name is a
  deliberate nod to checkit!, the real anonymous drug-checking service of Suchthilfe Wien; no ™, no logo or branding
  imitating it): a tool with a multi-label `scan_object` – `labels` → `grants` plus `map: [{ labels, grants, name }]`
  (`scanTargets` / `matchScan` in `@scutoons/content`: the best prediction across all targets wins; the server's scan
  grant honours the map, `Catalog::scanTools`). **Obtained by a recipe** (owner 2026-10-01): `checkit_camera` =
  `camera` ×1 + `testo` ×1 (the owner may change the ingredients later; they are plain catalog data). The same two items
  also make the testo camera, so the combine view asks "Was soll daraus werden?" with one button per result
  (`recipe-preview.svelte`, default = the first recipe). `grantable: ["story"]` stays as well (a story/admin grant
  remains possible; it is placed nowhere). Mapping – **only labels MobileNet v2
  (ImageNet) really has**, cooldown 15 s:

  | model labels (ImageNet) | recognised as (game text) | grants |
  |---|---|---|
  | `pill bottle`, `lotion`, `sunscreen` | "Pillendose / Tube" | `testo` |
  | `syringe` | "Spritze" | `semaglutide` (ImageNet has no injection pen; a syringe is the closest) |
  | `mushroom`, `agaric` | "Pilz" | `psilocybin` (any mushroom – the model can't tell kinds of mushroom apart for this) |
  | `envelope` | "Briefchen" | `lsd` (any envelope; ImageNet has no blotter paper) |

  Limits: it recognises **everyday objects**, never what is in them; only the centre square, threshold 0.35, 3 frames in a
  row. The scan result is game text only – "Erkannt: Spritze → Semaglutid-Pen" / "Recognised: syringe → Semaglutide pen",
  never "analysed", "contains" or "safe". `psilocybin`, `lsd`, `semaglutide` are now also grantable by `scan` (validation:
  every item a camera grants must be).
- **Mandatory notice** in the camera UI, every time a camera that finds substances opens (testo camera and Check It
  camera; compact, not dismissible): "Spiel-Kamera. Analysiert keine echten Substanzen. Echtes, anonymes Drug-Checking:
  checkit! (Wien)" / "Game camera. Does not analyse real substances. Real, anonymous drug checking: checkit! (Vienna)",
  linking **https://checkit.wien** – verified 2026-10-01 as the official site ("checkit! Kompetenzzentrum für
  Freizeitdrogen", footer "checkit! Suchthilfe Wien gGmbH, Gumpendorfer Straße 8, 1060 Wien"; the press also names
  www.checkyourdrugs.at). Object cameras other than the cucumber camera show their item name as the title and generic
  hints ("Nächster Fund in 15s").
- **Browser-check hook (DEV only):** `globalThis.__scutoonsFakePredictions` makes `loadClassifier()` return those
  predictions instead of downloading MobileNet (with a canvas `getUserMedia` stub the object loop runs headless); never
  active in a production build (`import.meta.env.DEV`).
- **Icons** (placeholders in the house style, `cdn/tools/make_substance_icons.py` → `scutoons/items/`, to replace):
  `checkit_camera.png` 31 KB (the camera with a small test tube, no logo), `testo.png` 41 KB (a squeeze tube), `testo_camera.png` 45 KB (the camera with a tube), `psilocybin.png` 29 KB (a
  mushroom), `lsd.png` 84 KB (a blotter square with a spiral), `semaglutide.png` 33 KB (a generic pen, no product look), `weed.png` 52 KB (a zip bag with green buds),
  `tuna_can.png` 60 KB (a generic flat tin with a ring pull and a fish outline, no brand). The tool takes icon names to
  render only those (`… --root cdn/root weed tuna_can`).
- **Tests:** content `test/substances.test.ts` (catalog, decay, apply + cap, effect expiry/replace/end, testo bands,
  character validation), PHPUnit `ReaderStateTest` (decay math, apply + cap + decay with the frozen clock, semaglutide,
  the timed effect's expiry + end, recipe + scan grant + cooldown, story pickup once, weed / cucumber / tuna can on the
  appetite incl. cap, floor and decay), api-smoke "Substances" (16 checks: + weed 50 → 75, tuna −30, cucumber −5),
  content "appetite" test (the same + food category, grantable),
  `npm run check:testo` (`scripts/check-testo.mjs`, headless 390 px, guest + test user, 14 checks: sandbox pickup,
  testo + appetite in the tab row on both tabs, "Benutzen" 50 → 70, "Essen" on a cucumber → appetite −5,
  decay → 60, combine → testo camera (+ scan grant and cooldown for the member), psychedelic on the
  slices / off with reduced motion / ended; camera + testo → the Check It camera through the recipe choice, its notice with the checkit! link on open and its hit
  "Erkannt: Spritze → Semaglutid-Pen" + the grant, through the DEV hook – 14 checks in all). Content test "Check It camera"
  (mapping, best match, the scan-grantable rule), PHPUnit + api-smoke (the map's scan grants + cooldown, no_tool).
- `npm run dev:test-user -- --reset` also clears the stats/effects and the story pickup refs and gives back a starter
  camera a recipe used up.

**Merge note (substances):** migration `server/migrations/{mysql,sqlite}/2026-10-01-reader-state.*.sql` (two new tables
`member_stats`, `member_effects`, idempotent, after `2026-10-01-embassy`). The characters statement of
`characters-seed.mysql.sql` / `content-seed.mysql.sql` carries Top Toy's `tendencies`; the seed skips an existing
`characters` entity, so on production publish the registry once from the creator (or `import-content
--only=characters --update`). CDN: `cdn/tools/make_substance_icons.py`, then `cdn/sync.py` (`scutoons/items/`).
The Check It camera needs nothing extra on deploy (catalog + viewer): it is crafted from camera + testo (recipe
`checkit_camera`, ingredients adjustable in `server/config/items.json`), a story grant stays possible.

## Collection limits & object-camera rules (2026-10-02, owner)

**Built:**
- **Collection limit:** at most **99 of any item per account** (`MAX_PER_ACCOUNT`, server `InventoryService::MAX_PER_ACCOUNT`).
  An item can set less with `max` (catalog). The server refuses more with 409 `max_reached` (grant and craft output);
  `add` caps silently (imports). A claimed physical product (QR code) is always added. Guests are capped locally.
- **Camera cooldown:** cucumber camera `cooldown_s: 60` (owner: "1 minute and we are cool").
- **Daily limit:** `per_day` on a `scan_object` skill = scan grants of that item per account in 24 h (cucumber camera
  3; 429 `daily_limit`, "Genug Gurken für heute").
- **"Die kenn ich schon!":** the camera keeps a picture fingerprint (MobileNet embedding; numbers, not an image) of each
  granted scan on the device only (`viewer/src/lib/components/camera/scan-memory.ts`, last 20, 7 days); a hit that
  looks ≥ 0.88 alike is not granted and the reader is told it is cheating. **No photos and no location** (owner
  2026-10-02: privacy) – it recognises "this cucumber in this place", which is the farming case.
- **Recognition:** only the frame under the aiming square is classified; cucumber + zucchini count together
  (`scoreScan` / `scanPasses`).

- **QR codes are unique (owner 2026-10-02, "important relation"):** one code, one account. **Why:** it ties the real
  world to the virtual one – the physical product and its virtual item are one thing with one owner, and that
  uniqueness is the guarantee (no copies). Never weaken it (no "share a code", no second claim, no admin copy). A code registered to
  another account can't be claimed (409 `already_claimed`, race-safe `UPDATE … WHERE claimed_by IS NULL`, api-smoke
  checks it). Physical products are therefore the existing "exists only once" items; reading with someone else's code
  is still allowed by the access model (device access, "Access model" above) unless the owner changes that.

**Planned (not built – no such item exists yet):**
- **Unique items:** some items exist **only once in the world**; if anybody else owns it, nobody else can get it
  (grant/craft/claim refuse, e.g. 409 `taken`). Needs a catalog flag (e.g. `unique: true`, or `max_world: <n>` for
  "only N exist"), a server check across all accounts inside the grant transaction, and a decision on what happens when
  the owner's account is deleted (back into the world?) and whether it can be given away.
- **Rarer items** set a lower `max` (built) – which items and how many is the owner's call per item.

## Certificates (2026-10-02, owner)

**What a certificate is (owner):** certificates (`SCUTOONS-COA-<code>.pdf`) are pre-generated in batches and **signed
to a buyer by hand** at purchase – the "Original buyer" field (`receiver`) is left empty on purpose. The certificate
plus the piece are the **proof of ownership, as in the art world, even if our servers were gone**; our ML-DSA-65
signature lets anyone validate that the document is authentic, so it cannot be forged. Each certificate prints the
code (UPPER CASE until 2026-10-03, since then exactly as it is – codes are case-sensitive), the fingerprint (first 32 bytes of the signature, `ABCD:EF01:…`) and "Verify at
https://qr.scutoons.com/verify.html with code: <CODE>".

**Fix (2026-10-02, owner: "it just checked if the signature is a proper signature, not if it also matches an entry in
the db"):** the old verify page verified a signature against the public key **embedded in the same JSON** → anyone
could sign their own data with their own key, upload it and get "✅ Signature Valid"; it never compared with our
records (the admin's "signature valid" badge had the same flaw). Now:

- **Authentic = signed with OUR published key (`/api/keys/keys.php`, KEYPAIR_PATH) AND it is the signature we stored
  for that code (`qr.signature`).** Verified **server-side** in plain PHP (`server/src/Infrastructure/Signing/MlDsa65.php`
  + `Shake.php`, FIPS 204 verify, external/pure with empty context = what noble's `ml_dsa65.sign(msg, sk)` signs;
  cross-checked against noble fixtures and all 421 stored signatures of the dev DB; ~50 ms per check). No PHP
  extension/Composer package does ML-DSA (OpenSSL < 3.5 on the host).
- Service `Domain/Certificates/CertificateVerifier`, controller `CertificatesController`:
  `GET /api/certificates/{code}` (also `?code=`/`?c=`, a pasted QR or verify URL) and `POST /api/certificates/verify`
  (the uploaded file). Per-IP lookup limit (`lookup_limit_per_hour`, shared with `/access/`), CORS `*`, the qr app
  always proxies `/api/certificates/*` (also with `PROXY_FULL_API=0`).
- **Answer** (200 for every verification result): `{ status:"success", valid, reason, source:"code"|"file", code,
  registered, revoked, certificate }`. `certificate` = **our record's** signed data (only when the stored signature
  itself is authentic): `{ code, type, product_name, edition_number, edition_count, creators, created_at, signed_at,
  fingerprint, algorithm }`. `registered` = claimed by an account (or the old email form), `revoked` = admin revoked –
  **never who owns it** (no email, no user id, no revoke reason). `code`/`registered`/`revoked` are null for an unknown code.
- **Reasons** (`valid: false`): `unknown_code` (also a code typed in another case) · `no_signature` · `key_mismatch` (not our key → not issued by SCUTOONS) · `bad_signature` (does
  not verify against our key: tampered / forged) · `signature_mismatch` (signed by us but not the signature on record
  for that code, e.g. an older re-signed file, or the stored one is for another code). A revoked code with an
  authentic certificate is `valid: true, revoked: true` (the page shows "Revoked"). Errors: 400 `invalid_code` /
  `invalid_signature_file` / `invalid_json`, 429, 503 `keys_missing`.
- **Case (owner 2026-10-03: "make it case sensitive … stick to one rule"):** exact match only, like every other code
  lookup – no case-insensitive fallback, no `ambiguous_code` any more. Certificates printed before 2026-10-03 show the
  code in UPPER CASE: typing that form gives `unknown_code`; the QR on the certificate (exact code) and the signature
  file upload still verify. New certificates print the exact code (admin `pdf.ts`, legacy `generate/` `pdf.js`).
- **Verify page** (`qr/public/verify.html`): asks the API only (no client-side crypto any more), de/en (`?lang=` or the
  browser language), `?code=` checks right away; states Authentic / Revoked / Not issued by SCUTOONS / Not authentic
  / Does not match our records / Unknown code / …; shows the registration status ("Registered to an account" / "Not
  registered yet – register it in your account, otherwise someone else can" + link `./?c=<code>`), the fingerprint to
  compare with the printed one; for a file that is not ours but names a known code, what our record says. Output is
  escaped (the old page put uploaded JSON into innerHTML). "Download Signature File" still loads
  `generate/get-signature.php` (kept public for that and for old links).
- **Admin** `crypto.ts` `verifySignature(signature, code?)` (async) verifies against the server keypair's public key
  and the code; the certificate panel's badge uses it.
- Tests: `tests/Unit/Infrastructure/MlDsa65Test.php`, `tests/Unit/Http/CertificatesApiTest.php` (fixtures
  `tests/Unit/fixtures/ml-dsa-65.json`, made by noble 0.5.4 with fixed seeds – test keys only), api-smoke
  "Certificates" (real stored signatures by their exact code, tampered, foreign key, revoked), qr public-urls (verify page,
  proxied identical to the API).

## Ownership, claims, gifting (2026-10-02, owner — built 2026-10-02 on branch `platform`)

**Owner decisions (final):**
- **QR codes are unique:** one code = one owner account; that ties the real world to the virtual one and is the
  uniqueness guarantee. Never copies, never a second claim. **Reading with someone else's code is allowed and
  encouraged** (sharing, a concert neighbour scanning it) – the access model is unchanged.
- **Certificates** are pre-generated and **signed to the buyer by hand** at purchase ("Original buyer" empty on
  purpose). As in the art world the certificate + the piece are the proof of ownership, even without our servers; our
  signature ("Certificates") proves a document is authentic. Pieces can be passed on (resale, gift) – **the platform
  records the ownership change**.
- **The problem:** a buyer who never registers the code can lose it to someone faster. People file a **claim** with us;
  admins move ownership between accounts. **Evidence is optional** (any receipt / photo / the certificate; no Bandcamp
  receipt required).
- **Registration strategy:** push registration at the moment of purchase / first scan (below).
- **Mails:** claim filed → the claimant (received) + the admins (new claim); ownership moved → the previous and the new
  owner.
- **Gifting** in the reader inventory: readers give their digital items to other readers ("beautiful for game play").
  No money, no marketplace (a second-hand market is a separate future topic, agents/research/second-hand-market.md).

### Data (migrations `2026-10-02-ownership`, `2026-10-02-ownership-signup`; sqlite + mysql)

- `ownership_claims` (id uuid, code, claimant_id, owner_id = owner when filed – NULL for an old email-form
  registration, statement, status `open|transferred|rejected|withdrawn`, admin_id = who decided, admin_note (internal),
  created_at, decided_at). One open claim per claimant + code (API check).
- `ownership_claim_files` (id, claim_id, filename under `FILES_PATH/claims/<claim>-<id>.<jpg|png|webp|pdf>`, the uploaded
  name, content_type – sniffed from the first bytes, never the client's word –, size, created_at). Admin-only download.
- `item_transfers` – **the transfer log** of every ownership change: kind `claim|gift|admin`, code (NULL = a virtual
  gift), item_id, variant, qty, from_user_id, to_user_id, by_user_id (the admin, or the giver), claim_id, note,
  created_at, seen_at (the recipient's notice). Append-only, no foreign keys (the record outlives an account).
  `code_events` was **not** extended (its admin views are anonymous statistics; the history needs accounts).
- `login_requests.dispute` (signing up to file a claim, below).

### Transfer core (`Domain\Ownership\OwnershipTransfers::moveCode`, one transaction)

The DB guard `qr_owner_immutable` forbids A → B in one UPDATE, so the code goes **A → NULL → B**; then every inventory
row with that code moves to B (the guard `inventory_code_owner_update` checks they follow the owner; source `qr`, or
`gift`; `acquired_at` = now), A's avatar slots that wore them are cleared, `qr.email` / `claimed_at` follow. A code that
was never registered to an account (free, or the old email form) is **claimed** for B instead (its items are added; a
code with options needs the item choice → `422 choose_item`). A physical product is always added (collection limit
rule). A keeps reading with the code on their devices (device access ≠ ownership). Everything is recorded in
`item_transfers` with who acted.

### Reader API (session; ITEMS "HTTP API contract" style)

| Method | Path | Body | Response |
|---|---|---|---|
| POST | `/claims/` | `{ code, statement }` (≥ 10 chars) | `201 { status, claim }` · `409 not_registered` (nobody owns it – just add it) `\|already_yours\|claim_open { claim }` · `422 not_claimable` · `429 rate_limited` (`CLAIM_MAX_PER_DAY` 5) · `400 statement_required` |
| POST | `/claims/{id}/files?name=` | the raw file (JPEG, PNG, WebP, PDF) | `{ status, claim }` · `413 file_too_large` (5 MB) · `415 unsupported_type` · `409 too_many_files` (5) `\|claim_closed` |
| GET | `/claims/` | – | `{ status, items: [claim] }` |
| POST | `/claims/{id}/withdraw` | – | `{ status, claim }` · `409 claim_closed` |
| POST | `/inventory/gift/` | `{ entry_id \| item_id (+ variant), qty?, to: email }` | `{ status, gift: { item_id, variant, qty, code, to, transfer_id }, inventory }` |
| POST | `/inventory/received/seen/` | `{ ids? }` | `{ status, seen }` |
| GET | `/me/` | – | + `received: [{ id, kind: gift\|claim\|admin, item_id, variant, qty, code, from, created_at }]` (unseen; `from` = the giver's email, gifts only) |
| POST | `/auth/request/` | + `dispute: true` | with a product code registered to someone else: a **new account may be created to file the claim** (nothing claimed, `/auth/verify/` → `dispute: true`); expired / revoked / non-signup codes still refuse |

`claim = { id, code, status, statement, files: [{ id, name, type, size }], created_at, decided_at }` – the claimant
**never learns who owns the code**, and never sees the admin note.

### Gift rules (`Domain\Ownership\Gifts`; agents/research/second-hand-market.md stage a)

- **Giftable** = the catalog's `giftable`, default true **except** starters / the basic dress (`starter`,
  `default_worn`) and membership / access-only items (`category` membership | access) – `Catalog::giftable` /
  `isGiftable` (@scutoons/content). → `403 not_giftable`.
- **Worn items must be taken off first** (`409 worn`); a stackable keeps its worn piece back.
- **Re-gift cooldown** `GIFT_COOLDOWN_HOURS` (24): what arrived (gift, claim, admin move) in the window can't be passed
  on (`409 cooldown { retry_after }`; a code: its last move into the account; other items: the pieces received).
- **Daily limit** `GIFT_MAX_PER_DAY` (10 gifts / 24 h, `429 daily_limit`) and `GIFT_LOOKUPS_PER_DAY` (30 recipient
  lookups / 24 h – unknown addresses count too, against probing which emails have an account; `429 rate_limited`).
- **Recipient must have an account** – decision: **refuse** (`404 recipient_not_found`, "ask them to sign up first"),
  no pending gifts: a new account needs a product or promo code anyway, so a pending gift would mostly never be
  collected. The giver is told the recipient sees their email address.
- **Recipient collection limit** (`409 max_reached { max }`), **no gifts to yourself** (`400 self_gift`), stackables
  `qty` 1…owned, a code-bound piece qty 1.
- **Code-bound items** (a physical product's twin) move **the whole code** (all its entries) through the transfer core,
  kind `gift`.
- Every gift is in `item_transfers`; the recipient gets the mail `gift_received` and a notice (`/me/ received` → the
  viewer's toast "X schenkt dir Y"; entry source `gift`).

### Admin API (`/api/admin`, role admin; server docs/API.md "Admin API")

`GET /claims` (list: status, code, user, q; `open` count) · `GET /claims/{id}` (claim, the code detail, **current owner
account – shown here on purpose for admins handling claims**, `legacy_registration`, the certificate check
(`CertificateVerifier::byCode`), the code's history, other claims) · `POST /claims/{id}/transfer { note }` (required) ·
`POST /claims/{id}/reject { note }` (required, internal) · `GET /claims/{id}/files/{fileId}` · `GET
/codes/{code}/history` · `POST /codes/{code}/transfer { to: email | id, note, item_id?, variant? }` (outside a claim, kind
`admin`). The codes list stays private (no owner, no emails).

### Mails (the email module)

`ownership_claim_received` (claimant), `ownership_claim_admin` (sendToAdmins, link `ADMIN_URL/claims/<id>`),
`ownership_claim_rejected` (claimant; the note stays internal), `ownership_transferred_from` (previous owner: "you can
still read with the code"), `ownership_transferred_to` (new owner: keep the certificate with the piece),
`gift_received` (recipient). A failed mail never undoes a transfer (it is in the mail log).

### Registration strategy

- **QR landing page** (`qr/`): an unregistered product code shows **"Add to my account" first** with "Register it now –
  otherwise someone else can"; a registered one shows **Read** + **"This is mine? File a claim"**
  (`redirect_to?code=…&claim=1` → the viewer's claim form). It never says whose it is.
- **Viewer claim sheet:** the same warning on an unregistered code; on someone else's code "You can still read with it"
  + "This is mine" (statement, optional proof files; a reader without an account signs up for it with `dispute`).
- **Verify page:** already says "Not registered yet – register it, otherwise someone else can" + link.
- **Certificate template** (`admin/src/lib/qr/templates/cos.md`, future batches – existing certificates are not
  regenerated): "Register this piece in your SCUTOONS account right away …". Note: the admin's certificate
  "Regenerate" uses the current template, so a regenerated old certificate gets the line too (the signed data is
  unchanged).
- Ideas not built (owner's call): a registration card / sticker in the package ("scan me first"), the shop's order
  mail linking `qr.scutoons.com/?c=CODE`, a reminder on the account page for codes scanned but not registered.

### UI

Viewer: claim sheet (above), item detail **"Verschenken" / "Give away"** (email, qty for stackables, a permanent-and-free
note, code note), toasts for received gifts / codes moved here, account page **"Meine Ansprüche" / "My claims"**
(status, add proof, withdraw). Admin: **Claims** page (codes group) + claim detail with transfer / reject, the code page's
ownership history and "Transfer ownership".

### Tests

PHPUnit `OwnershipTest` (DB guard still refuses A → B, transfer moves entries + clears avatar slot, admin transfer of a
free code, claim lifecycle with files + mails, reject / withdraw, claim limit, gift rules: stackable, notices + seen,
cooldown, not giftable, max_reached, invalid qty, daily limit, worn, code-bound gift moves the code, works before the
migration), `OwnershipApiTest` (HTTP: claim flow incl. raw upload + admin download, gift + `/me/ received`),
`LoginServiceTest` (dispute signup), `MailTest` (every template renders); api-smoke "Ownership claims + gifts",
admin-smoke "Ownership claims + transfers"; qr `public-urls` (registered → file-claim link, free → register now).

## Where to buy – stores carrying products (2026-10-02, owner — branch `stores`)

Physical products (`config/products.json`) can also be bought in real shops: the Embassy's stores (type `store` /
`record_shop`) carry product ids, and the listed ones appear on scutoons.com/stores, independent vendors first. Next to
"Kaufen" (toon page related row, landing editions) a product a store carries offers "Läden vor Ort" →
`/stores?product=<id>`. A store carries **products**, not items or editions: the product is what is sold (item + edition
come with it). Model, API, UI, tests, open decisions: agents/EMBASSY.md "Stores".

## Developing items and recipes (packs) (2026-10-02, owner – rule)

**Why.** Items and recipes are where most new ideas land (cucumber cross, Check It camera, onion camera, auto flute …),
often close to a release. They must never put a release at risk, and a finished idea must be easy to patch in later.

**Two layers.**
- **Fundamentals** = code that defines what items *can* do: skill types (`scan_qr`, `scan_object`, `apply`, `craft`,
  `play_audio`/players …), the camera and its recognition plumbing, inventory, collection limits, ownership, gifting,
  avatar slots, the item UI. They change only through the platform release flow (`platform` → `main`, full suite green).
- **Content (a pack)** = data on top of released fundamentals: item definitions, recipes, variants, icons and
  overlays, de/en texts, trained model weights for an existing model type, story grants in the seed.

**Rules.**
1. A new set of items/recipes is a **pack** with a short name, built on its own branch `items/<pack>` in a separate
   git worktree (e.g. `C:\Development\scutoons-<pack>`), branched from `platform` (or `main` after a release). Never
   on the branch that is being released.
2. A pack only **adds**: new catalog entries (`server/config/items.json` items + recipes), CDN assets
   (`cdn/root/<toon>/items/…`, `cdn/root/models/…`), texts, story grants (seed statements), tests. It never changes
   the id or meaning of a released item – inventories reference ids forever; a rename is a new item plus a migration.
3. If a pack needs something the fundamentals can't do yet (e.g. a camera with its own recognition model), **split
   it**: the fundamental lands first as a generic, dormant capability (no live item uses it), released and tested on
   its own; the pack follows and only switches it on with data. Both may be developed together on the pack branch,
   but merge the fundamental first.
4. **Patchable** means: merging the pack touches only catalog data, assets, texts and seed statements – no schema
   migration, no edits to other packs. Its merge checklist: catalog-count tests updated (api-smoke / admin-smoke),
   validation and content tests green, icons/models uploaded to the CDN, a browser check for its skill if it adds
   behaviour, ITEMS.md section for the pack, cutover note.
5. Unfinished packs stay unmerged. Nothing half-built ships in a release behind a flag.
6. Owner decisions a pack needs (how an item is obtained, limits, giftable, texts) are listed at the top of its
   ITEMS.md section and answered before the merge.

**First pack:** `items/onion-camera` (Top Toy's onion camera; agents/research/auto-flute-and-onion-camera.md) – not
in the 2026-10-03/04 release.

## Players, albums, related items (2026-10-02, owner — built 2026-10-02 on branch `platform`)

Owner: "You cannot play audio [just by owning the record]. You need a record player first … Also tape player and CD
player … perhaps a refactor of how the audio media is stored in relation to the item and how the item is related to the
season. We need a similar relation for chapters … Related items are nice to upsell physical products on the season
landing page [the season section of the toon page]."

**Data model** (`@scutoons/content` `albums.ts`, types in `items.ts` / `content.ts`, schemas regenerated):
- **Album** = the music, a language-neutral media entity in the **item catalog** (`server/config/items.json` →
  `catalog.albums`, served by `GET /api/items/`): `{ id, toon, title: Localized, tracks: [{ id, title, src, duration? }],
  download?: <zip>, cover?, access?: { download, listen } }`, paths from the CDN root like item icons.
  *Why the catalog and not the content store:* the album only exists through the items that carry it (records, tapes,
  CDs – owner-edited config like products / editions, not creator content); the reader needs album + carriers + players
  in one answer for the playback rule (the viewer already loads the catalog); no new entity kind, import, library endpoint
  or editor. When items / products move to DB tables (PLATFORM.md "Data") albums move with them.
- **Media item** (a sound carrier): `album: <album id>` + `format: vinyl | tape | cd` (both or neither; validation).
  `awakening_vinyl` and `awakening_test_press` carry `awakening` as `vinyl`; their `play_audio` skill is gone
  (`legacyPlayAudio` still reads an old one: it plays without a player).
- **Player**: `plays: MediaFormat[]` – `record_player` (vinyl), `tape_player` (tape), `cd_player` (cd); category tool,
  not stackable, giftable (default), virtual, toon null. Icons `scutoons/items/{record,tape,cd}_player.png` (placeholders,
  `cdn/tools/make_substance_icons.py record_player tape_player cd_player`).
- **Access per album** (owner 2026-10-02: marketing = open download, retrieval = owners re-download what they bought;
  "download and inline playing are different things"): `access.download` / `access.listen` = `everyone` (any visitor) |
  `members` (reading access: a member or a device unlocked by a code – the rule of the chapters and the download page) |
  `owners` (owns a media item carrying the album). Defaults `download: members` (= the toon page's music download as it
  was), `listen: owners`; a player for the format is always needed to listen. *Per album, not per item:* one zip and one
  track list, whichever carrier (vinyl or test press) someone owns – a per-item flag could give two answers for the same
  file. Awakening: `{ download: members, listen: owners }`. Edited in the admin: Catalog → Albums (agents/CATALOG.md part 3).
- **Related items**: `related_items: string[]` (catalog item ids, ordered) on the **season** and the **chapter**,
  language-neutral. Seasons have no neutral entity (DATA.md "Language-independent media": like `download`, `trailer`,
  `cover`, `supporters`) → one creator field writes the same list to every locale, `validateLocales` warns when the
  locales differ, `validateFile` checks ids against `ValidateContext.items` (catalog ids; unknown / duplicate = warning).
  The library passes both through unchanged (season overview, chapter summaries) – no server change.

**Helpers** (`albums.ts`): `albumOf`, `playersFor`, `carriersOf`, `ownsAlbum`, `albumAccess`, `albumDownload`,
`mayListen`, `albumByDownload` (the season's music zip → its album), `mediaPlayback(catalog, item, { owned, member })`
→ `play` (tracks, the player, the download) | `needs_player` (the catalog's players for the format, the download if
allowed) | null, `playerAlbums(catalog, player, who)` → per album the carrier (owned first, else the one to buy), owned,
listen, download; validation `validateAlbums` (inside `validateCatalog`) + `checkRelatedItems`.

**What the reader sees** (viewer):
- **Record (item detail)** with a record player: "Abspielen" / "Play" → the audio player plays the album's tracks in
  order ("Spielt: Awakening · Instrumental (Brass Morton)", next track on end); "Album herunterladen" when
  `access.download` allows it. Without one: "Zum Abspielen brauchst du einen Plattenspieler." / "You need a player to
  play it: Record player." (the player's catalog name) + "Album herunterladen". No "Benutzen" any more.
- **Record player (item detail)**: "Platten für diesen Spieler" – per album the carrier's icon, the album title, "In
  deinem Inventar" or the carrier's name, then "Abspielen" (may listen) or "Kaufen" (the carrier's shop product link,
  Bandcamp; "Ausverkauft" when sold out; the shop when it has no product).
- **Getting the record player** – repaired, not found (owner 2026-10-02, replaces the direct grant built earlier that
  day): "We have a BROKEN record player which we find in chapter 2 in the first scene. You find a Reparatur-Gutschein –
  a repair coupon issued by the city of Vienna – and this allows you to fix your record player at the end of chapter 2
  (for now; later at the end of season 1). It's a recipe of two things that creates a record player. The broken one is
  consumed." See "Record player repair" below.
- **Toon page** (the season section, owner: "season landing page" = `/<toon>`): "Zur Staffel" / "With this season"
  below the music button (phone/tablet: after the chapter list; desktop: in the season column) – per related item the
  icon, the name, the price + "Kaufen" (physical items with a product, the product's Bandcamp link like the landing
  page's editions) or "Ausverkauft", "Hast du schon" when owned (`toon/related-items.svelte`,
  `data-testid="season-related"`). catpope season 1: the vinyl only (owner).
- **Music download stays** (owner): the toon page's music button + `/<toon>/download` unchanged; the download page now
  follows the album's `access.download` when the zip is an album's download (`members` = the membership blocker as
  before, `everyone` = open, `owners` = "Diesen Download gibt es nur für Besitzer der Platte.").
- **Chapters**: `related_items` is stored and edited (creator chapter Meta); the reader shows nothing yet (candidate: the
  chapter end beside the next-bar).

**Creator**: "Verwandte Items" / "Related items" – the season editor's own section (after Downloads) and the chapter
Meta (under Featuring): chips with the item icon + name (UI language; the id in the tooltip), reorder (chevrons), remove
(x), "Item hinzufügen …" lists the rest of the catalog (`/api/items/`, loaded once), an id not in the catalog is marked
"nicht im Katalog" (`related-items-field.svelte`). Regular weight, Feather icons, tooltips on icon buttons.
**Admin**: Catalog → Items: `format · album` and `plays <format>` badges; an "Albums" table (tracks, carriers, download,
access download / listen).

**Seed / migration**: items.json (25 items, 27 with the repair: + 3 players, the album, the vinyl / test press relation); catpope
`season1.json` de + en `related_items: ["awakening_vinyl"]`; catpope `chapter2.json` de + en: the pickup events on the
last slice. `content-seed.mysql.sql`: only the 4 changed statements (`season:catpope:1:{de,en}`,
`chapter:catpope:2:{de,en}`), old timestamps kept. Local store: `import-content … --toon=catpope --only=season,chapter
--update` (backup `server/db/local.sqlite.bak-2026-10-02-related-items`; the stored versions equalled the seed before).

**Tests**: content `test/albums.test.ts` (catalog: players, album, carriers; a record plays only with a record player –
tape / CD players don't count; access download/listen for members / guests / owners, other flags; the old play_audio;
validation errors; the chapter 2 pickup; related items of the seed, cross-locale, unknown ids, chapters); api-smoke
(25 items; the vinyl's album + format, the record player's plays, `albums`); admin-smoke (25 items). Viewer verified in the
browser (test user): the chapter 2 pickup grants the record player, the player lists Awakening with "Kaufen", with a vinyl
"Abspielen" plays the track, without the player the note + download, the toon page block (390 / 1440), the download
page; creator: the season section and the chapter field (add / remove).

**Open owner decisions**: how the tape / CD player are obtained (`grantable: []`); the repair coupon's place once
chapter 5 exists (see "Record player repair"); whether the test press joins the related items later; the track list of Awakening (one track today:
"Instrumental (Brass Morton)", title a placeholder) and its `duration`s; whether the music button + record cut-out
on the toon page should fold into the vinyl's related entry (kept separate now: the download is free for members, the
related entry sells the vinyl); a viewer display for chapter related items.

### Record player repair (2026-10-02, owner — built 2026-10-02 on branch `platform`)

Owner-approved exception to the packs rule for the 2026-10-03/04 release: it replaces the already-built chapter-2 pickup
of the record player and is data only on existing fundamentals (two catalog items, one recipe, story grants in the seed,
two placeholder icons) – no code change.

**Open owner decisions** (answered with a default, the owner may change them):
- **The coupon is consumed** – CONFIRMED by the owner 2026-10-02 ("coupon is used up"). The broken player is consumed; a used coupon is spent, so the recipe
  consumes both (every recipe input is consumed; keeping the coupon would need a new fundamental). It is "good for
  one repair" in its description.
- **Wording**: de "Ein alter Plattenspieler – kaputt. Mitnehmen?" / en "An old record player – broken. Take it?"
  (first scene); de "Ein Reparatur-Gutschein der Stadt Wien – für dich! Kombiniere ihn mit deinem kaputten
  Plattenspieler, dann wird er repariert." / en "A repair coupon from the city of Vienna – for you! Combine it with your
  broken record player to get it repaired." (end); item names "Kaputter Plattenspieler" / "Broken record player",
  "Reparatur-Gutschein" / "Repair coupon". The coupon is a game item: "from the city of Vienna" in the text, no real
  logo, coat of arms or official claim (icon: a plain ticket with a wrench).
- The coupon's place once chapter 5 exists (owner: "for now at the end of chapter 2; later at the end of season 1") –
  move the events from `s1c2-slice-22` to chapter 5's last slice then.

**Items** (`server/config/items.json`, both toon null like the record player, virtual, not stackable, giftable by
default, `grantable: ["story"]`):
- `broken_record_player` "Kaputter Plattenspieler" / "Broken record player" – category tool, no `plays` (plays
  nothing), icon `scutoons/items/broken_record_player.png` (the record player with a cracked record and the tonearm
  knocked off).
- `repair_coupon` "Reparatur-Gutschein" / "Repair coupon" – category collectible, icon
  `scutoons/items/repair_coupon.png` (a ticket with a wrench).
- `record_player`: `grantable: []` now (only the recipe makes it), description: "… Du bekommst ihn, wenn du einen
  kaputten Plattenspieler mit einem Reparatur-Gutschein reparieren lässt."

**Recipe** `record_player_repair`: `broken_record_player ×1` + `repair_coupon ×1` → `record_player ×1`, both inputs
consumed. Combining in the inventory (item → "Kombinieren" → the other one) as every recipe; a guest crafts locally and at
sign-in the crafted record player is split back into its two story finds (`importGuest`), the account repairs again.

**Story** (slice events, `trigger: { type: intersect, once: true }`, one choice only – intersect observers fire in no
fixed order; the VN item bridge grants with source story and `ref` = the place, once per account; guests keep it
locally and import it at sign-in):
- **First scene of chapter 2** = the chapter's first group, the scene file `chapter2/scene-14.json`
  (`scene:catpope:2:scene-14:{de,en}`), first slice `s1c2-slice-1`: "Ein alter Plattenspieler – kaputt. Mitnehmen?" +
  "Mitnehmen" / "Take it" (`add broken_record_player 1`).
- **End of chapter 2** = the last slice `s1c2-slice-22` (group `scene-20`): the record player pickup is replaced by the
  coupon: "Ein Reparatur-Gutschein der Stadt Wien – für dich! …" + "Mitnehmen" (`add repair_coupon 1`).

**Seed / migration**: `content-seed.mysql.sql`: only the 4 changed statements (`chapter:catpope:2:{de,en}`,
`scene:catpope:2:scene-14:{de,en}`), old timestamps kept. Local store: `import-content ../shared/content/seed
--toon=catpope --only=chapter,scene --update` (backup `server/db/local.sqlite.bak-2026-10-02-record-player-repair`).
CDN: the two icons (`cdn/tools/make_substance_icons.py broken_record_player repair_coupon`, then `cdn/sync.py`).

**Tests**: content `test/albums.test.ts` (the recipe: inputs, output, both story-grantable, not stackable, icons; the
record player not grantable; chapter 2: the broken player on the first scene's first slice, the coupon on the last slice,
no record player pickup, one choice each, validation); api-smoke (27 items; `record_player` story grant → 403
`not_grantable`; grant both inputs → craft `record_player_repair` → record player 1, inputs 0; again → 422); admin-smoke
(27 items).

## Placed items (2026-10-03, owner — branch `slice-items`)

> "Add items to the slice – like we did with samples and text boxes. Scaling is possible." · correction: what a tap does
> is the editor's decision per placed item, not prebaked.

- Data: `Slice.items?: SliceItem[]` (`@scutoons/content`): `{ id, item, rect: { x, y, width, height? }, rotation?,
  action?: 'none' | 'pickup' | 'events', qty?, events?: VNEvent[], once? }`. `id` = `<item>-<n>` (stable, pickups are
  remembered by it); `rect` in % of the slice like text boxes, no `height` = the icon's own ratio; language-neutral
  (geometry is the same in every language – the creator writes every locale and makes a differing one follow the
  source on load, `unifyGeometry`).
- Rendering: `@scutoons/reader` `SliceItems` (inside `Slice`, between the image and the text boxes): the catalog icon,
  rotated around its centre; only `action` ≠ `none` makes it a button. The app resolves the icon (`itemInfo`).
- Tap (`viewer/src/lib/visual-novel/slice-items.ts`): `none` = art only (default); `pickup` = the VN item bridge
  (`grantStoryItem` in variable-actions.ts → `inventory.grant(item, 'story', qty, ref)` with the ref
  `vn:<item>:<page>#<slice image>#<placement id>`, then `add <item> <qty>` on the game variable) – the catalog's rules apply
  (grantable story, max count, the server's once-per-ref); on success the game state gets `picked:<slice image>#<id>`
  (synced with a member's progress) and the item is gone from the slice for that reader; on a refusal it stays.
  `events` = the placement's VN events run like a click trigger's (`EventProcessor`), `once` per page view by default.
- Creator: slice inspector tab "Items": "Place an item …" / "Choose the item …" open the item drawer
  (`ItemPickerSheet`), x / y / width / rotation, "On tap" (nothing · pick up + pieces + respawn · run events with the VN
  event editor, whose onomatopoeia events pick or create the text box they reveal). On the canvas: move, a round rotate
  handle above the box and a round scale handle off the bottom-right corner (width around the centre); the Snap toggle
  (grid 0.5 / 1 / 2.5 / 5 %, remembered per device) snaps moving, resizing, scaling and rotating (15°, Shift switches).
- **Round 2 (2026-10-03, owner):**
  - (replaced in round 3 by "Respawn" below)
  - A refused pickup shows a reader notice (items host, kind `refused`): max reached "Du hast schon genug davon." /
    "You already have enough of these.", not grantable, already taken (the server granted 0 for this ref, e.g. taken on
    another device), rate limited / cooldown, failed (offline). `inventory.grant` now reports `granted` and can stay quiet
    on 0 (`quietWhenNone`).
  - A slight hint on tappable placed items (`action` ≠ none): a slow glow (3.2 s) – static with reduced motion.
- **Respawn (round 3, owner 2026-10-03: "per item when they reappear, if they do … both: item, and an override within
  the range the item allows")** – `@scutoons/content` respawn.ts, server `Domain\Catalog\ItemRespawn`:
  - Interval = `"visit"` (every visit / reload) or `{ minutes | hours | days: n }`.
  - ITEM (catalog, `ItemDef.respawn`): `{ default, min?, max? }` – the default interval and the range a placement may
    choose (no `min` = the default is the shortest); none / null = it never reappears (a one-time find). Set in
    server config/items.json (cucumber 6 h and tuna can 12 h (1 h … 7 d), weed 12 h (1 h … 7 d), testo 1 d (6 h … 7 d), psilocybin and lsd 3 d (1 … 14 d), semaglutide 7 d (1 … 30 d); every other item: never – there is no onion item in the catalog yet) and
    since 2026-10-04 stored in the item's catalog row, edited in admin → Catalog → Items (the item form's "Respawn"; the
    API `/admin/catalog/respawn` stays); `/api/items/` and the admin catalog serve it.
  - PLACEMENT (`SliceItem.respawn`): none = the item's default; `"never"` = once here; an interval = an override within
    the item's range. The creator shows the item's default and offers only what the range allows; validation flags an
    out-of-range or never-reappearing choice (`placementRespawnProblem`, with the catalog in `ValidateContext.itemRespawn`).
  - Enforcement: the grant sends `respawn` (the placement's interval) + `visit` (the visit id for "visit"); the server
    checks it against the item's range (422 `respawn_out_of_range` / `respawn_not_allowed`, 400 `invalid_respawn` /
    `visit_required` / `ref_required`) and appends the period itself: `@m<minutes>:<floor(now / interval)>` from its clock,
    or `@v:<visit id>`. A ref is granted once (`rate_limits` grantref, permanent) → one piece per period; max counts and
    120 grants / hour stay authoritative. A client can't go below the item's minimum; it could choose any interval
    within the range (the server doesn't read the placement), which the range bounds.
  - The reader remembers the period (`picked:<slice image>#<id>` = the period key for timed intervals, in memory for
    visits; 1 = taken for good when it never reappears). A visit = one stay on the page path; a reload starts a new one.
