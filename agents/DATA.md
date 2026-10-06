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
| toon list entry (status, languages; the name = `title` per language, decision 4 of agents/NEUTRAL.md) | DB entity `toon:<toon>` | creator (toon page / toon settings) |
| language-neutral toon settings (authors, chapter count, thumbnail, cover, map, landing `tile`, `announce`) + the toon's title / description / release / status per language (locale maps, agents/NEUTRAL.md step 4) | DB entity `toon:<toon>` (the same entry); legacy `toon-settings:<toon>:<l>` | creator (toon settings) |
| toon settings per locale (title, description, release, status) | DB `toon-settings:<toon>:<l>` | creator |
| season (title, plot, sequence + status per locale, `download` = the music album, `supporters`) | DB `season:<toon>:<n>` (language-neutral, texts as locale maps); legacy `season:<toon>:<n>:<l>` | creator (season page) |
| chapter (meta, slices, text boxes, audio, inline groups) | DB `chapter:<toon>:<n>` (language-neutral); legacy `chapter:<toon>:<n>:<l>` | creator |
| VN scene (a chapter group stored on its own; messages, choices) | DB `scene:<toon>:<chapter>:<name>` (language-neutral); legacy `scene:<toon>:<chapter>:<name>:<l>` | creator |
| the world map (places, routes, layers; names `{ de, en }`; unlock points) | DB `map` (platform-wide, language-neutral; agents/MAP.md §8) | creator (Universe → Map; a slice's "unlocks place" in the chapter editor) |
| old per-toon map texts (locations, routes) | DB `locations:<toon>:<l>`, `routes:<toon>:<l>` – read-only since `map`, removed from the seed after one release | – |
| cast per season | DB `cast:<toon>` | creator (cast page) |
| characters, personas, looks, mood labels, wanted images | DB `characters`, `moods`, `asset-requests` (`{ de, en }` texts; the A-pose look is `swim` – the naked one was removed 2026-10-05, ITEMS.md "Reader avatar bodies"; **no male/female** – an optional `testo` level 0–100 per character/persona (more testo → reads more male, less → more female) and `tendencies`, ITEMS.md "Substances") | creator (characters) |
| reader stats (testo, appetite) and timed effects | tables `member_stats`, `member_effects` (guests: localStorage `scutoons_reader`), via `GET /me/`, `POST /inventory/use/` | reader inventory ("Benutzen") |
| credits (sections, custom roles) · people · role labels | DB `credits:<toon>` · `people` · `roles` | creator (credits) |
| the toon's screenplay (per language, creator only, never served to readers) | DB `script:<toon>:<l>` | creator (script page) |
| events | DB `events` (toon-bound entries) | import (no editor yet) |
| items, recipes, albums, editions, products, toons (catalog texts; wearables' `wear` overlays per body, ITEMS.md "Wardrobe") | tables `catalog_*` / `toons` (2026-10-04, agents/CATALOG.md; no file seed since 10.30 – tests: `server/tests/fixtures/catalog/`) via `/api/items/`, `/api/editions/`, `/api/products/`, `/api/toons/` | admin API `/api/admin/catalog/…` (history, undo); admin UI = CATALOG Part 3 |
| the landing page's featured editions (order, colour) | table `catalog_featured` (no default list since 10.30) via `/api/products/featured/` | admin ("Startseite") |
| permissions map | tables `catalog_permissions` / `catalog_code_types`, never served to readers | admin API |
| UI chrome (buttons, labels, errors) | each app's paraglide `messages/{de,en}.json` | code |
| codes, users, inventory, progress, avatar, settings | relational tables | admin / API |
| member preferences ("Hide sensitive content", "Media quality") | columns `users.hide_sensitive` (default on), `users.media_quality` (default `auto`), `GET/PUT /me/`; a guest's media quality in localStorage | reader account page |
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
season:<toon>:<n>                        chapter:<toon>:<n>      scene:<toon>:<chapter>:<name>   (language-neutral)
season:<toon>:<n>:<l>                    chapter:<toon>:<n>:<l>  scene:<toon>:<chapter>:<name>:<l>  (legacy per locale)
locations:<toon>:<l>   routes:<toon>:<l>  cast:<toon>   credits:<toon>   script:<toon>:<l>  (creator only)
characters   moods   asset-requests   people   roles   events   map    (platform-wide)
```
Segments are `:`-separated; numbers are digits without leading zeros – seasons from 1, **chapters (and their scenes)
from 0** (2026-10-01: chapter 0 = a season's prologue, `chapter:catpope:0:<l>`, file `chapter0.json`, `ref.src`
`chapter0/<name>.json`; `EntityKey::CHAPTER_NUMBER_PATTERN`, `parseEntityKey`); the locale is the last segment of localized kinds; an *entity* is a key without its
locale (`chapter:catpope:2`), which is what the creator edits across languages. `kind`, `toon`, `locale` are stored as
columns for filtering (NULL = platform-wide / language-neutral). A chapter group's scene reference stays in the content
format (`ref.src: "chapter2/scene-14.json"`) and resolves to `scene:<toon>:2:scene-14` (`sceneEntityOfRef`,
`EntityKey::sceneOf`). A chapter carries an optional `featuring` array (owner, built 2026-09-30, ITEMS.md "Cast & featuring") – the chapter
endpoint and the season overview pass it through; the `/cast` answer always has the toon's star in every season.

**Language-neutral chapters, scenes, seasons (2026-10-06, agents/NEUTRAL.md):** one document per entity, stored under
the entity itself (`chapter:catpope:2`, `EntityKey::NEUTRAL_DOC_KINDS` / `NEUTRAL_DOC_KINDS`, `isNeutralDocKey`;
`locale` column NULL). Structure once; every text leaf is a locale map – chapter `title` / `description` / `release`,
text box `text` (`{ de: string | string[] }`), VN `showMessage.params.text` / `addChoice.params.label`, season `title` /
`description` / `release` / `sequence[].status`. No `language`, no chapter `status`. Types `ChapterDoc` / `GroupDoc` /
`SeasonDoc` (`neutral.ts`), schemas `chapter-doc` / `group-doc` / `season-doc` (the save guard picks them for a key
without locale, `ContentSchemas::NEUTRAL_BY_KIND`). The per-locale keys stay valid (legacy) until production is
converted; the library prefers a neutral document and falls back to them per entity (§3). The interchange layout puts
neutral documents in `<toon>/_/chapter<N>.json`, `<toon>/_/chapter<N>/<scene>.json`, `<toon>/_/season<N>.json`.
The creator edits only the neutral documents (phase 2A, 2026-10-06): one `ContentDoc` per chapter / scene / season
(locale `_`, `texts` = the toon's languages), texts in the header's text language (`doc.text` / `doc.setText`); an
entity without a neutral document is converted in memory on load (`neutralizeDocs`, a notice) and saved as the neutral
key – the per-locale keys are never written again. Validation: `validateNeutral` (structure once + per-locale
completeness); accepted per-locale warnings keep their locale in the key (`en:<place>#<hash>`). The owner converts a
whole toon on `/<toon>/convert` (`planConversion`, shared with `npm run dev:neutralize`). Phase 2B (2026-10-06): season
page, cover editor, new chapter / season flows, the MCP tools (`read_entity` / `save_draft` / `validate` on the neutral
key, `validateNeutral`), the sandbox generator and the test fixtures are neutral too; a chapter's status per language is
edited in its meta panel only (in the header's text language). A new language adds nothing but the language in the
toon entry (texts missing → warnings); only the per-locale toon settings get a "create from the source".

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
video slice (`SliceVideo.player` + `variants`, DESIGN.md "Player video slice"; until 2026-10-01 `sources` with media queries): poster `images/webtoon/s1c0/s1c0-slice-1.webp`
(780 × 411, from the 1080p file at 11.2 s – twice the slice width so the poster stays sharp behind the controls; a single
file, no density variants), thumbnail `images/s1c0-thumbnail.webp` (202 × 142 like the others), videos
`videos/roar-little-kitty.{480p,720p,1080p}.mp4`. Seed + local store (new published `chapter:catpope:0:{de,en}`,
`season:catpope:1:{de,en}`; backup `server/db/local.sqlite.bak-2026-10-01-prologue`), `content-seed.mysql.sql`: only those 4
statements changed (old rows keep their timestamps). The seed is idempotent per key: on a database where
`season:catpope:1:*` already exists the new season version must be imported separately.

**Media quality (2026-10-01, owner, branch `media-quality`, merged after the release):** `SliceVideo.sources [{ src,
media }]` is replaced by `SliceVideo.variants { low?, medium?, high? }` next to `src` (the default; DESIGN.md "Media
quality", PLAN.md "RULE: one file per quality level"), plus `SliceVideo.controls`, `SliceVideo.viewport { position,
fit }` (full-viewport placement) and `SliceVideo.frame { ratio, fit?, position? }` (the box in the column; `ratio` "9:16" or
[w, h] within 4:1 … 1:4, the slice's `height` follows at 390 wide); `fit` is one of contain / width / height / cover
(DESIGN.md "Video placement"). No stored content uses `viewport` or `frame` yet, so nothing to migrate. Chapter 0: `src` 720p, variants 480p / 720p / 1080p (`player: true` as before). Seed
(`catpope/{de,en}/chapter0.json`) and `content-seed.mysql.sql` (only the 2 chapter-0 statements changed, timestamps
kept). **The seed is guarded by `WHERE NOT EXISTS`**, so a database that already has `chapter:catpope:0:*` (production
after the 2026-10-04 cutover, every local store) needs a new published version: locally `php bin/console import-content
../shared/content/seed --toon=catpope --only=chapter --update` (2 updated, everything else unchanged – checked on a copy
of the owner's store 2026-10-01); production: `shared/content/seed/2026-10-01-media-quality-chapter0.mysql.sql`
(idempotent: inserts the new published version per locale and turns the old published one into a draft only when the
published one still has `sources`). Member setting: column `users.media_quality` (migration `2026-10-01-media-quality`,
MySQL + SQLite, idempotent, default `auto`), `GET /me/ user.media_quality`, `PUT /me/ { media_quality }`; the season
trailer keeps its `sources` (desktop hero loop, a later candidate).

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

**Announced toons + the landing tile (2026-10-01, owner):** toon settings `announce?: boolean` and `tile?: { image,
color? }` (`Toon` type, schema, validation, creator toon page section "Startseite": "Ankündigen" + `tile.image` /
`tile.color`), language-neutral. `tile.image` is a path in the toon's CDN folder like the other settings images, a registry
portrait as `../characters/<id>/<file>`; `tile.color` overrides the star's registry colour (`toonTileColor`). Library
rule (`Library::toons`, `LibraryTest::testAnnouncedToonsAndTilesInTheList`): `/library/toons` = published entries + the
unpublished ones whose settings (source locale, else any) say `announce: true`, marked `announced: true`; every entry
carries its settings' `tile`. An announced toon's bundle/chapters stay 404, so no season, chapter or date leaks. Drafts
only with `CONTENT_SHOW_DRAFTS` (local) or `?draft=1` (creator); the sandbox (`Library::DEV_ONLY_TOONS`) never publicly.
Content: catpope tile = `../characters/catpope/catpope_portrait-neutral.webp` (blue from the registry); toptoy +
yellowmouse `announce: true` with `toptoy_portrait-neutral.webp` (black) and `yellowmouse_helmet_portrait-neutral.webp`
(yellow – Mausboss in her helmet, the mouse is not revealed). Top Toy is written in two words (toon entry names +
registry name, owner 2026-10-01) and its entry has the duo `avatar.webp`. Seed (`toptoy/{de,en}/toon.json`,
`yellowmouse/{de,en}/toon.json` new, from the owner's store versions), local store (new published versions; backups
`server/db/local.sqlite.bak-2026-10-01-landing`, `…-top-toy`), seed SQL: 6 toon-settings statements (2 changed, 4 new),
`toon:toptoy` and `characters` (also in `characters-seed.mysql.sql`); unchanged statements keep their timestamps.

**Since 2026-10-06 (agents/NEUTRAL.md step 4) the per-language part is in the entry too**: `title`, `description`,
`release` as locale maps and `languageStatus` (the status per language; the entry's `status` stays the whole toon's);
`toon-settings:<toon>:<l>` is legacy – the library reads it only while the entry has no `title` map, the creator edits
the entry only (one document), `/library/toons?locale=` leaves out a toon closed in that language. **The name is `title`**
(owner 2026-10-06, NEUTRAL.md decision 4): the old `i18n.<l>.name` is no longer stored (folded into `title` by the
conversion; `ToonIndex.i18n` is `@deprecated`, read only from older entries); every list / menu / nav reads
`toonName(entry, locale)` (`@scutoons/content`: `title` locale → source → any, else the old i18n name, else the id). The
answered entries (`/library/toons`, the bundle's `entry`) carry `title` = the name in each toon language (resolved) and,
**legacy for older viewer builds (drop later)**, `i18n.<l>.name` computed from it (`Library::indexEntry`, FORMAT 7); an
entry without texts keeps its stored `i18n`. `toonLanguages()` / `Library::languages`: `languages` → `title` keys → old
`i18n` keys. The paragraph below is the 2026-10-01 state.

**Language-neutral toon settings in the toon entry (2026-10-01, owner):** "fields with no text are edited and stored per
language, which is pointless". Every toon setting that is not text lives once in the index entry `toon:<toon>`
(`TOON_NEUTRAL_FIELDS` in `@scutoons/content` = `Library::TOON_NEUTRAL_FIELDS`): `authors`, `chapters` (count),
`thumbnail`, `map` (layers, width, height, center), `announce`, `tile` (image, color) and – since the same evening –
`cover` (below, "Language-independent media"). They stay per language in
`toon-settings:<toon>:<l>` (`ToonSettings`): `title`, `description`, `language`, `id`, **`release`** (catpope de 2025-10-04,
en 2025-12-01 – a translation is released later) and **`status`** (the library gates each language on it). Types: `ToonIndex extends ToonNeutralSettings`,
`ToonSettings = Omit<Toon, …>`, `Toon` = the reader's merged shape (unchanged); schema `toon.schema.json` ← `ToonSettings`.
- **Library:** `/library/toons/<toon>` merges the entry's neutral fields into the locale's `toon` (the entry wins over a
  stale per-language copy) and returns `entry` without them, so both answers keep their shape; `/library/toons` keeps only
  `tile` (+ `announced`) of them. Older data without them in the entry still reads `announce` / `tile` from the settings
  (`LibraryTest::testAnnounceAndTileFromOlderToonSettings`, `…MergesTheNeutralSettings`).
- **Validation:** the neutral fields in a `toon-settings` document are warnings ("belongs to the toon entry – the reader
  ignores it here"); the entry (`validateFile('index', …)`) checks them (all optional; map/tile rules as before).
- **Creator:** the toon settings page edits the entry too (`docs={[settings, entry]}` – the bottom bar saves/publishes
  both, like cast + characters); block "Für alle Sprachen gleich" / "Same in every language" without "per language".
  The entry's label is "Toon-Eintrag" / "Toon entry" (was "Languages"; `EntityLabels.entry`). New toons: `newToon()` writes
  the per-language part only, the neutral settings start empty in the entry.
- **Migration:** seed `toons.json` carries them (the source language's value, else the non-empty one – de and en were equal
  everywhere), the 8 `toon.json` files lost them; `make-sandbox.ts` checks the sandbox's entry (`neutralSettings`). Local
  store: new published versions of `toon:{catpope,toptoy,yellowmouse,sandbox}` and their `toon-settings:*:{de,en}` through
  `/api/entities` as the test user (backup `server/db/local.sqlite.bak-2026-10-01-neutral-settings`); the library answers
  were byte-for-byte the same before and after (sorted JSON). Seed SQL: only the 3 + 6 changed statements (toon entries,
  toon settings; old rows keep their timestamps), and its `characters` statement now equals `characters-seed.mysql.sql`'s
  (it lagged behind the last colour change). Idempotent per key: a database that already has these keys needs the new
  versions imported separately.

**Script (2026-10-01, owner):** "Per toon a place where I can put scripts … a huge text block which I can edit and that is
versioned, like the rest. Scripts come per toon, not per chapter." Entity `script:<toon>:<l>` (localized: a script is
text; the source language is the master, translations may be partial) = `ToonScript { text, meta? }`, `meta` = the
screenplay editor's `config.json` meta (`title, author, status, date, basedOn, basedOnFrom`, all text; `SCRIPT_META_FIELDS`).
The text is stored exactly as typed (LF, indentation, trailing spaces – the format is whitespace-based; JSON escapes
round-trip byte-identically). Format = the screenplay editor (`C:\Development\screenplay-editor`, README "Writing Guide",
`lib/src/parse.js`); `scriptOutline(text)` (`@scutoons/content` `script.ts`) reads chapters `# <n>. Kapitel|Chapter`,
scene headings `<n>   INT./EXT. …` (column 0, inside a chapter) and songs `# <n>. Lied|Song` with their lines, the same
rules as the editor's parser (which knows only the German terms; the English script writes Chapter/Song).
- **Creator only**: `EntityKey::CREATOR_ONLY_KINDS = ['script']`. Read/written only through `/api/entities` (role creator;
  member 403, anonymous 401); no library endpoint, and `Library\Snapshot` excludes the kind, so no reader answer can
  contain it and a script save never changes a library cache key (`LibraryTest::testScriptsAreNeverInTheLibrary`,
  entities-smoke "Script"). Validation: `validateFile('script')` (text a string, meta fields text, unknown meta keys
  warned), `validateLocales` notes an empty translation only; the media-locale check skips it (plain text).
- **Seed / interchange**: `<toon>/<l>/script.json` (`ContentFiles`). `import-content --sql` (the production seed) leaves
  scripts out unless `--only=script` – **the seed SQL does not contain them (owner default: scripts are not in
  production)**; a local `import-content` takes them like everything else. Generator: `shared/content/scripts/
  import-screenplay.ts --dir=<folder with screenplay.txt + config.json> --toon= --locale=` (checks the round trip).
- **Content**: catpope de = `cat-pope-awakening/de/screenplay.txt` (1834 lines, 71 177 bytes, complete), en = the partial
  translation (395 lines, 13 039 bytes), meta from each `config.json`; byte-identical in the seed and in the local store
  (published versions 482/483, backup `server/db/local.sqlite.bak-2026-10-01-script`). Other toons have none (the page
  starts empty).

**Reading progress** is not content: it is the reader's game state per toon (`Checkpoint[]`, DESIGN.md "Start button"),
in localStorage and for members in the relational `progress` table via `/api/progress/<toon>` (unchanged API). The map's
discoveries are one **world** state for every toon under the reserved id `scutoons` (`/api/progress/scutoons/`, localStorage
`scutoons_game_scutoons`): its `map` (places, routes, positions) merges as sets on the server and in the viewer
(`mergeMapState`), the rest is last-write-wins (agents/MAP.md §2.4, §8.2).

**The world map (2026-10-01, branch `map`, agents/MAP.md §8):** entity `map` (`WorldMap`: width, height, layers, places
with `{ de, en }` names, toons, `hidden` + `unlock { toon, chapter, at, slice, scene }`, card `chapter`, image; routes with
place / free waypoints and a style), seed `scutoons/_/map.json`, migrated by `shared/content/scripts/migrate-map.ts` from
catpope's `locations.json` / `routes.json` (every value kept – `entities-compare.php` proves it). The toon entry's `map` is
only the start view `{ center?, zoom? }` (set in the builder). The library computes the bundle's old `locations` / `routes` /
`toon.map` from the world for older viewer builds; the new reader reads `GET /library/map?locale=&toon=` (places and routes
of readable toons, names resolved, `view { center, zoom, color }` – the sea is the toon's colour).

**Migration `2026-09-30-entity-keys`** (MySQL + SQLite, idempotent, dated after `content-versions`): creates
`entity_versions` and copies every `content_versions` row with the **same id** (history, "v71" labels, drafts stay) and
a key computed from its old path. `content_versions` is left untouched (rollback) and unused.

**Interchange format** (JSON files, the only place with file names; `shared/content/seed/` + `legacy/` were deleted in
CUTOVER 10.30 – the layout stays for `import-content` / `export-content`): `toons.json`,
`events.json`, `<toon>/<l>/{toon,season<N>,chapter<N>,locations,routes}.json`, `<toon>/<l>/chapter<N>/<scene>.json`,
`<toon>/_/{cast,credits}.json`, `scutoons/_/{characters,moods,asset-requests,people,roles}.json`. The mapping lives only in
the server's `Console\ContentFiles` (`import-content`, `export-content`).

**Sensitive slices (2026-10-05, owner):** `Slice.sensitive?: boolean` (`SensitiveFlag`, sensitive.ts) – stored only when
true, the same in every language (the creator writes it to every locale like the media; `validateLocales` warns when
locales differ; not a boolean = error). Readers with "Hide sensitive content" on see the slice blurred and muted
(ITEMS.md "Sensitive slices"). Nothing stored needs a migration.

## 3. API

**Library** (public GETs, readers; `LibraryController`): `/api/library/toons` · `/library/toons/<toon>?locale=`
(`ToonBundle`: entry, settings, every `SeasonOverview`, map) · `/seasons/<n>` · `/chapters/<n>` (scenes inlined) ·
`/cast[?chapter=]` (registry reduced to the cast + `cast:<toon>`; resolve with `resolveCast`) · `/credits`
(credits, referenced people, roles) · `/library/characters` (whole registry, avatar bodies) · `/library/map?locale=&toon=`
(the world map, agents/MAP.md) · `/api/events/`.
- Locale `?locale=`, per-document fallback to the source language (`Content-Language`, `X-Content-Fallback`).
- Chapters, scenes, seasons (2026-10-06): a language-neutral document wins (`Context::pick` → `NeutralDoc::resolve`, the
  same rules as `resolveNeutral` in `@scutoons/content`, golden fixtures `shared/content/test/fixtures/neutral/`); the
  answer keeps the per-locale shape (`language` = served locale, a chapter's `status` = its season status in that
  locale). Served locale: the asked one when the document has it (chapter / season: a `title` in it; scene: any text;
  no text at all = every locale), else the source, else the toon's other languages; a text missing or blank in it falls
  back the same way. Without a neutral document the per-locale ones are read as before. Mixed works both ways (neutral
  chapter + per-locale scene and back). `Library::FORMAT` 5. `content:neutral-check` compares every answer with the
  legacy one (`Library::legacyOnly()`).
- Visibility on the server (the viewer's rules): `403 coming_soon { chapter }`, `403 not_public`, draft toon `404`;
  `CONTENT_SHOW_DRAFTS=1` locally. `?draft=1` (role creator) = newest versions, no gates, `private, no-store`.
- `Cache-Control: public, max-age=60` + `ETag`/304; server cache (APCu, else `storage/cache/library/`) keyed by the
  version ids involved → a publish is a new key, nothing is ever invalidated; drafts are never cached.

**Entities** (the creator's write path, role creator, `EntitiesController`): `GET /api/entities?toon=` (index; `toon=`
empty = platform-wide) · `GET /api/entities/<key>[?draft=1|?version=][&wrap=1]` · `PUT /api/entities/<key>`
`{ content, message?, base? }` (409 on a stale base) · `GET <key>/versions` · `POST <key>/publish { version? }` ·
`/unpublish` · `/restore { id }`. `/api/content/*` (paths) is gone. Admin: `GET /api/admin/content` lists entities
(`{ key, kind, toon, locale, latest, … }`).
- **Type guard (2026-10-04):** every PUT is checked against its kind's JSON Schema (`ContentSchemas`, opis/json-schema;
  the schemas are generated from the TS types – `npm run schema -w @scutoons/content` writes shared/content/schema/ and
  the server's copy server/schema/content/, `npm run check` fails when either is stale) → `422 invalid_content
  { problems: ["/0/city: …"] }`. Extra fields are allowed; `locations` / `routes` (old map) are not checked.
  `php bin/console content:check [--db] [--kind]` checks what is stored (newest + published per key; neutral
  chapters / scenes / seasons against their neutral schemas). Restore and
  the seed import are not guarded. The first run fixed stale types (narration `showMessage` without `character`,
  `RefLink { src }`, optional audio trigger `rect`/`gate`/`exclusive`, `keyvisual` as a plain path).

## 4. Seeds, compare, production cutover

- **Since CUTOVER 10.30 (2026-10-05) production is the only source**: local dev loads production's `entity_versions`
  (+ catalog + asset tables) from `server/db/prod-seed.sql` (`npm run dev:db -- seed`, `db:prod-seed`). The committed
  content seed, its SQL files, `migrate-*.ts` and `tests/entities-compare.php` are gone.
- `php server/bin/console import-content <dir> [--toon=] [--only=<kinds>] [--update] [--sql]` – idempotent; `--sql`
  prints guarded INSERTs into `entity_versions` (dev-only `sandbox` left out).
- `export-content <dir> [--toon] [--draft]` writes the store back into the layout (backup, diff).
- `npm run dev:neutralize [-- --apply] [--toon x] [--json file]` (local dev only): per-locale chapters / scenes /
  seasons → neutral documents with `neutralizeDocs` (report of what the source locale overrode); new versions only,
  idempotent. Production converts from the creator's Convert page (NEUTRAL.md, phase 2).
- The sandbox toon (dev only): `make-sandbox.ts` reads characters + moods from the local DB, writes `logs/dev/sandbox`
  and imports it (`npm run dev:db sandbox`). Chapters, scenes and the season are language-neutral (`_/chapter<N>.json`
  …; dialogue de + en, ko not translated → German); per-locale ones of earlier runs are deleted from the dev DB first.

**Cutover on production (Oct 3, phpMyAdmin, in this order; history – the seed SQL and compare suite no longer exist)** – replaces the earlier `import-content <viewer/static/api>`
step: (1) the pending migrations in file-name order, including `2026-09-30-content-versions` and then
`2026-09-30-entity-keys`; (2) `shared/content/seed/content-seed.mysql.sql` (covers credits + characters too); (3) deploy
api → qr → frontends ("Deploy all"); (4) `php server/tests/entities-compare.php https://api.scutoons.com --report` and
the printed-URL checks. The old released viewer keeps its bundled static files until the new build replaces it, so
nothing breaks in between; after step 3 every frontend reads the library (no fallback).

## 5. Creator / admin

The creator edits `ContentDoc`s keyed by entity (`new ContentDoc(entity, locales, primary)`, `doc.key(locale)`), writes
through `/api/entities`, lists toons from `/library/toons?draft=1`, previews in the live reader (`?preview=draft`, the viewer's draft preview), and has
no static fallback (a missing entity starts empty). Labels come from one helper (`entityLabel` in `@scutoons/ui`, keyed
by entity) – no file names, `_` chips or "file" wording in the creator or admin UI; JSON is named only in import/export.

**Accepted warnings (2026-10-04):** a warning the creator found fine ("Looks fine" in the Problems panel) is stored in the
document it is about, in the warning's locale (the source locale for one about every locale): `accepted_warnings: string[]`
on every validated root type (`AcceptedWarnings`, `@scutoons/content` accepted.ts). An entry is `<place>#<hash>`: the path
with array items by their `id` where they have one, and an FNV hash of message + value – a changed value brings the
warning back. Accepting is an ordinary edit (unsaved, undoable, saved/published with the page); readers ignore the field.
Errors can't be accepted. Creator side: `lib/editor/accepted.ts` (panel + the bar's warning count).
**Admin catalog (2026-10-04):** the catalog tabs items / recipes / albums have the same panel ("Problems e/w" in the bar,
a right drawer; `admin/src/lib/catalog/problems.ts`, `catalog-problems.svelte`): `validateCatalog` over the stored
entries + the open form, the wear overlays against the published character registry (`GET /library/characters`).
A warning is stored in the catalog entry it is about (`accepted_warnings` in its `data`; `catalogWarningPlace` in
accepted.ts: kind + id + the path inside the entry), one about a whole list ("no player plays …") in the meta entry
`items`. ✓ on the open entry goes into its form (unsaved, Save keeps it); on any other entry it is saved at once (one
history row, note "warning accepted", undo takes it back). The server never serves the field (`CatalogStore::reader`:
/api/items/, /api/products/ …) and refuses a malformed list (422).

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

## Linked groups and a group's end (2026-10-06, owner)

- **`Group.continues: true`** – the group flows straight into the next one on the same reader page, no follow-up bar
  between. A **chain** = a maximal run of consecutive groups where all but the last continue (`chainOf`, `chainSlices`,
  `linksToNext` in `@scutoons/content` chains.ts). `?label=` of any group of a chain shows the whole chain (scrolled to
  that group); slice indices on the page run on across it.
- **VN in linked groups** (owner 2026-10-06): any group of a chain may have a VN script and slice events. The cases:

  | Case | Rule |
  |---|---|
  | several scripts (`events`) in one chain | allowed. They run ONE AFTER ANOTHER through the one VN context (viewer `script-queue.ts`, `interactions/vn-page.svelte`): a script asks to start when its group's first slice is seen and starts once no other script of the page runs and nothing waits for the reader (no message waiting for its click, no choice open, no slice trigger running) – else it waits in line. The stage carries over between the scripts of a page (authors use join / leave; validation checks each script on its own, a chain can be entered in the middle). A script's label (random jumps, choices) = its own group's. |
  | when a script starts | a lone group (no chain): on mount (as before). In a chain – its first group too: when the group's first slice is seen; a group without slices: when the page slice before it is seen (at once when it is the page's first). |
  | a script that can jump away (a `jumpTo` event, a choice whose `onSelect` jumps) in a group that `continues` | allowed (other scenes may jump back into the chain). Whatever follows a jump must be reachable by a label: the next group needs a `label` – a **warning** when it has none (`scriptJumps`); the creator reminds next to the script with a one-click "Give it a label". |
  | slice events (triggers) | allowed in every group. A slice choice with jumpTo is the author's deliberate exit. |
  | a slice event's `divert` | goes into its OWN group's script (the slice's group context, viewer `group-scope.ts`); validation: that script needs the `label` point (`checkScriptPoints`, per group). |
  | text boxes revealed by scripts (`showTextBox params.dialog`) | kept per group on the page (`scope`: refs, revealed boxes, playing effects). Text box ids are unique within the chapter, its scenes included: the creator gives a new box an id no other box of the chapter has (ids stay hidden, never typed); validation **error** on a duplicate (safety net). |
  | scrolling | frozen (no wheel / touch scroll) while anything waits: a message waiting for its click, a choice open, a script running or in line, a slice trigger running – until all of it is through (`vn.state.frozen`). Shown by a Feather `pause` mark directly below the logo (an indicator, not a button; tooltip "Scrolling paused – answer the message first"). |
  | follow-up bar / `end` (`auto` too) | only when the chain's last slice was seen AND every script the page must run has finished (none in line, none waiting) and nothing is frozen (`vn.state.pageDone`). |
  | `?label=` into the middle of a chain (option B) | the whole chain renders, scrolled to the labelled group (as before); scrolling back up works. The groups ABOVE the target count as read: their scripts don't run (they don't block or freeze), their slice triggers don't fire, their text boxes show as plain (revealed) content, the follow-up bar doesn't wait for them. The target group and those below run as normal. |
  | a jump into the page the reader is on (same chain) | the page starts again (like "Nochmal"): another group of the chain = another arrival (`{#key}` on chain start + arrival group), the very same URL = a replay. |
  | "↺ Nochmal" / leaving the page | a fresh page: every script of the chain again, the stage and revealed boxes cleared (`vn.claimPage`); leaving cancels the line and every script. |
  | the VN box / characters overlay | one per page (vn-page.svelte), shared by all groups of the chain. |
  | game variables, reading progress | unchanged (variables are toon-wide; progress counts slices across the chain). |
  | the chapter's last group | can't continue (error; the creator doesn't offer it). |

- **Reachability** (owner 2026-10-06, `@scutoons/content` reach.ts `chapterReach`): a graph over the chapter – groups,
  slices, script events, slice-event runs and placed items' tap events, named points, diverts, jumps, choices,
  conditions – walked with visited sets (iterative, O(nodes + edges), never runs a script; cycles are harmless). Every
  group / slice / script event is *reachable*, *only through a condition* or *unreachable*. Entry points: the chapter's
  first group, a labelled chain start the follow-up bar leads to (the chain before can finish, no authored `end`), and a
  labelled group something in the toon refers to (`toonFacts`: any jump target, end, random list or map place in the
  toon's documents; without the facts every labelled group counts). Scrolling past a group needs its script to finish
  (the page is frozen meanwhile) and past a slice its scroll triggers' runs. A condition whose variable is set nowhere in
  the toon (setVariable, a choice's set / add, a picked-up item) and that needs a value is *never true* – its branch
  counts as never taken; a divert / question loop with no end and no jump is an *endless loop*. Validation warns
  (`checkReach`: "unreachable", "never true", "endless loop"); the creator labels the items with a chip (chapter tree,
  slice list, VN event lists – `reach-chip.svelte`, debounced, the toon's other documents loaded once) so the author can
  find and delete them.
- **`Group.end: { target: JumpToTarget, auto? }`** – where the reader goes after the chain's last group instead of the
  next group / chapter (`nextStep` kind `jump`; the URL via the VN's `buildJumpToUrl`). `auto` = at once, else the
  follow-up bar offers it with the countdown. Only read on a chain's last group.
- Stored like `renderer.params.fullWidth`: on the group itself (a `ref` group's in its scene document); the creator writes
  them to every language. Validation: `continues` on the chapter's last group = error; `end` on a group
  that continues = warning (ignored); a chain that isn't the chapter's start without a label on its first group =
  warning (unreachable); `end.target` is checked like a jumpTo target. The chapter check reads ref groups' scenes through
  `ValidateContext.resolveGroup` (the creator passes its loaded scenes).

## Language-independent media (2026-10-01, owner rule – PLAN.md RULES)

"Everything should be language-independent": media are shared by every language, text lives only in text fields, lettering
baked into an image is extracted into text and rendered by the UI in the reader's language.

**Covers are per SEASON (owner 2026-10-02: "It is per-season covers. Per toon I have no covers intended – I use the
avatar").** `season.cover` (art + lettering layout) is the cover; the toon is represented by its avatar. `cover` left
`TOON_NEUTRAL_FIELDS` / `ToonIndex` again (TS + `Library::TOON_NEUTRAL_FIELDS`, cache FORMAT 4); a `cover` in the toon entry
or the toon settings is a warning ("belongs to the season"); `Toon.cover` stays as a deprecated read-only fallback for
older per-language toon settings. Seasons have no neutral entity, so like `download` / `trailer` / `related_items` the
creator's cover editor writes the cover to every locale's season document (the validation warns when locales differ).
Reader: the toon page shows the cover of the season it presents (`season.cover`, else the old `toon.cover`); the desktop
hero's fallback background is that season's cover art. Data: catpope's seasons already had `cover` (the baked images, no
lettering) → only `toons.json` lost it (seed SQL: the `toon:catpope` statement, old timestamp); sandbox: `cover` moved from
its toons.json entry into its 3 `season1.json` (`make-sandbox.ts` writes it there). The history of the 2026-10-01 toon-entry
model below is kept for reference.

**(2026-10-01, superseded) Covers → the toon entry + rendered lettering.** `cover` joined `TOON_NEUTRAL_FIELDS` (type `Cover { square?, portrait?,
lettering? }`, `Toon.cover` optional now; schema `toon-index.schema.json`; a `cover` left in `toon-settings` is the usual
"belongs to the toon entry" warning). The library merges it like the other neutral fields (the entry wins; older data
without it in the entry keeps each language's cover – `LibraryTest::testCoverFromOlderToonSettings`).
- **Lettering model** (a layout since 2026-10-02, "Cover lettering layout" below): the artwork carries no text; `cover.lettering: true` = the UI sets the title over it: the toon's name
  (toon settings `title`) small on top, the season's title (season `title`) big in quotes beneath – `CoverArt` in
  `@scutoons/reader` (`cover/cover-art.svelte`): Bangers (`--font-lettering`), white fill, a heavy ink outline under the fill
  (`-webkit-text-stroke` + `paint-order`), a hard ink shadow, tilted −5° / skewed −4° like the hand-lettered catpope cover,
  sizes in `cqw` (the big line ≈ 12.5 % of the width, smaller for long titles so it stays inside ~84 %), real DOM text with
  `lang`. Default (no flag) = the image as it is – for artwork whose title is still baked in, so it never shows twice.
- **Where:** the viewer's toon page (phone/tablet cover, `toon.cover?.square`; no cover → no image); the desktop hero uses
  the bare artwork as its fallback background (it sets the title itself) and shows nothing when there is no image either.
- **catpope:** keeps `images/s1-cover-{square,portrait}.webp` (German "KATZENPAPST" / "DAS ERWACHEN" baked in), lettering
  off, until the owner delivers the lettering-free artwork (media requests below) – then `cover.square/portrait` → the
  `-textless` files and the flag flips (creator toon settings, "Same in every language" → Cover).
- **toptoy / yellowmouse:** their per-language covers `images/1x/<l>/s1-cover-*.jpg` never existed on the CDN → no cover
  now (requested lettering-free, below). Their draft toon pages (dev only, 404 publicly) show no broken cover any more.
- **Creator:** toon settings → "Same in every language" → **Cover** (`data-testid="toon-cover"`): `cover.square`,
  `cover.portrait`, the "Lettering" checkbox, a **preview** per language – "as readers see it" and, while the flag is off,
  "rendered title" (the `CoverArt` lettering over today's image with a soft blur behind it, `veil`, so the owner can judge
  the model before the clean art exists) – and the toon's open media requests. The covers left the per-language block.
- **Season covers** (`season.cover`, unused by the reader): seasons have no language-neutral entity – like `download`,
  `trailer`, `supporters` the creator writes the cover to every locale (one field) – so it stays in `season:<toon>:<n>:<l>`
  (catpope: the same two files in de + en). The cross-locale validation warns when a season's covers differ between
  languages, and `Cover.lettering` works there too. A real move needs a neutral season entity (owner decision; not built:
  keep the configuration small).
- **Migration:** seed `toons.json` (catpope + sandbox `cover`), the 8 `toon.json` lost it, `make-sandbox.ts`'s
  `neutralSettings` has it; local store: new published `toon:{catpope,sandbox}` + `toon-settings:{catpope,toptoy,yellowmouse,
  sandbox}:{de,en}` + `asset-requests` as the test user (backup `server/db/local.sqlite.bak-2026-10-01-covers`; every newest
  version was the published one). Library answers before/after (46 URLs, sorted JSON): identical except toptoy/yellowmouse's
  bundles, which lost only their non-existent `cover`. Seed SQL: only the 8 changed statements (`toon:catpope`, 6
  `toon-settings`, `asset-requests`; the old rows keep their timestamps), `characters-seed.mysql.sql`'s `asset-requests`
  statement equals it. Idempotent per key: a database that has these keys needs the new versions imported separately.

**Cover lettering layout (2026-10-02, owner: "a season cover image editor view for multi-language support").** The
boolean grew into a language-neutral layout, edited in the creator's cover editor (`/<toon>/season/<n>/cover`, DESIGN.md "Creator:
cover editor"):
- `Cover.lettering`: absent / `false` = the image as it is · `true` = **the default layout** (unchanged meaning; now drawn by
  the same line renderer, `defaultCoverLines`) · `{ square?: CoverLine[], portrait?: CoverLine[] }` = a layout per format
  (a format without lines → the default layout, `[]` → no lettering on that format). Types + helpers: `@scutoons/content`
  `cover.ts` (`COVER_*`, `coverLetteringOn`, `coverLines`, `coverLineText`, `defaultCoverLines`, `estimateCoverFit`).
- `CoverLine { text: 'toon' | 'season' | 'custom', custom?: { <locale>: text }, quotes?, x, y, size, maxWidth, rotate?, skew?,
  align?, color?, outline? }`: x / y = the centre of the line's box in % of the cover's width / height; `size` = font size in
  % of the cover's width (cqw); `maxWidth` = the box in % of the width; colours from the palette only (`COVER_COLORS`: paper,
  ink, cyan, purple, green, red = the design system's tokens; default paper fill, ink outline + shadow). **No text is baked
  into the layout:** `toon` = toon settings `title`, `season` = season `title` (per language), `custom` = the line's own text
  per language (a missing translation shows the first text there is; the validation warns per toon language).
- **Fit:** every line shrinks to its box in each language (`CoverArt` measures the real width; `estimateCoverFit` before
  that, ≈ 0.5 em per character); below 35 % it wraps inside the box. Checked with de / en / ko (sandbox): long German
  titles shrink, short ones keep their size.
- **Validation** (`checkCover` / `checkCoverLine`): an unknown text source, numbers outside `COVER_LIMITS`, non-palette
  colours, a bad `align` = errors; lines without the format's image, unknown formats, missing custom translations, `custom`
  on a title line = warnings. `lettering: true` stays valid. Schemas regenerated (`toon-index`, `season`); the server stores
  the object as it is (no PHP change).
- **Square only in the UI:** the reader shows `cover.square` only (toon page; the desktop hero uses the bare art); nothing
  reads `cover.portrait` (owner 2026-10-02: "portrait I think we are not using"). The type keeps `portrait` optional and
  the layout per format; the editor edits `square`.
- **Which cover:** the season's (`season.cover`, owner 2026-10-02). The editor is `/<toon>/season/<n>/cover`; it writes
  `cover.lettering` and `cover.square` to the language-neutral season document (since 2026-10-06; before: every
locale's season document). The season editor
  shows the cover (thumbnail + lettering state) and links to it; its two path fields are gone. The toon settings have no
  cover any more.
- **catpope** stays as it is (lettering off, the baked German cover) until the owner switches it on with the text-free art
  (`s1/s1-cover-square-textless.webp`, delivered 2026-10-02, upload in admin → Assets): Season 1 → "Open the cover
  editor" → "Art" → choose
  it, "Default" (or an own layout), Save draft, Publish. The default layout sits where the hand lettering was (the toon's
  name at y 19 %, the season's title at y 31 %).
- Open (owner): which season the toon page presents once there are several (today season 1); text sources (toon / season / own text – "Staffel 1" is an
  own text per language; no number source: keep the configuration small); palette colours only (6) or more.

**Validation: locale segments in media paths.** `mediaLocaleSegment` / `mediaPathsWithLocale` (`@scutoons/content`
`media.ts`): any media path (image, video, audio extension) with a locale folder (`de/`, `en-US/` …) or a locale suffix
(`_de.`, `-en.`, `.de.`) is a warning in every entity (`validateFile` for chapters, scenes, seasons, toon settings, the toon
entry; `validateCharacters`, `validateCredits`, `validatePeople`). The seed has none left.

**Media requests.** Asset requests (`asset-requests`, the creator's "Wanted images") can be toon media now: `view: "media"`
(`MEDIA_VIEW`), `toon`, `path` from the CDN root, `size`, `note`; character/persona/look/mood empty. `mediaRequests()`,
listed under "Toon media" in Wanted images (+ copy as text/CSV) and on the toon settings page; closed when the file exists.
Open (owner 2026-10-01):

| id | path | size | why |
|---|---|---|---|
| `catpope-s1-cover-square-textless` | `catpope/s1/s1-cover-square-textless.webp` | 1080 × 1080 | the cover without "KATZENPAPST" / "DAS ERWACHEN" → `cover.lettering` |
| `catpope-s1-cover-portrait-textless` | `catpope/images/s1-cover-portrait-textless.webp` | 1080 × 1920 | the same, portrait |
| `toptoy-s1-cover-square` / `-portrait` | `toptoy/images/s1-cover-{square,portrait}.webp` | 1080² / 1080 × 1920 | no cover exists (the old per-language paths never did) |
| `yellowmouse-s1-cover-square` / `-portrait` | `yellowmouse/images/s1-cover-{square,portrait}.webp` | 1080² / 1080 × 1920 | the same; the mouse stays unrevealed |
| `catpope-s1c2-slice-1-textless` | `catpope/images/webtoon/s1c2/s1c2-slice-1-textless.jpg` | 390 × 625 | chapter 2 slice 1 without the baked SFX "FLATTER FLATTER" → an onomatopoeia text box (outlined lettering, revealed by an onomatopoeia event) |

New files get new names (PLATFORM.md "CDN": a changed file gets a new name).

### Audit (2026-10-01): language baked into media

Every file referenced by the seed (+ the apps' static images) was looked at. Slice art was not changed.

| Path (CDN root unless noted) | Text baked in | Lang | Used by | Recommendation |
|---|---|---|---|---|
| `catpope/images/s1-cover-square.webp`, `s1-cover-portrait.webp` (+ unused `.jpg` twins) | "KATZENPAPST", "DAS ERWACHEN" | de | `toon:catpope` + `season:catpope:1:*` `cover`, sandbox cover | **new artwork** (requested); lettering via `cover.lettering` |
| `toptoy/images/1x/{de,en}/s1-cover-*.jpg`, `yellowmouse/…` | – (never existed) | de/en paths | were `toon-settings` covers | removed; **new artwork** (requested), lettering-free |
| `catpope/images/webtoon/s1c2/s1c2-slice-1.webp` (+ `.jpg`) | SFX "FLATTER FLATTER" (in no text field) | de | chapter 2, slice 1 | **extract** to an onomatopoeia text box (`dialog` + a `showTextBox` event); clean art requested. Slices 2–22 are text-free |
| `catpope/images/webtoon/s1c1/*` (source `webtoon/de/S1C1 - Slice N.jpg`) | speech, captions | de | chapter 1 | **extracted** already (`dialog` + `mask`, MEMORY "Chapter 1 text extraction"); `mask` is deprecated (owner 2026-10-03): re-import the slice backgrounds without the text boxes, keep the dialog data in the DB |
| `catpope/images/characters/{catpope,boy,crows,mountain-tiger,cat,plf}/*.svg` | placeholder names ("Katzenpapst", "Junge", "Krähen", "Gebirgs Tiger", "Katze"/"Cat", "PLF", "Character Placeholder") | de (en) | `characters` registry (old `image` / view paths of these characters) | **new artwork** via the characters' own wanted images; until then the on-brand "?" substitutes are a better fallback than named placeholders (owner decision) |
| `catpope/images/s1-trailer-poster.webp`, `images/s1c0-thumbnail.webp`, `images/webtoon/s1c0/s1c0-slice-1.webp`, `videos/s1-trailer-loop.*`, `videos/roar-little-kitty.*` | "ROAR LITTLE KITTY" (title card) | en (song title) | season `trailer`, chapter 0 thumbnail + player slice | **keep** – the song's name, the same in every language; a clean poster/thumbnail frame is optional |
| `catpope/images/s1-music-vinyl.webp`, `catpope/items/awakening_*.png`, `dago_*`, `catpope/merch/*.jpg` | SCUTOONS + katakana on the obi, "MEMBERSHIP", test-press handwriting, garment prints | brand | season `download.image`, items, products | **keep** – product design / photos of real products |
| `catpope/images/supporters/{ske,kuenstlerhof-frohnau}.svg` | "SKE", "KF" logo letterforms | proper names | season `supporters` | **keep** (third-party logos) |
| `viewer/static/SCUTOONS-logo*.svg` (+ admin, creator, embassy, shop copies), favicons, app icons | "SCUTOONS" + スクトゥーンズ, "S" monogram | brand | every app | **keep** (brand mark) |
| key visuals `s1c1-key-visual.*` (+ video), `s1c2-key-visual.*`; thumbnails `s1c1–s1c5`, `s1s1–s1s6`; `map/map-layer-terrain.svg`; `vectors/*`; item cut-outs (camera, cucumber, hoodie, amulet); toon avatars; sampled character portraits / A-poses | none | – | chapters, map, items, registry | nothing to do |

Not found: OG / social images (no `og:image` / `twitter:image` in any app). Side findings: `viewer/static/favicon.svg` (used by
admin, creator, embassy via `app.html`) is still the default Svelte logo; seed paths missing on the CDN besides the covers:
`catpope/images/3x/s1c3..5-key-visual.jpg` (chapters 3–5, known), `characters/boy/boy_full.svg`, `characters/crows/crows_portrait.svg`.
