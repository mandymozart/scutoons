Status 2026-10-12: merged into `main` (map, places, routes and characters as their own documents; two migrations run with the deploy); D5–D8 open (FEATURES 10.89).

# MAP-DOCUMENTS – the world map and the character registry as many documents (DRAFT, 2026-10-12)

Owner 2026-10-12: "das brauchen wir dann auch für die Items, dass alle einen eigenen Doc haben, den man versionieren
kann; dann ist das gleichmäßig refactored." Played through on branch `claude/zen-shaw-d9c2dd` (worktree
`.claude/worktrees/zen-shaw-d9c2dd`, own dev DB `scutoons_places`). **Not merged, not pushed.** Merging + pushing runs
both migrations on production with the deploy.

Parallel draft: the catalog's entries as documents (agents/CATALOG-IN-CREATOR.md, session "Draft: move the catalog from
admin into creator") – the same key shape, see §8.

## Owner decisions (2026-10-12, morning)

| | Question | Decision | Built |
|---|---|---|---|
| D1 | every place its own document (the later of the two 10-12 statements) | yes | – |
| D2 | layers stay in `map` | yes | as built |
| D3 | removing an entry | **delete when never published**; a published one keeps a `removed: true` version | §1, §2 |
| D4 | migration history | **only newest**: the published version + the newest draft | §4 |
| D9 | the seconds-long empty map between API upload and migration | accepted | §4 |
| D10 | the character registry the same way, partial publishing goes | yes | everywhere |

Open (left as built, no answer yet): D5 History panel per entry, D6 hide checklist lines whose draft equals the
published version, D7 server refusing a route to an unknown place, D8 creation order (vs an explicit order list).

## TL;DR

- A **collection** = an anchor document (its own fields) + one versioned document per list entry, `<kind>:<id>`
  (the entry without its id). Two collections: the world map (`map` = width / height / layers, `place:<id>`,
  `route:<id>`) and the character registry (`characters` = its own fields, `character:<id>`). Drafts, publish, history,
  restore, review rules and asset usages are now per place / route / character.
- Everything that READS still gets ONE composed document in the old shape (TS `composeCollection` / PHP
  `CollectionDocs`): `/library/map`, `/library/characters`, casts, the toon bundle's old lists, the flow index, the
  editors, the MCP. The viewer is untouched – its answers are byte-identical before / after both migrations (checked).
- The editors (map builder, characters page) edit the composed document as before; a save stores only the changed
  documents in ONE transaction, each against the version it was loaded at (a conflict is per entry), and deletes
  never-published entries that left. The publish checklist lists each entry by name; the ticked files go live in one
  transaction. **Partial publishing (partial-publish.ts) is gone.**

## 1. Keys and documents

| Key | Kind | Document |
|---|---|---|
| `map` | map | `MapSettings { width, height, layers, accepted_warnings? }` |
| `place:<id>` | place | `PlaceDoc` = a `MapPlace` without `id` (+ `removed?`, `accepted_warnings?`) |
| `route:<id>` | route | `RouteDoc` = a `MapPath` without `id` |
| `characters` | characters | `CharacterRegistrySettings` (its accepted warnings – nothing else today) |
| `character:<id>` | character | `CharacterDoc` = a `RegistryCharacter` without `id` |

- `<id>` = the slug the entries always had (`^[a-z0-9][a-z0-9_-]{0,63}$`, `EntityKey::ID_PATTERN`, TS `ENTITY_ID`).
  `EntityKey::ID_KINDS = ['place', 'route', 'character']`, an `id` property (last constructor parameter), `parse()`
  special-cases them; TS `ID_KINDS`, `EntityRef.id`, `entities.place / route / character(id)`. Platform-wide (toon
  column NULL) – like the catalog draft's entries.
- **Removal (D3):** an entry that leaves and was **never published** is deleted outright (`POST /entities/batch
  { deletes }` → `EntityStore::deleteUnpublished`: place / route / character keys only, 409 `published` otherwise,
  pending reviews of it annulled) – its id is free again (a rename before the first publish leaves nothing behind). A
  **published** one gets a last version with `removed: true` (a tombstone): readers' save games, casts and events know
  the id, so it stays taken; publishing the tombstone takes it off readers; restoring an older version brings it back.
