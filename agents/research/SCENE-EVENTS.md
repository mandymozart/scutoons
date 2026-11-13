Status 2026-11-13: concept reviewed and simplified (owner: "We keep it very simple … I want the fundamentals"); plan in §4, step E1 started (#516). Decisions: decisions/deck.md [scenes] 2026-11-13.

# SCENE-EVENTS — a deck's scene reacts to the story

## 1. The idea (owner 2026-11-13)

- The reader stays as it is. No stage view in the reader.
- A scene lives in the deck: its spots (left, centre, right …, spawn areas) are defined per scene, and characters join it.
  The text on screen is optional and drawn by the deck (later).
- A scene **listens** to the events the stories fire – *"listen to this event when this character is entering, then I'm
  doing this"* – chosen in the deck's scene editor. One event system; a scene's own events can go back to the story (later).
- **A join without an action is just appear.** Leave is disappear. Fundamentals first, complexity later.

The dependency points from the deck to the story: a chapter knows nothing of decks.

## 2. Review: what exists, and the one real gap

| Piece | Where | State |
|---|---|---|
| Story events with stable ids | `VNEvent` (shared/content/src/visual-novel.ts), `id` per event | built |
| A reader's events reach the session | relay role `Reader` sends only `story` messages; the relay hands them to the conductor (deck/relay/src/session.rs, #90, SHOW.md §7) | built |
| Something listens to them | the conductor's show: a **row fires on an event** (auto / suggest / hand) and sends `addresses: [{ address, action, where }]` down to the decks (SHOW.md §7–§8, deck/show subtree.rs `on_story`); "nothing travels unless asked" (owner 2026-11-02 [events]) | built |
| A story event as a deck event | deck/events adapters/story.rs: `story.<type>` with id, source, props, data | built (#236) |
| A scene's graph hears outside events | world node `listen` filter, sources midi / generator / far (deck/space/src/world.rs) | built |
| Spots, placeholders, spawn zones, casting | scene format 2; `scene_resolve::Casting::join` – the first free spot that admits | built (#504) – **only when the scene is resolved, never while it plays** |
| The cast drawn per spot | one look per holder, 3D or a 2D card | built (#492, #495) |

**Findings.**
1. The listener the owner describes exists one level up: a show row is already "when this event, do this". A scene's
   listener must use the same matching (an event by type or id, a character), not a second one (RULES A6).
2. **The gap is the action:** a playing scene's cast cannot change. `Casting` only runs before `resolve`; nothing joins
   a character into a scene that plays, or takes one out. Without that, neither a show row nor a scene listener can make
   a character appear.
3. A reader's events go to the conductor only. A scene on a deck hears them when the conductor's show passes them on, or
   when the deck asks the session for them (the far subscription's way, #442). Which one is step E2's choice.
4. Text layer, mood / focus, entrance animations, the way back: none is needed for the fundamentals.

## 3. The fundamentals

**3.1 Join and leave while a scene plays (E1).** The core takes two words on a playing scene: `join <character>` – the
character appears at the first free spot whose placeholder admits it (today's `Casting` rule; a spawn zone up to its
`max`), its look and rig as a resolve gives them; `leave <character>` – it disappears, the spot is free again. No
animation. Already on stage: nothing. No free spot that admits: nothing, said in the log. The words arrive as an address
action (`scene:stage > join` with the character), so a show row can send them today.

**3.2 A scene's listeners (E2).** The scene document gets listeners: an event (a story event type, optionally an event id
and a character) and what it does (`join`, `leave`). The core matches with the show's matching; the deck receives the
story events of its session (the conductor passes them on, or the deck asks – decided in E2). A story's `joinScene` with
Cat Pope → the scene's listener → she appears.

**3.3 The list of story events (E3).** The server lists every story event (id, type, character, a text excerpt) per toon
and chapter – the flow index extended – so the deck's scene editor can offer them. The editor's panel comes with #244.

## 4. Plan (replaces SCENES.md §9.6 rows 2·4, 2·5 and 2·7)

| # | What | Size | Done when |
|---|---|---|---|
| E1 | **Join and leave while a scene plays:** the words `join` / `leave` on a playing scene, through an address action; appear / disappear at the first free admitting spot; the look and rig added and removed without restarting the scene | M | headless: `scene:stage` plays empty, `join catpope` → she stands at `left` and her rig sounds; `leave` → `left` is empty; a show row sends the same words (#516) |
| | Built (#516): `join:<who>` / `leave:<who>` (rig/address.rs `CastWord`) on deck-space scene_play.rs `Playing`, re-resolved by the one resolve and loaded by live-core `recast` (a 30 ms crossfade, the running values carried; deck-engine has no in-place graph change, so the members' sound restarts under it); headless scenario 38; no native or browser host takes `address` yet | | |
| E2 | **A scene listens:** listeners in the scene document (event, optional id and character → join / leave), the show's matching, the deck receiving its session's story events | M | a reader's `joinScene` Cat Pope in a session → the deck's scene shows her; without a listener nothing happens |
| E3 | **The story events listed:** a server index of every story event, for the scene editor | S–M | the API lists a toon's events with id, type, character, excerpt |
| later | the text layer drawn by the deck · mood and focus · entrance animations · a scene's events back to the story · the scene editor's Events panel (#244) | – | – |

## 5. Answered (owner 2026-11-13)

- A scene listens; a chapter never addresses a deck (Q1).
- Join = appear, leave = disappear; no entrance actions now (Q4).
- Everything else waits until the fundamentals stand.
