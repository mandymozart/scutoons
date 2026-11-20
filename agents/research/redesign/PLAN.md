Status: plan 2026-11-20 for epic #626 – execution started (owner: "we can start planning this and executing"): phase 0 runs ticket by ticket (§8 "Now"); drafts round 2 (workspaces2.html, flow.html) are the current picture; §9 marks what the owner answered.

# The creator overhaul – holistic analysis and design plan

Owner 2026-11-20: *"the goal is now to do a holistic analysis and design plan so we can split properly. Map, catalog,
library (asset manager), characters, toons, season's chapters, storyboard, experiences, flows, generations … are my most
obvious but many smaller things are there. Also we have a panel with tabs and free moving tabs concept. I recommend also
to checkout how panels now are in osct."* Priority: before the February work; posters printed in December, rolled out in
January (#674). Inputs: INVENTORY.md (routes), WORKSPACES.md (backgrounds), workspace.html (approved look),
OSCT's editor (floats, #308, #318, #329, #330), the deck's [workspaces] / [design] decisions.

## 1. What is wrong today, in one page

- **Pages, not workspaces.** 20 routes, each with its own frame (`meta-page` for 11 of them), its own tree, its own
  inspector, its own dock. Toon navigation exists three times (ToonTabs, AreaMenu, crumbs).
- **One thing, several editors.** Characters (registry + cast), items (definition + collage + slice placement), rigs
  (creator Sound workspace + deck Asset / Module), scripts (one editor, four routes), Flow (page + dock), the storyboard's
  camera vs the deck's DOP.
- **Four 2D canvases, three 3D engines, three node graphs, three timelines** – each with its own pointer, zoom, snap.
- **Assets are scattered:** media live in slots inside documents; there is no place that shows *all* assets, where they
  are used, what is missing (only `wanted-images`), what was generated.
- **Phone layouts everywhere** (33 files with `@media`, sheets, PhoneBar) – overhead for a desktop tool.
- **The brutalist look** reads badly over time.

## 2. The model

### 2.1 Workspace · tab · panel

- **Workspace** = a kind of work, defined by its **background** (the stage, scene or canvas the panels float over) and a
  default set of panels. The top bar switches workspaces; each remembers its layout.
- **Tab (document)** = an opened item inside a workspace (`character:catpope`, `chapter:catpope/s1/c3`, `map:catpope`,
  `experience:poster-feilchen`). One tab per item; the last tabs reopen; Home when none. Only the front tab runs.
  The item's address is the address book's `kind:id` (#477) – also the URL.
  **Opening an item opens its own workspace** – the one its kind belongs to, as in the deck – with that workspace's helpful
  panels opened alongside; panels can still go anywhere (owner 2026-11-20). From Home, the toon board or the Library alike.
- **Panel** = a floating window over the background, declared once (`PanelSpec`), shown by any workspace that wants it.
- **Panel tab** = a panel's content unit. **Tabs are universal (OSCT #318):** any tab can live in any panel; drag a tab
  into another panel (a marker shows before / after), drag it out onto the stage → a panel of its own; a panel whose last
  tab leaves closes; essential content (the Inspector's selection) never gets lost.

### 2.2 Floating rules (take OSCT's `floats.ts` – the deck's floats, already ported to Svelte and refined)

- Each panel: id, name, icon, *about* text (Info shows it), default place by a corner + offset, size (rem, `fill`,
  `content`), min size, folded / closed by default, **essential** (folds, never closes), **inline** (a toolbar: one row
  or column, never folds).
- Place kept by its nearest corner → it comes back after a window resize; remembered per workspace in this browser;
  the panels menu resets.
- No overlap, 1rem gaps; a move snaps to walls and edges; **a resize makes room** (neighbours move on, then shrink to their
  least size – OSCT #329).
- Order of the specs = order of growth (the scrolling ones first).
- Fold = the pill (grip, icon, name). See-through ground without blur (night 50 %), ink halo.
- Night first, day as a variant; the deck's tokens (deck-kit) for the look.

### 2.3 Edit / Play

Only where something runs: Scene (a scene, a map's world, a chapter read as the reader), Asset rig / module, Experience
(the simulated tracker: found / lost / tap). Not on forms-only items (a recipe, a season). Play's changes are dropped
on Stop (deck rule).

### 2.4 The top bar

☰ (main menu §2.5) · ⌂ Home · the workspace switch · the tabs of this
workspace (kind icon, name, ✕, ＋ opens through the Library) · save state · Edit / Play where it runs · the text language
(the one switch, RULES H1).

### 2.5 Main menu and Settings – analog to the deck editor (owner 2026-11-20: *"create a settings menu and main menu analog to deck editor with additionally the stuff needed for the creator"*)

The deck's model (`deck/editor/src/bar.rs` MENU, `settings.rs`): ☰ at the row's left, then Home, then the tabs; the menu's
lines top down, then the open workspace's panels that can close (ticked); nothing to save (everything saves itself); a line
that leaves for the browser is marked ↗. Settings open as a floating panel, this machine's / this member's.

**Main menu** – deck lines kept, creator lines added (＋), proposal for the owner to check:

| Line | Deck | Creator |
|---|---|---|
| Open… | ✓ | opens the Library unfiltered (any kind, `kind:id` search) |
| New… | ✓ | ＋ new toon · season · chapter · character · item · place · scene · experience (poster first) – asks name and id first (#652) |
| Publish | "Publish in the creator" ↗ | ＋ **Publish…** – the open item's publish / problems / history (today meta-page's Save · Publish · History · Problems) |
| Settings… | ✓ | the panel below |
| Audio & inputs | ✓ | only where the sound workspace / deck stage runs; else hidden |
| ＋ Text language | – | the content language switch (RULES H1), also in the bar |
| ＋ Help | – | Guide (style guide + script rules, today `/guide`), Docs (`/docs`) ↗, keyboard shortcuts |
| ＋ Go to | – | Reader ↗ (the toon / chapter as readers see it), Embassy ↗, Admin ↗ (by role), the deck (open in the deck app ↗) |
| Account | ↗ | ＋ the member, role, org; sign out |
| Quit to stage F9 | ✓ | – (no stage to quit to; Home is the ⌂ pill) |
| *panels of this workspace* | ✓ | ✓ the same list, ticked = open |

**Settings** – deck sections kept, creator sections added:
- **Look** (deck): Day / Night (night default), **UI size** (1rem − / +).
- **Motion & feedback** (deck): the deck's switches that apply in the browser (hover reveal, focus flash, panel transitions,
  live previews, reduce motion); signal flow on wires where a node graph runs.
- **Sound** (deck): UI sounds, volume.
- ＋ **Language:** the UI language (inlang) – separate from the text language.
- ＋ **Local folder:** the member's local files for generations / the storyboard (today called "workspace folder" – §9.7, rename
  here, e.g. "Local folder").
- ＋ **AI & generation:** the member's own Claude via MCP (status, how to connect – law.md [mcp-byo-ai]), which providers the
  generation nodes may use (the back's choice shown, the member's keys if any), the default for manual vs AI nodes.
- ＋ **Notifications:** push opt-in (today `/notifications`; goes with the PWA decision #672).
- ＋ **Layout:** reset this workspace's panels / all workspaces' layouts; restore last tabs on start (on / off).
Stored like the deck's: look, motion, sound, layout in the browser per machine; language, AI and notifications per member (server).

## 3. Panels – defined once

| Panel (tabs) | What | Used by |
|---|---|---|
| **Library** (asset manager) | every asset and entity of the scope; one tile per asset family (#653); search, kinds, sort, tiles / list; use counts; ⤢ = the browser over the stage | all |
| **Tree / Outline** | the open item's structure (a chapter's slices, a scene's holders, an experience's scenes › surfaces › targets, a toon's seasons) | all with structure |
| **Inspector** (+ tabs) | whatever is selected, as Sections (OSCT pattern); essential | all |
| **Kanon / Text** | the script / Kanon of the selection's subject (one editor, four routes today) | all |
| **Problems** | errors and warnings of the open items (accept a warning, H7) | all |
| **Wanted** | what is missing (moods, looks, covers, place images, poster art) – the to-do list for Generations | Library, Generations |
| **History** | saved versions, revert, compare | all |
| **Info** | the control under the pointer (*about* texts) | all |
| **Reader** | the item as a reader sees it (phone / desktop screen) | Chapter, Map, Experience |
| **Flow** | the story graph around the open item | Toon, Chapter, Flow |
| **Timeline** | one ms axis, lanes, trim | Storyboard, (Show), Scene camera |
| **Generate** | prompt, generator, cost estimate, jobs | Generations, Storyboard, Wanted |
| **Tools** (inline) | the background's tools: select / move / rotate / scale, grid & snap, view | per background |
| **Scenes / filmstrip** | an ordered strip of the open item's parts (an experience's scenes, a chapter's slices, a storyboard's shots) | Experience, Chapter, Storyboard |

## 4. The workspaces

| Workspace | Background | Tabs (items) | Default panels | Today |
|---|---|---|---|---|
| **Home** | none (a board) | – | open recently · waiting for you · not live yet · toons | dashboard, toon list |
| **Toons** | the toon's structure as a board (seasons › chapters, covers) | `toon:` | Tree, Inspector (toon, season, credits, cast), Flow, Problems | `[toonId]`, season, toon, credits, cast, cover |
| **Chapter** | 2D layer canvas (slice / strip) | `chapter:` | Tree (slices), Inspector, Library, Text, Reader, Filmstrip, Tools | chapter editor (~11k) |
| **Storyboard** | the Scene's 3D stage + shot list + timeline | `storyboard:` | Shots (filmstrip), Inspector, Timeline, Generate, Text, Tools | storyboard (~11.8k) |
| **Map** | map (hex terrain, fog, routes) | `map:` | Tree (places, regions, routes, layers), Inspector, Library, Tools | map builder |
| **Characters** (Asset) | 2D / 3D isolated stage | `character:`, `mood:` | Library, Inspector, Moods strip / grid, Kanon, Wanted | characters, cast, moods, wanted |
| **Catalog** (Asset) | the item's stage (collage) or none (forms) | `item:`, `recipe:`, `album:`, `product:`, `edition:` … | Library (kinds), Inspector, Collage tools | catalog, collage |
| **Sound** (Asset) | the deck stage (wasm) | `rig:`, `module:` | Parameters, Nodes, Code, Assignment, Problems | edit/[key] |
| **Experiences** | surfaces on a 3D tabletop (OSCT stage, copied) | `experience:` | Tree (scenes › surfaces › targets › entries), Inspector, Scenes filmstrip, Library, Tools, Reader (simulated) | – (OSCT) |
| **Flows** | node graph | `flow:<toon>` / universe | Inspector, Problems, Library | flow page + dock |
| **Generations** | variants canvas (bands, lineage) + compare | per subject: shot, character, item, poster … | Generate, Wanted, Compare, Inspector, Library | storyboard's variants |
| **Library** | the asset browser itself, full window | – | details, places, filters | – (OSCT #330) |
| *(Show, Scene)* | deck – native first | – | – | deck editor |

**Why this cut:** each workspace has one background engine and one kind of document; the panels carry the rest. The
deck's Scene · Asset · Show remain the deck's grouping; in the web the Asset kinds are split by what they edit (characters,
catalog, sound), which is how the owner names them. Open (§9.1): whether to group them as one "Asset" workspace instead.

### Per area – what changes (UX rethink)

- **Map:** the builder becomes a workspace with the map as background; places, regions, routes, layers in the Tree; place
  props in the Inspector; the place image from Library / Generations. Later: Songlines (AR) and 3D rooms (#74) open from a
  place.
- **Catalog:** the catalog stops being a list of forms per kind: kinds become Library facets, an item opens as a tab with
  its collage stage; recipes, albums, products, editions are Inspector-only items (no stage). The slice's item placement
  uses the same Inspector sections.
- **Library (asset manager):** new and central. All media and entities in one browser: scope (this item / toon / all /
  unused), kinds, one tile per asset family with its quality versions folded (#653), use counts and "where used" (jump
  there), upload, replace, credits. It replaces every per-editor media picker (`picture-pick`, character slots, collage
  pickers). Same component as the deck's #580 (one concept, two kits).
- **Characters:** the approved draft 2 – character › persona › look in the Library's tree, the stage, Moods strip / grid,
  Inspector Character › Persona › Look › Mood; cast becomes a season facet (no second editor); Wanted feeds Generations.
- **Toons:** the toon page, season pages, toon settings, credits, cover become one Toons workspace: the board of seasons
  and chapters (publish state at a glance), the Inspector for the selected toon / season / chapter (meta, cast, credits,
  supporters), the cover as an Asset of its own (lettering canvas).
- **Season's chapters:** the chapter editor (the biggest piece) on the 2D canvas base: slices in a filmstrip and Tree,
  text boxes / tails / collage / media as layers, VN events as an Inspector section and a Flow view, the Reader panel as the
  preview. Split into several tickets once its own inventory is done.
- **Storyboard:** shots in a filmstrip, the 3D stage as background, the timeline panel, the six steps become panel presets
  (not modes); variants move out to Generations (the storyboard opens Generations for a shot). Camera / shots should
  converge with the deck's DOP (engine question §7).
- **Experiences:** copied from OSCT – scenes in a filmstrip, surfaces on the tabletop, targets on surfaces, entries; the
  simulated tracker as Play; the print path (targets from the print PDF, a print PDF export) for posters first.
- **Flows:** the flow graph as a workspace of its own (universe and per toon), the same graph component later used for
  rig nodes and show nodes (one port / wire grammar).
- **Generations:** the variants canvas and compare become a workspace for any subject (a shot, a character's mood, an item,
  a cover, a place, a poster); Wanted is its inbox; approved results land in the Library with lineage; Generate is a panel
  any workspace can open. The ledger (`storyboard_generations`) generalises to `generations` with a subject address.

## 5. The smaller things – where each goes

| Thing | Home |
|---|---|
| Kanon, scripts (toon / season / chapter / character / place) | Kanon / Text panel |
| Credits, people, supporters | Toons Inspector; people in Settings |
| Cast | Toons Inspector (season) + Library filter |
| Moods vocabulary | Characters: `mood:` items + Library facet |
| Wanted images | Wanted panel (Generations' inbox) |
| Cover (lettering) | Asset `cover:` on the 2D canvas base (Toons opens it) |
| Items placed in slices | Chapter Inspector, the item's sections |
| VN events | Chapter Inspector + Flow |
| Scenes list (deck) | Library (kind scene); opens in the deck / a later Scene workspace |
| Style guide, script rules, docs | Help (☰) + Info |
| Notifications, PWA | Settings – after #672 |
| Publish / review / versions | save state in the top bar + History panel + Home's "waiting for you" |
| Text languages | the top bar's one switch |
| Command palette | stays (☰ and a key) |
| Handheld camera `h/[code]` | stays a phone page |
| Address book | the tabs' addresses and every typeahead (#477) |

## 6. Data and API

- Tabs need **addresses**: every item a `kind:id` (#477, #487) – check each kind has one (chapter, map, experience, mood,
  cover, generation).
- **Library** needs one endpoint over assets + entities with use counts and asset families (#653; OSCT's #330 / #331
  shape). Today `shared/api/src/library.ts` exists – extend.
- **Generations** generalise from storyboard to any subject (`subject` = an address), keep the ledger, cost and MCP tools.
- **Experiences**: the `experience` catalog document in OSCT's schema 3.0 shape (shape A, decisions/xr.md), the print
  targets import.
- Nothing changes in printed URLs (I1); the creator's routes may change (desktop tool).

## 7. Engines – decide once

| Background | Today | Proposal |
|---|---|---|
| 3D stage | deck-space (wgpu / wasm), three.js ×3, OSCT three.js frame | Experiences: OSCT's frame as is (posters can't wait); Storyboard: stays three.js for now; long term one stage (#242's question: the wasm deck stage in OSCT's frame) |
| 2D layer canvas | four own canvases | one base from `canvas-geometry.ts` (pointer, zoom, pan, snap, selection, layers) – Chapter first, then cover, collage, character |
| Node graph | Svelte Flow, own SVG, egui | one Svelte graph component (Flows first), the deck's port / wire grammar |
| Timeline | shared/ui Timeline + deck | keep shared/ui Timeline, extend for camera moves |
| Panels | deck-kit floats (Rust), OSCT floats (Svelte) | **copy OSCT's floats.ts + FloatingPanel into `shared/ui`** (it is the deck's model, already in Svelte, with #329 and #318 coming) |
| three.js version | OSCT 0.174 (pinned by `^0.174`), creator / shared media 0.186 | owner 2026-11-20: **one version** – set when OSCT's stage is copied (Experiences), upgraded later – the 8th Wall engine binary is what holds 0.174; the creator's users are the model preview and the storyboard's viewport / take render, both small to move |

## 8. Order of work and the split

**Now (2026-11-20):** #655 tokens → #656 panels (one worker after the other, ORCHESTRATION) – neither waits on §9. #664 shell follows #656 – §9 1, 2, 6 answered, nothing waits on #673 any more. Posters (#674) start as soon as the owner names what the print needs.

**Phase 0 – ground (needs nothing):**
1. Tokens (#655): the deck's night / day tokens in `shared/tokens`.
2. Panels (#656): OSCT's floats + FloatingPanel + panel tabs (universal tabs #318 as the target) into `shared/ui`; Section,
   Toggle, ToolGroup, ScrubNumber.
3. Shell (#664): top bar, workspaces, tabs by address, Home, the panels menu, per-workspace memory.
4. Library v1 (new): the asset manager panel + browser on the existing `library.ts`, families later.

**Phase 1 – the posters (parallel track, dated):**
5. Experiences v1 (#674 + new): copy OSCT's stage frame, schema 3.0 shape, surface mode, filmstrip; the Poster type;
   targets from the print PDF; the reader's XR mode for one poster.

**Phase 2 – the areas (each its own ticket, any order after phase 0):**
6. Characters (#104) · 7. Toons (#665) · 8. Catalog (#666) · 9. Map (#669) · 10. Flows (new; was part of #664) ·
11. Sound (#667) · 12. Text / Kanon panel (#668) · 13. Generations (new: workspace + generalised ledger + Wanted).

**Phase 3 – the big canvases:**
14. 2D canvas base (new) · 15. Chapter (#670, split after its own inventory) · 16. Storyboard (#671, last).

**Throughout:** phone removal per area (#659), embassy on the kit (#660), docs (#661), PWA decision (#672).

**Size, honestly rough:** phase 0 ≈ 4 L tickets; posters ≈ 2–3 L; phase 2 ≈ 8 M–L; phase 3 ≈ 3 L (the chapter alone
likely 3–4 tickets). With one or two workers at a time (ORCHESTRATION's limit) that is many weeks, not days – the owner
said so too. Posters must not wait for phase 2.

## 9. Questions for the owner

1. ✓ **Asset kinds:** no Asset workspace – the Library is one panel like OSCT's, loading items and reaching every type anywhere; each workspace pre-filters it to its context, widenable to all (owner 2026-11-20, law.md [editor-style]). Opening an item opens its own item-specific workspace, as in the deck (a character → the character workspace); the helpful panels open alongside it by default and can go anywhere (owner 2026-11-20).
2. ✓ **Toons and Chapter:** one workspace – a Home screen like the deck editor's, the toon board opens chapters as tabs (owner 2026-11-20).
3. ✓ **Generations:** yes – "generate is like upload, done within and in context"; a blank asset is filled by upload, Library or generate, the last step is always a Library asset; nodes per asset type, manual (non-AI) nodes, 3D first (owner 2026-11-20, law.md [editor-style]).
4. ✓ **Library:** yes – the way in is Home plus the Library (owner 2026-11-20); the CSV list export goes (decisions/creator.md).
5. ✓ **Experiences:** yes – copy OSCT's surface model and frame; one three.js version, upgraded later (owner 2026-11-20, decisions/xr.md).
6. ✓ **Universal tabs** from the start in `shared/ui` (owner 2026-11-20) – part of #656.
7. **Rename** the storyboard's "workspace folder" (the member's local files) so "workspace" means one thing.
