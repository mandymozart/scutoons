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
- `?code=` on any page: logged in → claim; guest → keep old membership validation + offer "add to inventory" (login).
- Inventory UI: grid of item icons (cut-outs), detail sheet (name, description, edition `n / total`, skills as
  actions: Use / Combine / Put on), combine mode (select 2 → recipe preview), avatar view with slots.
- Camera: QR decode with `jsqr` (iOS has no BarcodeDetector); cucumber detection with TF.js MobileNet, lazy-loaded
  only when the cucumber camera is used.
