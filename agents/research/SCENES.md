Status 2026-11-12: design for #267 / #427 / #488; 1a built (#243: `scene:<id>`, `/api/scenes`, the schema from the Rust `SceneDocument`, the core's check, `scene:nave`, `Nudge.set` → `input` – server/docs/API.md "Scenes"); 1b built (#427: §5 "Built"); 1c built (#487); phase 1 of the leg "scenes and scene storage" (phase 2 #246, phase 3 #244). §1 is read from the code; §2–§6 are proposals until the owner answers §7; §8 reviews the words (scene, group, label, location, set, spot …) against the code, proposes names and a location model, with its own questions (§8.6); §9 (proposal, 2026-11-12) adds placeholder scenes, holders as format 2, the VN stage in the reader with its render ladder and composing in place (#488), and **replaces §6's phase 2 / 3 order** (§9.6), with its own questions (§9.7). The format and the package stay in SCENE-PACKAGE.md (#288, #393); this note is the whole picture and points there.

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
| **location** (`location:<id>`; stored as `place:<id>`, §8.3) | one set with its own state (CATALOG-LINK §8, owner 2026-11-10) | a scene is spawned at a spot of a location; words to the location reach the scene (§8.4) |
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
| `set`, `spot` | on the `space` node, as today (`set = "cathedral"`) | `location` beside or instead of `set` once locations carry their set; no third word such as `point` (§8.4) |
| `holder`, `world` | — | holders and placeholders as format 2 now (§9.3, owner 2026-11-12); `world` still after #462 |

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
- **Built (1b, #427):** `BUILTIN` = `idle`, `silent`, `outlet`, `test-pattern` (deck-space scene.rs); the 15 others are `deck/native/scenes/sandbox/` (`SANDBOX`; their texts compiled in only with the feature `sandbox`: the tests, a dev build of the browser deck). F5 lists the sandboxes in dev mode only (a sandbox folder: `--scenes`, the dev image's `/usr/share/deck/scenes/sandbox` – `DECK_SANDBOXES=1`, the dev update channel –, `--dev` the repo's), under their own heading; a sandbox name outside dev mode is refused with the reason. Enter loads, S sets the start scene. The browser's `?scene=<sandbox>` needs `npm run build:wasm -w shared/live-deck -- --sandbox`. The rig's room (owner 2026-11-11 `[scenes]`: rigs stand in a location's set or the character's home place, never the cathedral): `scenes/staging.toml` + `scene::staging(home)` – the character's home set when the deck knows it, else neutral (no set). Today no deck knows it: the registry's `home` / `home_scene` are not in scene-content, no character has a `home_scene`, and the only home scene with a 3D model is the cathedral.
- **Today's test scenes:** the four system scenes stay; the nine checks and six demos move to `sandbox/` unchanged. The first real scene is made **from real data**: the cathedral set with Cat Pope's catalog rig, saved as `scene:nave` through the API (it is #243's round-trip test); `cathedral.toml` stays a sandbox. `conductor` follows once the conductor's scene is a real one.

## 6. Order of work

**Phase 1 – what still needs building** (proposed, not filed):

| # | What | Size | Done when |
|---|---|---|---|
| 1a | `scene:<id>` on the server: ID kind, `ScenesController` (GET, PUT draft, publish, versions), the schema from deck-schema, `kind: "scenes"` in the build service's `/validate`, MCP read / `save_draft` | M | **= #243, rescoped**: a draft saves; a broken scene is refused with the core's words; `scene:nave` round-trips into the same `Scene` (test); PHPUnit, PHPStan, `generate --check` green |
| 1b | Sandboxes apart: `sandbox/` folder, `BUILTIN` = 4, F5 groups + dev mode, "set as start" apart from load, the stable image without sandboxes, the rig's room no longer the cathedral's (§5) | S–M | **= #427's build**: a stable build lists 4 + real scenes; tests read from `sandbox/` |
| 1c | `scene` in the address book (kind, ref, address; `toons`) | S | the typeahead finds `scene:nave`; the generated types are regenerated – **built (#487)**: published scenes only, per toon of `toons`, after the toons; label = the stage's name, `set` / `spot` / `version`; no creator event takes a scene, deck-show `PIECE_KINDS` for a cell's piece field (none yet) |

Phase 2 and 3 below are the first plan; **§9.6 replaces their order** (holders and uses first, the reader's stage beside the decks).

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

Drafts: `267-scenes/1-scene-paths.html` in the session's notes folder (today vs proposed: where a scene comes from, where it is stored, how it reaches a deck) · `267-scenes/2-words.html` (§8: the word map, the collisions, the names).

## 8. Words: scene, group, location, set, spot

Owner 2026-11-11 (`[scenes]`, thought aloud): *"Scenes currently in the script are labels and groups. So, but these groups are hopefully not problems later on for our tree in the editor. Please review that part. So we might have to call them something else."* Everything in §8 is a proposal. §8.1 is read from the code and the docs.

### 8.1 What each word means today

| Word | Story / creator | Catalog / map | Deck / show | Editor trees | XR |
|---|---|---|---|---|---|
| **scene** | the old word for a chapter group: DATA.md ("its groups = scenes inline"), stale comments in `shared/content` flow.ts / reach.ts / validate.ts / map.ts ("a scene in place of its ref"; ref groups ended 2026-10-19), `chapterSceneCharacters`, `SceneRefInput` (characters.ts), "isn't in the scene here" (creator VN hints); a **screenplay** scene `12 INT. …` (script.ts); the storyboard's Scene · Shot · Slice (content.ts:433); legacy keys `scene:<toon>:<chapter>:<name>` (`entity_versions_legacy` only) | **home scenes** (`home_scenes`, creator "Home scenes") = sets to be (#193) | **the deck scene**: one AV graph (`deck/space/src/scene.rs` `Scene`, TOML), `scene:<id>` (address.rs, a cell's piece); a rig played as a scene (`Played::deck_scene`); SHOW §4: the row replaced a "scene" level (Ableton's word for a row) | none | an OSCT scene inside its Experience (XR.md §5.4); VR scenes later = deck scenes one walks in (2026-11-10) |
| **group** | a part of a chapter (`Chapter.groups`, content.ts:500; creator "Groups", "Add group", the chapter tree); address book kind `group`, `chapter:<n>.<group>` (unconfirmed, #477); a Flow node kind `group` (flow.ts) | member groups (admin segments) | the relay group a client joined = its role / outlet (`Client.group`, protocol `group`, `AddressFilter.group`, SHOW §9 fact `group:`); an outlet's audio group (graph.rs "outlet group"; "the main group", A11); a rig = a module group (RIG-MODULE-GROUPS); a counted knob group in `expose` (params.rs); "switch groups" (OPEN-WORLD §4) | none | the open "unit that groups XR content" (2026-11-10) |
| **label** | a named point of a VN script (`label` event, vn-script.ts) and a group's jump name (`Group.label`, needed after a script that jumps away); a Flow node kind `label` | — | — | `TreeRow.label` = the text shown | — |
| (label, other) | mood / persona / look labels, credits "Label" = text a person reads | address book `labels` = text a person reads (A14: "what a person reads is a label") | knob hover names | — | — |
| **location** | events `unlockLocation`, `setAvatarLocation` (MAP.md) | address book kind `location`, ref `location:<id>`; CATALOG-LINK §8: one set + its state | SHOW §8 `location:*`; not an address place in address.rs yet | — | — |
| **place** | kanon's `place.<id>` (story variables) | the entity key `place:<id>` (entities.ts, MapRefs.php, AddressBook.php); creator "Map places", "Home (a place on the map)" | address.rs's grammar word: a level `<kind>:<id>` (`deck:`, `scene:` …, RULES A14 "places") | — | a surface's place on the stage (XR.md §5.9) |
| **set** | `setVariable`, `setAvatarLocation` (a verb) | a location's 3D staging (2026-10-30 "n per place", 2026-11-10 "exactly one set": the later wins) | the space node's `set = "cathedral"`; **also** `being.react[].set` = a control input a mood nudges (`feel.rs` `Nudge.set`); `set:` reserved later (address.rs) | — | — |
| **spot** | — | `RoomSpot` (LOCATION-SCENES §5: pickups, events in a room; not built) | an empty `spot_*` in a set's `.glb` (`glb.rs` `spots`); the space node's `spot` (where the being floats, the camera aims); `deck/show/src/validate.rs` `Spot` = where in a show document a problem is | — | — |
| **spawn point / point** | — | CATALOG-LINK §8, OPEN-WORLD §2.7: a named place in the set where a scene spawns, a slot claimed and released; two scenes may share one (owner 2026-11-10) | 2026-11-02: "a spawn where characters / entities enter"; SCENES §3 had "later `point`"; nothing in code | — | — |
| **room** | — | LOCATION-SCENES: "a 3D room for every place" | `Deck::room()` = the set the being floats in (deck.rs:142); `glb.rs` `RoomVertex`, `upload_room` | — | exhibition walls "as rooms" (XR §5.9) |
| **node** | a Flow node (group, label, choices …) in the creator's Flow | glTF nodes (empties become spots) | a graph node `[[node]]` of a scene or rig | the tree's third level: rig › track › **node** (main.rs:1444) | world nodes (SCENE-PACKAGE §2) |
| **outlet** | — | — | a show's stage outlet (a column, `?outlet=left`) = a role; the venue's logical outputs (audio groups, screens, light roles, MIDI, OSC; `deck_outlet`); the graph's `outlet` node; the system scene `outlet` | — | — |
| **role** | — | account roles (`live`, creator, admin, guest) | the show's role (`deck/show/src/roles.rs`: a stage outlet + its filter); the relay protocol's `Role` (conductor, output, bridge, performer, musician …: what a client is) | the Show tree lists roles | — |
| **piece** | collage `pieces` on a slice (`CollagePiece`) | — | what a cell plays: a scene, rig or module (SHOW §5, `piece.rs`) | — | — |
| **cell** | — | a look's image cell (character/persona/look/mood/view, characters.ts) | the show matrix's cell (row × outlet) | — | — |
| **show** | — | — | the live show (`show:<id>`, Show workspace) | show › seat › role › row | an **Experience type "Show"** (walls, screens, tabletops; XR §5.2) |
| **Experience** | — | — | — | — | the XR unit (`experience:<id>`), types Book · Show · Poster |
| **folder** | — | — | the folder bus (A11: a bus that sums children) | a track's icon in the tree | — |

### 8.2 The collisions that matter, ranked

| # | Collision | Who is hurt | What breaks | Proposal |
|---|---|---|---|---|
| 1 | **`scene` = deck scene and chapter group** (comments, DATA.md, names in shared/content) | agents above all: a "scene" in the story code reads as `scene:<id>` | a 1c typeahead or an MCP tool that lists groups as kind `scene`; a migration that matches `scene:` keys (the legacy ones look alike) | `scene:` and kind `scene` = the deck scene only. The story says **group**; the screenplay keeps "scene" for its headings, never as an entity or an address. A chore ticket fixes the comments, DATA.md and three names (`chapterSceneCharacters` → `chapterCast`, `SceneRefInput` / `normalizeSceneRef` → `CharacterRefInput` / `normalizeCharacterRef`: they are a VN character reference); no stored data changes |
| 2 | **XR scene as its own entity later** (owner's hope) | the address grammar | `scene:` cannot mean two kinds (A14) | an XR scene stays inside its Experience; if it ever becomes an entity, it gets its own word (§8.6 Q2), never `scene:` |
| 3 | **`set` twice in one scene document** (`space.set` the staging, `being.react[].set` a control input) | agents, the generated schema | once scenes are saved (1a), a rename needs a migration | rename `Nudge.set` → `input` now (only repo TOML has it) |
| 4 | **`group` = chapter group and the relay / filter group** (= role / outlet) | the address book vs the show's filters | `group` as a kind (1c) and as a filter fact (SHOW §9) | the chapter keeps `group` (stored in every chapter); the show's fact becomes `role` (protocol, deck/show; a migration only if a saved show holds `where.group`) |
| 5 | **location / place** (key `place:`, ref `location:`, creator "Map places", address.rs's grammar "place") | everyone | an agent writes `place:` into an address or `location:` into a key | **location** is the one word for people and references; `place:<id>` stays only as the storage key (a later migration, optional); the grammar's "place" stays (A14 uses it) |
| 6 | **spot / spawn point / point / room** | the scene document, the editor | a third field for one idea | **spot** is the stored word; a spawn point is the spot a scene sits at; `room` in code reads as set |
| 7 | **label** = a jump name (story) and text a person reads (A14, address book, trees) | agents | a label edited "for display" breaks a jump | no rename (every chapter and script stores it); the word rule in §8.5 |
| 8 | **Experience type "Show" and the live show** | people and agents, once XR is built (possibly from 2026-11-25) | "a Show has walls" next to "show › row › cell" | rename the Experience type before it is built: **Exhibition** (the owner's own example; §8.6 Q3) |
| 9 | **role** three times, **outlet** twice, **piece**, **cell**, **node** | low: each stays inside its domain | — | keep; the table in §8.1 is the reference |

**The owner's question: do the script's groups clash with the editor's node tree?** Not today, and they need not later. The native editor's tree is rig › track › node (`deck/editor/src/main.rs`) and, in the Show workspace, show › seat › role › row (`show/author.rs`). Neither has a group. The clash would come if a scene's node tree got a "group" item to bundle nodes, as 3D tools do. So: **the node tree never says group.** A container is a track (inside a rig) or a holder (a nested rig or scene). A summing bus is already a folder bus (A11 avoided Ableton's "group track"), and a purely visual bundle, if one is ever needed, is a **folder**. "Nodes", as the owner said, is the concept already: everything in the tree below a track is a node. The story keeps **group** (a part of a chapter) and **label** (a named point). A chapter shown in a scene (`chapter` node, `deck/space/src/chapter.rs` reads `groups[].slices`) lists its groups under that node in the story's own word. The creator's chapter tree and Flow already show groups and labels; that is the story's tree, not the editor's.

### 8.3 Names, smallest change first

| Change | Where | Kind of change | Migration |
|---|---|---|---|
| `Nudge.set` → `input` | `deck/space/src/space/feel.rs`, the scene TOMLs | code + fixtures, before 1a – **done in #243** (no alias) | none |
| "scene" for a group → "group" | comments in shared/content, DATA.md; `chapterSceneCharacters`, `SceneRefInput`, `normalizeSceneRef` (shared/content, creator chapter-editor, viewer visual-novel) | words and three names | none |
| `room` → `set` in deck-space words (`Deck::room`) | deck/space | code words | none |
| the show's filter fact `group` → `role` | deck/protocol `AddressFilter`, deck/show address.rs, SHOW §9 | protocol (A9: no back-compat) | only if a saved show uses `where.group` |
| Experience type "Show" → "Exhibition" | XR.md, decisions | words, before the build | none (nothing built) |
| `place:<id>` → `location:<id>` as the entity key | entity_versions, MapRefs, AddressBook, the creator's map | later, optional | yes (E2: SQL migration of keys) |

Address prefixes (A14): `scene:<id>` the deck scene · `location:<id>` a location (an address place once a deck receives it, CATALOG-LINK §8) · `set:` and `spot:` stay reserved in address.rs, not used: a spot is written below its location, `location:<id>/spot:<name>` only if a receiver ever needs it · `chapter:<n>.<group>` the group's reference (no address: nothing receives it) · `experience:<id>` the XR unit · no `group:`, `place:` or `point:` in addresses.

### 8.4 Locations in scenes

- A **location** (`location:<id>`) is a place of the story world on the map, with its state (intact or destroyed, the hour) and **exactly one set** (owner 2026-11-10).
- A **set** is the location's 3D staging: geometry, lights and its **spots**, named points (`spot_<name>` empties in the `.glb`, or picked in the editor). Every spot can carry stills, video or 3D (2026-10-30). The catalog's home scenes become sets (#193).
- A **scene** (`scene:<id>`) is placed at **one spot** of one set (v1: `set` + `spot` on the space node, as today; later `location` + `spot`, the set following from the location). That spot is its **spawn point**: the cast, items and events are placed relative to it, and two scenes may share it. Entrances for a scene's cast are further spots, named per holder when holders come (#462).
- **Scope** is `location › scene › entity`: a word or state on the location (`location:blocks > destroyed`) reaches every scene placed in it.
- **A deck plays a scene, never a location.** No story jump targets a location today (the jump types are chapter, season, toon, product, xr; locations have `unlockLocation` and `setAvatarLocation`, which move the reader's map and avatar). So "jump to a location" for a deck means: play the scene placed in that location (its default scene), crossfading as any switch (§4); a location without one is an address with no receiver, which stays silent and is logged (2026-11-10 `[address]`). The reader's own location events stay the reader's.

### 8.5 A word rule (proposed for RULES A14 or A11, not written there)

> Words: **scene** and `scene:` = the deck scene only; a part of a chapter is a **group**, a named point in a script a **label** (a jump name, not display text); a map place is a **location** (`location:`; `place:` is only its storage key); a location has one **set**, a named point in it is a **spot** (a scene's spawn point is its spot); the editor's tree has rigs, tracks, nodes and folders, never groups.

### 8.6 Questions for the owner

1. **The story keeps "group" and "label"; the editor's tree never says group (folder if ever needed).** *Recommended:* yes, as in §8.2 (no migration, the creator's words stay).
2. **XR scenes, if they become their own entity:** a word of their own, never `scene:`? *Recommended:* they stay inside their Experience (as decided 2026-11-10) and are addressed through it; a separate key only when a use needs one.
3. **Rename the Experience type "Show" to "Exhibition"** before XR is built? *Recommended:* yes; "show" stays the live show.
4. **One word "spot"** for every named point in a set, the scene's spawn point being its spot (no `point`, no separate spawn type)? *Recommended:* yes.
5. **A jump to a location on a deck** plays that location's default scene? *Recommended:* yes; a location names one default scene, and without one nothing plays (logged).

## 9. Placeholder scenes, holders and the VN stage

Owner 2026-11-12 (`[scenes]`): *"The combiner is programmatic. It is not intended to be a separate editor."* · *"place holder scenes is an important point ... these are used for multiple other entities to be the layout."* · *"So they can be populated by any or a filtered variety of entities."* · question 4: *"4b where we differentiate is how it is rendered. Because it doesnt use the slice backgrounds."* Everything in §9 is a proposal; §9.1 is read from the code.

### 9.1 Today (from the code)

| Thing | Today | Where |
|---|---|---|
| a scene's cast | one `being` (a look: the floating being); a rig's nodes flattened into the graph by hand (`scene:nave`: `voice_*`, `drone_*`) | `deck/space/src/scene.rs` `Scene.being`; `server/tests/fixtures/scenes/nave.json` |
| holders | none; SCENE-PACKAGE §2 sketches `[[holder]]` (place, rig, version, values), its P1 is not built | — |
| a holder's address | its filler's place: `/deck:pc/scene:demo/character:catpope/…`; a second copy (`#2`) is reserved and refused | `deck/space/src/rig/address.rs` |
| the VN stage in the reader | 2D: `joinScene` / `leaveScene` (`characters: VNCharacterRef[]` = character, persona, look, mood, variant) put characters on the dialogue box (avatars left, portraits right); `showMessage` (character, persona, mood) puts the speaker in front | `shared/content/src/visual-novel.ts`; `viewer/src/lib/visual-novel/stage.ts`, `script-runner.ts` |
| where a group is | no field: a group has label, renderer (vertical / horizontal), slices, events, continues, end; its cast is read from its events (`chapterSceneCharacters`); `setAvatarLocation` moves the reader's avatar on the map, not a stage | `shared/content/src/content.ts` `Group` |
| a location's set | none: a map place (`MapPlace`) has a diorama image for the map, no set, no scene | `shared/content/src/map.ts` |
| a set's sources | the home scenes catalog: `image` (a still in one-point perspective, 1080p / 4K), `video` (a muted loop over the still), `model` (`.glb` per quality; only the cathedral has one) | `shared/content/src/homes.ts` `HomeScene` |
| a character's home | `home` (a location), `home_scene` (a home scene) | `shared/content/src/characters.ts` |
| filters | the address book: an asker takes the kinds it may pick (`AddressKind`); a show role's `for` = `Admits` (account, approved, device, group: facts about relay clients, any one fact admits) | `server/schema/api/address-book.schema.json`; `deck/protocol` `Admits` (#478) |
| the browser deck | only on `/live/deck/<code>`; the chapter page has none | `viewer/src/routes/live/deck` |
| the quality ladder | the governor's rungs (presets low · medium · high · ultra), the deck's level the ceiling, a step down under load | `deck/space/src/quality.rs` |

### 9.2 Placeholder scenes

| Word | Means |
|---|---|
| **holder** | a place on a scene's stage that carries one filler at a spot (SCENE-PACKAGE §2) |
| **filler** | what fills a holder: a character (with persona and look), an item, a nested scene; for the scene's location, a location |
| **placeholder** | a holder (or the scene's location) with no filler, only `fill`: the filter of what may fill it |
| **concrete scene** | every holder filled, the location named: it plays as it is |
| **placeholder scene** (template) | at least one placeholder: it plays through a use, or in the editor and the browser with its preview fillers |
| **use** | a reference to a scene with its fillers, stored in the document that uses it: `{ "scene": "scene:stage", "with": { "where": "location:dom", "left": "character:catpope" } }` |

**The filter (`fill`).** Chosen shape: the address book's typeahead question, stored – not a role's `Admits`. What fills a placeholder is a pick from the address book (#477); an asker there already filters by kinds, and the facts below are an entry's own fields. `Admits` filters relay clients (account, device, group) and admits on any one fact, so "a character, and one of these" cannot be said with it.

| Fact | Values | Example |
|---|---|---|
| `kind` | `AddressKind` list; required | `["character"]`, `["item"]`, `["location"]`, `["character", "item"]`; "any" = every kind a holder carries |
| `ref` | a hand-picked list of references | `["character:catpope", "character:toptoy"]` |
| `parent` | entries below one reference | `"character:catpope"` (her personas and looks) |

Within a fact any value admits; every fact given must hold. "A toon's cast" is `kind: character` in the book of the toon that uses the scene. "Only items of a kind" (furniture) needs a field the book does not have yet (an item's category): it comes into the book first, then as a fact of the same name.

**Filling, three moments:**

| When | Who fills | Example |
|---|---|---|
| authored | the use names fillers (`with`) | a group's location; a show cell's `who` = Cat Pope |
| context | the receiver fills from what it knows | the avatar scene: the reader's current avatar and its home (save game, decisions/items.md `[cave]`); a location's default scene: the location; a character page: the character |
| story | the VN stage while the reader reads (§9.4) | `joinScene` Cat Pope → the first empty placeholder that admits her |

An empty placeholder plays nothing (its spot stays empty, silent); an empty location = the neutral staging (1b's `staging`). The editor's typeahead offers only fillers that pass `fill` (a hint); the server refuses a use whose filler does not (validity, A2). **The avatar scene** (draft 6) is the template `scene:avatar-home`: the location `home` and the holder `who`, both filled by context; "the avatar" is no special kind.

### 9.3 Holders now: format 2

| Field | Format 1 (1a) | Format 2 |
|---|---|---|
| `format` | 1 | 2; the core reads 2 only (A9) |
| `location` | — | `"location:<id>"`, or a placeholder `{ "fill": { "kind": ["location"] }, "label": { … }, "preview": "location:dom" }`; the set follows from the location (one set, §8.4) |
| `space` node `set` | the set | only for a scene with no location (a neutral staging, a set not yet a location's); both at once is refused |
| `space` node `spot` | the spot | the scene's spot = its spawn point (unchanged) |
| `holder[]` | — | `id` (its name in the document and in a use's `with`; not an address) · `spot` (where it stands and enters; missing: the scene's spot) · `ref` (a filler) **or** `fill` (a placeholder) · `rig` (`{ "ref": "rig:<subject>", "version": n }`; none = the filler's own rig, pinned when the use is saved) · `values` (values and preset choices only, SCENE-PACKAGE §2) · `label` (a placeholder's name for people) · `preview` (a filler for the editor and the browser) · `enter` (`start`, the default, or `join`: waits for the story) |
| `being` | one look | leaves the document: every character holder brings its look |
| `node`, `link` | the rig flattened in by hand | the scene's own nodes (space, mixers); the holders flatten in the core's resolve step, prefixed by the filler's place (SCENE-PACKAGE §2) |
| `world` | — | still later (#462: cables between holders, far addresses) |

- **Addresses:** a filled holder is its filler's place (`scene:stage/character:catpope/…`, as address.rs has it); an empty placeholder has no address – nothing can receive a word there (2026-11-10 `[address]`). The same filler twice in one scene stays refused (`#2`).
- **Migration (cheap now):** only `scene:nave` exists. Its fixture becomes format 2 (no location yet: `space.set = "cathedral"`; one holder `catpope`, `ref` `character:catpope`, its rig pinned; the hand-flattened nodes leave). In production: if the row exists, an SQL migration rewrites it (E2); otherwise only the fixture and API.md's one-off change.
- **The played `Scene`** (what the resolve step makes, and what the system and sandbox TOMLs are) keeps its `being` until the stage can draw several looks; drawing one look per character holder at its spot is its own ticket (§9.6, 2·1).
- The address book's scene entry gains `placeholders` (the kinds of its open slots), so a typeahead can show a template as one.

```json
{ "format": 2, "id": "stage", "toons": [], "stage": { "name": "Stage" },
  "location": { "fill": { "kind": ["location"] }, "label": { "en": "where" } },
  "holder": [
    { "id": "left",  "spot": "spot_left",  "fill": { "kind": ["character"] }, "enter": "join" },
    { "id": "right", "spot": "spot_right", "fill": { "kind": ["character"] }, "enter": "join" } ],
  "node": [ { "id": "space", "kind": "space", "spot": "spot_centre" } ] }
```

### 9.4 The VN stage in the reader

**Who hears what:** the reader's script runner is the source of the words; the receiver is the reader's deck – the browser deck running in the chapter page, one core (2026-11-10 `[address]`: only the deck in the reader takes part; A1: the core decides). A paired deck (TV, X1) receives the same words through the relay in the reader's own session and draws the same stage at its own rung.

| Event | Exists | On the stage |
|---|---|---|
| a group with a `scene` comes on screen | the field is new (a use, §9.5) | the stage switches with a crossfade (§4); a group without a scene keeps the last one, "No scene" stops it (decided 2026-11-12) |
| `joinScene` | yes | per reference: the holder that already carries the character shows it; else the first empty placeholder, in the scene's order, whose `fill` admits it takes it – at its spot, with its look and rig; persona, look and mood as given |
| `leaveScene` | yes | the holder empties (a placeholder) or hides (an authored filler) |
| `showMessage` | yes | the speaker's holder takes the focus (the camera turns to its spot) and the mood; narration: none |
| `setAvatarLocation`, `unlockLocation`, `migrate`, `giveItem`, `takeItem`, `jumpTo` | yes | not the stage: the map, the carrier bag, the reader's way (unchanged) |
| new events | — | none for v1. Later, only if needed: an optional `holder` on `joinScene` (a precise place) and items on stage – still no new type |

**Where it shows (owner: (b), not on the slice backgrounds):** the stage is the picture of the group that names it – full viewport, like a full-viewport slice video – with the script's message box over it; the group's slices do not lie behind it (§9.7 Q6). The 2D stage of today (stage.ts) stays for groups without a scene.

**The render ladder** – what the reader draws, by the sources the catalog and the chapter editor have for this scene:

| Rung | Needs | Where the source lives today | The cast is drawn as |
|---|---|---|---|
| **1 · 3D** | the location's set as `.glb` (per quality) | home scenes `model` (`HomeScene.model`, #193; only the cathedral) → scene-content | 3D bodies where a character has one (CHARACTERS3D), else its look as a standing card at the spot |
| **2 · video** | the set's loop | home scenes `video` (1080p / 4K), its still as the poster | looks (`full` view per mood: `media/characters/<id>/<id>_full-<mood>`) standing on the floor at the spots' foot points |
| **3 · 2D** | the set's still in one-point perspective with floor (`[cave]`) | home scenes `image` | as rung 2; items by `furniture.image` and `size` (homes.ts), as the cave places them |
| **4 · sound only** | the scene's graph and the holders' rigs | the scene document, the catalog's rigs | today's 2D VN stage over the group's slices (stage.ts); the scene plays under them |

- **Sources:** the catalog holds the set (one home scene entry per location: a new `set` field on the location, CATALOG-LINK §8), the looks, the bodies, the rigs. The chapter editor holds the use (the group's Scene field and its fillers) – the scene, not the art. A per-group stage video of its own is a possible later field, not proposed now.
- **The device moves it down, too:** the rung = the richest one that the sources allow, that the device can draw, and that the reader's settings allow. No WebGPU / WebGL2 → no rung 1; the governor already at its lowest preset and still over budget → one rung down (1 → 2), as it steps presets today; reduced motion → 3 instead of 2 (the home view's rule today); the reader's media quality picks the files within a rung (I3), never the rung itself. A paired deck usually stands on rung 1.
- **One renderer:** every rung but 4 is drawn by the core (the browser deck's stage: a `.glb` room, a video plane or a still plane, the looks as cards) – one owner of the cast picture (A1, A6); rung 4 is today's reader.

### 9.5 Composing in place (#488)

No combiner Sheet (draft 5 is off). Adding a scene where it is needed stores a **use** – a template and its fillers – in the document that needs it. A new `scene:<id>` only when the author changes more than fillers: "Save as a scene" makes a derivative naming its origin, edited in #244's stage.

| Where | Field | Context fills | Template (default) | The author adds |
|---|---|---|---|---|
| a show's row editor (a cell) | the cell's piece | the show's toons (the book asked) | `scene:solo` | `who` per outlet, a location |
| a location (creator, map) | Default scene | `where` = the location | `scene:place` | a cast, if any |
| a reader group (chapter editor) | Scene (draft 4) | `where` = the previous group's location, typed over; the cast slots stay for the story | `scene:stage` | the location |
| a character (creator) | Home scene: where its rig plays on its own | `who` = the character, `where` = its `home` | `scene:solo` | nothing |
| the reader's home (the cave) | — | `who` = the reader's avatar, `home` = its home | `scene:avatar-home` | nothing |

The templates (system placeholder scenes – published scene documents with no toon, listed in every toon's book; §9.7 Q2):

| Template | Location | Holders |
|---|---|---|
| `scene:place` | placeholder | none: the set's own sound and light |
| `scene:solo` | placeholder | `who` (character or item) at the scene's spot |
| `scene:stage` | placeholder | `left`, `centre`, `right`, `back` (characters, `enter: join`); a spot the set lacks → the scene's spot |
| `scene:avatar-home` | `home` (placeholder) | `who` (character) at `spot_home` |

### 9.6 The order of work (replaces §6 phase 2 / 3)

| # | What | Size | Done when |
|---|---|---|---|
| 2·0 | **Format 2:** `location`, `holder[]` with `ref` / `fill`, the filter, the schema from Rust and the generated types, the resolve step for referenced rigs (SCENE-PACKAGE P1 without embedded rigs), the core's checks (holder ids, filter kinds, missing spots warn), `scene:nave` migrated | M | nave as format 2 flattens to the `Scene` it played as format 1 (headless); `deck:ci`, `generate --check` |
| 2·1 | **Several looks on the stage:** deck-space draws one look per character holder at its spot (today one being), cards on rungs 2 and 3 | M | `scene:stage` with two fillers shows both, native and browser |
| 2·2 | **The use:** `{ scene, with }` in the contract; the server checks the fillers against `fill` in the toon's book; the book's scene entry gets `placeholders`, templates in every book; the four templates published | M | a use with a filler of the wrong kind is refused with words |
| 2·3 | **Sources:** a location names its set (`set` → a home scene entry); scene-content carries a scene's set, looks and rigs per rung | S | `location:cathedral` → the cathedral's `.glb`, still, video |
| 2a–2c | as §6: native, browser deck, preload – now resolving a use's fillers and the location's set | M + S + S | as §6 |
| 2·4 | **The reader's stage:** the browser deck in the chapter page; a group's use plays full viewport with the message box over it; `joinScene` / `leaveScene` / `showMessage` to the core; the ladder (rungs 2–4 first) | L (M + M) | a test chapter: Cat Pope joins → she stands at `left`, her rig sounds; reduced motion drops video to the still |
| 2·5 | **Rung 1 in the reader:** the 3D set on a phone, the governor's step down to rung 2 | M | the S22 holds rung 1 or steps to 2 without a stall |
| 2·6 | **Creator fields:** group Scene, location Default scene, character Home scene, the cell's piece with fillers (deck editor); the Scenes browser (draft 3) with a Template filter | M | each composes a use from its context |
| 2·7 | **A paired deck follows its reader** (the reader's session, the stage words) | M | the TV shows the same stage, rung 1 |
| 2d | the package (SCENE-PACKAGE P3–P5) | L | as §6 |
| 3 | #244: the editor's stage with holders in the tree (scene › holder › track › node), placeholders dashed, "Preview as" | L | as §6 |

**What of the merged 1a–1c changes:** 1a – `FORMAT` 1 → 2 in `scene_document.rs`, the schema and generated types, the fixture, API.md's one-off (2·0). 1b – `scene::staging(home)` becomes `scene:solo` filled by context, once 2·2 is in; until then it stays. 1c – scene entries gain `placeholders`, templates are listed in every book (2·2). #462 keeps cables between holders, decks' roles and values between decks; "scenes holding rigs" is answered here.

### 9.7 Questions for the owner

1. **The ladder's order: 3D → video → 2D → sound only, and the device moves it down** (no GPU, the governor at its floor, reduced motion), never the reader's media quality? *Recommended:* yes.
2. **Four templates** (`scene:place`, `scene:solo`, `scene:stage`, `scene:avatar-home`), published with no toon and listed in every toon's book? *Recommended:* yes; more only when a use needs one.
3. **Adding a scene in place stores a use** (template + fillers) in the group, cell, location or character; a new scene document only when more than fillers change? *Recommended:* yes.
4. **Casting:** `joinScene` fills the first empty placeholder that admits the character, in the scene's order – no new event (an optional `holder` later)? *Recommended:* yes.
5. **The filter:** the address book's kinds plus `ref` / `parent`, every fact must hold (unlike a role's `for`, where any one admits)? *Recommended:* yes.
6. **A group with a scene shows the stage instead of slices** (full viewport, the message box over it); comic and stage are separate groups, linked by a chain? *Recommended:* yes.

Draft: `267-scenes/7-placeholders.html` in the session's notes folder (a placeholder scene, what fills it, the VN stage over a chapter's events, the ladder).
