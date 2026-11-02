Status 2026-11-02: concept, partly decided – the creator's one editor, growing from the rig designer. Decided: the name "workspace" for a dock preset and "Sound design" for the first one; the scene editor joins the same editor with an inspector; the OSCT editor's logic is reused (#242); **first the sound engine, its wiring, compilation and storage are finished, scene elements come after.** The routing in §4 is a proposal waiting for the owner. Nothing here is built except the sound pipeline (#280, #281).

# EDITOR – one editor for everything that has a stage or a sound

Owner 2026-11-02: "the sound page? this is the beginning of our editor!" · "perhaps a sound design stage is perfect for how to call this preset. in blender different workflows basically get different presets for the docks." · "our scene editor will also find its way into this very same editor. so we will need an inspector to pick the right item on stage." · "here recycle the editor from osct logic." · "but first let us finish the current state with the sound engine and its wiring and compilation and storage. we can adopt the model to scene elements later." · "multiple things will access the editor: items, rigs, instruments, furniture, sets, spots, characters, eventually being different cut down scenes of the whole."

## 1. The model – subject · workspace · selection
- **Subject** – the document that is open, addressed like the entities API does: a key `kind:id`. Items (`item:`), furniture (an item with a `furniture` block), rigs (`rig:<subject>`), instruments, presets, modules, characters, sets, spots, scenes (`scene:<id>`, #288), later *cut-down scenes*: a composition document that references parts of the whole.
- **Workspace** – a Blender-style preset of docks for one kind of work: which panels, which layout, which tools. Fixed in code at first, not user-built. **Sound design** is the first (designer + the deck, a split screen: Rust · Parameters · Nodes · Assignment beside the engine host, RIG-EDITING.md §2 / §8). Later e.g. **Scene** (stage + outliner + inspector), **Layout** (sets, spots), **Look**.
- **Selection** – what is picked inside the subject: a track, a node, a parameter; on a stage, an object. The **inspector** shows the selection – one inspector for all kinds, the kind supplies its spec (OSCT's spec-driven Inspector / `EntityPanel` is the model).

## 2. The panels the workspaces share (what the OSCT editor already has, #242 survey)
The stage (the deck's engine behind a typed protocol, sandboxed), an outliner / tree, the inspector, problems + console, a code editor, a node view, and a selection store with "reveal". A workspace is a list of these. The compile service, the module storage and the engine host (#298) are the sound workspace's first real panels.

## 3. Scope now and later
- **Now (finish the sound pipeline):** #280 SDK ✔ · #281 compile service + storage ✔ · #298 engine host in the creator · #282 the Sound design workspace (Rust, Parameters, Nodes read-only, Play) · #283 assignment, missing defaults, standard rigs. Built as the **first workspace**, but the shell (generic tree, scene stage, generic inspector) is **not** built yet.
- **Later:** adopt the model to scene elements (#241 / #244 / #245): the Scene workspace, picking an item on the stage into the inspector; the OSCT reuse survey (#242) is done first thing then.

## 4. Routing and association – a proposal (not decided)
- **Canonical address of anything editable:** `/edit/<kind>:<id>` with `?w=<workspace>` and `?sel=<path>` – e.g. `/edit/rig:character.catpope?w=sound&sel=voice`. One thing, one URL, whatever page you came from. (Today the creator already addresses documents by key: `/catalog/<kind>?id=`.)
- **Context entries:** the pages that know where a thing lives link into it and hand over the breadcrumb (`from=`): the character page → its rig, an item page → `rig:item.<id>`, the catalog lists, and – later – the chapter: **`/<toon>/chapter/<n>/scenes`**, a page of the chapter like the existing `/<toon>/chapter/<n>/storyboard`, which opens the editor on the chapter's scenes in the Scene workspace. (Not `/live`: that word is the stage builder's lens, §6.) The context decides *what is offered and which breadcrumb is shown*, never the identity.
- **Why not only under the content tree:** instruments, items, furniture, sets and spots are platform-wide and reused by many chapters; under `/<toon>/chapter/2/…` one instrument would have twenty URLs.
- **Association = "uses / used by"** (it exists in part: `RigValidator`'s uses – an instrument by presets, rigs and items –, the asset registry): the inspector lists them, a click opens `/edit/<key>`. Editing a shared thing from inside a scene says "used by N" first.
- **Cut-down scenes:** a document that references parts; the Scene workspace opens it, picking an object on the stage selects the referenced part in the inspector, "Edit definition" opens that part in its own workspace.

## 5. Open
1. Routing as in §4 (canonical `/edit/…` + context entries incl. `/<toon>/chapter/<n>/live`) – or only context routes? Recommended: §4.
2. When the generic shell (outliner, generic inspector) starts: with the Scene workspace (#244 / #245), not before. Recommended.

## 6. Not to be confused with – four different things (owner 2026-11-02: "this is separate from the row view, the conductor view … not to be confused with the conductor in the 3d visualisation")
| | What it is | When / who | Where |
|---|---|---|---|
| **The editor** (this doc) | makes **things**: instruments, rigs, later scenes, sets, spots – in workspaces | before: a creator authors parts | creator app, `/edit/<kind>:<id>` |
| **The row view = the stage builder, the "Live" lens** (LIVE.md B.5.5, LIVE-LENSES.md §1, §6; not built) | arranges **a show over the story**: the toon's one flow as rows (scene keys, story order, branches); per row what every outlet and device does (cells, layers, cues, looks) – it **picks** scenes, sets, media and rigs, never copies them | before: the show's author, next to the Storyboard and Slice-editor lenses of the same rows | creator, a lens tab in the header |
| **The conductor view** (LIVE.md B.2; `/live/conductor`) | **runs** the show live: the position, the cue list go / next / hold, votes, panic, which decks are connected; sends the cells to the outputs | during the show: the conductor person | viewer |
| **The conductor's being** (deck-space `CONDUCTOR_SCENE`, `stage/scenes/conductor.toml`, #188) | a **picture**: the bigger being in the middle of the nave (PLF's portrait as its head) that stands for the conductor in the 3D, the decks' beings around it | during the show: what the audience sees | inside a scene on a deck's screen |

Make → arrange → run: the editor makes the parts, the row view arranges them along the story into a show, the conductor view runs that show, and the conductor's being is only how the conductor appears in the picture. In the docs "conductor" always means the person / role (B.2); the 3D one is called **the conductor's being**.
