# MEMORY — facts & findings (scutoons)

Established 2026-09-28 during the chapter-2 investigation. Verify file/line refs before relying on them.

## Repo layout

- `viewer/` — SvelteKit (Svelte 5 runes), adapter-static, paraglide i18n (`messages/{de,en}.json`, base `de`).
  Deployed by `.github/workflows/main.yml` on push to `main` → FTP to `ftp.scutoons.com:/`.
- `viewer/static/api/` — the "fake API", served at `/api` and shipped with the app build (not the CDN):
  `toons.json`, `events.json`, `products.json`, `featured-products.json`,
  `catpope/{de,en}/{toon,season1,chapter1..6,characters,collectables,locations,routes,credits}.json`,
  VN scene files `catpope/{de,en}/chapter2/scene-14..19.json`, `chapter6/scene-200.json`.
- `cdn/` — `root/` (gitignored, lives locally + on FTP), `sync.py` (interactive FTP push, "bidirectional" only
  in name), `download.py` (FTP pull), `server.py` (CORS static server on :8000). Env: `FTP_SERVER`,
  `FTP_USERNAME`, `FTP_PASSWORD`.
- `template/episode1.json` — obsolete legacy format, unused.
- `workflows/generate-shots.md` (untracked) — FLORA script→stills workflow; screenplay source of truth is
  `de/screenplay.txt` in private repo `mandymozart/cat-pope-awakening`.
- `issues/plans`, `issues/todo` — audio, VN, security, profiler plans.
- User does not want Claude artefacts tracked (`.claude` ignored; commits "i dont want claude to be in my code").
  Never add Claude attribution to commits/PRs.

## Chapter config

- Types: `viewer/src/lib/models/content.ts` (`Chapter` 123-137, `Group` 106-115, `Slice` 73-85, `Dialog` 11-18,
  `Onomatopoeia` 20-33), `enums.ts`, `audio.ts`, `visual-novel.ts`.
- Translation = one full chapter file per locale. No keyed i18n for content.
- `slice.image` is relative to `cdn/root/<toonId>/`; `asset()` (`lib/utils/cdn.ts`) → `https://cdn.scutoons.com/`
  in PROD, `http://localhost:8000/` in dev. `CDN_SERVER` in `lib/config.ts:2` has a dead hostname check.
- Group shown = `?label=` or `groups[0]`; VN mode whenever `group.events` exists. `ref.src` groups are fetched
  from `/api/<toon>/<locale>/<src>`.
- All overlay rects (triggers, samples) are **% of the slice box**. Chapter column `max-width: 500px`.
- **`Slice.dialog` (text box on slice) has a type but NO renderer** — never had one in any branch. Only usage:
  chapter 2 group `scene-20`, slice 23 (`rect {x:10,y:10,width:80}`, German text in both locale files).
- `Slice.onomatopoeia` also typed but unrendered; onomatopoeia works only as VN event `showOnomatopeia`.

## Images on CDN (catpope)

- Ch1: `images/3x/de/S1C1 - Slice N.jpg` 94 × 1170×1875 (text baked in, German). `images/webtoon/de/` = 799×1280
  exports for the Webtoon platform, unused by configs.
- Ch2: `images/1x/de/s1c2-slice-1..25.jpg` 390×625 (1x only; 15, 21, 22 unused). No 3x, no en.
- Missing on CDN but referenced: `images/s1c1-thumbnail.jpg`, `images/s1c2-thumbnail.jpg`,
  `images/1x/s1c2-key-visual.jpg`.

## Chapters (season1 sequence)

1 Denial/Leugnen published 2025-10-04 · 2 Anger/Ärger draft (config says 2025-11-01) · 3–5 coming-soon empty ·
6 VN test bed (draft). Ch2 = 6 VN ref groups scene-14..19 (only 14 has events: placeholder cucumber choice;
18/19 duplicates) + inline group scene-20. Homepage `routes/+page.svelte:19-31` next-release block has typo
`2.&Auml;ger`.

## Known bugs spotted (not fixed)

- Ch1 audio samples at `slice.audio.samples` but code reads `slice.audio.audio.samples` → ch1 samples never fire.
- `en/chapter6/scene-200.json` uses old string `jumpTo` and `{cucumbers}` vs variable `cucumber`.
- Scene files use `{"type":"end"}` → "Unknown event type".
- Ch2 `keyvisual` is a string, type expects `{image, video?}`. `Chapter.published` required but unused.
- `Collectable` type lacks `name`/`image` that JSON + components use.

## Membership / inventory (viewer)

- Membership: `lib/contexts/membership.svelte.ts`, localStorage `scutoons_membership`, one code only, validated
  once via `POST https://qr.scutoons.com/api/validate/` (`lib/api/membership.service.ts`), code arrives as
  `?code=` handled in `routes/+layout.svelte:18-31`. Types `membership | promo`.
- Inventory: `lib/components/toon/inventory/inventory.svelte` ("i" key) shows game variables that match a
  collectable id (`game.svelte.ts`, localStorage `scutoons_game_<toonId>`). Only collectable: cucumber.
- No accounts, no server session, no QR scanner, no `/qr` route. "Cucumber scan" does not exist anywhere.

## Merch data (duplicates)

- Viewer `static/api/products.json` = successor of deleted `api/static/merch.json` (commit 6960ae8, same
  entries + price/edition/soldOut). Keys: `hoodie_black|beige|grey` (one Bandcamp URL, edition 4),
  `tshirt_white|black` (one URL, edition 12). Colours modelled as separate products.
- Homepage hardcodes 12 (hoodie) / 25 (tee) pieces and uses the sold-out `hoodie_black` link.
- qr `api/editions/editions.json`: `1_catpope_dago` (37, excl. 3; hoodie grey/black/beige M–XXL, heavy T
  black/white S–XL), `2_catpope_beetles` (20, scorpion amulet), `3_catpope_vinyl` (300).
- Bandcamp (2026-09-28): Test Press 2/2 €75, Test Press 1/2 €100, Scropion(sic) Amulet €50, LE 12" vinyl €27.50,
  Love For Women – Kangaroo Cat Hoodie €59.99, Cat Pope Dago Hoodie €99.90, Cat Pope Dago Heavy T €59.90.
- Images: `cdn/root/catpope/merch/1_catpope_dago_{hoodie|heavy_t}_{color}_web-N.jpg` (55 files).

## qr.scutoons.com (`C:\Development\qr.scutoons.com`, github mandymozart/qr.scutoons.com)

- Plain PHP + PDO, MySQL prod / SQLite local (`db/local.sqlite`), FTP deploy on push to **any** branch.
- Single table `qr`: `code` nanoid(8), `type` (membership/event/xr/promo/page), `email` (= "claimed"),
  `redirect_to`, `expires_at`, `edition_id`, `signature` (ML-DSA-65). No users, no edition number column.
- `GET /?c=` claim page, `POST /` claim with email (no verification), `/api/validate/` returns full row incl.
  email (CORS *), `/api/editions/`, `/api/toons/`. Admin SPA `generate/` (batch: sign → insert → SVG QR →
  certificate PDF → sticker sheet). Local DB: 74 dago codes (edition 37), 40 beetles (20), 300 vinyl.
- Security issues: private key via `/api/keys/keys.php` (Referer check only), email leak, XSS
  `index.php:139`, path traversal `generate/files.php`, static admin token, tracked `.htpasswd`, no rate limit.

## Figma

- No Figma integration exists. `C:\Development\graphic-novel-figma-plugin` = React/Vite plugin boilerplate with
  one feature (mask image into vector panel), no network access, no export. Figma MCP connector exists in the
  registry but is not connected.

## Design system

- Extracted 2026-09-17, artifact https://claude.ai/artifact/9iweNqSed2pAjMhQcEE3bo — full content in DESIGN.md.

