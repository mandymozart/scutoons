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
