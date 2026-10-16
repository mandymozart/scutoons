Status: steps 1–4 built 2026-10-16 (map tiles; Assets → Unused as a server list with "Select all N" and a batched delete). Steps 5–7 open – owner to pick.

# ASSET-LISTS — long lists with filters, "select all", bulk actions with progress (owner 2026-10-16)

Owner: "Can we figure out how to hide or filter map tiles from the asset manager? They are thousands … So in images map
tiles may be a special image type?" · "This list is always a bit hard to handle if there are many entries … still filter
it and then still select all and have some progress reporting on for example the delete … a component that could be
shared already." Production then: Unused showed 7,252 assets, nearly all `catpope/map/terrain/…` and
`catpope/map/tiles/…` tiles – **including the live v2 tiles** (the map layer's `tiles.path` is a folder, the usage index
only saw strings with a media extension), so "select all → delete" would have broken the world map.

## What existed
- Server: `ListQuery` (page, page_size, sort, q, filters → `{ items, pagination }`) – used by the asset list, codes, users …
- `@scutoons/ui`: `DataTable` (checked rows, a bulk bar, sort links), `Pager`, `list-query.ts` (URL state:
  `readList` / `listHref` / `listParams`); filters as `.bar-filter` selects in the ActionBar (admin layout CSS).
- Users: "N match – add all to a segment" – the server applies the same filters (`matching: true`), no progress.
- Assets → Standard: progress in the button ("Converting 3 / 12 …"), no cancel. Unused: the whole list in one response,
  checked rows only, one request for every file.
- The creator's picker: its own sticky bottom bar (`.sc-picker-filters`), cards with "more" paging, no multi-select.

## The pattern (built)
1. **Map tiles are their own kind of image** (`AssetPaths::tileSet`, SQL `TILE_SQL`): `<set>/<z>/<x>_<y>.<ext>` below
   a `/map/` folder. Lists hide them by default (`tiles=hide|only|all`, + `tiles_hidden`); the Kind filter has "Map
   tiles" (= `tiles=only`) in the admin and the picker. A layer's `tiles: { path }` now uses every tile of its set.
2. **Server list**: filters, sort and page on the server; the response carries `totals` (everything) and `matching`
   (the filters); `all=1` = every matching row with all its data (what a bulk action needs). Rows that belong together
   are one row (a tile set: "507 map tiles").
3. **Selection**: `DataTable` `total` + `bind:allMatching` – a checked page offers "Select all N"; the bulk bar says
   "All N selected"; unticking a row or a filter change ends it.
4. **Bulk action**: resolve the selection (`all=1`) → units that must stay together (an asset's files) → `chunk(units,
   25, size)` → one confirm with the exact count → `BulkRun.start(batches, step)` (one request after the other; Cancel
   stops before the next batch) → `BulkProgress` (label, bar, "1,200 of 7,252 files", Cancel) → the page's report
   (done, skipped, failed with reasons, cancelled / stopped). The server re-checks every batch; only the first one pays
   the expensive refresh (`rebuild`).

## Next (not built)
5. Move Standard's convert and the users' "add all to a segment" onto `BulkRun` / `BulkProgress` (cancel, one report look).
6. A shared filter row (`.bar-filter` label + select from a list of options) once a third list hand-writes the same
   selects; the picker keeps its bottom bar but uses the same server parameters.
7. Deleting a tile set purges the CDN per file (bunny: one call each); one folder purge per set would make thousands of
   deletes much faster – measure on production first.
