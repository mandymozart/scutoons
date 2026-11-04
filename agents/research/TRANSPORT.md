Draft for owner review, 2026-11-04 (#359) – the deck's transport: start / stop / continue / position as core state, the nodes that follow it. Built on `feature/359-transport` (§6 says what); the choices marked *owner may change* are the conservative defaults a night worker took.

# Transport (#359)

## 1 · What it is
The transport is the tempo map's `running` + its beat, owned once by deck-core's tempo `Controller` (`deck_core::tempo`). Nothing new
holds a second copy: a source changes the map, the map is published to the engine (seqlock `Shared`), every node reads it per block.
What #359 adds is the node side: the sequencers stop, hold, release and resume through it – before, only the click and a synced `seq`
looked at `running`, and the sequencer thread played the notes it had already computed past a stop.

## 2 · State and events
- **State:** `running`, the beat (→ bar, beat in bar, phase: `Position`), the bpm, and the **transport epoch**: a counter the controller
  bumps on every start, stop, continue or position jump (a jump = the beat moves by more than `JUMP_BEATS` = 0.01 beat between two
  publishes; nudges and tempo changes never jump). The epoch travels with the map in the same seqlock write, so a reader always sees a
  map and the epoch it belongs to.
- **Edges** (deck-core `tempo::Follow`, one implementation for every node): between two blocks a node sees `Stop`, `Start` (running
  again on another beat: bar 1 or a song position) or `Continue` (running again on the beat where it stopped), and the frame it lies on
  (mid-block too: the frames before it play on the previous map).
- **Events** (EVENTS.md name dictionary, domain `clock`): `clock.start`, `clock.stop`, `clock.continue`, each with the prop `beat`
  (the position). A patch (`walk`, `events`) that `listen`s to one gets it injected at the edge's frame, by the engine – not the page.

## 3 · Who follows (`follow_transport`)
- A descriptor field in deck-catalog: `follow_transport: true` = the kind follows by default. On: `seq`, `walk`, `events`, `click`.
  Off (absent): every other kind – oscillators, noise, lfo, effects, analysis keep sounding and moving through a stop (ambient kinds).
- Per node: the scene param `follow_transport = false` (or `true`) overrides the kind's default. A node that does not follow plays on a
  free line at the clock's tempo while the transport stands, and re-joins the transport's line when it runs again.
- Modules: ABI v2's `Ctx` gets the transport and a module declares `needs = ["transport"]` – the module ABI ticket (#339–#346), not here.
- A `seq` with its own `bpm` is not on the clock at all: it runs free, the transport does not reach it (as today).

## 4 · Behaviour
- **Stop = hold + release:** the pattern position stays (the beat stands), every gate a following sequencer holds closes at the stop
  frame (all-notes-off for its voices), notes already computed past the stop are dropped. A stopped sequencer is silent.
- **Start = from a position:** MIDI Start (0xFA) restarts at beat 0 (bar 1); a song position (0xF2) sets where; the show clock's
  anchor carries its beat. The click ticks the downbeat it starts on.
- **Continue = resume where it stopped** (MIDI 0xFB, the deck's start / stop key, OSC `/tempo/start`, an anchor that runs again on the
  same beat).
- **Count-in / quantised start:** not built. *Owner may change:* the default is an immediate start; a later option starts on the next
  bar of a running show clock (a follower joining) and a count-in of one bar for the player's own start.

## 5 · Sources and sinks (all exist in the core already; nothing added in TS)
- **In:** MIDI clock in (Start / Stop / Continue / SPP – deck-core `ClockIn`), the show clock (the leader's anchor `running`: the
  conductor turning a show on is ONE source of the transport, not the only one), the deck's key (`Key::StartStop`: Space on the native
  deck; the browser deck's key path `clockKey('startstop')`), OSC `/tempo/start` `/tempo/stop` (OSC as the source).
- **Out:** MIDI clock out (`ClockOut`: 0xFA / 0xFB / 0xFC + song position), the show clock's anchor (a leader's `running`).
- **No new UI** (owner 2026-11-04: "the browser panel should not have a start stop button … all core functions are within the deck
  core"): no button in the browser panel; a control surface reaches the transport through the core's key / MIDI / OSC paths.
- **A scene's autostart:** a scene loads running (the map starts running, as today). *Owner may change:* a `tempo` param
  `autostart = false` (load stopped, wait for a start) is the natural next step – not built.
- **Clock domains (#358):** the anchor reserves `domain`; a transport per domain comes with the domain ticket, not here.

## 6 · Built (feature/359-transport)
- deck-core `tempo`: the epoch in `Shared` and the controller; `Follow` (edges, the free line for non-followers, mid-block split).
- deck-engine: `seq`, `click`, `walk`, `events` follow; the sequencer thread re-runs its windows from the render on a new epoch; the
  block drops notes of an old epoch and releases its gate on a stop; `clock.*` events into listening patches. No allocation in a block.
- deck-catalog: `follow_transport` (kinds.json, the TS types, NodeKinds.php regenerated).
- Headless scenarios: start / stop / continue, SPP, MIDI-in transport, a stopped sequencer silent, a non-follower sounding on;
  native ≡ wasm32 ≡ simd128 through the headless conformance (tests/wasm.rs).

## 7 · Hardware test (the A4, later, owner)
In the MIDI test rig's checklist (shared notes): A4 Play → the X1's sequencer starts on bar 1; A4 Stop → silent, no hanging note;
A4 Continue → resumes on the step; the deck's Space with MIDI clock out → the A4 stops / continues.
