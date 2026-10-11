# MAP-DOCUMENTS – the world map as many documents (DRAFT, overnight 2026-10-12)

Owner 2026-10-12: "das brauchen wir dann auch für die Items, dass alle einen eigenen Doc haben, den man versionieren
kann; dann ist das gleichmäßig refactored." Played through on branch `claude/zen-shaw-d9c2dd` (worktree
`.claude/worktrees/zen-shaw-d9c2dd`, own dev DB `scutoons_places`). **Not merged, not pushed.** Merging + pushing would
run the migration on production with the deploy – only after the decisions below.

Parallel draft: the catalog's entries as documents (agents/CATALOG-IN-CREATOR.md, session "Draft: move the catalog from
admin into creator") – the same key shape, see "Consistency with the catalog draft".

## TL;DR

- Every place is its own versioned document `place:<id>`, every route `route:<id>`; `map` keeps only the map's own
  settings (width, height, layers). Drafts, publish, history, restore, review rules, asset usages – all per place now,
  with the machinery every document already has. Partial publishing of the map (partial-publish.ts) is no longer needed.
- Everything that READS the map still gets ONE WorldMap, composed from the documents (TS `composeWorldMap`, PHP
  `WorldMapDocs`): the reader's `/library/map`, the old bundle lists, the flow index, the builder, the event forms, the
  MCP. The viewer is untouched – its answers are byte-identical before / after the migration (checked).
- The builder edits one WorldMap like before (`WorldMapDoc`); a save stores only the changed documents in ONE
  transaction, each against the version it was loaded at – a conflict is per place.
- The migration splits the WHOLE map history (every version) into place / route versions, keeps author / time / note,
  derives each published version from the published map; idempotent; dry-run on my DB: 24 rows, answers identical.

## 1. Keys and documents

| Key | Kind | Document | toon column |
|---|---|---|---|
| `map` | map | `MapSettings { width, height, layers, accepted_warnings? }` | NULL |
| `place:<id>` | place | `PlaceDoc` = a `MapPlace` without `id` (+ `removed?`, `accepted_warnings?`) | NULL |
| `route:<id>` | route | `RouteDoc` = a `MapPath` without `id` (+ `removed?`, `accepted_warnings?`) | NULL |

- `<id>` = the slug the map always had (`^[a-z0-9][a-z0-9_-]{0,63}$`, `EntityKey::ID_PATTERN`, TS `ENTITY_ID`). The key
  holds it, the document does not (no second copy to drift). `EntityKey::ID_KINDS = ['place', 'route']`, a new
  `id` property (last constructor parameter), `parse()` special-cases them; TS `ID_KINDS`, `EntityRef.id`,
  `entities.place(id)` / `entities.route(id)`. Platform-wide (toon NULL) – a place's toons are in its `toons` field,
  like the catalog draft's entries.
- **Ids stay stable forever:** events (`unlockLocation`, later `travel`, `revealRoute`), route waypoints and readers'
  save games (`map.places`) reference them. Removing a place stores a last version with `removed: true` – a
  **tombstone**: the key and its history stay, the id stays taken (`takenMapIds`; the builder's new-id / rename never
  reuses one), publishing the tombstone takes the place off readers' maps, restoring an older version brings it back.
  No hard delete anywhere.
- **What stays on `map`:** width, height, layers (the terrain art + tiles – edited rarely, the world's size depends on
  them) and accepted warnings about the map as a whole. Layers could be `layer:<id>` later (decision D2).
- **Order** of places / routes on the map = creation order (the id of each key's first version; `Snapshot::createdOrder`,
  GET /entities?kinds= `first`). Adding a place touches no other document. The migration inserts in the old list order.
