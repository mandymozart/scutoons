Status 2026-11-09: concept, under discussion (#462), nothing built. Rows, cells, the conductor tree, workspaces, triggers, addressing, filters. §2 is the owner's word; the rest is a proposal until the owner says otherwise. Would change LIVE.md B.2 and B.5.5 (§10).

# SHOW — how a live show is organized

Pulled together from the owner's statements of 2026-11-09 (`agents/decisions/deck.md`, `[live]` `[events]`) and the drafts on #462.
Builds on LIVE.md B.5 (show, outlets, cells), LIVE-LENSES.md, and the decisions of 2026-11-02 and 2026-11-03 on the conductor tree.

## 1. The whole in short

- The **creator** writes the story and its events. The **editor** builds pieces and shows. A **deck** plays. A **conductor** is a deck that also holds the authority over a show and passes on to its children.
- A show is a matrix: **rows × outlets = cells**. A cell plays one piece with values.
- Events from a reader fire rows, on autopilot or as suggestions. The operator can always fire anything.
- Anything can be addressed along one ladder, from a kind of entity down to a parameter, with an optional filter.

## 2. What the owner has said

| Statement | Date, key |
|---|---|
| The conductor is a role plus a scene type, not a deck kind. A conductor can relay to other conductors and decks: a tree. Its look is a scene on any deck | 2026-11-02 `[live]` |
| Sub conductors can have their own time; the clock is a question per subtree | 2026-11-03 `[clock]` |
| A rig can be held by other catalog items, locations, in the end almost any entity | 2026-11-09 `[rigs]` |
| Cell fills are not universal enough. Idea: a scene called video player, assigned a specific video | 2026-11-09 `[live]` |
| Going row by row; availability belongs to that. Single-cell triggers would add to the mess | 2026-11-09 `[live]` |
| A launched cell can be only parameter updates to what plays, without a reload | 2026-11-09 `[live]` |
| First use case: one phone reader connected to the conductor. Events set up in the chapter editor trigger rows. A character entering can be listened to or triggered in the viewer | 2026-11-09 `[live]` |
| The conductor takes events on autopilot or as suggestions, so accidental scrolls cannot create chaos | 2026-11-09 `[live]` |
| The conductor has a performance mode and needs a way to configure and create; it may belong in the deck editor | 2026-11-09 `[live]` |
| **GO is always allowed.** The conductor can start whatever; it is their responsibility | 2026-11-09 `[live]` |
| Relations are built on hierarchy with fallbacks: "callcenter or Arbeitsamt". For every entity, down to module and parameter | 2026-11-09 `[events]` |
| **An address that finds nobody does not fall back upward. It stays silent, and that is logged** | 2026-11-09 `[events]` |
| A filter can be applied to events: group, browser, hardware, periphery, previous actions, game state, inventory, condition, health, testo … | 2026-11-09 `[events]` |

## 3. Who does what (proposal)

| Part | Job |
|---|---|
| Creator | The story, the events in the chapter editor. Event ids are the contract with the show |
| Show core (new, Rust) | The show document, triggers, auto / suggest / hand, resolving cells, the conductor tree. Recursive from the start. Editor, conductor and web only draw it (RULE A1) |
| Editor: **Build** | Rigs, instruments, modules, scenes. What exists now |
| Editor: **Show** | Rows, cells, links, triggers, outlets, venues |
| Editor: **Perform** | The same matrix locked: GO, suggestions, readiness, panic, votes, the event log |
| Deck | Plays. Carries no editor code (research/EXTERNAL-EDITOR.md §5) |

If running the whole editor at a show proves too heavy, a conductor-only app reuses the same matrix and core.

## 4. The tree

```
show › row › cell › rig or scene › track › node › parameter
```

- A cable is saved one level above what it joins, and a row is what joins cells. The row takes the place of the "scene" level drafted earlier, and outlets replace the roles like `deck:left`.
- **Conductors form a tree.** A subtree is what goes together: it fires its own rows (following its parent, the reader, its operator or the clock), links stay inside it, and it can have its own clock.
- **Down:** rows, cells, events, the clock. **Up:** values, readiness, events.
- A cell on a child conductor says "go to your row 3" or "you are free", so a show nests as a rig does.
- In the editor a sub conductor is entered like an instrument: a breadcrumb, its own matrix, tabs for the parents.
- If a parent goes offline, its children keep playing their last row.

## 5. Cells

A cell is a piece and its values. A piece is a scene, rig or module; the current layer kinds become factory pieces (`chapter`, video player, poll …), so nothing breaks.

| Mode | Means | State |
|---|---|---|
| `play` | This piece with these values. Same piece already there = it only moves | new, replaces layers |
| `values` | Keep the piece, change these. Names the piece it expects | new |
| `keep` ↑ | Nothing happens on this outlet | designed |
| `mirror` | Same as another outlet | on the wire |
| `black` | Black, idle, all off | on the wire |

- You write only what changes. The conductor resolves keep and values along the path really taken and always sends a **full cell**. The deck compares: same piece, the values glide; other piece, load and crossfade.
- Full cells because a deck that reboots mid-show gets the snapshot and is right at once.
- A `values` cell on a path where another piece plays: the editor warns on that path; at the show it is skipped and logged.
- A piece says which outlets can play it, with a fallback for phones.
- A pure light controller is a plain scene today. Later a module or rig reaches lamp outputs by address (`dmx:`).

## 6. Reach

| Between | Reach | Saved in |
|---|---|---|
| Inside a piece | Always | the rig or module |
| Cells of the same row | While both ends stay on stage | the row |
| Another row | No link. A kept cell ↑ is the same piece, so it counts as this row | nothing |
| Another subtree, another deck | Events only, may stay silent | an address |

- A link ends when one end is replaced; the input glides back to its own value.
- The editor checks along every path the flow can take. The typeahead offers what is on stage in this row and greys the rest, with the row where it plays.

## 7. Firing

- A row says which event fires it and how: **auto**, **suggest** or **hand**, with a hold time and a moment (now, next bar, next phrase). An event must stand for the hold time before it counts. While it is a suggestion, a newer event replaces it and scrolling back removes it.
- **GO is always allowed**, also before the reader got there or with media not ready. A warning can show; it never blocks.
- A row can fire an event back into the viewer (a character enters, a vote result).
- On autopilot, a row whose media is not ready is held and shown as a suggestion (proposal).
- Launch model, proposal: **row per subtree, starting as one tree with every outlet**, which is exactly row by row. A single free cell is a subtree of one, which can only exchange events. Any-cell launching (5 rows × 4 outlets = 625 states to check, in the example) is what makes availability unknowable, so it is not proposed.

## 8. Addressing

**One ladder:** `kind › id › persona › look › track › module › parameter`. Any entity kind can be on top. Persona and look exist for characters only. Below the entity the ladder goes into what it plays.

**Two directions:**
- **Who picks up goes down.** An address reaches everything active under it.
- **Who answers goes up.** The receiver looks for a handler at look, persona, character, then the kind's default, as A-pose views and 3D models do (2026-11-03). Nobody knows: nothing happens, logged.
- A specific address that finds nobody does **not** go up (owner). It stays silent, logged.

| Sent | Who picks up | Who answers |
|---|---|---|
| `location:* > close_doors` | Every active location | Its own rig, else the location default |
| `character:yellowmouse > dance:samba` | Every yellow mouse, any persona or look | Look, persona, character, then the character default |
| `character:catpope.mountaintiger.ghost/delay.time = 500ms` | Only that look | That module's parameter |
| `item:*/being:glow = 0.8` | Every active item | That input on each |

- Spelling: dots for persona and look, as rig subjects do; `_` and `-` mean view and mood in asset names. The ladder's `/` and leaf forms are those of `rig/address.rs` (RULES A14).
- A place in front narrows the reach: `/deck:foyer/location:*`.
- Many matches are fine for a target, never for a source.
- A wide address is resolved when a piece loads, into compiled slots; nothing on the audio or frame path parses an address.
- Delivery is push: an event travels down the conductor tree and each receiver matches it against what it plays.

## 9. Filters

`address where filter > action`, for example `person:* where device:browser and inventory:sword >= 1 > haptic:pulse`.

- Reuses the chapter editor's "Only if": `{ variable, op, value }` or an expression.
- Facts are named by where they come from (A14): `group:`, `device:`, `periphery:`, `story:`, `inventory:`, `stat:`.
- The receiver tests itself, so inventories and stats need not travel. The filter tests whoever picked up. A missing fact means the filter fails.
- People become a kind on the ladder (`person:*`).

**The log:** one list, seen in Perform, kept with the show run: address, how many it reached, passed the filter, answered, or "nobody".

## 10. What this changes

| Where | Today | Would become |
|---|---|---|
| LIVE.md B.5.5 | The stage builder is in the creator | In the deck editor (Show) |
| LIVE.md B.5.5, 2026-10-07 | "Not a clip launcher": rows never launched, no launch quantization | Rows fired by events and by hand, with a moment |
| LIVE.md B.2 | The conductor runs the reader | A separate reader's phone is connected to the conductor |
| LIVE.md B.5.2 | A cell is a list of layers | A piece and its values |

**Timing:** LIVE.md has building start 2026-11-16 in lane 2 (research/KANON.md §8, issues #89–#91) for the Top Toy workshop on 2027-02-17. This concept must either fit that or be cut for it.

## 11. What exists in the code (read 2026-11-09)

- `cell` is per outlet and full (`deck/protocol/schema.json`), compiled in `deck/outlet/src/plan.rs`. No layer kind for a scene or rig. The `outlet` node sits inside a scene (`scenes/outlet.toml`).
- Video: a layer exists, the native outlet shows its still (#179). `video` in the catalog is `F_PLANNED`, not a graph kind.
- Poll: `vote.*` messages exist. No poll layer, and no vote handling on the native outlet.
- Light: a plain scene with `[outputs]` (#211). A rig has no outputs.
- A mood turn moves values without a reload; a loaded 3D set stays and only the spot moves. No message sets a named value on a deck from the conductor.
- Far events (#442, `far.rs`): one exact address, subscribed by the asking deck, only while the other deck is in the session.
- `address.rs` has places `deck:`, `scene:`, `character:`, `item:`. No `location:`, persona or look level, or wildcard.
- The relay is flat; the uplink only mirrors. `Tempo.domain` is reserved, nothing sets or routes it.
- The conductor page is the vote panel only (86 lines), opened on a deck with F12. No rows, no matrix, sends no cells.
- `story` messages exist on the wire. The reader bus of LIVE.md B.1 is not in `event-processor.ts`, and there is no reader role.
- A scene cannot hold several rigs yet.
- "Only if" exists in the chapter documents; the evaluator is TypeScript (`shared/content/src/expr/evaluate.ts`). The deck has no condition type. RULE A1: one evaluator, in the core, reaching the viewer through wasm.
- `deck/events/src/horizon.rs` walks what a scope can reach, nearest first: the same question as loading ahead.

## 12. Open

1. Launch model: row per subtree, starting as one tree (§7)?
2. One show document per conductor, the parent pointing to it (proposed: a child can be built, rehearsed and reused alone), or one for the whole tree?
3. Do the LIVE.md changes in §10 stand, above all the stage builder moving and "not a clip launcher"?
4. What fits before the Top Toy workshop?
5. Multichannel pan (#459), labels (#461) and nesting (#458) touch this; nesting should follow §4.

## 13. Drafts

Concept images (sent to the owner in the chat, not in the repository): `n0-tree`, `n1-reach`, `n2-cells`, `n3-launch`, `n4-editor`, `n5-conductor`, `n6-build`, `n7-show`, `n8-perform`, `n9-flow`. `n1`–`n5` still draw groups and one flat conductor; §4 is the correction.
