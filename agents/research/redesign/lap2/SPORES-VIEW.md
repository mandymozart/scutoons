<!-- status: concept draft for the owner, round 1 (2026-11-24) – the Spores view and the picker at an inlet, with the Show's row as the first case; nothing decided except what is marked "owner's word". Picture: spores-view.html (?v=0…5). Builds on redesign/NETWORK.md (#763, round 4) and AI-HOOKS.md §0; facts about today read from deck/show and deck/editor. -->
# The Spores view – draft, round 1

## The owner's words (2026-11-24)
- *"So basically a row can be triggered by any event in our universe, right."*
- *"So, we decide UI but we have to think universe, hence we also need to think spores view."*
- On where the spores canvas lives (the agent had said it is drawn in the creator today): *"That is a mistake. I thought the spores canvas is drawn from the deck in wasm?"*
- Earlier, standing: 2026-11-21 (#763) the tab is "Spores", a node is a spore, connecting is weaving, a port connects data, a pore organic matter, what runs a spore is a driver · 2026-11-21 spores get job and asset signals (a night run, a publish, a draft saved, a picture arriving in another spore, a text approved …) · 2026-11-21 thought aloud: more moves into the deck, since a deck can be reached in the browser through wasm · 2026-11-24 the list "fires on" *"is not much more than a selection tool … we got the slot that indicates the event … and something in spores which indicates incoming events"*.

## What exists today (read 2026-11-24)
| Thing | Where | State |
|---|---|---|
| A row fires on **one story event of the show's own toon**, by its id, or by hand; an event already used by another row is refused | `deck/show` story.rs, rows.rs (`fire.event`) | built |
| The picker: the row inspector's "fires on" opens a list in the stage's place, the toon's story events, found by typing | `deck/editor` show/events.rs | built |
| The row drawn as a graph: a far spore **reader** (the event), a far spore **clock** (beat, bar), the **row** (in: on event, on next bar; out: fire), its **cells**, **this row sends** | `deck/show` nodes.rs `row_graph`, the Show's Nodes view (LR · TB · Fit · Note) | built |
| A scene hears outside events (MIDI, a generator, a far deck) through its `listen` spore | `deck/space` world.rs | built |
| One network of scopes (story · XR · show · rig · deck scene), the crossings between them, one signal table, the tab that follows the front item, the Reception view | redesign/NETWORK.md, network.html | drafted, four rounds, not built |
| The spore canvas for generating (a weave ending in an output spore) | redesign/AI-HOOKS.md §0, spores.html | drafted |
| The creator's Generate canvas | `creator` generate/spore-canvas.svelte | a stub of cards in Svelte: nothing is woven, runs or is saved – **not** the deck's canvas |

## The draft in four sentences
1. **Everything that can be woken has the same inlet** – the ⚡ slot: a show's row, a scene's listen spore, a generator's run, an XR rule.
2. **Everything that fires is an event with an address** in the one address book (`event:<id>`, #707), under the scope it comes from.
3. **One connection, two ways to make it.** At the inlet: press it, Find, pick. In the Spores view: the same connection as a cable. Both write the same line in the listener's own document (the row's `fire.event`); neither is a second truth.
4. **The Spores view is one view, drawn by the deck** – natively in the deck, through the deck's wasm in the creator (the owner's word above; NETWORK.md Q11). It follows the front item's scope and steps up and down its levels.

## What fires
| Source | Example | An event today? | Can fire a row today? |
|---|---|---|---|
| Story | `ch3.dialog-start` (catpope › chapter 3 › nave) | yes, with an id | **yes – the only one** |
| Clock | beat, bar | yes | it is the row's *when* (the moment), not its *what* |
| Show | another row fired, a vote's result | yes inside the show | no |
| Scene | something inside a deck scene | yes inside the scene | no |
| Device | a MIDI note, a controller, DMX in | yes, into a scene or rig | no |
| Place | arriving at a place (XR, the map) | drafted | no |
| Jobs and assets | a night run, a publish, a picture arrived, a text approved | drafted (2026-11-21) | no |

## What listens (the same inlet on each)
| Listener | Its line today |
|---|---|
| A show's row | `fire.event` – built |
| A scene's listen spore | a filter on its sources – built |
| A generator's run | "on: sheet changed" – drafted (spores.html step 8) |
| An XR rule | `when` – built in OSCT |

## The picker at an inlet
- **Press the ⚡ slot: Find opens under it.** No tab, no panel on the stage (the owner, 2026-11-24).
- **It starts where today's list is:** the events of the show's own toon. One row of scope pills widens it – *This toon · Show · Universe* – the Log's scope pills (NETWORK.md §11), not a new control.
- **A line:** the event's id, where it comes from (its path), and who already listens to it. Wider than the toon, the lines are grouped by source.
- **What cannot be picked is greyed with the reason** – the same way a slot greys a misfit ("no id – give it one in the chapter").
- **"manual" stays the first line** and clears the inlet.

