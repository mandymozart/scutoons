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
| E2a | **A scene listens, on the deck:** listeners in the scene document (a story event type, optional event id and character → join / leave), matched in the core; a deck asks its session for story events and the relay passes a reader's story events to the decks that asked (besides the conductor); the native deck takes `address` words and story events into its playing scene | M | headless + relay test with a stub reader: `joinScene` Cat Pope → the deck's `scene:stage` shows her; no listener → nothing; the native deck does the same against the stub relay |
| | Built (#517): the document's `listener[]` (`on`, `id?`, `character?`, `do`), checked by the core; deck-space scene_play.rs `Playing::story` matches a story event as the story adapter reads it (its type, id, the `characters` / `character` it names) and makes each admitted character say E1's word through `word` – the conductor's id matching (deck-show `on_event`) is left as it was; `deck.story` (a deck asks, every `LISTEN_MS`, lapses at `LAPSE_MS`) and the relay's copy of a reader's `story` to each asking deck; deck-space relay.rs passes `address` and `story` as `In`, the space queues them; the native deck's live/cast.rs runs a published scene (its cache copy, published.rs `Cast`) and switches to the changed cast through a fade (no in-place recast on native yet); headless scenario 39, relay `a_reader_s_story_reaches_the_decks_that_asked`, native tests/scene_listens.rs | | |
| E2b | **The reader sends its events in a session:** the viewer, joined to a session, publishes the story events it fires (LIVE.md B.1's bus) as `story` messages with type, params and id; normal reading sends nothing | M | a phone reader in a session reaches `joinScene` → the deck shows Cat Pope |
| | Built (#518): a reader page with `?live=<code>` (`?relay=` / `?quic=` as `relayOf` reads them) joins as `reader`, once for the page's life (viewer reader-session.ts, loaded only then by the toon layout); the runner's `fired` (script-runner.ts, every event that passes its `if`, every type) goes through reader-story.ts to deck-space relay.rs `Client::story` (relay-web `story`): the VNEvent as it is, in order, only while the reader drives (#527: nothing held); no list of types in the viewer; relay.json script `reader`, viewer test/reader-session.test.ts (against relay.exe too). Open: the relay's reader rate (5 / s, the audience's) refuses a burst (#519) | | |
| | Built (#521): `GET /api/creator/story-events` (creator role, the flow index's rights; FlowIndex::storyEvents, FlowController) lists per toon and chapter every VN event of the newest chapter document in document order: `id` (null when none – listed, a listener names it by type and character only), `type` (generated server Generated\StoryEventTypes, the same list as deck protocol::story_events), `characters` (showMessage's `character`, joinScene / leaveScene's `characters`), `excerpt` (first text, 80 characters), `group` / `groupIndex`; contract in shared/content/src/flow.ts `StoryEventsIndex`; PHPUnit StoryEventsApiTest; scene documents not walked | | |
| E2c | **The browser deck takes the same:** `address` words and story events into its playing scene | S | the browser deck shows her too |
| | Built (#520): the browser deck's published scene (`?scene=scene:<id>`, now also `/live/deck/<code>?scene=…` or `?live=<relay>`) joins its session on a link of its own – deck-space published.rs `CastLink` (a musician of the decks; the loader's kept copy from fetched.rs `kept` as the `Cast`; `deck.story` only while it listens; `In::Address` / `In::Story` to `Cast::came`), space-web scene.rs `SceneLink` under the page's socket (live-deck published.ts `joinScene`); the changed cast becomes the deck's own scene and is switched to through black – the browser deck plays no sound, so live-core's `recast` is not in its path; test published.rs `a_link_of_its_own_…` (relay-shaped frames, not relay.exe); an outlet deck (`?outlet=`) that plays a published scene joins twice, as its outlet and as a deck | | |
| | Built (#538): a scene a show's cell launches listens like `--scene` / `?scene=` – one answer for the scene whose cast plays, deck-space piece.rs `Side::playing` (the cell's piece while one plays, else the deck's own), and `Side::recast` puts a changed cast beside the outlet as the Load did; native live/piece.rs `listens` / `hear` (cast.rs), the browser deck's outlet joins its `CastLink` too and plays the outlet store's kept copy; tests native `scene_listens.rs` `a_scene_a_cell_launched_…`, published.rs `a_scene_a_cell_launched_…` | | |
| E3 | **The story events listed:** a server index of every story event, for the scene editor | S–M | the API lists a toon's events with id, type, character, excerpt |
| later | the text layer drawn by the deck · mood and focus · entrance animations · a scene's events back to the story · the scene editor's Events panel (#244) | – | – |

## 5. Answered (owner 2026-11-13)

- A scene listens; a chapter never addresses a deck (Q1).
- Join = appear, leave = disappear; no entrance actions now (Q4).
- Everything else waits until the fundamentals stand.

## 6. Draft: the link to a linked reader (#522 – a design, nothing built; owner 2026-11-13)

### 6.1 What exists
- A reader page with `?live=<code>` joins that session as `reader` and sends the story events it fires (#518). Without it: nothing.
- A session is opened by an account with the role `live`: `POST /api/live/sessions` → a code and the conductor's token.
- The conductor is the deck editor (`deck-editor --show … --relay …/live/<code>`) or (until #533 removed it) the viewer's `/live/conductor/<code>` page. Neither shows a link or a QR for anybody today; the relay's presence already knows each client's role.
- The creator draws a QR for the handheld camera's phone (phone-pairing.svelte, `qrcode-generator`); the deck's Rust code draws none.
- The relay takes a reader without a token: whoever has the session code can join as reader.

### 6.2 The link
`<reader page>?live=<code>` – an ordinary reader address (toon, chapter, the place in it) with the session's code. One function in the core
makes it from the viewer's base address, the page and the code, so the editor and the conductor page offer the same text (RULES A6).

### 6.3 Where it is offered (proposal)
| Where | What it shows |
|---|---|
| **Deck editor** (the conductor), a "Session" panel | the session's code and state · a row **Linked reader**: the page it opens (6.4 Q2), **Copy link**, a **QR** for a phone, **Open here** (the system's browser on this machine – the conductor reads and drives itself) · below it the linked readers now (from presence: how many, their names) and the last story event heard (type, character) |
| **Conductor page** (`/live/conductor/<code>`, removed in #533) | the same row and the same list, drawn over the core's state |
| **The reader**, when linked | a small mark "linked to <code>" with a way out (6.4 Q3); nothing else changes |

A listening scene on a deck needs no setting: it asks for the story by itself (#517).

### 6.4 Questions for the owner
- **Q1 Who may be a linked reader?** Today the code alone is enough, and the audience's QR carries the same code – a visitor could add `?live=` and
  move the scene. (a) leave it for now; (b) a reader key in the link that the conductor hands out, checked by the relay (the handheld camera's way, #469). Recommended: (b) before a show with an audience, (a) is enough for the first tests.
  **Answered (owner 2026-11-13): (a) – "i want to keep it accesible for now."** No reader key.
- **Q2 Which page does the link open?** (a) the conductor picks toon and chapter in the row; (b) the show's home toon, first chapter; (c) the link carries no page – the conductor adds `?live=` to whatever they read. Recommended: (a), preset with (b).
  **Thought aloud (owner 2026-11-13): (a) – the starting point is picked in the conductor / editor: a chapter or a label, "just like how you can pick any label in the creator already for jumpTo", and the QR or link adds the live flag.** The link is then `/<toon>/chapter/<n>?label=<label>&live=<code>` – the chapter page takes `?label=` today. The picker's list is the creator's jump places (creator jump-places.svelte.ts, the flow index); the deck editor needs the same list from the API.
  Also said: "the link should start with some sort of question" – read: the reader is asked before it links (with the take-over warning, Q4, the same moment). **Confirmed (owner 2026-11-13): "Yes, ask first."** Nothing is sent before the answer.
- **Q3 Does the reader show that it is linked?** Recommended: yes, the small mark – the person sends what they read to a stage. **Answered (owner 2026-11-13): "Yes show a small linked sign".**
- **Q4 One linked reader or many?** Today every linked reader's events reach the decks that listen. Recommended: leave it, and show the list so the conductor sees who is in.
  **Thought aloud (owner 2026-11-13): one linked reader at a time – several "would be competetiv"; "maybe a small warning when taking over".**
  Read: the newest linked reader drives, the one before stops sending; a small warning at the take-over. **The newcomer sees it (owner 2026-11-13).** Open: whether the relay or the conductor holds "who drives" (a core / relay rule, not the viewer's).
  **Built (#527, step 1 of 4):** the relay holds "who drives" (relay session.rs `driver`) – one message `reader.drive` (protocol ReaderDrive): a reader → relay `{}` claims, the newest claim wins; relay → every reader `{ driver }` on each claim and when the driver leaves (missing: nobody), kept for a resume, so the reader before hears the take-over and a newcomer that it would take over; the welcome's snapshot (a reader's, also on a resume, and the conductor's) carries `driver`, presence the conductor's `readers` (names) and `driver`. Why both: a reader never gets presence (it is the conductor's), and the newcomer's warning needs the state before it claims and every change after. A reader joins without driving: only the driver's `story` goes on, another's reaches nobody. The core: deck-space linked.rs – `reader_link` (base, toon, chapter, label?, code → `/<toon>/chapter/<n>?label=<label>&live=<code>`, escaped; relay-web `reader_link`), `Steer` (relay.rs `Client::drive` / `driving`, `Out::Driving`: the claim goes at the welcome, again after a drop unless another reader took the place; story before the answer, while away or after a take-over is dropped, not held), `Readers` (relay.rs `Client::readers`, relay-web `readers`: the linked readers by name, the driver, the last story heard – type, characters). **Built (#528, step 2 of 4):** the reader page joins without claiming; linked-reader.svelte (loaded by the toon layout only with `?live=`) asks once – with the take-over line when the core's `Driving.other` says another drives – and only the yes calls `drive`; no leaves the session. Then the small sign over the core's `claim` (linking · linked · taken over with **Take it back**, a new claim) and **Leave**; leaving the toon's pages leaves too. The core's `Out::Driving` reaches the page as `{ driving }` through live-deck's `onOther`; texts `linked_reader.*` (de, en). Tests: relay `the_newest_reader_drives_and_the_one_before_reaches_nobody`, deck-space linked.rs, relay.rs script `reader`.

### 6.5 Build steps once answered (not filed)
the core's link function and the linked readers in the conductor's state (S) · the editor's Session panel with the row and a QR (M) · the conductor page's row (S) · the reader's mark (S) · the reader key, if Q1 = (b) (M, relay + API).

### 6.6 Thought aloud: the installation (owner 2026-11-13, not built)
Many decks installed in a room; someone scans a QR, starts reading, and the conductor extends their experience by itself – "good for demo".
What it needs beyond 6.3, as far as it is read today:
- the QR is public: printed, or drawn by a deck / an outlet in the room – so the link's page is fixed once for the installation (Q2 becomes: the installation's page);
- no operator: the show's rows on auto and the scenes' listeners do everything (#517; SHOW.md §7 auto rows);
- several visitors: one linked reader at a time (Q4, owner 2026-11-13) – the next one who scans takes over, with a small warning.

### 6.7 Design (#531): the deck editor's Session panel
- **Model:** deck-ui `session` (plain data + its words: code, state, the place picked, the link, the QR's modules, the linked readers with the driver, the last story heard) – read by a test without a window. deck-ui cannot see deck-space (deck-space depends on it), so deck-space `session_panel.rs` fills it, like pairing.rs fills the menu's `Pair`: it maps `Readers` and calls `reader_link` – the editor's only call (#530 changes that signature: one line).
- **Toons, chapters:** the public library the reader reads (shared/api library.ts): `GET /library/toons` (ids) and `GET /library/toons/<toon>` (`seasons[].chapters[].number`); the native deck's library.rs fetches only a chapter. No draft flag: the server lists the listed chapters itself (Library.php `seasonOverview`, `isListed`), so no client filter, no copy of the viewer's `isListed`. The editor does the HTTP (ureq, its api.rs pattern); reading the answers is deck-space `session_panel.rs`. No new endpoint. Names: the toon's id and the chapter's number, as in the address (no copy of `toonName` / `chapterName`).
- **Jump places:** the editor offers the chapters and a free label field (empty: the chapter's start); the label picker is a ticket. Why: `jumpLabels` is TS in `@scutoons/content`, read by the creator and the viewer without wasm; moving it into the core means wasm in both apps (the viewer is out of this ticket) – bigger, while a Rust copy would be a twin. The ticket: the jump-place rule into the core, TS reads it, then the editor's picker.
- **QR:** no new crate – deck-space already encodes one (pairing.rs `qr`, qrcodegen behind `gpu`, which the editor builds with) into deck-ui's `menu::Qr`; a second encoder would be a twin, and qrcodegen in a checked core crate would grow deck-deps' allow list. deck-kit's QR painter (live `hud::qr`, style-free) moves to the kit's base so the editor's components reach it; the live module re-exports it. The decode test reads the code back with `rqrr` (a dev-dependency, not in the normal tree).
- **The editor:** a **Session** sheet in the Perform workspace, where the show runner works (#533; the stage grows for it, the rail names the linked readers; deck/editor show/session.rs): the code and state, the toon and chapter pick, the label field, the link, Copy link, the QR, **Open here** (the system's opener – periphery), the linked readers and the last event. The readers come from the conductor's relay client (link.rs copies `Client::readers` when it changes). The viewer's base: deck-link `app_url()` (DECK_APP, else scutoons.com).
- **Built (#531, step 4 of 4):** deck-ui `session.rs`, deck-space `session_panel.rs` (tests: the driving reader named, the last event, the link = `reader_link`, the QR read back by rqrr), the editor's Session sheet (`--sheet session`; a check run: `--linked NAME,… --reader-place TOON,CHAPTER[,LABEL]`, `DECK_OFFSCREEN=1` opens the window off every screen for `--frames N --screenshot FILE`), deck-kit `base::draw::qr`.
