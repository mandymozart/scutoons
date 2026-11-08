Status 2026-11-08: concept for the owner's review (#288, #393), undecided, nothing built. One graph format for instruments, rigs and scenes; scenes hold rigs by reference or embedded; a signed package built from those documents is what gets flashed to decks.

# SCENE-PACKAGE – how scenes are stored, and how a config gets onto a deck

Owner 2026-11-08: "if we can use this for smart storage of scenes, since we have to define a format that stores how scenes are build and if rigs can be embeded configurations we might have a way to easily flash configs to devices." ("to decks"). Follows the decision that a wiring is a graph document (`[rigs]` "graph."), MODULE-WIRING.md (addresses, world nodes) and EVENTS-VS-PARAMS.md.

## 1. What exists (read from the code, 2026-11-08)
| Piece | Today | Reuse |
|---|---|---|
| Scene | `deck-space` `Scene`: `[stage]`, `[[node]]` (id, kind, params), `[[link]]` (from, to, scale, offset, map), `[being]`, `outputs`. TOML files in `native/scenes/`; 0.4–10 KB each. Flat. | The node and link vocabulary |
| Rig | `Bundle`: rig + instruments + presets (with ancestors) + a code rig's module descriptors. Served by `scene-content`; 0.8–8.5 KB. `flatten` already turns a rig into one playable `Scene`. | The embedded form, and the flatten step |
| `scene:<id>` | **Decided, not built** ([scenes-from-api] 2026-11-02): an entity with versions and drafts. `EntityKey` does not know the kind yet (the old `scene:<toon>:…` documents were retired). | The right moment to define it |
| Modules | `modules/<hash>.wasm` on the CDN, content-addressed, about 10 KB gz for a small one. | Stored once per hash |
| Media | Immutable CDN paths; `sync.rs` fetches what the cache lacks and plays offline. | As it is |
| Safe delivery | `deck/update`: manifest with a SHA-256 per file, Ed25519 signature, channels `dev` / `stable`, a dev PC found on the LAN. `allowed()` limits it to the deck's own program paths. | The signing, the channels and the LAN path; not its `allowed` |

Decided and respected here: a scene stores references to sets and uses their current version (2026-11-02); scenes write to logical outlets, and the room is the venue's document ([venue]); a deck may refuse for capability; a new module is taken only by restart (A9); rewiring on a deck is forbidden; the API is the only source (E3, A6); real scenes and sandboxes are separate (2026-11-08).

## 2. The format: one graph, three levels
Instruments, rigs and scenes all use the same `node` and `link` structs. A level adds only what it needs. Scenes get two new things: **holders** and **world nodes**.

```toml
# scene:nave (the TOML view; stored as JSON)
format = 1
[stage]
name = "Nave"
transition = "fade"

[[holder]]                         # a rig used by many: a reference
place = "character:catpope"
rig = "character.catpope"
values = { "voice:voice.index" = 4.0 }   # only values and preset choices may differ

[[holder]]                         # a rig only this scene has: embedded
place = "item:incense"
[holder.rig]
track = [{ id = "hiss", instrument = "breath" }]
# … the same Rig, instruments and presets as a Bundle

[[world]]                          # world node: the only place a name or a far address stands
id = "midi"
source = "midi"
listen = ["midi.note"]

[[node]]
id = "sky"
kind = "space"
set = "cathedral"                  # a reference, current version

[[link]]                           # addresses as in MODULE-WIRING §3
from = "character:catpope/@energy"
to = "item:incense/hiss:noise.gain"
```

