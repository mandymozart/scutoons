# DATA — everything from the API, stored in the database (widened P4)

Written 2026-09-30 (branch `platform`), Phase C done the same day. Companion docs: [PLATFORM.md](PLATFORM.md) ("Data",
domain architecture, CDN), [PERFORMANCE.md](PERFORMANCE.md) (P4), [ITEMS.md](ITEMS.md) (API contract), [PLAN.md](PLAN.md)
(rules, release).

**Owner decisions (2026-09-30):** "Switch the entire loading process to not file-based. It has to be served via the API
and stored in the database. [...] a real service layer that can cache and get chapters or other entities on demand
rather than everything at once. For a PWA we will need smart pre-loading or a save-to-device concept." Then: clean
entity keys in the database (not path-like keys); every **toon-related** text and translation lives in the database;
UI translations stay in the apps' message files; the catalog (items, editions, products) follows later (B2).

## 1. Which texts and data live where

| What | Where | Edited in |
|---|---|---|
| toon list entry (status, names per language, languages) | DB entity `toon:<toon>` | creator (toon page) |
| toon settings (title, description, covers, map) per locale | DB `toon-settings:<toon>:<l>` | creator |
| season (title, plot, sequence + status per locale, `download` = the music album, `supporters`) | DB `season:<toon>:<n>:<l>` | creator (season page) |
| chapter (meta, slices, text boxes, audio, inline groups) | DB `chapter:<toon>:<n>:<l>` | creator |
| VN scene (a chapter group stored on its own; messages, choices) | DB `scene:<toon>:<chapter>:<name>:<l>` | creator |
| map texts (locations, routes) | DB `locations:<toon>:<l>`, `routes:<toon>:<l>` | import (no editor yet) |
| cast per season | DB `cast:<toon>` | creator (cast page) |
| characters, personas, looks, mood labels, wanted images | DB `characters`, `moods`, `asset-requests` (`{ de, en }` texts; A-pose looks may be `sensitive`, ITEMS.md "Sensitive content") | creator (characters) |
| credits (sections, custom roles) · people · role labels | DB `credits:<toon>` · `people` · `roles` | creator (credits) |
| events | DB `events` (toon-bound entries) | import (no editor yet) |
| items, recipes, editions, products, featured products (catalog texts) | server `config/*.json` via `/api/items/`, `/api/editions/`, `/api/products/` | file → **B2** (after this) |
| permissions map | server `config/permissions.json`, never served | file (private) |
| UI chrome (buttons, labels, errors) | each app's paraglide `messages/{de,en}.json` | code |
| codes, users, inventory, progress, avatar, settings | relational tables | admin / API |
| member preferences ("Hide sensitive content") | column `users.hide_sensitive` (default on), `GET/PUT /me/` | reader account page |
| collectables | **removed** (merged into items; VN variables stay story logic, the VN → item grant bridge collects) | – |

No app ships content files any more: `viewer/static/api/**` is gone, and so is every static fallback.

## 2. Storage: one versioned document table, clean entity keys

**Table `entity_versions`** (`id, entity_key, kind, toon NULL, locale NULL, json, author, created_at, status, message`):
every save is a new version, at most one `published` per key, the newest is the working copy (draft/publish/history/
restore). Documents are in the configuration format of `@scutoons/content` (unchanged). No relational content tables:
every read is "one entity (+ its children) of one toon in one locale"; compositions are cached (§3).

**Entity keys** (`EntityKey` in PHP, `parseEntityKey` / `entities` in `@scutoons/content`):

```
toon:<toon>                              index entry            toon-settings:<toon>:<l>      settings
season:<toon>:<n>:<l>                    chapter:<toon>:<n>:<l>  scene:<toon>:<chapter>:<name>:<l>
locations:<toon>:<l>   routes:<toon>:<l>  cast:<toon>   credits:<toon>
characters   moods   asset-requests   people   roles   events          (platform-wide)
```
Segments are `:`-separated; numbers are digits without leading zeros – seasons from 1, **chapters (and their scenes)
from 0** (2026-10-01: chapter 0 = a season's prologue, `chapter:catpope:0:<l>`, file `chapter0.json`, `ref.src`
`chapter0/<name>.json`; `EntityKey::CHAPTER_NUMBER_PATTERN`, `parseEntityKey`); the locale is the last segment of localized kinds; an *entity* is a key without its
locale (`chapter:catpope:2`), which is what the creator edits across languages. `kind`, `toon`, `locale` are stored as
columns for filtering (NULL = platform-wide / language-neutral). A chapter group's scene reference stays in the content
format (`ref.src: "chapter2/scene-14.json"`) and resolves to `scene:<toon>:2:scene-14` (`sceneEntityOfRef`,
`EntityKey::sceneOf`). A chapter carries an optional `featuring` array (owner, built 2026-09-30, ITEMS.md "Cast & featuring") – the chapter
endpoint and the season overview pass it through; the `/cast` answer always has the toon's star in every season.

