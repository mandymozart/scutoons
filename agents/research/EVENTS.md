Status 2026-11-01: concept for the owner (§9 questions) – nothing built; encapsulated, not a rewrite (§10). Owner handoff "Generator/Event Architecture"; extends #191 (rigs) and #234 (sequences), #235 (patterns / clips).

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

