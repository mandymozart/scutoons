# MEMORY — facts & findings (scutoons)

Established 2026-09-28 during the chapter-2 investigation. Verify file/line refs before relying on them.

## Repo layout

- `viewer/` — SvelteKit (Svelte 5 runes), adapter-static, paraglide i18n (`messages/{de,en}.json`, base `de`).
  Deployed by `.github/workflows/main.yml` on push to `main` → FTP to `ftp.scutoons.com:/`.
- `viewer/static/api/` — the "fake API", served at `/api` and shipped with the app build (not the CDN):
  `toons.json`, `events.json`, `products.json`, `featured-products.json`,
  `catpope/{de,en}/{toon,season1,chapter1..5,characters,locations,routes,credits}.json`,
  VN scene files `catpope/{de,en}/chapter2/scene-14..19.json`; the dev-only sandbox toon `sandbox/` (see "Sandbox").
- `cdn/` — `root/` (gitignored, lives locally + on FTP), `sync.py` (interactive FTP push, "bidirectional" only
  in name), `download.py` (FTP pull), `server.py` (CORS static server on :8000). Env: `FTP_SERVER`,
  `FTP_USERNAME`, `FTP_PASSWORD`.
- `template/episode1.json` — obsolete legacy format, removed 2026-10-01 (owner; still in git history).
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

- 2026-09-30 (owner): the `images/1x/` and `3x/` folders are gone locally (still on the live CDN for the released
  viewer). Slices live in `images/webtoon/<sXcY>/<sXcY>-slice-N.webp`, 1x only, 390 px wide (standard 390 × 625),
  written by `cdn/tools/make_slice_webp.py` next to the delivered JPGs. Ch1: `webtoon/s1c1/s1c1-slice-1..94.webp` from
  `webtoon/de/S1C1 - Slice N.jpg` (799×1280, text baked in German – the owner's source, untouched). Ch2: the fresh set
  `webtoon/s1c2/s1c2-slice-1..22.{jpg,webp}` (390×625 JPGs = re-encode only), wired 2026-09-30 (see Chapters).
- Slice encode (2026-09-30, "1x but as sharp as possible"): Lanczos in sRGB from the biggest source + unsharp mask
  0.6 / 60 % / 2 (only after a downscale) + WebP q88 method 6. Linear-light resize was tried: it thins dark lettering.
  Pillow can't do `sharp_yuv`; the tool uses `cwebp -sharp_yuv` when `cwebp` is on PATH / in `CWEBP` (not installed
  here). A/B + sizes: PERFORMANCE.md "Slice sharpness". Always regenerate with `--force` from the biggest source.
- Key visual: `images/s1c1-key-visual.jpg` (1170×2535, moved out of 3x) + `.webp` (390×845, the video poster);
  video `videos/s1c1-key-visual.900w.mp4` (slice 1 of chapter 1; the 17 MB original stays). No s1c2–s1c5 key
  visual exists anywhere (local or live); ch2 keeps the dead path `images/1x/s1c2-key-visual.jpg` (object form),
  ch3–5 the string form `images/3x/s1cN-key-visual.jpg` – the owner has to pick the art.
- Season/toon cover: `images/s1-cover-{square,portrait}.{jpg,webp}` (1080², 1080×1920, restored from the live CDN by
  the coordinator); catpope season1 + toon.json (de + en) and the sandbox toon point to the `.webp`.
- Music `audio/music/*.128.mp3`, samples `audio/samples/*.128.mp3` (both 128 kbps CBR 48 kHz; originals locally
  `*.320.mp3`, the live CDN keeps the old names for the released viewer).

## Chapters (season1 sequence)

1 Denial/Leugnen published 2025-10-04 · 2 Anger/Ärger draft (config says 2025-11-01) · 3–5 coming-soon empty.
The old chapter 6 (VN test bed) was removed 2026-09-30 – the VN test bed is the sandbox toon now. Ch2 (2026-09-30, the 22-slice set `images/webtoon/s1c2/`) = 5 ref groups scene-14 (slices 1–4, the only one with
events: placeholder cucumber choice → scene-15 / scene-20), 15 (5–8), 16 (9–11), 17 (12–14), 18 (15–19; the old
duplicate scene-19 was merged into it and removed, static + store unpublished) + inline `vertical` group scene-20
(slices 20–22, regular scroll like chapter 1, not VN; the narration box stays on slice 20 over the sky – its en text
is still German). Slice 19 has a cucumber painted in, without any event. The owner adds more slices in the creator. Homepage `routes/+page.svelte:19-31` next-release block has typo
`2.&Auml;ger`.

## Known bugs spotted (not fixed)

- Ch1 audio samples at `slice.audio.samples` but code reads `slice.audio.audio.samples` → ch1 samples never fire.
- Scene files use `{"type":"end"}` → "Unknown event type".
- Ch3–5 `keyvisual` is a string, type expects `{image, video?}` (ch2 fixed 2026-09-30).
- ~~`Collectable` type lacks `name`/`image`~~ – collectables removed 2026-09-30 (see "Collectables removed").

## Membership / inventory (viewer)

- Membership: `lib/contexts/membership.svelte.ts`, localStorage `scutoons_membership`, one code only, validated
  once via `POST https://qr.scutoons.com/api/validate/` (`lib/api/membership.service.ts`), code arrives as
  `?code=` handled in `routes/+layout.svelte:18-31`. Types `membership | promo`.
- Inventory: `lib/components/toon/inventory/inventory.svelte` ("i" key) showed game variables that matched a
  collectable id (`game.svelte.ts`, localStorage `scutoons_game_<toonId>`). Only collectable: cucumber. (Historical:
  collectables were removed 2026-09-30, the inventory shows catalog items only.)
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
- `images/collectables/cucumber.png` (old VN art, no longer referenced, left on the CDN) is only 124px → cucumber icon is vector (same style as camera/cucumber_camera).
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
  (Superseded 2026-09-30 "Viewer locale": `/de/…` and `/en/…` now always win over the cookie.) The `/access/`+`/code/` per-IP limit (120/h) trips
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
  routes, credits stay static. `?preview=draft` on a reader URL loads the newest drafts with the session
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
- Reader language on unprefixed URLs is the cookie (see 2026-09-30 "Viewer locale"; `/en/…` always wins): the editor's
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

## Performance

2026-09-30 (details, numbers and proposals: agents/PERFORMANCE.md).
- The slowness was the dev stack + media size, not the file-based content store (SQLite read 0.2 ms).
- Dev: `npm run dev:api` / `composer serve` = `server/bin/dev-serve.mjs` (4 `php -S` workers on [::1]:9200-9203 with
  OPcache behind a Node proxy on :9000; `composer serve:php` = old single process). `cdn/server.py` is threaded and
  listens on 127.0.0.1 + ::1. Vite dev + `npm run check` compile paraglide as `locale-modules`; `npm run build`
  rewrites `src/lib/paraglide` as per-message modules → restart a running dev server after a local build.
- Prod: chapter 1 = 43 MB (31 MB 320 kbps music decoded up front, 11 MB eager 3x JPG slices) → 23-25 s on 4G;
  proposals P1 (music), P2 (lazy + WebP slices, needs slice dimensions) are the big wins.
- Measure: `node scripts/perf/measure.mjs <label> <url> [--throttle] [--init=file.js]` (headless Chrome, CDP).

