Status 2026-11-11: design for #267 / #427, nothing built; phase 1 of the leg "scenes and scene storage" (phase 2 #246, phase 3 #244). §1 is read from the code; §2–§6 are proposals until the owner answers §7. The format and the package stay in SCENE-PACKAGE.md (#288, #393); this note is the whole picture and points there.

# SCENES — what a deck scene is, where it is stored, how it reaches a deck

Owner 2026-11-01 (#267): *"we need a concept for how to dynamically load and execute scenes. this is crucial also for how we design them and store them in the database for the live system."* · 2026-11-08 (#427): *"das deck muessen wir dann mal in debug bzw oder development sandboxes und richtige szenen mit realen daten unterteilen. damit wir weiter sauber arbeiten koennen"*.

Why a new note and not SCENE-PACKAGE.md: that note answers the stored format (one graph, holders, world nodes) and the signed package for flashing. It does not cover today's sources, the API, the deck's run-time loading, sandboxes or the order of work. Those are here; the format is not repeated.

Already decided and kept here: scenes are entities `scene:<id>` in `entity_versions`, not catalog, loaded through the API (2026-11-02 `[scenes-from-api]`, RULES A4); the scene TOMLs are test fixtures / offline fallback at most (same); a scene is AV and switches with a crossfade (2026-11-01, 2026-11-02 `[scenes]`); a scene names a set and a spot, the set at its current version (2026-11-02 `[scenes]`); a reference to a rig stays on its version until pulled (2026-11-08 `[scenes]`); the Rust build service builds on publish, before that the work plays only in the editor (2026-11-08 `[scenes]`); a package freezes versions (2026-11-08 `[scenes]`); a scene belongs to its Experience on the XR side (2026-11-10 `[xr]`); one schema, generated typings (2026-11-10 `[contract]`); an address is real only where a receiver exists (2026-11-10 `[address]`).

## 1. Today: every place a scene comes from

| # | Source | Format | Who loads it | Cached | Code |
|---|---|---|---|---|---|
| 1 | **Built in** – 12 names + `silent`, compiled into every binary | TOML text (`include_str!`) | native `read_scene` (`--scene <name>`, deck.toml `scene`), F5, browser `?scene=`, a cell's `scene:<name>` | in the binary | `deck/space/src/scene.rs` `BUILTIN` / `builtin()`; `deck/native/src/main.rs` `read_scene` |
| 2 | **A folder of TOML** – `/usr/share/deck/scenes` on the X1, every file of `deck/native/scenes/` | TOML file | F5 (built-ins first, then the folder), `--scene <file>`; `--dev` reloads it on change | the image; `deck-update bundle --scenes` | `deck/native/src/switcher.rs` `SCENES_DIR`, `entries`; `deck/os/container/build.sh:70`; `deck/native/src/watch.rs` |
| 3 | **A rig played as a scene** – `--scene rig:<subject>[@<mood>]` | rig `Bundle` → `Rig::flatten` → `Scene` | native `rig.rs`; browser `rig_play` | core rigs in the image; catalog rigs by scene-content's version hash | `deck/native/src/rig.rs`; `deck/space-web/src/lib.rs` `scene_rig` |
| 4 | **A show's cell** – piece `scene:<id>`, `character:<id>`, `item:<id>` | the show JSON names the piece; the piece is (1), a core rig, or fetched | deck-space `piece.rs` (`load`, `available`), on every deck; a flash checks every piece first | show: `<cache>/shows/<id>@<v>.json`; native pieces: `<cache>/pieces/<kind>.<id>.toml` (**nothing writes scenes there yet**) | `deck/space/src/piece.rs`; `deck/native/src/live/piece.rs` `find`; `deck/space-web/src/outlet.rs` `find`; `deck/space/src/fetched.rs` (*"No place serves a scene that is not built in"*) |
| 5 | **scene-content** – not a scene, what a scene needs: sets (the catalog's home scenes), portraits, rigs + instruments + presets + module descriptors | JSON, CDN paths (immutable), version hashes | native `sync.rs` at a scene load; conductor's `scene.sync` push → `deck.synced` | `<cache>/sets`, portraits, `<cache>/sync/index.json` (path + size) | `server/src/Domain/Library/SceneContent.php`; `deck/native/src/sync.rs`; protocol `SceneSync` |
| 6 | **Editors** – the deck editor's stage plays a core rig; the creator's Sound workspace plays a rig in the wasm engine | rig `Bundle` | `deck/editor/src/stage.rs` (#444); `creator/src/lib/editor/rig-session.svelte.ts` + `shared/engine-host` | — | nothing authors or saves a scene |
| 7 | **The database** | — | — | — | no `scene` kind in `EntityKey::KINDS`; the old `scene:<toon>:<chapter>:<name>` group documents live only in `entity_versions_legacy` (2026-10-19), so `scene:<id>` is free |

The 19 files of `deck/native/scenes/`, sorted (#427):

| Group | Scenes | What they are |
|---|---|---|
| **System** – the deck's own states | `idle` (start, #252), `silent` (no rig, #425), `outlet` (hosts a Live outlet, #186), `test-pattern` (what plays when a scene cannot, #185) | not content: the deck needs them with no network and no account |
| **Checks** – hardware and engine tests | `channel-walk`, `cue-34`, `surround-51` (#318), `strobe`, `lamps-off`, `pulse-thunderwash` (#211, #261), `show-clock`, `show-clock-lead` (#321), `lfo-glide` (#319) | sandboxes; several are read by tests (`outputs.rs`) |
| **Demos** – sketches with real art | `cathedral` (owner 2026-11-08: *"ein test gewesen"*), `conductor`, `madness`, `catpope-analog` (#418), `module-walk`, `module-analog` (#406) | sandboxes; `cathedral` and `conductor` are used widely in tests (the pad, the relay's beings, conformance) |
| **Real scenes** | none | — |

## 2. What a scene is

**A scene is one AV graph that a deck plays as a whole:** nodes and links of sound, picture and light (and their logical outputs), the rigs it holds, placed in one set at one point. Stored as `scene:<id>`; while a deck plays it, `scene:<id>` is also its address.

| Word | What it is | Relation to a scene |
|---|---|---|
| **set** (`home-scenes` catalog for now, #193) | a place's 3D staging, with spots | a scene names one set + a spot, never its files |
| **location / place** (`place:<id>`) | one set with its own state (CATALOG-LINK §8, owner 2026-11-10) | a scene is spawned at a point of a location; words to the location reach the scene |
| **rig** (`rig:<subject>`, catalog) | the sound and behaviour of one holder | a scene holds rigs by reference (pinned version) or embedded (SCENE-PACKAGE §2) |
| **module** (`module:<id>`, by hash) | compiled code | reached only through a rig or a node; never inlined |
| **piece** (SHOW §5) | what a cell plays: a scene, a rig or a module | a scene is one kind of piece |
| **show › row › cell** | the conductor's matrix; a cell = piece + values per outlet | a row decides what each deck shows; a scene does not say which deck it is on (owner 2026-11-09, #462) |
| **chapter group** ("scene" in the flow and the creator) | a part of a chapter: slices, text, VN script | not a deck scene; in the address book it is `group` |
| **XR scene** (inside `experience:<id>`) | OSCT's scene on a target, played by the reader's three.js engine | a different engine and home: it stays inside its Experience ([xr] 2026-11-10 §5.4); never a `scene:<id>` |

In the address book (#477) a published scene is an entry of kind `scene` with `ref` and `address` = `scene:<id>`: a receiver exists (the deck that plays it; `address.rs` has the place `scene:`). Its exposed knobs are addresses below it (`scene:nave/sky.light`).

## 3. Storage

| | **A · entity `scene:<id>`** (recommended; as decided 2026-11-02) | B · catalog document (like `rig:`) |
|---|---|---|
| Where | `entity_versions`, an ID kind like `show:<id>` (`EntityKey::ID_KINDS`) | drafts in `entity_versions`, publishing writes a `catalog_scenes` row |
| API | its own `ScenesController`, the shape of `/api/shows` | `/api/admin/catalog/scenes` |
| Fits | content edited by creators, versions, restore, review | admin data; the owner said "not catalog" |

**The document** (v1 = today's `Scene` as JSON, nothing new invented):

| Field | v1 | Later |
|---|---|---|
| `format` | `1`; a deck refuses a format it does not know (A9) | — |
| `id`, `version` | the key's id; `version` stamped by the server on every change (as shows) | — |
| `toons` | list, may be empty: the address books it is listed in | — |
| `stage`, `node`, `link`, `outputs`, `being` | exactly deck-space's `Scene` (TOML is only a view of it) | — |
| `set`, `spot` | on the `space` node, as today (`set = "cathedral"`) | `point` once locations have points |
| `holder`, `world` | — | SCENE-PACKAGE §2, after #462 settles who a holder is |

- **Schema from one source ([contract]):** the Rust structs. deck-schema already writes the rig `Bundle`'s schema (`deck/catalog/schema.json`) and TS types; it adds `Scene` → `server/schema/content/scene.schema.json` (the server's save guard, `ContentSchemas`) + `shared/live` TS + PHP shapes via `server/schema/generate.ts`. `--check` in `deck:ci` and `npm run generate -- --check`. No hand-written twin.
- **Validity is the core's (A2):** `POST /api/scenes/<id>/publish` asks the build service's `POST /validate` with `kind: "scenes"` (as shows and rigs): unknown node kinds, broken links, missing references, nesting deeper than 4 or a cycle → `422 invalid_scene { problems }`. The JSON Schema on save is only the shape.
- **References and how they resolve:**

| Reference | Held as | Version | Resolved by |
|---|---|---|---|
| set | id (`cathedral`) | current (2026-11-02) | scene-content → CDN `.glb` per quality |
| character / portraits | registry id | current | scene-content |
| rig | `rig:<subject>` + `version` | pinned until pulled (2026-11-08) | scene-content (`version` = hash) |
| module | through its rig / node | by hash | the CDN `modules/<hash>.wasm` |
| media (video, audio) | asset id / CDN path | the path is the version | the signed media route (#470) |
| nested scene | `scene:<id>` | current in the document | the scene API |

  At publish the build service resolves everything into one frozen part (SCENE-PACKAGE §3); a deck plays that part and fetches only media.
- **Drafts vs published:** every save a new draft version; one published version per key; restore as for chapters. A draft is never served to a deck: it plays only in the editor where it is edited (2026-11-08).
- **Ownership:** platform-wide id (no toon segment: a scene can serve several toons and shows); `toons` for listing. Writers: role `creator`; publishing in the browser by a creator (G2: the MCP saves drafts only). Shows reference scenes; scenes never name a show.

## 4. Loading and running a scene by reference

| Step | Proposal |
|---|---|
| **Name it** | `scene:<id>` (newest published) or `scene:<id>@<v>` (pinned; what a flash sends). The same text in `--scene`, deck.toml `scene`, a cell's piece, the browser's `?scene=`. A bare name = a system scene (or a sandbox in dev mode). |
| **Fetch** | `GET /api/scenes/<id>[?v=<n>]` → `{ id, version, hash, scene, needs }` (public; the deck token when there is one). Then the existing scene-content sync for the set, portraits, rigs. |
| **Cache** | `<cache>/scenes/<id>@<v>.json`, like `<cache>/shows/`; a version already there is never fetched again; an unpinned name asks once per load and keeps the newest it has. Later the package store `<sha256>` (SCENE-PACKAGE §3) replaces it. |
| **Offline** | the newest cached version plays; nothing cached → the reason on the HUD and `test-pattern` (as a missing module today). |
| **Preload** | a flash already lists every piece of the deck's cells (`ready.rs`): it fetches the scenes too and reports `lacks`; `scene.sync` names scenes as well as sets. Nothing loads on the frame (A10). |
| **Swap** | as today: the new scene builds beside the running one, the switch crossfades (`stage.transition`); a scene that does not build keeps the old one playing. |
| **Replace the built-ins** | `BUILTIN` shrinks to the four system scenes; everything else comes from the API. No rebuild for a new scene; only a scene that needs a node kind the build lacks waits for a build (`carried_part`, #185). |
| **Hot reload in development** | three loops: a sandbox file with `--dev` (exists, `watch.rs`); a draft in the editor's stage, changed by patches in process (#244), no API call from the stage; a new published version reaches a deck at its next load or flash, never mid-show by itself. |
| **Security line** | who publishes: role `creator`, checked by the core. What a deck runs: published versions only; node kinds its build carries; modules only from the compile service, in their sandbox; media only by asset id from the CDN / signed route, never a file path or a free URL. A scene cannot change the deck configuration or the venue (its outputs are logical, [venue]). The package's signature comes with SCENE-PACKAGE P3. |

Today vs proposed, per platform: the native deck gets the scene API and the `scenes/` cache in `live/piece.rs` `find` and `read_scene`; the browser deck gets it in `space-web/src/outlet.rs` / `fetched.rs` (asked by the page, kept while it lives: no disk). One core decides (A1): `piece::load` takes `Found::Scene` either way.

## 5. Sandboxes vs real scenes (#427)

| Where | System | Sandbox (checks, demos) | Real scene |
|---|---|---|---|
| Source | the binary (4 TOML, `include_str!`) | files in the repo: `deck/native/scenes/sandbox/` (checks and demos), read by tests from there | `scene:<id>` in the DB, published |
| Deck, F5 | always, under "System" | only in dev mode (`--dev` or deck.toml `sandboxes = true`), under "Sandbox" | the published scenes this deck has cached or is assigned, by name |
| `--scene` | the name | the name or the file (dev mode) | `scene:<id>[@v]` |
| X1 image | in the binary | only on the `dev` update channel / image; the `stable` image carries none | fetched, cached on DECKDATA |
| Browser `?scene=` | the name | only on a dev build of the viewer | `scene:<id>` |
| Address book, flow, typeahead | never | never | listed per toon |
| A show's cell | `outlet` / `black` only | refused by the editor ("sandbox") | yes |

- **What the deck starts with when no real scene is assigned:** `idle` (as today). F5's Enter only loads; making it the start scene is a separate action (fixes #267's *"idle became cathedral"*).
- **A catch:** `cathedral` is also the room a rig is staged in (`Played::deck_scene` in `deck/space/src/rig/bundle.rs`; space-web `being_of` takes its `space` node for the browser's being). Before it leaves the built-ins, that room gets its own system default (the idle scene's space, or the assigned real scene's).
- **Today's test scenes:** the four system scenes stay; the nine checks and six demos move to `sandbox/` unchanged. The first real scene is made **from real data**: the cathedral set with Cat Pope's catalog rig, saved as `scene:nave` through the API (it is #243's round-trip test); `cathedral.toml` stays a sandbox. `conductor` follows once the conductor's scene is a real one.

## 6. Order of work

**Phase 1 – what still needs building** (proposed, not filed):

| # | What | Size | Done when |
|---|---|---|---|
| 1a | `scene:<id>` on the server: ID kind, `ScenesController` (GET, PUT draft, publish, versions), the schema from deck-schema, `kind: "scenes"` in the build service's `/validate`, MCP read / `save_draft` | M | **= #243, rescoped**: a draft saves; a broken scene is refused with the core's words; `scene:nave` round-trips into the same `Scene` (test); PHPUnit, PHPStan, `generate --check` green |
| 1b | Sandboxes apart: `sandbox/` folder, `BUILTIN` = 4, F5 groups + dev mode, "set as start" apart from load, the stable image without sandboxes, the rig's room no longer the cathedral's (§5) | S–M | **= #427's build**: a stable build lists 4 + real scenes; tests read from `sandbox/` |
| 1c | `scene` in the address book (kind, ref, address; `toons`) | S | the typeahead finds `scene:nave`; the generated types are regenerated |

**Phase 2 – #246** (published scenes on the decks):

| # | What | Size |
|---|---|---|
| 2a | Native: `scene:<id>[@v]` in `--scene` / deck.toml / cells; `<cache>/scenes/`; offline; tests (a new version replaces, an unchanged one is not fetched) | M |
| 2b | Browser deck: the same through `fetched.rs`, `?scene=scene:<id>` | S |
| 2c | Preload: the flash and `scene.sync` fetch scenes too; `lacks` names them | S |
| 2d | The package (SCENE-PACKAGE P3–P5: build on publish, the store, LAN / SD delivery) | L (three M) |

**Phase 3 – #244** (the deck stage as the editor's stage): edit mode in the wasm stage, load a scene document, pick, transform, patches out – **L**; it needs 1a. Its shell is #245 (open: creator, recommended there) and holders (SCENE-PACKAGE P1, M) wait for #462.

## 7. Questions for the owner

1. **Who may read a published scene?** *Recommended:* everyone, like the catalog and scene-content (a reader's browser deck needs it; readers save nothing); drafts never leave the editor.
2. **v1 without holders?** *Recommended:* yes – v1 stores today's scene graph as JSON, so #246 can start now; holders and world nodes come as format 2 after #462 decides who a holder is.
3. **Sandboxes stay files in the repo** (dev mode and dev image only, never in the DB or the address book)? *Recommended:* yes – they test code and change with it (E4).
4. **The first real scene is `scene:nave`** (the cathedral set + Cat Pope's catalog rig), and `cathedral.toml` stays a sandbox? *Recommended:* yes.
5. **A scene's toons:** a list (like an Experience's), empty allowed; a scene with none is reachable by id only? *Recommended:* yes.

Drafts: `267-scenes/1-scene-paths.html` in the session's notes folder (today vs proposed: where a scene comes from, where it is stored, how it reaches a deck).
