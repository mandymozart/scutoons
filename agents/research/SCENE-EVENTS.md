Status 2026-11-13: concept draft for the owner – nothing decided, nothing built. Answers decisions/deck.md [scenes] 2026-11-13 (five lines, "thought aloud").

# SCENE-EVENTS — a deck's scene listens to the story

## 1. The idea (owner 2026-11-13, in short)

- The reader stays as it is. No stage view in the reader.
- A scene lives in the deck: the deck's scene editor places and designs it, with its spots (left, centre, right …,
  spawn areas), and the characters walk in there. The text on screen is optional and drawn by the deck.
- Inside a scene, events drive it. The scene **listens** to the events the stories fire and piggybacks them:
  *"listen to this event when this character is entering, then I'm doing this"*. The deck's scene editor chooses which
  events a scene acts on.
- A scene's own scripting are **internal events**. They are again visual-novel events, so they can go back to the story:
  one event system, back and forth.
- An entrance (walking in) is a join with an action, perhaps an animation.

**The split this gives:** the dependency points from the deck to the story. A chapter knows nothing of decks; a scene
subscribes to what the stories fire. *"Maybe that's even the better split."*

```
 reader (as is)                         deck (one core)
 ┌────────────────────┐   story events   ┌──────────────────────────────────────────┐
 │ chapter runner     │ ───────────────► │ scene                                    │
 │ joinScene, show-   │   (only in a     │  listener: story.joinScene, catpope      │
 │ Message, fx, …     │    session with  │    → join holder `left`, action walk-in  │
 │                    │    decks)        │  listener: story.showMessage             │
 │                    │ ◄─────────────── │    → text layer, mood, focus             │
 └────────────────────┘  internal events │  internal event → back to the story (?)  │
                          sent back (?)   └──────────────────────────────────────────┘
```

## 2. What exists already

| Piece | Where | State |
|---|---|---|
| Story events with stable ids | `VNEvent` in shared/content/src/visual-novel.ts (`id` per event, assigned by the creator) | built |
| A story event as a deck event | deck/events/src/adapters/story.rs: `story.<type>` (`story.joinScene`, `story.showMessage` …), a trigger with `id`, `source`, props and data; control flow (choices, jumps, variables) stays the runner's | built (#236) |
| Story events between decks | relay message kind `story` (deck/protocol/schema.json): a story trigger for the other decks | built (#236) |
| A scene that listens | the world node (deck/space/src/world.rs): a source from outside as a node of the scene's graph with a `listen` name filter, cabled like a module. Sources today: `midi`, `generator`, `far` (another deck / scene / character). "VN state" is named there as a later source | built (#430, #442); the story source is not |
| Spots and casting | scene format 2: holders at spots, spawn zones, placeholders with `fill`; `Casting::join` puts a character in the first free spot that admits it | built (#504, plan 2·0–2·2) |
| Cast drawn per spot | one look per holder; 3D on a 3D set, else a 2D card (#492, #495); pick by click (#496) | built |
| Published scenes on decks | the core's loader, cache, preload (#246, #513, #514) | built |
| The reader's event bus | LIVE.md B.1: every fired event published as `{ seq, t, kind, id, payload }`, forwarded only in a performance; a vote is applied back as the reader's choice event | proposed; no bus in the viewer yet |
| A list of what stories fire | server FlowIndex.php scans every chapter for its ways out (jumps); it does not list events yet | partly |
| The scene editor in the deck | #244 (plan step 3, the deck stage as the editor's stage) | not built |

So the event system the owner names is already one: the story adapter turns story events into the same events the
deck's graph carries. What is missing is the story as a **source a scene listens to**, the **actions** a listener
triggers, the **way back**, and the **editor** that lists the events.

## 3. Proposal

**3.1 Story events reach the deck.** While a reader is in a session with decks (a paired deck, or a performance with the
conductor), its runner publishes what it fires (B.1's bus) as `story` messages; decks turn them into `story.<type>`
events with their `id` and `source` (`story:<toon>/<chapter>`). Normal reading: no session, nothing is sent. Every deck
in the session hears them; nothing happens unless a scene listens.

**3.2 A scene listens.** A world node with `source = "story"` and a filter: the type (`story.joinScene`), and optionally
the event id, the toon / chapter, the character. Its outputs are cabled to actions, as any world node's.

**3.3 Actions in the scene** (a small fixed set first):
- **join** a holder or spawn zone (the given one, or the first free one that admits – today's casting), with an
  **entrance action**: appear (now), walk in (from off-stage to the spot), fade …, each an animation;
- **leave** (an exit action the same way); **mood**; **focus** (the camera turns to a spot);
- **text**: the deck draws the message text as an optional layer of the scene (speaker, text; off by default);
- **set** a rig's value or preset (what `deck.values` does today);
- **fire** an internal event.

**3.4 Internal events and the way back.** A scene's own events are named events in the same system (`scene.<name>`).
A scene can send one back to the story as a story event – applied by the reader's runner as if the reader had fired
it (B.1's vote precedent), so the story's logic stays the toon's own. Which ones may go back is open (§5 Q3).

**3.5 The scene editor in the deck** (part of #244): place and design a scene (set, spots, spawn areas, cast); an
**Events** panel lists the events the stories fire – per toon and chapter, each with its type, character and a text
excerpt, from a server index (FlowIndex extended to events) – and "when … then …" adds a listener with its action.

**3.6 What it replaces.** SCENES.md §9.4 and plan steps 2·4 / 2·5 (withdrawn), #516 (on hold), and my reading that a
chapter author addresses decks from the event. From the stopped #515 branch, two core parts may be reused later: a still
or video backdrop in the deck's renderer, and the loader taking a template with fillers.

## 4. Steps (rough, not filed)

1. The story as a world source (`source = "story"`, filters), fed by the relay's `story` messages – headless scenario:
   `story.joinScene catpope` → she takes `left`.
2. Actions: join / leave with appear and walk-in, mood, focus; the text layer.
3. The reader's bus, forwarding in a session (LIVE.md B.1).
4. The events index on the server (every story event with id, type, character, excerpt) and its list in the deck editor.
5. Internal events and the way back.
6. The scene editor's Events panel (with #244).

## 5. Questions for the owner

1. **The split:** a scene listens to story events (the deck's author picks them in the scene editor); a chapter never
   addresses a deck. *Recommended:* yes – the dependency points from the deck to the story.
2. **When events leave the reader:** only while the reader is in a session with decks (a paired deck, a performance);
   normal reading sends nothing. *Recommended:* yes.
3. **The way back:** which events may a scene fire back into the story – effects (`fx`), items (`giveItem`, `takeItem`),
   places (`unlockLocation`), variables (`setVariable`)? Control flow (jumps, choices) stays the runner's. *Recommended:*
   start with `fx` only; the rest after a use case.
4. **Entrances:** a small fixed set (appear, walk in from left / right, fade) for every character, or animations per
   character from its rig or model? *Recommended:* the fixed set first; per-character animations once a model has them.
5. **The text layer:** the deck draws `showMessage` text when a scene turns it on (off by default). *Recommended:* yes.
6. **Which events a scene lists:** every story of every toon, or only the toons the scene names (`toons`)?
   *Recommended:* the scene's toons, with a switch to show all.