**Season downloads and supporters (2026-09-30, owner):** the music album belongs to its season: `Season.download
{ music?, complete? }` (was `Toon.download`, removed from the type/schema; a leftover `download` in toon settings is only
a validation warning) and `Season.supporters [{ name, logo?, url? }]` (ordered, logo = image path in the toon's CDN
folder). Both language-neutral (the creator writes them to every locale), passed through by the library unchanged
(`LibraryTest::testSeasonDownloadAndSupportersPassThrough`); the reader's `/<toon>/download` (printed URL, unchanged)
offers season 1's `download`. catpope migrated in the seed + the local store (new published versions of
`toon-settings:catpope:{de,en}`, `toon-settings:sandbox:{de,en}`, `season:catpope:1:{de,en}`) and in
`content-seed.mysql.sql` (only those 4 catpope statements changed).

**Prologue (2026-10-01, owner):** catpope season 1 starts with chapter 0 "Roar Little Kitty" (the music video, public:
`isPreview: true`, published de + en, first in the season `sequence`, no chapter audio). Its one group holds one player
video slice (`SliceVideo.player` + `sources`, DESIGN.md "Player video slice"): poster `images/webtoon/s1c0/s1c0-slice-1.webp`
(780 × 411, from the 1080p file at 11.2 s – twice the slice width so the poster stays sharp behind the controls; a single
file, no density variants), thumbnail `images/s1c0-thumbnail.webp` (202 × 142 like the others), videos
`videos/roar-little-kitty.{480p,720p,1080p}.mp4`. Seed + local store (new published `chapter:catpope:0:{de,en}`,
`season:catpope:1:{de,en}`; backup `server/db/local.sqlite.bak-2026-10-01-prologue`), `content-seed.mysql.sql`: only those 4
statements changed (old rows keep their timestamps). The seed is idempotent per key: on a database where
`season:catpope:1:*` already exists the new season version must be imported separately.

**Season trailer (2026-10-01, owner):** `Season.trailer { loop, sources?, poster, chapter? }` – the toon page's
desktop hero loop (DESIGN.md "Toon page desktop"), language-neutral like `download` (the creator's season page writes it
to every locale; library passthrough tested in `LibraryTest`). catpope season 1: `videos/s1-trailer-loop.1280w.mp4` +
`1920w` for `(min-width: 1600px)`, poster `images/s1-trailer-poster.webp`, chapter 0. Seed + store (new published
`season:catpope:1:{de,en}`), the seed SQL's season statements carry it.

