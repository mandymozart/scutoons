# CATALOG – the catalog in the database (CUTOVER 10.29, done 2026-10-04)

How it is now: the tables, loaders, admin API and admin UI (Part 3). **Since 2026-10-12 items, recipes, albums, products
and editions are edited in the creator** (every entry its own document, agents/research/CATALOG-IN-CREATOR.md); the
admin's catalog page keeps "Shop & codes" (toons, permissions, code types, meta) and refuses writes of the five kinds. The plan and status sections below are the
history of the move (from `server/config/*.json`, gone since 10.30); the one-off production steps ran 2026-10-05.

Owner 2026-10-04: "cant we just create the db table for this already? it is not like this is going to be hard coded for
ever … lets just do it properly", "we have to do them all now", "a basic ui to the admin with file upload where possible
and replace the usage". Beta for friends – no release pressure.

## What moves where

| Today (server/config) | Goes to | Notes |
|---|---|---|
| `items.json` `items` | table `catalog_items` | one row per item, the definition as JSON in today's shape (`data`), `toon`, `position`; the `item_respawn` setting folds into the item's `respawn` and is retired |
| `items.json` `recipes` | `catalog_recipes` | inputs / output |
| `items.json` `albums` | `catalog_albums` | tracks with their media |
| `items.json` `slots` | **code** (PHP + `@scutoons/content` constants/types) | "slots are merely enums for an avatar model" |
| `items.json` `stats` | **code** (type definitions + constants) | testo / appetite: baseline, half-life |
| `products.json` | `catalog_products` | relation to digital items (`item_id`) and editions; shop meta (`currency`, `checked_at`, `source`) as a row or setting |
| `editions.json` | `catalog_editions` | |
| `featured-products.json` (+ setting `featured_products`) | `catalog_featured` | ordered `{ product, variant? }` |
| `permissions.json` | `catalog_permissions` + `catalog_code_types` | server-only (security): never served to readers beyond what is served today |
| `toons.json` | `toons` | **created by admins only** in the admin UI (id, name, redirectUrl); the creator then fills content and meta (`toon:<id>` entities). The sandbox toon keeps working |
| – | `catalog_history` | every change: kind, id, before / after JSON, by, at – an admin can see and undo |

Rules:
- ~~The JSON files become seeds for empty / local databases~~ – since 10.30 (2026-10-05) production is the only source:
  local dev loads production's tables (`server/db/prod-seed.sql`, `db:prod-seed`); the files are the committed test
  fixture `server/tests/fixtures/catalog/`. The production migration carries the 2026-10-04 data as INSERTs (FROZEN;
  owner ran it in phpMyAdmin).
- Every consumer keeps its output: `GET /api/items/`, `/api/products/`, the admin catalog response, the QR toons –
  same JSON shape, so the viewer / shop / qr / embassy don't change.
- The asset registry indexes the catalog rows as uses (it reads the rows),
  so the delete check, orphans, relink / rename keep working (relink rewrites rows instead of files).
- Images are WebP (owner: "webp is everywhere the format to go", AVIF later via a server job); uploads go to
  `<toon>/items/<item-id>[_<variant>].webp`; a file already there is replaced in place (new asset version, CDN purged).

## Parts

1. **Database + server** (branch `catalog/db`): tables (mysql + sqlite migrations, schema files), the seed import, the
   loaders read the database, slots / stats as code, admin CRUD API (list / get / put / delete / reorder per kind,
   validation incl. `validateCatalog`, history + undo), toon creation API, AssetUsages on rows, tests.
2. **Shared media components** (branch `shared/media`): the creator's asset picker sheet, media slot, media-variants
   slot and `media-upload.ts` (WebP + upload-or-replace) move into a shared package both creator and admin use.
3. **Admin UI** (after 1 + 2): basic pages – Toons (create), Items (list, form, picture slot + variants, bulk
   "Import images …" by `<item-id>[_<variant>].png`), Recipes, Albums (tracks with audio slots), Products (item
   relation, pictures), Editions, Featured, Permissions / code types; history per entry.

## Status

