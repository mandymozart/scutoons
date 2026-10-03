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