- **Accepted warnings** travel with their place: a place's own list holds keys relative to it (`hidden#x1`); composed
  they are `locations.@<id>.hidden#x1` again (the creator's validation is unchanged). None stored today.
- Schemas: `map` → `MapSettings`, new `map-place` (PlaceDoc), `map-route` (RouteDoc). A `map` document with
  `locations` / `routes` is refused (422) – an older client can't silently save into the void.

## 2. Code (what was built)

- `@scutoons/content` **map-docs.ts**: `composeWorldMap`, `splitWorldMap`, `worldMapDocs`, `mapDocChanges` (what to
  store: settings / added / changed / removed → tombstone / restored), `applyMapDocChanges`, `takenMapIds`,
  `mapDocsFromEntities`, `checkMapDocs` + `mapDocOfPath` (a composed path → the document and the path in it).
  Tests: `test/map-docs.test.ts` (round trip on the fixture, tombstones, changes, accepted warnings, problems per doc).
- **Server**: `EntityKey` (place, route), `ContentSchemas` (+ the guard above), `Domain\Content\WorldMapDocs`
  (compose from a Snapshot / from arrays), `Library::map` + `Library::toon` (old bundle lists) + `FlowIndex` read the
  composition; `Snapshot::createdOrder` (order + documents in one query). Interchange: `scutoons/_/map.json` stays ONE
  file – import splits it (`ContentFiles::splitMap`, `--only=map` takes places / routes along), export composes it
  (tombstones left out). Fixtures unchanged.
- **Generic batch endpoints** (meant for the catalog's items too): `GET /api/entities?kinds=a,b&content=draft|published`
  (every entity of those kinds WITH its document + `first`), `POST /api/entities/batch { saves: [{ key, content, base?,
  message? }] }` (one transaction, all or none; 409 / 422 name the `key`; each document guarded like a PUT; asset usages
  reindexed per key), `POST /api/entities/publish { versions: [{ key, version? }] }` (atomic; review guard per key).
  `Database::transaction` joins an open transaction. Client: `@scutoons/api` `withContent`, `saveBatch`, `publishMany`.
- **Creator**: `lib/editor/world-map-doc.svelte.ts` – `WorldMapDoc extends ContentDoc`: one request loads all
  documents, `data['_']` = the composed WorldMap (builder, undo, canvas, validation unchanged); `save()` =
  `mapDocChanges` → `saveBatch` with each key's own base (someone else's save of ANOTHER place is no conflict; of the
  same place → the usual conflict bar); `storedKeys()` = every map key (ContentDoc got a default `storedKeys()`, used by
  DocSession). `loadWorldMap()` for readers (event forms' place select, flow, wanted images, the builder's "published
  ids are fixed"). New / renamed ids skip every id ever stored.
- **Publish checklist** (publish-tree.ts): each place / route is its own line by its name – "Place: Pond · new",
  "Place: At the well · removed", "Place: Cathedral · changed"; the ticked whole files go live in ONE transaction
  (`publishMany`) – a route and the new place it runs through are never half published. `map` left `DOCUMENT_PARTS`
  (partial publish stays for the characters until they get the same treatment).
- **MCP**: `read_map_places` composes (each place with `key`, `draft`, `removed`; the stale `unlock` field – a
  pre-existing tsc error – is gone), `read_flow`'s map too, `validate` on `map` / `place:<id>` / `route:<id>` checks the
  composition with that document swapped in and reports its own problems, `save_draft` works on the keys as on any.
  The rules resource explains it. `scripts/dev/map-docs-check.mjs` (5 checks through the MCP server).
- **Asset usages**: nothing to build – the index is per entity key, so a place's images are now `ref = place:<id>`
  (finer than `map`).

## 3. Reader side

`/library/map` and the toon bundle are composed from the published documents of the snapshot (`WorldMapDocs::
fromSnapshot`); the answer shape is unchanged, so the viewer needs nothing. **Caching** stays content-addressed: the
snapshot's fingerprint holds every place / route version id, so publishing one place gives a new cache key – nothing to
empty, not even after the migration (new keys). A draft of a place changes nothing for readers
(`LibraryTest::testWorldMapFromDocuments`). Cost: two queries per uncached answer (keys + documents), then the cache.

## 4. Migration `server/migrations/mysql/2026-10-12-map-documents.mysql.sql`

- Splits EVERY stored version of the one-document map, in order (JSON_TABLE + window functions, MariaDB 10.6+;
  production 10.11): a `place:<id>` / `route:<id>` version where an entry first appears, changes (JSON differs from the
  map version before), leaves (tombstone = its last document + `removed: true`) or comes back. Each carries the map
  version's author and time; the note says `map v<id>: <old note>`. **Published** = the newest version made from a map
  version up to the published map version – readers see exactly what they saw. Newer drafts stay drafts.
- Then the lists leave every old `map` version **in place** (same ids, status, history – only settings remain).
- Idempotent: the split runs only while a map version still has lists AND no place / route document exists; the strip
  only touches versions with lists. Data statements only → the runner runs it in one transaction (all or none).
- **History**: kept completely (better than "newest draft + published"): each place's history is the map's history
  filtered to that place. The old composed snapshots themselves are gone from `map` (the strip) – reconstructable from
  the place versions (export-content --draft composes the newest).
- **Checked**: `EntityStoreTest::testTheMapDocumentsMigration` (4 crafted map versions: move, add, remove, come back,
  route change, published in the middle; a second run changes nothing). Dry-run (MigrationRunner::plan) + run on
  `scutoons_places` (the 2026-10-06 seed + the 10-11 unlock migration + one extra draft map version: a moved place, a
  removed one, a new one): 24 rows, 3 map versions stripped; `/library/map` (de; en + catpope; de + catpope) and
  `/library/toons/catpope` (de, en) **byte-identical** to main's code before (main's server copy on another port, same
  DB); `export-content --draft` identical.
- Check SELECTs are in the file's header (lists left, place documents, accepted warnings on a place = must be 0).
- **Deploy order**: the deploy uploads the API, then runs pending migrations (owner 2026-10-12, HANDOVER
  "Migrations"). Between the two (seconds) the new code reads a `map` without place documents → an empty map; the
  library cache is keyed by the snapshot, so that answer is never served after the migration. Alternative (D9): split
  the deploy – migration first is impossible (old code would read a map without lists → empty map, worse).
- After the run: `assets:scan` (admin Assets → Rescan) moves the map's image usages from `map` to `place:<id>`; until
  then the old rows still protect the images (nothing breaks).

## 5. Validation across documents

`checkWorldMap` of the composition is still THE check (references route → place, duplicate ids, positions inside the
map's size); `checkMapDocs` addresses each problem to its document (`place:<id>` + path). The builder validates the
composed map as before; the MCP's `validate` checks one document against the others; the server checks each save
against its schema only (as it did for the one map) – a route to an unknown / removed place is an error in the creator
and MCP, not a refused save (D7).

## 6. Not done / known gaps

- **History panel** on the map page still shows `map`'s history (now the settings only). A place's own history (and
  restore) is the obvious next step: History follows the selection (D5).
- **The checklist's "Map" line** also appears for a newer `map` version whose settings equal the published ones (after
  the migration: every map draft newer than the published map). Harmless (publishing it changes nothing); hide no-op
  drafts in the checklist generally? (D6)
- **Rename before the first publish** leaves a tombstone under the old id (the builder's id follows the name until the
  first publish). Harmless, the old id stays taken (D3).
- Not run: `scripts/check-map.mjs` (headless builder / reader check) – it needs the full dev stack on the default ports;
  `npm run check:mcp` fails on my DB for an unrelated reason (the 10-06 seed's chapters are not converted to neutral
  documents yet; the same on main with this seed). The one-off scripts `scripts/places-sizes.mjs` /
  `places-upload.mjs` save the whole map into `map` – obsolete with this model (the server now refuses that).
- The admin's content overview lists every place / route as a row (platform-wide) – fine for 20, maybe group by kind.
- Owner's dev DB `scutoons` and production: untouched.

## 7. Consistency with the catalog draft (agents/CATALOG-IN-CREATOR.md)

Same shape on both branches: `<kind>:<id>`, no toon segment (toon column NULL, the entry's own field says which toons),
an `id` property on EntityKey, parse() special-cases the kinds, idempotent migration. At the merge: one list (e.g.
`ID_KINDS = [...CATALOG_KINDS, 'place', 'route']`) – EntityKey.php parse / __toString / entity() will conflict a little,
nothing structural. Shared building blocks to reuse there: tombstones instead of deletes, `GET /entities?kinds=`,
`POST /entities/batch`, `POST /entities/publish`, the checklist listing documents by name. (Told that session.)

## 8. Decisions waiting for the owner

- **D1 Direction.** On 2026-10-12 (commit 35669c6e) the owner said "die Karte nicht mit ganz vielen Dokumenten" and
  partial publish was built for that; later the same day "alle einen eigenen Doc". This draft follows the later one –
  confirm.
- **D2 Layers**: stay in `map` (recommended: few, rarely edited, the size depends on them) or `layer:<id>` too?
- **D3 Removal**: tombstones (built) – or allow a hard delete of a never-published place (rename before the first
  publish would then leave nothing)?
- **D4 History**: the whole history split (built) – or only newest draft + published (smaller, simpler SQL)? And strip
  the old `map` versions in place (built) or leave them as they were?
- **D5** History panel on the map page: follow the selection (the place's own versions, restore per place)?
- **D6** Hide checklist lines whose draft equals what is published?
- **D7** Should the server refuse a route save that points at an unknown / removed place (today: creator / MCP error only)?
- **D8 Order**: creation order (built) – or an explicit order list in `map` (would touch `map` on every add)?
- **D9 Deploy**: accept the seconds-long empty-map window between API upload and migration, or a maintenance step?
- **D10** Same for the characters registry (`character:<id>`) – then partial-publish.ts can go entirely?

## 9. Trying it

```
cd .claude/worktrees/zen-shaw-d9c2dd
SCUTOONS_DB_NAME=scutoons_places npm run dev:db -- up --no-api       # own DB (migration included)
SCUTOONS_DB_NAME=scutoons_places node server/bin/dev-serve.mjs --port=9400   # own API (server/config.local.php: gitignored, own secret)
cd creator && VITE_API_URL=http://localhost:9400/api npx vite dev --port 10400
SCUTOONS_API=http://localhost:9400/api SCUTOONS_DB_NAME=scutoons_places npm run dev:test-user -- --browser http://localhost:10400
SCUTOONS_API=http://localhost:9400/api node scripts/dev/map-docs-check.mjs
```
Browser check done: the builder loads the composed map; nudging one place and saving stored exactly one version
(`place:dry-run-pond`); the checklist listed Map / Cathedral (changed) / Pond (new) / At the well (removed); publishing
two of the four put only those live (`/library/map`: Pond at its new x, the well gone, Cathedral still at its
published position).

Commits (branch, oldest first): shared model → server composer → migration → batch endpoints → creator → MCP → preload
→ guard + this file. Checks: see the last commit message.