- 2026-10-04: plan written; parts 1 and 2 started.
- 2026-10-04: **part 2 done** (branch `shared/media`): new package `@scutoons/media` (shared/media) with the asset
  picker sheet, media slot, media-variants slot, `toWebp` / `uploadOrReplace` and the `{ src, variants }` helpers; own
  de/en texts; creator call sites switched, creator copies deleted; creator and admin call `setupMedia({ assets, cdn,
  locale })` in their root layout (both render `DialogHost`). Ready for part 3 – COMPONENTS.md "Shared media components".
- 2026-10-04: **Part 1 built** (branch `catalog/db`). Details below.
- 2026-10-05: production migration has run (owner check: all 11 tables, the old settings folded).
- 10.30 (2026-10-05): files are no data source any more – test fixture `server/tests/fixtures/catalog/` (`CatalogSeed`
  = test-only loader); `catalog:seed` / `catalog:export-sql`, the file fallback (`fallback` field, 503
  `migration_pending`), the featured default list and the respawn endpoints (`/admin/catalog/respawn`, "reset") removed;
  SQLite migrations gone. Local data = production's (`npm run dev:db -- seed`). Part 1 below is the 2026-10-04 state.

### Part 1 – what is built

**Tables** (migration `2026-10-04-catalog`, `server/migrations/mysql/` – the sqlite one is gone since 10.30; utf8mb4, `utf8mb4_bin` = case-sensitive
ids): `catalog_items` (+ `toon`, `edition_id`), `catalog_recipes`, `catalog_albums` (+ `toon`), `catalog_products`
(+ `toon`, `item_id`, `edition_id`), `catalog_editions` (+ `toon`), `catalog_featured` (+ `product_id`, `variant_id`; id
`<product>[:<variant>]`), `catalog_permissions` (data = the description string), `catalog_code_types` (id `default` =
the old `default`), `toons`, `catalog_meta` (the files' top-level fields: `items` `{ version }`, `products` `{ version,
currency, checked_at, source }`, `permissions` `{ version }` – not in the original list, needed for byte-equal answers),
`catalog_history` (kind, entry_id, action create/update/delete/reorder/replace, before_json / after_json, position,
undo_of, note, changed_by, changed_at). Every row: `id`, `position`, `data` (LONGTEXT + JSON_VALID – not MySQL's JSON
type, which reorders keys), `updated_at`, `updated_by`. Relations are real columns kept in step with `data`; integrity
is checked by the API (no FOREIGN KEYs: undo / reorder stay simple).
- The migration carries **today's data** (one INSERT per table, each only while the table is empty → re-running never
  brings back deleted entries), generated from the files (the generator `catalog:export-sql` and its CatalogTest check
  were removed in 10.30; the block is FROZEN). It **folds and deletes** the settings `item_respawn` (overrides →
  the items' `respawn`) and `featured_products` (the stored list → `catalog_featured`, before the seed).
- `migrations/schema/*` (the base schema of a fresh DB) is NOT changed: it has never had the later tables; a fresh
  database gets the base schema and then every migration, this one included.
- ~~`catalog:seed`~~ and ~~the file fallback without the tables (`503 migration_pending`)~~ – removed 2026-10-05
  (10.30); a local DB gets production's rows from `npm run dev:db -- seed`.

**Loaders** (`server/src/Domain/Catalog/`): `CatalogStore` (rows per kind, the only reader of the tables), `Catalog`
(`data()` = `{ version, slots, items, recipes, albums, stats }` as before), `Editions`, `Products` (catalogJson /
featuredJson keep objects and key order), `FeaturedProducts`, `Toons`, `Access\Permissions::map()` – APIs unchanged,
the ~25 consumers untouched. `ItemRespawn` reads the item rows (`apply()` is a no-op now); its admin update is an item
update (history, note `respawn`). (Its admin endpoints with "file" = the seed's rule and `reset` were removed in 10.30;
respawn is edited in the item form.)
**slots / stats are code:** `Domain\Avatar\AvatarSlots::ALL`, `Domain\Reader\ReaderStats::DEFINITIONS` =
`@scutoons/content` `AVATAR_SLOTS`, `READER_STATS` (CatalogTest compares them); removed from `items.json`;
`catalogFromSeed()` (content) adds them back for tests / scripts reading the fixture file; `validateCatalog` falls back to
them.

**Admin API** (role admin; docs/API.md "Admin API → Catalog"): `GET /api/admin/catalog/<kind>`, `GET|PUT|DELETE
/api/admin/catalog/<kind>/<id>` (`PUT { data }` creates 201 / updates 200; `422 invalid_catalog { problems }`, `409
in_use { uses }`), `POST /api/admin/catalog/<kind>/reorder { ids }`, `GET /api/admin/catalog/history?kind=&id=&limit=`,
`POST /api/admin/catalog/history/<n>/undo { force? }` (`409 changed_since`). Kinds: items, recipes, albums, products,
editions, featured, permissions, code-types, toons, meta. The old `/catalog`, `/featured` keep working (`/catalog/respawn`
removed in 10.30; `/featured` has no default list since then – PUT needs a list, `[]` = nothing featured).
Validation = `CatalogValidator` (the essentials of validateCatalog / validateAlbums; wear-vs-registry stays client-side).
**Toons:** `PUT /api/admin/catalog/toons/<id> { data: { id, name, redirectUrl } }` – the id must be a content toon id
(`[a-z0-9_-]`); creating also stores a draft content index entry `toon:<id>` (names from `name`, de + en) when none
exists, so the creator lists the toon and creates its settings on first open. The sandbox stays content-only (it never
was a QR toon). A toon with content / catalog entries can't be deleted.

**Assets:** the usage index reads the rows: source `catalog`, ref `<kind>:<id>` (`item:camera`, `product:dago_hoodie`,
`album:awakening`), field = the pointer in `data`; every catalog write re-indexes; relink / rename update the rows through
the admin path (validated, history; a refused one is reported as `manual`).

**Tests:** `CatalogTest` (tables = fixture, the folds, constants vs TS, validation, history / undo, toons, respawn /
featured), `CatalogApiTest` (public answers = fixture, admin CRUD, toons), AssetsTest / AppTest adapted (10.30: MariaDB
only; the export / fallback tests and the contract suites are gone).

### Production (owner) – history: ran 2026-10-05

1. phpMyAdmin → database **d0487143** (select it first) → tab **SQL** → paste the whole
   `server/migrations/mysql/2026-10-04-catalog.mysql.sql` → Go. Before or after the deploy of `catalog/db` (either order
   is safe; running it twice changes nothing).
2. Check (same SQL tab):
   ```sql
   SELECT 'items' k, COUNT(*) n FROM catalog_items UNION ALL SELECT 'recipes', COUNT(*) FROM catalog_recipes
   UNION ALL SELECT 'albums', COUNT(*) FROM catalog_albums UNION ALL SELECT 'products', COUNT(*) FROM catalog_products
   UNION ALL SELECT 'editions', COUNT(*) FROM catalog_editions UNION ALL SELECT 'featured', COUNT(*) FROM catalog_featured
   UNION ALL SELECT 'permissions', COUNT(*) FROM catalog_permissions UNION ALL SELECT 'code_types', COUNT(*) FROM catalog_code_types
   UNION ALL SELECT 'toons', COUNT(*) FROM toons UNION ALL SELECT 'meta', COUNT(*) FROM catalog_meta
   UNION ALL SELECT 'old settings left', COUNT(*) FROM settings WHERE `key` IN ('item_respawn', 'featured_products');
   ```
   Expected: items 27, recipes 5, albums 1, products 6, editions 3, featured = the landing page's list (2 if none was
   chosen in the admin), permissions 1, code_types 6, toons 3, meta 3, old settings left 0.
3. After the deploy: `https://api.scutoons.com/api/admin/catalog/items` (signed in as admin) answers `"fallback": false`
   (that field is gone since 10.30).

### Open / decided differently

- ~~An empty featured list comes back as the default list~~ – 10.30: no default list; `[]` = nothing featured.
- Undoing a toon's creation is refused while its starter content entry exists (`in_use`: a toon with content is never
  deleted by the API).
- Part 3 (admin UI) uses the endpoints above through `@scutoons/api` `createAdminApi` (`catalogList` … `catalogUndo`).

### Part 3 – the admin UI (branch `admin/catalog`, 2026-10-04)

**Where:** `/catalog/<kind>` (`admin/src/routes/catalog/[kind]/+page.svelte`), one tab per kind: items, recipes, albums,
products, editions, toons, permissions, code types, meta. `/catalog` redirects to items (the old `?tab=items|editions|
toons|permissions` links land on that tab). The sidebar keeps one "Catalog" entry; Ctrl K has "Catalog: <kind>" per tab
plus "New Item" / "New Toon".

**One scaffold** (`admin/src/lib/components/catalog/catalog-editor.svelte`) + one definition per kind
(`admin/src/lib/catalog/kinds.ts`: label, list columns, a new entry's data, the keys the form shows, id rule, what the
kind allows) + one form component per kind (`…/catalog/<kind>-form.svelte`):
- list in order (DataTable); round ↑ / ↓ per row = `catalogReorder` with the whole order (no drag: less UI, one way);
- a click opens the entry next to the list (SplitView: the admin's right dock – drags wider, folds to a rail; covers the
  page on phones; agents/DOCKING.md), URL `?open=<id>`; "New …" = `?new=1`
  with an id field (checked against the server's pattern and the existing ids before saving); leaving an edited entry
  asks first ("Discard the changes?");
- Save / Delete in the entry's header. 422 `invalid_catalog`: the problems as a list above the form, and each problem
  that names a field (`problemFor`) under that field. Delete: in-page `confirmDialog`; 409 `in_use`: the uses listed
  above the form;
- "Advanced" (items, albums, products, editions): the keys the form does not show (skills, effects, effect, wear,
  grantable, default_worn, plays …) as JSON, applied while typing once valid – so nothing is uneditable and nothing is
  shown twice. Toons, permissions, code types, meta and recipes have no Advanced (their forms cover the whole shape);
- history: per entry under the form (a fold), per kind behind "History" in the ActionBar (a drawer; also reorders);
  each row: action, #, time, by, note, the changed top-level fields, "Undo". 409 `changed_since` → in-page "Changed
  again since" → "Undo anyway" (= `force`).
- `admin/src/lib/catalog.ts` `invalidateCatalog()` after every write (put / delete / reorder / undo / import): the
  codes / batches / events pages' `loadCatalog()` (edition filters) read the server again on their next load.

**Forms:** items – toon (or every toon), name / description per language (`locale-text.svelte`: one field group, a
line per language), kind, category, picture (`MediaSlot`, CDN-root path, uploads to `<toon|scutoons>/items/`),
stackable / starter, slot (`AVATAR_SLOTS`), edition, product, album + format, variants (id, name per language, product,
picture slot each), respawn (`respawn-field.svelte`, was the separate RespawnEditor – removed); recipes – inputs (item +
qty, add / remove) and output; albums – toon, title per language, cover slot, download + track audio as file slots
(`file-slot.svelte`: the asset picker with kind archive / audio, a small player for audio – MediaSlot only previews
images / video), access, tracks (id, title, duration, ↑ / ↓ / remove); products – toon, name, item, edition, category,
edition size, link, price, limited, remaining, sizes (comma list) + sold-out sizes (checkboxes), credits (lines),
picture slot, variants (id, name per language, sold out, picture slot, the shop images as slots – an empty slot at the
end adds one), default variant; editions – toon, name, description, numbers, created, creators, variants (id, name,
sizes, colours) + links to its codes / batches; toons – name, redirect URL ("Fill it in the creator" →
`CREATOR_URL/<id>`); permissions – the description; code types – claimable, signup, grants (checkboxes of the
permissions), **saving asks first** (security); meta – version (+ currency, checked, source for `products`).

**Import images …** (items tab, `admin/src/lib/catalog/import.ts`): `<item-id>.<ext>` / `<item-id>_<variant>.<ext>`,
case-insensitive, the longest matching item id first (`dago_hoodie_black` = item `dago_hoodie`, variant `black`) →
`toWebp` (scaled so the longest side is ≤ 1024 px, never up; transparency kept) → `uploadOrReplace` to
`<toon|scutoons>/items/<item-id>[_<variant>].webp` (ids in their stored case) → one `catalogPut` per item (note
`image import`) only when an icon path changed. A report above the list: "n new · n replaced · n unchanged", the items
whose pictures were set, unknown names ("no item with this id" / "has no such variant") and failures.

**Decisions:**
- One page with tabs, not nine sidebar entries (the admin's nav stays short; the old page was tabs too).
- **Featured stays on the Homepage page** (`/homepage`, `/admin/featured`, table-backed since part 1): one place; the
  products tab links to it. The `featured` kind has no tab.
- Up / down buttons, no drag (one way to reorder; drag in a table + SplitView is fiddly on touch).
- Per-field problems are matched by word (the server's problems are plain strings); the full list is always shown.
- After the image import the open entry is read again (unless it has unsaved edits).
- A toon can't be deleted once created (its starter content entry counts as a use – part 1's rule); the UI shows the
  refusal like any other `in_use`.

**Verified** (own dev server on :5173 against the main checkout's API / CDN, test user): edit + save an item name, undo
it from the entry history, "Undo anyway" dialog on the older change (cancelled); create a recipe (422 "output: unknown
item" shown in the list and at the field, then saved), reorder down / up, delete; deleting `cucumber` refused with its
uses (3 recipes, a scan grant, 44 inventory entries); a test item: picture uploaded through the slot (to
`scutoons/items/`) and replaced at the same path ("Replace the file?"), bulk import (2 matched → 1024×512 and 273×1024
WebP, 1 unknown reported; a second run: 1 replaced, 1 unchanged, 1 unknown variant), then deleted; a test toon created
(422 for a bad redirect URL first) and listed in the creator; saving every kind unchanged = "Nothing changed" (the forms
don't alter data on load); code type save confirmation.

### Accepted warnings (branch `catalog-accept`, 2026-10-04)

"Problems e/w" in the bar of items / recipes / albums opens the creator's Problems panel for the catalog (DATA.md §5):
`validateCatalog` over the lists + the open form, wear overlays against the published character registry. ✓ "Looks
fine" stores `accepted_warnings` in the entry's `data` (a list-wide warning in meta `items`): the open entry → its form
(unsaved); another entry → saved at once (history note "warning accepted", undo). Readers never get the field
(`CatalogStore::reader`); a malformed list is a 422. No migration (the field lives in `data`).

## Score collages (2026-10-18, research/COLLAGE.md §4)

- **Data:** a score item's `score.collage` (the full score) / `score.parts.<name>.collage` = `{ file, pieces }` in the
  item's data (its `item:<id>` document: drafts, history, Publish like every entry). `file` = the MusicXML it was placed
  on; `pieces` = `CollagePiece`s (DATA.md §2 "Collage pieces", CDN-root `src`) with `at: { bar, nth?, staff?, page? }` –
  x in % of the bar's time (a piece on a note stays on it whatever the line breaks), y in staff spaces below the
  staff's top line, the width in staff spaces; `at.page` = % of the page holding the bar; `behind` = under the notes.
- **Server:** `CatalogValidator::collage` checks each one at a write (structure, `file` under `scores/`, unique ids,
  media `src`, `at.bar`, the blend in `BLEND_MODES` – CatalogTest keeps the list equal to collage.ts –, the colours
  #rgb / #rrggbb: fill, gradient stops, outline, shadow); the creator's validator checks `collage.file` under `scores/`
  too, so the stage names it before Publish; `publicItem` drops it (`/api/items/` never names a vault path);
  `GET /api/items/<id>/score/musicxml[?part=]` answers `collage: { pieces }` next to the signed URL for the score / part
  opened (never `file`; the PDF none) and the viewer's score view draws them; a piece that names a custom preset arrives
  baked with the published `colorations` (its drawn `style`, no `preset` – DATA.md §2 "Collage pieces").
- **Creator:** Catalog → Items → a score → More → **Collage** → `/catalog/items/<id>/collage`, on the part readers open
  (choir, else the full score; the published file, signed like a reader's – a never-published item does not open):
  the readers' phone / desktop screen with their score view (their sizes, so their page breaks; zoom 30 / 40); a piece
  moves on the page re-anchored to the bar + staff under the pointer (one undo step a drag), a round corner handle
  scales it; the list (over / behind, ⚠ a bar the file lacks) and the piece inspector beside it (below under 1280px).
  The first piece writes `collage = { file, pieces: [] }`, removing the last takes it away; a replaced file shows "made
  on another file – check the bars" + **Checked**. The stage draws custom presets with their newest draft and saves the
  presets document with the item (unsaved: a local copy, offered back at load); its Publish list and the score's catalog
  page's list "Collage presets" while it has a draft.
  `npm run check:creator-collage` (cloud9_en; its drafts deleted again).
- An entry only in the table (no document yet: an admin write, a migration – locally the score items) starts its
  creator document from the table's data (`loadCatalogDoc`, `ContentDoc.startFrom`): unchanged until edited, the first
  save is its first version.
