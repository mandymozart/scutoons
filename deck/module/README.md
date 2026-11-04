# deck-module – the module SDK and its ABI (v2)

A rig module (research/RIG-EDITING.md §4) is a Rust crate against this one, compiled to `wasm32-unknown-unknown`
(`crate-type = ["cdylib", "rlib"]`) and run by wasmtime on a deck (deck-native, feature `modules`); the browser's engine
takes it with its web backend (#290). The examples are `deck/modules/walk` (the built-in `walk` node, proven equal to it),
`deck/modules/analog` (an instrument with a note input) and `deck/modules/fixture` (the conformance fixture). Module ABI v2
(research/ABI-V2.md, #339–#343) is the only ABI: a module built against v1 is refused and recompiled (owner 2026-11-03: no
backwards compatibility; the server's `modules:rebuild` lists them). The full guide is #345's.

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

- `#[module(name, domain = "audio"|"control", doc, needs = "motion", wants = "position|camera.motion", latency = 32)]`,
  `#[reads("story.act", …)]` (→ `Self::STORY_ACT: Var`, `ctx.var(Self::STORY_ACT)`), `#[asset(name, kind = "pcm"|"bytes"|"image",
  max_bytes, doc)]` (→ `Self::KICK: AssetSlot`, `ctx.asset(Self::KICK)`), `#[preset(name, doc, values(p = x, …))]` (the one
  factory preset; every other preset is a document).
- `#[port(input|output, audio|cv|control|event|video, name, channels = 2|"any", sidechain, names = "music.note", props =
  "pitch|vel", capacity = 64, path = "lead:@preset", mode = "set"|"add"|"mul", choices = "a|b", doc)]`; every port gets
  `Self::<NAME>`, its index in its group (audio, control, event per direction).
- `#[param(range, default, curve = log, unit = "Hz", choices = "saw|pulse", doc)]` on `f32` fields (choices: the value is the index).
- `Block`: `audio_in(p) -> Planar` / `audio_out(p) -> PlanarMut` (`ch()`, `chan(c)`, `stereo()`), `events_in(p) -> &[Ev]`
  (sorted by frame; notes on / off paired by id, Hz and key both filled by the host), `control_in(i)`, `set(i, v)`,
  `emit(port, Ev)`, `set_silent(p)`, `audio_out_with_events`. `Outputs` (control domain): `set`, `emit`. `Ctx`: times
  (`frame`, `show_ms`, `tempo`, `beats`, `bar`), `seed`, `character()` (+ mood index, energy / touch / strain), `var`, sensors
  (`motion()`, `position()`, `face()`, `camera_motion()` with a `Status`), `asset`, and the inputs `control_in` / `events_in`.
- Parameters are a table the host writes (clamped, NaN ignored); the dirty ones are copied into the fields before a call and
  `Module::params_changed(dirty)` follows. `Smooth` and `deck_module::math` (libm: the same bits native and wasm) for the DSP;
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

## Tests

`testing::Rig<M>` runs a module natively through the same `rt` code the exports call – strict, as the build service's tests
and the creator's Play: install `deck_module::alloc::Counting` as the test binary's `#[global_allocator]` and a call that
allocates fails the test. Prove a module against a built-in the way `modules/walk/tests/builtin.rs` does, and its wasm
against its native run the way `modules/walk/tests/wasm.rs` and `modules/analog/tests/wasm.rs` do (native ≡ wasm).

## Build

`cargo build --release --target wasm32-unknown-unknown -p <module>` → `target/wasm32-unknown-unknown/release/<module>.wasm`.
No `build.rs`, no proc-macro of your own, no dependency beyond this crate (the compile service writes `Cargo.toml` itself).
Clippy takes the repeated `#[port(…)]` lines of one struct for duplicates: put `#![allow(clippy::duplicated_attributes)]` at the top of the module crate.

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
  instances in the show mode. Node params besides the module's own: `seed` (default 1), `character`, `persona`, `mood`,
  `vars` (table key → number). Video ports are refused with the reason (#185).
- **browser**: the TS module host of ABI v1 is deleted (#343); the browser's engine (live-core) takes modules through the
  same `deck_engine::register_module` path once deck-module-host has its web backend (#290).
