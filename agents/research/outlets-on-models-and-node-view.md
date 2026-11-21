Status: research 2026-11-21, plan for #712 – not built

# Outlets on 3D models, and the node view's drag-and-drop with Blender-style sockets

Owner 2026-11-21 (#712): *"which tools are missing to place outlets onto 3d models. And secondary about the node view i want all
the drag and drop any type of asset into the node view and the blender style outlets etc. All the stuff that we researched for
osct. And what we got already built there."*

Read from SCUTOONS `develop` at 2ba378fb8 and OSCT (read only, reference only: no code crosses, decision 2026-11-20) at
8525607b. Nothing here is built; section 4 lists ticket titles to file, section 5 the owner's questions.

**Words used below.** "Outlet" already means three things in SCUTOONS: the **Live outlet** (a display a deck shows cells on,
`deck/outlet`, decisions/deck.md 2026-11-10 [outlet]), a node's **outlet** pin (deck-kit `graph.rs` inlets / outlets) and the
owner's "outlets on 3D models". RULES A14 wants one word per thing. This doc calls a named point on a model a **spot**, the word
the stage already uses for a set's named empties (`spot_*`, `glb.rs`) and that address.rs reserves as a later place kind
(`spot:`). Question Q1 asks the owner.

---

## 1 · What exists

### 1.1 SCUTOONS: points on models, 3D and gizmos

| What | State | Where |
|---|---|---|
| Spots in a **set** GLB: named empties `spot_*`, `view` read at load | built | `deck/space/src/space/glb.rs` (header), `scene_document.rs` `STAGING_SPOTS`, `Holder.spot` + offset in metres |
| GLB loader | built, **flattened**: one world-space vertex list; normals, **skins, morph targets, animations** ignored | `glb.rs` lines 1–8 |
| Characters on the stage | a **being** per look node at its holder's spot (#492); 2D picture fallback open (#495); rigged character GLBs exist on Proton Drive, not in the catalog (#497) | `space/src/space/being.rs`, `cast.rs`, research/SCENES.md §9.6 |
| Picking on the stage | whole cast members only (bounds sphere / card quad), no hit point on a mesh | `space/src/space/pick.rs` (#496) |
| Moving on the stage | members slide on the floor, snap to feet, spots, grid axes, steps | `space/src/space/place.rs`, `space/src/snap.rs`, editor `grid.rs` (#581) |
| Gizmo | only the **view** gizmo (XYZ orbit, look along an axis; decision 2026-11-09 [platform]); **no transform gizmo** in the deck | `deck/editor/src/main.rs` `Drag::Gizmo`; the transform gizmo is open in #244 |
| Module bodies (a GLB or sketch per module, tubes end on port anchors) | researched / ticketed, not built | #335 (element import), #336 (exact port anchors: click to place, drag the direction, or named empties), #445 (a GLB on a node, outlets defined or guessed; test files Croiser Claw Node, Twin Key Gyro Sequencer), #419 (Blender / Tripo round trip, `extras`, core 3D elements), #347 (sketch → 3D services) |
| Module slots (where a module attaches on an entity) | built (#590): the slot table, the motion domain | `deck/catalog/src/slot.rs`, research/MODULE-EXTENSIONS.md |
| Addresses | one parser; places `deck: scene: character: item:`, `spot:` named as later | `deck/space/src/rig/address.rs` lines 1–20 |
| Asset side data the core writes and the server stores | built for media edits (crop, trim, gain) | `deck/space/src/asset_edits.rs` (migration 2026-11-20-asset-edits) |
| XR anchors (geo, surfaces) | proposals | #714 (geo-anchor), #682 (world tracking epic), research/XR.md, XR-SHARED-CORE.md |

Owner decisions that bind part A: the real interface of a module is its 3D object, its outlets mapped to elements (2026-11-05
[modules]); a Blender round trip is needed (2026-11-05 [modules], #419); nodes in 3D with a GLB attached, outlets "defined or
guessed" there, one UI with play mode (2026-11-09 [platform], #445); wiring like a 3D editor, cables cut (2026-11-09
[platform]); modules dropped into the rig's 3D space, one interaction for rigs and scenes (2026-11-16 [workspaces]).

### 1.2 SCUTOONS: node views, sockets, the Library's drag

| What | State | Where |
|---|---|---|
| The one signal table: Event, Control, Clock, Audio, Picture, Address, Vote, Scene, Far – shape + colour + weight; `feeds` (same signal, or Vote → Control) | built (#591, #618) | `deck/kit/src/base/signal.rs` (192 lines) |
| Graph model + auto layout (layers LR / TB on the 1rem grid), folding to a pill, wire tag placement | built | `deck/kit/src/base/graph.rs` (562): `Node{ins, outs, collapsed, picture, group}`, `Pin{label, signal, value}`, `Link`; **no stored node positions** |
| Show node view: a row as nodes (from far, the row, cells, sends, outlets); zoom/pan, F frames, box select, Tab focus, groups (Ctrl+G, labelled, coloured), notes, folds, pinned wire values, focus tone, previews switch | built (#591, #648, #675) | `deck/editor/src/show/nodes.rs` (732), `node_paint.rs` (395), `node_pick.rs` (272), `node_card.rs` (202); deck-show `nodes.rs`, `canvas.rs` (`Edit::{Fold, Group, GroupLabel, GroupColour, Note, NoteText}`) |
| Wiring cells by typed ports: drag outlet → inlet, lift a wired inlet, only feeding inlets light, busy rows bundled | built **in the rows band only**, not in the node view | `deck/editor/src/show/wiring.rs` (309) → `deck_show::rows::Edit::Wire / Unwire`; deck-show `wiring.rs` `CellWire` |
| Rig wiring | 3D: the being's hoses, tubes cut and dragged (#431 write-back proof open) | `being.rs`, research/MODULE-WIRING.md |
| Library: 12 kinds (module, instrument, rig, scene, character, item, place, image, audio, video, model, file); `Purpose::{Build, Show, Module, Scene}` says which kinds a view takes (`takes`, in the core) | built | `deck/catalog/src/library.rs` 19–35, 156–210; `library/shelf.rs` (#698/#701) |
| Library drag: a card carried (`kit/editor/carry.rs`), dropped: Show → `drop_pieces` on a **cell of the rows grid** (scene / character / item / place); Build stage → module / instrument as a track, scene slots and layers | built | `deck/editor/src/library.rs` 608–640, `show/scenes.rs` 32–55 |
| Drop into the **node view** | **not built** (the drop target is the rows grid's cell only) | – |
| Typeahead (reusable list over a field, a caller's own find rule) | built (#455) | `deck/kit/src/editor/typeahead.rs` |
| Creator flow views (web, `@xyflow/svelte` + dagre) | built, read-only story flow | `creator/src/lib/components/flow/` |
| Logic nodes, shared with OSCT | concept decided L1–L13 (#702): documents are the logic, the graph is a computed view, an edit is an op on one document, only the canvas is stored; free nodes (reference, maker) write only through an explicit apply | #702, OSCT #334 |

### 1.3 OSCT (reference only)

| Idea | OSCT state | Where | SCUTOONS |
|---|---|---|---|
| Typed sockets (event / value / node / scene / address / far / media) + spec types (number min/max, bool, colour, choice, texture, model, file accepts) | **built** | `editor/src/lib/logic/types.ts`, `spec-ports.ts` | signals built; spec param types per pin: not |
| Colour per type | **not** (shape + dash only, OSCT rule) | – | built (colour + shape) |
| Ports from the spec (params → value inlets, actions → inlets, events → outlets) | **built** | `spec-ports.ts`, `shared/types/components.ts` | ports from cells (`wiring.rs`), from module descriptors in the being |
| Multi in / out (`many`), `do` outlet as "+" to add an action | **built** | `logic/types.ts`, `apply.ts` | not |
| Unwired ports hidden behind "+", hide unused (Ctrl+H), `exposed` in the canvas | **built** | `LogicBox.svelte`, schema 5.1 | not |
| Only accepting inlets light during a drag; a refusal says why | **built** | `LogicCanvas.svelte` | built in the rows band |
| Add menu with typeahead (right click, "+", a node type dropped) | **built** | `palette.ts`, `LogicMenu.svelte` | typeahead component built, no add menu in the node view |
| Wire dropped on empty canvas → add menu | researched (§5.5), **not built** | concept §5.5 | not |
| Cut wires (Ctrl-drag), insert a node onto a wire, reroute | **neither researched nor built** | – | cut: decided for 3D tubes (2026-11-09) |
| Frames / groups, notes, folds | data model built, UI open (L-T7) | `shared/types/logic.ts` | **built** (canvas.rs) |
| Free node positions | off (`nodesDraggable=false`, own auto layout) | `logic/layout.ts` | same: auto layout only |
| Asset dropped onto the canvas → a reference box; box previews (thumb, first frame, silhouette) | researched (§5.9 points 5–6, L-T12, L-T15); a cheap `preview` built | concept §5.9 | `Node.picture` built |
| Files dropped onto tree rows (a node takes the file) | **built** (#330) | `library-drag.svelte.ts` | Library drop onto cells / tracks built |
| Type purity: conversions as visible nodes | researched | `2026-11-20-logic-basic-nodes.md` §5 | `feeds` has one implicit rule |
| Picking a point on a mesh | **not built** (raycasts select whole objects) | `stage/stage-core.svelte.ts` | not |
| Attach to bones | **not built, not researched** (#253 rig trial closed, #256 Blender pipeline open) | – | not |
| Transform gizmo (W/E/R, parent space) | **built** (three.js TransformControls) | `stage/gizmo.ts`, `surface-gizmo.ts` (#314) | not (view gizmo only) |
| Snapping (grid, axes, boxes, pivots, outlines, degree / step snaps) | **built** (#190) | `stage/snap.ts`, `snap-scene.ts` | built (#581, own core snap.rs) |
| Named nodes addressed `target:<id>/<node>` | **built** (#163) | `shared/types/address.ts` | address.rs; GLB node names not addressable yet |
| Per-model anchor list | **none** | – | none (set spots only) |
| XR anchors (world tracker, surface / floor anchoring) | researched (`2026-11-20-shared-xr-core.md` §4.3), epic #299 open | – | #682, #714 |

**So:** OSCT is ahead on the editor gestures of a typed node canvas (hidden ports, "+", spec types, add menu) and on the
transform gizmo; it has nothing for points on a model or bones. SCUTOONS is ahead on colour typing, frames / notes / folds and
the core-owned graph and snap. Neither has reroutes, wire cutting in 2D, insert-on-wire or asset → node drops.

---

## 2 · The gaps

### Part A – placing spots on a model
1. **No model anatomy in the core.** `glb.rs` flattens everything and drops skins and animations: there are no nodes, parts or
   bones to attach to, and nothing moves with an animation.
2. **No skinned drawing.** Characters stand as beings or (soon, #495) pictures; a rigged GLB is never drawn, so a spot on a hand
   has no hand to follow.
3. **No surface pick.** pick.rs returns a member, not a point, a normal, a part or a bone.
4. **No transform gizmo** to move / turn a spot (or anything else) in the deck editor (#244 open).
5. **No spot data model:** no type, no place to store hand-placed spots per model, no validation (names, bone exists, ports match
   a module's descriptor), no `spot:` address.
6. **No spot editor:** list, add by click, name, kind, attach to a bone, mirror, delete.
7. **No consumer:** lights, emitters, sound sources and targets of a rig / scene / show cannot name a spot; module tubes cannot
   end on one (#336).
8. **No guess:** #445 wants outlets "defined or guessed"; nothing proposes spots from a model's shape or skeleton.
9. **No round trip:** the Blender naming convention / template of #419 is not written.

### Part B – the node view
1. **The node view takes no drops.** Library cards land only on the rows grid's cells (Show) or Build's stage.
2. **A drop has no rule per view and kind** beyond `Purpose::takes`: what a sound, an image, a model or a rig becomes in a
   Show row is undefined; raw media has no document home in a row.
3. **No free nodes:** a reference (picture, sound, model) that is not content has no place on the canvas (L11).
4. **No wiring in the node view:** wires are drawn, but made only in the rows band.
5. **Missing Blender gestures:** wire dropped on empty space → add menu filtered by signal; insert a node onto a wire; Ctrl +
   right drag cuts; reroute points; hidden unused sockets with "+"; multi-in pins; dragging a node by hand (auto layout only).
6. **No spec types on pins** (range, choice, file kind) for a preview or a refusal text finer than the signal.
7. **No 2D rig node view:** the rig is wired in 3D only; whether a 2D view of the same graph is wanted is open (Q5).
8. **Spots don't show as pins** on a model / character node (needs A).

---

## 3 · The design (core first, RULES A1, A6; the editor draws)

### 3.1 Spots (part A)

**Data (deck-catalog, one type):**

```text
Spot  { name, kind, attach, at: [x y z], turn: quat, port: Option<name> }
kind   := light | emitter | sound | target | camera | port | mark      (one word each, listed once, A14)
attach := root | node:<gltf node name> | bone:<joint name>
```

- **Where they come from, in order:** (1) the GLB's own named empties (`spot_<name>`, and `port_<name>` for module bodies, the
  #419 convention, kind from a suffix or glTF `extras`), (2) hand edits stored like an asset's `edits` (asset_edits.rs pattern:
  the core writes the list, the server stores what the core wrote, `GET /api/assets/{id}` carries it). A hand edit may add,
  move, rename or hide a file spot; a re-export keeps hand edits that still resolve and reports those that don't.
- **Validation in the core** (one function, run by `/validate` on save, A6): names are whole lowercase words, unique per model;
  a `bone:` / `node:` exists in this model version; a module body's `port` spots match its descriptor's ports (missing / extra
  / renamed listed as problems, not silently fixed).
- **Address:** `spot:` becomes a place in address.rs: `character:catpope/spot:hand_l`, `item:lantern/spot:flame`, relative
  `spot:hand_l` inside the document that holds the model. Its reason line goes into the one parser.
- **Pose:** `spot_pose(model, spot, pose_at_t) -> Mat4` in the core: root × node × (bone at t) × local. Native and wasm the same
  (golden test).

**Model anatomy (deck-space):** `glb.rs` keeps, beside today's flattened draw list, a light **anatomy**: the node tree with
names and parts, skins (joints, inverse bind matrices), the animations' channels. The flattened path stays for sets; a model
with a skin gets a skinned draw (GPU skinning in `gpu.rs`, a bone palette per instance) – the biggest single step (A3).

**Tools (core logic, editor draws):**
- **Surface pick** – the pointer's ray against the model's triangles (a BVH per model version, built once), returns point,
  normal, part / node, and the dominant bone by skin weights at the hit. Pure, headless-testable.
- **Snap targets** – vertex, face centre, part centre, bone head / tail, existing spots, the mirror across the model's
  symmetry plane (`_l` ↔ `_r`), on top of snap.rs's grid / steps. Same `Settings` as Build's grid popover.
- **Transform gizmo** – one deck-kit `base::gizmo`: handles (move axes / planes, turn rings), the drag → delta in local / parent
  / world space, steps from snap.rs; the editor paints it. Spots first, then scene members and set lights (#244's gizmo).
- **Guess** – a core proposal, never applied by itself: bone ends (hands, head, feet), open round ends of parts (#445's nozzles),
  part centres; each shown as a ghost spot the person accepts or drops.

**Editor (deck editor, Asset tab of a model and Build):** a Spots panel (deck-kit editor components: list, typeahead for names
with kind-based suggestions, the kind as a chip, attach root / node / bone, mirror, delete); click on the model adds at the pick,
the gizmo moves / turns, Alt+click picks the bone only. Every change is a core edit with a refusal text; save as a draft;
history and revert through #687's History.

**Consumers:** a rig's light / emitter / sound / target node names a spot by address; the stage places it at `spot_pose` each
frame (sound sources feed the engine's pan, `engine/src/pan.rs`); module tubes end on `port` spots (#336); the node view shows
a model's spots as pins (3.2).

### 3.2 The node view (part B)

**Rule 1 – a drop is a core plan, not editor code.** One function in the core:

```text
drop::plan(view: View, kind: Kind, address, at) -> Result<Vec<Op>, Refusal>
View := ShowRow(row) | RigGraph(rig) | SceneGraph(scene)
```

It extends today's `Purpose::takes(kind)` and gives the document operations (L1: the graph is computed, an edit is an op on one
document) or the reason it refuses. The editor's carry calls it while hovering (lights the target) and on letting go.

| Kind dropped | Show row node view | Rig (3D stage now; a 2D view if Q5 says so) |
|---|---|---|
| scene, character, item, place | a cell's piece (`rows::Edit::Piece`) on the outlet column under the pointer, or a new cell in a free column | character / item: a world node (its `being:` / spot inputs, 2026-11-08 [rigs]) |
| module, instrument | refused (a row holds pieces) | a track (built, Build stage) |
| rig | refused | a nested rig as a track (2026-11-09 [rigs]: nesting) |
| show | a far node of that show (its events), read-only | refused |
| model | reference node (Q4) | a body on the node under the pointer (#445), its spots its port anchors |
| image | reference node (picture preview) | a 2D body / sketch on a node (#335) |
| audio | reference node (playable) | a sampler track with the file as its parameter |
| video | reference node (first frame) | a picture source node |
| file | reference node | refused |

**Rule 2 – free nodes live in the canvas only.** A reference node (L11 "editor only") is stored in the row's `canvas.rs` beside
groups and notes (`Canvas.refs: [{id, address, near}]`), never reaches play, and has a "Use" action that runs `drop::plan` for a
real op when one fits (L13: a free node writes only through an explicit apply).

**Rule 3 – sockets are the signal table plus a few flags.** `Pin` gets `many: bool` (multi-in: several wires into one inlet,
drawn as a stretched socket), `hidden: bool` (unused and folded behind "+"), and an optional `spec` (range / choice / file kind)
taken from the piece's knobs or the module descriptor – for the refusal text and the preview, not a new signal. Colours and
shapes stay the one table (2026-11-17 [design]); conversions, if ever needed, are visible nodes (OSCT type-purity research),
`feeds` keeps its single implicit rule.

**Rule 4 – gestures map to ops.** All in the node view, the same ops as the rows band (`rows::Edit::Wire / Unwire`):

| Gesture | Result |
|---|---|
| drag outlet → inlet | `Wire` (only feeding inlets light; a refusal says why) |
| drag a wired inlet away, let go on nothing | `Unwire` |
| let go a wire on empty space | the add menu (typeahead) listing only nodes / cells with a feeding inlet; choosing wires it |
| drag a node onto a wire | inserted when it has a matching in and out (two `Wire` + one `Unwire`, one undo step) |
| Ctrl + right drag across wires | cut (the 3D tubes' cut, same word) |
| Shift + click on a wire | a reroute point: canvas data (`Canvas.bends`), typed by its wire, never a document node |
| "+" on a node / Ctrl+H | show / hide unused sockets (`Canvas.exposed`) |
| drag a node by its head | pins its position in the canvas over the auto layout (Q3); "Tidy" drops the pins |

The canvas stays authoring data (L4): groups, notes, folds, refs, bends, exposed, pins. Nothing of it reaches the deck's play.

**Rule 5 – spots as pins.** A node whose piece has a model shows one pin per spot, grouped under the model, its signal by kind:
light / emitter → Control, sound → Audio, target / camera / mark → Address, port → the port's own signal. Wiring a pin writes the
spot's address into the cell or rig, so the node view, the rows band and the 3D tubes stay one grammar.

**Shared with OSCT (#702):** the Signal names and shapes, the `Op` idea, the canvas fields (groups, notes, folds, exposed) and
the gesture list above are the parts worth keeping in sync by words, not code; OSCT's open L-T7 / L-T12 / L-T15 are the same
ideas as B4 / B5 / B6 below.

---

## 4 · Stepwise plan (ticket titles to file; not filed)

Order: B1–B4 can start at once (the node view exists); A needs A1–A3 first. Sizes: S ≤ a day, M a few days, L a week or more.

### Part B – the node view (first: it builds on what exists)

| # | Ticket title | Done when | Size | Needs |
|---|---|---|---|---|
| B1 | Drop plan in the core: what each asset kind becomes in a Show row and a rig, or why not | `drop::plan` in deck-catalog/deck-show with the table of 3.2, unit tests per (view, kind), refusal texts; `Purpose::takes` answers through it | S | – |
| B2 | Node view takes Library drops: pieces onto cells, the target lit while carrying | a scene / character / item / place card dropped on the Show node view lands as a cell's piece (headless check `--library-drag` on the node view), a refused card shows the core's reason at the pointer | M | B1 |
| B3 | Reference nodes: any asset as an editor-only node in the row's canvas, with "Use" | an image, sound, video, model or file dropped where no op fits stands as a reference node with its preview, is saved in `canvas.rs`, is absent from the played show, "Use" applies a real op when one fits | M | B1, B2 |
| B4 | Wiring in the node view: drag outlet → inlet, lift, cut, insert on a wire | the node view makes and removes `CellWire`s with the rows band's ops; Ctrl + right drag cuts; a node dropped on a wire inserts in one undo step; deck-show tests per gesture | M | – |
| B5 | Wire to empty space opens the add menu filtered by signal | letting go a wire on the canvas opens the typeahead with only feeding targets; choosing one wires it | S | B4 |
| B6 | Sockets: hidden unused behind "+", multi-in pins, spec on pins | `Pin.many / hidden / spec`; Ctrl+H hides unused for all nodes; `Canvas.exposed` saved; a refusal names the spec ("expects 0…1") | S | – |
| B7 | Reroute points on wires (canvas bends) | Shift + click adds a bend, drag moves it, Delete removes it; stored in the canvas, typed by its wire, survive a relayout | S | – |
| B8 | Pinned node positions over the auto layout, and "Tidy" | a dragged node keeps its place across reloads (canvas), "Tidy" clears the pins; LR / TB still lays out the rest | M | Q3 |
| B9 | A model's spots as pins on its node | a cell whose piece has a model shows its spots as typed pins; wiring one writes `spot:<name>` into the cell | S | A1, B4 |

### Part A – spots on models

| # | Ticket title | Done when | Size | Needs |
|---|---|---|---|---|
| A1 | Spots in the core: type, file spots from GLB empties, hand edits on the asset, validation, the `spot:` address | `Spot` in deck-catalog; `spot_*` / `port_*` empties read from any model GLB; hand edits round-trip through `GET/PUT /api/assets/{id}` like `edits`; `/validate` reports bad names / missing bones / port mismatches; address.rs parses `character:x/spot:y` with tests | M | Q1, Q2 |
| A2 | Model anatomy in the GLB loader: node tree, parts, skins, animation channels | a rigged character GLB (#497, e.g. catpope) loads with its joint names; a bone's world matrix at time t matches a reference export (native ≡ wasm test); sets load unchanged | M | – |
| A3 | Skinned models on the stage (GPU skinning, one animation playing) | a rigged character stands at its holder's spot in its idle animation, native and browser; budget measured on X1 / S22 (#306 bench) | L | A2, #497 |
| A4 | Surface pick and spot snap targets in the core | headless: a ray on Croiser Claw Node returns point, normal and part; on a rigged character the hand bone; snaps to vertex / part centre / bone end / mirror | M | A2 |
| A5 | Transform gizmo in deck-kit (move, turn; local / parent / world; grid steps) | one `base::gizmo` with tests of the drag math; the editor draws it; used by spots, then scene members (#244's gizmo) | M | – |
| A6 | Spot editor in the deck editor: list, add by click, name, kind, attach to a bone, mirror, delete | the owner places three spots on a model in its Asset tab, saves a draft, reloads and finds them; every change goes through the core with a refusal text; History reverts | M | A1, A4, A5 |
| A7 | Spots follow animation: lights, emitters, sound sources and targets placed at a spot | a rig's light named `character:x/spot:hand_l` moves with the hand in play; a sound source pans with it; deck:ci golden | M | A1, A3 |
| A8 | Module bodies: port spots, tubes end there (merges #336 / #445's anchor part) | Croiser Claw Node's nozzles carry its ports, the being's tubes start and end on them along the spot's direction; missing ports fall back to points on the bounds | M | A1, A4, #335 |
| A9 | Guess spots: bone ends, open part ends, part centres as ghosts to accept | on a new model the editor shows proposed spots; accepting one writes a hand edit; nothing is applied without a click | S | A4, A6 |
| A10 | Blender round trip: the naming convention and a template per module (finishes #419's design note) | a written convention (`spot_` / `port_` + `extras`), a template GLB generated from a module descriptor, a checker the import runs | S | A1 |

**Not in this plan:** XR anchors to the real world (#714, #682) – an XR scene's spot is a different thing (a geolocation / a
surface); A1's `Spot` could later carry an XR anchor kind, but nothing here depends on it.

---

## 5 · Open questions for the owner (each with a recommendation)

1. **The word.** "Outlet" already names the Live outlet and a node's out pin. *Recommend:* points on a model are **spots** (the
   set's word already, `spot:` in the address); node pins stay inlets / outlets; the Live outlet stays.
2. **Where spots are stored.** On the model asset (every character / item using it shares them) or on each document that uses the
   model? *Recommend:* on the asset version (file empties + hand edits, like an asset's `edits`); a per-use override only if a
   real case appears.
3. **Node positions.** Keep the auto layout or move nodes freely like Blender? *Recommend:* auto layout stays the default; a node
   dragged by hand is pinned in the canvas, "Tidy" lays everything out again (B8).
4. **Raw media dropped into a Show row.** A row holds pieces; an image, sound or model is not one. *Recommend:* it becomes an
   editor-only reference node now (B3); a "one-asset scene" wrapper only if you want media to play straight from a row.
5. **A 2D node view of the rig** beside the 3D being? Decided so far: modules are dropped into the rig's 3D space (2026-11-16).
   *Recommend:* no 2D rig view for now; the drop plan (B1) serves the 3D stage, the 2D view can come later from the same graph.
6. **Priority.** This competes with the web tools overhaul (#626, before February) and the posters (#674, December). *Recommend:*
   file the tickets now, build B1–B4 after the overhaul's current tickets, and A2–A3 (skinned characters) together with #497 /
   #495, since a spot on a hand needs a drawn hand first.

No sketches were made: the node card look is settled (#648); the gestures and the spot panel follow the existing kit
components. A night-mode sketch of the spot editor can follow with A6's draft round (RULES G1).