- **Who may change what (corrected 2026-11-08 after the owner's question):** a *holder* is the place on a stage that carries a rig (character, item, scene, later show or world), not a person. A holder that references a rig changes values and preset choices. Any change to the graph is the moment of a **derivative**: the editor asks "used by N: change it for everyone, or make a version for this holder?", and the second makes an embedded rig that names its origin and version (preset.rs's lineage, #350's derive). A copy with no origin is refused (A6). Deriving copies only the rig's own graph, a few KB; instruments and modules stay shared references. "Extract to catalog" makes it reusable. For this a rig's `subject` must move out of the rig and onto the holder, so any holder can use any rig.
- **Holders may hold scenes** (`scene = "scene:choir"`), which gives the nesting the editor needs. It stays depth-limited and refuses cycles.
- **One resolve step, in the core:** holders flatten to the one `Scene` the engine plays, using today's `flatten` with `<place>/` as the prefix. The engine, knobs and relay keep compiled slots and never see nesting.
- **Rust where it must be:** a module stays code. A code rig's marked table (the #203 answer, W3) stays in its Rust and arrives in the package as that module's descriptor, so "graph" and the marked table do not collide: documents hold graphs, modules hold code.
- **JSON is the storage, TOML the view.** You asked whether TOML makes sense long-term for saving: not as storage. The database holds JSON documents, a package needs one exact byte form to hash and sign, and TOML has no null and loses comments anyway. As a second view of the nodes it is fine, generated from the same structs, so the two are the same thing by construction.
- **The schema comes from the Rust structs** (`deck-schema`, as decided for the node catalog), and `format` is a plain integer. A deck refuses a format it does not know (A9); documents in the database migrate by SQL (E2).

## 3. The smart part: a thin manifest over shared parts
Storing is like git, not like a zip of everything. Every resolved scene, rig and module is a **part** named by its SHA-256. A **package** is a small manifest listing parts.

```json
{ "format": 1, "id": "cathedral-show", "build": 1762600000, "channel": "stable",
  "scenes": { "nave": "sha256:…", "choir": "sha256:…" }, "start": "nave",
  "modules": ["sha256:…"], "media": [{ "path": "cdn/path/set.medium.glb", "bytes": 48211200, "sha256": "…" }],
  "needs": { "abi": 2, "features": ["space", "modules"] },
  "from": { "scene:nave": 412, "rig:character.catpope": 88 } }
```

- **A deck keeps one store** (`/var/lib/deck/store/<sha256>`). Ten shows using one module hold it once, and a new version of a show fetches only the parts that changed. That is what makes it quick.
- **A resolved scene part is self-contained:** rigs inlined as `Bundle`s, exact versions. A deck needs nothing else to play it, so it plays with no network.
- **It is a build product, never a source.** Nobody edits a package. `from` records exactly which document versions went in, so it is traceable and rebuildable.
- **Size (estimated from the files above):** 20 scenes with their rigs about 200 KB, plus 20 modules at 10–50 KB. So about 1 MB of config, sent in about a second. Media (a set's `.glb`, HAP video) is listed, not included, and is the only heavy part.
- **Two forms of the same package:** *config* (parts only, the default, media comes through `sync` beforehand) and *full* (media included, for an SD card or a venue with no internet).

## 4. Flashing to a deck
1. **Publish in the browser** as today. Build a package from published versions (§6, question 3).
2. **Deliver**, by any of three ways, all the same bytes: the deck pulls it with its read-only token; a dev PC on the LAN offers it (the path `deck-update` already has); or an SD card / USB stick.
3. **The deck checks before anything changes:** the signature, every part's hash, and `needs` against its own build (it may refuse for capability, [venue]). A refusal is shown and the old config stays.
4. **It waits for a safe moment:** only between sessions, never during a show, as with updates. Then one atomic switch and a restart of the stage (A9). The previous package is kept for a one-step rollback.
5. **It reports** what it runs, up to the conductor and the board.

**Trust.** The manifest is signed with Ed25519 like an update, with a **separate key from the release key:** a config must never be able to replace the deck's programs, and it writes only under `/var/lib/deck`. `dev`-channel packages may carry locally built modules and are accepted only by decks on the `dev` channel. A `stable` package holds only modules from the compile service. The modules stay in their sandbox either way.

**What stays out of the package:** the room. Speaker layouts, DMX, channel maps and displays are the venue and deck configuration documents. They are delivered the same way but as their own package kind, so one show fits many rooms. A reader's or member's data never goes in.

## 5. What this gives the native editor (#393) and the sandboxes
The editor's local build makes a `dev` package and offers it on the LAN: change a rig on the PC and it plays on a real X1 seconds later, with no server and no separate path. The split into sandboxes and real scenes becomes the channel: sandboxes are `dev` packages, real scenes are `stable`. The TOML files in `native/scenes/` shrink to the idle scene and test fixtures, as already decided.

## 6. Questions for the owner
1. ~~Pinning.~~ **Decided by the owner 2026-11-08:** documents keep "current" (as decided 2026-11-02); a package freezes what current was when it was built, and rebuilding it brings the new versions.
2. **Changing a shared rig.** *Recommend §2:* the derivative is the switch. No overlay that patches a rig from outside: it keeps the origin's updates, but breaks when the origin changes. The price is that a derivative gets no updates by itself; the editor shows "origin has a newer version".
3. **Who builds and when.** *Recommend:* the Rust build service builds it (the resolve step is the core's, no second copy in PHP), on a "Build package" action after publishing, not automatically at every publish.
4. **Media.** *Recommend:* config by default, full only when asked for.
5. **Nesting depth.** *Recommend:* a limit of four levels until the scene editor is designed.

**What a package is of:** one scene is enough for the first version. A show (several scenes, assigned to several decks) waits for the stage builder, and the format already allows it.

## 7. Slices (proposed, not filed)
| # | What | Size | Done when |
|---|---|---|---|
| P1 | `holder` and `world` in the structs, the resolve step in the core, the schema | M | a scene with one reference and one embedded rig flattens to the same `Scene` as today's hand-written TOML (headless) |
| P2 | `scene:<id>` in the store, `EntityKey`, the core's validation | S–M | a draft saves, a wrong scene is refused with the core's sentence |
| P3 | Package builder: parts, manifest, signing with its own key | M | the same inputs give byte-identical parts |
| P4 | The deck's store: verify, atomic switch, rollback, report | M | a broken signature changes nothing; a power cut mid-install leaves the old config playing |
| P5 | Delivery by LAN and SD card | S | **the proof below** |
| P6 | `dev` packages from the native editor | S | after #393's proof |

**The proof (after P1–P5):** one scene with catpope by reference and the incense burner embedded, packaged on the PC, put on the X1 by SD card with the network off. It plays after a cold boot; a second version with one changed rig transfers only that part.
