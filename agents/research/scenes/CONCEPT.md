Status 2026-11-20: concept for the owner's review (#703, epic #626), nothing built and nothing to build until the owner says the concept is clear. §1 records the owner's idea as thought aloud. §2 is read from the code (main @1d064d30, plus develop for #642 / #686 and feature/698 for the Library). §3 to §7 are proposals. §8 holds the questions for the owner.

# SCENES / CONCEPT — one sequence model for reader, storyboard, XR and deck

## 1. The idea (owner 2026-11-20, thought aloud, not a decision)

> "XrScenes and Groups/Scenes in the webtoon use a very similar concept, and basically eventually could also all be chained. So the type of scene seems like something we can generalize. So do deck scenes. It really just depends on the deck or reader or XR viewer to interpret them and know when to flip to the next, and how to preserve resources. And I suppose if we think hard enough, between live and reader there isn't much of a conceptual difference at some point. … I noticed groups in the chapter tree look almost like the scenes filmstrip in the storyboard, or the OSCT filmstrip for the scenes."

The owner has said related things before:
- 2026-11-11 `[scenes]`: specialised scenes attached to groups in the reader, or to cells in show rows.
- 2026-11-13 `[scenes]`: the reader acts like a driver. Its events, from its continuous scroll or anything it triggers, drive a deck's scene. The reader itself stays as it is.
- 2026-11-20 `[deck-os]`: being a conductor may in the end be only a flag on a deck.
- 2026-11-21 `[editor-style]`: Edit / Play applies only in the chapter, with the reader as the driver, and in the storyboard.

## 2. What exists today

| Piece | Where | The ordered unit | What a unit holds | Who flips to the next | Preload / free |
|---|---|---|---|---|---|
| **Chapter** (reader) | `shared/content/src/content.ts:500` `Group`, `:528` `Chapter.groups[]`; creator `components/chapter/chapter-tree.svelte`; viewer `routes/(toons)/[toonId]/chapter/[chapterNumber]/+page.svelte` | **group**: no id; the array index or an optional `label` addresses it | `slices[]` (image, video, `audio` per slice, dialog, items, VN `events`), `renderer` (vertical / strip / endless), `continues` (flows into the next on the same page, a **chain**, chains.ts:47), `end` (a jump, optionally `auto`) | **The reader's scroll** inside a chain. Between chains and chapters, `nextStep()` (shared/content/src/progress.ts:185) picks the authored `end`, else the next labelled group, else the next chapter in the season. next-bar.svelte shows it, with a 10 s countdown to the next chapter | `viewer/src/lib/lookahead.ts`: N screens ahead (data-budget tiers 0 / 1.5 / 4 / 10), within the current chain only; nothing prefetches the next chain or chapter. Freeing happens only per chapter (Howler unload) |
| **Storyboard** (creator) | `shared/content/src/storyboard/types.ts:219` `ChapterStoryboard`, entity `storyboard:<toon>:<chapter>`; creator `components/storyboard/*` | **script scene** (`StoryboardDoc`) › **storyboard group** (mirrors `Group`) › **shot** (`sb-<scene>-<n>`, stable id) | shots: camera, figures, takes, dialog | nobody: it is authoring. The timeline (`storyboard-timeline.svelte`, `filmstrip()` in `lib/editor/storyboard-timeline.ts:235`) plays picked shots back to back | — |
| Storyboard → chapter | `scripts/storyboard/compile.mjs` (offline, saved through MCP `save_draft`) | one storyboard group becomes one chapter group, one shot becomes one slice (`slice.shot`) | | | |
| **XR** (#482, docs only) | research/XR.md §5, §8; worktree `wip/482-xr-experiences-7f10f7` has no code | **Experience** (Book · Concert reading · Exhibition · Poster) › surface › entry › target › **scene** | an entity tree anchored on an image target, behaviour rules (found / lost / tap → play / show / emit …) | **A found image target** (random access). Story jumps to and from targets are proposed (§8.2) | §8.3, proposed: the experience document first, the engine on "Start the camera", a scene's files when its target is found |
| **OSCT** (comparison) | `osct/editor/src/lib/ScenesPanel.svelte`, `SceneCards.svelte`, `scene-strip.ts`; `engine/src/utils/prepared-scenes.ts`; `client/src/services/PreloaderService.ts` | Experience › `scenes[]` (order = position) + `groups[]` (a scene names its group; **a group = one stop of the reader's switcher and meant as one loading window**) | surfaces, targets, entity tree, passes | a tracked target (`onSceneSeen` → `switchScene`), the bottom switcher, links `/scene/<id>`, history; scene-to-scene flows planned, not built | **The prepared window**: by default the 2 scenes with targets on each side, nearest first, cancellable. `AssetCache.release(keep)` frees what the next scene does not share |
| **Deck scene** | `deck/space/src/scene_document.rs:48` `SceneDocument` (format 2), `scene_resolve.rs:154` | a scene is **content, not a step**: one AV graph played as a whole, with no "next" | location, holders (cast, spawn zones), listeners (#517), stage, nodes, links, camera | a show's cell (`piece.rs`: the same piece glides, another piece loads and crossfades, `transition_secs`); a listener changes the cast (`join` / `leave`, #516 / #517) | `push.rs` `preload` (the conductor's `scene.sync` fills the cache, "nothing plays"); `published.rs` caches per version. No explicit unload was found: the old world is dropped when the switch rebuilds |
| **Show** | `deck/show/src/document.rs` `Show` › `rows[]` › `cells{outlet → Cell}`; `conductor.rs` | **row** (id) | one cell per outlet: Play (a piece `scene:<id>` + values) · Values · Keep · Mirror · Black · Look · Ask | `Row.fire`: a story event id (Auto after `hold_ms`, or Suggest), at a moment (Now / Bar / Phrase); **GO** = the held suggestion, else the next row; a hand fire is always allowed; a vote's result becomes an event | the flash checks every piece (`ready.rs`). #686 `alone.rs`: `needs()` lists every piece of the outlet in row order and fetches **all** of them before the first row |
| Deck Play (#642 / #686) | develop `deck/ui/src/start.rs` | scenes and shows of the sandbox, then the others | | the deck as its own conductor, GO → next row, THE END | as above |

What is already shared, and where it lives: the reader drives a deck today. A linked reader (`?live=`, #518 / #528) sends its story events, which fire rows (`on_event`) and make a scene's listeners act (#517). The Library (#698) shows the pattern this concept would follow: the rule lives in the core (`deck/catalog/src/library`), the creator calls it through wasm, the deck editor draws it with deck-kit, and the TS twin is deleted.

## 3. The common core

Under the four names (group, storyboard group, XR / OSCT scene, show row) sits one structure. A deck scene is not one of them: it is what a step plays on a stage.

- **Sequence**: an ordered list of **steps**, addressable, owned by one document. A chapter, a show, an experience and a storyboard are sequences.
- **Step**: an id (stable, never reused) plus an optional label, and one **lane** per output that says what plays there:
  - `page`: slices, as the reader draws them;
  - `stage:<outlet>`: a piece, i.e. a deck scene, rig or module with values (today's cell);
  - `target:<id>`: an XR scene;
  - `device` / `look`: light.

  A step also says how it is **entered** (`fire`: an event, a found target, reaching it by scroll, GO, a moment) and how it **ends** (`end`: next, a jump, a hold, auto).
- **Window**: contiguous steps that share one page and one loading unit. The chapter's chain (`continues`) and OSCT's group are both windows. A show's row is a window of one.
- **Cursor**: where a driver stands, i.e. the current step (or window) plus a position inside it.
- **Driver**: anything that moves the cursor. The **player** draws only its own lane.

  | Driver | Moves the cursor by | Kind |
  |---|---|---|
  | reader scroll | position, continuous; reaching a step | linear |
  | story event (reader, VN script, vote) | an event id → the step whose `fire` matches | jump |
  | conductor | GO (next / suggestion), hand fire, auto after hold, moment | linear + jump |
  | XR tracker | found target → its step | random access |
  | timer / `end.auto` | the end of a step | linear |

- **Next**: one rule, `next(sequence, cursor) → candidates`. It returns the authored `end`, else the next step in order, else the sequence's own `end`, which may be another sequence's step. This is `nextStep()`, GO and OSCT's planned flows in one place.
- **Prepare and release**: `window(sequence, cursor, budget) → {prepare, keep, release}`, nearest first and cancellable. This one rule covers the reader's lookahead (screens), OSCT's prepared window (±N steps), `alone.rs` (today: all of them) and `scene.sync` (the conductor's push). The budget is the player's: data tier, GPU memory, the deck's quality governor (A10).
- **Live and reading are two drivers on the same sequence.** A chapter read alone is a sequence with the reader as its only driver and the page as its only lane. Add a `stage` lane to a group and connect a deck, and the same reading drives the deck: that is today's linked reader, without a hand-built show. A show is a sequence whose driver is a conductor and whose lanes are stage outlets. Let a reader drive it, and it is the first use case of SHOW.md §2.
- **Chaining across kinds**: every `end` and jump names a step address: `chapter:<n>.<step>`, `show:<id>.<row>`, `book:<id>.<target>` (XR.md §5.5). `JumpToTarget` already has chapter, season, toon, product and xr. The general form is "a step of any sequence".

## 4. Where it breaks (the real differences)

1. **Linear, continuous vs discrete vs random access.** The reader is a continuous position over slices, with several groups visible in one chain. A show fires discrete rows on a clock (bar, phrase). XR has no order at all: whichever target the camera sees wins, and order only hints at what to preload. So "next" cannot be the model's centre. The cursor plus jumps is the centre, and order is the default and the preload hint.
2. **The lanes share no renderer.** Slices in the DOM, an AV graph in deck-engine and a three.js entity tree have nothing to draw in common. Only the sequence, the cursor, next and the window are shared. This is the A2 split: the players are UI or adapters, the sequence rules are the core's.
3. **Time.** A show has a clock and moments, while the reader and XR have none. A step's `fire.moment` exists only where a clock drives. Under the reader driver, Bar and Phrase mean "now".
4. **Authority and many decks.** Live has a conductor tree, seats, suggestions vs autopilot and "GO is always allowed". Reading has one person. Suggest and hold are needed because a reader can scroll by accident (SHOW §7). That stays a driver's policy, not the sequence's.
5. **The reader runs no Rust today.** `nextStep()` and the lookahead are TypeScript in `shared/content` / `viewer`. Moving next and window into the core means either wasm in the reader (bundle and first paint: the reader's speed is a product rule, LOADING.md) or A6's contract by **shared test vectors** (#474): the core is the one definition, and the reader's TS is checked against its vectors. That is a twin by another name, and A6 allows it only as one of the four mechanisms. **The owner decides (§8 Q3).**
6. **Storage differs, and groups have no ids.** A chapter is one document with id-less groups. Shows are their own documents with row ids. An XR scene sits inside its experience document. Storyboard groups are compiled offline into chapter groups. A step needs a stable id to be addressed or fired, so chapter groups need ids: an SQL migration (E2) that keeps `label` as the jump name.
7. **Words.** `scene:` is the deck scene only (SCENES.md §8.5, proposed; XR.md and `address-book.ts:3` follow it). The owner's word for the general thing is "scene". If the generalised unit were called scene, `scene:` would mean two kinds (A14). This doc therefore says **sequence / step / lane / window**: a group, a row and an XR scene are steps, and a deck scene is what a stage lane plays. **The owner picks the words (§8 Q1).**
8. **XR is a second engine by decision** (2026-11-10 [xr]: three.js accepted, browser only, not on decks). Its player stays TS, but OSCT's `prepared-scenes.ts` is a sequence rule. Copying it as it is would be a TS twin of the window rule. It should come from the core when XR is built (§8 Q5).

## 5. What it would look like in the core (proposal)

- A small sans-IO crate `deck/sequence` (or a module in deck-core). It holds `Sequence`, `Step`, `Lane`, `Window`, `Cursor`, `Driver` input, `next()` and `window()`, takes no documents directly, and is shaped by adapters per kind: chapter → sequence, show → sequence, experience → sequence. It is the same shape as `scene_resolve` (pure, the caller hands in what it needs).
- The show conductor keeps all its features: rows, cells, fire, suggestions, votes, seats, subtrees. Only its ordering, GO-next and preload set come from the crate. Things only grow.
- The deck editor's Show band, the creator's chapter tree, the storyboard's group strip and (later) the XR scene strip draw one **strip**: steps as cards, windows as bands over them (OSCT's look), and the cursor. Its rules, i.e. what a card shows, what a band is, reorder and group / ungroup, come from the core, as the Library does (#698). This is the owner's observation about the filmstrips. Each app draws it its own way: Svelte in the creator, deck-kit in the editor.

## 6. Plan in steps

Each step is small, keeps every existing feature, and waits for the step before. UI steps need a draft first (G1).

| # | Step | Size | Done when |
|---|---|---|---|
| S0 | **Words and the answers** (§8): record them in decisions, the RULES A14 word line, SCENES.md §8.5 | S | the owner answered Q1–Q6 |
| S1 | **Chapter groups get stable ids** (migration, `label` stays the jump name); storyboard groups carry theirs through `compile` | M | every group in the DB has an id; `chapter:<n>.<id>` resolves; the reader's `?label=` unchanged |
| S2 | **The sequence core**: `deck/sequence` with Sequence / Step / Lane / Window / Cursor, `next()`, `window()`; test vectors as JSON; adapters chapter → sequence and show → sequence | M | core tests; vectors cover chains, `end`, `auto`, the season's next chapter, GO, a fired row, a found target |
| S3 | **The show runs on it**: conductor GO-next and `alone.rs`'s fetch use `next()` / `window()`; `alone` prefetches ±N rows and releases the rest instead of all | M | headless scenario: a 12-row show alone keeps ≤ N+1 rows' pieces cached; every show test is still green |
| S4 | **The reader's next and lookahead from the core** (by Q3: wasm or test vectors); the lookahead crosses into the next chain and the next chapter's first screen | M | `nextStep()` has one definition; the reader prefetches the next chapter's first slices at idle on generous / normal |
| S5 | **A stage lane on a chapter group** (the owner's 2026-11-11 "scenes attached to groups in the reader"): a group may name a `scene:<id>` (+ fillers, `Use`) for a stage; a linked reader's cursor plays it on the session's decks with no hand-built show; the reader is unchanged | M | a phone reader on `?live=` scrolls into group 3 → the deck crossfades to its scene; a show row may still override it |
| S6 | **The strip, drafted** (G1): one strip for the chapter tree, storyboard groups, the Show band, the OSCT-like XR strip; plain-JS draft in `agents/research/scenes/` | S | the owner OKs the draft |
| S7 | **The strip's rules in the core, drawn twice** (creator Svelte + deck-kit), as #698 did for the Library | M | the creator's chapter tree and the deck editor's rows band use the core's strip rules; no TS twin |
| S8 | **XR on the sequence** (with #482's build): an experience adapts to a sequence, a found target is a jump, OSCT's prepared window becomes `window()` | M | XR's preload comes from the core; no copy of `prepared-scenes.ts` |
| S9 | **Cross-kind chaining**: `end` / jumps take any step address (`show:`, `book:` …); a chapter's end may open an experience's target or a show row's event | S–M | the reader follows `end → book:<id>.<target>`; an XR scene's "back to the chapter" works |

## 7. Draft follow-up tickets (unfiled, for the owner to approve)

- **S0** · `decide` · "Words for the general sequence: step / lane / window, or scene?" (area:deck, S)
- **S1** · feature · "Chapter groups get stable ids (migration), `chapter:<n>.<id>` in the address book" (area:content, M) · chore · "compile.mjs carries storyboard group ids into chapter groups" (area:creator, S)
- **S2** · feature · "deck/sequence: the sequence core with next() and window(), test vectors" (area:deck, M) · chore · "adapters chapter → sequence and show → sequence" (area:deck, S)
- **S3** · feature · "Show alone prefetches a window of rows, not all" (area:deck, M) · chore · "Conductor GO-next through the sequence core" (area:deck, S)
- **S4** · `decide` · "The reader and the core: wasm vs test vectors for next / lookahead" (area:viewer, S) · feature · "Lookahead across chains and into the next chapter" (area:viewer, M)
- **S5** · feature · "A stage lane on a chapter group: a linked reader plays the group's scene on the session's decks" (area:deck, M) · feature · "Chapter editor: a Scene field on a group" (area:creator, S, after the S6 draft)
- **S6** · feature · "Draft: one strip for chapter tree, storyboard, Show band and XR" (area:creator, S)
- **S7** · feature · "Strip rules in the core, drawn by creator and deck-kit" (area:deck, M)
- **S8** · feature · "XR experience as a sequence; preload window from the core" (area:xr, M; with #482)
- **S9** · feature · "End and jumps take any step address" (area:content, S–M)

## 8. Questions for the owner

1. **Words.** Should the general unit be called **step** (in a **sequence**, grouped into **windows**, with one **lane** per output), keeping `scene:` for the deck scene only? Or should "scene" become the general word and the deck scene get a new one? *Recommended:* step / sequence. The address grammar stays one meaning per word (A14), and nothing stored is renamed.
2. **A chapter group is a step with lanes, and so is a show row.** Is that the picture? (A chapter read alone = a sequence driven by the reader. A show = the same driven by a conductor.) *Recommended:* yes. The show keeps everything it has.
3. **The reader and the core.** Should the reader load the sequence core as wasm, or keep its TS `nextStep()` / lookahead checked against the core's shared test vectors (A6 contract)? *Recommended:* test vectors first (the reader's first paint stays light), and wasm once the reader takes the core for something heavier.
4. **A stage lane on a chapter group (S5).** Should a group be able to name a deck scene that a linked reader's decks play, without a show? This is the 2026-11-11 idea, now with the reader unchanged (2026-11-13). *Recommended:* yes, with a show row still able to override it.
5. **XR's preload from the core.** When XR is built, should OSCT's prepared window be taken from the core rather than copied? *Recommended:* yes. A copy of `prepared-scenes.ts` would be a TS twin.
6. **Windows as loading units everywhere.** Should a chain (chapter), an OSCT group (XR) and a row (show) all be "what is loaded together"? *Recommended:* yes. The budget stays the player's: data tier, GPU, governor.
