# deck-module – the module SDK and its ABI (v1)

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

## What a module sees and may do

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

- **deck** (deck-native, feature `modules`, `deck/native/src/module_host.rs`): wasmtime. `DECK_MODULES=<folder of .wasm>` is loaded before a scene is built and each module becomes the node kind `module:<name>` (ports from the descriptor: audio / control inputs as declared, then every parameter as a control input; audio outputs). Compile cached in memory and, with `DECK_MODULE_CACHE=<dir>`, on disk by SHA-256. A trap or a time-budget overrun (≈ 0.75 × the block, 2–50 ms, wasmtime epoch interruption) silences that node and logs it; memory capped at 64 MiB; one import (`env.deck_log`, 10 lines burst, then 10 a second). Node params besides the module's own: `seed` (default 1), `character`, `persona`, `mood`, `vars` (table key → number). Not yet wired: control outputs and video ports (the host refuses such a module with the reason), events out (emitted, readable by `Instance::events`, nothing consumes them yet).
- **browser** (`shared/live-core/src/modules.ts`): the same `.wasm` in the browser's engine, V8 in the tests; `module-abi.ts` is generated from this crate (`UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts`). A browser has no epoch budget: a runaway loop is the worker's to terminate.
