# PLAN — Chapter 2 "Anger" (branch `anger`)

Release target: **2026-10-04** (today when written: 2026-09-28 → 6 days).
Companion files: [MEMORY.md](MEMORY.md) (facts, gotchas, findings), [DESIGN.md](DESIGN.md) (design system + text-box spec).

## Goals

1. **Figma → CDN pipeline**: export chapter slices from Figma straight into `cdn/root/catpope/images/…`,
   repeatable, no manual "export → rename → copy".
2. **Generated chapter config**: build `viewer/static/api/catpope/{de,en}/chapter2.json` from the Figma file,
   with every speech bubble / caption coming from Figma TEXT layers → rendered as live, translatable text
   on top of text-free slice art (the `Slice.dialog` "text box on slice" feature).
3. **Ship chapter 2** (status `draft` → `published`, release date, thumbnail, key visual).
4. *(Second track, priority to be confirmed)* **Physical merch in the user inventory**: items bought on
   Bandcamp carry a QR code (qr.scutoons.com); scanning/claiming registers the item to the user's account
   and it shows up in the viewer inventory. Clean up the duplicate product data.
5. *(Later)* Retrofit chapter 1 the same way (text is baked into its 94 slices today).

## Decisions (2026-09-29, user)

- Figma exporter (Track A) **postponed** — the manual Figma → CDN workflow is fine for now. User keeps working on the
  chapter-2 layout in Figma.
- Slice size stays **390×625** (1x) / 1170×1875 (3x).
- Text box on slice: build the renderer and **try it on chapter 1 "Denial"** by extracting the text baked into its
  slices (OCR/vision) → live, translatable overlays (masking the baked German text).
- Track D is **in scope now**, redefined in [ITEMS.md](ITEMS.md): items, skills, combining (camera + cucumber →
  cucumber camera with real-world cucumber detection), avatar slots, product cut-out icons, email login ported from
  osct, n codes per account, codes always contain items, progress stored per account.
- Work autonomously, run in parallel, few questions. Commit in small steps so the user can rewind.
- qr.scutoons.com is a separate git repo (`C:\Development\qr.scutoons.com`) — work on its local branch `anger`,
  **never push** (its workflow deploys production on push to any branch).

---

## Track A — Figma export pipeline (`cdn/figma_export.py`)

### Figma file convention (to agree on with the artist)

```
Page "S1C2 Anger"
├── Frame  "flow"                 390 × N    long reading-flow master (reference only, not exported)
├── Frame  "s1c2-slice-01"        390 × 875  one mobile slice (height may vary per slice)
│   ├── Frame "art"               390 × 875  everything that is image: panels, bubble shapes, SFX art
│   └── TEXT layers               …          every translatable line (bubble text, captions, narration)
├── Frame  "s1c2-slice-02"
└── …
```

- Slice frames are matched by name `^s1c2-slice-(\d+)$`; order = the number.
- The `art` child is exported as the image → **no text is ever baked in**.
- Every TEXT node directly in the slice frame (or in a group named `text`) becomes one `dialog` entry.
  Optional layer-name hints: `#caption`, `#narration`, `#bubble`, `#sfx` → `variant`.
