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

Single source: `C:\Development\qr.scutoons.com\api\items\items.json`, served at `GET /api/items/` (public). The viewer
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

All JSON. Auth = `Authorization: Bearer <session_token>`. CORS: `https://scutoons.com`, `https://www.scutoons.com`,
`http://localhost:5173`, `http://localhost:5174`, `http://localhost:4173`. Errors: `{ "status": "error", "error": "<code>", "message": "…" }`.

| Method | Path | Body / query | Response |
|---|---|---|---|
| POST | `/auth/request/` | `{ email, code?, locale? }` | `{ status: "sent" }` (always, no user enumeration) |
| POST | `/auth/verify/` | `{ email, otp }` or `{ token }` | `{ status:"success", session_token, user:{ id, email, created_at }, claimed?: [code] }` |
| POST | `/auth/logout/` | – | `{ status: "success" }` |
| GET | `/me/` | – | `{ user, memberships:[{code,type,expires_at}], inventory:[Entry], avatar:{ slots:{ <slot>: entryId|null } } }` |
| GET | `/items/` | – | catalog (above) |
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

`C:\Development\qr.scutoons.compi\permissions.json` (the owner edits this file; no code change):
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