## The Spores view, level by level (what the picture draws)
| `?v=` | Screen | From |
|---|---|---|
| 0 | **Today:** the Show with its Nodes view on row 02, as built – reader, clock, the row, its cells, this row sends – in the approved kit | the deck's row graph |
| 1 | **The inlet's picker:** row 02's ⚡ pressed, Find, the scope pills, the lines; a second state widened to Universe with groups and greyed kinds | today's picker + this draft |
| 2 | **Spores · the row:** the same graph; the far spore "reader" becomes the event's own stub, named by its far end (*catpope › ch 3 › nave · ch3.dialog-start*); the row's "on" port opens the same picker | row graph + NETWORK.md stubs |
| 3 | **Spores · the show:** every row a spore with its inlet; the events that fire them at the left, grouped by where they come from; rows by hand have an empty inlet | show.html v3, corrected |
| 4 | **Spores · the universe:** the toon, the show, the scenes and the deck as nested scopes; the one crossing *chapter 3 → top-toy · row 02* lit; the breadcrumb | NETWORK.md round 2–3 (B) |
| 5 | **The same inlet elsewhere:** a scene's listen spore, a generator's run, an XR rule – each with the ⚡ slot and the same picker | AI-HOOKS.md §0, world.rs |

The look: palette D, see-through grounds, ports and cables in their signal's colour and shape (the deck's signal table: event triangle, scene square, value dot, address diamond, far ring) – the one exception to "no colour" (owner 2026-11-24, replacing kit point 14).

## Open – each needs the owner's word
1. **Answered 2026-11-24: yes – several listeners can hang on one event.** Asked as: **One event, many listeners?** Today an event fires one row; a second row picking it is refused. In a universe-wide network several listeners are the normal case. Recommended: allow it; the picker's line says who else listens.
2. ~~May a row fire on several events?~~ **Answered 2026-11-24: yes, several – any of them wakes the row.** Today one (`fire.event`).
3. **Which sources can fire a row in lap 2?** **Answered 2026-11-24: the ones not built stay greyed in the picker, as a marker that they get a plan of their own after this iteration.** Confirmed 2026-11-24: besides story events, **a row's fire and a vote's result count as sources** in this lap. Recommended: story as today, another row, a vote's result; the rest drawn greyed as "later" so the picker's shape already holds them.
4. ~~Signals by shape only?~~ **Answered 2026-11-24: no – signals keep their colours, the one exception under palette D (RULES H11); the deck's signal table is the concept, OSCT's missing kinds are adopted.** The pictures are redrawn with the table's colours. Asked as: NETWORK.md gave each signal a colour (2026-11-17's grammar); palette D and kit point 14 put cables in the text colour. Recommended: shape only, a cable lights when its event fires.
5. **Answered 2026-11-24: not in this lap – with #764.** *"This is a design and UX round. That other thing is fundamental refactoring."* Asked as: **The creator's Generate stub** (Svelte cards): replaced by the deck's canvas through wasm – in lap 2, or with #764 (the creator inside the deck's wasm)?
6. **Answered 2026-11-24: yes, "Spores" is decided.** Asked as: **The deck's "Nodes" tab** becomes "Spores" (the name decided 2026-11-21) – applied with this lap?

## The demo – two chains played step by step (spores-demo.html)
The owner, 2026-11-24, on round 1: *"I want to have a demo of how this would be triggered from an action like a found XR target or a generation that finished or got triggered."*

One page, the Spores view at the universe level (nested frames as in `?v=4`), a **Play** on the source spore, step dots, ← / →. Each step lights one hop – the spore, a bead travelling its cable, the next spore – and adds one line to a Log strip. Every frame says where it runs (phone · relay · conductor · deck · server); every step says whether it is built.

**Chain A – a found XR target fires a row** (`?d=xr`). The listener is row 04 finale (by hand today), its ⚡ set to *target found*.
| Step | What happens | Runs on | Today |
|---|---|---|---|
| 0 Woven | row 04's ⚡ shows *target found*, picked in the picker under Universe › XR | – | drafted |
| 1 Found | a reader's phone sees the target; the XR engine fires `targetFound` | the reader's phone | built in OSCT |
| 2 Sent | the event leaves the phone the way a reader's story event does: a `story` message to the relay | relay | built for story events; XR events are not sent |
| 3 Wakes the row | the conductor matches the row's ⚡; how: auto; it fires on the next bar | conductor | built (auto · suggest · hand, the moment) |
| 4 Plays | the row's cells play on the decks that hold its roles | decks | built |

**Chain B – a generation is triggered, runs, and its finish fires two listeners** (`?d=gen`). The generator is AI-HOOKS's example (Cat Pope's sheet → image to 3D → auto-rig → output GLB); its ⚡ is its run, its outs are the run's states.
| Step | What happens | Runs on | Today |
|---|---|---|---|
| 0 Woven | the generator's ⚡ shows *sheet changed*; the 3D model slot holds the generator as its stand-in; row 04's ⚡ shows the generator's *succeeded* | – | drafted |
| 1 Triggered | Cat Pope's sheet is saved; *sheet changed* wakes the generator's ⚡ | server | drafted (jobs and assets) |
| 2 Runs | the run is `queued`, then `running`; progress shows on the spores inside | server, the tool behind the hook | the states exist for storyboard generations |
| 3 Succeeded | the run is `succeeded`; the out of that name fires (`failed`, `cancelled` stay dim) | server | the state exists; it is not an event yet |
| 4 The slot takes it | the 3D model slot takes the yield as a new version (auto-accept) | server | decided 2026-11-22, not built |
| 5 A row fires | the same *succeeded* wakes row 04 – a second listener to one event (open point 1) | conductor, decks | not built |

Names that are not in production (the XR experience and its target) are generic and marked as examples.

7. **Answered 2026-11-24: any reader's. Later a filter can allow some automatically.** Asked as: **Whose event counts?** (new, from chain A) A target is found on one reader's phone. Does one reader's find fire the row for everyone – any reader, the readers of one group (a QR group), or a count ("ten readers found it")? Today's `how` (auto · suggest · hand) already lets the conductor hold it: *suggest* waits for GO.