- Bubble *shapes* stay in the art (keeps the artist's hand-drawn look); only the words are live text.
- Scene/group split: optional Figma **Sections** named like `scene-14` around slices → config `groups`
  (`label` = section name). No sections → one vertical group.

### Script behaviour

1. `GET /v1/files/:key/nodes?ids=<page>` (token from `cdn/.env` `FIGMA_TOKEN`, file key `FIGMA_FILE_KEY`).
2. Collect slice frames, their `art` child ids and TEXT nodes (characters, absoluteBoundingBox, style:
   fontFamily, fontSize, fontWeight, textAlignHorizontal, fills, textCase).
3. `GET /v1/images/:key?ids=<art ids>&format=jpg&scale=1` and `scale=3` → download to
   `cdn/root/catpope/images/1x/s1c2-slice-NN.jpg` and `…/3x/s1c2-slice-NN.jpg`
   (language-neutral paths — no `/de/` since the art has no text).
4. Convert text boxes to % of the slice frame (see DESIGN.md → Text box), font size to `cqw`.
5. Write/merge:
   - (superseded 2026-09-30: no separate i18n file — text stays inline in the per-locale chapter configs)
     new/changed German source text marks `en` as stale (`"en?": "old text"`), never silently overwritten.
   - `viewer/static/api/catpope/{de,en}/chapter2.json` — **merge**, keyed by slice image: the generator owns
     `image` + `dialog`, all hand-authored fields (`audio`, `events`, `onomatopoeia`, group `renderer`,
     VN `ref` scene files) are preserved.
6. `--dry-run` prints a diff/report (new/removed slices, missing translations) without writing.
7. Upload stays `python sync.py` (existing FTP sync, option 3 → catpope).

Alternatives kept on file: extend `C:\Development\graphic-novel-figma-plugin` with an "Export chapter"
command (hide text → `exportAsync` → collect TEXT) if the art/text layer split can't be kept in the file;
the Figma MCP connector for Claude to inspect the file while authoring (not needed for the pipeline itself).

## Track B — Text-box renderer in the viewer

The `Dialog` type and one usage in chapter 2 exist, **nothing renders them** (see MEMORY.md).

- Extend `Dialog` in `viewer/src/lib/models/content.ts` (schema in DESIGN.md → Text box).
- New `viewer/src/lib/components/toon/dialog/dialogs.svelte` + `dialog.svelte`; mount in
  `toon/slice.svelte` above the `<img>`, below `Triggers`.
- `.slice { container-type: inline-size }` so font sizes scale with the slice width (`cqw`).
- Text is real DOM text (selectable, screen-reader readable); image `alt` stays decorative.
- Fonts: whatever comic/lettering font the Figma bubbles use must be self-hosted in `viewer/static/fonts`
  (licence check) — open question.
- Fix the existing chapter-2 "scene-20" entry to the new schema.

## Track C — Chapter 2 content & release

- Run exporter → review in dev (`python cdn/server.py` + `npm run dev`) at 375/390/500 px widths.
- Translate `i18n/s1c2.json` → `en` (German is source of truth, like the screenplay).
- Chapter meta: `release: 2026-10-04`, `status: published` in `chapter2.json` (both locales) and
  `season1.json` `sequence`; create missing `images/s1c2-thumbnail.jpg`, fix `keyvisual` to object form.
- Decide what happens to the VN scene files `chapter2/scene-14..19.json` (placeholder cucumber choice,
  18/19 duplicates).
- Fix homepage "next release" block (`routes/+page.svelte:19-31`, typo `2.&Auml;ger` → `2.Ärger`).
- `python sync.py` → CDN; merge `anger` → `main` → GitHub Action deploys viewer.

## Track D — Physical merch → inventory (priority TBC)

Today: one QR per physical item already exists (qr.scutoons.com batch generator, `edition_id` per code),
but a "claim" is just an email written on the code row, there are no accounts, the viewer keeps **one**
membership code in localStorage, and inventory only shows VN collectables. No "cucumber scan" exists anywhere.

- **D0 – qr.scutoons.com security (do first, independent)**: private key served by `/api/keys/keys.php`
  behind a Referer check; `/api/validate` returns the claimant's email with CORS `*`; unescaped output on
  `index.php:139`; deploy to production on every branch push; path traversal in `generate/files.php`.
- **D1 – one catalog**: merge viewer `static/api/products.json` (5 colour entries, prices, images) with qr
  `api/editions/editions.json` (3 editions, variants, counts) into one product model keyed by `edition_id`
  with colour/size variants; add the Bandcamp items missing from both (LE vinyl, test presses 1/2 and 2/2,
  Scorpion Amulet, Love For Women – Kangaroo Cat Hoodie). Fix edition-size contradictions (4/12 vs 12/25 vs 37).
- **D2 – item API**: `GET /api/item/?code=` → public fields only (`edition_id`, `edition_number`, `variant`,
  `claimed`), no email. Add `edition_number` (+ `variant`) columns; backfill from batch manifests; dedupe the
  doubled dago (74 vs 37) and beetles (40 vs 20) codes.
- **D3 – accounts**: magic-link email login on qr.scutoons.com (`users` table, signed session token, CORS for
  scutoons.com); claiming links code → user; `GET /api/me/items`. Viewer `/account` login + "My items".
- **D4 – inventory UI**: viewer inventory gets a "Items" tab (physical, from account) next to collectables;
  multiple codes per user; membership = any valid membership item.
- **D5 – QR landing page update** (`qr.scutoons.com/index.php`): show the item (image, name, `n / total`,
  variant), "Add to my inventory" → login/magic link → redirect to `scutoons.com/catpope?code=…&inventory=1`.

## Track Creator — editor at scutoons.com/creator

Web tool to make webtoons "SCUTOONS style" on the **existing** config structure (text inline per locale, no
translation files). Details: MEMORY.md → "Creator editor"; API: qr `docs/ACCOUNTS.md` → "Content API".

**Phase 1 — done (2026-09-30)**
- Content store in qr.scutoons.com (`content_versions`, drafts/published/history/restore, role `creator`), import
  script, reader loads store → static fallback, `?preview=draft` for creators.
- Editor: toon → season → chapters (new chapter, season status), meta per locale, groups + scene files, slices
  (thumbnails, drag reorder, image picker with CDN preview + 390×625 / 1170×1875 hint), WYSIWYG text boxes (draw / move /
  resize, properties, de/en side by side, overrides, 130 % warning, grid snap, nudge, 1x/3x), audio (tracks, sample
  library, per-slice volumes, samples with drawn trigger areas), VN forms + raw JSON + onomatopoeia preview, validation,
  undo/redo, autosave, unsaved guard, save draft / publish / preview, style guide, version history.

**Phase 2 — open**
1. Image upload to the CDN (separate FTP host; today `python cdn/sync.py`) – upload from the image picker.
2. Figma import (Track A: art frames → images, TEXT layers → `dialog` boxes, merge into the store instead of files).
3. Production rollout: run `2026-09-30-content-versions.mysql.sql`, seed with `import-content.php` (needs PHP + DB
   access or a phpMyAdmin import), grant `creator`, then treat the static files as seed/fallback only.
4. Nice to have: publish per locale / per file from the top bar, diff view between versions, locking or live presence
   for two editors, creating/renaming VN scene files for existing groups, slice-level `onomatopoeia` (typed, not rendered).

## Parallelisation (running 2026-09-29)

| Agent | Scope (files it owns) |
|---|---|
| B – textbox | `viewer/src/lib/models/content.ts` (Dialog), `viewer/src/lib/components/toon/dialog/*`, `toon/slice.svelte`, `viewer/static/fonts`, `static/css/fonts.css`, ch2 scene-20 fix |
| B2 – ch1 text | `cdn/tools/*` (bubble detection), `viewer/static/api/catpope/i18n/s1c1.json`, `dialog` in `catpope/{de,en}/chapter1.json` |
| D-backend | everything in `C:\Development\qr.scutoons.com` (branch `anger`): security, accounts, items API, landing page |
| D-viewer | viewer account/login, inventory, camera, avatar, `config.ts`, `package.json`, `routes/+layout.svelte`, `lib/api`, `lib/contexts` |
| D-catalog | cut-out icons in `cdn/root/catpope/items/`, viewer `static/api/products.json` dedupe + shop/homepage |

Agents don't commit in the scutoons repo (shared working tree); the coordinator commits per track. Track A postponed.

## Open questions (mostly answered 2026-09-29 — kept for history)

1. **Priority**: chapter 2 only until 2026-10-04, merch after? Or merch must ship with it?
2. **Figma access**: file URL + a personal access token in `cdn/.env` (`FIGMA_TOKEN`, you add it yourself),
   and/or connect the Figma connector for Claude in claude.ai settings.
3. **Slice size**: existing chapter-2 slices are **390×625** (1x) / 1170×1875 (3x), not 390×875. Is 875 the new
   height for the redesigned chapter? (Renderer doesn't care; exporter supports any height.)
4. Can the Figma file keep the `art` frame / TEXT layers split per slice? Which font(s) do the bubbles use?
5. What is the **"cucumber scan"**? Nothing by that name exists (cucumber is only a VN collectable). Should
   scanning a QR on a physical item also grant a collectable like the cucumber?
6. Accounts: is magic-link email login OK, or keep it "email written on the code" + device-local wallet?
7. Do you know which variant (colour/size) each printed QR sticker went on? Otherwise the user picks it at claim.
8. Keep the VN placeholder scenes 14–19 in chapter 2 or is the Figma flow the whole chapter?

## Log

- 2026-09-28 — investigation done (viewer, cdn, qr.scutoons.com, graphic-novel-figma-plugin, cat-pope-agent,
  Bandcamp merch page, earlier design-system session). Branch `anger` created. Docs written. No code changed yet.
- 2026-09-29 — user decisions (above); ITEMS.md concept + API contract written; 5 parallel agents started.
- 2026-09-29 — landed on `anger` (scutoons): text-box renderer (b6aa594, 3cb7d19), chapter 1 text extraction
  70 dialogs / 53 slices de+en (1f339a4, b163051), shop catalog + item icons (ae694b4), accounts/inventory/camera/
  avatar viewer (8f7a62f). qr.scutoons.com local branch `anger` (not pushed): security fixes, accounts + items API,
  landing page, smoke test 56/56. `npm run check` 0 errors, `npm run build` ok.

## Next

1. User review in the browser (see "Run locally" in MEMORY.md / below) and of the chapter-1 translations.
2. Chapter 2 content from the user's Figma layout: slices 390×625 → `images/{1x,3x}/s1c2-slice-NN.jpg` (text-free),
   dialogs authored per DESIGN.md Part 2 (manual or via `cdn/tools` scripts), release meta for 2026-10-04.
3. Production rollout of qr.scutoons.com (docs/ACCOUNTS.md there): DB migration, backfill, secrets (AUTH_SECRET, SMTP),
   decide which duplicate dago/beetles code set was printed, then merge qr `anger` → main.
4. `python cdn/sync.py` for new icons (`catpope/items/`) and `merch/bandcamp/`.
5. **Server-side content gate (security, after release unless the user wants it earlier):** the paywall is
   client-side today (device access in localStorage, chapter JSON shipped static with the viewer build, slice images
   on the public CDN with predictable names). Plan: chapter configs served by a PHP gate that verifies the signed
   `access_token` from `/access/`·`/me/`; slice images via signed/expiring URLs or a token-checked rewrite on the CDN.
   Ownership (claims) is already server-enforced (API + DB triggers).
6. Minigames (registry has a placeholder only), avatar art, flat-lay product photos for cleaner icons.

## Run locally

- CDN: `python cdn/server.py 8001` (port 8000 is used by nadeos on this machine)
- QR API: `cd C:\Development\qr.scutoons.com && php -S localhost:9000 router.php` (login codes in `logs/mail.log`)
- Viewer: `cd viewer && npm run dev` (dev env in `viewer/.env.development`: VITE_CDN_URL, VITE_QR_API)
- 2026-09-30 — access model final: codes (membership/promo via permissions map) unlock reading on the device without
  login; accounts only for owning codes (one owner, API + DB triggers + race test); revoke per code; code_events
  activity log + admin Stats view. qr `anger` smoke 108/108. Viewer 5b4dd60.
- 2026-09-30 — Track Creator phase 1: content store (qr 660117b, 1475223, 7d9b917, b95cfb6, badc1eb, 0332912), `/creator` editor, reader loads
  the store first. Browser walkthrough on chapter 1 (edit de+en → draft → preview → publish → restored). Phase 2 open.

## RULE: printed / public URLs never change (2026-09-30, owner)

Physical items in the real world carry these links (QR stickers, certificates, prints). Until a deliberate, tested
migration via the server's code mappings exists, **these URLs, paths and parameter names must keep working exactly as
they are** — in every refactor, on every host/docroot change, in both old and new server layouts:

| Where | URL / parameter | Used by |
|---|---|---|
| qr.scutoons.com | `https://qr.scutoons.com?c=<CODE>` and `https://qr.scutoons.com/?c=<CODE>` (param **`c`**) | every printed QR code (`QR_BASE_URL = QR_DOMAIN + '?c='`) |
| qr.scutoons.com | `/?c=<CODE>&skip=1` (param **`skip`**) | legacy "continue without claiming" links |
| qr.scutoons.com | `/verify.html` (+ its code input / query) | printed certificates: "Verify at https://qr.scutoons.com/verify.html with code: <CODE>" — the code is printed in **UPPER CASE** |
| qr.scutoons.com | `/api/validate/` (GET `?code=` / `?c=`, POST `{code}` / `{c}`) and its response shape | old viewer builds, external links |
| qr.scutoons.com | `/api/keys/keys.php` (public key) | verify page, third-party signature checks |
| scutoons.com / www.scutoons.com | `/<toonId>` e.g. `/catpope` (toon entry + chapter overview) | `redirect_to` of all codes (`https://scutoons.com/catpope`, `https://www.scutoons.com/catpope`) |
| scutoons.com | `/<toonId>/chapter/<n>` (+ `?label=`) | chapter links |
| scutoons.com | `/<toonId>/credits`, `/<toonId>/download` | links in prints / socials |
| scutoons.com | query param **`code`** on any viewer page (`?code=<CODE>`), plus `&inventory=open` | redirects from the QR landing page |
| scutoons.com | `/de/...`, `/en/...` locale prefixes | shared links |

- New URLs may be **added**; existing ones may only change through server-side code mappings (redirects per code),
  with a migration plan, tests against real printed codes and a rollback. Not now.
- Refactors must include a test that hits every URL above (status + target) before and after.
- Known issue to handle carefully (not changed yet): certificates print the code in upper case, but codes are
  case-sensitive nanoids — verify must accept the printed form (exact match first, then a case-insensitive match only
  when it is unique). Test with real certificate codes before changing anything.

## RULE: every chapter belongs to a season (2026-09-30, owner)

A chapter exists only as an entry in a season's `sequence` (per locale) plus its chapter file. No chapter without a
season: the creator creates season 1 first on a toon without one; stray chapter files get attached to a season or are
flagged as errors by the validation. The season entry is also the single source of chapter visibility (status).

## Release 2026-10-04 — `platform` + `anger` together (owner decision 2026-09-30)

`anger` is fully contained in `platform` (checked: no commits on `anger` missing from `platform`; the old qr repo's
last commit 0332912 is the imported subtree). The release ships `platform` → `main`.

| When | Owner | Coordinator / agents |
|---|---|---|
| Sep 30 – Oct 2 | Chapter 2 slices from Figma: text-free art 390×625 (1x) + 1170×1875 (3x) → `cdn/root/catpope/images/{1x,3x}/s1c2-slice-NN.jpg`; GitHub environment `production` from `.env.example`; all-inkl: subdomains `api.` `admin.` `shop.` (+ SSL, `creator.` SSL), folders per hosting layout, mailbox `no-reply@` + SPF/DKIM | credits (running), scan-limit setting (running); wire chapter 2 into the creator once slices land (text boxes, season status `coming-soon` → `published` on release day, release date 2026-10-04); release checklist dry runs |
| Oct 3 | backups (DB, old qr folder, viewer folder); run migrations in phpMyAdmin (order in PLATFORM.md cutover) + content seed (`import-content --sql`) + credits seed (`shared/content/seed/credits-seed.mysql.sql`); `cdn/sync.py` (chapter 2, icons, avatars, mannequins) | `npm run deploy:dry-run` review; merge `platform` → `main`; "Deploy all"; point `api.` + `qr.` → test real stickers; then frontends |
| Oct 4 | publish chapter 2 (creator: season status → published, Publish) | final smoke: printed URLs, scan, claim, login mail, chapter 1 + 2 de/en, shop, admin |

Rollback per subdomain = point back to the old folder (old folders untouched, migrations additive).

## RULE: definition of done = full suite green (2026-09-30, owner)

No task (agent or coordinator) is finished until ALL of these pass on the current tree:
- `npm run check` at the root (every workspace, 0 errors)
- `npm run build` for every app (viewer, creator, admin, shop)
- `cd server && composer test` (PHPUnit, PHPStan, all contract suites incl. claim-race, public-urls via qr)
- `cd qr && composer test`
- the dev servers the owner uses (viewer 8000, CDN 8001, API 9000, qr 9001, creator 10000, admin 11000, shop 12000)
  still start and load
If something breaks that another task caused, report it — don't silently skip. After a larger stretch of breaking
changes the coordinator tags a known-good state (`git tag stable-YYYY-MM-DD[-n]`).

## RULE: keep the configuration small (2026-09-30, owner)

Model only what the product reads or the owner edits. Story context the owner explains (e.g. "she leads the biker
gang") is context for us, not a data field. Before adding a field/file/vocabulary: which screen edits it, which code
reads it? If neither, leave it out. (Groups/affiliations were removed for this reason.)

## Performance roadmap (2026-09-30, owner)

1. **Now (media agent):** P1 music 128 kbps (+ streaming), P3 character portraits WebP, P2 slices WebP + srcset +
   lazy loading (slice width/height), video slices (`Slice.video`, editable in the creator) with the chapter-1 key
   visual video re-encoded as slice 1.
2. **Then:** P4 one request per chapter (chapter endpoint with inlined scenes + light season overview), P5 production
   PHP tuning (OPcache check on all-inkl, compiled DI container, authoritative classmap, HTTP/2), P6 CDN plan
   (immutable caching for hashed media, Brotli/gzip, bunny.net — agreed with the owner before any change).
See PERFORMANCE.md for measurements.

## RULE: slice size 390 × 625 (2026-09-30, owner)

The default slice is **390 × 625 px** (webtoon standard, Naver), delivered at 1x as WebP. Every slice carries
`width`/`height`; new slices in the creator default to 390 × 625 (overwritten by the real image size when an image path
is set). Other heights are allowed, the width stays 390.

## RULE: one resolution everywhere (2026-09-30, owner)

No multiple image resolutions/densities anywhere in the apps (no 2x/3x, no srcset by density, no resolution toggles).
Slices are 1x WebP 390 px wide; the reduced load matters more than retina sharpness for now.

## RULE: content is served by the API from the database (2026-09-30, owner)

No file-based loading. All content lives in the database and is served on demand by the API service layer (entities,
caching, one request per chapter). The JSON configuration format stays only as import/export. Design: DATA.md.
PWA preloading / save-to-device is designed in DATA.md and built after the data refactor.

