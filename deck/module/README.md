# deck-module – the module SDK and its ABI (v1, and v2 opt-in)

A rig module (research/RIG-EDITING.md §4) is a Rust crate against this one, compiled to `wasm32-unknown-unknown`
(`crate-type = ["cdylib", "rlib"]`) and run by wasmtime on a deck (deck-native, feature `modules`) or by the browser's
engine (live-core). The example is `deck/modules/walk` (the built-in `walk` node, proven equal to it by its tests).

```toml
[dependencies]
deck-module = { path = "…" }   # no other dependency, no build.rs, no macro of your own
```

```rust
use deck_module::{Block, Ctx, Event, Module};

#[derive(Module)]
#[module(name = "walk")]                       // the node kind `module:walk`; default = the struct's name in snake_case
#[port(output, audio, name = "cv")]            // input|output, audio|control|video|event, name
#[port(output, event, name = "notes")]
struct Walk {
    #[param(range = 0.0..1.0, default = 0.0)]  // literals only: the descriptor is a constant
    spread: f32,
    state: Option<u32>,                        // any other field must be `Default`
}

impl Module for Walk {
    fn init(&mut self, ctx: &Ctx) {}                                // once
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {}          // audio blocks – no allocation
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {}          // control rate (~60 Hz)
}
```

## ABI v2 (#340, research/ABI-V2.md) – beside v1 until #343

`#[module(abi = 2)]` opts a module into v2: `use deck_module::v2::{Block, Ctx, Ev, Module, Outputs …}`. The memory layout is
derived from the descriptor (`v2::Layout::of(&Shape)`, one `const fn` for the derive and the host; `dm_layout()` exports its
hash), so ports, channels, parameters, events, sensors and assets are descriptor fields, not ABI limits. What the derive adds:

- `#[module(name, abi = 2, domain = "audio"|"control", doc, needs = "motion", wants = "position|camera.motion", latency = 32)]`,
  `#[reads("story.act", …)]` (→ `Self::STORY_ACT: Var`, `ctx.var(Self::STORY_ACT)`), `#[asset(name, kind = "pcm"|"bytes"|"image",
  max_bytes, doc)]` (→ `Self::KICK: AssetSlot`, `ctx.asset(Self::KICK)`), `#[preset(name, doc, values(p = x, …))]` (the one factory preset).
- `#[port(…, audio|cv|control|event|video, channels = 2|"any", sidechain, names = "music.note", props = "pitch|vel", capacity = 64,
  path = "lead:@preset", mode = "set"|"add"|"mul", choices = "a|b", doc)]`; every port gets `Self::<NAME>`, its index in its group.
- `#[param(range, default, curve = log, unit = "Hz", choices = "saw|pulse", doc)]` (choices: the value is the index).
- `Block`: `audio_in(p) -> Planar` / `audio_out(p) -> PlanarMut` (`ch()`, `chan(c)`, `stereo()`), `events_in(p) -> &[Ev]`,
  `control_in(i)`, `set(i, v)`, `emit(port, Ev)`, `set_silent(p)`. `Outputs` (control domain): `set`, `emit`. `Ctx`: times
  (`frame`, `show_ms`, `tempo`, `beats`, `bar`), `seed`, `character()` (+ mood index, energy / touch / strain), `var`, sensors
  (`motion()`, `position()`, `face()`, `camera_motion()` with a `Status`), `asset`, and the inputs `control_in` / `events_in`.
- Parameters are a table the host writes (clamped, NaN ignored); the dirty ones are copied into the fields before a call and
  `Module::params_changed(dirty)` follows. `Smooth` and `deck_module::math` (libm) for the DSP.
- Exports: `dm_abi` (2), `dm_io`, `dm_layout`, `dm_descriptor_ptr/len`, `dm_new`, `dm_process` **or** `dm_control`,
  `dm_asset_buf` (with assets). No `dm_set_param`. The counting allocator counts allocations inside a call into
  `OutHdr.allocs` and traps when the header's `strict` is set.