**Music image (2026-10-01, owner):** `Season.download.image` (optional, image path in the toon's CDN folder) = the
decorative record beside the toon page's music button (DESIGN.md "Toon page"); language-neutral, creator season page
"Downloads" → `download.image` (image field). catpope season 1: `images/s1-music-vinyl.webp` (320 × 215, the
`awakening_vinyl` item cut-out cropped to its content, WebP q85 – the "Awakening" LE vinyl is the album the season's
music zip belongs to). Seed + store (new published `season:catpope:1:{de,en}`, backup
`server/db/local.sqlite.bak-2026-10-01-music-image`), the seed SQL's 2 season statements carry it.

**Reading progress** is not content: it is the reader's game state per toon (`Checkpoint[]`, DESIGN.md "Start button"),
in localStorage and for members in the relational `progress` table via `/api/progress/<toon>` (unchanged API).

**Migration `2026-09-30-entity-keys`** (MySQL + SQLite, idempotent, dated after `content-versions`): creates
`entity_versions` and copies every `content_versions` row with the **same id** (history, "v71" labels, drafts stay) and
a key computed from its old path. `content_versions` is left untouched (rollback) and unused.

**Interchange format** (JSON files, the only place with file names): `shared/content/seed/` = `toons.json`,
`events.json`, `<toon>/<l>/{toon,season<N>,chapter<N>,locations,routes}.json`, `<toon>/<l>/chapter<N>/<scene>.json`,
`<toon>/_/{cast,credits}.json`, `scutoons/_/{characters,moods,asset-requests,people,roles}.json`. The mapping lives only in
the server's `Console\ContentFiles` (`import-content`, `export-content`). Legacy inputs of the one-time migrations
(per-locale `characters.json`, `credits.json`) are in `shared/content/legacy/`.

## 3. API

**Library** (public GETs, readers; `LibraryController`): `/api/library/toons` · `/library/toons/<toon>?locale=`
(`ToonBundle`: entry, settings, every `SeasonOverview`, map) · `/seasons/<n>` · `/chapters/<n>` (scenes inlined) ·
`/cast[?chapter=]` (registry reduced to the cast + `cast:<toon>`; resolve with `resolveCast`) · `/credits`
(credits, referenced people, roles) · `/library/characters` (whole registry, avatar bodies) · `/api/events/`.
- Locale `?locale=`, per-document fallback to the source language (`Content-Language`, `X-Content-Fallback`).
- Visibility on the server (the viewer's rules): `403 coming_soon { chapter }`, `403 not_public`, draft toon `404`;
  `CONTENT_SHOW_DRAFTS=1` locally. `?draft=1` (role creator) = newest versions, no gates, `private, no-store`.
- `Cache-Control: public, max-age=60` + `ETag`/304; server cache (APCu, else `storage/cache/library/`) keyed by the
  version ids involved → a publish is a new key, nothing is ever invalidated; drafts are never cached.

**Entities** (the creator's write path, role creator, `EntitiesController`): `GET /api/entities?toon=` (index; `toon=`
empty = platform-wide) · `GET /api/entities/<key>[?draft=1|?version=][&wrap=1]` · `PUT /api/entities/<key>`
`{ content, message?, base? }` (409 on a stale base) · `GET <key>/versions` · `POST <key>/publish { version? }` ·
`/unpublish` · `/restore { id }`. `/api/content/*` (paths) is gone. Admin: `GET /api/admin/content` lists entities
(`{ key, kind, toon, locale, latest, … }`).

## 4. Seeds, compare, production cutover

- `php server/bin/console import-content shared/content/seed [--toon=] [--only=<kinds>] [--update] [--sql]` –
  idempotent; `--sql` prints guarded INSERTs into `entity_versions` (dev-only `sandbox` left out).
  Committed SQL: `shared/content/seed/content-seed.mysql.sql` (everything), `characters-seed.mysql.sql`,
  `credits-seed.mysql.sql` (subsets; all idempotent – running more than one is harmless).
- `export-content <dir> [--toon] [--draft]` writes the store back into the layout (backup, diff, refresh the seed).
- Generators write the interchange layout and import it: `make-sandbox.ts` (→ `shared/content/seed/sandbox`, then
  `import-content --toon=sandbox --update`), `migrate-characters.ts` (→ seed, then `--only=characters,moods,
  asset-requests,cast --update`); `--no-import` skips the import. Regenerate the SQL after a seed change.
- Compare: `tests/entities-compare.php` (in `composer test`, fresh DB seeded from `shared/content/seed`) checks every
  library answer against the files; against production: `--report`.

**Cutover on production (Oct 3, phpMyAdmin, in this order)** – replaces the earlier `import-content <viewer/static/api>`
step: (1) the pending migrations in file-name order, including `2026-09-30-content-versions` and then
`2026-09-30-entity-keys`; (2) `shared/content/seed/content-seed.mysql.sql` (covers credits + characters too); (3) deploy
api → qr → frontends ("Deploy all"); (4) `php server/tests/entities-compare.php https://api.scutoons.com --report` and
the printed-URL checks. The old released viewer keeps its bundled static files until the new build replaces it, so
nothing breaks in between; after step 3 every frontend reads the library (no fallback).

## 5. Creator / admin

The creator edits `ContentDoc`s keyed by entity (`new ContentDoc(entity, locales, primary)`, `doc.key(locale)`), writes
through `/api/entities`, lists toons from `/library/toons?draft=1`, previews through the library in draft mode, and has
no static fallback (a missing entity starts empty). Labels come from one helper (`entityLabel` in `@scutoons/ui`, keyed
by entity) – no file names, `_` chips or "file" wording in the creator or admin UI; JSON is named only in import/export.

## 6. PWA concept (design only)

- **Service Worker** (SvelteKit `src/service-worker.ts`): app build + fonts precached per build version (cache-first,
  old caches dropped on `activate`); library JSON stale-while-revalidate with the ETag; slices/posters/portraits
  (immutable names, PLATFORM.md CDN rule) cache-first with an LRU of ≈ 150 MB; audio/video from the network (range
  requests) except for saved chapters (the SW answers ranges with 206).
- **Smart preloading** (idle time, not with `saveData` / 2G): after the first 2 slices + audio start → the next chapter
  (library), its first 3 slices, and the portraits of characters joining in the next scenes (inlined groups +
  `resolveExpression`).
- **Save to device** (chapter or season): a future `GET /library/toons/<t>/chapters/<n>/assets?locale=` lists every
  media path with sizes → the SW fills `offline-<toon>-<locale>-c<n>`, IndexedDB keeps fingerprint/bytes/saved_at;
  `storage.estimate()` + `storage.persist()` first; a new ETag downloads only changed paths; remove per chapter/season.
- **Membership**: saving only for chapters the device may read; the SW keeps the access `expires_at` and stops serving
  gated offline content after it or after a `410` on the 6-hourly `/access/` re-check (gated caches deleted).

## 7. Phases, release, risks

| Phase | What | When |
|---|---|---|
| A | this design | ✅ 2026-09-30 |
| B | server library endpoints, cache, import/export, contract suites | ✅ 2026-09-30 |
| C | entity keys + migration, `/api/entities`, all frontends on the API, static files and fallbacks removed, UI without file names | ✅ 2026-09-30 – **ships with the release** |
| B2 | catalog (items, editions, products/featured) into the store; QR toons list derived; permissions stay a private file | after the release |
| D | PWA (§6) | after the release |

**Risks**: no static fallback – API down = reader down (`/api/health/` monitoring; PWA later); visibility decided on the
server (compare test, `403` reasons); an `en` reader may get a flagged `de` document instead of an error; all-inkl may
lack APCu (file cache in `storage/cache`, writable, deploy-protected); the store is the only source (use
`export-content` as backup/diff; the seed folder is the reviewed snapshot); drafts are `no-store` and never cached.

## Phase B — result (2026-09-30)

Library service (`server/src/Domain/Library/`: `Library`, `Snapshot`, `Context`, `Answer`), `LibraryController`, cache
layer (`server/src/Infrastructure/Cache/`), settings `CACHE_DRIVER`, `CACHE_PATH`, `CONTENT_SHOW_DRAFTS`,
`export-content`, PHPUnit `LibraryTest` + `LibraryApiTest`, contract suites `entities-smoke` and `entities-compare`.
Decisions: a season entry whose chapter is missing is listed with empty fields (+ `X-Content-Missing`); the summary drops
`groups` + `audio` only; cast answers are language-neutral.

## Phase C — result (2026-09-30)

- **Server**: migration `2026-09-30-entity-keys` (local DB migrated: 298 → 298 rows, same ids; backups
  `server/db/local.sqlite.bak-2026-09-30-library`, `…-entity-keys`), `EntityKey`, `EntityStore`, `EntitiesController`
  (`/api/entities`), `Console\ContentFiles` (the only path ↔ key mapping), library + admin content + avatar unlocks on
  keys, `/api/library/characters`. Tests: `EntityStoreTest` (store, key scheme, interchange layout, the SQL migration),
  `content-smoke` rewritten for `/api/entities` (62), `entities-smoke` (32), `entities-compare` (37), admin-smoke on keys.
  The legacy `ContentService` / `ContentController` classes are no longer routed or used; deleting them needs the
  owner's permission (the deletion was refused by the permission guard).
- **Content files**: `viewer/static/api/**` moved to `shared/content/seed/` (interchange), legacy inputs to
  `shared/content/legacy/`; seed SQL regenerated.
- **Viewer**: root layout `/library/toons`, toon layout one bundle + cast, chapter page one request, credits
  `/library/…/credits` (legacy credits component removed), download from the layout, events `/events/`, avatar
  registry `/library/characters`; `API_BASE` gone.
- **Creator**: `ContentDoc` by entity, `/api/entities` write path, no `STATIC_API`/`getStatic`, preview and image
  picker through the library (draft), toons from the library, labels by entity (breadcrumbs, groups, history, problems).
- **Admin**: content overview by entity (+ a "platform-wide" group), labels by entity.
- **Measured** (`scripts/perf/measure.mjs`, production builds, 4G, cold, median of 3; before = the previous build with
  its bundled static files):

  | Page | loaded (no media) | FCP | requests | content requests |
  |---|---|---|---|---|
  | `/` | 1534 → 1546 ms | 664 → 676 ms | 57 → 57 | 1 → 1 |
  | `/catpope` | 1376 → **1133 ms** | 984 → **736 ms** | 73 → 64 | 12 (+ failed store reads) → **3** |
  | chapter 1 | 2839 → **2672 ms** | 1036 → **896 ms** | 86 → 78 | 14 → **4** |
  | sandbox chapter 5 (VN) | 1008 → **862 ms** | 808 → **740 ms** | 67 → 61 | 13 → **4** |
