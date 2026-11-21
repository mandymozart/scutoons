Status: research 2026-11-21, plan for #713 – not built

# A foundation of rudimentary real-time audio nodes – and the three-crows rave demo

Owner 2026-11-21 (#713): *"create a foundation for rudimentary nodes which will be useful for real time audio and basic effects like
delays filters reverbs impulse response and sequencers that can be used to create synths that work for drums and percussions ... I will
use them as a starting point for linking up visuals and light."* The demo's words are quoted in §4.

What this note does: §1 what the deck already has and what is missing, §2 the node set and its contracts, §3 the hooks to visuals
and light, §4 the demo exactly, §5 the tickets in order, §6 questions for the owner. Read against RULES A1–A10 (one core, modules
in Rust, native ≡ wasm, no allocation on the audio path, no second copy).

## 1. What exists, what is missing

### 1.1 Built (develop, 2026-11-21)

| Area | Where | What it gives the foundation |
|---|---|---|
| The engine | `deck/engine/src/lib.rs` | One audio graph, blocks of ≤ `MAX_BLOCK` = 1024 frames, nodes split into dependency levels, every buffer allocated at build; `tests/no_alloc.rs` proves the render allocates nothing. |
| Built-in audio kinds | `deck/space/src/graph.rs` (`Kind`), `deck/engine/src/blocks.rs`, `voices.rs` | `osc` (sine / saw / square / triangle, cv in), `noise` (white / pink + lowpass), `fm` (2-op), `seq` (pitch + gate as cv), `walk`, `events`, `mixer` (4 stereo in, a gain each), `fft`, `click`, `chapter_music`, `module:<name>`. |
| Routing | `deck/engine/src/route.rs`, `deck/core/src/solo.rs` | `bus` (sum, fader, mute / solo, sends from `pre` / `out`, folder buses), `pan` (equal power, VBAP, DBAP), `downmix`, `out`, `feedback` (one block late). One solo rule (Reaper's) for buses and the rig's mixer strips (#683). |
| DSP units in the SDK (#736) | `deck/module/src/dsp/` | One copy for modules and the engine: PolyBLEP and wavetable oscillators, seeded white + pink noise, the SVF (four modes; the lowpass per sample), ADSR, one-pole, follower, delay line, allpass, comb, FFT; tested against analytic targets; same bits native and wasm. |
| Limiter, meters, analysis | `deck/core/src/limiter.rs`, `meters.rs`, `analysis.rs` | Look-ahead soft limiter last before the device (1.3 ms); one meter model; `Analyzer`: level + bass / mid / high in 0–1, aligned to the heard moment. |
| Control half | `deck/engine/src/control.rs` | Stepped once per 128-frame quantum inside the render (~375 / s at 48 kHz): `lfo`, `smooth`, `mul`, `add`, control outputs of modules (one block late). |
| Events | `deck/events/src/` (`generators/arp.rs`, `pulse.rs`, `random_walk.rs`, `adapters/midi.rs`, `route.rs`, `tuning.rs`) | Timed events, the dispatcher (lock-free, drop-and-count), an **arpeggiator generator** (root × chord ratios, up / down / up-down, steps a beat), the MIDI adapter (`midi.note`, `midi.cc` with `ch`). |
| Module event host | `deck/engine/src/evhost.rs` | `music.note` / `midi.note` → `NoteOn` / `NoteOff` at their frame, id-keyed, the tuning fills Hz ↔ key; module-to-module event cables (#430); MIDI enters through a `world` node (`deck/space/src/world.rs`, `source = "midi"`, `listen = ["midi.note"]`). |
| Tempo clock | `deck/core/src/tempo.rs`, `tempo/` | One tempo map (beat = f(frame)); sources internal / tap / MIDI clock in / Link / OSC / the relay's show clock (#321); MIDI clock out; transport follow. Modules read it through `Ctx` (ABI-V2 §1, "Tempo"). |
| Module SDK, ABI v2 | `deck/module/` (README), research/ABI-V2.md | Audio / control / event ports, params with unit, curve, sections; `QUANTUM` = 128 frames a call; asset slots (`AssetKind::Pcm` – decoded samples), `latency` in the descriptor (for compensation, ABI-V2 §1 table), deterministic `math` (libm), `Smooth`, the counting allocator test rig. |
| Factory modules | `deck/modules/analog`, `stepseq`, `walk`, `bob`, `dolly`, `fixture`, `env`, `vca`, `filter`, `noise`, `osc`, `mixer`, `transpose` | **analog** (#331, #350): 2 osc + sub + noise → 2 SVF (LP / BP / HP / notch, drive) → 2 amps, 2 filter + 2 amp ADSR (log time knobs, migration 2026-11-08-analog-envelope-curves), 2 LFO (tempo sync), hard sync, 8 voices, oldest stolen, 91 params in Analog Four pages (migration 2026-11-19-analog-sections). **stepseq**: 16 steps, one pitch lane, `rate` steps a beat, gate, `music.note` out. **env, vca, filter, noise, osc, mixer, transpose** (#737, step 4 of §5): the small nodes of §2.3, one module each on the SDK's units, in the catalog by migration 2026-11-21-factory-audio-modules; their ports and params as built: deck/module/README.md "The small audio modules". |
| Modules as plugins | research/MODULE-EXTENSIONS.md, #590 | The slot table (character: rig = audio + control, motion, picture); motion built (`dolly`, `bob`); picture / object / device codec declared. |
| Rigs | `deck/space/src/rig/` (instrument, flatten, wiring, rig_preset, template), research/INSTRUMENTS.md, RIG-EDITING.md | A rig = tracks (instrument + mixer strip) + silent tracks (a note source, no strip) wired by event cables; flattened into one playable scene; rig presets `rig-preset:<id>` (#677, migration 2026-11-20-rig-presets); a policy module drives `<track>:@gain`, `@preset`, `<track>:<node>.<port>`. |
| Outputs to light | `deck/outputs/` (`shape`, `fixture`, `strobe`, `config`), `deck/adapters/dmx-usbpro`, `adapters/midi` | Every parameter is also an output: DMX (fixture profiles, 8 / 16 bit, curves, smoothing), MIDI out, a clock-synced strobe. |
| Preview, ring, node view | `deck/engine/src/sound.rs` (#650), `deck/native/src/audio/ring.rs`, #591 | A sound asset's preview voice; the render thread's ring to the device; the show's node view with typed wires (decisions/deck.md 2026-11-16 [design]: event triangle, value dot, clock red …). |
| Checks | `deck/headless/` (scenarios), `deck/conformance/` (native ≡ wasm golden renders), `deck/module-bench/` | Where every new node proves itself. |

OSCT (C:\Development\osct\osct) has no audio nodes to look at (only a logic registry); nothing to take from it here.

### 1.2 Missing – what the foundation and the demo need

1. **Live MIDI notes.** The MIDI adapter emits a note **when it ends** (`deck/events/src/adapters/midi.rs` lines 1–6: "born when it ends") – fine for
   recording, wrong for playing: a held key sounds on release, and an arpeggiator never knows what is held. Needed: note on at key-down,
   note off at key-up (the `NoteOn` / `NoteOff` the module ABI already has, `deck/module/src/ev.rs`).
2. **MIDI port and channel on the world node.** `world` has `source` and `listen` only; ports are opened by name substring
   (`adapters/midi`), the per-device input mapping is open (#468). "MIDI port 1" and a channel cannot be said yet.
3. **No delay, no reverb, no convolution, no chorus** anywhere (the master FX of `deck/space/src/controls.rs` – `master.delay`, `master.reverb` –
   are labels with no DSP; the stage environment `deck/engine/src/config.rs` `StageEnvironment` is a pass-through, #330).
4. **No standalone envelope, VCA or envelope follower**: ADSRs live inside analog and inside the engine's `osc` / `fm`.
5. **Two copies of the DSP** (done, #736: both moved into `deck_module::dsp`): analog carried its own `svf.rs`, `adsr.rs`, `osc.rs`, `math.rs`, the core had `dsp.rs` (SVF, oscillators).
   A module compiles as flat `src/*.rs` against the SDK only, so it cannot use deck-core. A6 says one copy: the units belong in the SDK.
6. **The arpeggiator** is a deck-events generator (root × ratios), not a module; it does not play held notes, has no gate, no
   transpose, no note-value rate.
7. **No drum sequencer**: stepseq is one pitch lane; no lanes, no accent, no swing.
8. **analog has no glide** (`glide` arrives as a note property, `params.rs` port props, but no portamento time) and no mono / legato
   switch; nothing else blocks the demo patch.
9. **Units**: deck-core's catalogue (`deck/core/src/units.rs`: time, length, angle, level, tempo) has no frequency or pitch; modules write
   `"Hz"`, `"st"`, `"cent"`, `"oct"` as free strings.
10. **The crows' 3D models are not in the catalog.** `character:crows` (catpope) is one 2D character (14 moods, `media/characters/crows/`).
    The three crows exist only on Proton Drive `Projects/Scutoons/characters/crow-fox`, `crow-lizard`, `crow-wolf`: one `.glb` each (1.3 / 2.7 /
    1.8 MB, one skinned mesh, a Mixamo humanoid skeleton of 65 bones – `Head`, `HeadTop_End`, `Neck`, `Spine`–`Spine2`, `Hips`, arms to the fingers,
    legs; **no beak or wing bones**, no animations) plus four turnaround PNGs. Placing outlets on a model is #712's research.

## 2. The node set

### 2.1 Where the nodes live

- **The DSP units go into the module SDK** (`deck/module/src/dsp/`: osc, svf, adsr, delay line, allpass / comb, one-pole, FFT, partitioned
  convolution, envelope follower). deck-core's `dsp.rs` and analog's private copies **move** there (A6: a move, not a copy; the SDK's
  `smooth.rs` already says deck-core's `Ramp` moves in the same way). The engine's built-in `osc` / `noise` keep working by calling the
  SDK's units (deck-engine already depends on deck-module with `modules`).
- **Every node is a factory module** (`deck/modules/<name>`, flat `src/*.rs`, the catalog's read-only `module:<name>` through a migration as
  2026-11-05-factory-modules did), so it plugs into a rig slot, a scene and a bus the same way on every deck (A3), shows in the library,
  and its descriptor draws its UI. No new built-in engine kind.
- **The core keeps** the clock, the event host, buses / sends / solo, the limiter, the analysis – unchanged.

### 2.2 Port types

| Type | Graph type it rides on | Rate | Use |
|---|---|---|---|
| audio | audio (planar, n channels) | per frame | sound |
| control | control (one f32) | per 128-frame quantum, one block late between modules | knobs, LFOs, envelopes as modulation, levels to visuals |
| gate / trigger | event (`music.note`, or `gate.on` / `gate.off`, `trigger` with a velocity) | frame-accurate | drum hits, envelope gates, steps |
| MIDI | event (`midi.note`, `midi.cc` with `ch`, and new `port`) | frame-accurate | the world node's output |
| clock | the tempo map in `Ctx` (not a cable); a `clock:` event (`clock.step` with its index) only where a node emits steps | per frame | sync of arp, sequencer, delay, LFO |

Gates are events, not control values: a control value moves once per 128 frames (2.7 ms at 48 kHz), a drum hit must land on its frame.
Every node with a gate input also takes `music.note` (a note on = gate on), so a sequencer lane plugs into a synth or an envelope alike.

### 2.3 The nodes

Units are deck-core's where it has them (s, ms, beats, dB, %, gain); **step 2 adds frequency (Hz, kHz) and pitch (st, cent, oct)**. Times
that sync take a note value (`1/16 … 2 bars`, the LFO's `choices` list in analog) or seconds. Every node: no allocation after `init`,
`QUANTUM` = 128 frames a call (events cut it), params smoothed in the node (SDK `Smooth`), SDK `math` only, a golden render
native ≡ wasm, a `no_alloc` test, a module-bench budget measured on the X1.

| Node | Ports in → out | Parameters (unit) | Real-time notes |
|---|---|---|---|
| **analog** (exists, extended) | notes, sustain → out (2) + **new** control outs `env` (amp env 1, 0–1), `level` (0–1) | the 91 of today + **`glide`** (0…2 s, log) + **`mono`** (poly / mono / legato) | glide per voice in pitch (st per s); mono steals voice 0 |
| **osc** (voice building block) | notes / gate, `fm` (audio), `pitch` (control, st) → out (1) | wave (sine / tri / saw / pulse), `semi` (st), `fine` (cent), `pw` (%), `detune` (cent) + `voices` 1–7 (supersaw spread) | PolyBLEP from the SDK; voices fixed at init (7 max) |
| **noise** | gate → out (1) | color (white / pink), `level` (dB) | seeded |
| **env** (ADSR / AD) | gate / notes → control `out` (0–1) + audio `out` (the env as audio, for VCA) | `mode` (ADSR / AD / AR), `a` `d` `r` (s, log), `s` (%), `curve` (lin / exp), `velocity` (%) | per-sample in audio, once per quantum on the control out |
| **vca / gain** | in (n), `cv` (audio or control) → out (n) | `gain` (dB, −inf…+12), `cv amount` (%) | ramps across the block |
| **filter** | in (n), `cutoff` (control, oct), notes (key track) → out (n) | `mode` (LP12 / LP24 / BP / HP / notch), `cutoff` (Hz, log), `res` (%), `drive` (%), `key` (%) | the SVF from the SDK, cascaded for 24 dB |
| **delay** | in (2), `time` (control) → out (2) | `time` (ms or note value, synced), `feedback` (%), `damp` (Hz), `mix` (%), `spread` (ms, ping-pong), `mod depth` (ms) + `mod rate` (Hz) – chorus / flanger at short times | ring buffer of `max` 4 s × rate allocated at init; fractional read (cubic); feedback clamped < 100 % |
| **reverb** (algorithmic) | in (2) → out (2) | `size` (%), `decay` (s, 0.2–20), `damp` (Hz), `predelay` (ms), `width` (%), `mix` (%) | an 8-line feedback delay network (Householder matrix), lines prime-length, allocated at init; denormal guard (wasm has no flush-to-zero) |
| **convolver** (impulse response) | in (1 / 2) → out (2), asset slot `ir` (`AssetKind::Pcm`) | `ir` (a sound asset), `gain` (dB), `predelay` (ms), `mix` (%), `length` (s, cap 4) | see 2.4 |
| **mixer** | in1…in8 (2 each) → out (2), `send` (2) | per input `gain` (dB), `pan` (−1…1), `send` (dB), `mute`; `master` (dB) | the engine's `mixer` stays for scenes; this one is the rig's |
| **arp** | notes (held), `clock` from `Ctx` → notes | `mode` (up / down / up-down / as played / random seeded), `rate` (note value, default 1/8), `gate` (%), `octaves` (1–4), `transpose` (st, −24…24), `latch` (off / on), `swing` (%) | the held set is a fixed array (16); steps from the tempo map's beat, so it is on the grid however blocks are cut (the generator's rule) |
| **transpose** | notes → notes | `semi` (st), `fine` (cent), `channel` filter (all / 1–16) | stateless; arp has its own `transpose` so a crow rig stays at three slots |
| **stepseq** (exists) | → notes | as today | unchanged; the melodic lane |
| **drumseq** | → one notes output per lane (`lane1`…`lane8`) + control `step` (index) | `steps` (1–16), `rate` (steps a beat, default 4), `swing` (%), per lane: `note` (st or key), `length` (% of a step), per step × lane: `on`, `accent`; `accent` velocity (%), `normal` velocity (%) | 8 × 16 cells are params (switches); the step index from the beat (deterministic) |
| **follower** | in (n) → control `level` (0–1), `gate` (event, above a threshold) | `attack` (ms), `release` (ms), `threshold` (dB) | the bridge from sound to light and picture (§3) |

The convolver and the reverb are the only heavy nodes; everything else is a few operations a sample.

### 2.4 The convolver (impulse response) in detail

- **Loading**: the IR is a sound asset (wav / mp3 / ogg decode in deck-media, #650) in the node's asset slot; the host hands it decoded
  (`Pcm`, frames × channels). In `init` the module resamples it to the deck's rate, normalises it, cuts it to `length`, and transforms the
  partitions into the spectral domain – all before the first block; a new IR = a new instance (A9: a module changes by restart).
- **Algorithm**: uniformly partitioned overlap-save with a frequency-domain delay line; partition = 128 frames = `QUANTUM`, FFT size 256;
  for long IRs a second, coarser partition size (1024) for the tail (non-uniform): a 4 s IR at 48 kHz is 8 partitions of 128 for the
  head and ~187 of 1024 for the tail instead of 1500 of 128.
- **Latency**: 128 frames (2.7 ms at 48 kHz), declared in the descriptor's `latency` – the compensation is the engine's (§2.6a of the engine
  plan); a later zero-latency mode adds a direct 128-tap FIR for the head.
- **Memory**: the partitions and the delay line at init (4 s stereo ≈ 3 MB of spectra); the SDK's FFT (radix-2, real input, twiddles at
  init) – the second FFT in the tree after deck-core's analysis, which then uses the SDK's too (A6).

## 3. Signals to visuals and light (the hooks only)

1. **Control out → a picture or 3D parameter:** a node's control output with a `path` (`<track>:<node>.<port>`, rig/template.rs) writes the
   being's or the space node's input (hue, glow, breath) or a motion slot's transform, one block late – no new mechanism.
2. **Control out → light:** deck-outputs maps any parameter to a DMX channel (range, curve, 8 / 16 bit, smoothing – `deck/outputs/src/shape.rs`),
   so an `env`, `level` or `follower` output drives a lamp's dimmer the same way a knob does.
3. **Gate / trigger → light:** a drumseq lane's or arp's notes reach the outputs' fixtures as events (a flash per hit) and the strobe already
   steps on the tempo grid (`strobe.rs`); the event route gets a frame-thread consumer (deck-events route.rs names it) for picture triggers.
4. **Audio → anything:** the `fft` node (level, bass, mid, high) or a `follower` per track turns sound into control values aligned to the
   heard moment (`analysis.rs` looks back by the output lead).
5. **Notes → external light desks:** MIDI out of deck-outputs repeats a lane or an arp as MIDI notes on a channel.
6. **Across decks:** a control or event output reaches another deck's lights through far addresses (#442, `deck.listen`).
7. **Back from the picture:** camera focus outlets ("in focus", "distance to camera", decisions/deck.md 2026-11-16 [modules]) and the
   motion of a being come into audio nodes as control inputs (more reverb out of focus).

## 4. The demo, exactly

Owner (#713): *"the three crows each rigged with an analog synth which listens to midi port 1. 1st a base tone, second at a 3rd and the last
at minor 5th. The analog sound should be tuned to a dissonant hover rave sound. All in one scene with 3 slots as 3d models inside the rig …
the outlets you can put onto the crows body at your own discretion. There is an arpeggiator also in each rig. Basic up at 8ths. Furthermore
there is another rig in the scene next to the fox crow and it is a drum machine … Basic. Into each step i can plug an analog synth and create a
basic punk rock groove."*

### 4.1 How I read the intervals

- **"minor 5th"** does not exist as a name in theory. Two readings: (a) the **diminished fifth = tritone, 6 semitones** – the most dissonant
  interval, matching *"dissonant"*; (b) "a minor third plus a perfect fifth" (a minor triad 0 / 3 / 7) – consonant, contradicts *"dissonant"*.
  **Recommended: (a), +6 st.**
- **"a 3rd"**: with +6 above, a **minor third (+3 st)** makes 0 / 3 / 6 = the **diminished triad**, the classic dissonant chord of rave stabs;
  a major third (+4) makes 0 / 4 / 6 (a Lydian ♭5 / whole-tone colour), also tense but brighter. **Recommended: +3 st** (open question 1).
- So: crow 1 at the played note (0 st), crow 2 at +3 st, crow 3 at +6 st, in each rig the arp's `transpose`.

### 4.2 The scene

- One scene `crows-rave` (toon catpope – the crows are catpope's – or sandbox; open question 5), one `tempo` node at **160 BPM**, 4/4, the
  click off, source internal (an external MIDI clock or Link may take over, as every scene).
- Four holders on a row, left to right: **crow-wolf** (crow 1, root, centre-left), **crow-lizard** (crow 2, +3), **crow-fox** (crow 3, +6),
  and **drum-machine** next to the fox crow on its right. My assignment of who plays what (the owner may swap): the wolf has the deepest
  look, the fox stands next to the drums as the owner placed it.
- One MIDI `world` node in the scene: `source = "midi"`, `port = 1` (the deck's first MIDI input in its input map), `channel = 1`,
  `listen = ["midi.note"]`, cabled to each crow rig's arp `notes` input (event cables, many to many).
- The scene's FX: a bus `fx-delay` (a `delay` node) and a bus `fx-space` (a `reverb` node); every crow track sends to both; the drum
  snare lane sends to a `convolver` on a bus `fx-room`. Strips: crows −9 dB each, wolf pan 0, lizard −0.5, fox +0.5; drums −6 dB.

### 4.3 A crow rig (three of the same, one transpose apart)

**Three slots** (my reading of *"3 slots as 3d models inside the rig"*: each rig holds three plugged modules, the first is the crow's model):

| Slot | What | Track |
|---|---|---|
| 1 · model | the crow's `.glb` (crow-wolf / -lizard / -fox), its skeleton gives the outlet places | – (the rig's body) |
| 2 · arp | `arp`: mode up, rate 1/8, gate 50 %, octaves 2, transpose 0 / +3 / +6 st, latch off, swing 0 | silent track `arp` |
| 3 · voice | `analog` with the rig preset `hoover-dissonant` | track `voice` (strip) |

Wiring: world `midi.note` → `arp.notes` → `analog.notes` → `voice` strip → main + sends. Holding one key plays the note and its octave
in 8ths; a held chord climbs through its notes; the three crows stack a diminished triad on every step, in lock-step on the clock.

**The patch `hoover-dissonant`** (analog parameter names; what is not listed stays at its default):

| Section | Values | Why |
|---|---|---|
| OSC 1 | `o1_wave` saw, `o1_fine` −15 cent, `o1_level` 0.75, `o1_route` filter 1 | the left detuned saw |
| OSC 2 | `o2_wave` pulse, `o2_pw` 0.35, `o2_fine` +15 cent, `o2_level` 0.7, `o2_sub` 0.45, `o2_route` + `sub2_route` filter 2 | the right side, PWM, the octave-down weight |
| NOISE | `noise_level` 0.04, `noise_route` both | air |
| FILTER 1 / 2 | lowpass; `f1_cutoff` 1800 Hz, `f2_cutoff` 2400 Hz; `f1_res` 0.35, `f2_res` 0.25; `f1_drive` = `f2_drive` 0.45; `f1_key` = `f2_key` 0.4; `f1_env` 0.30 (≈ 1.8 oct), `f2_env` 0.25 | the growl, brighter on the right |
| FILTER ENV | `fe1` a 0.002 s · d 0.18 s · s 0.25 · r 0.12 s; `fe2` a 0.002 s · d 0.09 s · s 0 · r 0.10 s | a pluck on every 8th |
| Pitch scoop | `fe2_p2` −7 st (osc 2 starts a fifth below and slides up in 90 ms) | the hoover's dive, a flam against osc 1 |
| AMP | `a1_level` = `a2_level` 0.8, `a1_pan` −0.45, `a2_pan` +0.45, `a1_vel` = `a2_vel` 0.3 | stereo width from the detune |
| AMP ENV | `ae1` = `ae2`: a 0.004 s · d 0.25 s · s 0.7 · r 0.15 s | tight enough for 8ths at 160 BPM (a step = 187.5 ms, the gate 94 ms) |
| LFO 1 | triangle, `l1_rate` 0.35 Hz, `l1_pwm` 0.25 | the slow PWM shimmer |
| LFO 2 | sine, `l2_sync` 1 bar, `l2_f1` = `l2_f2` 0.6 oct | a filter swell per bar |
| VOICE | `volume` 0.55, `vel_pwm` 0.1, **new** `glide` 0.06 s, **new** `mono` poly | glide smears the arp's jumps |

Detuning ±15 cent (30 cent apart) beats at ~3.8 Hz at A3; across the three crows the tritone and the minor third add their own
beating – the dissonance is in the chord, the "hoover" in the scoop, the PWM and the drive.

**Outlets on the crow's body** (skeleton bones of the Mixamo rig; the beak has no bone, so it is an offset from `Head`):

| Outlet | Place | Why |
|---|---|---|
| `arp.notes` in (MIDI from port 1) | `HeadTop_End` – the crown | the crow "listens"; the input sits where you look first |
| `analog.out` (audio) | the beak: `Head` + a forward offset | the sound comes out of the mouth |
| `arp` step (notes out) | `RightHand` – the right wing tip | a wing flap per step later (motion hook) |
| `analog.env` (control out) | `LeftHand` – the left wing tip | the second wing answers the envelope |
| `analog.level` (control out) | `Spine2` – the chest | breath and glow (§3 path 1) |

### 4.4 The drum machine rig

Slots: 1 · its body (a placeholder model until one is made; question 4), 2 · `drumseq`, 3–6 · four **analog** instances, one per lane,
each a drum preset. *"Into each step I can plug an analog synth"*: a **lane** is a cable from `drumseq.laneN` to an analog's `notes`;
each step of the lane triggers it. Lane 1–4 plugged; lanes 5–8 free for more. A step plays its lane's note at the step's velocity.

| Lane | Plugged | Preset (analog values; the rest default) |
|---|---|---|
| 1 kick | analog `kick` | `o1_level` 0 · `o2_wave` sine, `o2_key` 0 (ignores the note), `o2_semi` −24 st (65.4 Hz), `o2_level` 1, `o2_route` filter 2 · `f2_cutoff` 8000 Hz, `f2_drive` 0.35 · `fe2` a 0.001 · d 0.045 · s 0 · r 0.05, `fe2_p2` +24 st (the sweep from 262 Hz down) · `a1_level` 0, `a2_level` 1, `a2_vel` 0.4 · `ae2` a 0.001 · d 0.32 · s 0 · r 0.08 |
| 2 snare | analog `snare` | `noise_level` 0.9, `noise_route` filter 1 · `f1_mode` highpass, `f1_cutoff` 1200 Hz · `ae1` a 0.001 · d 0.17 · s 0 · r 0.06, `a1_level` 0.8 · `o1_level` 0 · `o2_wave` triangle, `o2_key` 0, `o2_semi` −5 st (196 Hz), `o2_route` filter 2, `f2_cutoff` 6000 Hz · `fe2` a 0.001 · d 0.03 · s 0, `fe2_p2` +5 st · `ae2` a 0.001 · d 0.09 · s 0 · r 0.05, `a2_level` 0.6 |
| 3 closed hat | analog `hat-closed` | `noise_level` 1, `noise_route` filter 1 · `o1_wave` pulse, `o1_key` 0, `o1_semi` +24 st, `o1_pw` 0.3, `o1_level` 0.25 · `f1_mode` highpass, `f1_cutoff` 7500 Hz, `f1_res` 0.25 · `ae1` a 0.001 · d 0.045 · s 0 · r 0.03, `a1_level` 0.55 · `a2_level` 0 |
| 4 open hat | analog `hat-open` | as `hat-closed`, `ae1` d 0.35 · r 0.2 |

All presets keep `a*_vel` > 0 so the accent is heard. The four presets are preset documents of `module:analog` (#346's file form).

**The groove** – a basic punk beat at 160 BPM, 16 steps = one bar of 16ths (`rate` 4 steps a beat), swing 0. `X` = accent (velocity 100 %),
`x` = normal (70 %), `.` = rest:

```
step        1 2 3 4 | 5 6 7 8 | 9 10 11 12 | 13 14 15 16
kick        X . . . | . . . . | X  .  x  . |  .  .  .  .
snare       . . . . | X . . . | .  .  .  . |  X  .  .  .
hat closed  X . x . | X . x . | X  .  x  . |  X  .  .  .
hat open    . . . . | . . . . | .  .  .  . |  .  .  x  .
```

Kick on 1 and 3 with a push on the "and" of 3, snare on 2 and 4, closed hats on the 8ths accented on the beats, the open hat on the last
"and" to lift into the next bar (the closed hat on 1 cuts it only by its own short decay – a choke group is not in the basic set). A second
pattern for later (the punk "skank" two-beat): kick 1 5 9 13, snare 3 7 11 15, hats every 8th.

### 4.5 Done when (the demo's acceptance)

A deck opens `crows-rave`; holding one key on the controller in MIDI input 1, channel 1 plays three crows in 8ths a diminished triad apart,
starting on the next 8th of the clock; the drum machine plays the groove with the transport; Stop stops all four; the headless scenario
renders the scene native and wasm bit-identically; the X1 holds it without a glitch (seven analog instances, two FX buses, one convolver).

## 5. Plan – tickets in order

Sizes S / M / L as RULES C7. Every ticket: one module or one change, `deck:ci`, `check:core-only`, a golden render, a `no_alloc` test,
a module-bench row on the X1; a factory module's sources reach production by a migration (as 2026-11-05-factory-modules).

| # | Ticket title | Done when | Size | Depends on |
|---|---|---|---|---|
| 1 | Live MIDI notes: note on at key-down, note off at key-up, and port + channel on the MIDI world node | a held key sounds at once in analog; `world` takes `port` and `channel`; a headless scenario plays a held chord; the recorded form (note with a duration) stays for recording | M | – (uses #468's input list as far as it exists) |
| 2 | Units: frequency and pitch in deck-core's catalogue | Hz / kHz and st / cent / oct are `Unit`s with conversions; the module params' unit strings are checked against the catalogue | S | – |
| 3 | DSP units in the module SDK: one copy for modules and the engine | `deck_module::dsp` holds osc (PolyBLEP + wavetable), svf, adsr, one-pole, delay line, allpass / comb, real FFT, follower; deck-core `dsp.rs` and analog's copies moved (not copied), analog renders unchanged (its golden hash), the engine's `osc` / `noise` unchanged | L | 2 |
| 4 | Factory modules env, vca, filter, noise, osc, mixer, transpose | each a module with the ports and params of §2.3, a preset, golden render, budget | M | 3 |
| 5 | analog: glide, mono / legato, env and level outputs | the three params and two control outputs; old presets render as before; migration for `module:analog` | S | 3 |
| 6 | arp as a module: held notes, up / down / up-down / played, 1/8 default, gate, octaves, transpose, latch | plays held notes on the beat grid however blocks are cut; the deck-events `arp` generator removed or delegating (no two arps) | M | 1, 3 |
| 7 | delay module: synced or free time, feedback, damping, ping-pong, modulation (chorus) | a click in gives the right echoes at 1/8 dotted, 160 BPM; modulated at 7 ms it chorus-es; no allocation after init | S | 3 |
| 8 | reverb module: an 8-line FDN | decay time within 10 % of `decay` (RT60 measured in the test), stable at 20 s, denormal-safe in wasm | M | 3 |
| 9 | convolver module: impulse responses from a sound asset, partitioned, 128 frames latency | a wav IR in the asset slot; the output equals direct convolution within −90 dB; latency 128 declared; 4 s IR within the X1 budget | L | 3, 7 (FFT in 3) |
| 10 | drumseq module: 8 lanes × 16 steps, accent, swing, a notes output per lane | the §4.4 groove as its default pattern plays the four analog lanes on the clock; follows the transport | M | 1, 3 |
| 11 | analog drum presets and the hoover preset as documents | `kick`, `snare`, `hat-closed`, `hat-open`, `hoover-dissonant` import as preset documents of `module:analog` | S | 5, #346 |
| 12 | The three crows into the catalog with their 3D models | `crow-fox`, `crow-lizard`, `crow-wolf` as characters (or personas of `crows`) with their `.glb` from Proton Drive, allocated to catpope | S (an owner step for the upload) | – (owner, question 3) |
| 13 | Outlets on a model: bones as outlet places | an outlet can be pinned to a bone (+ offset) of a holder's glb and the tube attaches there | M | #712's plan |
| 14 | Sound → light and picture hooks, first wire-up | a `follower` / `env` output drives a DMX dimmer and the space node's glow in a sandbox scene; one strobe flash per kick | S | 4, 5 |
| 15 | The three-crows rave demo scene | §4.5 is true on the X1 and in the browser deck | M | 1, 4–13 |

Steps 1–3 are the foundation; 4–10 the nodes (parallel after 3); 11–13 the demo's content; 14 the first step towards visuals and light;
15 the proof.

## 6. Open questions for the owner

1. **The third:** minor (+3 st, a diminished triad with the tritone) or major (+4 st)? *Recommended: minor, the darkest and most dissonant.*
2. **"Minor 5th":** the tritone (+6 st)? *Recommended: yes – the alternative (minor third + perfect fifth) is a consonant minor chord.*
3. **The crows:** three characters (crow-fox, crow-lizard, crow-wolf) or three personas of `character:crows`? *Recommended: personas of
   `crows` – the story already has "the crows" as one character; the 3D models become the personas' looks.*
4. **The drum machine's body:** a model of its own (a module's 3D object, decision 2026-11-05 [modules]) or a plain node for now? *Recommended:
   a plain node for the demo; its 3D object later in the module design steps.*
5. **Which toon holds the demo:** catpope or the sandbox? *Recommended: sandbox (agents/SANDBOX.md) – a test scene, the crows referenced from
   catpope.*
6. **"MIDI port 1":** the deck's first MIDI input, channel 1? *Recommended: yes, input 1 of the deck's input map, channel 1 – or omni if you
   want any channel to play the crows.*
7. **Tempo:** 160 BPM for rave and punk at once? *Recommended: 160; the punk beat also works at 170–180 if you want it faster.*
8. **The arpeggiator's home:** a module (one copy) instead of the deck-events generator? *Recommended: the module; the generator's
   rule (the step from the beat, not from block counting) moves with it.*
9. **Convolution latency:** 128 frames (2.7 ms) with the engine's compensation, a zero-latency head later? *Recommended: yes.*
