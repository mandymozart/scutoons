# DATA — everything from the API, stored in the database (widened P4)

Written 2026-09-30 (branch `platform`). Companion docs: [PLATFORM.md](PLATFORM.md) ("Data", domain architecture, CDN),
[PERFORMANCE.md](PERFORMANCE.md) (P4), [ITEMS.md](ITEMS.md) (API contract), [PLAN.md](PLAN.md) (rules, release).

**Owner decision (2026-09-30):** "Switch the entire loading process to not file-based. It has to be served via the API
and stored in the database. [...] a real service layer that can cache and get chapters or other entities on demand
rather than everything at once. For a PWA we will need smart pre-loading or a save-to-device concept."

## 1. Today

| Data | Where it is read from today |
|---|---|
| toon index (`toons.json`) | viewer static only (creator: static + store draft `<toon>/_/index.json`) |
| toon, season, chapter, VN scene files | content store (published) → static fallback, **one file per request** (layout: season + every chapter file) |
| map (`locations.json`, `routes.json`) | viewer static only, per locale |
| collectables (`collectables.json`) | **removed** (owner 2026-09-30): merged into items (`config/items.json`); VN variables stay story logic, the VN → item grant bridge does the collecting |
| characters | store (`scutoons/_/characters.json` + `<toon>/_/cast.json`) → legacy static `<toon>/<l>/characters.json` |
| credits | store (`<toon>/_/credits.json` + `scutoons/_/{people,roles}.json`) → legacy static `<toon>/<l>/credits.json` |
| events | viewer static `events.json` |
| products, featured products | server `config/*.json` via `/api/products/` |
| items, editions, permissions, QR toons | server `config/*.json` (`/api/items/`, `/api/editions/`, `/api/toons/`; permissions never served) |
| codes, users, inventory, progress, avatar, settings | relational tables (unchanged by this plan) |

## 2. Storage decision: documents, keyed by the path they already have

**One store: `content_versions`** (versions, draft/published, history, restore – exists, works, seeded on production on
Oct 3). Every entity is a JSON document in the configuration format (`@scutoons/content` schemas = the import/export
format, unchanged). No new tables, no schema change:

- **The key stays `(scope, locale, path)`** – `scope` = toon id or the platform pseudo-toon `scutoons`, `locale` = `de`/`en`
  or `_` (language-neutral), `path` = the file name of the export format. The path already *is* the entity key
  (`chapter2.json` = chapter 2, `chapter2/scene-14.json` = one of its scenes); turning it into another key would add a
  mapping to the import/export and the creator's write path without a query that needs it. The server maps entity ↔
  path in one class (`Domain\Library\Keys`).
- **No relational tables for content**: every read is "one entity (+ its children) of one toon in one locale". The
  season overview needs the list fields of a season's chapters – composed on read and cached (§4), no projection table.

| Entity | Document (scope/locale/path) | Editor | New in the store |
|---|---|---|---|
| toon index entry | `<toon>/_/index.json` (`ToonIndex`) | creator toon page | seeded from `toons.json` (split per toon) |
| toon | `<toon>/<l>/toon.json` | creator | – |
| season N | `<toon>/<l>/season<N>.json` | creator | – |
| chapter N | `<toon>/<l>/chapter<N>.json` | creator | – |
| VN scene | `<toon>/<l>/chapter<N>/<name>.json` | creator | – |
| map | `<toon>/<l>/locations.json`, `routes.json` | none yet (import / raw JSON) | **yes** |
| cast | `<toon>/_/cast.json` | creator | – |
| characters, moods, asset requests | `scutoons/_/{characters,moods,asset-requests}.json` | creator | – |
| credits, people, roles | `<toon>/_/credits.json`, `scutoons/_/{people,roles}.json` | creator | – |
| events | `scutoons/_/events.json` | none yet | **yes** |
| products, featured | `scutoons/_/{products,featured-products}.json` | none yet | after the release (step B2) |
| items, editions | `scutoons/_/{items,editions}.json` | admin later | after the release (step B2) |
| permissions | `scutoons/_/permissions.json` – **private** (never served by any public GET) | admin later | after the release (step B2) |
| legacy `<toon>/<l>/characters.json`, `<toon>/<l>/credits.json` | not imported – replaced by the registry / credits store; deleted with the static files | – | – |

