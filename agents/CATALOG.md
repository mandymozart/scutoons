# CATALOG – the catalog moves from config files into the database (CUTOVER 10.29)

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
- The JSON files become **seeds** for empty / local databases only (like `shared/content/seed`, CUTOVER 10.15); the
  production migration carries today's data as INSERTs (generated from the files, owner runs it in phpMyAdmin).
- Every consumer keeps its output: `GET /api/items/`, `/api/products/`, the admin catalog response, the QR toons –
  same JSON shape, so the viewer / shop / qr / embassy don't change.
- The asset registry indexes the catalog rows as uses (today it scans the catalog files: `AssetUsages::CATALOG_FILES`),
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

### Part 1 – what is built

**Tables** (migration `2026-10-04-catalog`, `server/migrations/{mysql,sqlite}/`; utf8mb4, `utf8mb4_bin` = case-sensitive
ids): `catalog_items` (+ `toon`, `edition_id`), `catalog_recipes`, `catalog_albums` (+ `toon`), `catalog_products`
(+ `toon`, `item_id`, `edition_id`), `catalog_editions` (+ `toon`), `catalog_featured` (+ `product_id`, `variant_id`; id
`<product>[:<variant>]`), `catalog_permissions` (data = the description string), `catalog_code_types` (id `default` =
the old `default`), `toons`, `catalog_meta` (the files' top-level fields: `items` `{ version }`, `products` `{ version,
currency, checked_at, source }`, `permissions` `{ version }` – not in the original list, needed for byte-equal answers),
`catalog_history` (kind, entry_id, action create/update/delete/reorder/replace, before_json / after_json, position,
undo_of, note, changed_by, changed_at). Every row: `id`, `position`, `data` (LONGTEXT + JSON_VALID – not MySQL's JSON
type, which reorders keys), `updated_at`, `updated_by`. Relations are real columns kept in step with `data`; integrity
is checked by the API (no FOREIGN KEYs: undo / reorder / SQLite stay simple).
- The migration carries **today's data** (one INSERT per table, each only while the table is empty → re-running never
  brings back deleted entries), generated from the files: `php bin/console catalog:export-sql --write` (CatalogTest
  checks the committed block = the generator's output). It **folds and deletes** the settings `item_respawn` (overrides →
  the items' `respawn`) and `featured_products` (the stored list → `catalog_featured`, before the seed).
- `migrations/schema/*` (the base schema of a fresh DB) is NOT changed: it has never had the later tables; a fresh
  database gets the base schema and then every migration, this one included.
- `php bin/console catalog:seed [--replace] [--db=]`: empty tables ← the files (a fresh DB is already seeded by the
  migration; `--replace` resets a local DB to the files). `npm run dev` / `npm run dev:db` need nothing new (`migrate`).
- Without the tables (deploy before the migration) the API reads the files, read-only (writes → `503 migration_pending`).

**Loaders** (`server/src/Domain/Catalog/`): `CatalogStore` (rows per kind, the only reader of the tables), `Catalog`
(`data()` = `{ version, slots, items, recipes, albums, stats }` as before), `Editions`, `Products` (catalogJson /
featuredJson keep objects and key order), `FeaturedProducts`, `Toons`, `Access\Permissions::map()` – APIs unchanged,
the ~25 consumers untouched. `ItemRespawn` reads the item rows (`apply()` is a no-op now); its admin update is an item
update (history, note `respawn`); "file" in its admin view = the seed's rule, `reset` = back to it.
**slots / stats are code:** `Domain\Avatar\AvatarSlots::ALL`, `Domain\Reader\ReaderStats::DEFINITIONS` =
`@scutoons/content` `AVATAR_SLOTS`, `READER_STATS` (CatalogTest compares them); removed from `config/items.json`;
`catalogFromSeed()` (content) adds them back for tests / scripts reading the seed file; `validateCatalog` falls back to
them.

**Admin API** (role admin; docs/API.md "Admin API → Catalog"): `GET /api/admin/catalog/<kind>`, `GET|PUT|DELETE
/api/admin/catalog/<kind>/<id>` (`PUT { data }` creates 201 / updates 200; `422 invalid_catalog { problems }`, `409
in_use { uses }`), `POST /api/admin/catalog/<kind>/reorder { ids }`, `GET /api/admin/catalog/history?kind=&id=&limit=`,
`POST /api/admin/catalog/history/<n>/undo { force? }` (`409 changed_since`). Kinds: items, recipes, albums, products,
editions, featured, permissions, code-types, toons, meta. The old `/catalog`, `/catalog/respawn`, `/featured` keep working.
Validation = `CatalogValidator` (the essentials of validateCatalog / validateAlbums; wear-vs-registry stays client-side).
**Toons:** `PUT /api/admin/catalog/toons/<id> { data: { id, name, redirectUrl } }` – the id must be a content toon id
(`[a-z0-9_-]`); creating also stores a draft content index entry `toon:<id>` (names from `name`, de + en) when none
exists, so the creator lists the toon and creates its settings on first open. The sandbox stays content-only (it never
was a QR toon). A toon with content / catalog entries can't be deleted.

**Assets:** the usage index reads the rows: source `catalog`, ref `<kind>:<id>` (`item:camera`, `product:dago_hoodie`,
`album:awakening`), field = the pointer in `data`; every catalog write re-indexes; relink / rename update the rows through
the admin path (validated, history; a refused one is reported as `manual`).

**Tests:** `CatalogTest` (tables = files, seed / export, the folds on SQLite + MariaDB, fallback, constants vs TS,
validation, history / undo, toons, respawn / featured), `CatalogApiTest` (public answers = files, admin CRUD, toons),
AssetsTest / AppTest / contract suites adapted.

### Production (owner)

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
3. After the deploy: `https://api.scutoons.com/api/admin/catalog/items` (signed in as admin) answers `"fallback": false`.

### Open / decided differently

- An empty featured list stored as the setting (`[]`) comes back as the default list (the table can't tell "never set"
  from "chosen empty"); the admin can clear it again.
- Undoing a toon's creation is refused while its starter content entry exists (`in_use`: a toon with content is never
  deleted by the API).
- Part 3 (admin UI) can use the endpoints above; `@scutoons/api` has no client functions for them yet.