## Text box renderer

2026-09-29 — `Slice.dialog` now renders (Track B).
- Code: `viewer/src/lib/components/toon/dialog/{dialogs,dialog}.svelte`, mounted in `toon/slice.svelte` after the
  `<img>` (layer `z-index:1`, triggers stay on top at `z-index:2`). `.slice` has `container-type: inline-size`, so
  `size` → `calc(var(--size) * var(--fit) * 1cqw)`. `Dialog` type extended per DESIGN.md Part 2.
- Fonts (both SIL OFL 1.1, from fontsource via `npm pack` into scratch, NOT in package.json; licences in
  `static/fonts/*-OFL.txt`): **Bangers** (latin + latin-ext) = `font: 'lettering'` — it is the SFX / title font in
  chapter 1 ("TRILLERILLER", "GRRRR") and the baked "2. ÄRGER" in s1c2-slice-24; **IBM Plex Mono italic 400/700**
  added for narration. Chapter-1 caption/narration text itself is IBM Plex Mono (regular = speech boxes, italic =
  narration, ~3.8 cqw, line-height ~1.35, padding ~1em) — so defaults are per variant: bubble/sfx → lettering +
  center, caption/narration → mono + left (DESIGN.md says default 'lettering'; deviated on purpose).
- sfx = white fill + ink `-webkit-text-stroke` with `paint-order: stroke fill`, uppercase by default.
- Auto-fit only runs when `rect.height` is set; steps `--fit` 1 → 0.8, then lets it overflow (no clipping).
- Gotchas: `container-type` also makes `.slice` a stacking context + containing block for `position:fixed`
  descendants (none today) — text overflowing a slice's bottom is painted under the next slice. This is intended:
  chapter 1 split boxes (negative `rect.y`) rely on the later slice painting over the earlier one — keep it.
- Fit pass (fixed 2026-09-29): resets `--fit` to 1 before measuring, skips while not laid out (width/height 0),
  re-runs via ResizeObserver on slice + box + content (content catches the web-font swap; its own --fit resize is
  ignored by comparing sizes) and on `document.fonts` `loadingdone` — `fonts.ready` alone resolves before lazily
  requested fonts start loading. `frame: false` renders caption/narration without the paper box. Chrome on Windows didn't hyphenate German here
  (`overflow-wrap:anywhere` breaks instead). `lang` = `getLocale()` (chapter files are per locale); en chapter2
  still carries the German narration text.
- Dev testing: port 8000 on this machine was taken by another project, so the CDN was run as
  `python server.py 8001` and img srcs swapped in the browser; ch2 is behind the membership blocker (set a
  fake `scutoons_membership` in localStorage for local testing only).

## qr.scutoons.com backend