The server's `config/*.json` files (items, editions, products, …) stay as a fallback until B2 moves them; the QR toons
list (`/api/toons/`, `config/toons.json`) is later derived from the index documents (same bytes, compare test first).

## 3. API service layer (`/api/library/…`, public GETs)

Base as everywhere (`VITE_API_URL`: `…/api` on api.scutoons.com, qr during the transition). `/api/toons/` is taken by the
QR toons list (contract), so reading entities lives under `/api/library/`. The old `/api/content/*` endpoints stay
unchanged (the creator's write path and the current viewer).

| GET | Answer (types from `@scutoons/content`) |
|---|---|
| `/library/toons` | `ToonIndex[]` – published entries (`status` published) |
| `/library/toons/{toon}?locale=` | `{ entry: ToonIndex, toon: Toon, seasons: SeasonOverview[], locations, routes }` – everything the toon layout needs in **one** request |
| `/library/toons/{toon}/seasons/{n}?locale=` | `SeasonOverview = { number, season: Season, chapters: ChapterSummary[] }`; `ChapterSummary` = the chapter file without `groups`/`audio`/`status` + `type`, `number`, `status` (from the season) |
| `/library/toons/{toon}/chapters/{n}?locale=` | `Chapter` with every `ref` group replaced by its scene file (P4: one request per chapter; a missing scene keeps its `ref` group, header `X-Content-Missing`) |
| `/library/toons/{toon}/cast[?chapter=N]` | `{ registry: CharacterRegistry, cast: ToonCast }` – the registry reduced to the toon's cast (or to the characters a chapter's events reference); the client resolves with `resolveCast` (shared code, no PHP port) |
| `/library/toons/{toon}/credits` | `{ credits: ToonCredits, people: PeopleRegistry (referenced people only), roles: RoleVocabulary }`; client: `resolveCredits` |
| `/events` | `Event[]` (the stored document) |

- **Locale**: `?locale=` (query, not `Accept-Language` → cacheable URLs, no `Vary`). Default and fallback = the toon's
  `sourceLanguage`. Per entity: requested locale if the toon lists it and the document exists there, else the source
  language; the answer says which one: `Content-Language`, `X-Content-Fallback: 1`. Neutral documents ignore it.
- **Visibility** (moves from the viewer into the API, same rules): the season's `sequence[].status` of the served
  locale decides. `coming-soon` → `403 coming_soon { chapter: ChapterSummary }`, `draft` / not listed →
  `403 not_public`, draft toon (index or toon.json) → `404 not_found`. Server setting `CONTENT_SHOW_DRAFTS` (default
  off, on in `config.local.php`) = today's `import.meta.env.DEV` behaviour for local development.
- **Drafts (creator preview)**: `?draft=1` with a `creator` session → the newest version of every document, no status
  gate, `Cache-Control: private, no-store`; without the role `401 unauthorized` / `403 forbidden`. The creator's
  preview page switches to these endpoints (no own composition).
- **Membership gate (later, after the release)**: chapter content will need the `access_token` (ITEMS.md) unless
  `isPreview`; then those answers become `private`. The endpoint is the single place to add it.
- **HTTP caching**: public answers `Cache-Control: public, max-age=60` + `ETag` (= the fingerprint below; `304` on
  `If-None-Match`); CORS like `/products/` (simple GET, no preflight). Errors: `{ status:"error", error, message, … }`.
- **Server cache**: documents are immutable per version id. Each request reads the ids of the published (or newest)
  versions it needs (one indexed query), the fingerprint = hash of endpoint + those ids; the composed answer is cached
  under the fingerprint (APCu if present, else files in `storage/cache/library/`, pruned by age). Publishing creates a
  new fingerprint → **invalidation on publish without any invalidation code**, no stale reads.
- **Pagination**: none needed (toons < 10, a season < 50 chapters, events < 100); lists are complete arrays.

## 4. Migration

1. `import-content` learns the new kinds: `toons.json` → per-toon `_/index.json`, `events.json` → `scutoons/_/events.json`,
   `<toon>/<l>/{locations,routes}.json`. Idempotent as today, `--update`, `--sql` for production
   (phpMyAdmin), dev-only toons (`sandbox`) stay out of `--sql`.