- Tests: `v2::testing::Rig` (strict; install `v2::alloc::Counting` as the test binary's global allocator to count natively).
  The host side reads the descriptor with `host::parse_v2` / `validate_v2` and computes the same layout with `Shape::of`.
- **The host** (deck-module-host `v2::Instance`, #341): stages `inp` through `v2::io::InpMut` (the code `Rig` uses too) and
  crosses the boundary with **one copy in and one copy out per call**. It refuses a `dm_layout()` it does not compute itself,
  binds the assets **before `dm_new`** (so `init` sees them; `dm_asset_buf`, the bytes, the slot record), then **freezes the
  memory** at its size: from then on `memory.grow` fails – allocate in `init`; a call's allocation is served only from the heap
  `init` left. Two modes (`Setup::mode`): **strict** (tests, Play – an allocation or an overrun of 0.75 × the quantum traps, a
  trap silences the node for good) and **show** (allocations and overruns are counted in `ModuleStats`; a runaway past 4 × the
  quantum is stopped; a trap **restarts** the instance: the last block fades out, `take_rebuild()` hands the platform's loader
  thread a `Rebuild` – the dead instance, a snapshot of the parameters and context – whose `run()` builds a fresh one from the
  cached compile, `offer()` swaps it in at the next block with a fade-in; a module that keeps trapping waits one block, 0.1 s,
  1 s, then 10 s). A block in which a `needs` sensor is not live is not run (`unavailable()`). `Preset::resolve` turns values by
  name into indices once (gone names and out-of-range values reported), `Instance::apply_preset` applies one without allocating.
  The engine's seam (`deck_module_host::Instance`) picks v1 or v2 by `dm_abi()`. Its conformance suite:
  `deck/module-host/tests/conformance_v2.rs`, fixture `deck/modules/fixture-v2`.
- **The engine** (deck-engine `module.rs`, `evhost.rs`, #342; ABI-V2 §9c): a node's graph inputs are the audio inputs, then
  the control inputs, then the parameters; its graph outputs the audio and control outputs in declaration order
  (`deck_engine::register_module`, the one mapping). Audio crosses as channel slices of the planar pool (as many as the
  cable carries; an `any` output follows the engine's width). A control output written in block k reaches its consumers at
  k + 1, unwritten = held; a `path` output is bound by deck-space to the input it names (`<node>.<port>`, a rig's
  `<track>:<node>.<port>`; `@preset` / `@gain` … with #314). A `domain: control` module steps in the control half, once a
  block. Event inputs listen to their `names` (the deck's dispatcher and other modules); notes arrive as on / off with host
  ids, a duration expanded, Hz and key both filled from the deck's tuning; event outputs reach other modules one block late
  at their frame and the platform as deck-events events with `dur` (`Events::take_outbox`). A deck opens v2 instances in
  the show mode; a restart's rebuild runs on its loader thread (`Handles::restarts`), the held notes are sent again.

## What a module sees and may do (v1)

- **`Ctx` (read-only)**: `tempo()` (bpm, running, per_bar, `beat_at(frame)`, `frame_of(beat)` – deck-core's `TempoMap`,
  same formulas), `beats()` (the beat span of this call), `phase()`, `frame()`, `frames()`, `rate()`, `character()`
  (id, persona, mood), `var("key")` (scene and world variables, key → number), `seed()` (the same on every deck that plays
  the same rig – use it for anything random, never a clock).
- **`Block`** (audio): `audio_in(n)` / `audio_out(n)` (left, right; the n-th declared audio port), `control_in(n)`,
  `set_control(n, v)`, `emit(Event)`. Audio outputs start at silence.
- **Writing back** only through declared outputs: audio, control, events (`Event::at_beat("music.note", beat).lasting(b).with("pitch", hz)`,
  up to 4 props, 64 events a call). A module cannot change a global, a file, the network.
- **`deck_module::cv::GateCv`** renders `music.note` events to the cv the stage's `seq` writes (pitch Hz left, gate right,
  exponential glide) – the way a melodic module plays an fm / osc voice.
- **`log!("…")`**: a line to the console under the editor; rate-limited by the host, per module.
- A **panic or trap** silences the node, not the show; so does running past the time budget.

Parameters are also the node's control inputs (a link into `walk.spread` sets it; unlinked, the node's TOML value or the
default does). Ports are numbered per direction and type in declaration order: the first `#[port(output, audio)]` is
`block.audio_out(0)`.

## The descriptor

JSON, written by the derive: the module's name, version, ABI version, parameters (name, min, max, default), ports
(name, dir `in|out`, kind `audio|control|video|event`). It is a constant, stored in the wasm's custom section
**`deck.module`**: a host reads it without instantiating (`host::read_descriptor(&wasm)` in Rust,
`WebAssembly.Module.customSections(module, "deck.module")` in JS) and `host::parse` reads it into deck-catalog's
`Descriptor` – the one type the built-in kinds use too (#339) – and `host::validate` checks it (ABI version, names, ranges,
counts, and `Descriptor::check` for the ABI v2 fields: event `names` / `props` / `capacity`, `sidechain`, control-output
`path` / `mode` / `choices`, `reads`, `wants`, `assets`, `preset`, `latency`; the derive writes them from #340 on). Fallback for a host that instantiates anyway: `dm_descriptor_ptr()` / `dm_descriptor_len()`.
Limits: 128 parameters, 4 audio ports (stereo) and 8 control ports per direction. `video` ports are declared for later
(WGSL data, #185); a host that cannot run them refuses the module with a message.

## The ABI (v1, `wasm32-unknown-unknown`)

The module owns one `Io` (deck-module `abi::Io`, `repr(C)`, numbers and byte arrays only, ~72 KB) in its linear memory.
The host writes `Io::inp`, calls an export, reads `Io::out`; field offsets from `dm_io()` come from the same type
(`host::Offsets::get()`), so wasm32 and the host cannot drift (a test checks the layout).

| export | |
|---|---|
| `dm_abi() -> u32` | `1`. A host refuses another number. |
| `dm_io() -> u32` | address of the `Io`. |
| `dm_descriptor_ptr() -> u32`, `dm_descriptor_len() -> u32` | the descriptor JSON. |
| `dm_new()` | builds the module with every parameter at its default, then `init`. Before: `inp.rate`, `inp.seed`, the tempo, character, variables. |
| `dm_set_param(index: u32, value: f32)` | parameter `index` (declaration order), clamped to its range; NaN ignored. |
| `dm_process(frames: u32)` | one audio call, ≤ 1024 frames (the host splits a longer block): audio outputs cleared, then `process`. |
| `dm_control(frames: u32)` | one control tick: `control`. |

Each call clears `out.events_len` first – read the events after each call. One import, only if the module logs or panics:
`env.deck_log(level: u32, ptr: u32, len: u32)` (0 info, 1 warn, 2 error; UTF-8 in the module's memory). The host sets
`inp.frame` (the audio frame of the call's first frame, on the tempo map's axis), `inp.frames`, the tempo anchor
(`tempo_frame`, `tempo_beat`, `bpm`, `rate`, `running`, `per_bar`), the control inputs, the audio inputs, and – when they
change – character and variables. Strings are NUL-padded byte arrays.

## Tests

`testing::Rig<M>` runs a module natively through the same `rt` code the exports call: `Rig::new(rate, seed)`,
`set_param`, `set_tempo`, `set_character`, `set_vars`, `process(frame, frames)`, `events()`, `audio_out(n)`. A module's
tests are plain `#[test]`s (the build service runs them as wasm under wasmtime with fuel and memory limits – never
natively). Prove a module against a built-in the way `modules/walk/tests/builtin.rs` does.

## Build

`cargo build --release --target wasm32-unknown-unknown -p <module>` → `target/wasm32-unknown-unknown/release/<module>.wasm`.
No `build.rs`, no proc-macro of your own, no dependency beyond this crate (the compile service writes `Cargo.toml` itself).
Clippy takes the repeated `#[port(…)]` lines of one struct for duplicates: put `#![allow(clippy::duplicated_attributes)]` at the top of the module crate.

## Hosts

- **deck** (deck-native, feature `modules`, `deck/native/src/module_host.rs`): wasmtime (NaNs canonicalized, relaxed SIMD and threads off – the same bits on every deck). This paragraph is v1; v2: the section above. `DECK_MODULES=<folder of .wasm>` is loaded before a scene is built and each module becomes the node kind `module:<name>` (ports from the descriptor: audio / control inputs as declared, then every parameter as a control input; audio outputs). Compile cached in memory and, with `DECK_MODULE_CACHE=<dir>`, on disk by SHA-256. A trap or a time-budget overrun (≈ 0.75 × the block, 2–50 ms, wasmtime epoch interruption) silences that node and logs it; memory capped at 64 MiB; one import (`env.deck_log`, 10 lines burst, then 10 a second). Node params besides the module's own: `seed` (default 1), `character`, `persona`, `mood`, `vars` (table key → number). A v1 module's control outputs and any video port are refused with the reason; its events stay unwired (readable by `Instance::events`) – v2 wires both (above).
- **browser** (`shared/live-core/src/modules.ts`): the same `.wasm` in the browser's engine, V8 in the tests; `module-abi.ts` is generated from this crate (`UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts`). A browser has no epoch budget: a runaway loop is the worker's to terminate.