- 2026-09-29 – Accounts/items backend implemented on local branch `anger` of `C:\Development\qr.scutoons.com`
  (8 commits c300a80..45223b6, **not pushed** – push to any branch used to deploy; now only `main` deploys).
  Contract: `ITEMS.md` + its "Backend implementation notes"; ops/setup: qr `docs/ACCOUNTS.md`.
  - Security: `/api/keys/keys.php` public key only (SPA uses `generate/keys.php` with admin token), validate without
    email/signature (expired → 410), landing XSS fixed, `files.php` filename whitelist, generic errors, `.htpasswd`
    untracked + excluded from FTP sync, `NOW()` replaced by PHP time.
  - DB: migration `db/migrations/2026-09-29-accounts-inventory.{mysql,sqlite}.sql` (users, login_requests, sessions,
    code_items, inventory, avatar_slots, progress, rate_limits; `qr.edition_number`, `qr.claimed_by`).
    Production: run the .mysql.sql in phpMyAdmin + `scripts/backfill-edition-numbers.php --sql` (not done yet).
  - Local DB migrated (backup `db/local.sqlite.bak-2026-09-29`), edition numbers backfilled for 357 manifest codes.
    Duplicates: `scripts/report-orphan-codes.php` → 37 dago codes (06-16 12:10, signed #1–37) and 20 beetles codes
    (06-16 11:02, signed #1–20) are in no manifest; the dago manifest batch is numbered 11–47 (> edition of 37).
    Nothing deleted – owner decides which set was printed.
  - Local dev: `php -S localhost:9000 router.php`, mails in `logs/mail.log`, icons from `http://localhost:8001/`,
    `php tests/api-smoke.php` = 56/56 checks. Test users/claims now exist in the local DB (smoke-*, browser-test@,
    magic-link@example.com).

## Catalog & icons

2026-09-29 (item icons + shop dedupe):
- Item icons: `cdn/root/catpope/items/<item_id>[_<variant>].png` (512², transparent, trimmed, ~6% pad, sRGB),
  17 files, generated by `cdn/tools/make_item_icons.py` (sources hardcoded in `PHOTO_ICONS`; venv `cdn/.venv`,
  deps `cdn/tools/requirements.txt`: rembg isnet-general-use + u2net_cloth_seg for worn garments, birefnet-general
  for objects, resvg-py for SVG). Also writes shop-size copies `merch/bandcamp/web/*.jpg`. Not uploaded (sync.py).
- No flat lays of the Dago merch exist, all photos are worn by a masked model. Hoodie icons use the BACK views
  (front views leave a face hole in the hood); tees use front views (cloth mask eats the white back print).
  Base icons: dago_hoodie = beige, dago_heavy_t = white, awakening_test_press = 1_of_2, scorpion_amulet = purple.
- `images/collectables/cucumber.png` is only 124px → cucumber icon is vector (same style as camera/cucumber_camera).
- Bandcamp originals in `cdn/root/catpope/merch/bandcamp/` (f4.bcbits.com `_0` = original size).
- Bandcamp facts (checked 2026-09-29): Dago Hoodie "edition of 12", 3 remaining, €99.90; Heavy T "edition of 25",
  €59.90 (12 + 25 = 37 = qr edition `1_catpope_dago`); Kangaroo Cat Hoodie edition of 4, 3 remaining, €59.99;
  Scorpion Amulet €50 (no edition shown; qr says 20); LE 12" vinyl edition of 300, €27.50; Test Press 1/2 €100,
  2/2 €75 (edition of 1 each). No per-colour/size stock on Bandcamp → colour soldOut flags are manual.
- `viewer/static/api/products.json` v2 = `{version, currency, checked_at, source, products:[{id(=item_id), item_id,
  edition_id, edition_size, name, link, price, limited, remaining, sizes, soldOutSizes, credits, icon,
  defaultVariant, variants:[{id, name{de,en}, soldOut, icon, images, price?}]}]}`; `featured-products.json` =
  `[{product, variant}]`. Types + helpers: `viewer/src/lib/components/shop/products/shop.ts`.
  → since platform P2b (2026-09-30): files in `server/config/`, served by `GET /api/products/` +
  `/api/products/featured/`; types + helpers in `@scutoons/content` (`products.ts`).

## Chapter 1 text extraction

- 2026-09-29 — Ch1 "Leugnen" baked text extracted → 70 live dialogs on 53 slices (`i18n/s1c1.json` ids
  `s1c1-<hostSlice>-<n>`, `dialog` in `catpope/{de,en}/chapter1.json`). Tools: `cdn/tools/extract_bubbles.py`
  (detector: dark glyph holes in bright regions, no OCR) + `cdn/tools/s1c1_dialogs.py` (hand transcription +
  translation table → measures box border/size, writes i18n + dialog keys; idempotent, re-run after edits).
  All ch1 lettering is IBM Plex Mono in white boxes with a ~6px@3x black border; upright = `caption`,
  italic = `narration`; rect = the box's OUTER border so the renderer's paper box redraws the baked box.
  Lettering sizes: slices 1–24 ≈3.85% (60px pitch @3x), 25+ ≈4.05% (63px). Boxes crossing a slice seam
  (4/5, 5/6, 11/12, 12/13, 14/15, 74/75, 86/87) live in the LATER slice with negative `rect.y` (later slices
  paint over earlier ones). Text that can't be a redrawn box (s30 GRRRR overlap, s43 panel, s62 dashed
  border, s92 music notes, s94 end card) = `bubble` + `font: mono` + text-only mask (loses italic on s43).
  Left baked on purpose: English-only boxes (s17 "I hate Mondays...", s26/s45/s90 song lines, s37 "... okay!"),
  the s1 title "1. KAPITEL / LEUGNEN", all SFX (Bangers art), s8 ad lettering, elevator numbers, book covers.
  en config: short single-line boxes are widened where English is longer (`rect` differs from de).

## Viewer items & accounts

2026-09-29 (D-viewer):
- Config: `lib/config.ts` → `QR_API` (`VITE_QR_API`, fallback `https://qr.scutoons.com/api`, no trailing slash),
  `CDN_SERVER` (`VITE_CDN_URL`, default prod `https://cdn.scutoons.com`, dev `http://localhost:8001`), cucumber
  threshold `VITE_SCAN_OBJECT_THRESHOLD` (0.35), 3 consecutive frames at 2 fps. Dev values live in
  `viewer/.env.development` (gitignored; a plain `.env` would be baked into prod builds). `.env.example` documents them.
- API client `lib/api/client.ts` (Bearer from localStorage `scutoons_session`, throws `ApiError{status, code, body}`)
  + typed endpoints `lib/api/qr.service.ts`. Models `lib/models/items.ts`.
- Contexts, all created in `routes/+layout.svelte`: `session.svelte.ts` (login, `/me/`, events login/logout/me,
  `getCurrentSession()`), `inventory.svelte.ts` (catalog, entries, avatar; guest = localStorage `scutoons_inventory`
  with starter items handed out once; on login guest items are imported — crafted items are split back into their
  recipe inputs since only grantable items import; `getCurrentInventory()`), `items-ui.svelte.ts` (camera / claim /
  minigame / audio overlays rendered by `components/items/items-host.svelte`).
- Access (final model 2026-09-30): `membership.isValid` = device access OR account access.
  Device access = `scutoons_access` {code, permissions, expires_at, checked_at, access_token} from `POST /access/`
  (any holder of a code whose type grants `catalog`, also if someone else owns it; survives logout; re-checked via
  `/access/` when older than 6 h → `410 revoked|expired` drops it with a one-time banner). Account access = `/me/
  access.permissions` includes `catalog`. The viewer hardcodes no code types (server permissions map). Legacy
  `scutoons_membership` → silently `/access/` → `scutoons_access`, key removed.
- Claim sheet (`?code=`, in-app `scan_qr`): grants device access right away, previews via `/code/`, and *offers*
  "Add to my account" (login inline → `/inventory/claim/`; 409 → "already in someone's account, you can still read";
  `email_required` → "sign in with the email you used"; `choose_item` → picker). `/account?code=` (QR landing "Add to
  my account") = same sheet with `auto` → sign in, then claim. Codes that neither unlock nor are claimable → sheet
  closes. Blocker: "Scan your code", "Sign in", "Buy access". Every QR_API request carries `X-Device-Id`
  (random `crypto.randomUUID()` in localStorage `scutoons_device`; server stores only an HMAC, for stats).
- Dropped (never shipped): the 2026-09-29 "login required" gate and the anonymous-account / `/auth/code/` draft.
- Game progress sync lives in `contexts/game.svelte.ts`: pull once per login (server newer → replace local; a device
  that never saved takes the server state), debounced (2s) PUT on every save. API `updated_at` = **ms since 1970**;
  PUT answers `status:"stale"` + newer state → applied locally.
- Magic link from the mail is `<viewer origin>/?login=<token>` (backend decides the origin); root layout verifies it.
  `?code=`: logged in → claim sheet auto-claims when there is nothing to choose; guest → claim sheet with inline login (`/auth/request/` with `code` → backend claims at verify, `claimed:[code]`).
  `&inventory=open|1` opens the toon inventory.
- VN `set`/`add` on a variable that is a catalog item id also grants it (source story) in
  `visual-novel/variable-actions.ts`; the game variable stays for text interpolation.
- Camera (`components/camera/*`): jsQR on video frames (~8 fps, white-filled canvas so transparent stills decode);
  TF.js + MobileNet v2 lazy-loaded (separate ~700 kB chunk), classified on the **centre square** of the frame
  (full 16:9 frames squashed to 224² dropped a test cucumber from 74% to 16%; centre crop → 91%). Dev harness:
  `/account/lab` (DEV only) runs the same decoder/classifier on an image URL or file.
- Browser-pane testing trick: stub `navigator.mediaDevices.getUserMedia = async () => canvas.captureStream(10)` and
  draw a photo / QR SVG (qr repo `generate/app/lib/qrcode/qrcode.min.js`) into the canvas → exercises the real loops.
  The pane must be visible (hidden tabs don't composite). Two dev servers (5173 + 5174) share `viewer/.svelte-kit` and
  the paraglide outdir → occasional full reloads / a stale `proxy+layout.ts` ENOENT (fix: `npx svelte-kit sync`, restart).
- Open: `/api/editions/` (old endpoint) sends no CORS headers → inventory shows `No. n` instead of `n / total`
  (claim preview has the total from `/code/`). Login mail rate limit: 5 per email per hour.

- 2026-09-30 – Access model changes (qr `anger`, commits c1240ae..275b154, not pushed):
  - Anonymous-account draft (`/auth/code/`, nullable `users.email`, merge) was built, then superseded by the final
    model and reverted with `git revert` (3d7d5e8, aa1454c). Local DB: the one anonymous test user removed and the
    users table rebuilt NOT NULL (backups `db/local.sqlite.bak-2026-09-30*`).
  - Final model: viewing is device-level via `GET/POST /api/access/` (any unexpired, unrevoked code whose type grants
    `catalog`, also when owned by someone else); owning needs email login, one owner per code (triggers kept).
    `api/permissions.json` (server-only, denied over HTTP, no endpoint) drives claimable/grants; `/me/ access`;
    signed `access_token` (HMAC, `Permissions::verifyToken()`, not enforced yet). `/validate/` backwards compatible
    again. Landing page: preview + Continue + "Add to my account" (`<viewer>/account?code=`), no email step.
  - Revoke + statistics: `qr.revoked_at/revoked_reason` (410 revoked everywhere), `code_events` (scan, access per
    device/day, claim, claim_refused, login, revoke, unrevoke; hashed device id + daily-salted IP hash), admin
    "Stats" view in the generate SPA (table, chart, CSV, timeline, revoke). Prod: run
    `2026-09-30-revoke-events.mysql.sql` after the owner-guard migration.
  - Tests: `tests/api-smoke.php` 108/108, `tests/claim-race.php` PASS.

- 2026-09-30 — Dialog text lives only inline in the per-locale chapter configs (`catpope/{de,en}/chapterN.json`,
  `slice.dialog[].text`), same structure as every other field. Removed the redundant `i18n/s1c1.json` and the one-off
  generator `cdn/tools/s1c1_dialogs.py` (git history has them). `cdn/tools/extract_bubbles.py` stays as a detector.
  Editing goes through the creator editor (`/creator`).


- 2026-09-30 — Roles, signup rule, promo codes (qr `anger`, commits aa0096e..aa10f53, not pushed; viewer uncommitted):
  - qr: migration `2026-09-30-roles-signup.{mysql,sqlite}.sql` (`roles` seeded member/admin/creator with fixed ids
    1/2/3, `user_roles` + member backfill, `users.marketing_opt_in/_at/signup_code`, `login_requests.marketing_opt_in`).
    Prod: run it in phpMyAdmin after the revoke-events one. `api/lib/Roles.php`: `has`, `of`, `requireRole` (401/403
    `forbidden`), `grant`, `revoke` – the creator content API (Content.php) uses `requireRole(Roles::CREATOR)`.
  - Signup rule: `permissions.json` types carry `signup`; promo is `claimable:false, signup:true`. New account only via
    `Codes::signupRefusal()` = null (exists, unexpired, unrevoked, signup type, claimable ones still free). No account +
    no usable code → `{status:"sent"}` + explanation mail without OTP/link (`LoginMail::composeNoAccount`, de/en);
    verify re-checks → `403 signup_code_required`. Promo is never claimed (`claimed` absent in verify, claim → 422).
    `/access/` + `/code/` return `signup` (and `/code/` `claimable`). `PUT /me/ { marketing_opt_in }`; ticking the
    opt-in at sign-in also opts an existing account in (never out). Event `signup`.
  - Found + fixed: "newest login request of an address" was random within one second (UUIDv4 tiebreak) → login
    requests now use `Db::uuid7()` (time-ordered).
  - Admin SPA "Users" (users.php, user-roles.php, views/users.js); CLI `php scripts/roles.php grant|revoke|list`.
    Landing page promo: "Promo code – no item to collect" + Continue + Create account (`/account?code=`).
  - Viewer: claim sheet promo branch (Continue reading · Create account → LoginForm `signup` with unchecked opt-in
    checkbox), membership login form also shows the opt-in; account page: role tags, opt-in toggle, "Creator" link
    for `creator`; guests see the signup rule; `signup_code_required` message. Session: `roles`, `hasRole`,
    `setMarketingOptIn`; `requestLogin`/`verifyOtp` take the opt-in.
  - Tests: api-smoke 171/171, claim-race PASS. Local test accounts left: browser-promo-0929a@, browser-member-0929@
    (claimed vinyl vyJFcccV), browser-nocode-0929@ (no account). DB backup `db/local.sqlite.bak-2026-09-30-roles`.
  - `npm run check` regenerates `.svelte-kit/types` and can break the running dev server (ENOENT
    route_meta_data.json) → `npx svelte-kit sync` fixes it without a restart.
- 2026-09-30 — Follow-up (qr 5a87d11..616a32d): signup with a product code is atomic (`Auth::confirm` creates the
  user and calls `Codes::claimInTransaction` in one transaction; failure → 409 already_claimed / 422 choose_item,
  no account, login request stays unused so the same OTP can be confirmed again with item_id/variant; the viewer now
  sends the picked item/variant with `/auth/request/`). `tests/claim-race.php [n] [claim|signup|both]`: 8 concurrent
  signups with one vinyl → 1 account owning it, 7× 409. Promo accounts: `/me access` includes the valid
  `signup_code` grants (not owned; `access.signup_code` set) → account page "All chapters · through your promo code X".
  `/de` showing English is not a bug in these files: paraglide strategy is cookie → globalVariable → baseLocale
  (no `url` strategy), so the locale cookie wins over the path. The `/access/`+`/code/` per-IP limit (120/h) trips
  after ~4 smoke runs locally → `DELETE FROM rate_limits WHERE bucket LIKE 'lookup:%'`.

## Creator editor

2026-09-30 — `scutoons.com/creator` (phase 1). **Source of truth for chapter / season / toon configs and VN scene files
is now the content store** in qr.scutoons.com (`content_versions`); the static files in `viewer/static/api/<toon>/<locale>/`
are the fallback and the seed.
- Store (qr `api/lib/Content.php`, docs `docs/ACCOUNTS.md` → "Content API"): one row per saved version of a file
  (toon, locale, path); at most one `published` row per file (live), all others `draft` (newest = working copy).
  Public `GET {QR_API}/content/<toon>/<locale>/<path>.json` = published (ETag, 404 when none). Drafts, `PUT`,
  `versions`, `index`, `publish`, `unpublish`, `restore` need role `creator` (`Roles::requireRole`). Path whitelist
  `^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$`, 2 MB limit, JSON decoded as objects (keeps `{}`). `restore` copies an old
  version as the newest draft (publish it to go live). Seed: `php scripts/import-content.php <viewer/static/api>`
  (published v1, idempotent; `--update` adds new published versions for changed static files; `--db=<sqlite>` for copies;
  `--sql` prints guarded MySQL INSERTs for phpMyAdmin = the production seed, not done yet).
- Viewer loader (`lib/api/content.service.ts` `fetchContent`): `(toons)/[toonId]/+layout.ts` (toon.json, season1.json,
  chapterN.json), `chapter/[chapterNumber]/+page.ts` (chapter + `ref.src` scene files), `download/+page.ts` (toon.json)
  read the store first (4 s timeout) and fall back to the static `/api/...` file on 404/error. characters, locations,
  routes, collectables, credits stay static. `?preview=draft` on a reader URL loads the newest drafts with the session
  token (creators only; others fall back to published → static). jumpTo/navigation inside the reader drops `?preview`.
- **Local dev: the local store is seeded** (2026-09-30), so the dev reader shows store content: editing a static file
  no longer shows up until `php scripts/import-content.php C:\Development\scutoons\viewer\static\api --update` (or edit
  in `/creator`). Chapter 1 was edited/published/restored in the browser test; its published version equals the static
  file again (v49 de / v50 en = restores of v17 / v32).
- Editor code: route `routes/(creator)/creator/**` (gate: session + `user.roles` includes `creator`, else sign-in form
  / "No access"), components `lib/components/creator/*`, logic `lib/creator/*` (`doc.svelte.ts` ContentDoc,
  `validate.ts`, `paths.ts`, `defaults.ts`, `images.ts`). Chapter editor = `chapter-editor.svelte`.
- Model: a `ContentDoc` = one file in all locales (`data[locale]`). Shared fields (geometry, style, image, audio, VN
  structure) are written with `setShared`: to the source locale `de` and to every locale whose value still equals the
  old de value – differing locales keep their value and show as **override** ("Use shared value" resets). Text
  (dialog text, VN text/labels, title, description) is per locale (`setLocal`). Structural edits (add/remove/reorder
  slices, boxes, events, groups) apply to every locale file by index, so the locale files must stay aligned (the
  problem list reports mismatches). Canvas drags/nudges move every locale by the same delta (keeps e.g. the wider en
  rect of chapter 1) or only the shown locale (geometry scope "only en").
- Canvas: real `Dialogs` renderer (`lang` prop) in a `container-type: inline-size` box of 390/1170 px (1x/3x) with an
  overlay for dialog / sample / event rects (move, 8 handles, draw on empty space, snap to 0.5–5 % grid, arrows nudge,
  Shift ×10, Delete). The reader's dialog/onomatopoeia components are used unchanged (no editable hooks were needed).
- Undo/redo per file (snapshots, merged typing/drag steps, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y), Ctrl+S saves, autosave of
  unsaved files to localStorage `scutoons_creator:<toon>:<path>` (restore banner after a reload), `beforeNavigate`
  guard. Save = `PUT` per changed locale with `base` = loaded version (409 → reload / overwrite). Publish = save, then
  publish the newest version of every stored file of the chapter (incl. scene files).
- Reader language is a cookie (paraglide strategy cookie → globalVariable → baseLocale), not the URL: the editor's
  "Preview de/en" links set `PARAGLIDE_LOCALE` before opening `/<toon>/chapter/N?preview=draft[&label=]`.
- Validation (`validate.ts`) mirrors the TS types: required chapter fields, enums, rects, dialog props, audio track /
  sample ids vs the chapter library, trigger rects, VN event params, characters from `characters.json`, keyvisual
  string form, `audio.samples` misplacement (one-click "Move them"), cross-locale structure, missing translations,
  en > 130 % of de.
- Browser tests with a second account: `http://[::1]:5173` is the same dev server with its own localStorage; it is
  in `ALLOWED_ORIGINS` of the local `config.local.php` only. Test creator `creator-test@example.com` (local DB).
- **2026-09-29 — source language per toon** (owner: not per chapter, not global). `ToonIndex` got `sourceLanguage` +
  `languages` (source first; optional – `toonLanguages()` in `@scutoons/content` defaults to `languages` → `i18n` keys
  → `de`); `viewer/static/api/toons.json`: all three toons `de` + `["de","en"]`. The creator keeps the entry in the
  content store as the language-neutral file `<toon>/_/index.json` (server `LOCALE_PATTERN` now `^([a-z]{2}|_)$`,
  `ContentService::NEUTRAL_LOCALE`); before the first save it falls back to the static `toons.json` entry. The viewer
  still reads only the static `toons.json` (unchanged). Toon page "Languages": source select, add/remove translations,
  saved/published via the bottom ActionBar together with season1.json/toon.json; after saving a new language a bar
  offers "Create <l> from <source>" (createLocale + save as draft for toon.json, season, chapters and their scene
  files). `LOCALES`/`SOURCE_LOCALE` are gone: `ContentDoc(toon, path, locales, primary)` always gets the toon's
  languages (`creator/src/lib/editor/languages.ts`: `loadToonLanguages`, `toonDoc`, `toonIndexDoc`); the chapter route
  awaits the languages and keys the editor on them; the preview accepts only the toon's languages. Browser test: source
  → `en` (draft v73): en first + "source", de "missing" on the scene-200 VN texts, length warnings "de … % of en";
  adding `es` showed "es: 8 files … Create es from en" (not clicked); restored to `de` + `["de","en"]` (v75 = the
  static entry), **nothing published** – local store has 3 draft versions of `catpope/_/index.json`, no published one.

- **2026-09-29 — chapter visibility = the season status, per locale** (single source of truth; a toon may release de
  before en). **Rule (owner): a chapter always belongs to a season** (PLAN.md "RULE: every chapter belongs to a season") – chapter files without a season entry are invalid.
  - Viewer: the toon layout lists `isItemListed` items of the season (published; drafts in dev; coming-soon as
    non-clickable "Coming Soon" entries with 0.5 opacity) and sets `LoadedSequenceItem.status` from the season (the
    chapter file's `status` is dropped). prev/next skip coming-soon. `chapter/[n]/+page.ts` gates via `parent()` season:
    coming-soon → "Coming soon" + chapter title + release date (also in dev); draft or not in the season → the existing
    `chapter.not_public` text in production (dev shows drafts); the URL keeps resolving, the chapter content is not
    fetched. Before this, drafts/coming-soon chapters were readable by direct URL in production.
  - `@scutoons/content`: `Chapter.published` removed, `Chapter.status` optional + `@deprecated`; helpers
    `isItemListed`, `sequenceStatus`; `validateFile(ctx.seasonStatus)`: chapter status ≠ season → warning, no season
    entry → error, a `published` key → info. Schemas regenerated.
  - Creator: meta panel "Status" = one select per locale editing `season1.json` → saved/published with the chapter via
    the ActionBar (season doc is part of the editor's docs). On save the chapter file's `status` mirrors the season and
    `published` is removed (`lib/editor/visibility.ts` `syncChapter`; "Align now" in the meta panel). "published (flag)"
    field and the "also set the status in the season" hint are gone. Toon page: status per locale per chapter, saving
    also saves the mirrored chapter files (published with the season). Toon without a season: "Create season 1"
    (season1.json + toon.json if missing, drafts; adopts existing chapter files as draft entries); "New chapter" always
    creates the chapter inside the season (numbers after every stored chapter file, errors shown in the ActionBar).
  - Data: static catpope chapter files have no `published`, statuses match the season (de + en). Store: only
    `toptoy/{de,en}/chapter1.json` had `published: false` → new drafts without it (v81/v82). Test edits on catpope
    chapter 2 en (status → coming-soon → back) left drafts v77–v80, the newest equal to the published versions.
    toptoy (was: no season, no toon.json, chapter 1 orphan) now has draft season1.json (chapters 1 + 2, draft),
    toon.json and chapter2.json – nothing published.
  - Caveat: the released viewer still reads the chapter file's `status` for clickability, so a season status change
    needs the mirrored chapter file published too (both publish flows do that).

- **2026-09-29 — season + toon meta editors.** `/<toon>/season/<n>` (`season-editor.svelte`) and `/<toon>/toon`
  (`toon-editor.svelte`) on a shared shell `meta-page.svelte` (ActionBar, problems, history, autosave/recover, missing
  locales, leave guard). Language-neutral fields use `shared-field.svelte` (one editor → `setShared`; when locales differ
  or "per language" is chosen, one editor per locale with source/override pills and "same for all"), names use
  `list-field.svelte`. Validation (`@scutoons/content` `validateFile` season/toon + `validateLocales` missing
  translations, sequence mismatch between locales). Real catpope data: release, covers, credits.music/sound differ
  de ↔ en (shown as overrides); the en covers `images/1x/en/s1-cover-{square,portrait}.jpg` are **not on the CDN**.
  - What the viewer renders: season → title, description (toon page) + sequence status; toon.json → title,
    description, cover.square (toon page), map (map overlay), download.music/complete + title (download page).
    **Ignored by the viewer:** season authors, number, release, language, cover, credits; toon id, authors, chapters,
    language, release, thumbnail, cover.portrait, status (the list uses toons.json). The credits route reads the static
    `<toon>/de/credits.json` (FullCredits, always German, not in the store) – season `credits` are not shown anywhere.
  - Browser test: catpope season description de/en + a credits.artist name saved as drafts v93/v94, restored in
    v95/v96 (= published v30/v45), nothing published. yellowmouse "Create season 1" → drafts season1.json (v97 de,
    v98 en) + toon.json (v99 de, v100 en), unpublished.

## Platform P1a

2026-09-30 (branch `platform`, commits 52ba2d7..see git log; details + package map: PLATFORM.md "P1a — result").
- Monorepo: root `package.json` workspaces `viewer`, `creator`, `admin`, `shared/*`; one lockfile + `.npmrc` at the
  root; `node_modules` only at the root (viewer/node_modules removed). Scripts: `dev:viewer` (:8000), `dev:creator`
  (:10000), `dev:admin`, `check`, `build`. Versions unchanged (svelte 5.55.0, kit 2.55.0, vite 8.0.3, paraglide 2.15.1).
- Shared packages are source packages (`exports` → `src/index.ts`, `svelte` condition); every app's vite config needs
  `server.fs.allow: ['../shared']` (otherwise fonts answer 403). After adding a new `@scutoons/*` dependency, restart
  the dev server (vite caches the failed resolution).
- The viewer's code moved: models → `@scutoons/content` (import from `'@scutoons/content'`), `lib/api/*` →
  `@scutoons/api` + app wiring `viewer/src/lib/api.ts`, dialog/onomatopoeia components → `@scutoons/reader`,
  fonts + `fonts.css` → `@scutoons/tokens`. `viewer/src/lib/components/toon/slice.svelte` is now a thin wrapper.
- The creator lives in `creator/` (standalone, own login on its origin, role gate, magic link `<creator>/?login=`,
  draft preview `/<toon>/chapter/<n>/preview?locale=de|en&label=`). The viewer has no creator code and no
  `?preview=draft` mode any more; its account page links to `VITE_CREATOR_URL` for the creator role.
- Env: `VITE_API_URL` (dev `http://localhost:9000/api`; old `VITE_QR_API` still read), `VITE_CDN_URL` (dev
  `http://localhost:8001`), `VITE_CREATOR_URL` (viewer), `VITE_VIEWER_URL` / `VITE_STATIC_API` (creator).
- `npm run check` compiles paraglide first (`<app>/scripts/paraglide.js` + `<app>/paraglide.config.js`, shared with the
  vite plugin), so it no longer depends on a dev server having generated `src/lib/paraglide`.
- Visual regression method (worked well): a probe that records tag + rounded box + 20 computed styles of every element
  into localStorage (per origin!) before the change, then compares after, in an 800px iframe. Gotchas: localStorage is
  per port, so baselines only compare on the same origin; a hidden browser tab never runs `requestAnimationFrame`, so
  the dialog auto-fit stays at 0.8 there (not a bug of the page; bring the tab to the front); lazy images below the
  fold and the home carousels load late (image heights 0 vs 294).
- Verified identical against the pre-P1a baseline: home (en), shop (de/en), events, /catpope, credits, download,
  /en/catpope, account, chapter 1 de + en (94 slices, 70 text boxes), chapter 1 with the inventory open.
- Local sessions: tilman@porschuetz.de (creator) is signed in on :8000 and :10000 in the browser pane.

## Platform P1b

2026-09-30 — server refactor to Slim 4 (branch `platform`, renamed to `server/` on 2026-09-30
– `git mv` was refused by the permission guard). Details: agents/PLATFORM.md "P1b — result", server docs
`server/README.md` + `docs/*.md`, admin API contract ITEMS.md "Admin API".
- Layout: `public/` web root (front controller, `router.php` for php -S, verify.html, assets, generate/ SPA files),
  `src/{Http,Domain,Infrastructure,Console}`, `config/` (settings/container/routes + items/editions/toons/permissions
  JSON), `templates/`, `migrations/{sqlite,mysql,schema}`, `bin/console`, `storage/` (private), `tools/keys/`.
- Local: `composer serve` (:9000; `serve:9100` next to the old repo), `composer test` (PHPUnit 45 tests + contract
  suites on php -S :9180/:9181 with a DB copy), `composer analyse` (PHPStan level 8). `config.local.php` rewritten:
  `DB_SQLITE_PATH` honours `QR_DB_SQLITE_PATH` (env) → servers/tests can use a copy; APP_URL :8000, CDN :8001,
  origins incl. 8000/10000/11000. Backup of the old header: scratchpad only (contains secrets – not kept).
- Local data moved: `generate/files/` → `storage/files/`, `api/keys/keypair.json` → `storage/keys/keypair.json`.
  `db/local.sqlite` untouched by the test runs (all on copies); tilman@porschuetz.de got `admin` via
  `bin/console roles grant`.
- Contract results (on DB copies): api-smoke 182/182, content-smoke 62/62, claim-race claim+signup PASS, admin-smoke
  60/60, public-urls 48/48 identical to the old server on :9000 (+ 21 host-routing checks, also with the transition flag
  off). A byte/header diff of ~80 read-only requests against the old code on a scratch copy (:9199) was identical
  except: private files (`/api/keys/keypair.json`, `/api/permissions.json`) now 404 as JSON `not_found` (before: plain
  "Not found" 404 from the dev router / 403 on Apache).
- Gotchas: symfony/console prints CRLF on Windows (diff with --strip-trailing-cr); Slim cannot bind static closures
  as route handlers (use invokable classes); `php -S` + `$app->run()` works with `public/router.php` setting
  SCRIPT_NAME; on Windows close the PDO before deleting SQLite files in tests (`Database::close()`); PHPStan needs
  `rememberPossiblyImpureFunctionValues: false` for DB-reading services.
- Production (not done): cutover steps in `docs/DEPLOY.md` (keys to storage/keys first, deploy, rename generate/files →
  storage/files, docroot → public/, generate/ Basic auth path, api.scutoons.com subdomain); no DB change in P1b.

## Platform P2b

2026-09-30 — shop split out of the viewer (details: agents/PLATFORM.md "P2b — result").
- `shop/` = standalone SvelteKit SPA on :12000 (`npm run dev:shop`), deploy `.github/workflows/shop.yml`
  (`SHOP_FTP_*`). Env `VITE_API_URL`, `VITE_CDN_URL`, `VITE_WWW_URL`; the viewer has `VITE_SHOP_URL`.
- Products: server `config/products.json` + `featured-products.json`, `GET /api/products/` and
  `/api/products/featured/` = the files byte-for-byte (CORS `*`, 5 min cache). `viewer/static/api/products*.json`
  are gone. To change products: edit the server's config files (deploy the server), not the viewer.
- Public data → a header-free API client (`createApiClient({ baseUrl })`): the viewer's normal client sends
  `X-Device-Id`, which forces a CORS preflight that only ALLOWED_ORIGINS pass.
- Paraglide in the shop: strategy url → cookie → preferredLanguage, both locales prefixed (`urlPatterns` in
  `shop/paraglide.config.js`); `/` without prefix redirects to `/<locale>`. Locally the `PARAGLIDE_LOCALE` cookie is
  shared by every localhost port (cookies ignore ports): switching the shop to en switches the viewer too.
- Viewer `/shop`, `/de/shop`, `/en/shop`, `/shop/*` → `routes/(shop)/shop/[...rest]/+page.ts` (client redirect,
  307 via kit `redirect`) to `<VITE_SHOP_URL>/<locale>/`.
- Verification method: the same box + computed-style probe as P1a, run on a second viewer instance (:8010, env
  overrides via process env: `VITE_API_URL=…9100…`, `VITE_CDN_URL=…8011`) before the move and on the shop after;
  identical at 1280 and 375px.
- Local dev trap: `C:\Development\qr.scutoons.com` still serves :9000 and has no `/api/products/`; until the
  platform server owns :9000 the viewer home on :8000 shows no product slides and the shop needs
  `VITE_API_URL=http://localhost:9100/api` (`composer serve:9100`). The python CDN on :8001 is single-threaded and
  hung during this session (a second one on :8011 was used).
- Mistake: `php tests/api-smoke.php http://localhost:9100` was run once against the server using the real
  `server/db/local.sqlite` (not a copy) – it claimed one unclaimed vinyl code and one Dago code for fresh
  `smoke` accounts there (vinyl `v-Hr9Pn_`, Dago `rVLQjIiE`, accounts `smoke-20260929133603-fef6-{a,b,c}@example.com`;
  the DB already held such rows from earlier direct runs). Always use `composer test` (DB copies).

## Platform P2a

2026-09-29/30 — admin app `admin/` (details: PLATFORM.md "P2a — result"; app README `admin/README.md`).
- Admin on :11000 against the platform server on :9000 (`admin/.env.development`); sign-in tilman@porschuetz.de,
  one-time codes in `server/logs/mail.log`. The browser pane is 761px wide: desktop checks need the viewport
  emulation (1280x800) – the first screenshot after a navigation is often blank, take a second one; the SPA needs
  ~5–8 s for the first render in dev (single-threaded `php -S`, ~0.3 s per API call).
- Parity method that worked: open the legacy SPA page (`/generate/` on :9000) in a second tab and `import()` its
  `app/utils/pdf.js` / `printSheet.js` (globals `QRCode`, `jspdf` are loaded by its index.html, no login needed),
  `import('/src/lib/qr/pdf.ts')` in the admin tab (vite dev), render the same data, hash the output after removing
  `/CreationDate (…)` and `/ID […]` → identical hashes for CoA, batch CoA, SVG and print sheet.
- Test data (local DB only, all **revoked** afterwards, reasons "P2a admin test code/batch"): single code `ahhdknMu`
  (membership, no edition), batch `BATCH-2026-09-29T140225-99_catpope_admintest.json` with `J307HdAr`, `P8mBhY4n`
  (edition `99_catpope_admintest` exists only in a scratch copy of editions.json that a :9100 server used for the
  create call via env `QR_EDITIONS_PATH`; the batch itself ran in the admin against :9000). Their files are in
  `server/storage/files/{svgs,certificates,signatures,batch}`.
- Commits next to other agents: `git commit -- <paths>` (only those paths) or a temporary index
  (`GIT_INDEX_FILE=… git read-tree HEAD` + `git apply --cached` of the own hunks), never a plain `git commit` of the
  shared index; check `git status` for other agents' edits inside a folder before committing the whole folder (the
  creator commit had to be redone once because another agent's `creator/package.json` + `config.ts` slipped in).

## Platform P3

2026-09-30 — qr app, API-only server, deploy targets, versions (details: agents/PLATFORM.md "P3 — result" and
"Cutover checklist (all-inkl)"; deploy/README.md; qr/README.md).
- `qr/` = qr.scutoons.com, plain PHP (no Composer deps), :9001 (`npm run dev:qr`), config `qr/config.local.php`
  (`API_URL` http://localhost:9000, `QR_PROXY_SECRET` = the one in `server/config.local.php`). Landing HTML is
  byte-identical to the old server's; templates moved from `server/templates/landing/`.
- `server/` is the API only: `/?c=` and `/verify.html` answer 404 there now – use :9001. `composer test` starts the qr
  app itself (ports 9180–9185); `api-smoke.php` / `admin-smoke.php` take the qr base as 2nd argument.
- Scans: the qr app calls `/api/code/?c=&scan=1`; the API logs the scan only for the trusted proxy (secret header).
  Running the qr app against the dev DB logs real `scan` events there (as the old landing page did).
- The printed-URL suite lives in `qr/tests/public-urls.php` (`--api=` required). Baseline recorded before the change
  (scratchpad `p3/pu-baseline.txt`) = the P1b baseline; qr result identical (+ a revoked code that now exists in the
  local DB: `ahhdknMu`).
- Deploy: `node deploy/deploy.mjs <app|all> --dry-run` stages into `.deploy/<app>/` (gitignored); `api` runs composer
  in the staging copy (never touches `server/vendor`). Builds write `version.json` (`deploy/version.mjs`); on Windows
  composer's `vendor/composer/tmp-*.zip` can vanish mid-listing (handled).
- Env names: one FTP set (`FTP_SERVER/USERNAME/PASSWORD`), `VITE_WWW_URL` replaces `VITE_VIEWER_URL` (creator reads
  both; admin gets it via `buildEnv`), `.env.example` is the owner's checklist. `MAIL_FROM` must be a bare address.
- PowerShell gotchas: `git commit -m @'…'@` with quotes breaks native arg passing → use `-F <file>` (ASCII/UTF-8
  without BOM; `Set-Content -Encoding utf8` writes a BOM into the message). `[IO.File]` paths are relative to the
  process cwd, not the PowerShell location – use absolute paths.

## Runtime settings + scan limit

2026-09-29 — owner requirement: the QR scan limit is a setting in the admin (details: ITEMS.md "Admin API" → Settings,
server/docs/API.md "Rate limits").
- Table `settings` (`key`, JSON `value`, `updated_at`, `updated_by` = `admin:<email>`), migration
  `2026-09-30-settings` (dated after `roles-signup` so it sorts last; in the cutover list). Service
  `server/src/Domain/Settings/RuntimeSettings.php`: stored value wins, else config (`SCAN_MAX_PER_IP_HOUR` 1000,
  `ACCESS_MAX_PER_IP_HOUR` 120); rows read once per request; a missing table = defaults (production before the
  migration keeps working). `0` = no limit (nothing counted). Admin page `admin/src/routes/settings` (nav group System).
- Buckets: `qrscan:<ip hash>` for `/code/?scan=1` through the trusted qr proxy, `lookup:<ip hash>` for direct
  `/code/` + `/access/`. Not `scan:` – `InventoryService` already uses `scan:<userId>:<item>` for scan-grant cooldowns.
- Locally, curl `http://localhost:9001/…` reaches the qr app as `::1`, so the forwarded client IP (and the bucket) is
  `::1`, not 127.0.0.1. `run-contract.php` migrates the DB copy and deletes its stored settings (a stored value would
  override the suites' env limits); admin-smoke sets the scan limit to 3 → 4th scan via qr = 429 page, another
  forwarded IP 200, 0 = unlimited, then restores the previous values and deletes its `qrscan:` rows.
- Verified 2026-09-29 on the dev stack: admin UI scan limit 3 → `?c=membership_test` 200/200/200/429 → back to 1000 in
  the UI → 200. Local SQLite backup before the migration: `server/db/local.sqlite.bak-2026-09-29-settings`.

## Credits

2026-09-29 — credits model + creator Credits page + viewer credits from the store (paths: agents/PLATFORM.md → Data →
"Credits").
- Migration (`node --experimental-transform-types --no-warnings shared/content/scripts/migrate-credits.ts`, Node 22.14
  runs the TS package directly with that flag): catpope de/en `credits.json` (general, songs, performances,
  specialThanks) + `season1.json` `credits` + `toon.json` `authors` → 38 people, 40 shared roles, 2 custom roles,
  12 sections, 79 entries, 120 role entries. De/en roles aligned by position; ambiguous: "Toningenieur" and
  "Aufnahmeleitung" are both "Recording Engineer" in en → custom roles `recording-engineer` / `recording-direction`;
  normalized: "Musiker" → "Musiker:in", "Bariton"/"Baritone" → one role (de Bariton, en Baritone), "Translations
  English" → role `translation` + language `en` ("Translation English"). Names: "Thomas Dalby" (authors/season) =
  "Thomas B. Dalby" (alias), season de "Nova Lux" = en "Chris Nova Ludwig" (same slot → alias, **owner to confirm**);
  owner 2026-09-29: "Julia Edhofer" = **Julia Edthofer** (migration rule `NAME_ALIASES`, alias; duplicate entries in a
  section are combined) → 37 people; the two recording roles are confirmed as they are; Nova Lux still open. Many de labels are English in the source files
  (Idea, Voice, Trumpets, Soprano, …) – kept as they were. Lead ("Created by") = toon authors with their season-credit
  roles; season 1 section = the other season-credit people (Chris Nova Ludwig: music, sound).
- Local store: imported as published v1 (ids 101 credits, 102 people, 103 roles) = same content as the static files,
  restructured. Browser test (creator-test, headless Edge): test person + custom role saved as drafts v104–v106,
  restored as v107 (credits) / v108 (people). Then the Julia merge was imported with `--update` and **published**
  locally (owner decision): credits v109, people v110, roles v103 – newest = published for all three.
- Viewer check (headless Edge, cookie `PARAGLIDE_LOCALE`): every person and role of the old de and en files is on the
  page (38/38 people; only the intended normalizations differ). The old page always loaded `de/credits.json` – fixed.
- MetaPage got `validate` (cross-file problems) and `wide`; undo/redo acts on the most recently edited doc; the
  history panel switches between the page's files. `kindOfPath`: credits/people/roles.
- Headless browser testing without touching the owner's tabs: Edge `--headless=new --remote-debugging-port` with a temp
  profile, driven over CDP from Node (global WebSocket); cookies via `Network.setCookie`, OTP from `server/logs/mail.log`.

## AppShell (sidebar borders reach the ActionBar)

2026-09-29 — owner bug "the page sometimes scrolls and the sidebar border stops short of the bottom".
- Root cause: `ActionBar` pads `body` with `calc(var(--action-bar-height, 3.25rem) + var(--space-1))`, but the apps
  define `--action-bar-height` on inner elements, so `body` used the rem fallback (admin 68px, creator 102px at the
  fluid 24px root). Admin frame = `100dvh - topbar` + 68px → every page scrolled 68px and the sticky nav (`align-self:
  start`, content height) ended early (settings at 1280×800: 16px short when scrolled; /codes at 768: 384px short).
  Creator chapter editor = `100dvh - 64` + 102px → scrolled 102px, tree border 102px short.
- Fix: `@scutoons/ui` `AppShell` (topbar / sidebar / aside snippets, `mode` page|fill, `collapsible` drawer < 768px).
  It zeroes the body padding (`html body:has(.sc-app-shell)`) and reserves `--action-bar-height` itself; sidebar =
  sticky, exactly `100dvh - measured top bar - bar` tall, own scroll. Admin root layout uses it with the nav; the
  creator root layout wraps every page in it (pages keep their own top bars; `.creator` sets `--action-bar-height:
  64px`), the chapter editor's style-guide/problems/history panel now stops at the bar.
- Chrome: a fixed box with `top` + `bottom` and `align-self: start` (grid child) shrinks to its content – the old
  admin mobile drawer had that bug too; the shell sets `align-self: stretch` there.

## Characters

2026-09-30 — characters model + creator Characters/Cast pages + viewer from the store (paths: agents/PLATFORM.md → Data →
"Characters"). The model was revised with the owner many times during the day; final: Character → Persona → Look →
Mood × View, casting per season, open mood vocabulary, asset requests, A-pose on the character level; groups /
affiliations, the cast group and the mannequin field were added and removed again ("keep it small").
- Migration (`node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts`): catpope
  de+en `characters.json` → catpope, boy, crows; plf / cat / mountain-tiger became Cat Pope personas `human` / `cat` /
  `mountaintiger` (their colors → persona `color`, their images → the persona's default look, merged with the new
  `characters/catpope-{human,cat}/` neutral PNGs; their old single images are dropped). de/en path conflicts: de had the
  real PNG art for plf/cat avatar + portrait, en the 151-byte placeholder SVGs (and a broken `cat_portrait-neutral.svg`)
  → the PNGs win, so the **en reader shows the same art as de**. Missing on the CDN (kept, flagged): `boy_full.svg`,
  `crows_portrait.svg` (the crows' neutral portrait = an error in the problem list).
- Owner additions: Cat Pope own toon, personas regular (papal look, 17 moods), human, cat, mountaintiger, biker (looks
  "ohne Helm" 17 + helmet 17 + dizzy); yellowmouse (looks kimono 17 + tripping and helmet 17, transition helmet →
  default = `yellowmouse_keyvisual-helmet-off.png`, renamed 2026-09-30 → `yellowmouse_transition-helmet-to-default.png`); toptoy "duo" as one character (owner to split if needed).
  A-poses: catpope swim/naked + the dressed example as artwork, yellowmouse swim (SPHYNX / MOUSE anchors), yellowmouse
  naked = an open asset request. Casts season 1: catpope (catpope regular+human+cat+mountaintiger, boy, crows),
  yellowmouse (catpope regular+biker, yellowmouse), toptoy (toptoy).
- VN speaker plates change for the former characters: "PLF" → "Katzenpapst · Mensch" / "Cat Pope · human", "Katze" →
  "Katzenpapst · Katze", "Gebirgs Tiger" → "Katzenpapst · Gebirgs Tiger" (persona label appended to the name).
- Scene refs: `migrate-scene-refs.ts` rewrote `catpope/{de,en}/chapter6/scene-200.json` (static + store, published
  locally as v152 de / v153 en) into `{ character, persona, look?, mood?, variant? }`; chapter 2 scenes 14–19 only have
  narrator messages (no character refs). Pre-existing content bugs left as they are: scene-200 de has `crow` (not
  `crows`, never rendered); the en scene-200 shows "cat" before its joinScene, so en stops after Cat Pope.
- Local store: seed imported and updated several times with `--update`; newest = published. Browser test (creator-test,
  headless Edge): test character "Testfigur Zett" as a draft, added to toptoy season 1 via the picker in a joinScene of
  toptoy chapter 1, saved as drafts v141–v144, restored as v145–v148 (= the versions before), nothing published.
- Viewer check (headless Edge, membership code, cookie locale): chapter 6 scene-200 renders Cat Pope (new PNG),
  · Mensch, · Katze, · Gebirgs Tiger and Boy in de; en renders Cat Pope, then stops (content bug above).
- Creator gotchas: a new paraglide key must exist in de + en before a component uses it (else `m.creator.x is not a
  function`, the page hangs on "Loading …"); **removing** keys while the creator dev server runs breaks its module graph
  (404 on `paraglide/messages/<key>.js`) – keep unused keys until the dev server is restarted. `npm run check`
  regenerates `.svelte-kit` under the running dev server (harmless so far).
- Headless testing: Edge `--headless=new` with a temp profile, driven over CDP from Node; session token via
  `/api/auth/request` + OTP from `server/logs/mail.log` + `/api/auth/verify/`, stored in localStorage `scutoons_session`.
- 2026-09-30 (later) — owner fixes + avatar bodies:
  - Cat Pope · Biker `introduced` = yellowmouse S1C1. Transitions named after their states (PLATFORM.md → Characters);
    `cdn/root/characters/yellowmouse/yellowmouse_keyvisual-helmet-off.png` moved to `…_transition-helmet-to-default.png`
    (plain move, cdn/root not in git – **upload with `cdn/sync.py`**, together with
    `characters/catpope-biker/catpope-biker_transition-helmet-to-default.png`, `characters/yellowmouse/yellowmouse_apose-naked.webp`
    and `characters/_missing/`). Both transitions registered; the asset requests for the biker transition and the mouse's
    naked A-pose (now registered with MOUSE_WORN anchors) are `done` → no open requests. Seed + SQL regenerated; local
    store: characters v155, asset-requests v154 (published).
  - Scene fixes (static + store, published locally v156 de / v157 en): de `crow` → `crows`; en: the cat now joins
    before it speaks (as in de), every message waits, choice jumpTo + the final `end` event in the reader's jumpTo form,
    `{cucumbers}` → `{cucumber}`. Headless Edge: de plays 26/26 events, en 18/18, no console errors; both end on the
    final `jumpTo` (wait) to chapter 6 scene-300, which doesn't exist (content, open). Still German text in the en file:
    the opening narration and "Weg ist das mistvieh?" (owner to translate).
  - Missing-image substitutes: `cdn/tools/make_missing_images.py` (resvg, byte-identical on re-run) →
    `characters/_missing/missing_{avatar,portrait,full,apose}.png`; `resolveExpression(…, options)` / `characterImage` /
    `legacyCharacterImage` return them last (`missingImagePath`, `isMissingImagePath`); viewer VN marks paths whose `<img>`
    failed (`lib/utils/missing-images.svelte.ts`), creator marks failed CDN checks (`isKnownMissing` in
    `editor/characters.ts`). `crows_portrait.svg` / `boy_full.svg` stay missing → substitutes.
  - Avatar bodies & unlocks built (ITEMS.md "Reader avatar bodies & character unlocks" → Implemented). Local DB migrated
    (backup `server/db/local.sqlite.bak-2026-09-30-member-characters`). Local test accounts: `avatar-member-0930@example.com`
    (hoodie in inventory, yellowmouse granted by the check, currently wearing Cat Pope swim + hoodie) and
    `avatar-admin-0930@example.com` (role admin), made directly in the DB (no login mail). Headless check 13/13: member
    default Cat Pope swim/naked → admin grants yellowmouse → mouse appears, pick saved, survives reload, hoodie at the
    mouse torso anchor (and at Cat Pope's after switching back); guest Cat Pope only, stored per device, old `mouse`
    value → default.
  - Unused viewer message keys `items.avatar_body_{sphynx,sphynx_nude,mouse}` kept on purpose (removing keys under a
    running dev server breaks paraglide, see above) – remove after a dev-server restart.