2. **Compare** (`tests/entities-compare.php`, part of `composer test`): a fresh DB seeded from `viewer/static/api` →
   every library endpoint of every toon/locale/chapter == the same composition done from the static files (JSON-equal).
   On production the same script runs read-only against the API with `--report` (differences = creator edits).
3. Frontends switch (Phase C), then `viewer/static/api/**`, `getStatic`, `STATIC_API`, the store→static fallbacks and
   the legacy characters/credits branches are deleted.
4. `bin/console export-content <dir>` writes the published (or `--draft`) documents back into the file layout – the
   import/export format, a backup, and the seed for new environments.

**Cutover order** (production): the released viewer build carries its own static files and keeps reading them until a
new viewer build is live, so nothing breaks in between. (1) API deploy with the library endpoints → (2) seed the new
kinds (`import-content --sql`, only new files) → (3) `entities-compare --report` against production → (4) frontends
(viewer, creator) with the library client → (5) the next release deletes the static files. Old `/api/content/*` stays
(creator writes).

## 5. Creator / admin write paths

Unchanged: `PUT /content/<scope>/<locale>/<path>` (draft, `base` conflict check), `publish`, `unpublish`, `restore`,
`versions`, `index` – they already are the entity write API. The creator stops falling back to static files (new files
start empty) and lists toons from `/library/toons?draft=1`. Admin `/admin/content` stays as is. Documents without an
editor (events, map; later products/items/editions/permissions) are written by `import-content --update`
or the creator's raw JSON editor until an editor exists; private documents are only readable with `draft=1` + role.

**No JSON files in the UI (owner, 2026-09-30).** Once the API serves everything from the database, the creator and
admin never show file names (`chapter1.json`, `toon.json`, `season1.json`), `_` locale chips, `v76` path chips or
"store file" wording – only entities: "Chapter 1", "Season 1", "Toon settings", "Characters", "Credits", "Draft v71 /
Published v70". The path stays the storage key on the server (§2), hidden behind one mapping:
- `@scutoons/content` gets `EntityKey = { kind, toon, locale?, number?, scene? }` (kinds: `toon-index`, `toon`,
  `season`, `chapter`, `scene`, `map-locations`, `map-routes`, `cast`, `credits`, `characters`,
  `moods`, `asset-requests`, `people`, `roles`, `events`) with `entityKey(scope, locale, path)` ↔
  `entityPath(key)`; `@scutoons/api`'s content functions take an `EntityKey`.
- The creator's entity → label helper (built by the UI agent) switches from paths to `EntityKey` in Phase C; editor
  documents (`ContentDoc`) are keyed by `EntityKey`, and the remaining path-based internals (path regexes in
  components, `getStatic`, path chips) go.
- JSON may be named only in the import/export UI (the interchange format).

## 6. PWA concept (design only)

- **Service Worker** (SvelteKit `src/service-worker.ts`), caches per type:
  | What | Strategy | Cache |
  |---|---|---|
  | app build + fonts | precache per build version, cache-first; old versions dropped on `activate` | `app-<version>` |
  | library JSON | stale-while-revalidate with `ETag` (304 = no body) | `library` |
  | slices, posters, portraits (immutable names, PLATFORM.md CDN rule) | cache-first, LRU ≈ 150 MB | `media` |
  | audio, video | network (range requests); only saved chapters serve them from cache (SW answers ranges with 206) | `offline-*` |
- **Smart preloading** (idle time, not with `saveData` / 2G): after the first 2 slices + audio start → the next
  chapter's JSON (library endpoint), its first 3 slices, and the portraits of characters joining in the next scenes of
  the current chapter (from the inlined groups + `resolveExpression`).
- **Save to device** (chapter or season): `GET /library/toons/{t}/chapters/{n}/assets?locale=` lists every media path
  (slices, video, poster, key visual, music, samples, cast portraits) with sizes → the SW downloads them into
  `offline-<toon>-<locale>-c<n>`, a record in IndexedDB (fingerprint, bytes, saved_at). Before: `storage.estimate()`
  and `storage.persist()`; the UI shows the size. Update: on the next online open, a new ETag → download only the
  changed paths. Remove per chapter / season.
- **Membership**: saving is offered only for chapters the device may read; the SW stores the access `expires_at` and
  stops serving gated offline content after it or after a `410` on the 6-hourly `/access/` re-check (then the gated
  offline caches are deleted). Preview chapters stay.