- **Order** in a list = creation order (each key's first version id: `Snapshot::createdOrder`, `first` in
  GET /entities?kinds=). Adding an entry touches no other document.
- **Accepted warnings** travel with their entry (relative keys inside it, `locations.@<id>.…` when composed).
- **Guard:** an anchor document that still carries its list (`map` with `locations` / `routes`, `characters` with
  `characters`) is refused (422) – an older client can't save into the void.
- Schemas: `map` → MapSettings, `map-place`, `map-route`, `characters` → CharacterRegistrySettings, `registry-character`.

## 2. Code

- **`@scutoons/content`**: `collection-docs.ts` (spec, compose, split, from stored entities, changes incl. tombstones,
  apply incl. deletes, taken ids, problems addressed to their document); `map-docs.ts` (`MAP_COLLECTION`,
  `composeWorldMap`, `checkMapDocs`) and `character-docs.ts` (`CHARACTERS_COLLECTION`, `composeCharacterRegistry`,
  `checkCharacterDocs`) are its two instances. `partial-publish.ts` deleted. Tests: `test/map-docs.test.ts` (8).
- **Server**: `Domain\Content\CollectionDocs` (MAP, CHARACTERS: compose from a Snapshot / arrays / the published store,
  split an interchange file, `ofAnchor` / `ofPart`), `WorldMapDocs` delegates. Readers: `Library::map`, `::toon` (old
  bundle lists), `::cast`, `::characters`, the map's sea colour, `FlowIndex`, `CharacterUnlocks` (+ admin user pages).
  Interchange: `map.json` / `characters.json` stay ONE file each – import splits (`--only=map|characters` takes the
  entries along), export composes. Fixtures unchanged; tests store collections with `TestCase::storeCollection`.
- **Generic endpoints** (for the catalog's items too): `GET /api/entities?kinds=a,b&content=draft|published` (all
  documents of those kinds + `first`), `POST /api/entities/batch { saves, deletes?, message? }` (one transaction; 409 /
  422 name the `key`), `POST /api/entities/publish { versions }` (atomic; review guard per key). `Database::transaction`
  joins an open transaction. Client: `@scutoons/api` `withContent`, `saveBatch(saves, message, deletes)`, `publishMany`.
- **Creator**: `lib/editor/world-map-doc.svelte.ts` – `CollectionDoc` (one request loads, `data['_']` = the composed
  document, save = changes → batch with per-key bases + deletes of never-published entries; `storedKeys()` = every
  key) with `WorldMapDoc` and `CharacterRegistryDoc`; `registryDoc()` / `mapDoc()` return them; `loadWorldMap()`,
  `loadCharacterRegistry()` for readers. New ids skip every stored id. The publish checklist (publish-tree.ts): one line
  per file, entries by name ("Place: Pond · new", "Character: passerby · removed"), the ticked ones live together.
- **MCP**: `read_map_places`, `read_characters`, the VN checks, `read_flow` read the compositions; `validate` on `map` /
  `place:<id>` / `route:<id>` / `characters` / `character:<id>` checks the composition with that document swapped in
  and reports its own problems; `save_draft` works on the keys. Rules resource updated. `scripts/dev/map-docs-check.mjs`
  (7 checks through the MCP server).
- Scripts: `make-sandbox.ts` composes the registry from the documents; `creator-cast-check.mjs` expects the batch save /
  publish (not run – needs the full stack on the default ports).
- **Asset usages**: per entity key – images now count under `place:<id>` / `character:<id>`.

## 3. Reader side

Composed from the snapshot's published documents; the answers' shapes are unchanged. **Caching** stays
content-addressed: the fingerprint holds every entry's version id, so publishing one place / character gives a new key
– nothing to empty, not even after the migrations. A draft of an entry changes nothing for readers
(`LibraryTest::testWorldMapFromDocuments`). Cost: two queries per collection per uncached answer.

## 4. Migrations (D4: only newest)

`2026-10-12-map-documents.mysql.sql` and `2026-10-12-character-documents.mysql.sql`, the same rules:
- Per entry at most: the version in the **published** anchor version (published), the **newest** anchor version's
  where it differs or is new (draft), and a `removed: true` draft where the newest dropped a published entry. Author,
  time and note of that anchor version (`map v<id>: …`, `characters v<id>: …`). Older drafts are not split.
