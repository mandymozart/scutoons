Status 2026-11-01: built (#236, PR #217): `deck/events` (the contract, the graph, rt events, random_walk / pulse / arp, gate_cv, glide, patch.rs; allocation-free in a run), the stage's `walk` / `events` nodes (one Host block) and live-core's `[[walk]]` / `[[events]]` – a rig's melody and a rigged arp play on the deck and in the browser; encapsulated (§10); the stage's sequencer + dispatcher threads and the routing table (§16, PR #240); horizons + preload (the walker, the rig's mood prebuild) and the references guard (§16, PR #238). In progress in its own session and branch with a PR into `wip/quirky-pascal-qmago4`: adapters story / relay / MIDI (§12, `wip/events-adapters` – acceptance: a chapter's showMessage mood switches catpope's preset). Owner handoff "Generator/Event Architecture"; extends #191 (rigs), #234 (sequences), #235 (patterns / clips).

# EVENTS — generators that talk in events

Owner 2026-11-01: "arpeggiators are just like instruments, but can be rigged … ableton chain. important features. maybe
we are going about this completely wrong. generators are a concept". The handoff, short:
- **Generator** = anything that emits events (random processes, Markov chains, physical models, sonification, wrappers
  around external synths); its behaviour lives only in its implementation.
- **Event** = the shared contract, minimal: `start_time`, `duration`, `name`, a property bag; events may contain events
  (sound → phrase → section → piece). Generators talk only through events, never through each other's internals.
- Requirements: add generators without touching others; a small stable interface; simple ones combine into specific
  ones; pure where possible ("given X it emits Y"). Risks: real-time cost, ordering / feedback, events vs signals,
  routing complexity. Suggested: a UGen runtime for the signal, an event layer on top; generators the one extension
  point. **Decided:** our own implementation (a friend's pre-alpha engine not for now), interface first.

## 1. Wrong? No – two layers that were one
- **The data layer** (#191, research/INSTRUMENTS.md): what is saved, owned, derived – instruments, presets, rigs,
  lineage, factory + members, the carrier bag. It stays as it is.
- **The runtime layer** is what was missing. The node graph has three port types – video, audio, control (one f32 a
  frame) – and notes are smuggled through audio: `seq.cv` = pitch on the left channel, the gate on the right
  (graph.rs), voices.rs gathers pitch + gate lanes from that cv. There is no **event** port.
- So: a fourth port type, `event`. An instrument gets an event input (its notes); a generator is a node with event
  outputs; an arpeggiator is events → events. The Ableton chain – MIDI effects → instrument → audio effects – is one
  track's chain: event → event → audio → audio.

## 2. The event
```rust
pub struct Event { pub start: Time, pub dur: Time, pub name: Sym, pub props: Props, pub children: Vec<Event> }
pub enum Time { Beats(f64), Seconds(f64) }   // beats on the universal tempo map (deck-core tempo.rs), or seconds
```
- `name` interned (`Sym` = u32): `note`, `mood`, `cue`, `flash`, `section` … `props`: a small sorted array of
  (Sym, f32) on the audio thread – `pitch` in Hz (continuous; no 12-tone grid, owner 2026-10-31), `vel`, `timbre` … –
  plus a loose table (strings, vectors) only off the audio thread. MIDI notes are a conversion at the edge.
- **Compound**: children are relative to the parent's start; a section is expanded lazily, window by window.
- **Time**: stored in beats by default, so a phrase follows the tempo; resolved to a frame offset at the audio edge.
  The tempo clock is the universal grid (owner 2026-11-01), never a source.

## 3. The generator
```rust
pub trait Generator: Send {
    fn ports(&self) -> &Ports;                                          // event / control / audio ins and outs
    fn query(&mut self, span: Span, ins: &Inputs, out: &mut Outbox);    // the events starting in [span)
}
```
- **The query model** (TidalCycles / Strudel): a pattern is a function of a time span → events. Pure where possible
  (its state + a seed), so it is testable, cacheable, re-orderable – and **replayable on every deck**: same seed, same
  tempo map → the same notes on every deck without streaming each note. The relay sends seeds, params and cues.
- **One interface; the roles come from the ports:**
  - source (→ events): a sequence, a random walk (the phone page's melody), Markov, a score / clip player (#235),
    MIDI in, the relay, face / motion triggers (#212)
  - transformer (events → events): arpeggiator, quantize, transpose, chord, probability, humanize, echo
  - instrument (events → audio + image): allocates a voice per note
  - bridge (events ↔ signals): gate / cv / envelope out; threshold / onset → events (the feelings' rules are this)
  - sink: MIDI out, OSC, the relay, the being's look
- **Compound generators** = a sub-graph published as one generator (an instrument rig, a "crazy arpeggiator" rig) –
  the handoff's requirement 4, and #191's "instruments rigged together".

## 4. Runtime
- The signal stays UGen-style: deck-core's BatchGraph, voices.rs' lanes, blocks.rs. The event layer runs above it,
  **per block**: before a block every generator is queried for [now, now + block + lead) in topological order; its
  events land in each consumer's sorted queue with a sample offset. Starts are sample-accurate, the cost is block rate.
- The render thread already renders ahead (#208's ring, the adaptive lead): the query window is that lead.
- Real-time safe: fixed-capacity queues, interned names, no allocation in a block; compound expansion and loose props
  on the control thread.
- **Ordering / feedback**: event links order like audio links; a feedback edge is one window late (graph.rs already
  lets control links into audio nodes cross a frame late – the precedent).
- **Events ↔ signals**: only through bridge nodes; nothing converts silently.
- **Inspection**: the being draws the graph (#188 – bubbles in the hoses are events, literally), the patch editor
  (#203) edits it; an event log / timeline for debugging.
- **Browser**: the same Rust in wasm; live-core gets the event layer and an event-fed voice kind, so the page's JS
  melody goes.

## 5. Events that exist already (one format, not one bus)
- The relay's messages (B.6 envelope with `at`), `scene.sync`, cue actions.
- The story: the viewer's EventProcessor (`unlockLocation` …) – generators at the story's time scale.
- The feelings: feedback → a mood (the rules = bridges); a mood change is an event the rig answers (a preset switch).
- MIDI in / clock, Link, OSC; face and motion triggers (#212).
- Scores, MIDI files, Ableton clips (#235) = stored compound events.
- One *format* could serve them all; the buses stay apart (a deck's block-rate bus is local, the relay carries the coarse ones).

## 6. The rig model with events (#191)
- **Instrument**: + an optional event input (`notes`); generator and transformer instruments exist next to sound ones –
  an arpeggiator is an instrument rigged into a chain.
- **Track**: `chain: [device …]` (event devices → the instrument → audio devices) + its **source** (#234: the scene's,
  the character's, the relay, MIDI, or another track's generator).
- **Presets** keep values. Sequences are a generator's content (patterns, #235) – derivatives with lineage like presets.

## 7. Prior art
- **DISCO** (Kaper, Tipei & Wright 2000): an abstract Event, compound events, Generator classes – the closest model.
- **Csound's score**: `i <instr> <start> <dur> <p4 …>` – start, duration, name, properties.
- **SuperCollider Patterns / Events**: `Pbind` → event streams with default keys; `Pseq` / `Ppar` compound.
- **TidalCycles / Strudel**: pattern = query(span) → events, pure and composable – DISCO's generators in real time.
- **Ableton's MIDI effects** in a device chain.
- The signal layer: Lazzarini 2017 (object-oriented UGens in Csound), OOPS (NIME 2017), Pedersini, Sarti & Tubaro 2000
  (objects in a dynamic interaction topology).

## 8. A first spike (proposal, encapsulated – §10)
- A crate `deck/events`, depending only on deck-core's tempo map:
  - `Event`, `Time`, `Sym`, `Span`, the `Generator` trait
  - a scheduler: queries per block, sorted queues with frame offsets, topological order, the feedback delay
  - generators: `random_walk` (the page's melody: spread, rate, glide → notes with continuous pitch), `pulse` (the
    tempo grid), `arp` (events → events)
  - tests: determinism by seed; window independence (querying [0, 2) = [0, 1) + [1, 2)); sample offsets; an arp's output
- **One host node kind** in the stage graph, `generators`: it holds a generator sub-graph and speaks today's ports.
  - Out: `cv` in seq's encoding (pitch left, gate right), so fm / osc / voices.rs play it unchanged; control outputs
    for any event property (a `mood` event → a value, a `flash` → a pulse).
  - In: control inputs as the generators' params (knobs, mappings, presets work as today).
- The page's melody becomes `random_walk` in that node: the rig's voice gets its notes on the deck and in the browser.

## 9. Open – for the owner
1. Stored time: beats by default, seconds for clips and cues?
2. Voices: each instrument allocates its own, or a shared voice-allocator node?
3. Event names: free, or a registry (like the moods)?
4. Determinism across decks: a seed per scene / generator, the relay sending seeds instead of notes – right?
5. One event format for the story, the decks and the relay – or the story keeps its own?

## 10. Encapsulated – grow around, don't rewire (owner 2026-11-01)
Owner: "is there a possibility to encapsulate this approach and not rewire our entire software? … a universal
rhizomatic system design, where stages etc. are also generators". Yes – by **adapters at the boundaries** (the strangler
fig: the new grows around the old; nothing has to move, each piece can move later):
- **Inward – a generator inside today's graph:** the `generators` host node (§8) speaks the ports the graph has (audio
  cv, control). Nothing in graph.rs, BatchGraph, the stage, the rigs or the being changes; a native `event` port type
  can come later, or never.
- **Outward – today's things as generators:** an adapter maps a thing's existing interface to events, and the thing
  stays as it is:
  - **a stage / deck** – its `deck.graph` + `deck.values` + mood out, `scene.sync` / cues / values in: to the
    conductor, every deck is a generator already (#188's mirrored beings are that view)
  - the relay: an envelope `{ kind, at, p }` is an event (`name`, `start`, props) – the bus between stages
  - MIDI in / out, Link, OSC; the feelings (mood changes out); face / motion (#212)
  - later the story's EventProcessor (chapter-scale generators), a score / clip player (#235)
- **The rhizome:** every one of these is a generator through its adapter, and any generator can feed any other through
  events at any time scale – a deck into the conductor, the conductor into a deck, a chapter into a show. No hierarchy
  is built in; a "stage" is just a compound generator whose inside is a node graph.
- **What is new code:** the `deck/events` crate (the contract + the scheduler + the first generators), the host node,
  and one adapter per boundary as it is needed. The contract is the only thing the rest of the system ever learns.

## 11. What it buys the project now
Concrete gains for the open work, without waiting for a rewrite (§10):
- **The rig's silent melody** (#191 `?rig=1`: the voice holds its pitch): `random_walk` in the host node gives it its
  notes on the deck and in the browser from one Rust source; the page's JS melody goes (DECISIONS 2026-10-31: "as
  much as possible in the Rust engine").
- **Multi-deck sync** (the CATH 3-device test, values streamed at ≤ 30 / s): seeded generators play the same notes on
  every deck from a seed + the tempo map; the relay carries seeds, params and cues, not notes; a late joiner gets the
  seed and the bar and is in step.
- **The Live kickoff (2026-11-16) and Top Toy (2027-02-17):** the conductor's cues, cell changes (#186) and the show's
  rows become events on the same timeline as the music – a cue lands on the bar.
- **#234 sequences**: the scene's, the character's, the relay's and MIDI's sequences are four sources of one kind;
  **#235**: scores, MIDI files and Ableton clips become stored compound events with one player.
- **#212 / #211 – everything audiovisual**: shake → flash, smile → a mood, a gamepad button → a mood are events; one
  event drives the sound and the image (the strobe's rate cap is a transformer).
- **#209 MIDI learn, Link**: MIDI in is a source generator, MIDI out a sink – no extra path.
- **#188 feelings**: the rules are bridges (signal → mood events), the rig's mood switch (`moodRig`) answers them –
  the same thing, now named and testable.
- **#210 scene objects**: a bell ringing on the beat = `pulse` → the bell's instrument.
- **#208 dropouts**: events are scheduled per block inside the render lead – no per-sample work, nothing new on the
  audio callback.
- **Tests** (RULES "TypeScript + unit tests"): a generator's contract is a unit test – given X, it emits Y.
- **Items**: an arpeggiator or a generator can be an item in the carrier bag like an instrument (DECISIONS
  2026-11-01), its patterns derivatives with lineage.

## 12. The three formats today (adapter cost)
| | story (`VNEvent`, shared/content visual-novel.ts) | relay (`Envelope`, deck/protocol) | inside a deck |
|---|---|---|---|
| shape | `{ type, params, id?, if? }` + on a slice `trigger { type: intersect·hover·click, repeat?, rect? }` | `{ v, seq, at?, from, to?, kind, p }` | control values `node.port → [f32…]` (live-core wire records `op, mode, node, port, amount, v[]`); MIDI bytes (`DeviceMsg`); the head's mood (a string) |
| name | `type`, camelCase: `showMessage`, `fx`, `unlockLocation` … (18) | `kind`, dotted: `cue`, `fx`, `deck.values`, `scene.sync` … | a path `voice.ratio`; a MIDI status byte |
| time | none – a sequence the reader drives; `delay` / `duration` in ms, `wait` blocks the script | `at` = relay ms (absolute, clock-synced) | audio frames; beats on the tempo map |
| payload | typed params per type (strings, numbers, character refs) | per kind; `cue.actions` / `fx` / `say` loose | floats only |
| semantics | commands that change the save game; ordered; conditions; choices block | messages; some are state (`deck.graph`, `snapshot`), some are moments (`cue`, `fx`, `say`) | signals (continuous), except MIDI notes and mood changes |

Adapters: **story → event** easy (`type` → name, numeric params → props, strings → the loose table, `id` kept, `duration`
→ dur; `if` is checked first and `wait` / choices stay the story runner's – a deck gets a copy, the story keeps its
authority). **relay → event** easy for the moments (`kind` → name, `at` → start through the clock the decks already
sync, `p` → props, `cue.actions` → children); the state kinds stay messages. **MIDI → event** a standard transformer
(note on + off → one note with a duration). **Signals** are not events – bridges (threshold → event, event → value).
The real differences are time (none · absolute ms · frames / beats) and names (camelCase · dotted · paths) – both
settled by the event's `Time` and a name registry.

## 13. One timeline for every medium (owner 2026-11-01: "timekeys … for sequences, video, audio, midi, scores, animations")
An event is a time key: a start, a duration, a name, props, children. Every medium places itself on the same timeline;
the medium's data (samples, frames, curves) stays an asset or a signal, the event says when and how it plays.

| medium | as events |
|---|---|
| MIDI | `note { pitch, vel }`; CC → a `curve` (below); General MIDI's program map → an instrument id |
| scores (#235) | compound: piece → movement → measure → note (MusicXML maps 1:1) |
| audio clips | `clip { asset, offset, gain, warp }` – the audio stays the file, the event places it (an Ableton arrangement clip) |
| video | `video { asset, in, out, rate }` (the relay's media cells already do this per outlet) |
| animation | `curve { target, keys: [time, value, ease] }` – a bridge turns it into a control signal; a mood sprite change = an event |
| story (VN) | a chain = a compound event whose children have an order but no clock times yet (below) |

**Two kinds of time** (owner 2026-11-01: "vn events mostly are there to trigger things … the timeline is there for
timeline sensitive stuff like videos, and audio"): a **trigger** happens now (a story event, a tap, a cue without a
time); a **timed** event sits on the timeline in **beats** (music, on the tempo map) or **seconds** (clips, video). The
story's chains stay the story runner's; what reaches a deck is a trigger. (The `step` time of §14 goes.)

Compared with MIDI: an open vocabulary (a registry) instead of a fixed one, f32 instead of 7-bit, beats / seconds / steps
instead of ticks, nesting, routing by name and receivers instead of 16 channels, and every medium instead of notes only.
MIDI, MusicXML, Ableton and OpenTimelineIO (video / audio edits: clips, tracks, stacks – the closest timeline model)
stay import / export adapters.

## 14. The proposed types (TypeScript + Rust)
```ts
type Time = 'now' | { beats: number } | { seconds: number };   // a trigger · music · clips / video
interface Event {
  start: Time;
  dur?: Time;                        // none = a moment
  name: string;                      // from the registry: 'note', 'fx.flash', 'story.showMessage'
  props?: Record<string, number>;    // fast: pitch, vel, gain … (real-time safe)
  data?: Record<string, unknown>;    // loose: text, ids, refs – never on the audio thread
  children?: Event[];                // relative to `start`
  id?: string;                       // stable id (the story's save game, lineage)
  source?: string;                   // 'story:catpope/2' · 'relay:c7' · 'midi:keys'
}
```
```rust
pub enum Time { Now, Beats(f64), Seconds(f64) }
pub struct Event {
    pub start: Time,
    pub dur: Option<Time>,
    pub name: Sym,                          // interned registry name
    pub props: SmallVec<[(Sym, f32); 6]>,   // fast, real-time safe
    pub data: Option<Box<serde_json::Value>>,
    pub children: Vec<Event>,
    pub id: Option<Sym>,
    pub source: Sym,
}
/// what the audio thread sees: flat, resolved to a frame, no allocation
pub struct RtEvent { pub frame: u32, pub name: Sym, pub props: [(Sym, f32); 6], pub len: u8 }
```
The story's `VNEvent`, the relay's `Envelope` (shared/live protocol.ts, generated) and a deck's wire records stay as
they are; adapters map them in (§12).

## 15. Names, cost, latency, loops
**Names: a dictionary with domains** (owner 2026-11-01: "makes it really easy to know where things go and what they
will trigger"): `story.fx`, `story.showMessage`, `relay.cue`, `midi.note`, `deck.mood`, `media.video` … – the domain
says where it comes from, a receiver subscribes to names or whole domains.

**Cost.** A name is an interned integer, a receiver list an array: dispatching one event is a few hundred nanoseconds.
A block is 256 frames = 5.3 ms at 48 kHz (deck-stage audio/ring.rs); 1,000 events in a block cost well under 1 % of it
– one FM voice costs more. The audio thread gets `RtEvent`s (fixed size, no allocation); loose `data` stays off it.

**Latency** (events add nothing to today's paths):
| path | latency |
|---|---|
| a generator → an instrument on the same deck | 0 – scheduled ahead, sample-accurate |
| a live input (tap, MIDI key, story trigger) → sound | native: the sequencer's horizon (device buffer + lead + a block) + the render lead (5.3–85 ms adaptive, #208); 43–64 ms in the browser (live-core's lead) |
| → the image | the next frame: ≤ 16.7 ms at 60 fps |
| a story trigger → a deck on the same phone page | ≤ one animation frame + the lead |
| → other decks over the relay | the network (LAN ≈ 1–10 ms) + the lead; a cue with `at` fires on every deck together, at the time chosen |

**Where it can get slow:**
- an **event storm**: a generator emitting thousands per block (a curve sent as events instead of one `curve`)
- an **expensive receiver**: an event that reloads a preset's shape or a scene (milliseconds, off the audio thread)
- **huge compound events** (a whole score): expanded lazily, window by window, on the control thread
- **fan-out** across many decks: the relay's rate cap (30 / s for values today)

**Loops – yes, accidentally possible, and bounded by design:**
- a loop without a delay is refused when the graph loads (as graph.rs refuses cycles); a loop needs a delayed edge,
  so it runs at most once per block (≈ 188 / s) – it can't spin faster than the audio
- a loop that multiplies (each pass emits two) hits the **per-edge queue cap** (e.g. 256 a window): the rest is
  dropped, counted and shown in the HUD like a dropout
- every event carries a **hop count** (like IP's TTL): past a limit it is dropped – this also stops loops across decks
  through the relay (deck A → B → A), which are slow but endless otherwise
- **heavy receivers are rate-limited** (a mood already has to hold 1.2 s before the head shows it, being.rs `MOOD_HOLD`)
- the patch editor (#203) shows cycles, delayed edges and fan-out before they play

## 16. Transports, references, priorities, threads, horizons (owner 2026-11-01)
**Timelines are transports** ("time lines can be also async if they get triggered from somewhere"): a timeline (a
clip, a video, a pattern) is a generator with its own clock – `start` · `stop` · `seek { pos }` · `rate` · `loop` –
driven by trigger events from anywhere; it runs free or locked to the tempo map, and a start may be quantized to the
next beat / bar (Ableton's clip launch).

**References, not payloads** ("sending references to the entries or sequences … in the database, … the receiver can
load data and handle caching themselves"): an event carries ids and immutable paths (`pattern:m:01H…`,
`catpope/map/sets/cathedral.low.v2.glb`, `chapter:catpope:2`), never the data. A guard refuses big payloads at the
adapter (e.g. > 1 KB of `data`: "send a reference"); the receiver resolves through its cache (the deck's #195 cache, the
reader's), versioned paths keep the caches correct.

**Events first, playback second** ("the execution of event effects does not slow down our event flow at all"): routing
never waits for an effect. Receivers get events through queues and act on their own threads; an effect whose data
isn't ready by its time follows a policy (late / skip / placeholder) and the dispatcher never blocks. Values may
coalesce (latest wins); events are never reordered.

**Threads** (deck-stage; the building blocks exist – `rtrb` rings, named threads):
| thread | does | talks through |
|---|---|---|
| **dispatcher** (high priority, not the audio thread) | owns the routing table, fans events out | MPSC in from adapters, one SPSC ring (`rtrb`) per consumer out |
| **sequencer** | runs the generators one window ahead of the render thread | writes `RtEvent`s into the render thread's ring |
| **audio-render** (exists, #208, MMCSS Pro Audio / SCHED_FIFO) | only consumes `RtEvent`s at their frame, renders | its SPSC ring – no locks, no allocation, no user generator code |
| **frame** (exists) | visual events per frame (the being, the light) | its SPSC ring |
| **loaders** (a small pool, like today's `set` / `portraits` / `sync`) | resolve references, decode, fill the cache | answer with `ready` events |
| **network** (tokio) | the relay adapter in / out | MPSC to the dispatcher |
- The routing table is immutable and swapped atomically on a patch change (`arc-swap`): no lock on the hot path.
- One timebase across threads: audio frames (deck-stage clock.rs) – events carry it, threads convert at their edge.
- Names are interned at load, never per event.
- Browser: the main thread (UI, the story adapter) → a SharedArrayBuffer ring (live-core's shared transport, COOP /
  COEP on `/live/…`) → the worker (dispatcher + sequencer + engine) → the AudioWorklet copies; fetches in a loader
  worker; space-web draws on the main thread from the deck's state. The worker is already the browser's "sequencer":
  live-core's walks run the patches there (the in-block path), never in the AudioWorklet; the dispatcher there = an
  adapter ring (shared/live-core ring.ts, one writer, one reader) from the page into the worker, drained before each
  render into deck-events' `Table` (route.rs, platform-free) – not built yet.

**Built in deck-stage (#236):** `audio/sequencer.rs` (the hosts' patches one window of 256 frames ahead, a ring of
`Timed` events per host, late windows dropped and counted, a full ring = wait), `audio/dispatch.rs` (a bounded MPSC
`Inlet` → the dispatcher thread → `Table::route` → one SPSC ring per `listen` input; the table swapped through the
queue, so no `arc-swap` dependency; `Counters` in the HUD), `listen` in patches (patch.rs), `inject_at` (graph.rs).
`DECK_EVENTS=block` keeps the patches in the block; the same samples (deck/stage/tests/walk.rs), both allocation-free
(tests/no_alloc.rs). Routing costs ≈ 1.4 µs median from `send` to the receiver's ring. deck/stage/README.md "Audio".

**Event horizons – preparing before the trigger** ("anticipate potential events by running through scoped event
horizons, and preload"):
- **Scope** = what lists its reachable items: a show's cue rows, a rig's mood variants, a set's objects, a chapter's
  triggers and choice branches. Each item is a **reference** (an id, an immutable path, a mood) with a distance – rows
  ahead, seconds ahead, choice depth – and a cost in bytes (0 = cached or free to build).
- **Horizon** = the reachable items within a **bound** (n rows, t seconds, choice depth d, n items, a byte budget),
  **nearest first** (seconds, then rows, then depth; ties in the scope's order), each reference once at its nearest. The
  budget fills as a **prefix**: the first item that doesn't fit ends it, so a preloaded horizon has no holes.
- **Preload or pre-build:** a loader resolves the references into its cache; an engine can also **pre-build state** (the
  next mood's lowered scene, a video decoder rolled to its first frame) and swap it in on the trigger. Before a show: the
  whole horizon, every deck answers ready; during it: a rolling horizon from the current row.
- **Built (deck-events `horizon.rs`, deck-space `rig/horizon.rs`):** the platform-free walker (`walk(reach, bound)`) and
  the rig case – the energy ladder's moods by rungs, the rig's off-ladder moods (sad) one branch away, a mood the rig
  doesn't name shares the defaults' entry; `Bundle::prebuild` plays each reachable mood ahead (equal to a fresh `play`).
  The guard: deck-events `boundary.rs` (`admit`, a loose payload over 1 KB → "send a reference").
- **The protocol needs nothing new yet:** `scene.sync` (#195: `sets`, `characters`) and `media.preload` (#186: `items`)
  are already reference lists, and their arrays are ordered – a conductor can send its horizon nearest first in them; decks
  answer `deck.synced` / `media.ready` as now. Old decks read the same messages (printed / public things never change).
  A rolling horizon is then just a new `scene.sync` per step.
- **Decided (owner 2026-11-01, "i would go with recommendations for now"):** `scene.sync` stays ordered reference lists
  (a `horizon` field with distances only when a deck needs them); a refused payload answers the sender with an `error`
  ("send a reference"), never a silent drop.
- **Defaults – provisional, tuned after real-show tests (owner 2026-11-01):** a show's rolling horizon = the next 8 cue
  rows or 60 s, whichever ends first (`Bound::show`; before doors the whole show, `Bound::all`); a chapter = 2 choices
  deep (`Bound::chapter`); a deck loads at most 256 MB ahead, and past it loads on the trigger (late – the show never
  stops), it does not refuse the cue; a deck pre-builds moods only (the next scene's whole stage graph later, if real
  shows need it). Constants: deck-events `horizon.rs` (`SHOW_ROWS`, `SHOW_SECS`, `CHOICE_DEPTH`, `DECK_BYTES`).
