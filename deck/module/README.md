# deck-module – the module SDK and its ABI (v2)

A rig module (research/RIG-EDITING.md §4) is a Rust crate against this one, compiled to `wasm32-unknown-unknown`
(`crate-type = ["cdylib", "rlib"]`) and run by wasmtime on a deck (deck-native, feature `modules`); the browser's engine
takes it with its web backend (#290). The examples are `deck/modules/walk` (the built-in `walk` node, proven equal to it),
`deck/modules/analog` (an instrument with a note input) and `deck/modules/fixture` (the conformance fixture). Module ABI v2
(research/ABI-V2.md, #339–#343) is the only ABI: a module built against v1 is refused and recompiled (owner 2026-11-03: no
backwards compatibility; see "Migration" below). Written against the code as built (#339–#344); the design and the reasons are
research/ABI-V2.md.

```toml
[dependencies]
deck-module = { path = "…" }   # no other dependency, no build.rs, no macro of your own
```

```rust
use deck_module::{Block, Ctx, Ev, Module};

#[derive(Module)]
#[module(name = "walk", doc = "…")]                    // the node kind `module:walk`; default = the struct's name in snake_case
#[port(output, cv, name = "cv")]                         // a 2-channel audio port: pitch Hz left, gate right
#[port(output, event, name = "notes", names = "music.note", props = "pitch|vel|glide", capacity = 64)]
struct Walk {
    #[param(range = 0.0..1.0, default = 0.0, doc = "…")] // literals only: the descriptor is a constant
    spread: f32,
    state: Option<u32>,                                  // any other field must be `Default`
}

impl Module for Walk {
    fn init(&mut self, ctx: &Ctx) {}                         // once, after the parameters and the assets
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {}   // domain "audio": one call ≤ 128 frames – no allocation
}
```

## What the derive takes and writes

The memory layout is derived from the descriptor (`Layout::of(&Shape)`, one `const fn` for the derive and the host;
`dm_layout()` exports its hash), so ports, channels, parameters, events, sensors and assets are descriptor fields, not ABI
limits (sanity bounds: 8 audio and 8 event ports a direction, 256 control ports, 1024 parameters, 32 channels a port).

- `#[module(name, domain = "audio"|"control", doc, needs = "motion", wants = "position|camera.motion", latency = 32,
  moods = "thinking|sad|happy", characters = "catpope|yellowmouse.kid")]` (`moods`, A1 #409: the moods the module knows –
  `character().mood_index` counts in them; `characters`: the characters it is made for, listed beside their rigs in Build as
  related – it still plugs in anywhere; `slot = "camera.motion"`: the entity's slot it attaches to, see "Slots" below),
  `#[reads("story.act", …)]` (→ `Self::STORY_ACT: Var`, `ctx.var(Self::STORY_ACT)`), `#[asset(name, kind = "pcm"|"bytes"|"image",
  max_bytes, doc)]` (→ `Self::KICK: AssetSlot`, `ctx.asset(Self::KICK)`), `#[preset(name, doc, values(p = x, …))]` (the one
  factory preset; every other preset is a document).
- `#[port(input|output, audio|cv|control|event|instances|video, name, channels = 2|"any", sidechain, names = "music.note", props =
  "pitch|vel", capacity = 64, path = "lead:@preset", mode = "set"|"add"|"mul", choices = "a|b", doc)]`; every port gets
  `Self::<NAME>`, its index in its group (audio, control, event per direction). `instances`: see "Drawing" below.
- `#[param(range, default, curve = log, unit = "Hz", choices = "saw|pulse", doc)]` on `f32` fields (choices: the value is the index).
- `Block`: `audio_in(p) -> Planar` / `audio_out(p) -> PlanarMut` (`ch()`, `chan(c)`, `stereo()`), `events_in(p) -> &[Ev]`
  (sorted by frame; notes on / off paired by id, Hz and key both filled by the host), `control_in(i)`, `set(i, v)`,
  `emit(port, Ev)`, `set_silent(p)`, `audio_out_with_events`. `Outputs` (control domain): `set`, `emit`, `instances`. `Ctx`: times
  (`frame`, `show_ms`, `tempo`, `beats`, `bar`), `seed`, `character()`, `var`, sensors (`motion()`, `position()`, `face()`,
  `camera_motion()` with a `Status`), `asset`, and the inputs `control_in` / `events_in`.
- `ctx.character()` is **live** (#409): `id` (the node's `character`), `persona`, `mood` (strings), `mood_index` (the mood's
  place in the module's own `moods` + 1; 0 = a mood it does not list), `energy`, `touch`, `strain` (0 … 1, the being's).
  The node's params give them until the deck's first live context; from then on the deck's frame thread sends its mood,
  persona and the being's senses (deck-space `Deck::context`, the native being) through the engine's command ring, and
  every module node has them from the next block's start – natively and in live-core alike (`lc_context`). Match the
  string or switch on the index; both change on the same block.
- Parameters are a table the host writes (clamped, NaN ignored); the dirty ones are copied into the fields before a call and
  `Module::params_changed(dirty)` follows. `Smooth` (`land`: arrived, once this close), `deck_module::math` (libm: the same bits native and wasm) and the units of `deck_module::dsp` (below) for the DSP;
  `cv::GateCv::render_notes` renders a module's own notes into a `cv` port.
- Exports: `dm_abi` (2), `dm_io`, `dm_layout`, `dm_descriptor_ptr/len`, `dm_new`, `dm_process` **or** `dm_control`,
  `dm_asset_buf` (with assets). The counting allocator counts allocations inside a call into `OutHdr.allocs` and traps when
  the header's `strict` is set. One import, only if the module logs or panics: `env.deck_log(level, ptr, len)` (`log!`).

## The descriptor

deck-catalog's one `Descriptor` as JSON (`abi: 2`; the type the built-in kinds use, #339), written by the derive into the
wasm's custom section **`deck.module`**: a host reads it without instantiating (`host::read_descriptor`), `host::parse`
reads and `host::validate` checks it (names, ranges, a domain a module can have, the layout's limits, the closed sensor
vocabulary, `Descriptor::check`) and returns the `Shape` the layout comes from. The server stores it with the compile
(ModuleSource refuses any other ABI) and serves it as the kind `module:<id>` (GET /api/kinds).

## Sensors, assets, presets

- **Sensors** are a closed vocabulary (`Sensor`: `motion` 10 channels, `position` and `position.precise` 5, `camera.face` 55,
  `camera.motion` 4). `needs` is hard: the host does not run a block while the status is not live, and the engine node stays
  silent; `wants` runs with zeros. In the module: `ctx.motion()`, `position()`, `face()`, `camera_motion()` or `ctx.sensor(s)`
  give `(Status, values)`; `Status` is `Unavailable · Asking · Denied · Live · Replay` (`status.live()` = Live or Replay). A
  platform feeds them with `Instance::set_sensor(sensor, status, values)`; the core's channel table, permission prompts and the
  web adapters are #312's, not built. The conformance suite replays a JSON-lines log (`{frame, channel, value}`).
- **Assets** are read-only slots a descriptor declares (`#[asset]`, kind `pcm` planar f32 / `bytes` / `image` RGBA8). The host
  binds them from `Setup::asset(AssetData::pcm(..) | ::bytes(..))` **before `dm_new`**, through `dm_asset_buf(slot, len)`; a slot
  is filled once per instance. `ctx.asset(Self::KICK)` is `None` until ready - play silence, never wait. Resampling is the module's.
- **Presets:** the descriptor holds one factory preset (`#[preset]`, only what differs from the defaults). `Preset::resolve(desc,
  name, values)` maps parameter names to indices and reports unknown ones; `Instance::apply_preset` / `apply_factory_preset` write
  defaults, then the values, without allocating. Other presets are documents (the module-preset form and its file import are
  #346; analog's other three presets wait in `modules/analog/analog.preset.json`).

## Slots (motion)

A module can attach to an entity's slot instead of a rig's track (#590, research/MODULE-EXTENSIONS.md): the slot table is
deck-catalog's `slot` (each entity kind, its domains, what a module there reads and writes; `camera.motion` and
`item.motion` run today). `slot = "<entity>.motion"` makes a control-domain module without ports that gets the transform
regions: `ctx.transform()` (`TransformIn`: what the entity is, where its shot, move or spot puts it, the row's time, its
marks – a camera's shots, an item's spot) and `out.transform()` (`TransformOut`: `offset_position`, `offset_rotation`,
`scale_by`, `offset_target`, `offset_focal` on top of that base, or `set_*` to drive a part while attached;
`set_projection`). The deck calls it once a drawn frame, after the moves; only the parts its slot writes are taken.
`modules/dolly` (a camera) and `modules/bob` (an item) are the examples; `module-host/tests/motion.rs` proves them
native ≡ wasm.

## Drawing (the instance table)

A control-domain module draws by declaring one table, `#[port(output, instances, name = "shapes", capacity = 256)]`
(1 … 1024 instances, 256 when not said): `out.instances()` is its `Table` – `clear`, `push(Instance)`, `rows_mut`. An
`Instance` (64 bytes, `repr(C)`) is a shape (`instances::shape`: `PUFF` a lumpy ball, `SEGMENT` an upright prism from its
foot – the core's placeholders, about 1 m), a position (m, from the node's place), a turn (unit quaternion x y z w), a scale
per axis and a linear RGBA colour. The table holds what the module last wrote. Its region (a `TableHead`, then the records)
is the last of `out`, allocated with the module; the host copies it with the rest of `out` after every call, the engine
publishes it after each step to deck-space's `instances::Table`, and the frame thread draws it the next frame (one block
late, the latest table wins – the producer never waits). The space (`space/drawn.rs`) draws every table with one instanced
pipeline, a draw per shape, at the module node's place: a node of a holder's rig stands at that member's feet (its motion
module turns it), any other at the scene's spot on the floor.

Why a port type and not a new domain or the `Item/Location.Object` slot: what a module draws must come from the same
deterministic step its physics and its notes come from (the 128-frame control quantum, native ≡ wasm, golden-tested), so it
is a control module's output like its control values and events – a port, declared in the descriptor, laid out by
`Layout::of`, with no second call, no second scheduler and no render-thread module runtime. A domain would split a module
that both sounds and draws (a bamboo forest: hits → notes, stalks → instances) into two; the Object slot runs on the render
thread per drawn frame, which would make a simulation depend on the frame rate. The slot can later take a table too.
TODO(#780): a shape may name a model asset (the owner's low-poly models) instead of a placeholder; the browser deck draws
tables once it runs modules (#290).

## DSP units (`deck_module::dsp`)

One copy for every module and the engine's built-in kinds (`src/dsp/`): build a unit in `init` (the ones with a buffer or a
table allocate there), then it never allocates (`tests/dsp_no_alloc.rs`), and a feedback state that decays ends at 0 instead of
in the denormals (`dsp::flush` – wasm has no flush-to-zero).

| unit | what |
|---|---|
| `osc` | `Osc`: PolyBLEP saw / pulse (width) / triangle / sine, pitch and width per sample, hard sync as a band-limited step (output one sample late); `Sub`: a square an octave down. |
| `wavetable` | `WaveTable` + `Osc`: saw / square / triangle from a table per quarter octave – cleaner than PolyBLEP (aliasing under −80 dB), ≈ 350 KB a wave, built once. The engine's `osc`. |
| `noise` | `White` (xorshift64*, seeded), `Pink` (−3 dB an octave). The engine's `noise`. |
| `svf` | `Svf`: LP / BP / HP / notch from a control-rate `Coef` (`tick`), or the lowpass with cutoff and Q per sample (`lowpass`); `drive`. |
| `onepole` | `OnePole`: 6 dB lowpass / highpass, `coef(cutoff, rate)`. |
| `adsr` | `Adsr`: linear attack, exponential decay and release that take exactly their time. |
| `line` | `Line`: the ADSR with straight attack, decay and release, the same gate and times. |
| `follower` | `Follower`: a signal's size as a level, attack and release times. |
| `level` | `gain(db)`: a dB parameter as a factor, 0 at `OFF_DB` (−60) and below. |
| `delay` | `Delay`: a ring with whole (`read`) and cubic fractional (`read_frac`) reads; `Allpass` (Schroeder); `Comb` (feedback with a lowpass in the loop). |
| `fdn` | `Fdn`: eight delay lines, a Householder mix, a gain and a lowpass in every loop (`tick`); `gain` (a loop gain from RT60), `primes` (mutually prime lengths). |
| `fft` | `Fft`: radix-2, a real signal into its spectrum (`real`) and back (`inverse`), in the caller's buffers. |
| `grid` | `steps`: a sequencer's steps on the beat grid (step `k` at beat `k / per_beat`); `frame_in`: the call a beat lands in, one call whatever the cut. |
| `arith` | `exp2`, `log2`, `sin_turns`, `tan_pi`, `soft` in f64 from `+ − × ÷ floor` – what `osc`, `svf` and `adsr` compute with. |
| `arp` | `Held`: the held notes (16, latch) and step `k`'s note by `Mode` (up / down / updown / played) through octaves; `steps`: the beat grid. The `arp` module and deck-events' `arp` generator both play it. |

## The small audio modules

Eleven factory modules on those units (`deck/modules/<name>`, one `lib.rs` each; research/audio-nodes-foundation.md §2.3),
the pieces a rig wires into a voice or a drum: `arp` → `osc` → `filter` → `vca` ← `env`, summed by `mixer`, echoed by `delay`, spaced by `reverb`, a drum rig played by `drumseq`. A level in dB is
silent at its bottom (−60); every knob glides; an event takes effect on its frame.

| module | ports in → out | parameters | preset |
|---|---|---|---|
| `osc` | `notes` (event), `fm` (audio 1), `pitch` (control, st) → `out` (1) | `wave` saw / pulse / triangle / sine, `semi` st, `fine` cent, `pw` %, `detune` cent, `voices` 1–7 | supersaw |
| `noise` | `gate` (event) → `out` (1) | `color` white / pink, `level` dB, `mode` free / gated | hat |
| `env` | `gate` (event) → `out` (audio 1, a value a sample), `env` (control, a value a call) | `mode` adsr / ad / ar, `a` `d` `r` s (log), `s` %, `curve` lin / exp, `velocity` % | pluck |
| `vca` | `in` (any), `cv` (audio 1), `cv_control` (control) → `out` (any) | `gain` dB, `amount` % (how much the cv plays the gain) | vca |
| `filter` | `in` (any), `cutoff_mod` (control, oct), `notes` (event, key tracking) → `out` (any) | `mode` lp12 / lp24 / bp / hp / notch, `cutoff` Hz (log), `res` %, `drive` %, `key` % | acid |
| `mixer` | `in1` … `in8` (2 each) → `out` (2), `send` (2) | per input `gain` dB, `pan` −1 … 1 (a balance), `send` dB, `mute`; `master` dB | pair |
| `transpose` | `notes` (event) → `out` (event) | `semi` st, `fine` cent | octave up |
| `delay` | `in` (2), `time_mod` (control, oct) → `out` (2) | `sync` free / 1/32 … 1/1 (triplets, dotted), `time` ms (log), `feedback` % (under 98), `damp` Hz, `pingpong` %, `spread` ms, `mix` %, `depth` ms + `rate` Hz (chorus at short times) | dub |
| `reverb` | `in` (2) → `out` (2) | `size` %, `decay` s (RT60, 0.2–20, log), `damp` Hz (top: none), `predelay` ms, `width` %, `mix` % | hall |
| `arp` | `notes` (event, held: on at key down, off at key up) → `out` (event, a note a step on the beat grid) | `mode` up / down / updown / played, `rate` 1/1 … 1/32 (t = triplet; 1/8), `gate` %, `octaves` 1–4, `transpose` st, `latch` | crow |
| `drumseq` | → `lane1` … `lane8` (event), `step` (control) | per lane and step `lNsM` off / on / accent; per lane `noteN` (MIDI key), `lengthN` % of a step; `steps`, `rate` steps/beat, `swing` %, `accent` %, `normal` % (velocities) | skank |

One factory module draws (`deck/modules/cloud`, a control module, see "Drawing"):

| module | ports → out | parameters | preset |
|---|---|---|---|
| `cloud` | → `shapes` (instances, 96: 24 puffs, then 72 drops) | `puffs` 3–24, `size` m (glides), `drift` m/s (the wind along x, either way), `height` m, `range` m (it wraps after), `hue`, `tint` %, `light` %, `rain` 0–1 (the share of the drops falling – segments from the underside to the place, each starting again under a puff), `split` 0–1 (slices of the cloud drift apart as smaller parts and back), `swell` 0–1 (slowly grows and shrinks); its layout is the node's `seed` | storm |

Each has `tests/golden.rs` (its script's hash and what it does), `tests/no_alloc.rs` (the script with every parameter
turned), a row in module-bench (`cargo run --release -p module-bench -- factory`, results/factory/) and is proven
native ≡ wasm by `deck/build/tests/factory_wasm.rs`. A module's golden and no_alloc tests share one script through
`deck_module::golden` (`Bench`: parameters by name, a seeded input signal; `Hash`).

## Tests

`testing::Rig<M>` runs a module natively through the same `rt` code the exports call – strict, as the build service's tests
and the creator's Play: install `deck_module::alloc::Counting` as the test binary's `#[global_allocator]` and a call that
allocates fails the test. Prove a module against a built-in the way `modules/walk/tests/builtin.rs` does, and its wasm
against its native run the way `modules/walk/tests/wasm.rs` and `modules/analog/tests/wasm.rs` do (native ≡ wasm).

## Build

`cargo build --release --target wasm32-unknown-unknown -p <module>` → `target/wasm32-unknown-unknown/release/<module>.wasm`.
No `build.rs`, no proc-macro of your own, no dependency beyond this crate (the compile service writes `Cargo.toml` itself).
Clippy takes the repeated `#[port(…)]` lines of one struct for duplicates: put `#![allow(clippy::duplicated_attributes)]` at the top of the module crate.

## The build service

`deck/build` (`scutoons-build`, routes `POST /compile`, `POST /validate` – the core's checks of instruments, presets and rigs for the API, #313 –, `GET /health`) writes the member's crate itself: the one dependency
`deck-module` (`compile::manifest`, stub `STUB_SOURCE`), the fixed release profile, a `clippy.toml` and pinned flags
(`determinism.rs`). The `clippy.toml` disallows the `std` float functions that differ between platforms (`sin exp powf ln tan` ...,
the message names `deck_module::math::...`); the service runs `cargo clippy` after the build and returns only those as warnings
(a hint, the conformance suite is the guarantee). The flags are `-Ctarget-cpu=generic -Ctarget-feature=-simd128,-relaxed-simd`.
Then the **strict run** (`strict.rs`): the built wasm is played in deck-module-host's strict mode - 96 blocks of 128 frames at
48 kHz, noise on audio inputs, 0.5 on control inputs, a note on / off on every event input, once with the defaults and once with
the factory preset. An allocation in `process` / `control`, a panic, a runaway or an overrun of 0.75 x the quantum (retried twice
first) is a 422 error carrying the module's own log line. Flat source files only (`lib.rs` + modules); a crate with a folder
like `modules/analog` is a repository module. `scutoons-build rebuild <dir>` compiles every module of a folder through the same
path and lists the failures (exit 1).

## Migration

ABI v1 is gone everywhere: the host reads `abi: 2` only (`host::validate`, `Instance::new`, the server's `ModuleSource`).
Migration `2026-11-04-module-abi-v2` clears the compile result (`descriptor` + `wasm`) of every stored v1 module; the sources stay.
`bin/console modules:rebuild` is the recompile list (a dry run; `--all` every module, `--apply --as <creator>` compiles through
the service and saves the results as that creator's drafts); the failures are the sources that use a removed v1 API. The TS host
(`modules.ts`, `module-abi.ts`, `build:modules`) is deleted.

## Hosts

- **The host** (deck-module-host `Instance`, #341): stages `inp` through `io::InpMut` (the code `Rig` uses too) and crosses
  the boundary with **one copy in and one copy out per call**. It refuses a `dm_layout()` it does not compute itself, binds
  the assets **before `dm_new`**, then **freezes the memory** at its size (allocate in `init`). Two modes (`Setup::mode`):
  **strict** (tests, Play – an allocation or an overrun of 0.75 × the quantum traps, a trap silences the node for good) and
  **show** (allocations and overruns counted in `ModuleStats`; a runaway past 4 × the quantum stopped; a trap **restarts**
  the instance off the audio thread – `take_rebuild()` → `Rebuild::run()` on the platform's loader thread → `offer()`, faded
  in; a module that keeps trapping waits one block, 0.1 s, 1 s, then 10 s). A block in which a `needs` sensor is not live is
  not run. `Preset::resolve` / `Instance::apply_preset` apply a preset without allocating. Conformance suite:
  `deck/module-host/tests/conformance.rs` (generic over the `Backend`; a browser backend adds one line).
- **The engine** (deck-engine `module.rs`, `evhost.rs`, #342; ABI-V2 §9c): a node's graph inputs are the audio inputs, then
  the control inputs, then the parameters; its graph outputs the audio and control outputs in declaration order
  (`deck_engine::register_module`, the one mapping). Audio crosses as channel slices of the planar pool. A control output
  written in block k reaches its consumers at k + 1, unwritten = held; a `path` output is bound by deck-space to the input it
  names. A `domain: control` module steps in the control half. Event inputs listen to their `names` (the deck's dispatcher
  and other modules' outputs); event outputs reach other modules one block late at their frame (walk's `notes` play
  analog's `notes`) and the platform as deck-events events with `dur`.
- **deck** (deck-native, feature `modules`, `deck/native/src/module_host.rs`): wasmtime (NaNs canonicalized, relaxed SIMD
  and threads off). `DECK_MODULES=<folder of .wasm>` is loaded before a scene is built and each module becomes the node kind
  `module:<name>`; compile cached in memory and, with `DECK_MODULE_CACHE=<dir>`, on disk by SHA-256. A deck opens its
  instances in the show mode. Node params besides the module's own: `seed` (default 1), `character`, `persona`, `mood`
  (until the deck's live context, #409), `vars` (table key → number). Video ports are refused with the reason (#185).
- **browser**: the TS module host of ABI v1 is deleted (#343); the browser's engine (live-core) takes modules through the
  same `deck_engine::register_module` path once deck-module-host has its web backend (#290).
