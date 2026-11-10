Status 2026-11-10: concept (#462); the show document, the conductor and the tree exist in `deck/show` and `deck/relay` (#89, #90: reader role, relay tree, sub conductors, flash, addressing, `show:` saving); the Show and Perform workspaces are built in the deck editor (#91, the matrix, rows with addresses, the inspector, the 2D overlay, flash with each deck's report, the cue sheet, the problems list, fixtures and looks; venue patches, rehearsal, drills are open). Rows, cells, the conductor tree, workspaces, triggers, addressing, filters. §2 is the owner's word; the rest is a proposal until the owner says otherwise. Would change LIVE.md B.2 and B.5.5 (§10).

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
| A parent may lock another member's sub conductor only if that seat accepted it | 2026-11-09 `[live]` |
| Each subtree launches its own rows | 2026-11-09 `[live]` |
| One show document per conductor, related by seats saved in the parent's document | 2026-11-10 `[live]` |
| The workshop scope is the cut in §10, with sub conductors in it | 2026-11-10 `[live]` |
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

## 4b. Saving and delegating (proposal)

Three separate things: the **show** (a document saved in an account, versioned, with lineage), the **deck** (paired to a member) and the **seat** (the place a parent keeps for a child).

| | Same member | Another member |
|---|---|---|
| The child's show | Their own, a separate document | Stays with its owner, the parent gets no copy |
| Getting a seat | Picks their own deck | Invite and accept, like pairing a deck |
| What the parent sees | Everything | Only the face: named rows, events, outlets (like `rig/face.rs`) |
| Parent's word | Follow, suggest or free, per seat | Same, the child's owner decides |
| Media and rigs | Own rights | Lent for the show, gone after it (`Rights.lent`) |

- **Flashing:** a flash is one assignment: the show at a pinned version, what it references, the deck configuration, the role. A conductor gets the same as a deck. Online it trickles down the tree as set on 2026-11-02 (a lock from above wins, otherwise the nearest wins, a deck may refuse, every deck reports what it runs). Offline the same package goes on a stick (2026-11-08 `[scenes]`: flash configs to decks).
  - Built (#91, deck-show `ready.rs`): each conductor passes the flash down with `here`, its own show at the pinned version. A deck takes it by the same rule, keeps that show as `<cache>/shows/<id>@<v>.json` (fetched once from `/api/shows/<id>?v=<n>` with its token), checks every piece its outlet's cells name (deck-space `piece::available`, as a cell would load it) and answers `flash.report` with `outlet`, `ready` and `lacks` in words. A missing or broken piece is a reason on the HUD and in the report, never a block on GO.
  - **Choosing what to flash (owner 2026-11-10, #471, not built):** Flash opens a list of every device of the tree, all chosen; lines can be unticked, by hand or by a filter (quick ones: not on this version, ready, has local changes, kind, seat; and the typed filter of §9). A deck can be marked "please don't flash" on the deck, with a line of why: it shows on its HUD and in the conductor, the deck starts unticked, and it is a request, not a lock. **Every flash is confirmed first:** the prompt names how many devices get it, which lose unsaved configuration, which keep local mappings (INPUT.md §6b) and which are left out. For that a deck says about itself: asks not to be flashed, has unsaved configuration, how many local mappings. **Decided 2026-11-10:** once the prompt has warned and the operator accepted, a forced flash forces, the mark and the local changes included. The deck keeps what was replaced as its own copy, its operator gets a notice, and with the right to do so can go back to it or switch between the forced version and their own. Open: what a deck that was not here gets when it joins, and which right allows going back (proposed: the deck's owner always; anyone else unless the flash was locked).
- **A child treats its parent like a reader:** the parent's row arrives as an event and the child's rule applies: follow = auto, suggest = its operator decides, free = ignored. One mechanism for triggers and delegation. The local GO always works.
- Today: decks pair with a key pair and get exactly their owner's rights (`server/src/Domain/Decks/Decks.php`); rigs have `lent / owned / core`; the relay token names session, role, show. No show documents, seats or invitations, and a deck of one member cannot read another's media.
- **A parent may lock another member's child only if the seat accepted it** (owner). Without that, the lock from above does not cross members. Proposal: the invitation says whether the seat can be locked, so accepting the seat is accepting the lock, and every seat shows it.
- **The relation between show documents** (owner, 2026-11-10): **one document per conductor, and the seat is the relation, saved in the parent's document.** A seat holds: its name, the child show and its version, the deck or member sitting there, the word (follow, suggest, free), whether it can be locked, and what the parent uses of the child's face (rows, events, outlets).
  - The child does not name its parent, so it stays reusable. Same direction as a rig: "the children are the rig document's, not the module's" (`rig/face.rs`).
  - The tree is read by following seats from the top. No third document, and a cycle is refused on save.
  - "Used by" is a server question, as for assets; another member sees the seats they accepted.
  - A seat pins a version. A new child version is offered to the parent with what changed in the face, and a row the parent uses that has gone is an error in the editor. Proposal: while rehearsing, a seat of your own can follow the latest draft.
  - A flash resolves the whole tree into one list of exact versions, like a lock file. That is the assignment each deck gets, and what "the show at a pinned version" means for a tree.
- Open: can a seat be handed on? Does an organisation own a show?

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
- Lamps on a device outlet (#91, LIVE.md B.5.2's model, the smallest that runs): the show's `fixtures` (`id`, `outlet`, `address`, `profile` dimmer · rgb · rgbw · rgbw-strobe) and `looks` (`values` by fixture and channel, 0–255, a missing channel is 0); a device cell `{ "mode": "look", "look", "fade" }` sends one DMX range per lit fixture. Venue patches are open: LIVE.md B.5.4 keeps them in the show, the 2026-11-02 `[venue]` decision makes the venue its own document that shows reference.

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
- **Rows are the main way to launch** (owner). Any single cell can also be launched by hand (the clip launcher era, amending 2026-10-07): it is the operator's responsibility, like GO. A cell launched off its row is marked, links it breaks end, and a `values` cell that then meets the wrong piece is skipped and logged. The editor checks rows, not hand launches, so availability stays knowable.
- **Each subtree launches its own rows** (owner): a sub conductor has its own matrix, the parent fires it or leaves it free. The show core is recursive from the start; a show can still begin as a single conductor.

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

- In the document (#91): a row's `addresses: [{ address, action, where }]`, asked down its subtree when the row fires; read by `rig/address.rs` `Target` (kind, an id or `*`, persona and look for characters only; an action is a word or `scope:word`).
- Spelling: dots for persona and look, as rig subjects do; `_` and `-` mean view and mood in asset names. The ladder's `/` and leaf forms are those of `rig/address.rs` (RULES A14).
- A place in front narrows the reach: `/deck:foyer/location:*`.
- Many matches are fine for a target, never for a source.
- A wide address is resolved when a piece loads, into compiled slots; nothing on the audio or frame path parses an address.
- Delivery is push: an event travels down the conductor tree and each receiver matches it against what it plays.

## 9. Filters

`address where filter > action`, for example `person:* where device:browser and inventory:sword >= 1 > haptic:pulse`.

- Reuses the chapter editor's "Only if": `{ variable, op, value }` or an expression.
- Facts are named by where they come from (A14): `group:`, `device:`, `periphery:`, `story:`, `inventory:`, `stat:`. The reader stats today are `stat:testo` and `stat:appetite`; health does not exist yet (owner 2026-11-09).
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

**Timing and scope** (owner, 2026-11-10): building starts 2026-11-16 in lane 2 (research/KANON.md §8, #89–#91), code complete 2027-02-05, workshop 2027-02-17. The four changes above stand. For the workshop:

| | For the workshop | After |
|---|---|---|
| Data model | Show document per conductor with seats, recursive from day one | |
| Conductors | Main conductor and sub conductors on the owner's own decks: own rows, own operator, own clock | Sub conductors of other members |
| Relay | A tree: rows, cells, events and the clock pass down, values, readiness and events up (today it is flat) | |
| Parent and child | The child treats its parent like a reader: follow, suggest, free. Local GO always works | |
| Show and Perform | Both workspaces in deck-editor: matrix, rows, cells as a piece plus values, entering a sub conductor, the 2D overlay | |
| Launching | Row ▶, single cell ▶ by hand, GO, moments | |
| Triggers | Reader events firing rows: auto, suggest, hand; the log | |
| Addressing | Kind and id, `character:yellowmouse > dance:samba`, silent and logged | Down to module and parameter over the wire |
| Filters | Group and device | Inventory, stats, periphery |
| Saving | Show documents with versions; a flash resolves the tree to exact versions, down own decks or on a stick | |
| Other members | Nothing | Invitations, faces, lent media, locks across members |

Largest new piece of work: the relay tree. Everything else in the table reuses what the deck already has or is editor work.

## 11. What exists in the code (read 2026-11-09)

- `deck/show` (#89): the show document, validation, the tree's lock, the bare conductor. A wire cell carries a piece as a layer (`LayerSrc` `piece`). #467: deck-space `piece.rs` compares it with what plays (same piece: the values glide; another: load and crossfade, live-core `lc_crossfade`; not on the deck: a reason, what plays stays); the headless host plays it (scenario 29). #91: the native deck's outlet hands every cell to it (`outlet/src/route.rs` `Routed::Piece`, `native/src/live/piece.rs`): a Load plays the piece's scene beside the outlet (`piece::beside`) through the deck's switch (a fade, not yet a crossfade), glides go into the controls. The browser deck does not call it yet.
- #90: role `reader` (its `story` goes to the conductor only); the relay tree (deck/relay README "The tree"); `Subtree` in deck-show (the word, the lock from above, own clock as `Tempo.domain`, flash by `flash::take`, `address::receive` for a receiver); `show:<id>` with versions and the `live` role (server docs/API.md "Live"). #91: the deck editor runs one `Subtree` (its Show and Perform workspaces, `deck/editor/src/show/`, joined to its relay as conductor; what they draw is deck-show's `view.rs`); a sub conductor's document is shown there, not run. The native deck's flash side is built (live/flash.rs → deck-show `ready.rs`); how a deck answers an `address` (`address::receive` against what it plays) is not wired on the native deck yet. The Show workspace has the cue sheet and the problems list (deck-show `view::cue_sheet`, `view::problems`: `validate`'s errors with their `Spot`, the decks' lacks as warnings).
- `cell` is per outlet and full (`deck/protocol/schema.json`), compiled in `deck/outlet/src/plan.rs`. No layer kind for a scene or rig. The `outlet` node sits inside a scene (`scenes/outlet.toml`).
- Video: a layer exists, the native outlet shows its still (#179). `video` in the catalog is `F_PLANNED`, not a graph kind.
- Poll: `vote.*` messages exist. No poll layer, and no vote handling on the native outlet.
- Light: a plain scene with `[outputs]` (#211). A rig has no outputs.
- A mood turn moves values without a reload; a loaded 3D set stays and only the spot moves. No message sets a named value on a deck from the conductor.
- Far events (#442, `far.rs`): one exact address, subscribed by the asking deck, only while the other deck is in the session.
- `address.rs` has places `deck:`, `scene:`, `character:`, `item:`. No `location:`, persona or look level, or wildcard.
- The browser's conductor page is the vote panel only (86 lines), opened on a deck with F12; the matrix is the deck editor's Perform workspace (#91).
- `story` messages exist on the wire. The reader bus of LIVE.md B.1 is not in `event-processor.ts`, and there is no reader role.
- A scene cannot hold several rigs yet.
- "Only if" exists in the chapter documents; the evaluator is TypeScript (`shared/content/src/expr/evaluate.ts`). The deck has no condition type. RULE A1: one evaluator, in the core, reaching the viewer through wasm.
- `deck/events/src/horizon.rs` walks what a scope can reach, nearest first: the same question as loading ahead.

## 12. Open

1. Multichannel pan (#459), labels (#461) and nesting (#458) touch this; nesting should follow §4.

2. The link to the creator's catalog and what each entity plays (assignments per character, persona, look, location, item): research/CATALOG-LINK.md, #472.

## 13. Drafts

Concept images (sent to the owner in the chat, not in the repository): `n0-tree`, `n1-reach`, `n2-cells`, `n3-launch`, `n4-editor`, `n5-conductor`, `n6-build`, `n7-show`, `n8-perform`, `n9-flow`; current: `m1-show`, `m2-perform`, `m3-tree`. `n1`–`n5` still draw groups and one flat conductor; §4 is the correction.