- 2026-09-30 (later) — P1–P3 done + video slices (PERFORMANCE.md "P1–P3 + video slices — result"): chapter 1 on 4G
  42 MB / 22.3 s → 6.4 MB / 5.1 s (page without media streams), CLS 0.76 → 0. Music streamed through its own AudioContext
  (never Howler's: Howler closes/replaces its context), loads after the first 2 slices; slice-1 music now actually starts.
  `Slice.width`/`height` required (error), `video` optional (image = poster); reader lazy-loads all but the first 2.
  Character images: WebP next to the PNGs, registry points to WebP. Test helpers used: a CDP recorder of gain ramps /
  sample starts while scrolling, and a VN click-through counting character image bytes (both ad hoc, not committed).
- 2026-09-30 (later) — sharper slices (+21 % bytes), samples 128 kbps (chapter 1 on 4G 6.7 → 5.0 MB, page without
  media streams 4.8 → 3.0 s). Gotchas: a plain `vite build` bakes the PROD URLs (cdn.scutoons.com, qr.scutoons.com/api)
  into the preview – for local measurements build with `VITE_API_URL=http://localhost:9000/api VITE_CDN_URL=…`; a
  build while `vite preview` serves `viewer/build` kills the preview (and can fail the build); after a build run
  `node <app>/scripts/paraglide.js` (or `npm run check`) or the running dev server fails with "Failed to fetch
  dynamically imported module". (Locale in headless checks: fixed 2026-09-30, `/en/…` alone is English now – "Viewer locale".)
- Gotcha: `vite preview` (sirv) caches the file list at start – restart it after every `vite build`. (Earlier note "the
  `/en/` prefix alone renders German, set the cookie" is obsolete since 2026-09-30 "Viewer locale".)

## Sandbox (VN test bed) – chapter 6 removed

2026-09-30 — owner: remove catpope chapter 6 (the VN test bed / demo data) and build a sandbox that shows every
character, persona, look and mood and doubles as the VN test bed.
- **Chapter 6 is gone**: static `catpope/{de,en}/chapter6.json` + `chapter6/scene-200.json` deleted, season 1 ends at
  chapter 5 (de + en); local store: new published season1 versions, the four chapter-6 files unpublished (their versions
  stay as unreferenced drafts). No test, seed or script used it. Production never had it in the store.
- **Where the sandbox lives: a hidden toon `sandbox`** (not a catpope season 0). Why: the viewer loads only
  `season1.json` (layout, chapter gate via `parent()`), the creator's toon page, chapter editor (status), images and
  visibility helpers edit `season1.json` only, chapter numbers are unique per toon (a season 0 would need chapter numbers
  like 901+ in catpope's space), and `castSeasons()` ignores season 0 – multi-season support is not cheap. A draft toon
  needed **no app code**: its own season 1 / chapters 1–N, its own `cast.json` (everyone), the existing hidden-toon
  pattern (toptoy, yellowmouse). Hidden in production by status: `toons.json` entry `draft` (home + menu list only
  published toons) and every chapter `draft` in the season (production shows "not published"; checked in a
  `vite build` + `vite preview`). A direct `/sandbox` URL still shows the empty toon page in production (like any draft
  toon); a toon-level gate would be a separate viewer change (done: "VN layout, one-click rule, draft toons 404").
- **Generated, not hand-written**: `shared/content/scripts/make-sandbox.ts` (usage in `shared/content/README.md`)
  → `viewer/static/api/sandbox/{de,en}/{toon,season1,chapterN,chapterN/<persona>-<look>,locations,routes}.json`
  + `sandbox/_/cast.json`; then `php server/bin/console import-content viewer/static/api --toon=sandbox --update`
  (published locally). `import-content --sql` (the production seed) skips it (`ImportContentCommand::DEV_ONLY_TOONS`,
  unit test) – not in any seed/SQL. Encounter order Yellow Mouse, Toptoy, Boy, Crows (then any new character), then Cat
  Pope's own personas (human, cat, mountaintiger, biker); lines de (source) + en, mood labels from `moods.json`.
  Backgrounds / thumbnail / cover / toon avatar reuse catpope art via `../catpope/…` (slice paths are relative to
  `<cdn>/<toon>/`; the browser resolves the dot segment, works on any CDN, no new files). Chapters `isPreview: true`
  (no membership blocker).
- Today: 5 chapters, 11 VN groups, 211 messages per locale; 75 moods shown (Yellow Mouse 18 + 17, Toptoy 0 – no images,
  Boy 1, Crows 1, Cat Pope personas human/cat/mountaintiger 1 each, biker 17 + 18), 73 mimicked, 2 failed (`tripping`,
  `dizzy` → Cat Pope `sad` + a disappointed line); 4 look switches, 2 with a transition image (helmet → default for
  Yellow Mouse and the biker; default → helmet has none), Cat Pope lacks the helmet look twice (→ `sad`). Ends with a
  goodbye group (everyone met as avatars) and a jump to `/sandbox`.
- **VN renderer bugs the test bed found (fixed):** (1) `vn.clear()` on a VN group's destroy also cleared the toon's
  characters, which the layout sets only once → every later VN group of the toon (another label or chapter) had no
  characters; `clear()` now resets only the scene. (2) the chapter page keyed the group on `?label=` only → a jump to
  another chapter without a label kept the old VN component (it runs its events on mount) → stuck on the last message;
  the key is now chapter number + label.
- Verified (headless Edge over CDP, own temp profile, dev viewer :8000): de and en played to the end, 211/211 messages in
  file order, speaker plates, 394 portrait checks (mood badge + the look's mood image), both transitions shown, 0 console
  errors/warnings; only substitute: the Crows' neutral portrait (`crows_portrait.svg` missing on the CDN, known).
  Creator (creator-test): home lists "Sandbox · draft", `/sandbox` shows season 1 with chapters 1–5, chapter 1 editor
  0 problems.
- Local DB backup before the store changes: `server/db/local.sqlite.bak-2026-09-30-sandbox`.

## VN layout, one-click rule, draft toons 404 (2026-09-30)

- **Draft toons are dev-only**: the toon layout (`routes/(toons)/[toonId]/+layout.ts`) throws the normal 404 in a
  production build when the toon has no *published* `toons.json` entry (the root layout keeps only published ones) or
  its `toon.json` is `draft` (`isItemPublic`) → `/sandbox`, `/toptoy`, `/yellowmouse` and every page below them
  (chapters, credits, download) show "Not found"; dev shows them. Checked in `vite preview` of the build: /catpope,
  chapter 1 (membership blocker as before), credits, download, `?code=` unchanged. The static JSON under
  `build/api/sandbox/` is still shipped with the viewer build (not linked or routed; excluding it = a build step, open).
- **VN layout** (DESIGN.md → "VN layout"): owner decision – no `lead` field, the joinScene ref's view is the slot.
  `avatar` = left slot in the dialogue box while speaking (`vn/message.svelte`), `portrait` = right stage
  (`vn/characters.svelte` + `lib/visual-novel/stage.ts`), `full` unchanged. Mood badge removed. Creator: the view select
  is "Slot / Ansicht" with "Avatar (links)" / "Portrait (rechts)" / "Full (Mitte)". Sandbox generator: Cat Pope `avatar`,
  everyone else `portrait`; the goodbye brings everyone met back in waves of 4 (the 1–4 portrait stage test).
- **Last click needed twice (fixed)**: generated scenes end with `jumpTo` `wait: true`; the click on the last message
  resolved the message, then the jump waited for a second click. Now `continueFromWait` marks the message as answered
  (`message.wait = false`), `jumpTo` (wait) uses `vn.waitForContinue()` (resolves at once when the last message was
  clicked away or no message is shown; a message shown without wait gets the "…" and one click), the runner stops after
  a jump / choice navigation, and a group's box closes after its last message is clicked (`endOfEvents`). Same in the
  slice-trigger `EventProcessor`.
- **Regression check**: `node scripts/vn/vn-check.mjs [--locale=de|en] [--width=375] [--toon=sandbox] [--shots=dir]`
  (headless Chrome, own profile, locale cookie; needs the dev stack): per group clicks == waited messages and the jump
  happens; no portrait overlap, depth k+1 right of k, speaker in front and glowing, avatar slot left in the box, no mood
  badge. 2026-09-30: sandbox de + en at 375×812 and 500×900: 11/11 groups, 211 steps, 95 with the avatar slot, both
  transitions, 0 console errors. Stage boxes (x px, 375 wide): 1 = 154–360 (206); 2 = 14–198 | 204–361;
  3 = 14–145 | 152–264 | 269–361; 4 = 16–122 | 128–218 | 224–298 | 303–361 (500 wide: 4 = 17–162 | 169–292 | 298–399 | 405–484).
- Chapter 2 scenes 14–19 have no characters (narrator + choices); scene-14 renders and its choice jumps (de + en, with a
  local membership code). At the time of the check the local CDN had no `catpope/images/{1x,3x}` (media agent's slice
  work) → chapter-1/2 slices and the sandbox backgrounds were broken images locally; not a VN issue.
- 2026-09-30 (later) — **sandbox backgrounds + cucumbers**: the VN groups scroll vertically over the chapter-2 slices
  `../catpope/images/webtoon/s1c2/s1c2-slice-N.webp` (390 × 625): encounter 1–5 → scene 1–5 (slices 1–4, 5–8, 9–11,
  12–14, 15–19; all groups of an encounter share it), the goodbye → scene 6 (20–22). Three cucumbers in the art, each a
  slice click trigger (4 slice events with the same `trigger { type: click, rect, once }`: message "Eine Gurke!" /
  "A cucumber!", choices "Einsammeln" (`add cucumber 1`) / "Liegen lassen") in the first group that shows the slice:
  slice 4 `{x 7.69, y 37.76, w 22.56, h 10.08}` (ch. 1 regular-default), slice 8 `{x 78.46, y 7.52, w 8.72, h 8.64}`
  (ch. 2), slice 19 `{x 52.05, y 15.68, w 45.38, h 16}` (ch. 5 human-default). The owner expected the third one in
  scene 4 (12–14); it is in slice 19 (scene 5) – nothing green in 12–14.
- **Slice triggers inside a running VN group**: a trigger's message used to overwrite the group's waiting message and
  lose its click (the group got stuck). `EventProcessor.run` now parks the group's waiting message (`vn.interrupt()`),
  the trigger uses the box, and `resumeIfIdle()` (after the choice, `clearChoices`, or after the click's task) brings the
  message back with its wait → one click continues the group.
- **Story grant ref per place**: `grantStoryItem` sent `ref: vn:<variable>` → the server grants a ref once per account,
  so an account got only the first cucumber ever. The ref is now `vn:<variable>:<path without locale>?label=<group>`:
  each find counts once per account, replays grant nothing. Guests (localStorage) count every collect.
- Verified (dev, headless Chrome, real mouse events): all 3 cucumbers at 375 (de) and 500 (en): offer, collect, "+1
  Gurke" notice, guest `scutoons_inventory` cucumber 1 → 2 → 3, the group's message back and one click goes on; account
  `avatar-member-0930@example.com` (local): 3 cucumbers on the server, a replay stays 3, the inventory shows "3 Gurke".
- Gotcha: a `vite build` of the viewer (anyone's suite run) rewrites `src/lib/paraglide` as per-message modules → the
  running dev server answers 404 for them ("Failed to fetch dynamically imported module …/nodes/0.js") until
  `npm run check` regenerated the locale modules; it recovers without a restart.
- 2026-09-30 (later) — **"Sonntagnachmittag" / "Sunday afternoon"** = sandbox chapter 6 (`/sandbox/chapter/6`, dev only),
  the layout test bed, generated (`sunday()` in make-sandbox.ts) from the registry: everyone meets on the street
  (scene 5, slices 15–19: alley, lanterns, cobblestones). Cat Pope and his personas speak from the avatar slot, every
  other character (all personas) stands on the portrait stage: join one by one until all 9 are there (4 portraits at
  once today), speaker changes, a step back (re-join → back, further right), leave + come back, 2 moods each, every
  registered look transition (Yellow Mouse and the biker, helmet → default), everyone leaves. Played de 375 / en 500:
  38/38 clicks, 14 steps with the avatar slot, both transitions, 4 portraits without overlap (x at 375: 16–122 | 128–218
  | 224–298 | 303–361), 0 console errors.
- **Crowded stage** (`stage.ts`): side by side while the front portrait can stay ≥ 7 rem (≤ 4 at 375 px, ≤ 5 at 500 px);
  beyond that the sizes stay (front 98 px at 375) and the lineup stacks – same order, steps shrink evenly (overlap at
  375: 5 → 3 px, 6 → 16 px, 8 → 33 px). `vn-check` allows overlap only on a stacked stage (`data-crowded`).
- Fix for the stale dev server after a build: `touch viewer/vite.config.ts` (and `creator/vite.config.ts`) – Vite restarts
  itself in the same process and serves the regenerated paraglide modules (a touch of the paraglide files or messages
  is not enough).

## Creator/admin refinement: one resolution, Info panel, navigation (2026-09-30)

- **One resolution:** the chapter editor's 1x/3x zoom is gone; the canvas shows the slice at its 1x size (`width` of the
  slice, 390) scaled down to fit the column (`max-width`, aspect ratio from width/height before the image loads). No
  other density switch / srcset / 2x-3x handling existed in the apps (the CDN check's "not 390 px wide" is the real
  width and stays). New-file defaults: key visual `images/<sXcY>-key-visual.jpg`, covers `images/s<n>-cover-*.webp`.
- **Info panel** (`@scutoons/ui` `InfoPanel` + ActionBar `info` snippet / "Info" toggle, left-most, also narrow):
  closed by default, open state in localStorage `sc-info-open` (per app origin = per user/browser), stays open across
  pages. From 1100 px the AppShell content makes room (`--info-panel-width`, creator 360 px because its root font is
  fluid 24px; admin 24rem = 384 px). Page help = `creator/src/lib/components/page-help.svelte` and
  `admin/src/lib/components/page-help.svelte` (one `page` key per page), link to the full style guide = creator
  `/guide` (admin links there via `VITE_CREATOR_URL`). The chapter editor's Info also holds the whole style guide; it
  and the problems/history side panel close each other (`bind:infoOpen`).
- **Navigation** (DESIGN.md "Creator navigation"): the root layout owns the one top bar (breadcrumb) + sidebar;
  `usePageNav({ sidebar, extra, fill })` from a page registers its children / deeper crumbs (owner token, so a leaving
  page never clears the next one's entries). Chapter editor: `fill` mode, tree = sidebar snippet, selection mirrored in
  `?group=&slice=` (`replaceState`; URL → selection effect applies deep links once the chapter is loaded).
  Measured (Edge pane, 1280 and 768): top bar top 0 / 64 px high on home → toon → season → chapter → group → slice →
  back via Alt+ArrowUp, toon settings, characters; Alt+ArrowDown opened chapter 1, the group's first slice.
- **No file names in the UI:** `entityLabel()` (`@scutoons/ui`) + `fileLabel()` in each app; version chips
  "de · Draft v71" / "Published v70"; scene groups are created by name (`chapter<N>/<name>` stays the internal key).
- **No decorative arrows** on buttons/links (ActionBar "More", Menu, back links, "↗"); reorder/remove icon buttons have
  aria-labels.
- Gotcha: a Python script writing files on Windows must use `newline='\n'` (else CRLF); a heredoc piped into
  `python -` breaks on non-ASCII unless `PYTHONUTF8=1`.

## Collectables removed; cucumber camera only scans cucumbers (2026-09-30, owner)

- **Items are the only concept.** Collectables (the old VN inventory) are gone: `viewer/static/api/{catpope,sandbox}/
  {de,en}/collectables.json`, the `Collectable` / `CollectableRef` types (`@scutoons/content` visual-novel.ts), the toon
  layout's `collectables.json` fetch + `data.collectables`, the inventory's legacy icon fallback and the unused
  `inventory/collectable.svelte`, the sandbox generator's empty `collectables.json`, `make_item_icons.py`'s empty
  `COLLECTABLE_ICONS`. The server side (import-content, `/library/toons/<toon>` bundle, entities-compare, docs) was
  removed by the data-refactor agent; the local store has no collectables documents. `catpope/images/collectables/
  cucumber.png` stays on the CDN, unreferenced.
- VN variables stay story logic (`{cucumber}` interpolation, conditions); VN `set`/`add` of a catalog item id grants the
  item (`variable-actions.ts`). The inventory shows catalog items only.
- **Cucumber camera = `scan_object` only** (`server/config/items.json`, descriptions de/en say it no longer reads QR
  codes); the recipe is unchanged, so crafting trades the starter camera's `scan_qr` for cucumber detection. Viewer:
  a camera session has one mode (`items-ui.use`: `scan_object` wins → object mode, `qr: false`); the overlay's QR/Cucumber
  toggle and the `camera.mode_qr` / `mode_object` messages are gone; jsQR is not loaded for the cucumber camera.
  api-smoke checks the skills (camera `scan_qr`, cucumber_camera `scan_object`).
- Verified (headless Chrome, dev :8000): sandbox chapter 1 cucumber as guest (de) → `scutoons_inventory` gets cucumber 1,
  grid "Kamera, Gurke"; as `avatar-member-0930@example.com` (en; its chapter-1 grant ref row deleted locally first) →
  server 3 → 4, grid "4 Cucumber"; no `collectables` request, 0 console errors. Guest craft camera + cucumber → grid
  "Gurkenkamera", Use → "Finde eine echte Gurke", meter, no mode toggle, no jsQR request. Chapter 2 scene-14 only
  *mentions* cucumbers (message + "Weiter Spazieren" / "Auf Arbeit" jumps) – it has no `add cucumber`, so nothing to
  grant there; it renders with the promo code `promo_test`.
- 2026-09-30 (later) — **VN stage reworked with the owner (several rounds; DESIGN.md "VN layout" is the final rule set):**
  everyone in the scene is always visible and stands on the box (no in-box avatar any more, `vn/characters/avatar.svelte`
  removed; `portrait.svelte` renders both sides, `stage.ts` places them, the box reports its height to the context).
  Speaker in front of the box (z 101, glow), the rest behind it (z 90 − depth, brightness 0.85, 1 rem lower). Order per
  side = speaker, then recency of speaking (`lastSpoke` in the VN context), then the silent ones in join order. Left:
  framed 4 rem avatars, front one 0.25 rem inside the box, +1 rem per step to the right. Right: 9 rem portraits, −9 % per
  step, overlap 1/3 of the one in front → up to 2/3 when crowded → the side scales down (no "+N", nobody hidden).
  Intermediate owner rounds (no-overlap spacing, 1/2 avatar overlap, "+N" badge, in-box avatar) were replaced.
  Sonntagnachmittag: the "step back" beat became three speakers in a row (the recency order).
- `vn-check` asserts all of it per waited message, replaying the group's script: who is on stage per side, the order,
  the avatar positions (0.25 rem + k rem, 4 rem), right overlaps (33 % with room, ≤ 67 % crowded, equal within a side),
  grounding (speaker box top + 4 px, the rest + 1 rem), z against the box, glow, dimming. x is measured from computed
  left/width (the speaker's wiggle is a transform).

- 2026-09-30 — Paraglide builds no longer break running dev servers: all four apps compile `outputStructure:
  'locale-modules'` for dev AND build (vite.config.ts), so `npm run build` / `npm run check` no longer swap the modules
  under a running `vite dev`. The old workaround (`touch <app>/vite.config.ts` after a build) is not needed any more.


## Media: chapter-2 key visual + cucumber, Toptoy, avatar crops, cwebp (2026-09-30)

- **Chapter 2**: key visual `catpope/images/s1c2-key-visual.png` (owner's `s1c2-keyvisual.png` renamed) + `.webp`
  (390 × 845, q88) = `keyvisual.image` de/en. Cucumber on slice 19 (scene-18, the same rect as the sandbox) → offer →
  `add cucumber 1`. Fix needed for it: a group's VN host (message box) was mounted only when the group itself had
  `events`; `toon/group.svelte` now mounts it when any slice has events too (plain vertical groups with slice triggers).
  Verified guest de/en and `avatar-member-0930@example.com` (server 4 → 5, a replay stays 5). The chapter-2 page shows
  the promo-code sheet first with `?code=`; a click on the slice only closes it.
- **Toptoy** (`characters/toptoy/`): A-poses `toptoy_apose-{swim,naked}.png` as delivered (768 × 1152, figures at
  16–86 % of the height, i.e. not the spec's 5–92 %; a few stray alpha specks between the figures in the swim file),
  full = the owner's `22.webp` normalized to 600 × 1200 (scale 0.834, width-limited by the duo, 12 px side margin,
  soles at 96 %, head top at 27 %), portrait 1024 → crop (19, 19, 1024, 1024) → 600 (eye line = mean pupil centre
  y 431 → 41 %, eye centre x 50 %). Anchors `TOPTOY_WORN` in migrate-characters.ts (worn slots on the left figure).
  Before/after + anchor renders were scratchpad-only.
- **Avatars from portraits** (`cdn/tools/make_character_webp.py`, owner rule): no delivered 600 px avatar → square
  head-with-neck crop of each mood's portrait with one box per `<name>[_<look>]` (`cdn/tools/avatar-crops.json`,
  `"source": "manual"` = override). Box: neck = narrowest smoothed alpha row between 25 % of the figure and 75 % of the
  canvas, centred on the head's alpha centroid, side max(1.4 × head height, 1.2 × head width), then grown so every
  mood's head fits (a neutral-only box clipped raised ears / tilted helmets). Old 192/96 px avatar PNGs are ignored.
  `--sheet=<png>` = contact sheet. Full and A-pose PNGs get WebPs too.
- **cwebp**: `winget install Google.Libwebp --scope user` → `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Google.Libwebp_…\
  libwebp-1.6.0-windows-x64\bin\cwebp.exe` (no PATH link; set `CWEBP` to it). Slices with `-sharp_yuv` q88 vs the current
  Pillow q88: chapter 1 2.473 → 2.556 MB (+3.4 %), chapter 2 0.560 → 0.573 MB (+2.4 %), luma PSNR equal (44.5 dB), chroma
  +0.6–0.8 dB, no visible difference in 3× crops of the most colourful edges → **not adopted**, slices unchanged.
- **Sandbox Sonntagnachmittag** now also uses the `full` view (everyone with a delivered full image under
  `characters/…`: Toptoy, Bill; the legacy catpope `*_full.svg` are 550-byte placeholder silhouettes and are skipped).
  Full placement today (`vn/characters/full.svelte`, not redesigned, for the owner to look at): every full character
  is `position: fixed; inset: 0`, centred, `max-height 90vh / max-width 90vw` in a slot with z-index 1 → two full
  characters overlap exactly (at 375 × 812 both 600 × 1200 images are 337 × 674 px at x 19–356, y 69–743), not grounded
  (the box top is at y 626, so the lowest ~117 px – legs and feet – are behind the box), behind the portrait stage
  and avatars (z 86–101), no speaker-in-front / dimming rule (only the wiggle). vn-check does not assert full.
- `scripts/vn/vn-check.mjs` reads the scripts from the library API now (`--api=`, `SCUTOONS_TOKEN` for drafts outside
  dev). One en run had chapter-6 failures while the viewer was being switched live; two reruns passed.

## Fonts: IBM Plex Mono everywhere, measured in the browser (2026-09-30, owner)

- **Check:** `npm run check:fonts` (`scripts/check-fonts.mjs --session`; docs in `shared/tokens/README.md` → Check).
  Headless Chrome/Edge over CDP, 52 pages/dialogs of viewer, creator, admin, shop, qr (Info panel, character picker,
  claim sheet, inventory/avatar, VN dialogue box, main menu, account/login, guide, draft preview, certificate and print
  sheet pages, 404s). Per page: `document.fonts.check`, a loaded Plex face, a Plex woff2 answered 200, no font-CDN
  request, computed `font-family` of every visible text + form control, and the font actually rendered
  (`CSS.getPlatformFontsForNode`). `--session` = throw-away admin+creator user in the local DB, removed at the end.
  After the fixes: 52/52 at 390 × 844 and at 1280 × 900.
- **Failed before:** qr landing page (every state) loaded Plex from Google Fonts; qr `verify.html` had no Plex at all
  (Times New Roman text, Arial controls) and linked the old site's `https://scutoons.com/assets/styles.css`; creator
  and admin form controls in the UA font (checkboxes/radios of the characters page, cast, character picker, admin code
  list; empty icon buttons on the admin user detail: Arial); `<code>` in the generic `monospace` (Consolas) in the
  creator guide / chapter-editor help / characters page. Viewer and shop passed (viewer `static/css/styles.css` sets
  the controls' font itself).
- **Fixes:** tokens `base.css`: `:where(button, input, select, textarea, optgroup) { font: inherit }` and
  `:where(code, kbd, samp, pre) { font-family: var(--font-mono) }` (specificity 0, app rules still win). Hardcoded
  `'IBM Plex Mono', monospace` / `'Bangers', …` stacks → `var(--font-mono)` / `var(--font-lettering)` (viewer
  main menu, events, styles.css; reader dialog + credits). Commented Google Fonts links gone from `viewer/src/app.html`.
  qr: `qr/bin/sync-tokens.mjs` generates `qr/public/assets/tokens/scutoons-tokens.css` (tokens + Plex `@font-face` +
  base, relative urls) + the 14 Plex woff2 + OFL; committed (plain PHP, no build step); the tokens check (`npm run
  check`) and `qr.yml` fail when it is stale; `bin/` excluded from the deploy; `.htaccess` `AddType font/woff2`.
  Landing layout: the stylesheet instead of Google Fonts, its inline `:root` copy removed (from the tokens now),
  `p { line-height: inherit }` + `.faq h2 { font-weight: 700 }` keep base.css from changing it → computed styles and
  boxes of 5 states × 3 widths identical to before; body HTML identical; printed-URL `--out` diff = only the
  verify.html body sha1. verify.html: the local stylesheet + a few token-based input/button rules.
- Not failures (by design): glyphs Plex lacks (☰ ⓘ ✕ ★ ⋮ ⌕ →, emoji, CJK/Hangul on the home page) fall back to system
  fonts; the certificate/print-sheet PDFs use jsPDF `courier` (printed artefacts, byte-identical rule).
- **Found in passing (not fixed, reported):** `qr/src/Config.php` is not in git: `qr/.gitignore` `config.php` matches
  it on Windows (`core.ignorecase=true`). A CI checkout / deploy of qr has no `Config.php` → every qr page 500s.
  Fix: `/config.php` in `qr/.gitignore` + `git add qr/src/Config.php`.

## Avatar slots per body + Toptoy A-pose normalized (2026-09-30)

- **Toptoy A-poses** (`cdn/root/characters/toptoy/toptoy_apose-{swim,naked}.png` + WebPs, cdn/root not in git – upload
  with `cdn/sync.py`): stray alpha specks removed (connected components < 5000 px: swim 427 px in 17 islands – three
  opaque specks between the figures –, naked 156 px in 12), one transform for both (overlay IoU 0.989), soles at 92 %,
  centred, soft floor shadow per figure (blurred ellipse, alpha ≤ ~48, like catpope/yellowmouse; background stays
  transparent like every other A-pose file – the spec's "white background" is only how the delivery looks). **The spec's
  head top 5 % is impossible for the duo**: at that scale (1.26) the pair would be 931 px wide on a 768 px canvas, so the
  scale is width-limited (1.012, 10 px margins) → head tips at 22 %, soles 92 %. Originals, the one-off script
  (`normalize_toptoy.py`) and the before/after sheet were in the scratchpad only (originals also in the owner's
  `Downloads/toptoy`).
- **Slots per avatar body** built (ITEMS.md "Slots per avatar body"): `apose.looks[].slots` instances, Toptoy twins
  (`<type>-left/right`), server stores instance ids in `avatar_slots.slot` (no migration needed, legacy type rows are
  read by type), body switch keeps same-id instances else first free of the type. Seed + both SQL files regenerated (the
  content SQL was generated from a seed copy with the HEAD chapter files, since another agent's `featuring` edits were
  uncommitted), characters imported locally (backup `server/db/local.sqlite.bak-2026-09-30-avatar-slots`).
- Local test data: `avatar-member-0930@example.com` has Toptoy unlocked (admin API, ref `slots-check-2026-09-30`) and a
  second torso item, a `dago_hoodie` (black) entry `slots-check-dago-hoodie-0930` inserted **directly in the local DB**
  (no admin/story grant path for physical items); it now wears Toptoy swim with the kangaroo hoodie on `torso-left` and
  the Dago hoodie on `torso-right`. Browser check (430 px): 18 tiles "Kopf links … Füße rechts", picker filtered to
  torso items, both hoodies drawn on their figure, survives reload, Toptoy → Cat Pope keeps the first torso and drops the
  second, back → `torso-left`; item detail "Anziehen" puts on the first free instance; guest with a stored `torso-left`
  shows it on Cat Pope's `torso`.
- Gotcha: regenerating the seed while `composer test` runs makes entities-compare fail (the fresh DB is seeded at the
  start, the files are compared later).

## Cast & featuring, creator toon page, full view, vn-check (2026-09-30)

- **Featuring** (ITEMS.md "Cast & featuring" → Implemented): `Chapter.featuring` + helpers in `@scutoons/content`; the
  star rule in three places (creator `castWithStar`, `migrate-characters.ts`, library `/cast` via `Library::withStar`);
  all existing casts already had their star (catpope, yellowmouse, toptoy, sandbox) – nothing to migrate in the store.
  catpope chapters 1 + 2: `featuring: ["catpope"]` in the seed, imported with `import-content … --only=chapter --update`
  (published locally: 4 new versions). Chapter 2's scenes 14–18 still have no character refs, so "suggest" = the star.
  Open for the owner: who else appears in chapters 1 and 2 (Boy? the Crows?).
- **Creator**: toon page title = display name (links Toon settings), season title links the season editor, rows open the
  chapter; statuses in lists = read-only badges (`status-badges.svelte`), changed only in the chapter meta panel. Toon
  names come from `CreatorNav.toons` (loaded once the session is a creator – the old `onMount` request could run before
  the session was restored and leave ids in crumbs / up link). The toon page still shows "Chapters without a season: 6"
  for catpope: the removed chapter 6's unpublished drafts are still in the entity index (store data, not touched).
- **Full view** (DESIGN.md "VN layout" → `full`): centre group, `stage.ts` `fullLayout` (80 vh, ≤ width − 2 rem, 1 : 2),
  feet on the box's bottom edge, alternating ±1.5 rem per step, −9 % per step, z 70 − depth / speaker 95 (box 100).
  Sonntagnachmittag at 375 × 812: Bill (speaking) centre 187.5, 325 × 650, feet 798 (box bottom), z 95; Toptoy behind
  centre 166.5 (−1.5 rem), 296 × 591 (−9 %), z 69, dimmed.
- **vn-check flake – root cause**: not a VN race (10 stress runs with CPU throttling ×6 + 30 ms latency passed; a
  probe showed `Page.navigate` returns after the new document committed). The check itself was timing-based – a fixed
  420 ms sleep per step, 10 s for the group to start, 3 s for the next message, then it gave up silently – and it
  didn't notice when the **dev server reloaded or hot-updated the page** mid-group (another agent's / app's file change,
  dependency re-optimization): the VN component re-mounts and the group starts over, so the run shows "nothing shown /
  N clicks for M messages / no jump". A 10× loop of the old flow during parallel work reproduced it (e.g. biker-default
  39 clicks for 37, "right shows –"). It also leaked a ~120 MB Chrome profile per run in `%TEMP%` (64 `vn-check-*` =
  8.4 GB) until **the disk was full** (0 bytes free on C: – page loads failed with `chrome-error://`); those profiles
  were deleted and the check now removes its own. Fix: `settled()` (three equal measurements 150 ms apart), `open()`
  (retries a failed load, waits up to 30 s for the first message), waits up to 15 s for a due message, counts main-frame
  document loads + `[vite] hot updated` and **replays the group** (max 3 attempts, failures of a replayed attempt are
  dropped, printed as "replay"), robust `measure()` while a document is replaced; stress options `--cpu`, `--latency`.
  Other temp profiles of ad-hoc scripts (`perf-chrome-*` 3.8 GB, `edge-cdp-*` 2.9 GB, `vn-chrome-*` 1.8 GB,
  `sandbox-edge-*` 0.7 GB) are still in `%TEMP%` – not ours to delete, but they fill the disk the same way.
- **No version numbers in the UI** (owner): creator chips / history and the admin content overview show state words and
  dates (DESIGN.md "Creator navigation"); `ContentDoc.changedAt` holds the date of the version a locale is based on.

## Toon page: no featuring, start button, season music, credits colophon, supporters (2026-09-30)

- **Owner:** "this looks ugly … the reader frontend is supposed to be super logical and simple". Featuring removed from
  the reader (season line, chapter-list rows, chapter-start band; `toon/featuring.svelte` + messages
  `chapter.season_featuring` / `chapter.featuring` deleted); data + creator untouched. The dev-only debug overlay on the
  chapter page (top right "Kapitel 1 / description / Preview / Label") was removed by the coordinator (owner request).
- **Progress found:** the viewer's game context (`contexts/game.svelte.ts`) already had `progress: Checkpoint[]`
  (`uri`, `isComplete`) stored in localStorage `scutoons_game_<toon>` and synced for members to `/api/progress/<toon>`
  (server table `progress`, last write wins) – but nothing ever wrote a checkpoint (`addCheckpoint` unused, only a debug
  `console.log(hasCheckpoint(...))` in `toon/group.svelte`, removed). No scroll position, no "finished". Also fixed:
  `parsed.progress || {}` made a missing list an object.
- **Added (minimal):** `Checkpoint.position` (0..1) + `readingState` / `advanceCheckpoint` / `chapterCheckpointUri`
  (`@scutoons/content` progress.ts, node tests), `game.readChapter()`, `reading/track-reading.ts` on the chapter page
  (IntersectionObserver over `[data-slice-index]`, ≥ 50 % visible). Rule + UI: DESIGN.md "Start button". Server
  unchanged (the state is opaque JSON). In dev chapter 2 (draft) is readable, so "finished ch1" shows "S1 K2 lesen";
  in production (only ch1 published) the same state is "S1 K1 erneut lesen".
- Verified headless (scratchpad `shots.mjs`): the real flow with `?code=promo_test` – scrolling 16000 px of chapter 1 at
  375 px → position 0.29, button "S1 K1 weiterlesen" + 29 % bar; scrolled to the end → `isComplete`, "S1 K2 lesen";
  reopened → new read-through "weiterlesen" 1 %. Member `avatar-member-0930@example.com`: server progress with position
  0.73 → a fresh browser (empty localStorage) shows "Continue S1 C1" 73 %; the member's old server state
  (`cucumber: 2`, no checkpoints) was restored afterwards.
- **Paraglide locale in headless checks:** ~~a `/en/` URL alone renders German – set the cookie~~ wrong since
  2026-09-30 "Viewer locale": `/en/…` is English whatever the cookie; the cookie only decides unprefixed URLs.
- **Season music:** `Toon.download` → `Season.download` (DATA.md). Store: new published versions of
  `toon-settings:catpope|sandbox:{de,en}` and `season:catpope:1:{de,en}` (backup `server/db/local.sqlite.bak-2026-09-30-
  season-download`). The seed SQL regenerated keeping the old timestamps of unchanged rows (the generator stamps every
  row with "now"; a 4-statement diff instead of 42).
- **Credits colophon** (DESIGN.md): typography only, the `credits:<toon>` content as it is (owner: no PDF comparison, no
  new credits, no track list, no songs ↔ chapters). Streamed from the toon page's own `+page.ts` (the page does not
  wait for it) → one more library request on `/catpope` (credits).
- **Supporters:** `Season.supporters`, creator season page list (add / remove / reorder, name, url, logo via the image
  picker). catpope season 1 filled by the owner's instruction: SKE / austro mechana, Westbahnstudios, Künstlerhof
  Frohnau, logos from the vinyl PDF as single-path SVGs in `cdn/root/catpope/images/supporters/` (**upload with
  `cdn/sync.py`** – cdn/root is not in git). Rendered as `<img>` + `filter: brightness(0)` (ink; `currentColor` in an
  `<img>` SVG is black anyway).

## Hide sensitive content (2026-09-30)

- Owner: "Make it a member-wide setting. Hide sensitive content is on by default." Built as ITEMS.md "Sensitive
  content": `sensitive: true` on every `naked` A-pose look (registry, generic `SensitiveFlag`), member setting
  `users.hide_sensitive` (default 1, migration `2026-09-30-sensitive-content`), `GET/PUT /me/`, `422 sensitive_hidden` on
  `PUT /avatar/`, body moved to the safe look when the setting is turned on. **Guests always hide** (no toggle).
- Storage choice: a column on `users`, like `marketing_opt_in` (there is no preferences/settings mechanism per member;
  the `settings` table is the admin's runtime settings). Local DB migrated (backup
  `server/db/local.sqlite.bak-2026-09-30-sensitive`), characters re-imported (`import-content … --only=characters
  --update`), `characters-seed.mysql.sql` + `content-seed.mysql.sql` regenerated (characters statement only).
- Browser check (headless, 375 × 812, throw-away member, removed afterwards; script + screenshots
  `sensitive-{1-guest,2-member-default,3-toggle-off-naked,4-toggle-on-swim}.png` were in the session scratchpad): guest =
  Cat Pope swim, no picker; member default = toggle on, swim only, no picker; off → picker "Badehose" / "nackt", naked
  worn and kept after reload; on → body back to swim (stored), picker gone.
- Gotcha: two contract runs at the same time (another agent's `composer test`) share ports 9180+ and the mail log →
  dozens of unrelated FAILs (OTP not found, 401s). Rerun when the other run is done.
- The viewer at `/en/account` rendered German in the headless check – that was the locale bug, fixed 2026-09-30
  ("Viewer locale").

## Test user for agents + Cast page Save/Publish (2026-09-30)

- **Test user**: `agent@scutoons.test` (member, creator, admin; Cat Pope unlocked with every A-pose look, source admin,
  ref `dev:test-user`), created with the local promo code `promo_test` (the proper signup path; no raw SQL for the
  account). `npm run dev:test-user` [`-- --browser <local app url>` | `--reset` | `--fresh`] – details in PLAN.md "RULE:
  agents debug as the local test user". Session file `logs/dev/test-user.json` (gitignored). `--reset` is the only raw
  SQL (local SQLite: non-starter inventory, progress, avatar slots/body, codes it claimed; no API empties an inventory).
  The login mail subject is localized ("<otp> is your SCUTOONS code" / "<otp> ist dein SCUTOONS-Code") – the script
  reads the 6 digits at the start of the subject.
- **Cast page "Save draft / Publish don't work" (owner)**: with the test user in headless Chrome the save/publish
  mechanics worked (PUT/publish 200 for `cast:catpope` and `characters`, chips, reload) – no 4xx, no console errors.
  What the owner saw is the page without changes: Save draft was **disabled** (grey, looks broken) and Publish
  **returned silently** when everything was already published (`publishAll`: `if (!files.length) return;`). Plus a real
  dead control: unticking the only persona of an entry (Junge, Krähen: just "regular") cleared the box but changed
  nothing (`togglePersona` forces `[regular]` back), so the page stayed clean and Save draft grey.
  Fix (`MetaPage`, used by cast, credits, season, toon settings, characters): Save draft is always clickable (secondary
  style while clean) and says "Nothing to save …" when clean (Ctrl+S too); Publish says "Nothing to publish …"; the
  publish confirm names no `(_)` locale for language-neutral entities (`publish_confirm_neutral`). Cast editor: the last
  ticked persona is disabled (tooltip), drag reorder sets `dataTransfer` (Firefox) and prevents the default drop.
  Regression: `npm run check:creator-cast` (headless, test user, 18 checks, restores the published cast + characters;
  refuses to run while either has an unpublished draft; don't run it while the owner edits the cast – it creates versions).
- Chrome on this machine is `C:/Program Files (x86)/Google/Chrome/Application/chrome.exe` (not `Program Files`).

## Viewer locale: URL prefix wins (2026-09-30)

- **Bug (owner):** `/en/…` rendered German unless the cookie `PARAGLIDE_LOCALE` was set (a first visit to
  `scutoons.com/en/catpope` was German). Cause: `viewer/paraglide.config.js` set no strategy → paraglide's default
  `cookie → globalVariable → baseLocale`; `hooks.ts` strips the prefix (`deLocalizeUrl`) but nothing read it. Just adding
  `url` to the global strategy doesn't work either: with the default URL pattern (`de` unprefixed) paraglide reads every
  unprefixed path as `de`, so the cookie and the browser language would never be used.
- **Fix (config only):** global `strategy: ['cookie', 'preferredLanguage', 'baseLocale']` (unprefixed URLs, i.e. every
  printed/QR URL, unchanged byte for byte) + `routeStrategies: [{ match: '/:locale(de|en)/:path(.*)?', strategy: ['url',
  'cookie', 'baseLocale'] }]` (a prefix always wins). URL patterns stay the default: `localizeHref` gives `/en/…` for en
  and unprefixed paths for de; `/de/…` is understood. On the first `getLocale()` paraglide writes the resolved locale
  into the cookie (cookie is in both strategies), so a visit to `/en/…` makes the unprefixed links followed from there
  English too (no flip back), and a first unprefixed visit stores the browser language. The menu's `setLocale` writes the
  cookie and, on a prefixed URL, goes to the other locale's URL (`/en/x` → `/x`). No server redirects (the dev
  middleware's `shouldRedirect` never fires: unprefixed has no `url` strategy, prefixed URLs already match).
- The static build serves one fallback `index.html` with `<html lang="de">` (ssr = false, no text in it):
  `routes/+layout.svelte` now sets `document.documentElement.lang = getLocale()`; the dev server's `%paraglide.lang%`
  already agreed. Download page "back" uses `localizeHref`.
- **Check:** `npm run check:locale` (`scripts/check-locale.mjs`, headless Chrome 375×812, fresh profile): dev-server HTML
  lang + status, and after hydration `<html lang>`, UI text (de-only vs en-only strings of `viewer/messages`) and the
  API `?locale=` for: `/en/catpope` (fresh, cookie de), `/en/catpope/chapter/1`, `/en/account`, `/de/catpope` with
  cookie en, `/catpope` by cookie / Accept-Language en / de / fr, the menu switch de↔en (reload, internal link, back),
  and the printed URLs via qr (`/?c=<CODE>&skip=1` → unchanged `https://(www.)scutoons.com/catpope?code=…`, then that
  path on the local viewer in en / de / cookie en). 28/28. `--static` for a plain static host of `viewer/build` (the raw
  shell is always `lang="de"`, only its status is checked); verified that way too with a local-URL build.
- Other apps: **shop** already had `url → cookie → preferredLanguage → baseLocale` with both locales prefixed – correct,
  unchanged. **creator** / **admin** have no locale URL prefixes (UI language by own cookie → browser language), not
  affected.

## Prologue: chapter 0 = the "Roar Little Kitty" video, player video slices, CDN ranges (2026-10-01)

- **Owner:** the video (chronologically before chapter 1, a trailer) is catpope season 1 **chapter 0 "Prolog" / "Prologue"**,
  public (`isPreview: true`), published de + en, first in the sequence; reader texts never say "music video" (owner wording:
  description de "Der Prolog zur ersten Staffel." / en "The prologue to season one."). A normal chapter whose one group holds
  one **player video slice** – season/chapter logic and the creator unchanged apart from number 0 being allowed.
- **Chapter 0 everywhere:** entity keys `chapter|scene:<toon>:0[...]` valid (PHP `EntityKey::CHAPTER_NUMBER_PATTERN`,
  `ContentFiles` `chapter0.json` / `chapter0/<scene>.json`, TS `parseEntityKey` reads the raw digits – `Number('')` is 0, so
  `chapter:x::de` must stay invalid; seasons and leading zeros stay invalid); the library route already took `[0-9]{1,6}`;
  viewer chapter load `number > 0` → digits ≥ 0; chapter-item clamp `sanitizeNumber(n, 1, 0, 999)`; creator new chapter =
  max + 1 (unchanged, ch0 never collides), creator layout crumbs (`"0"` is truthy) and toon page fine (screenshots).
  content-smoke: `chapter:<toon>:0:de` is now a valid PUT; `00` / `season:…:0` still 400.
- **Start button / list:** `readingState` gets a `prologue` state (chapter 0 among the readable ones, nothing else read, the
  prologue not finished); everything else is computed over the other chapters, so ch0 is never the "first chapter" for
  "again" nor the last opened one. List row label from paraglide `chapter.prologue`; number column `min-width: 3rem` in a
  `6rem 1fr auto` grid (normal rows unchanged).
- **Player slice** (`SliceVideo.player`, `sources`): reader `slice-video.svelte`; ducking = the bubbling `sliceplayer` event
  (`SLICE_PLAYER_EVENT`, `SlicePlayerDetail`) caught by the viewer's `use:audio` on `.chapter-view` → `holdForPlayer`
  (pauses the streamed music tracks; music that starts while held waits and starts on release; `unload` clears the hold).
  Samples (Howler one-shots) are not paused. Progress: `trackReading` skips player slices in its IntersectionObserver and
  reads them from the same event (share = video progress, 1 at `ended` or ≥ 95 %). `afterFirstSlices` waits ≤ 4 s for a
  player video (preload metadata may never reach `loadeddata`).
- **Media** (cdn/root, not in git): `videos/roar-little-kitty.{480p,720p,1080p}.mp4` (encoded by the coordinator; 910×480 /
  1366×720 / 2048×1080), poster `images/webtoon/s1c0/s1c0-slice-1.webp` (780 × 411 = 2 × the slice width, frame at 11.2 s of
  the 1080p file: title card with the sphynx cat; Pillow Lanczos + unsharp 0.6/60/2, WebP q88 m6), thumbnail
  `images/s1c0-thumbnail.webp` (202 × 142 centre crop of the same frame). The slice is 390 × 206. The earlier
  `roar-little-kitty.1280w.mp4` is superseded (not referenced). Chrome picks `<source media>` per viewport at load: 375 →
  480p, 1280 → 720p, 1600 → 1080p (check-prolog asserts the network log).
- **`cdn/server.py` ranges:** single `bytes=a-b` / `a-` / `-n` → 206 (+ `Content-Range`, `Accept-Ranges`, CORS `Range` /
  exposed headers), past the end → 416, multi-range → 200 whole file; curl-verified (byte-identical slice of the file). A CDN
  process started before this commit still answers 200 – restart it (`npm run dev` brings it back after it was stopped);
  check-prolog then routes the video requests through its own throw-away `server.py` (CDP `Fetch.continueRequest` with a
  rewritten URL works across ports). Production hosting serves ranges natively.
- **Check:** `node scripts/check-prolog.mjs [--shots=<dir>] [--only=375-de] [--no-member]` – guest de/en at 375 + 1280
  (start button, list label + bar, no blocker, player attributes, ratio, resolution per width, sound via
  `webkitAudioDecodedByteCount`, music paused/resumed with chapter 1's music injected into the chapter-0 answer by CDP
  Fetch, seek to 200 s with 206s, partial + finished checkpoints, "S1 K1 lesen" after `ended`, chapter 1 still gated),
  1600 px → 1080p, and the test user as a member (finished checkpoint reaches `/api/progress/catpope`, the member's
  previous server progress is PUT back). Chrome flags `--autoplay-policy=no-user-gesture-required --mute-audio`.
- Open for the owner: chapter 0's `release` is set to the season's 2025-10-04 (not shown for published chapters).

## Player autoplay + the follow-up bar (2026-10-01)

- **Owner:** player videos autoplay (sound when allowed, else muted + "Ton an"); a follow-up bar at the end of a group /
  chapter with a 5 s countdown; then "Nochmal" / "Replay" as the secondary and the members case as the conversion moment
  (not signed in: "Anmelden →" + "Code einlösen" + a hint; signed in without a membership: "Code einlösen →" + "Wo
  bekomme ich einen Code?"; no countdown in both; "Nochmal ansehen" only for members). Spec: DESIGN.md "Player video
  slice" + "Follow-up bar".
- **Autoplay:** `@scutoons/reader` `SliceVideo` `autoplayPlayer` (+ `soundOnLabel`), threaded through `Slice`; only the
  viewer passes it (the creator preview stays still). `prefers-reduced-motion` is read synchronously now (an `onMount`
  read let the autoplay start for a frame). Chrome keeps the sticky user activation across SvelteKit navigations, so
  the hero's "Trailer ansehen" / the start button → the prologue plays with sound; a direct load gets `NotAllowedError`
  → muted + the button. Headless: `--autoplay-policy=document-user-activation-required` = the normal rule; a CDP
  `Input.dispatchMouseEvent` click counts as a gesture, `el.click()` from `Runtime.evaluate` doesn't.
- **Done signals:** the bar observes the group's last `[data-slice-index]` itself (`sliceSeen` / `SEEN_THRESHOLDS`
  exported from track-reading.ts, the tracker uses them too) and the `sliceplayer` events (`ended` / `play`); VN:
  `vn.state.eventsDone` (`beginEvents()` on the runner's mount, `endOfEvents()` sets it, `clear()` resets). All sandbox
  groups end with `jumpTo` → no bar there (they navigate themselves); check-next-bar drops sandbox chapter 2's final
  jump via CDP Fetch to test a scene that just ends.
- **`nextStep`** (`@scutoons/content` progress.ts, node tests): next group by `label` (a following group without a label
  is skipped → the chapter's end), else the next chapter of the season's **whole** `Season.sequence` (drafts included;
  the loaded list – which drops drafts in production – only adds `isPreview` / `release`): coming soon (with its date) /
  a draft not public here → "Kapitel 2 erscheint bald" (never skips to "Kapitel 3 erscheint am …") / readable (preview or
  member → `chapter`, else `members`) / nothing → the season's end. In dev chapter 2 (draft) is readable, so chapter 1's
  end offers "Kapitel 2: Ärger".
- **Login return:** `/account?return=<path>` (new, additive; same-origin paths only) → `goto` after the OTP login. The magic
  link in the mail doesn't carry it (opens the root).
- **Hidden map blocked clicks:** `.map.hidden` (opacity 0, `translate(0, 100%) scale(1.1)`) still covered the bottom ~5 %
  of the viewport (the scale grows it upwards, the map content is translated) and swallowed clicks there – the bar's
  buttons, before it anything at the very bottom. Now `pointer-events: none` on the hidden map and its subtree.
- **Check:** `node scripts/check-next-bar.mjs [--shots=dir] [--only=390-de] [--once]` – 390 / 1440 × de / en, guest +
  the test user: muted autoplay + "Ton an", in-app click → sound, every bar state (members not signed in / signed in
  without access via a stripped `/me/` answer, chapter + countdown → navigates, cancel, replay video / group, scroll-up and
  overlay stop it, next group → `?label=`, coming soon, season end via a trimmed toon answer, VN scene end), the login
  round trip (OTP from `server/logs/mail.log`) back into chapter 1, `?code=promo_test` → member bar, reduced motion;
  restores the test user's server progress. 88/88. check-prolog: the video now starts by itself (step 3 rewritten: the
  music waits while it plays, starts after a pause, pauses again on play).
- The test user can't be a signed-in non-member (`--reset` keeps its signup promo code, which grants `catalog`), hence the
  `/me/` rewrite in the check.

## Toon page desktop: trailer hero, 12-column grid, menu button, menu type (2026-10-01)

- **Owner:** desktop design "A · Vollbild" (artifact AYHvMG81k7CQc1JrVf4n5R, `project/Main.dc.html` as visual reference)
  from 1024 px; phone/tablet unchanged. Then: the credits colophon in the LEFT column (below the music button), not
  centred under the grid, filling the column's width (no 36em measure there). Spec + sizes: DESIGN.md "Toon page desktop".
- creator-cast check: the owner's cast:catpope now has one member (Cat Pope; boy + crows removed by the owner's own
  published version 10:36 UTC) – the reorder step assumed ≥ 2 members; it now creates the inline character first and
  moves that one up (works for any cast size).
- **Data:** `Season.trailer { loop, sources?, poster, chapter? }` (type, schema, validation incl. "chapter not in the
  sequence" warning, creator season page section "Trailer", library passthrough test). Desktop detection in the page
  (`matchMedia('(min-width: 1024px)')`, read synchronously so the phone never renders the hero); the hero reads
  `prefers-reduced-motion` synchronously too – reading it in `onMount` let the `<video>` render for one frame and start
  the loop download (caught by check-toon-hero).
- **Loop media** (cdn/root, not in git): `videos/s1-trailer-loop.{1280w,1920w}.mp4` = 12 pieces of 2 s at
  `i × (D − 2) / 11` (D = 474.795 s) of `roar-little-kitty.1080p.mp4`, concat, no audio, H.264 high, CRF 26, preset slow,
  faststart, 24.16 s, 3.8 MB / 8.0 MB; poster `images/s1-trailer-poster.webp` (1920 × 1012, frame at 1 s: the title over
  the city, q82). Piece 2 (≈ 43 s) is black in the source – kept, the spec says evenly spread.
- **Icons on the hero:** `html.on-hero` (hero bottom > 64 px) + global rules in `viewer/static/css/styles.css`
  (scoped component classes keep their names, `html.on-hero .x` out-specifies `.x.svelte-…`).
- **Menu:** `main-menu.svelte` menu button (z 1002, under the open slide-out 1003, so it never covers the language
  buttons); the toggles moved one slot left. Menu type before → after (computed): toon names + "Anmelden" 16 px vs the
  rest 24 / 14 px → all `--type-button-size` (24 px desktop, 14 px phone), line-height 1. Screenshots
  `menu-before-*.png` / `menu-after-*.png` (scratchpad).
- **Check:** `node scripts/check-toon-hero.mjs [--shots=<dir>]` – 1440×900, 1280×800, 1024×768 de + en and 1920×1080 en:
  hero box, muted/loop/playsinline, plays, exactly one loop file in the network log (requests filtered by the current
  document's `loaderId`), buttons + hrefs + inverted start button, icons white on / ink below, pause offscreen + in a
  hidden tab (`visibilityState` overridden) and resume, grid columns, colophon inside the season column, 112 × 72
  thumbnails, "Prolog" right-aligned; reduced motion = poster, no request; 390 / 768 no hero, no request, colophon at
  the bottom; the menu button at 1440 / 390 on the toon page and the home page + one font size; the test user's
  synced progress → "S1 K1 lesen" / "S1 K1 weiterlesen" + rule (previous server progress restored). Gotcha: a visit to
  `/en/…` writes the locale cookie, so unprefixed German pages need the cookie set per page in headless checks.

## Menu: toons first, logo → home; the record beside the music button (2026-10-01)

- **Owner:** slide-out with the toons first, no "SCUTOONS" entry, the logo top left links to the landing page and the
  menu opens only via the menu button (d657f8c); the vinyl cut-out next to "Musik herunterladen" (desktop ≈ 150 px left of
  the button in the season column, phone / tablet a smaller one beside the centred button). Spec: DESIGN.md "Top-right
  icon group", "Toon page", "Toon page desktop"; data: DATA.md "Music image".
- `main-menu.svelte`: the logo is `a.logo-link` (was `.main-menu-toggle button`, which opened the menu; `html.on-hero`
  rule in `static/css/styles.css` renamed). Entries are links now (toons, account, shop, events), so a middle click /
  "open in new tab" works; the shop link is the plain `shopUrl(locale)` (was `window.location.href` in a handler). The
  languages lost their `onkeydown={() => setLocale(locale)}`: any key – Tab included – switched the language while
  tabbing through. The closed slide-out is `inert` (it was only translated off screen, so Tab walked through hidden
  entries). Fixed in passing: the account entry was squeezed to the 3rem language-button width (its icon stuck out
  left over the menu border) – the square sizing is for the language buttons only now.
- Checks that clicked the logo to open the menu now click `[data-testid="menu-toggle"]` (check-fonts, check-locale).
  check-toon-hero: + menu order / inert / focus / Escape / logo link at 1440 + 390 de/en, + the record at 390 / 768 /
  1280 / 1440 de/en (src, `alt=""`, lazy, loaded, left of the button, centred vertically, width per layout).
- Media (cdn/root, not in git, **upload**): `catpope/images/s1-music-vinyl.webp` (from `catpope/items/awakening_vinyl.png`,
  the `awakening_vinyl` item icon; 19 KB instead of the 264 KB PNG).
- Flaky once each, green on the rerun (not touched here): check-fonts (51/52, the failing page not printed on the green
  run) and check-toon-hero's "plays again on the hero" at one desktop size.


## Hero: "Ton an" toggles the loop's own sound in place (2026-10-01)

- **Owner:** the desktop hero's "Ton an" / "Sound on" toggles the background loop's sound in place (it opened chapter 0);
  only "Trailer ansehen ▶" navigates to the prologue (besides the start button); a click on the video / scrim does
  nothing. Spec: DESIGN.md "Toon page desktop" → Hero.
- **Loop audio:** the 24 s montage (12 × 2 s) gets ONE continuous excerpt of `Roar Little Kitty.mp4` (Proton Drive
  `Projects/Cat Pope`), not cut-up music: **318.9 s – 343.06 s (5:18.9 – 5:43.1)** = the entry of the last full-energy
  section (short-term loudness jumps from ≈ −22 to ≈ −12 LUFS at 5:19 and peaks ≈ −8 at 5:40; the loudest 24 s window of
  the song by `ebur128` short-term mean). Length = the video's 24.16 s, `afade` in 0.5 s / out 1 s, two-pass `loudnorm`
  I −16 / TP −1.5 (linear; result −16.4 LUFS integrated, peak −2.1 dBFS), AAC 128k 48 kHz stereo, muxed with
  `-c:v copy -movflags +faststart` into `cdn/root/catpope/videos/s1-trailer-loop.{1280w,1920w}.mp4` (3.8 → 4.2 MB,
  8.0 → 8.4 MB; the old files in the session scratchpad). **Upload** both with `cdn/sync.py`.
- `hero.svelte`: `muted` state (true on every load, never remembered), the sound control is a `<button aria-pressed>`
  with `data-icon="speaker"` / `"speaker-muted"`; one `update()` decides play/pause (in view + visible tab, with reduced
  motion also only while the sound is on); the IntersectionObserver effect reads `muted` with `untrack` so a toggle
  doesn't tear it down (that would pause/replay). Reduced motion: the click sets `started`, the video renders and its
  effect plays it with sound (sticky user activation from the click). Messages: `hero.sound_off` new,
  `hero.sound_on_title` removed.
- check-toon-hero: at 1440 / 1280 de + en real CDP mouse clicks (a user gesture, unlike `el.click()` from
  `Runtime.evaluate`): background click → URL + mute unchanged, no pointer; "Ton an" → unmuted + playing in place,
  label/icon/aria-pressed, `webkitAudioDecodedByteCount` > 0; scroll away pauses, back resumes unmuted; "Ton aus" mutes;
  reload → muted; "Trailer ansehen" → chapter 0. Reduced motion: "Ton an" plays with sound, "Ton aus" stops. 79/79.

## Landing page "D · Brutal", announced toons, featured editions in the admin (2026-10-01)

- **Owner:** the landing page rebuilt after design "D · Brutal" (artifact XSq3qwDDuf8qDmKSAhK64a, `project/Brutal.dc.html`
  + `BrutalPhone.dc.html` as visual reference; the draft left out the fixed animated logo by mistake – kept). Spec:
  DESIGN.md "Landing page"; data: DATA.md "Announced toons + the landing tile"; API: ITEMS.md "Featured editions".
- **Announce:** toon settings `announce` + `tile { image, color? }`; `/library/toons` lists announced unpublished toons
  with `announced: true` (+ every entry's `tile`); the viewer's root layout keeps `isToonListed` (published or announced),
  the toon layout's production gate wants `status: published` (an announced toon's pages stay 404 on the server too).
  `Library::FORMAT` 1 → 2 (the toons answer changed shape; old cache entries are never read). The sandbox is excluded from
  the public list explicitly (`Library::DEV_ONLY_TOONS`, also `ImportContentCommand::DEV_ONLY_TOONS`).
- The local store had owner-made toon settings for toptoy (published, "Top Toy", texts) and yellowmouse (drafts) that the
  seed lacked: the new seed files are those + `announce`/`tile` (the leftover `download` dropped). The store's
  `toon:toptoy` keeps the owner's `sourceLanguage: en` (the seed says `de`) – not touched. Top Toy (coordinator/owner): the
  index names and the registry name are "Top Toy", the entry got `avatar: "avatar.webp"` (the duo); published as the
  test user through `/api/entities` (versions 436, 437). The sandbox's generated VN texts still say "Toptoy"
  (`shared/content/scripts/make-sandbox.ts`, dev only).
- **Featured editions:** admin page `/homepage` ("Startseite" / "Homepage", nav Data), `GET/PUT /api/admin/featured`,
  stored as the runtime setting `featured_products` (table `settings`, existing migration → nothing new for the cutover).
  The public `/products/featured/` now has `max-age=60` (was 300) so an admin change shows within a minute. While testing
  a 4-item list (vinyl, hoodie, heavy T, amulet) was already stored as `agent@scutoons.test` at 13:14 UTC – not by this
  task; check-landing restores whatever was stored before it.
- **Check:** `node scripts/check-landing.mjs [--shots=<dir>] [--no-member]` – 390 / 768 / 1280 / 1440 de + en (logo,
  wordmark, tiles from the API incl. colours / helmet / no sandbox / no dates, editions in the API order with price,
  pieces, struck sizes, buy link, slider per view + arrow, footer, menu) + the test user changing the list in the admin UI
  (remove, add, save) → the landing page → restore. 51/51.

## Toon settings without text live once in the toon entry (2026-10-01)

- **Owner:** in the creator's toon settings, fields with no text were edited and stored per language (screenshot: tile.image
  with "per language", the map at 0 in one language). Now `authors`, `chapters`, `thumbnail`, `map`, `announce`, `tile` are
  stored once in `toon:<toon>` (`TOON_NEUTRAL_FIELDS`); `release`, `status`, `cover` stay per language on purpose (catpope's
  en release is later, status gates a language, toptoy/yellowmouse covers are per-language files). Spec: DATA.md
  "Language-neutral toon settings in the toon entry".
- Library merges the entry's fields into the bundle's `toon` (entry wins), strips them from `entry` and from the list
  (except `tile`) → viewer unchanged. Old data (fields still in the settings) keeps working for `announce` / `tile`.
- Creator `toon-editor.svelte`: `docs={[settings, toonIndexDoc(toon)]}`, block "Same in every language" (testid
  `toon-neutral`, sub-blocks `toon-landing`, `toon-map`); the publish confirm says "Publish Toon entry?" when only the entry
  changed. Entry label "Toon entry" / "Toon-Eintrag" (creator + admin, `EntityLabels.entry`).
- Verified as the test user in the creator: tile.color `#ff0000` + map center 1300/450 → Publish → `/library/toons` +
  bundle and the viewer (red catpope tile on `/`, `/catpope` map `translate(-587.5px, 0px)` at 1425 × 900) → restored
  (empty colour, 1320/440; the restored `toon:catpope` JSON equals the migrated one, library answers identical to before
  the migration; map back at `translate(-607.5px, 10px)`). Gotcha: the library's `max-age=60` keeps the old bundle in the
  browser for a minute after a publish – reload after that.
- Gotcha while testing: another agent's dev-port work restarted the Vite servers, and every HMR re-mount dropped unsaved
  creator edits (also the screenshot lagged once) – check field values with JS before saving. The pane's `confirm()` returns
  false, so Publish needs `window.confirm = () => true` in the test tab.

## Local dev ports: one file, `shared/dev-ports.json` (2026-10-01)

- Owner: "Where do you define the ports … in code, no special place, a single source of truth." Every consumer reads
  `shared/dev-ports.json` (list in PLATFORM.md "Local dev ports"); Node scripts via `scripts/dev/ports.mjs`
  (`local('api', '/api')`), composer/npm `php -S` scripts via `scripts/dev/php-serve.php <service> <docroot> <router>`.
- The API allows every port of the file as a CORS origin on `php -S` only (`Settings::allowedOrigins()`, also the magic
  link origin in `LoginService`); the Embassy sign-in from :13000 worked with the 13000 entry removed from
  `config.local.php`. Production never runs `php -S` and has no `shared/` folder.
- `npm run check:ports` (first step of the root `npm run check`) catches hard-coded ports in vite configs / apps'
  `config.ts` / `up.mjs` / package + composer scripts, mismatching URLs in the `.env.example` / `config.local.php.example`
  files and a drifting PLATFORM.md table. `.claude/launch.json` is gitignored: regenerate with `npm run dev:launch-json`.
- Not covered (can't read JSON): `cdn/.htaccess` (production CDN, allows `http://localhost:8000`), the CI qr smoke in
  `.github/workflows/qr.yml` (own `php -S 127.0.0.1:9001`), server PHP smoke scripts' usage examples.

## Language-independent media: covers in the toon entry, rendered lettering, audit (2026-10-01)

- **Owner rule** (PLAN.md RULES): media are shared by all languages, text only in text fields, baked lettering → text the UI
  renders. Spec + audit table + media requests: DATA.md "Language-independent media".
- `cover` is a language-neutral toon setting now (`TOON_NEUTRAL_FIELDS` / `Library::TOON_NEUTRAL_FIELDS`), `Cover.lettering`
  turns on `CoverArt` (`@scutoons/reader`): toon name + season title in Bangers over text-free art. catpope keeps its baked
  German cover with the flag off; toptoy/yellowmouse have no cover (their `images/1x/<l>/` paths never existed).
- The creator's toon settings show the cover once ("Same in every language" → Cover) with a per-language preview incl. the
  rendered title over today's image (a blur `veil` behind it); the open media requests of the toon are listed there and in
  Wanted images → "Toon media". Asset requests with `view: "media"` + `toon` + `size` = toon media (no character).
- Validation warns on media paths with a locale segment in every entity (`media.ts`); the seed has none left.
- Store migrated as the test user (backup `server/db/local.sqlite.bak-2026-10-01-covers`); library answers compared before/
  after (46 URLs): identical except toptoy/yellowmouse losing the dead cover. Seed SQL: only the 8 changed statements.
- Gotcha: Python's text mode on this Windows box writes CRLF – edit files with `newline=''` (or the Edit tool), else
  `schema.js --check` reports every schema with a multi-line JSDoc as out of date.
- Season covers stay in the season documents (no neutral season entity; one creator field writes every locale); the
  validation warns when they differ between languages.
- Audit side findings (not fixed): `viewer/static/favicon.svg` is still the Svelte logo (admin/creator/embassy use it); the
  catpope placeholder SVGs in `images/characters/*` carry German names; ch2 slice 1 has the German SFX "FLATTER FLATTER"
  baked in (clean art requested); toptoy's draft toon page has an empty map terrain `<img>` (`map.layers.terrain: ""`).

## Toon script / Drehbuch (2026-10-01, branch `toon-script`)

- **Owner:** "Per toon a place where I can put scripts … a huge text block which I can edit and that is versioned, like
  the rest … scripts come per toon, not per chapter." Spec: DATA.md "Script" (entity `script:<toon>:<l>`), DESIGN.md
  "Creator: script page".
- Format source: `C:\Development\screenplay-editor` (README "Writing Guide", `lib/src/parse.js` + `utils/regex.js`). Its
  `lib` is NOT used as a dependency (outside the repo, built with jspdf, German-only markers): `scriptOutline` in
  `@scutoons/content` re-implements only the heading rules (+ `Chapter` / `Song` for translations).
- Cat Pope screenplay: `C:\Development\cat-pope-agent\external-repos\cat-pope-awakening\{de,en}\screenplay.txt` + `config.json`
  (repo last commit 2025-01-08; de complete 1834 lines, en partial 395 lines). LF, no tabs, 351 / 137 lines with
  trailing spaces – they are part of the file and are kept. Re-import a newer version: `import-screenplay.ts` then
  `import-content shared/content/seed --toon=catpope --only=script --update`.
- Gotchas: `ContentDoc.publish()` publishes every stored locale (an unchanged one keeps its version – harmless); the
  textarea → doc write is debounced, so `beforeSave` / blur flush it; scripts never reach `Library\Snapshot` (cache keys
  unchanged by script saves).
- Local store: `script:catpope:{de,en}` published (482, 483), backup `server/db/local.sqlite.bak-2026-10-01-script`.
  The seed SQL is unchanged (scripts are creator-only, `--sql` skips them).
- Check: `npm run check:creator-script` (27 checks: byte-identical load + file, outline 6 chapters / 39 scenes / 6 songs,
  jump, typing ~17 ms/keystroke with no long tasks, edit → draft → publish → history → restore byte-identical → publish,
  en switch, empty page for the sandbox, restores the published versions).
- **Merge note (`toon-script` → `platform`)**: branched at bab74a3, `platform` merged in at 264baa6 (one conflict in
  `shared/content/src/validate.ts`: both imports kept; `checkMediaLocale` skips `script`). Merge is a fast-forward-able
  merge commit. Until it is merged, the owner's running API (old `EntityKey`) answers 400 for `script:` keys and
  `export-content` on the local store would stop at the 2 script rows (unknown kind) – merge before exporting. No
  migration, no new dependency, no seed SQL change; production gets no script rows unless someone runs
  `import-content --sql --only=script` deliberately. Fresh worktrees need the gitignored `server/storage/keys/keypair.json`
  and `storage/files/batch/*` copied for the contract suites (admin-smoke batches, public-urls keys) – not a code issue.

## Single sign-on (2026-10-01, branch `sso`)

- **Owner:** one sign-in for every app (roles permitting); a signed-in account without the role sees "Dein Konto (x@y) hat
  keinen Zugang zu dieser App." + "Mit anderem Konto anmelden" (= sign out everywhere → that app's sign-in); one shared
  component. Built on `sso` (from `platform` a83b1cc), not merged – the owner merges after the 2026-10-04 release.
  Design, role matrix, cutover + merge/rollback: PLATFORM.md "SSO — result"; API + security: server/docs/API.md "Single
  sign-on"; browser side: `shared/auth/README.md`.
- Cookie `scutoons_sid` (HttpOnly, SameSite=Lax, Path=/api, Secure except http on loopback, Domain=scutoons.com only for
  hosts under `SESSION_COOKIE_DOMAIN`), set by `/auth/verify/` only for `ALLOWED_ORIGINS` requests; Bearer still works
  and wins. CSRF = Origin check on cookie writes (`403 csrf_rejected`). `GET /auth/session/` answers 200 `{ user: null }`
  when signed out (a 401 there was a red console line on every guest page view). Logout clears the cookie; `{ all: true }`.
- `POST /auth/session/` (Bearer → cookie) moves tokens stored by older builds; the browser checks (`scripts/check-*`)
  still inject `localStorage.scutoons_session` and keep working through that path. **Signing out in a check now means
  deleting the cookie too** (`Network.deleteCookies` `scutoons_sid`, done in check-fonts); `localStorage.clear()` alone
  leaves the browser signed in.
- **Deploy order: api before the frontends** – the new apps send `credentials: 'include'`, which the old api's CORS
  (no `Access-Control-Allow-Credentials`) makes the browser reject.
- Owner decision (same day): admins get into the creator **for now** – server `Roles::CONTENT_EDITORS` = creator, admin
  (`requireContentEditor`: entities read/save/versions/publish/unpublish/restore + library `?draft=1`; 403 still says
  `role: creator`), client `APP_ROLES.creator` = creator, admin. Narrow later by changing just those two lists.
- Isolated test runs from a worktree: API `QR_ALLOWED_ORIGINS=… node server/bin/dev-serve.mjs --port=<p>` on a DB copy,
  apps `VITE_API_URL=… npx vite dev --port <p> --strictPort`; `scripts/dev/creator-cast-check.mjs` takes
  `SCUTOONS_API` / `SCUTOONS_CREATOR`. The local dev DB already holds data of the main checkout's uncommitted "neutral
  toon settings" work (`announce` / `tile` in the index entry), so `check-landing` against committed `platform` code
  reports "no announced toon" (not an SSO issue).
- Check: `npm run check:sso` (15 checks; a throw-away member account via the promo-code signup, deleted afterwards).

## Wardrobe: real clothing items with overlays (2026-10-01, branch `wardrobe`)

- Owner decision + the build: ITEMS.md "Wardrobe" (default wearables T-shirt / swim pants / shorts as starter items, full-body
  overlays per body in `items.json` `wear`, swap keeps the old item, cover rule while "Hide sensitive content" is on, naked
  base only when allowed, one picker tile per character). Branch `wardrobe` from `platform` a83b1cc; merge after the
  release (merge note in ITEMS.md).
- **cdn/root could not be written from the agent worktree** (permission guard: shared resource). The placeholder overlays and
  icons were made into a scratch CDN root; the owner (or the next session with permission) runs
  `cdn/.venv/Scripts/python cdn/tools/make_wear_overlays.py` (default `--root` = `cdn/root`, `--sheet x.png` = QA sheet) and
  uploads with `cdn/sync.py`. Until then the viewer falls back gracefully: a missing legs overlay never loads → the safe
  swim base stays; a missing icon shows the initials placeholder.
- Extraction tricks: difference of the **blurred** A-poses (sigma 2.5) instead of a blurred difference (Top Toy's fur noise
  otherwise passes); hysteresis 60/24 for the low-contrast white waistband; wide opening + biggest blob per half + regrow
  over the weak mask drops the tail drawn slightly differently in swim vs naked. The dressed reference's colour masks: shirt
  S < 0.075 (sphynx skin S 0.10–0.20, hue 8–17°), shorts hue 190–250° S > 0.15.
- Lazy default wearables: `missingStarterWearables` = starter items with a slot the member owns none of (wearables are
  never consumed); `test-user --reset` deletes those rows too so the next `/me/` re-grants and wears them like a signup.
- Local test env of the worktree: own `server/config.local.php` + a copy of the owner's `server/db/local.sqlite` (read only),
  `storage/files/batch` copied for admin-smoke, a throw-away fake `storage/keys/keypair.json` (not a real key) for the
  `/api/keys/keys.php` check; servers on 127.0.0.1:9770 (API), 9771 (scratch CDN), 9772 (viewer). Gotcha: other agents
  listen on `[::1]:9490/9491/9492` – `localhost` resolved there; bind and call `127.0.0.1` explicitly.
- 2026-10-01 (later, owner): **the basic dress = T-shirt + shorts + sandals** (`default_worn`), `scutoons_sandals` (feet)
  added, the swim pants stay a starter in the inventory as the sensitive-content safety. A body without a shorts overlay
  (yellowmouse, Top Toy) gets the swim pants instead (`basicDress`). Sandals overlay: moved + scaled per foot from the
  dressed reference (its feet stand wider and 20 px higher than the naked A-pose's).

## Substances, reader stats, testo (2026-10-01, branch `wardrobe`)

- Owner: no male/female for anyone – an optional `testo` level 0–100 on characters/personas (no values set, no visual
  effect yet; `testoBand` is the hook) and a reader testo stat; then "other drugs too" → one substances model: catalog
  `stats` (testo, appetite: baseline 50, half-life 3 days), `substance` items with `effects` / `effect`,
  `POST /inventory/use/`, `POST /effects/end/`, migration `2026-10-01-reader-state`. Details + merge note: ITEMS.md
  "Substances".
- No gender field existed anywhere (types, seed, creator, people) – nothing to remove.
- `GET /me/ stats.<id>.updated_at` is "as of" (= now) on purpose: the first version sent the stored time with the already
  decayed level and the viewer decayed it a second time (55 instead of 60 in the check).
- `DELETE` is not in the reader API's CORS methods (only admin/embassy) → ending an effect is `POST /effects/end/`.
- A story pickup is once per place via a rate_limits ref (`grantref:<user>:<item>`, never pruned): `test-user --reset`
  now deletes those refs and the reader stats/effects, and gives back a starter camera a recipe consumed.
- The testo camera can only react to ImageNet's `lotion` / `sunscreen` / `pill bottle` (MobileNet) – the honest limit is
  in ITEMS.md; the multi-label "substance camera" is on hold (owner questions; the name "Check it" collides with checkit!).
- Check It camera (owner unblocked it 2026-10-01): multi-label `scan_object.map`, mandatory notice linking
  https://checkit.wien (verified as the official checkit! site of Suchthilfe Wien), DEV-only `__scutoonsFakePredictions`
  hook for the headless check. The global grant toast briefly covers the notice for ~3 s after a hit (toast position is
  global, left as is).

## Media quality setting + full-viewport video placement (2026-10-01, branch `media-quality`)

- **Owner:** media quality is a reader setting, not a responsive question: low / medium / high + Automatic, members on the
  server, guests on the device; video now (480p / 720p / 1080p), audio and slice images (1x / 2x / 3x) later on the same
  model; `SliceVideo.sources` (media queries) → quality variants; a "controls" flag; full-viewport placement (top /
  centre / bottom × fit width / fit height); the creator's slice tab "Bild" → "Hintergrund" / "Background". Spec:
  DESIGN.md "Media quality", "Video placement – full viewport", "Creator slice tab"; PLAN.md "RULE: one file per quality
  level"; DATA.md "Media quality".
- **No "full viewport" option existed** for slices or groups before (searched types, schema, reader, creator, all
  branches' commit messages): the only full-height placement was the VN `full` view (the centre group). Built new as
  `SliceVideo.viewport`.
- **Automatic, measured:** headless Chrome with a fresh profile reports `navigator.connection` `4g` / `downlink` ≈ 1.4–1.6
  Mbit/s even on localhost; CDP throttling (400 kbit/s, 400 ms) turns it into `3g` / 0.35–0.4 at once. So `downlink` only
  ever lowers (< 1 Mbit/s → low) and a 4G-class connection is measured: 512 KB of the low file (`Range: bytes=0-524287`,
  `cache: no-store`, ≤ 4 s, a reader cancels after 512 KB if the server ignores the range) → ≥ 7 Mbit/s high, ≥ 5
  medium, else low; a failed / CORS-blocked probe → low. The probe needs CORS on the CDN (`cdn/server.py` sends `*` +
  `Range`); production `cdn.scutoons.com` must answer `Access-Control-Allow-Origin` for it, else every Automatic reader
  without the API gets low (check after the deploy: DevTools → the `Fetch` request for `….480p.mp4`).
- **The media-quality module is imported with the chapter route**, so on an in-app navigation it is created (and reads
  `navigator.connection`) when the chapter chunk loads – the throttled case decided "low by the API alone".
- **Members:** `setting` waits (null) while a stored session token exists and `/me/` isn't loaded – the layout signs in
  after the page mounted, and a member's video first loaded the device's 1080p, then 480p (caught by the check).
- **Placement:** `.renderer--vertical` had `overflow-y: auto` (never scrolled – no height) which turned overflow-x into
  auto and clipped the full-viewport box at the 500 px column → `display: flow-root`. The width is the measured
  `documentElement.clientWidth` (100vw would include a desktop scrollbar and scroll sideways). Contain = the frame is the
  video's own box (`min(100%, 100svh × ratio)`) so the controls and "Ton an" sit on the video, not in a bar.
- **Data:** chapter 0 `src` 720p + variants; seed + the 2 chapter-0 statements of `content-seed.mysql.sql`. The worktree
  ran on a copy of the owner's store (`server/db/local.sqlite`, gitignored), migrated (`2026-10-01-media-quality`) and
  `import-content … --toon=catpope --only=chapter --update` (2 updated, 10 unchanged); the owner's own store is untouched
  until the merge (merge note). Production: `shared/content/seed/2026-10-01-media-quality-chapter0.mysql.sql` (tested on a
  copy of the store, SQLite dialect, twice: one published version per locale with variants, the old ones drafts).
- **Check:** `node scripts/check-media-quality.mjs [--viewer] [--api] [--creator] [--shots=<dir>] [--only=…]` – each level
  at 390 / 1440 → exactly its file (currentSrc + network), Automatic (API slow → low without a probe; 4G → measured; API
  hidden like Safari → measured, throttled → low), a switch at 30 s keeps the position, the account page as a guest
  (de 390 / en 1440, stored + kept after reload) and as the test user (server value wins over the device, `PUT /me/`,
  restored), placement 3 × 2 at 390 × 844 and 1440 × 900 (box = viewport, no sideways scroll, geometry, object-position,
  "Ton an" on the video; reduced motion = the poster placed the same), controls on / off (click toggles), the follow-up
  bar after a full-viewport player (fixed at the bottom), the creator's "Hintergrund" tab. 46/46. check-prolog now sets
  the guest's quality per run (375 low → 480p, 1280 medium → 720p, 1600 high → 1080p): 46/46.
- **Follow-up (owner, same day): four fits, frames, a readable picker.** `fit` = contain ("Ganz sichtbar") / width ("An
  Breite einpassen") / height ("An Höhe einpassen") / cover ("Füllen (beschnitten)") – the earlier two values were named
  "fit width/height" for contain/cover, which was misleading; width/height resolve to contain or cover per box from the
  two ratios (`viewportObjectFit`, unit-tested: a 16:9 video on a 390 × 844 phone = fit height → cropped sides, on a
  2560 × 1080 screen = fit width → cropped top/bottom). Nothing stored used `viewport` yet, so no data migration. Frames in
  the column: `SliceVideo.frame { ratio, fit, position }`, presets 5:8 / 6:13 (the s1c1 and s1c2 key visuals are both
  exactly 390 × 845 = 6:13, measured with Pillow) / 9:16 / 1:1 / custom clamped to 4:1 … 1:4; the creator writes the
  slice size from the ratio (`fillSliceSize` follows the frame), validation errors outside the limits and warns on a stale
  height or frame + viewport. The creator picker went through three rounds the same day: a 4 × 3 grid of tiny icons
  (unreadable for the owner) → picture tiles with sentences (too descriptive) → **basic**: one segmented row "Ganz ·
  Breite · Höhe · Füllen", "Oben · Mitte · Unten" only when the position matters, short ratio labels, one small preview,
  short size lines (`placement-picker.svelte`). The owner tried it in the preview and published chapter 0 with
  `viewport { center, height }` **in the worktree's DB copy only** (not in the seed, not in the owner's store); the
  regression checks then ran on a second fresh copy (API 18020 / viewer 18010 / creator 18030). Sizes in `em`: the creator's chrome is a fixed 13 px base while its
  `html` root is the reader's fluid 24 px, so `rem` came out ~1.8× too big. check-media-quality: 12 full-viewport
  combinations × 390 / 1440 × video / poster, 5 frames × 4 fits × 2 widths, the creator's ratio select + clamp + size
  lines. Preview for the owner ran from the worktree on 21310 (viewer) / 21340 (creator; 21300 was another agent's) /
  21320 (API) / 21330 (CDN).

## World map M0–M2 (2026-10-01, branch `map`, agents/MAP.md §8)

- One world map entity `map` (`scutoons/_/map.json`), migrated by `shared/content/scripts/migrate-map.ts` (one-time; refuses a
  second run). `/library/map?locale=&toon=` for the new reader; the bundle's old `locations` / `routes` / `toon.map` are
  computed from it for older builds. World game state `scutoons` (`/api/progress/scutoons/` merges `map` as sets on the
  server). Owner decisions: unlocks universe-wide, catpope's places unlock over time (screenplay → `unlock` points, 10 new
  places, `garden` = Garten Eden end of ch. 5), card on click, sea = the toon's colour (default `#00ff00`), bidirectional
  unlock (slice ↔ builder, stored once in the place's `unlock.slice` = the slice's image), no hand tool, start view set per
  toon in the builder, Wanted images → Map places derived from the map.
- Gotchas: a worktree agent runs its own stack on side ports (here API 19300 – workers 19500+ –, viewer 18300, creator 20300,
  admin 21300, shop 22300, qr 19301; 18000/19000/20000 were taken by another agent) with `.env.development.local` files
  (gitignored) and its own `server/config.local.php` + copies of `server/db/local.sqlite`, `server/storage/keys`,
  `server/storage/files/batch` and `qr/config.local.php` (all gitignored; without keys/batches admin-smoke and public-urls
  fail). `npm run dev:test-user` and `check:creator-cast` take `SCUTOONS_API` (+ `SCUTOONS_CREATOR`) for such a stack.
  `entities-smoke` must not run against a dev API with `CONTENT_SHOW_DRAFTS=1` (it expects drafts hidden) – it publishes test
  maps; it now deletes its own map versions. A Vite dev server's very first page load can miss the fonts (check-fonts flake
  on a cold admin; a rerun passes). Global creator/viewer button styles make text white: set `color` on new buttons.
- `platform` merged into `map` on 2026-10-01 (started by the coordinator with the owner's OK; conflicts resolved: both sides
  kept – covers / scripts / media requests + the world map); the merge note for `map` → `platform` is MAP.md §8.8.
- After the merge: the airport is `unlock: null` ("not unlockable yet", owner), the builder uses Feather `<Icon>`s, the map
  places are derived toon-media requests in Wanted images (`mapMediaRequests`, never stored). `npm install` after merging
  `platform` (feather-icons); a freshly (re)started Vite server can fail check-fonts once (dep re-optimisation) – rerun.

## Brutalist tools, phase 1 (2026-10-01, branch `brutal-ui`)

- Owner: tools use the reader's font sizes, "clear brutalism". Rules: DESIGN.md "Brutalist tools". Base:
  `shared/tokens/tools.css` (opt-in class `sc-tool` on the creator / admin / embassy root; in `index.css`, inert
  without the class), new tool tokens in tokens.css (`--control-height`, `--tool-bar-height`, `--tool-sidebar-width`,
  `--tool-panel-width`, `--type-title-*`, `--tracking-label`, focus), @scutoons/ui frames / hit targets / focus /
  inverted selection, DataTable `secondary` columns from 1600px. qr copy re-synced (`node qr/bin/sync-tokens.mjs`).
- Removed root overrides: admin `html { 16px }` + `.admin { 14px }`, creator `.creator { 13px }`, embassy 16px/15px.
  Done end to end: creator chapter editor + slice inspector (all four inspector tabs' px sizes), admin codes list
  (filters moved above the table, pager stays in the bar) + code detail. Feather icons on both screens.
- Gotchas: base.css's `h1–h6 { font-weight: 400 }` beats `:where()` rules → tools.css headings use `.sc-tool :where(h…)`.
  Creator-wide button rules are zero-specificity (`:global(:where(.creator button…))`) and `inline-flex` (centered
  vertically, content starts left: flex-row buttons that relied on `text-align` need `justify-content`).
  Measured (headless, test user): before creator 1440 = text 10–13px on a 24px root, 128 controls < 44px; after = only
  24px body / 18px caption, 0 controls < 44px; admin 14/12px → 24/18px.
- Phase 2 left: ~129 px font sizes in 39 creator/embassy files (other screens), see the phase-2 plan in the report.
- Owner review of phase 1 (2026-10-01): buttons = the viewer's (secondary `border-muted` → ink on hover, no hover
  inversion, rem padding; compact `space-05 space-15`, phones `space-05 space-1` in the ActionBar); badges FILLED from
  `--badge-<name>` / `--badge-<name>-text` in tokens.css (the owner's palette, ratios in DESIGN.md); bold sparingly
  (regular caps 0.08em `--tracking-label`, bold caps only for section headings 0.12em `--tracking-label-bold`);
  `--control-height` is rem-only now (2rem + frame = 32 · 36 · 52px, below the old 44px floor on phones).

## Brutalist tools, phase 2 (2026-10-01, branch `brutal-ui`)

- Every creator / admin / embassy screen on the type scale: 129 px font sizes swept (≤ 11px → caption or the tool label,
  12–18px → body, larger → title), glyph icons → Feather (`Icon`, icon-only buttons got `aria-label`), grey / tinted
  fills → state rules (hover = 2px frame, selected = inverted, warn / error / note bars = a thick state rule), filled
  status badges everywhere (`status-badges`, file-status, credits role tags, wanted-image notes = info). Legends are the
  plain tool label (an inverted legend plate clashed with components colouring their own legends).
- Touch minimum: one rule in tokens.css, `@media (pointer: coarse)` lifts `--control-height` to max(44px, …);
  `all: unset` buttons (credits handles, role chips, wanted links) need `display: inline-flex` + `min-height`.
- Checks: `npm run check:tools-css` (static, in `npm run check`), `npm run check:tools-ui` (pages at 1440 fine + 390
  touch; desktop pass first – once touch emulation was on, Chrome keeps `pointer: coarse` for the tab). Measured on 34
  screens × 2 widths: texts under caption 953 → 0 (comic lettering exempt), controls under 44px on touch 119 → 0.
  `creator-cast-check` reads chips case-insensitively now (status chips are uppercase) and takes SCUTOONS_API /
  SCUTOONS_CREATOR like the script check.
- The script page's textarea is 1rem / 1.4 (was 13/20 px; DESIGN.md updated, the scroll math reads the computed line
  height).

## Testo on the avatar, appetite food, worn items off the Items list (2026-10-01, branch `platform`)

- Owner: "the testo is part of the avatar" → `reader-stats.svelte` takes `include` / `exclude`; the testo bar is in
  `avatar.svelte` under the body picker (so also on the account page), the Items tab shows the other stats (appetite).
- Owner: worn items are not listed in the Items tab (the "Am Avatar" / "Im Inventar" sections and their messages are
  gone); a stackable entry shows qty minus the one worn piece. Items tiles carry `data-item`, the list
  `data-testid="inventory-items"`.
- Appetite: new `weed` (substance, appetite +25, no reader effect – kept simple), new category `food` (`apply` +
  negative appetite delta, button "Essen"/"Eat", toast "Gegessen: …"): `cucumber` −5 (still a recipe ingredient),
  new `tuna_can` −30. Catalog 20 → 22 items (api-smoke, admin-smoke). Icons: `make_substance_icons.py` now takes icon
  names (`--root cdn/root weed tuna_can`). The coordinator's cucumber camera labels (+ cuke, zucchini, courgette) went
  in with the same commit.
- Gotchas: the dev DB's sandbox chapter 2 was older than the seed (no testo pickup → check:testo failed at step 1);
  fixed with `php server/bin/console import-content ../shared/content/seed --toon=sandbox --update` (4 entities: ch. 2
  de/en). check:wardrobe's "guest" runs after a member run were really the member: the viewer moves the token into the
  API's SSO cookie, so `localStorage.clear()` is not a sign-out – the check now clears the browser cookies
  (`Network.clearBrowserCookies`) before each guest run (member 1440 used to start in the swim pants).

## Certificate verification fixed (2026-10-02, branch `platform`)

- Owner: the verify page "just checked if the signature is a proper signature, not if it also matches an entry in the
  db". Confirmed: `qr/public/verify.html` (and the admin badge, `admin/src/lib/qr/crypto.ts`) verified against the
  public key embedded in the same JSON → a self-signed file passed. Fixed: `GET /api/certificates/{code}` +
  `POST /api/certificates/verify` verify server-side against the published key AND the stored `qr.signature`
  (agents/ITEMS.md "Certificates" has the contract, the reasons and the owner's note on what a certificate is).
- PHP had no ML-DSA (no Composer lib, OpenSSL 3.0, no SHAKE in `hash()`): `server/src/Infrastructure/Signing/MlDsa65.php`
  + `Shake.php` are a plain-PHP FIPS 204 verifier (~50 ms). noble's `ml_dsa65.sign(msg, sk)` = external ML-DSA with an
  empty context (signs `0x00 0x00 ‖ msg`); all 421 stored dev-DB signatures verify, a 1-byte change fails.
- Gotchas: a pasted URL can't go in the path segment (`%2F` → 404) → `?code=`; PHP hex literals ≥ 2^63 are floats
  (Keccak round constants are built by the LFSR instead). PHPStan reports 3 errors in `InventoryService.php` (not from
  this change, they come from commit 1edde25).

## Email module (2026-10-02, branch `platform`)

- Owner: "a place to manage emails … refactor across apps". Inventory: only the API sent mail (the two sign-in mails of
  `/auth/request/`); qr, the frontends, embassy (logs activities + `mailto:` only), scripts and `generate/` send none.
- Module `server/src/Domain/Mail/` (`MailTemplates` registry, `MailRenderer`, `Mailer::send($id, $to, $locale, $vars)` /
  `sendToAdmins`, `MailLog`, `MailRecipients`, `MailFailed`); texts `server/templates/mail/texts.php` (locale → id → key),
  one `layout.html.php`; delivery `Infrastructure\Mail\Transport` (ex `Mailer`). `LoginMail`, `login.html.php`,
  `no-account.html.php` deleted. Sign-in mails byte-identical (scratch compare of 8 variants), `logs/mail.log` format
  unchanged. Migration `2026-10-02-mail-log` (applied to the local dev DB). Admin page `/emails` + `/emails/templates`.
  How to add the ownership emails: agents/PLATFORM.md "Emails".
- Gotcha: the first `composer test` run had dozens of FAILs (401s, OTP not found) – another agent's contract run at the
  same time (MEMORY "two contract runs"); the rerun was green.
- Browser pane: with `resize_window` 1440×900 emulation, clicks by ref landed beside the target (scaled pane); typing
  into a textarea did nothing – `form_input` + clicks after resetting to the desktop preset worked.

## Command palette in creator + embassy (2026-10-02, branch `platform`)

- Owner: the admin's "search or jump to" palette also in the creator and the embassy. Shared in @scutoons/ui:
  `PaletteTrigger` (`label`, `shortcut`, `compact`, `onclick`; `data-testid="palette-trigger"`) and
  `palette-commands.ts` (`linkCommands`, `languageCommand`, `signOutCommand`, `otherLocale`); the admin uses them
  (same look). Texts: `palette.*` in each app's messages. PLATFORM.md P2a "Decisions" has what each palette lists.
- The full label (14rem+, ~430px at the 24px root) does not fit the creator bar (it pushed the breadcrumb out at
  800px) nor the embassy bar → both use `compact` (icon + `Ctrl K`). Collapsed trigger is `--control-height` wide (44px
  touch).
- Lists load on the first palette open (creator: `contentIndex` per toon; embassy: `orgs({ sort: 'name', limit: 500
  })`); a query typed before the response arrives only shows the static entries until it lands.
- Gotchas: the Browser pane's key name `Return` does not fire `Enter` (use `Enter`); a synthetic `keydown` dispatched
  on `window` has no `closest` → `CommandPalette`'s hotkey handler now checks `instanceof Element`. Pre-existing, not
  fixed: the embassy top bar overflows between 768 and ~1280px (brand + nav + account), the embassy Today page logs
  `Cannot read properties of undefined (reading 'user')` (`routes/+page.svelte` team filter).

## Ownership, claims, gifting (2026-10-02, branch `platform`)

- Owner: QR codes unique (one code = one owner, reading with someone else's code stays allowed), certificates signed to
  the buyer by hand are the proof, a stolen registration is fixed by a claim + an admin transfer (evidence optional),
  readers can gift items. Built end to end: agents/ITEMS.md "Ownership, claims, gifting" (decisions, data, API, rules,
  mails, registration strategy); server docs/API.md "Ownership, claims, gifting".
- Decisions taken: gifts only to existing accounts (404 `recipient_not_found`, no pending gifts – signup needs a code
  anyway); `item_transfers` is its own table (code_events stays anonymous statistics); a reader without an account can
  sign up to file a claim (`/auth/request/ dispute`, nothing claimed) – otherwise the typical victim (never registered,
  so no account) could not claim at all; the admin note of a rejection stays internal.
- Gotchas: the dev API runs the working tree – new tables broke `GET /me/` for everyone until the migration ran
  (coordinator); read paths now tolerate the missing tables (`OwnershipTransfers::missingTable`), writes answer 503
  `not_available`. Always back up `server/db/local.sqlite` and `php bin/console migrate` right after adding a migration.
  The Bash tool's heredocs broke on some quote mixes (Python patch scripts in the scratchpad worked). Python on Windows
  writes CRLF unless `newline=''` – git normalizes, but use `newline` to avoid noise. The shared Browser pane is used by
  other agents too: open your own tab (`tabs_create`). `/auth/logout/` from a test tab ends the shared test-user cookie
  session.

## Creator content language "i18n" + translate mode, Korean (2026-10-02, branch `platform`)

- Owner: an easier way than every language side by side; translators switch back and forth; more languages coming.
  Top-bar control (globe + "ALL"/"KO", native select invisibly over a compact frame) → `textLanguage` in
  `creator/src/lib/editor/text-language.svelte.ts` (localStorage `scutoons_creator_text_language`, default all). The two
  shared field components follow it: `locale-text.svelte` (per-locale files) and `credits/locale-map-field.svelte`
  (`{ <locale>: text }` values); translate mode = `text-original.svelte` under the field. Rules: DESIGN.md "Creator:
  content language (i18n) + translate mode". Owner rounds the same day: label "i18n" (not "Texts"), the original as a
  muted caption line (no boxed panel), the control as compact as the palette trigger, no account / UI-language cluster in
  the header (sign out + UI language stay in Ctrl K), chapter lists = a plain list (no table header, one title in the
  chosen language, other languages in the tooltip) with word badges only when something is up (`chapter-badges.svelte`).
- N languages: the editors already took their locales from the toon (`toonLanguages`); what assumed two: the toon
  page's season subtitle (languages ran together without a separator), the chapter tables (one column per language),
  the cover preview, the "language:" hint lines. `Language.KR = "kr"` (unused) → `KO = "ko"` (schemas regenerated);
  the server and the library fallbacks were already generic. Paraglide UI languages unchanged (de/en).
- Sandbox `ko`: `make-sandbox.ts` writes `ko/` = the `de` files + Korean toon / season / chapter titles and descriptions
  (`PARTIAL`); `toons.json` sandbox `languages: ["de","en","ko"]` + `i18n.ko`; imported with `import-content … --toon=sandbox
  --update` (DB backup `server/db/local.sqlite.bak-2026-10-02-textlang`). The sandbox isn't in content-seed.mysql.sql.
  The viewer asks the API in de/en only, so `ko` is creator-only; a de/en toon asked in `ko` falls back (`X-Content-Fallback`).
- Gotchas: a Svelte 5 `<select value={x}>` with `{#each}` options rendered "KO" (the last option) before the value
  applied → `selected={…}` on the options. Python on Windows writes CRLF in text mode → `newline='\n'`. The Browser pane
  was shared with the owner / other agents (the stored choice and the tab changed under me) – check state before
  asserting. `check:creator-cast` fails 4 chip-word assertions since the status chips show only the language (commit
  9847809, words in the tooltip) – the check needs updating, not the chips. Chapter-editor undo on the Meta section works
  on the active group's doc, not the chapter (pre-existing): "Copy original" on a chapter title can't be undone there.

## Bold only very rarely in the tools (2026-10-02, branch `platform`)

- Owner (2026-10-02, a standing rule that had been ignored): "only use bold very rarely". Rule: DESIGN.md "Brutalist
  tools" → "Bold only very rarely": regular everywhere; bold only for the page's h1 and `<strong class="sc-emphasis">`
  (rare whole-phrase emphasis). Hierarchy = size, uppercase + `--tracking-label`, spacing, ink / muted.
- Done across creator, admin, embassy, `@scutoons/ui`, `@scutoons/auth`, tools.css: every bold `font-weight` → regular
  (with it `--tracking-label-bold` → `--tracking-label` on the panel / section / sheet / split-view / info headings), every
  `<b>` / `<strong>` → `<span>` or plain text (help + style guide), h2–h6 / `.sc-heading` regular in tools.css, `optgroup`
  regular. Kept bold: tools.css `h1`, AuthGate's h1, the creator's stale-autosave warning (`sc-emphasis`, chapter editor +
  MetaPage), `signed-in-as` `.email` (the reader's account page; tools use `.compact` = regular), the map canvas handle
  glyph (`font: 700 9px` shorthand, a canvas marker; the check doesn't parse `font:` shorthands).
- Guard: `check:tools-css` now also fails on a bold `font-weight`, `<b>`, `<strong>` without `sc-emphasis` in the tools
  (`bold-ok` + reason exempts a line). Changing a token in tokens.css needs `node qr/bin/sync-tokens.mjs` (the tokens
  check compares qr's copy).

## Players, albums, related items (2026-10-02, branch `platform`)

- Owner: a record plays only with a record player (also tape / CD player); music belongs to an album; `related_items` on
  season + chapter (toon page upsell). Built: agents/ITEMS.md "Players, albums, related items" (model, access flags,
  reader / creator / admin, seed, tests, open decisions). Albums are in the item catalog (`items.json` `albums`), not the
  content store. Catalog 22 → 25 items (api-smoke, admin-smoke).
- Owner decisions the same day: the record player comes from the story at the end of season 1 (today: chapter 2's last
  slice, chapters 3–5 have none – move it to chapter 5 later); related items catpope s1 = the vinyl only; the free music
  download stays, `album.access` { download: members, listen: owners } (per album).
- Gotchas: several `intersect` slice events with the same trigger fire in no fixed order (two `addChoice` came out
  swapped) – the pickup has one choice only. The VN group of chapter 2 is chosen with `?label=scene-20`. Claiming a code
  as the test user (`?code=`) writes `qr.email` too, and `npm run dev:test-user -- --reset` then fails (`UPDATE qr SET
  claimed_at = NULL` – the column is NOT NULL in the dev DB): restore the code row from a backup before resetting (done
  for `Qewa1jrr`). The Browser pane's 390 px emulation put the toon inventory off-screen for ref clicks; the desktop preset
  + `?inventory=open` worked.

## Record player repair (2026-10-02, branch `platform`)

- Owner: a BROKEN record player is found in chapter 2's first scene, a Reparatur-Gutschein (repair coupon, "issued by
  the city of Vienna") at the end of chapter 2 (later: the end of season 1); a recipe of the two makes the record player,
  the broken one is consumed. Built as data only (owner-approved exception to the packs rule, replaces the direct grant
  from earlier the same day): agents/ITEMS.md "Record player repair". Catalog 25 → 27 items, 5 recipes
  (`record_player_repair`); `record_player` is no longer story-grantable.
- Interpretation to confirm: the coupon is consumed too (every recipe input is consumed). Wording de/en in ITEMS.md.
- The first scene of chapter 2 is the scene file `chapter2/scene-14.json` (its own entity `scene:catpope:2:scene-14:<l>`),
  so the seed change is 4 statements (chapter 2 + scene-14, de/en) and the local import needs `--only=chapter,scene`.
- Gotchas: guests cannot read chapter 2 locally without a code (draft) – the browser check ran as the test user only
  (headless CDP script like check-testo, from the scratchpad); a guest's crafted record player is split back into the
  broken player + coupon at sign-in (`importGuest`), both story-grantable, so nothing is lost.

## One control height (2026-10-02, branch `platform`)

- Owner (screenshot of the creator's Languages section: select, chip, select and "+ Add" all different heights):
  "buttons, selects and input fields have varying heights. A general rule should apply". Rule: DESIGN.md "One control
  height" – every single-line control in every app (tools, viewer, shop, auth) is exactly `--control-height`
  (2rem + 2 × 2px; max(44px, …) on a coarse pointer), border-box, no vertical padding, line-height 1, centred. No small
  variant (`Button size="sm"` = less side padding only). Interactive chips: `.sc-chip-control`; custom framed controls /
  link-buttons: `.sc-control` (both global in tools.css). Status badges (`.sc-chip`) are not controls.
- Done: tools.css (fields `height`, buttons `padding-block: 0`), @scutoons/ui (Button md/sm, IconButton, Tabs, Menu
  trigger + entries, ActionBar Info / More, SearchField, Field, CommandPalette input, RecordInspector switch), creator
  (layout buttons / ghost, Languages chips, credits role chips, featuring / related-items chips, pickers, map tools,
  tabs, mini buttons, chapter-tree add buttons one line + ellipsis), admin filter chips, shop variant chips + global
  button, viewer (static/css/styles.css button / .btn / fields; the top-right round toggles are `--control-height` and
  placed `0.5rem + n × (--control-height + 0.5rem)` from the right; menu language / close / link-buttons, hero start /
  trailer / sound, next bar, item / claim / recipe / inventory / map / camera buttons, home buy / all), the reader's
  video "sound on".
- Guard: `check:tools-css` (static: height / min-height / vertical padding on control selectors without the token;
  `control-ok` exempts a CSS line) and `check:tools-ui` (measures every single-line control on the tool pages + 7 viewer
  pages at 1440 / 390 touch, ±1px; `data-control-exempt="<reason>"` in markup exempts cards, tiles, markers, VN choices).
  `--browser-only` runs the browser part even when the static part fails.
- Gotcha: heredocs sent through the agent Bash tool here turned `\` into `\` (even quoted 'EOF') – write scripts with a file tool,
  not a heredoc, when they contain backslashes.

## Embassy stores + "Where to buy" (2026-10-02, branch `stores`)

- Owner: an Embassy type STORE, every store carrying SCUTOONS products listed on a public viewer page, "encourage small
  vendors at first". Built: agents/EMBASSY.md "Stores" (model, API, UI, tests, open decisions). `store` + `record_shop`
  are stockists; store columns on `embassy_orgs` (migration `2026-10-02-embassy-stores`); public `GET /api/stockists/`
  (`Domain\Embassy\Stockists`, public fields only, independent first); viewer `/stores`, "Läden vor Ort" next to
  "Kaufen" (related row, landing editions), menu + footer link only when a store is listed.
- Local fake data: `php server/bin/console sample-stores [--remove]` (SQLite only). `npm run check:stores` needs a listed
  store (exit 2 otherwise).
- Gotchas: in `related-items.svelte` `.btn { flex: none }` comes after `.actions > *`, so the buttons never grew – a third
  button ran out of the 390 px column until `.actions` got `flex-wrap: wrap`. A worktree without `server/storage/keys` +
  `storage/files` (gitignored) fails the certificate / batch contract checks – copy them from the main checkout. A
  products map that may be empty must be encoded as an object (`(object)`), PHP's `[]` would be a JSON list. Bash heredocs
  with mixed quotes broke again: write generator scripts as files.


## Close cross (2026-10-02, branch `platform`)

- Owner (screenshot of the map card "Kloster"): the close × as high as the headline, square, 2px like the box borders
  (or 1px like Plex Mono's strokes). Rule: DESIGN.md "Close cross". One drawn icon `CloseIcon` + the control
  `CloseButton` in `@scutoons/ui` (`shared/ui/src/close-icon.svelte`, `close-button.svelte`); the viewer imports them
  from `@scutoons/ui` as well (it already took `Field` from there, through `@scutoons/auth`'s dependency).
- Geometry: the svg box = 0.698 × `size` (IBM Plex Mono cap height), lines corner to corner, `vector-effect:
  non-scaling-stroke` keeps 2 / 1 CSS px. Measured: 16.75 px at the 24 px root next to a 24 px title, 9.77 px at 14 px –
  centre on the title's centre (Sheet, InfoPanel, map card).
- Replaced: @scutoons/ui Sheet, InfoPanel, SplitView, ToastStack (CloseButton); every creator Feather `x` (pickers,
  history / problems panels, notices, list / chip / row removes, 25 files) and the two "clear testo" ✕; admin filter
  chips (claims, emails, events) and the homepage featured remove; viewer map close + place card, menu, claim sheet,
  audio player, camera HUD (`stroke={1}`). The command palette, embassy, shop and auth had no cross of their own (they
  use Sheet / SplitView / toasts).
- Guard: `check:tools-css` flags × ✕ ✖ `&times;` as a button / link content and `<Icon name="x">` in every app's markup
  (`close-ok` exempts a line).