## 7. Phases, release, risks

| Phase | What | When |
|---|---|---|
| A | this design | ✅ 2026-09-30 |
| B | server: library endpoints, cache, import of the new kinds, `export-content`, PHPUnit + `entities-smoke` + `entities-compare` | ✅ 2026-09-30 (below) – additive, the frontends don't call it; safe to ship with the release |
| C | viewer, creator, admin, shop on the library endpoints; static files + fallbacks removed; `EntityKey` instead of paths in the creator/admin (no JSON file names in the UI, §5); measured with `scripts/perf/measure.mjs` + `scripts/vn/vn-check.mjs` | **after the release** (coordinator's go; the VN agent owns the viewer loaders until then) |
| B2 | products/featured, items/editions, permissions (private) into the store, services read store → config fallback, QR toons list derived | after C |
| D | PWA (§6) | after C |

**Release 2026-10-04:** nothing of this plan is required for it. Oct 3 runs `import-content --sql` as planned (with B
merged it also seeds events/map/index – harmless, nothing reads them yet). The release viewer keeps
store → static.

**Risks**
- No static fallback after C: API down = reader down → `/api/health/` monitoring; the PWA cache (D) covers repeat readers.
- Visibility moves to the server: a wrong season status now hides a chapter for every client → compare test + the
  `403` answers carry the reason.
- Locale fallback: an `en` reader may get a `de` chapter (flagged) instead of today's error page.
- all-inkl may lack APCu → file cache; `storage/cache/` must be writable and excluded from deploys (like `storage/`).
- Store/static drift: after C the store is the only source; `export-content` is the backup/diff tool.
- Draft leakage: draft answers are `no-store` and never enter the server cache.

## Phase B — result (2026-09-30)

| Where | What |
|---|---|
| `server/src/Domain/Library/` | `Library` (compositions, gates, locale fallback, cache key), `Snapshot` (version ids of the scopes, lazy documents, fingerprint), `Context`, `Answer` |
| `server/src/Http/Controllers/LibraryController.php` | the 7 GET routes (`config/routes.php`), headers, 304, `?draft=1` role check |
| `server/src/Infrastructure/Cache/` | `Cache` + `ApcuCache`, `FileCache` (atomic writes, pruned by age), `NullCache`; `config/container.php` picks by `CACHE_DRIVER` |
| settings | `CACHE_DRIVER` (auto), `CACHE_PATH` (`storage/cache`, gitignored, deploy-protected like all of `storage/`), `CONTENT_SHOW_DRAFTS` (0; `1` in the local `config.local.php`) |
| `ImportContentCommand` | `sources()`: `toons.json` → `<toon>/_/index.json` (file order), `events.json` → `scutoons/_/events.json`, `<toon>/<l>/{locations,routes}.json` (collectables: removed, merged into items); `--only` also matches the source file name |
| `ExportContentCommand` | `export-content <dir> [--toon] [--draft] [--db]` – the store back into the file layout; reads back to the same documents (unit test) |
| tests | PHPUnit `LibraryTest` (9) + `LibraryApiTest` (3); `tests/entities-smoke.php` (32 checks, through the creator's write path); `tests/entities-compare.php` (35 checks: every toon/locale/chapter, cast, credits, events of a fresh DB seeded from `viewer/static/api` + `shared/content/seed` == the static composition); both in `run-contract.php` (API on `<port+6>` with the fresh DB) |

Decisions made while building: a season entry whose chapter file is missing is listed with empty fields (+
`X-Content-Missing`) instead of failing the whole toon; the chapter summary drops `groups` + `audio` only (everything
else of the chapter file stays, the viewer's `LoadedSequenceItem` minus the heavy fields); the cast answer is
language-neutral (resolution stays in `resolveCast`), `?chapter=` walks every `character` field of the composed chapter.

Local dev store: backup `server/db/local.sqlite.bak-2026-09-30-library`, then `import-content … --only=toons.json,
events.json,locations.json,routes.json,collectables.json` (15 new documents; the 4 collectables documents were deleted again when collectables were merged into items) and `--update --only=toons.json
--toon=catpope` (catpope's index entry had drafts only). toptoy's published index entry (source `en`, a creator
decision) was left alone – the store wins over `toons.json`.