- Then the list leaves every anchor version **in place** (same ids, status). The migration runner (deploy / MCP
  `apply_migrations`) backs the changed rows up first (`storage/migration-backups`) – the old composed versions are not
  lost.
- Idempotent (split only while an anchor still has its list and no entry document exists; strip only where a list is
  left). Data statements only → one transaction.
- **Checked** – tests: `EntityStoreTest::testTheMapDocumentsMigration` (4 map versions: an older draft not carried
  over, published, an intermediate draft, the newest with a moved / new / dropped place), `…CharacterDocumentsMigration`.
  Dev DB `scutoons_places` (fresh from the 10-06 seed + the 10-11 unlock migration + extra drafts): map 24 rows
  (21 published, 3 drafts, the intermediate draft not carried), registry 11 rows (8 published, 3 drafts). Against
  main's code before on the same DB: `/library/map` (3 variants), `/library/toons/catpope` (de, en),
  `/library/characters`, `/library/toons/catpope/cast` (2), `export-content --draft` – **byte-identical**.
- **Deploy (D9 accepted):** the API upload comes first, then the migrations – for seconds the new code sees no entry
  documents (an empty map / registry). Old code never sees new data.
- After the run: `assets:scan` (admin Assets → Rescan) moves image usages from `map` / `characters` to the entry keys;
  until then the old rows still protect the images.
- Dev note: `npm run dev:db` loads a production seed AFTER migrating, so a seed taken before production ran these
  migrations stays in the old shape although they are recorded as applied (true for every data migration) – delete the
  two rows from `schema_migrations` and migrate again, or take a fresh seed after production ran them.

## 5. Validation across documents

The collection's own check of the composition is still THE check (`checkWorldMap`, `validateCharacters`);
`checkMapDocs` / `checkCharacterDocs` address each problem to its document. The editors validate the composed document
as before; the MCP's `validate` checks one document against the others; the server checks each save against its schema
(D7 open: refusing a route to an unknown place on the server).

## 6. Not done / known gaps

- History panel (D5): the map / characters page shows the anchor's history (now only its own fields). A place's /
  character's own history and restore is the next step – History follows the selection.
- The checklist lists an anchor version whose own fields equal the published ones (after the migration: every newer
  draft of `map` / `characters`) – harmless (D6).
- Not run: `scripts/check-map.mjs`, `check:creator-cast` (need the full dev stack on the default ports), `check:mcp`
  (fails on the 10-06 seed's unconverted chapters – the same on main).
- The admin's content overview lists every entry as a row (platform-wide) – maybe group by kind.
- The owner's DB `scutoons` and production: untouched.

## 7. Trying it

```
cd .claude/worktrees/zen-shaw-d9c2dd
SCUTOONS_DB_NAME=scutoons_places npm run dev:db -- up --no-api
SCUTOONS_DB_NAME=scutoons_places node server/bin/dev-serve.mjs --port=9400   # server/config.local.php: gitignored, own secret, DB scutoons_places
cd creator && VITE_API_URL=http://localhost:9400/api npx vite dev --port 10400
SCUTOONS_API=http://localhost:9400/api SCUTOONS_DB_NAME=scutoons_places npm run dev:test-user -- --browser http://localhost:10400
SCUTOONS_API=http://localhost:9400/api node scripts/dev/map-docs-check.mjs
```
Browser checks done: map – a nudge saved exactly one place version; publishing 2 of 4 checklist lines put only those
live; deleting the never-published Pond left no row. Characters – changing one colour saved exactly `character:boy`;
the checklist listed Boy / Cat Pope (changed), Test cat (new), passerby (removed).

## 8. Consistency with the catalog draft

Same shape: `<kind>:<id>`, no toon segment (toon NULL, the entry's own field), an `id` on EntityKey, parse()
special-cases the kinds, idempotent migrations. At the merge: one list (e.g. `ID_KINDS = [...CATALOG_KINDS, 'place',
'route', 'character']`) – EntityKey.php will conflict a little. Reusable there: tombstones / deletes, the three generic
endpoints, `collection-docs.ts` + `CollectionDocs` if the catalog wants a composed answer (`/api/items/`), the checklist
listing entries by name. (Told that session.)
