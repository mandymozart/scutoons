Status: research + design drafts 2026-11-21 for #763 (epic #626), round 4 for the owner's review (§10–§12) – nothing built. Drafts: network.html (?v=1–4 round 1, ?v=5–6 round 2: C, ?v=7–9 and 11–12 round 3: B + the Reception tab, ?v=10 the scope, ?v=13 latency), restyled to the node-canvas dogma; the maps in the kit's own look.

# One network, many scopes: Flow, Logic and the deck's nodes as one view

The owner, 2026-11-21 (#763): *"how we globalize the flow and logic into one, in the end i could just have that tab open anywhere
and when going thrugh the creator it will contextualize as it does with the xr experience, all of these can be connected in the end.
and also the deck editor flow i just a duplicate of this at the moment. with some nodes specific to it since it runs also another
audio context."* And today: *"take the deck editors nodes into consideration since they are just another scope basically of the same
large scale network."* Owner input on the wires (#763 comment): labels with an outline instead of a backdrop; a wire lights up when its
event fires or its variable changes.

This file collects the research that was spread over five places, then proposes the model and the view. §7 holds the questions.

## 0 · In short

1. **There is one network already – it is just never drawn as one.** A reader's choice fires a story event; the relay carries it to a
   show; the show's row fires and a cell plays a scene or a rig on a deck; a chapter's `jumpTo` lands in an XR experience, whose rule
   `goto`s a scene. Every one of those links has an address in the one grammar (`rig/address.rs`, A14). Today four views each draw one
   slice of it, with three graph models and three renderers (§1).
2. **The unit of unification is the *scope*, not the container.** The owner kept the containers apart (2026-11-20 [scenes]: chapter
   groups, show rows, deck scenes and XR scenes keep their names, types and dogma). So we do not merge them: each scope keeps its
   documents, its rules and its runtime. What becomes one is **the view contract, the signal table, the address of every edge, and the
   tab** that shows them.
3. **The pattern is decided twice already and matches:** the documents are the logic, the graph is a projection, an edge knows its
   `home`, an edit is an op on that one home, only the canvas is stored (deck `wiring.rs` / `edit.rs`; OSCT L1–L13, #702). OSCT has
   since **built** it (`editor/src/lib/logic/`, 3,480 lines: `project.ts`, `navigation.ts`, `apply.ts`, `spec-ports.ts`, `signals.ts`,
   xyflow canvas, schema 5.x). SCUTOONS adopts the *contract*, not a fork (§4).
4. **One tab, "Flow", that follows the front item's address**, steps up and down the levels with a breadcrumb, and at the top shows the
   whole network: toons, shows, experiences and decks as boxes, laid out in **runtime lanes** (Reader · XR · Deck) so "another audio
   context" is visible as a lane, and the cables that cross a lane say how they travel (relay, navigation, same engine).
5. **Each scope brings a projector and its node types.** Story (TS, `shared/content`), XR (OSCT's package), show and rig (Rust core
   via wasm – no TS twin, A6). The audio nodes are types the rig projector registers – the deck's "specific nodes" – not a second canvas.

## 1 · What exists (read 2026-11-21, develop 45e9f139, OSCT develop)

| View | Scope it draws | Model | Edits | Renderer |
|---|---|---|---|---|
| Creator **Flow** tab + Flows workspace (`creator/src/lib/components/flow/`, `shell/panels.ts` id `flow`) | universe › toon › season › chapter › group › slice | `shared/content/src/flow.ts` (closed unions `FlowNodeKind`, `FlowEdgeKind`; reach / never / loop / stuck from `reach.ts`); server twin `FlowIndex.php`, `FlowCheck.php`; MCP `read_flow` | no | `@xyflow/svelte` + dagre |
| Rig **Nodes** tab, web (`shared/rig-designer` `nodes.ts`) | rig › track › node › port | Rust `deck/space` `rig/wiring.rs` `wire_view(context)`: levels, stubs, `home` per cable; ops `rig/edit.rs` | no | own SVG |
| Native deck editor (`deck/editor` `wires.rs`, `show/nodes.rs`, `show/wiring.rs`) | rig (3D hoses) and show › row › cell | the same core + `deck/show` `nodes.rs` `row_graph`, `canvas.rs`; `deck/kit` `base/graph.rs` layout, `base/signal.rs` | yes (ops) | egui (deck-kit) |
| OSCT **Logic** (`editor/src/lib/logic/`, `panels/Logic*.svelte`) | experience › group › scene › entity › node | `project.ts`, typed ports from specs (`spec-ports.ts`), `navigation.ts` (`?logic=` in the URL), `apply.ts` ops, issues bar | yes | `@xyflow/svelte` |
| Creator **Log** concept (redesign/LOG.md, #767) | lines scoped by address prefix (selection · item · chapter · season · toon · show · all) | – | – | – |

Research that already answered parts of this (read in place, not repeated):

- OSCT `agents/research/2026-11-20-logic-nodes-shared-concept.md` (#334): §1 the deck's two graphs in detail, §2 the reader's flow
  runtime, §5 the shared model, §6 levels, navigation and "the stage follows", §8 L1–L13 decided.
- research/outlets-on-models-and-node-view.md (#712): the deck node view's drop plan in the core, free nodes in the canvas only, sockets
  = the signal table plus `many` / `hidden` / `spec`, the gesture list (B1–B9 tickets #715–#723).
- research/audio-nodes-foundation.md (#713): the audio node set as factory modules, port types audio / control / gate / MIDI / clock.
- research/MODULE-WIRING.md (W2 world nodes, W5 levels), research/SCENE-PACKAGE.md (one graph format for instruments, rigs, scenes –
  undecided), research/scenes/CONCEPT.md §0 (what travels between containers: assets, sets, events).
- Owner decisions: [design] 2026-11-16 (signals flow on wires, frames, comments, collapse; direction LR / TB a setting; previews a
  switch), 2026-11-17 (one port / wire grammar for rows band, node panel, node view and 3D tubes – *"unification and ui sharing or dogma
  sharing intended"*), [rigs] 2026-11-08 (world nodes with an image of their origin; hand rewiring native only), [nodes] 2026-11-19
  (focus tone per signal, box select), [scenes] 2026-11-20 (no renaming, containers stay media specific).

## 2 · The network: scopes, levels, and what crosses them

A **scope** is a place in the address book with its own documents, rules and runtime. A **level** is one step of a scope's tree.

| Scope | Levels (root first) | Documents (the truth) | Runtime that runs it |
|---|---|---|---|
| Story | universe › toon › season › chapter › group › slice | chapter documents: groups, VN events, triggers, `jumpTo` | the reader (VN runner, TS) |
| XR | experience › group › scene › entity › node | OSCT content: behaviour, scene / experience rules, `goto` | OSCT's engine (three.js) in the reader |
| Show | show › row › cell | `show:<id>`: rows, fire, cells, `CellWire`, canvas | the conductor + decks over the relay |
| Rig / instrument | rig › track › node › port | rig, instrument, preset (catalog) | the deck engine (audio graph, its own clock – "another audio context") |
| Deck scene | scene › layer / slot | `scene:<id>` | the deck engine |

**Cross-scope edges** – the network proper. Each already exists as data; none is drawn across its scope today:

| From → to | How it is written (its home) | Travels by | Signal |
|---|---|---|---|
| chapter → chapter / season / toon / xr | `jumpTo` / `end.target` (`JumpToTarget`) | navigation | scene |
| story event → show row | the row's `fire.event` names a story event id | relay `story` message | event |
| show cell → deck scene / rig / character | `Play { piece }` | relay → deck | scene |
| show row → address | `addresses` (`character:yellowmouse > dance:samba`) | relay | address |
| variable set here, read there | VN `setVariable` / XR `set` vs `if`, `{expr}`, text slots | the save game (`global.x`, `toptoy.x`) | control (value) |
| item given / taken | `giveItem` / `takeItem`, carrier bag reads | the save game | address |
| XR rule → scene | `goto scene:<id>` | navigation in the engine | scene |
| world → rig | world nodes (MIDI, generators, carrier bag, VN state) | adapter → core | event / control |

So the top level of the one view is not a new document: it is a **network index** of these edges, computed from the documents. The server
already computes the story part (`FlowIndex.php`: toons, jump / choice / end edges, variables, dead things); the show and XR parts are
the same kind of read.

## 3 · One signal table for all scopes (no new signal)

The deck's `Signal` (deck-kit `base/signal.rs`, the owner's one grammar of 2026-11-17) already covers every edge above. OSCT took a
subset (event · value · node · scene · address · far, no clock: OSCT 4.6.1). The story flow maps onto it without a new entry:

| Story / XR edge today | Signal | Shape · night colour |
|---|---|---|
| continues, next group, choice → group, divert | event | triangle · #f3f3f0 |
| `jumpTo`, `end`, XR `goto` | scene | square · #ff6b8a |
| variable set / read, XR `set` / `when` | control (OSCT: "value") | dot · #46d36a |
| item, character, place, entity reference | address | diamond · #b46bff |
| a vote / choice counted live | vote | dot · #ffb020 |
| beat, bar | clock | dot · #e5484d |
| audio between rig nodes | audio | square · #19b8d8 |
| picture / video between nodes | picture | square · #6f9bff |
| an edge leaving the level (stub) | far | ring · #a7af9b |

`FlowEdgeKind` (story) then becomes a *label* on a typed edge, not a closed union of its own. OSCT's "value" and the deck's "control"
need one word – recommendation: keep both names as aliases of one entry for now (OSCT owns its docs), decide at Q4.

## 4 · The view contract, and where each part lives

```text
View      = { scope, context, levels[], nodes[], edges[], stubs[], problems[], canvas }      // OSCT §5.2, the deck's wire_view
Node      = { id, kind, label, detail, ports[], runtime, address, enter?, pick?, far?, picture? }
Edge      = { id, from:[node,port], to:[node,port], signal, label, home | null, crosses?: transport }
Stub      = { edge, side, far: address, label }                                               // a cable that leaves the level
Projector = (documents, context) -> View          Ops = (View, gesture) -> Op[] | Refusal       // per scope
```

Two additions to OSCT's model, both for the network level: `runtime` on a node (reader · xr · deck) and `crosses` on an edge (relay ·
navigation · save game), so the view can draw the lanes and say how a cable travels.

**Where the parts live** – the rules decide most of it:

| Part | Lives | Why |
|---|---|---|
| The contract (types above, signal table) | one schema, **generated** into TS and Rust (A6 [contract] mechanism 1) + shared fixtures `documents → View` (mechanism 4) | three languages read it (creator TS, OSCT TS, deck Rust) |
| Story projector | `shared/content` (where `flow.ts` is; becomes a projector over the contract) | story rules live there and in the reader; no deck logic |
| XR projector + ops | OSCT's `logic` package, consumed at a pinned tag (L9 = yes, contract shared) – or copied with #482 | OSCT built and owns it |
| Show and rig projectors + ops | the Rust core (`deck/show` `nodes.rs`, `rig/wiring.rs`), reached in the creator through wasm (`space-web`) | A1 / A6: the deck's graph exists once, in the core |
| Network index (top level) | computed from the projectors' stubs; the server's `FlowIndex.php` grows the show / XR edges or the client joins them | a read, no new truth |
| Web canvas | one Svelte canvas over `@xyflow/svelte` (both apps already on it), drawing any `View` – **round 2: perhaps deck-kit through wasm instead (§9)** | one renderer for the web |
| Native canvas | deck-kit `graph.rs` over the same `View` (egui), kept in step by the fixtures | the native editor can't host a web canvas (#763 D3); #764 may change this |
| Live feedback | each runtime posts activity by address (reader preview: fired events; XR stage: `observeEvents`, `allVars`; deck: `Activity`), rate-limited, into the tab | the owner's #763 comment |

## 5 · The tab

- **One tab "Flow"** (the creator's id stays `flow`), a universal shell tab like Tree, Library or Log: any workspace may show it.
- **It follows the front item** (`shell.front.address`): a chapter → chapter level; a group or slice selected → that level; a rig → the
  rig level; an XR scene → that scene; a show → its rows. A **pin** keeps the level when the author wants it to stay ("the level the author
  picked; a new selection follows again" – the creator's Flow panel and OSCT's L-T3 both do this).
- **Breadcrumb of levels** (`View.levels`), across scopes: *Universe › sandbox › Season 1 › Chapter 3 › Group 4*. ↑ / Alt+↑ goes up,
  double-click or Enter on a box with `enter` goes in, a click selects (the stage, Tree and Inspector follow, OSCT §6.3).
- **Stubs at the edge**, named by their far end (*"→ show Storm · row 4 fires"*). Clicking one **changes scope**: the tab steps into
  the show; the workspace's item stays until the author opens it (the tab may lead, it never drags the workspace along).
- **The network level** (Universe): boxes per toon, show, experience and deck rig, in three lanes by runtime; cables between them bundled
  by kind and pair with a count; a cable across a lane wears its transport. Variant B in the drafts nests the scopes instead of lanes.
- **Node types per scope** fill the add menu: story verbs (giveItem, jumpTo, choice …), OSCT verbs (play, show, set, emit, goto), show
  cells, the rig's audio nodes (#713) – the menu lists what the current scope's projector registers, filtered by the dragged signal (#719).
- **Editing follows the scope's own rules:** XR and show rows edit (ops); the story flow starts read-only as today; the rig's hand
  rewiring stays native only (2026-11-08 [rigs]) – the web tab draws it and links "Open in the deck editor" until the owner changes that (Q5).
- **Wires:** labels with an outline, no backdrop; a wire lights when its event fires or its value changes (subtle beads, 2026-11-17);
  the direction LR / TB is the workspace's setting.
- The **Log** (#767) and this tab share the scope idea: the Log filters by address prefix, the Flow tab draws the same addresses. A
  problem line jumps to its box; a box shows its problem count.

## 6 · Steps (titles to file after the owner's answers; not filed)

1. The contract: schema for View / Node / Edge / Stub / Signal, generated TS + Rust, fixtures (S).
2. The universal "Flow" tab: follows `shell.front.address`, pin, breadcrumb; today's story flow inside it through a projector over
   `flow.ts` (M).
3. Rig and show projectors to the contract in the core, exported through `space-web`; the rig Nodes tab and the show rows draw on the
   one web canvas; the native editor reads the same View (M).
4. XR scope: OSCT's package behind the contract (pinned tag or the #482 copy), editing included (M).
5. The network index and the Universe level with runtime lanes, cross-scope stubs and scope changes (M).
6. Live feedback by address from the reader preview, the XR stage and the deck (M).
7. Story editing in the tab (ops over chapter documents), if the owner wants it (L).

## 7 · Questions for the owner (each with a recommendation)

| # | Question | Recommendation |
|---|---|---|
| Q1 | The tab's name: **Flow** (creator), **Logic** (OSCT) or **Nodes** (deck)? | **Flow** – it is the creator's word and the deck's ("signal flow"); OSCT keeps "Logic" in its own editor |
| Q2 | The network level: **A runtime lanes** or **B nested scopes**? | answered 2026-11-21: B reads best, A's rows show where a thing shows → **C** (§9) |
| Q3 | Adopt OSCT's package (#702 L9): share the **contract + fixtures** and consume the package for XR, or copy it with #482? | share the contract and fixtures; the package itself copied with #482 (as decided for the editor) until OSCT's core tag exists |
| Q4 | One name for the value signal: deck "control" vs OSCT "value"? | one entry, two labels for now; decide when OSCT and the deck next touch `signal.rs` |
| Q5 | Rig rewiring in the web tab: stay native only (2026-11-08), or allow it in the creator? | answered 2026-11-21: **native only** (deck.md [rigs]); open detail Q12 |
| Q6 | Does a stub click change only the tab's scope, or open the far item in its workspace too? | only the tab; a second click ("Open") opens the item – the tab never drags the workspace |
| Q7 | Story editing in the tab (step 7) – wanted, or the chapter editor stays the place to write events? | later; read-only first, as today |
| Q8 | Order of steps 1–6 | 1, 2, 3, 5, 4, 6 – the deck's scopes before XR, because the deck's node view is the duplicate the owner named |

## 8 · Not checked

Nothing built or run. OSCT's package was read by its file list and commit log, not used. The web Nodes tab gets the core's whole
`Wiring` from the engine host (`shared/rig-designer/src/nodes.ts` header); `wire_view`'s levels and stubs are not exported to the web
yet (no hit in `deck/space-web`) – step 3 adds that. The cross-scope edge table (§2) is from the documents' shapes; the
show-row → story event link was read from OSCT §1.2 / §2 and research/LIVE.md, not traced at runtime.

## 9 · Round 2: C – the nested scopes in rows of where a thing shows

The owner, 2026-11-21, on round 1: *"love the nested scopes in term so readability, but wiht the runtime rows, i can see more clearly
the devices or groups something is potentially showing at. can we rethink this even fruther one time. rig rewiring native only.
potentially we will lean to integrate more and more into the deck rather than the web. my feeling at the moment. because we can always
access a deck in browser through wasm"* (thought aloud on the deck; recorded in creator.md and deck.md).

**C = B's frames placed in A's rows, but the rows change meaning: not runtimes, places that show.** Drafts `?v=5` and `?v=6`.

- **The rows come from the address book.** The readers' phones, AR, the conductor, and the show's **roles**: its outlets
  (`deck/show/src/document.rs` `Outlet`: a stage role is a screen or a speaker, a device role is a bridge's MIDI / DMX / OSC; adjacent
  roles share a `group` head such as *light & sound*). A role is taken by decks (#478), so a row says where a thing *may* show.
- **Two ways to read the rows, one switch.** *Where it may show* (`?v=5`, from the documents: roles) and *devices now* (`?v=6`, from a
  running session: how many readers, which deck took which role, a deck without a role at the bottom, a role nobody took marked
  empty). The relay knows both. Recommendation: roles outside a session, devices inside one, the switch to override.
- **Placement rule.** A scope sits in the row where it shows. The **show spans its roles**: its rows sit in the conductor's row and each
  cell in the row of the role that plays it, so the show's grid becomes part of the network. What a cell plays sits beside it in that
  row (the deck scene on the left screen, the rig on the sound role, the DMX look on the lamps). A thing that may show in several rows
  (a rig taken by two roles) is drawn once, in its first row, with a ghost mark in the others (not drawn yet).
- **The runtime stays visible** as the frame's chip (story · xr · deck · audio context), and the cables across rows keep their
  transport (relay per role, navigation, story events).

**The deck as the home (thought aloud, not decided – #764's question).** If more moves into the deck, this tab is the obvious first
piece: deck-kit already has the graph layout (`base/graph.rs`), the signal table (`base/signal.rs`) and the show node view with
groups, notes, folds and live activity. One renderer would then draw the tab natively and, through the deck's wasm in a canvas, in the
creator – instead of xyflow on the web and egui natively (§4's two-renderer problem goes away, D3 of the issue with it). The projectors
stay where their rules live and hand the View over the contract (story in TS, XR from OSCT, show and rig in the core), unless #764
moves the story's rules too. xyflow would remain OSCT's only.

**New questions**

| # | Question | Recommendation |
|---|---|---|
| Q9 | C as the network level (nested scopes in where-it-shows rows)? | yes |
| Q10 | Rows: roles outside a session and devices inside (automatic), or always the switch? | automatic, with the switch to override |
| Q11 | Draw the tab with deck-kit everywhere (natively, and in the creator through wasm) – decide now for this tab, or wait for #764? | make this tab #764's first test piece: deck-kit, hosted in the creator through wasm; fall back to xyflow only if the wasm host fails a measured budget (load, memory) |
| Q12 | "Native only" for rig rewiring: does the deck editor running as wasm in a browser count as native? | yes – the deck editor is the place to rewire, wherever it runs; the creator's own pages only draw |

If Q11 is yes, the steps of §6 reorder: 1 the contract · 2 the tab in deck-kit with the show and rig scopes it already draws, and its
wasm host in the creator · 3 the story projector feeding it · 4 the network level C · 5 XR · 6 live feedback.

## 10 · Round 3: the logic and the reception are two views, side by side

The owner, 2026-11-21, on round 2 (thought aloud): *"maybe C and B somehow. cause the devices are sort of additional in terms of
organisation. have to see what will be useful in the long run. i need a radically different view. maybe those two dont mix ... if we
have a secondary view that can go along side if we want to see where events are received on devices connected and potential. because
also this view mode we would also figure out which user iis receviing what or which user group"*.

**The split.** The logic stays **B** (nested scopes, readable). Devices are a second organisation, so they get **their own view** beside
it on the split stage (#765's two-way split), instead of rows inside the graph. Drafts `?v=7`–`9`.

- **One question, two directions.** Select a box or cable in B and the Reception view shows *who receives it and what it fires*:
  received now, sent from here, potential, or filtered out with the reason. Select a receiver in Reception and B dims to what reaches it.
- **Receivers come from facts that exist.** The relay tests every cell's reach against the show row's `where` (by outlet, protocol
  `Filter`: `group` – the QR's or a deck's outlet –, `device` – browser / native / phone –, `account` – guest / member / creator / admin /
  live –, `approved`) and a role's `for` (`Admits`, #478). So the view groups readers by **group** and **account** (the "user group"),
  decks by **role**, and can name one person where an account is signed in. *Potential* = would be admitted but isn't there (a group
  with 0 now), or is there but not admitted (an unapproved deck); *filtered* = there, but a `where` fact excludes it.
- **Three ways to show the same answer** (one switch in the view's head):
  - **Matrix** (`?v=7`): receivers (grouped) × what the selection fires; a cell is a mark, a count, a latency, or the excluding fact. For
    checking that the right user group gets the right thing.
  - **Constellation** (`?v=8`): the audience as it is – readers as dot clouds per group, decks as squares by role, the conductor; the
    selection in the middle, lines to who received, dashed to potential, the excluding fact in red. For the live show.
  - **Timeline** (`?v=9`): one lane per receiver, time running to *now* on the beat grid; taps sending the event, the row firing on the
    beat, each reception with its latency. For order and lag.
- **Where the numbers come from.** Outside a session: from the documents (roles, `where`, the toons' groups) – only *potential*.
  Inside one: the relay's counts and receipts (`here` per role already exists; per-message receipts would be new), posted by address.
- **Deck-first still holds** (§9): both views are deck-kit material; the Reception view is close to what a conductor needs anyway.

**Questions**

| # | Question | Recommendation |
|---|---|---|
| Q13 | B as the logic view, Reception as a separate view beside it (instead of C's rows)? | yes – C stays as a drawing option only if the Reception view isn't enough |
| Q14 | Which Reception form first: Matrix, Constellation or Timeline? | answered 2026-11-21: **all three are built**, one may be dropped later; the rendering stays modular – one reception model (receivers × what the selection fires, with now / sent / potential / filtered and the reason), one renderer per form |
| Q15 | Per-message receipts from the relay (who received what, with latency) – wanted, at what cost? | counts per role and group first (cheap, partly there); receipts sampled, only while the Reception view is open |
| Q16 | Show single users (signed-in accounts) in the Reception view, or groups only? | groups by default; a user only on search, and only for the show's creators (privacy) |

## 11 · The scope drafted, Reception as a tab, its later maps, the style

The owner, 2026-11-21: *"the reception view can also be its own tab of course. so we could use it in any workspace … and i havent
seen the scoping idea properly drafted."* · *"for the receiption view we can introduce a world map later and a venue map ... venue map
can work with a blueprint of a venue or a background picture we can pin devices to. and world view will show geo locations of
recipience in real time."* · on the style: *"redesign so it matches our dogma"*.

**The scope (`?v=10`, a storyboard).** One scope for the whole shell: an address (`chapter:sandbox/1/2`). Every universal tab reads it –
Flow draws it, Reception lists who receives it, the Log filters its lines by the address prefix (LOG.md), the Library offers what fits
it. Four moments:
1. *Follow* – the front item is Chapter 2, the scope is Chapter 2 (selecting inside narrows it to a group or a cell).
2. *Leave the item* – following the stub *→ show · row 4* moves the scope into the show; the workspace stays on Chapter 2, the path
   pill says *scope ≠ item* and offers ↩.
3. *Enter* – a double-click on *cell · sound* goes into the rig the cell plays (the deck's scope; rewiring opens the deck editor).
4. *↑ to the top* – the network, with the trail of where you went lit; pinned until a new front item or ↩.
The scope pills (Selection · Item · Chapter · Season · Toon · Show · All) are the Log's, a quick way to widen it.

**Reception is a universal tab** (with Log and Library), any workspace may show it; split beside Flow is one arrangement, taking the
stage another. One reception model, one renderer per form (Q14).

**Later forms, drawn (`?v=11`, `?v=12`):**
- **Venue:** the venue's own blueprint or a photo, uploaded once per stage environment; devices dragged onto it where they hang,
  each lit in the colour of what it received; reader groups pinned to zones by their QR (a zone = a group), so a zone lights as one.
- **World:** recipients by city, live – a dot per city sized by count, lit where the event arrived, dashed where it may; arcs from the
  venue. City level only and only with consent (the reader's location is personal data; Q17).

**The style** follows the deck's node canvas now: one card per node with its whole thin border in its kind's colour and a faintly
tinted head (#648), groups on a dotted grid in their colour with a fold / swatch / label head (2026-11-16), wires that hang with a soft
glow and small dim beads (2026-11-17), rows as hairlines edge to edge, see-through panels with the ink ring on the black stage.

| # | Question | Recommendation |
|---|---|---|
| Q17 | World map: where does a recipient's location come from? | answered 2026-11-21: the map is **zoomable**, OpenStreetMap in our own monochrome style, close enough to see where a device is (§12); readers stay a coarse cell, opt-in (Q19) |
| Q18 | Venue map: zones as QR groups – one QR per zone printed in the venue? | answered 2026-11-21: **yes**, and the Reception view is the place to get a group's QR (§12) |

## 12 · Round 4: QR per group, the map, latency and compensation, no extra load

The owner, 2026-11-21: *"we got this so this is good place to retrieve the qrs specifically for a group. q17 i say this map is
zoomable. a map we need some open world map with simplified monochrome view. so we can see pretty close were some device is. all of
these views, i would love to see what real latency is or are... and if we got timed events later, when they will arrive with a count
down. and if we can use these latencies and compentsate for them. somehow"* · *"also in this exploration please consider performance of
the view and how we not additionally overload the relay"* · *"open street map i was meaning. perahse we find an alterantive that is
possible to download ahead of a show and something we can run in the deck without a browser view present"* · with a screenshot of
Maputnik's dark style: *"we can use this, give it our colors and give the entire thing a more cyber science fiction vibe matching our
game. venue maps can be fictional maps, we can build based on real stage designs. and venue concepts."*

**QR per group (`?v=11`).** A zone on the venue map is a group (protocol `Filter.group`, the QR's group); each zone carries its QR
with *print*, so the Reception tab is where a group's join code is picked up.

**Venue concepts (`?v=11`).** A venue map is a **venue concept**: real or fictional, built on real stage designs, drawn with the kit's
tokens like every other view. It belongs to the **stage environment** (the per-venue layer, A11): devices
pinned where they hang, zones as groups.

**The map (`?v=12`).** OpenStreetMap data, zoomable to street level, in **our own style** – made in Maputnik (a MapLibre style:
one JSON file) with the kit's tokens only, no extra decoration (owner 2026-11-21: *"just foloow our deck and creator ui kit dogma … basic. focused on point"*). Devices exact (they are ours),
readers a count per ~250 m cell, never a point (Q19).
- **Offline and native:** the show's region is cut **ahead of the show** into one vector-tile file and preloaded with the show's media
  (the deck already preloads by horizon); the **deck draws it itself** (deck-kit painter, wgpu), no browser view, no relay. The world
  overview is a small low-zoom file. Attribution *© OpenStreetMap contributors* stays visible (ODbL).
- Candidates to check before a ticket (not verified here): Protomaps' PMTiles (one file of vector tiles, a region cut from the planet);
  in Rust a PMTiles reader and an MVT decoder, drawn by our own painter; our Maputnik style read for the subset the deck draws (layers,
  filters, colour / width / opacity by zoom). Open: whether to read the MapLibre style spec or keep a small style of our own.
- **Browser first is possible** (owner 2026-11-21: *"maplibre also has a ts version for the browser. just in case our first
  implemetnation is browser based"*): MapLibre GL JS draws the same Maputnik style over the same region file in the creator; the
  deck's own renderer comes later and reads the same style and file – so a browser start costs no second style. (A browser map is UI,
  A2; nothing it draws decides anything.)

**Latency in every form, countdowns, compensation (`?v=13`, `?lead=adaptive`).**
- Every receiver shows **network one-way + its own output latency** (projector lag, audio buffer, DMX frame).
- A timed cue (a row on a bar) shows, as one quiet line, when it plays and who it will reach.
- **What exists:** one shared clock (`ping` / `pong`, the offset's median of five), every cue carries `at = now + lead`, a client plays
  at `at − offset`; ≤ 300 ms late plays at once, later only the state (research/LIVE.md B.6; the relay's `RELAY_LEAD_MS`, 150 hosted / 50 LAN);
  the relay already tracks every link's smoothed round trip (`relay/src/link.rs`).
- **New, per device** (owner 2026-11-21: *"the latency adapt needs to actually be per device otherwise this is kind of a weird
  approximation"*): each device has its own **need** = its link's one-way p95 (the relay already smooths each link's round trip) + its
  own output latency (reported once and on change: projector lag, audio buffer, DMX frame) + a small margin. A cue's lead is the
  **largest need among the devices it reaches**, capped (e.g. 250 ms); the relay sends to all of them at once, and **each device starts at
  `at − its own output`**. A device whose need is over the cap plays at once and is marked late – the show never waits for the slowest
  phone. The lead moves like the deck's adaptive audio lead (deck-core `lead.rs`): take the margin at once, give it back slowly. Later:
  an in-venue calibration (a clap / flash measured by a deck's microphone or camera) for output latencies.

**Performance and relay load.**
- **The relay sends nothing new per message to the audience.** It already measures each link; it keeps **aggregates per group**
  (count, p50, p95, late, state-only) and sends them **only to a subscribed Reception tab**, at 1–2 / s, with the existing `presence`
  and `vote.tally` traffic as the model (250 ms to the conductor today).
- **Receipts:** full from decks and bridges (few, on the LAN), a **sample** of phones (e.g. 1 in 20, capped), riding the existing `pong`
  / keepalive – never one message per phone per cue.
- **Locations** are sent once at join (coarse, opt-in), not streamed; **map tiles** never go through the relay.
- **The view:** crowds are density (one canvas, not a node per phone); level of detail by zoom; it pauses when hidden and drops to a
  few frames a second when nothing moves; every animation can be switched off (2026-11-16 [design]).

| # | Question | Recommendation |
|---|---|---|
| Q19 | Readers on the map: exact with consent, or always a coarse cell? | answered 2026-11-21: **always a coarse cell** – a reader's place is snapped to a grid cell of about 250 m and only the cell's count is shown, never a point; a cell with fewer than 5 readers merges into its larger parent cell, so no single person can be picked out. Devices that are ours are exact |
| Q20 | May the send time differ while `at` stays one moment? | answered 2026-11-21: **yes** – and the adaptation is **per device**, not per group: each device's own need, one lead per cue from the largest, each device starting at `at − its output` |
| Q21 | Map style: read the MapLibre style spec in the deck (one style for web and deck), or a small style of our own? | answered 2026-11-21: **yes, the MapLibre spec** – GL JS in the browser first, the deck reads the subset later; Maputnik stays the editor |
| Q22 | Venue concepts: a new asset kind in the Library, or part of the stage environment document? | answered 2026-11-21: **part of the stage environment**, the plan as its image slot |

