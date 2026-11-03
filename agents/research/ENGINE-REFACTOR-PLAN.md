Draft for owner review, 2026-11-02 – the plan for converging the deck onto one engine. Nothing in it is built; the phases need the owner's go (and the decisions of §8) before the first worker starts. Revised with the owner 2026-11-02: channels and routing (§2.6, decided), fundsp and the library survey (§3.1, §3.2). **Built so far: P0–P3b, P4 (#317, branch `feature/317-engine-move`, not merged) – §4 P4 says what moved where – and P4b (#318, branch `feature/318-channels-routing`, not merged) – §4 P4b says what is built and where it differs from §2.6.**

# ENGINE-REFACTOR-PLAN – one engine in the universal core

Owner 2026-11-02: "deck-core is everything. with built I mean only lightweight adapters for other browser services and
the crate compiler target. really need to stick to one universal core." · on the native deck running its own audio engine:
"this can not be and is a fundamental design flaw" · "the tempo clock should also be part of the deck engine" ·
"browser, UI, no business logic." · "module extensions are all in rust and will have to be universal" · "rigs are
module groups" · "batchgraph can document their schema in rust structs are easy to export their schema to other
languages and an easy json can be provided to explain the api".

These decisions are binding and are not argued again here (DECISIONS 2026-11-02, research/ENGINE-ALIGNMENT.md §0).
This plan covers **how** to carry them out: the order, the checks, the risks and the few choices that are still open.
It reads the code itself and does not rely on the report. Where the code and the report disagree, §1.4 says so.

## 1. Verified starting point (code read 2026-11-02, `main` at b497a65f)

### 1.1 The code that matters (lines of code / lines of tests)
| Where | Code / tests | What it is | Who runs it |
|---|---|---|---|
| `deck/core` (deck-core) | 2,542 total | math, `F8` lanes, smooth `Ramp`, limiter, FFT `Analyzer`, `TempoMap` (210 + tests), **three evaluators**: `ControlGraph` (the #182 patcher reference), `BatchGraph` (418 / 143), `synth.rs` blocks; three kind tables (`module.rs` `KIND_TABLES`, `batch.rs` `K`, `stage.rs` `STAGE_MODULES`) | everything (foundations); `BatchGraph` only live-core and deck-space's `lower()` |
| `deck/native/src/audio` | 4,081 total: **engine ≈ 1,960** (mod 500, blocks 428, voices 248, dispatch 241, walk 194, sequencer 151, module 116, click 80) · **device + diagnostics ≈ 1,320** (backend 717, glitch 262, ring 196, priority 92, lead 49) · tests ≈ 800 | the native deck's own engine: level-ordered steps over a stereo buffer pool, audio-rate links, `F8` voice batches (AVX2 copy), fundsp blocks, envelopes, cv (pitch L / gate R) | native deck |
| `deck/native/src/control.rs` | 362 / 205 | **a third duplicate, not in the report:** the native deck's control domain (LFO sine / saw / square, the tempo node, FFT → control), stepped once per **video frame** on the frame thread, handed to audio through atomic `Param`s | native deck |
| `deck/native/src/tempo/` | mod 418 / 128, map 73, midi 477, osc 139, link 68 | the clock controller (sources, priority, tap, quantized start, seqlock) and the sources | native deck |
| `deck/native/src/module_host.rs` + `audio/module.rs` | 576 + 116 | wasmtime module host + the module node | native deck, feature `modules` only |
| `deck/live-core` | 1,492 (core 575, walks 247, signals 238, wire 233) | the browser's engine: `BatchGraph` + walks driving a voice's `freq` + limiter + analyzer + outputs, raw wasm exports | browser worker |
| `deck/space` (deck-space) | 12,900 (rig/ ≈ 1,400 code + 644 tests; graph 419, params 151, scene 198, modreg 124) | scene model, the `Kind` table, `Spec` ranges, rigs (`play`, `flatten`, `lower`, checks), the being | stage (model, being), space-web (rigs, being) |
| `deck/events`, `deck/outputs` | 2,560, 2,474 | generators / patches / routing; the outputs engine | both (already one code base) |
| `deck/module` (+ macros) | 1,045 + 290 | the SDK and ABI v1 (`Io` with audio, control in **and control out**, events, vars, tempo, character) | modules, both hosts |
| `deck/build` | 1,875 | compile service (#281); not deployed (owner's bunny step open) | server |
| `shared/live-core/src` | 1,276 TS | worker, worklet, rings, `lead.ts`, `modules.ts` (346, **imported by its test only**) | browser |
| `shared/live-deck/src` | 2,100 TS | `deck.ts` melody via `Math.random`, `noise.ts` (Web Audio + `Math.random`), `scene.ts` hand twin of `Spec` ranges, `space.ts:416` a **fixed BPM** fed to the being | browser |
| `server/src/Domain/Rigs` | RigValidator 704 + generated NodeKinds 38 | second implementation of deck-space's checks | API |

### 1.2 What plays where today
- **The native deck plays scenes, not rigs** (`main.rs:9`: "the stage plays scenes, not rigs, yet"). Rigs are played
  only in the browser: space-web `rig_play` → `Rig::flatten` → `lower()` → live-core. "Rigs are universal" is not true
  yet on the deck that matters most for a show.
- **Control rate differs:** the browser steps its control graph once per 128-frame block (`engine.ts` `BLOCK`), the
  native deck once per video frame (~60 Hz). Any LFO-driven sound differs by design, before any DSP difference.
- **Block segmentation leaks into the sound:** `Ramp::block` glides per call, and live-core splits its render at every
  walk note – the output depends on where blocks are cut. The native deck's tests compare builds with the same cut pattern
  only.
- **Clock sync is missing:** the relay answers `ping {t0}` with `pong {t0, t1}`, but no client computes an offset (the
  TS client sends `t0: 0`). The relay's `at` (ms) cannot be placed on a local audio frame today.
- **CI gap:** `npm run deck:ci` runs `cargo test --workspace` with default features; the native deck's `modules` feature is
  not default, so `module_host.rs` and `tests/modules.rs` are never built or tested by it.
- **wasm:** `live_core.wasm` is 706 KB (249 KB gz), built **without simd128**; space-web 3.6 MB (1.2 MB gz). The lab's
  Chromium numbers: 16 FM voices × 128 frames – patcher 49.8 µs, BatchGraph without simd **64.0 µs (slower)**, with
  simd128 29.4 µs. Budget per 128-frame block: 2,667 µs.
- **Allocation:** the native deck's render is proven allocation-free (`tests/no_alloc.rs`). live-core's render calls
  `resize`, `extend` and `sort_unstable` on its hot path and has no such test.

### 1.3 Node kinds – what each engine does today (read from code; P0 measures the numbers)
| Kind | Native deck | Browser (live-core after `lower()`) | Parity expected |
|---|---|---|---|
| `osc` sine | `F8` batch, envelope (5 ms attack), start phase = fundsp's `rnd1(0)`, cv in | `BatchGraph` osc, no envelope, start phase 0, no cv | close, not equal (phase, envelope) |
| `osc` saw / square / triangle | fundsp wavetable units | **`wave` is dropped by `lower()` – plays a sine** | wrong |
| `fm` | `F8` batch, envelope, cv | `BatchGraph` fm ("synth.rs's Fm, bit for bit"), no envelope / cv | close; bit-equal once phase and envelope agree |
| `lfo` | `control.rs`, three shapes, per video frame | sine only (`shape` dropped), per block | differs (rate, shape) |
| `smooth` · `mul` · `add` | – (not in deck-space's `Kind`) | batch kinds used by `lower()` (strip gains) | browser-internal |
| `noise` (white / pink + lowpass) | fundsp `noise` / `pink` / `lowpass` | TS page node, Web Audio, `Math.random` | missing in the engine |
| `seq` | block, tempo clock or own bpm, cv | page node, not played | missing |
| `click` | sample-accurate tick on the tempo map | – (no clock) | missing |
| `walk` · `events` | deck-events on the sequencer thread → cv (pitch, gate) + envelope | live-core `walks.rs` → `send` on the voice's `freq`, own bpm, no gate | differs (mechanism, clock) |
| `chapter_music` | symphonia-decoded track, resampled | – | missing |
| `mixer` | 4 stereo inputs, gains | folded into `mul`s, **mono** main output | differs (stereo) |
| `fft` | analyses its input (a tap), frame thread, heard window | analyses the limited main output | differs (where) |
| `audio_out` + limiter | stereo, main gain ramp, deck-core limiter | mono, deck-core limiter | close for mono material |
| `module:*` | wasmtime (feature `modules`) | pushed to page nodes, not played | missing |
| `tempo` (control) | the clock's outputs | – | missing |

### 1.4 Corrections to ENGINE-ALIGNMENT.md
1. **"glitch" is not a node kind.** `audio/glitch.rs` is the device glitch monitor (#208: underrun, long render, late
   callback) – platform diagnostics, it stays with the adapter. #309's and #310's kind lists name it wrongly.
2. **The native deck's control domain (`control.rs`) is a third duplicate** of the batch model, missing from §2 of the report.
3. **"~4,081 lines vs 561"** compares unlike things: ≈ 1,320 of the 4,081 are device I/O and diagnostics that stay, and
   ≈ 800 are tests; the engine logic to converge is ≈ 1,960 (audio) + 362 (control) lines against `BatchGraph`'s 418
   (+ live-core's ≈ 600 around it). The **native deck's engine is the bigger and more capable one** (stereo, audio-rate links,
   cv, envelopes, `F8` + AVX2, no-alloc proof).
4. **The browser does not play `osc` / `fm` at parity:** `lower()` keeps only a kind's ports, so `wave`, `shape`,
   `attack` / `release` vanish silently.
5. **"Picture side: genuinely one code base"** holds for deck-space (the being, sets, rigs) only. The native deck's video graph
   (`render.rs` 1,036, shader / chapter / overlay / outlet nodes, `live/` ≈ 1,500, `text.rs`) is native-only. Out of
   this plan; it belongs on #311 as a later leg.
6. **The native deck does not play rigs at all** (§1.2); the report's open question "how the native deck plays rigs natively" has
   the answer "not yet".
7. **Module control outputs need no ABI change:** ABI v1 already has `MAX_CONTROL_OUT`, `Out.control` and the SDK's
   `set_control`; only the host refuses them. #289 is host work.
8. **Not mentioned:** the control-rate difference, the block-segmentation dependence, the missing clock-offset
   estimate, the CI gap for `modules`, simd128 being off, live-core's unproven no-alloc, the TS fixed BPM in `space.ts`.
9. **Kind metadata exists eight times:** deck-space `Kind` (ports, types), deck-space `params.rs` `Spec` (range, curve,
   default), deck-core `KIND_TABLES`, `batch.rs` `K`, `stage.rs` `STAGE_MODULES` (feature per kind), the generated
   `NodeKinds.php`, the TS `scene.ts` `Spec` twin, and – for modules – `deck_module::host::Descriptor` (+ the TS
   `modules.ts` types). No schema derive exists (no `schemars` in the workspace); the PHP file is written by a test
   (`UPDATE_FIXTURES=1 cargo test -p deck-space node_kinds`).
10. **Everything is stereo or mono by construction.** The native deck's pool is `Stereo { l, r }` per port, the browser's
    main output is mono, the module ABI's audio ports are fixed `[[f32; 1024]; 2]`, the limiter is stereo-linked, the SAB ring
    is interleaved stereo. The owner wants more than stereo ("the engine is supposed to be able to do even more than
    stereo"), so the stereo pool cannot be the shared engine's model (§2.6).
11. **How fundsp is really used:** six units (saw, square, triangle, white noise, pink noise, the noise's lowpass)
    through `Box<dyn AudioUnit>` with **`tick()` per sample** (a virtual call per sample per node, `blocks.rs:196`),
    plus one constant (`rnd1(0)`, the sines' start phase, `voices.rs:51`); the patcher build (the tests' reference) also
    uses its `sine`. None of its graph notation, `Net` or block processing. The native deck pins 0.20; 0.21 and 0.22 broke the
    API, the current release is 0.23 (§3.1).

## 2. Target architecture

### 2.0 Names, and no backwards compatibility (owner 2026-11-02)
Owner: "the native apps name is stupid, it is deck, and stage is a part of it."
| Term | Means |
|---|---|
| **deck** | a device running the core: the **native deck** (the app – crate `deck-native`, binary `deck`, in `deck/native/`; renamed by #316 before P4) or the **browser deck** (/live/deck: live-core + the page) |
| **stage** | the part of a deck that performs: the playing scenes, the stage environment, the picture |
| **stage environment** | the named layer for the room a show plays in ("club", "open air", "school hall"), for every outlet kind – sound, screens, lights, MIDI, OSC; it outlasts scene switches (§2.6b) |
| **outlet** | a logical output a scene writes to: an audio group ("main", "cue"), a screen, a light role, a MIDI part, an OSC address (LIVE.md B.5) |
| **deck configuration** | one deck's hardware binding, a saved document: channel map and fold, safety settings, quality, decoding, disk, controller map (§2.6b) |
| **venue** | a room in one selection: its stage environment plus a deck configuration per deck (§2.6b) |
| **deck settings** | performance and external hardware, per deck instance (older DECISIONS lines say "stage settings") |
| **folder bus** | a bus that sums its children (§2.6) |
| **main** | the sum of an output group (the `main` group, a main bus) |

**No master / slave wording** (owner 2026-11-02: "can we call it main, generally or something that doesnt use master
slave relation ships? in wording."): "main" for a sum, "stage environment" for the per-venue layer, "original" for an
uploaded source file, "leader / follower" where a relation needs words.

**No backwards compatibility** (owner: "we dont need backwards compatibility for the audio engine, as we have not
released anything. all work is exploratory"): no A/B flags (`DECK_DSP`, `DECK_CONTROL` are gone from this plan); the
six fundsp units are rewritten during the engine move (P7 folded into P4); today's sound is no gate – P0's hashes
remain a signal that something changed, never a block on an intended change; ABI v2 replaces v1; stored rig /
instrument / module documents are migrated once or recreated, no compatibility code. What stays a contract: native ≡
wasm bit for bit (§3.4).

### 2.1 The crates after the refactor
| Crate | Holds | Builds for |
|---|---|---|
| **deck-core** | foundations: math, `F8` lanes, `Ramp`, limiter, FFT analyzer, the DSP units (band-limited oscillators, noise, SVF), **the tempo clock** (map + controller + relay-sync filter), the lead rule | native, wasm32 |
| **deck-space** (no features) | the scene / instrument / rig model, **the kind catalog** (§2.4), rig logic (`play`, `flatten`, checks, later `bake`) | native, wasm32, build service |
| **deck-events**, **deck-outputs** | generators, patches, routing; the outputs engine (unchanged) | native, wasm32 |
| **deck-module** | SDK + ABI (v1, later v2) + **the one descriptor type** (built-ins and modules) | wasm32 (modules), native (hosts) |
| **deck-module-host** (new, #305) | descriptor / layout, `Io` staging, log limiter, budget, `ModuleStats`, `Backend` trait; backends: `wasmtime` (feature, native), `web` or `wasmi` (feature, wasm – #306 decides) | native, wasm32 |
| **deck-engine** (new) | **the one engine**: graph build, control half (block rate), audio half (**N-channel planar buffers, multichannel links with channel maps**, `F8` batches, blocks), every kind, routing (nodes, buses, sends, `out` nodes – §2.6), the in-block event host, the module node, analysis, `send` / `set_base` / paths table | native (baseline, AVX2 copy), wasm32 (+simd128) |
| **deck-media** (new) | assets and media (§2.9): the derivative choice by capability, load-ahead, demux (MP4 / MOV / WebM), audio decoding (symphonia), video decoding (real HAP; AV1 by rav1d), the `DecodeBackend` trait (software always, hardware decode-only adapters), the frame queue, the frame for the heard time | native, wasm32 – its own wasm, loaded on need in the media worker |
| deck-native (today `deck-native`) | adapter + app: device backends (cpal, PipeWire, ASIO), render thread + ring, priority, glitch monitor, time base (`clock.rs`), tempo sources (Link, MIDI, OSC), the sequencer thread as a scheduling wrapper, window / GPU, video graph (later leg), sync, files, the asset cache on disk, hardware decoders (VideoToolbox / MediaCodec / VA-API) | native |
| live-core | adapter: the worker's raw exports (inbox / outbox, pointers), nothing else; its TS side: worker, rings, worklet, `fetch`, the WebCodecs `VideoDecoder` adapter in the media worker | wasm32 |
| space-web | adapter: wasm-bindgen entry for the being and rig play | wasm32 |
| scutoons-build | compile, **validate** (deck-space checks), later bake | native (server) |

Why a new crate and not "inside deck-core": deck-events depends on deck-core (`TempoMap`), and the engine hosts
deck-events' generators – an engine inside deck-core would be a dependency cycle. The owner's rule names "deck-core and
the crates it is built from"; deck-engine is one of them. (The alternative – folding deck-events into deck-core – moves
2,560 lines for no gain.)

### 2.2 The picture
```
            ┌──────────────────────── the universal core (Rust, one source) ─────────────────────────┐
            │ deck-core: math · lanes · DSP units · limiter · analyzer · TEMPO CLOCK · sync filter  │
            │ deck-space: scene/rig model · KIND CATALOG · rig play/flatten/check/bake              │
            │ deck-events · deck-outputs · deck-module (SDK, ABI, Descriptor) · deck-module-host     │
            │ deck-engine: THE ENGINE = control half + audio half + event host + module node        │
            └───────────────┬───────────────────────┬──────────────────────────┬───────────────────┘
          native target     │        wasm32 (worker) │       wasm32 (page)       │  native (server)
   ┌────────────────────────┴───┐  ┌────────────────┴────────┐  ┌──────────────┴───┐  ┌────────────┴────────┐
   │ deck-native (adapters)     │  │ live-core (exports)     │  │ space-web        │  │ scutoons-build      │
   │ cpal/PipeWire/ASIO → ring  │  │ TS worker → SAB ring →  │  │ wgpu WebGPU/GL2  │  │ /compile /validate  │
   │ Link · MIDI · OSC sources  │  │   AudioWorklet          │  │ rig_play (data)  │  │ /bake · kinds.json  │
   │ time base clock.rs         │  │ getOutputTimestamp      │  │ pointer, touch   │  └────────────┬────────┘
   │ wasmtime backend           │  │ web|wasmi backend       │  └──────────────────┘   PHP API forwards,
   │ window/GPU · files · DMX   │  │ relay bytes, Web MIDI,  │                         serves kinds.json
   └────────────────────────────┘  │ sensors (on need, #312) │
                                   └─────────────────────────┘
   TypeScript / Svelte: UI and authoring state only (creator editor, deck page) – reads kinds.json, never decides
```

### 2.3 Where each responsibility lives
| Concern | Lives in | Adapter part (platform) |
|---|---|---|
| what plays, kinds, DSP, mixing, limiter | deck-engine / deck-core | device callback, ring, worklet |
| control domain | deck-engine, stepped **per 128-frame engine block** | the picture reads a snapshot (by heard frame) |
| tempo clock: map, controller, source priority, tap, quantize | deck-core `tempo` | sources' I/O: Link, MIDI clock, OSC, relay socket, Web MIDI clock |
| show clock: relay offset filter, anchor → frame | deck-core `tempo::sync` | sending ping, passing pong / tempo bytes, time-base stamps |
| module hosting, budget, stats | deck-module-host | the VM (wasmtime, or the browser's WebAssembly / wasmi) |
| rig play, flatten, checks, bake | deck-space | – |
| validation of documents | deck-space via scutoons-build `/validate` | PHP forwards; the creator shows the answer |
| events, routing | deck-events (+ engine's in-block host) | the native deck's sequencer / dispatcher threads (scheduling only) |
| lead rule (when to grow / shrink) | deck-core (from `lead.rs`; `lead.ts` goes) | measuring underruns |
| kind metadata and its exports | deck-space catalog + deck-module `Descriptor` | – |
| time: sample time, musical time, show time (§2.8) | deck-core `tempo` + deck-engine | device stamps, relay bytes |
| assets and media: which derivative, what to load when, demux, decode, which frame shows | deck-media | fetching bytes, the disk cache, hardware decode-only adapters |

### 2.4 The kind catalog – one descriptor for built-ins and modules (owner 2026-11-02)
The node kinds, their ports and parameters and the graph documents are **plain Rust types that derive their schema**;
everything else is exported from them.
- **One type, `deck_module::Descriptor` (v2 shape):** `name`, `version`, `domain`, `doc`, `feature` (the build module
  that carries it), `scene_only`, `needs` (capabilities, #312), `abi` (modules only), `ports[]` {`name`, `dir`,
  `type` audio · control · cv · video · event, for audio its `channels` (a count or `any`, §2.6), `doc`}, `params[]` {`name`, `min`, `max`, `default`, `curve` lin · log,
  `unit` (Hz, s, dB, ratio …), `choices` (for `wave`, `shape`), `doc`}. Fields are `Cow<'static, …>`, so the built-in
  table is a `const` and a module's descriptor deserializes into the same type. A module and a built-in kind are then
  the same thing to the engine, the validator, the editor and the agents – RIG-EDITING §3's "modules add kinds that
  come from the library" becomes literally true.
- **The built-in catalog** lives in deck-space next to `Kind` and replaces `Kind::inputs/outputs`, `params.rs` `Spec`,
  deck-core's `KIND_TABLES`, `batch.rs` `K` and `stage.rs` `STAGE_MODULES` (five tables → one). deck-engine has a test
  that every catalog kind has an implementation and every implementation a catalog entry. Engine defaults
  (`default_for`) move into the catalog – one place for "what is the default gain".
- **The graph documents** (instrument, preset, rig, scene) are serde structs in deck-space already; they get the same
  derive.
- **Exports, one generator** (`cargo run -p deck-space --features schema --bin deck-schema`, and `--check` in deck:ci
  the way `generate.ts --check` guards the protocol):
  (a) **JSON Schema** of `Descriptor` and of the documents (`schemars`) – for other languages and the creator's forms;
  (b) **`kinds.json`** – the catalog as data with its doc strings, the API that explains itself: served by the API
  (built-ins + the descriptors of published `module:<id>` documents, merged), read by the editor, the Nodes tab, the
  Parameters tab, the SCUTOONS MCP (a read tool) and agents;
  (c) **TS types** for `Descriptor` and the documents, written by the existing `deck/protocol/generate.ts` extended to
  this schema (it already refuses constructs it does not know – the safe property);
  (d) **`NodeKinds.php` is generated from the catalog** until #313 retires it with `RigValidator.php` – then the PHP
  holds no kind knowledge at all.
- **The module section** (`deck.module`) carries the same JSON shape; the SDK macro writes it; ABI v1 descriptors
  (without the new fields) still parse (serde defaults).
- **Catches, stated:** (1) generated files drift – each is checked in and `--check`ed in deck:ci, stale = red; (2)
  versioning – the catalog carries `catalog_version`; adding kinds / ports / params is additive, a rename or removal is a
  stored-format change (RULES "Migrations and stored data": analyse, migrate the documents, then the code) and bumps it;
  modules keep their `abi`; the descriptor type itself sits in deck-module behind a small feature (serde only), so a
  module's own wasm carries no schema code; (3) two codegen directions in one repo – the protocol stays schema-first (`schema.json` →
  Rust / TS), the kinds become Rust-first; flipping the protocol is not part of this plan; (4) `schemars` must not reach
  the wasm builds or the module SDK's wasm – the derive sits behind a `schema` feature; (5) doc strings must be data
  (`doc` fields), not Rust doc comments; labels in de / en stay on instruments' exposed knobs (RIG-EDITING §2), not on
  kinds; (6) module kinds are only known at run time – `kinds.json` from the API is built-ins + published modules,
  the generated file is built-ins only.

### 2.5 Build matrix
| Build | Crates | Notes |
|---|---|---|
| native deck, x86-64 baseline | all + wasmtime backend | AVX2 loops chosen at start (as today) |
| native deck, x86-64-v3, aarch64 | same | aarch64 cross on GitHub by hand |
| live-core, wasm32 + simd128 | deck-core, deck-space (no features), deck-events, deck-outputs, deck-module-host (web/wasmi), deck-engine | size gate ≤ 500 KB gz (owner 2026-11-02; 355 measured in P0); no fundsp, no symphonia, no schemars |
| space-web, wasm32 | deck-space (gpu) | unchanged |
| scutoons-build | deck-module, deck-space | gains `/validate`, later `/bake` |
| deck-media, wasm32 + simd128 (+ threads) | deck-media | its own wasm, loaded on need in the media worker; its gate is set by the media probe (§3.8) |
| conformance (tests) | deck-engine, live-core, wasmtime (dev) | runs `live_core.wasm` under wasmtime to compare targets bit for bit |

### 2.6 Channels and routing – Max's nodes, Reaper's buses (owner 2026-11-02)
Owner: "the engine is supposed to be able to do even more than stereo" · "between the audio routing in max msp which
is to what ever i pick as an output node, and all routing before is modulare based on nodes, and routing in reaper,
where everything is basically a bus." · "ambisonics is not so important at the moment we can build an ambisonic adapter
on a fundamental modular system anyways. by injecting ambisonic panners and mapping them to outputs."
The stereo pool goes; a channel is a channel, stereo is two of them. Max and Reaper become one model once a bus is a node.
- **Nodes (Max):** everything before the output is the modular graph. A link carries n channels (Max's `mc.` cord);
  several links into one input are summed; a port declared `channels = any` takes the widest linked input's count.
- **`out` nodes (Max's `dac~ 3 4`):** an `out` lists the device channels it writes (`[1,2]`, `[3..8]`) or a named
  group (`main`, `cue`); a graph has as many as it wants, anywhere. **Which device channels exist and their names are
  deck configuration** (the deck settings' external hardware), not the scene: a phone has `[1,2]`, the X1 with an
  interface many. An `out` naming channels the device lacks is folded by a deck setting (drop, or the speaker
  down-mix of its layout) – the core decides, the adapter only reports the count.
- **Buses (Reaper):** `bus` is one node kind: n channels, gain, mute, solo, pan, an insert chain (a sub-graph),
  **sends** to any other bus (pre / post fader, with a channel map), and children – a **folder bus** sums its children (the owner's term). Every
  instrument's out is a bus, every rig track is a bus, a rig is a folder bus of tracks ("rigs are module groups"). To the
  engine a bus is a node; the mixer view is the editor's.
- **Channel maps on links** (Reaper's send offsets "1/2 → 3/4" and pin connector): a link or send may carry
  {source channels → target channels}, identity by default (stereo into a 7-channel bus lands on 1–2 unless the
  map says e.g. 1/2 → 3/4); one source channel may go to several targets (a copy), several sources into one target sum. This replaces `split` / `merge` / `matrix` kinds; a
  weighted map is a later extension if a show needs one.
- **Sidechain** (owner 2026-11-02: yes): a kind may declare a second audio input (`sidechain`, its own channel count);
  a send feeds it (usually pre-fader), e.g. a compressor on the pad bus ducking to the kick. Side effects: §2.6a.
- **`in` nodes** (owner 2026-11-02), the mirror of `out`: device input channels (a mic, a line in) anywhere in a graph
  or chain; in the browser a need-basis service like the sensors (#312).
- **Chains** (owner 2026-11-02: "i will design both parallel chains and serial chains … i just dont want to narrow my
  options"): a track is a bus whose content is a chain – serial (events → instrument → effects, the insert chain),
  instrument into instrument, event chains (deck-events patches), and parallel chains in a folder bus inside the
  track. Selectors between parallel chains (by mood, persona, a knob) are designed later with the modules.
- **Discrete by default:** channel n goes to channel n, as in Max and Reaper; no implicit up / down-mix inside the
  graph. A speaker down-mix (5.1 → stereo, the ITU / Web Audio matrices, tested against the spec's tables) happens only
  where asked: a `downmix` kind (the standard tables or a custom gain matrix, any n → m) inserted anywhere, or – when
  nothing is inserted – the deck's fold at an `out`, so a show never goes silent on a phone.
- **Layouts are labels with positions, as data:** a bus or `out` may carry a layout (`stereo`, `quad`, `5.1`, `7.1`, or
  `speakers [{az, el, dist}]` for a venue's array) – read by panners and the down-mix, never needed for plain routing.
  `pan` is a kind: stereo equal-power; VBAP / DBAP over a layout's positions.
- **Ambisonics, binaural: not engine concepts.** An encoder / panner writes 4 / 9 / 16 channels into a bus, a decoder
  maps them to an `out`'s speakers or to two binaural channels – kinds or Rust modules on this system, built when a show
  asks. No reserved type.
- **Buffers:** one planar pool of 128-frame channel buffers, sized at build from the graph's channel counts; a port
  owns `channels` consecutive buffers; a channel map is an index copy. The `F8` voice batches stay mono sources placed
  by `pan` or a bus. The stereo case must reproduce today's hashes bit for bit (the guard of the change).
- **Loops** only through an explicit `feedback` node (one block, 128 frames = 2.67 ms, like Max's `tapin~` /
  `tapout~`); any other cycle is an error (`levels()`).
- **Safety and analysis:** there is no mandatory main bus. After all `out` nodes are summed per device channel, the
  engine's limiter runs last (linked per named group). Meters per channel, the FFT on a mono sum of what it taps.
- **What platforms give:** native – every device channel (cpal / PipeWire / ASIO: the Fireface's many, HDMI's 8, the
  X1's 2). Browser – the spec requires at least 32 channels for buffers; `destination.maxChannelCount` is the device's
  (Safari 14.1–14.5 reported 0); the worklet takes `outputChannelCount: [n]` and the SAB ring becomes n-channel; phones
  are stereo in practice; desktop Chrome on Windows with a multichannel interface is unverified (§3.8, owner 10 min).
- **Modules and instruments declare channels:** a module's audio port says `channels = n | any` in its descriptor; an
  instrument's out is a bus with its count; at most 32 channels per port. ABI v1's fixed stereo ports are one reason
  for ABI v2 (§3.6).

### 2.6a Side effects of sends, sidechains and chains
- **Loops:** the core rejects a loop and names it (the editor highlights it as a hint); a loop on purpose goes through
  `feedback` (one block late).
- **Latency** (owner 2026-11-02: "automatically with the live flag, for rigs with live input. but we will see if that
  is actually needed"): every kind and module declares its `latency` (frames) in its descriptor; at build the engine
  delays the shorter parallel paths (sidechains too), fixed at build, no allocation while playing; a rig or `out` group
  marked `live` skips it. Built only when a kind with latency exists.
- **Mute / solo** gate a bus's link to its folder bus or `out`, never its computation – a muted kick still ducks
  through its pre-fader send.
- **Sums in a fixed order** (the links' document order) – float addition is not associative; bit-exact needs it.
- **Silence flags** keep a computed-but-silent source cheap.
- **Control side effects** (a module's control outputs, events, a rig's code driving its tracks) land at the next
  block – the same on every target; a control loop is always one block late.

### 2.6b Outlets, stage environment, deck configuration, venue – fitting a show to the room (owner 2026-11-02)
Owner: "it can be assigned by the show runner, or by any conductor or by the invidual deck. so it can trickle down, be
rejected or override under certain conditions." · "if i want to orchestrate many devices in a room without having to go
to each one, assigning deck channel maps, and engines safety by selecting the saved configuration for that deck, can be
of quite some help." · "this is audio specific, the stage environment. but it can also have video or midi or dmx or osc
out put" · "good suggestions, this way we can fix our shows depending on the venue".
```
scene graph ── outlets: audio groups "main" / "cue", screens, light roles, MIDI parts, OSC addresses
                 │   during a scene switch the old and the new scene both feed in and crossfade
                 ▼
stage environment   the room, a named document: audio – shaping, speaker layout, speaker EQ and delays; video – screen
                    layout, projection mapping, colour; lights – fixture patch, dimmer curves; MIDI / OSC – remaps
                 ▼
deck configuration  this device, a named document: channel map and fold, displays, DMX interface and universes,
                    MIDI / OSC ports, safety settings, quality, decoding, cache disk, controller map
                 ▼
engine safety       per kind, always on: audio limiter per channel; light strobe-rate cap, max intensity, moving-head
                    speed; MIDI all-notes-off on panic, rate limits
                 ▼
devices
```
- **Scenes write to outlets, never to hardware.** The same show plays in any room; the room is fitted by the layers
  below. A deck with fewer channels than a scene's audio: a `downmix` the creator inserts (artistic, e.g. a parallel
  chain chosen by the channel count), else the fold of the deck configuration (§2.6).
- **The stage environment** is the room, for every outlet kind. It stays loaded while scenes come and go (tails,
  compressor state, fixture states carry on); a scene may send it parameters (a bigger reverb in the cave), never
  replace it. Variants are derivatives (§2.7): "Club X – quiet" inherits Club X's speaker correction and fixture
  patch and changes only what differs. It describes all of the room's devices; each deck renders only what its
  configuration binds.
- **The deck configuration** is one device's hardware binding, a saved document with a name ("Fireface rack 8 ch",
  "X1 kiosk", "phone") – what DECISIONS 2026-11-02 calls the deck's one "configuration", now assignable remotely.
- **Engine safety is not a layer anyone removes:** it is built into the engine and runs last per kind; a configuration
  may set it tighter (a lower ceiling), never off or past the engine's hard limits.
- **The venue** bundles a room: its stage environment plus a deck configuration per deck (by deck id or role). It is
  its own document that shows reference (a room serves many shows; LIVE.md B.5's `venues` inside the show become
  references). Picking a venue sets up sound, screens, lights and devices in one step.
- **Assignment cascades down the relay tree:** the show runner (the show's default and optional allowed list) → any
  conductor (for its subtree, a group or one deck; nested conductors again) → the deck itself (its own pick in deck
  settings). A message `{ what: environment | configuration | venue, id, scope, from, lock?, at }` travels only to
  subscribed decks; `at` is a show-time point (next bar, next cue), so a scope switches together, crossfaded.
- **Each deck resolves it in its core, the same everywhere:** (1) it may always reject for capability (too few
  channels, no DMX interface, over budget, a module it can't run) and keeps what it runs; (2) a lock from above beats
  every level below it; (3) otherwise the nearest assignment wins (deck over conductor, a closer conductor over a
  farther one, a conductor over the show); (4) nothing assigned anywhere → no stage environment, outlets go straight to
  the deck configuration. A deck's own lock (a fixed installation) refuses remote changes.
- **Every deck reports up** who it is (deck id, name), what it can do (channels, displays, GPU formats, decoders, DMX,
  MIDI) and what it runs and why ("Club X, from conductor B" · "rejected Open air: 2 channels" · "own pick"); the
  conductor view shows the whole room. Conditions (scope, group, role, lock, allowed list) are data, like the relay's
  forwarding conditions.
- **Scope of this plan:** the audio parts are built here (P4b, the stage environment's audio graph, the channel map,
  the limiter); screens, lights, MIDI and OSC follow the same shape in the Live leg (LIVE.md, `deck-outputs`; §9 create
  17).
- Cost: during a crossfade two scene graphs run at once – a short CPU peak.

### 2.7 Library, derivatives, baking
Owner: "i can design and create derivatives of rig and instruments, any of these modules in any combination and bake
them into the app for runtime. if needed. eventually i will end up with a proper library of modules which can be
reused."
- **Layers:** module (Rust → wasm, a leaf) → instrument (a graph of modules and built-in kinds, exposed knobs) → rig (a
  folder bus of tracks, each a chain). All are catalog documents with versions; a module and a built-in kind are one
  `Descriptor` to the engine, editor and validator (§2.4), so any combination works without special cases.
- **A derivative** names its parent and stores only what it changes (a module in a slot, knob values, an extra track);
  the full graph is resolved at build. Whether it pins the parent's version (an update is an explicit step, the editor
  shows "parent has a newer version") or follows the current version like sets is open (§8 D17; recommended: pin – a
  change to a shared module must not silently change every show).
- **Baking, two levels:** (1) data – a rig or instrument with its derivative layers folded into one flat graph (#296);
  what a show loads; (2) code – a reviewed module compiled into the deck build (native, and into live-core), same
  source, id and descriptor, the conformance suite proving wasm ≡ baked (the owner marks it, DECISIONS 2026-11-02).
- **Bake per show bundle, not into every app build:** a baked bundle is loaded when a show or rig needs it (the
  performance / edit mode rule); only a small reviewed core set becomes built-in for everyone, so live-core stays small.

### 2.8 Time – one clock per deck, seen three ways (owner 2026-11-02)
The native deck stepped its control domain once per video frame (`control.rs`) – the screen's refresh rate drove the
sound; since P5 (#319) the engine steps it per block.
- **Sample time is the truth:** the samples the audio device has played. Audio must never glitch; a video frame can be
  dropped or shown twice unseen – every media player syncs this way. A deck without speakers runs the same counter on
  a timer (the null backend).
- **Musical time** (beat, bar, phase) comes from sample time through the `TempoMap`; **show time** (the relay's ms)
  maps to sample time through the offset filter (§3.7). Device clocks drift a few ppm: music absorbs it with a tempo
  nudge, media re-anchors at the next cue.
- **Modules** read all three from the core per block (sample frame, beat / bar / phase, show time) – never a wall clock
  or a platform API.
- **Video is a stream, not a clock.** A video with sound splits: its audio plays through the engine as a node (process
  it like any audio – effects, FFT, a sidechain; sample-accurate), its frames go to the picture with their media time,
  and the picture shows the frame for the heard time – lip-sync by construction. If its audio passes an effect with
  latency, the engine reports it and the picture shifts by as much (§2.6a).
- **Image → sound:** frame analysis (brightness, motion, face features) enters the core as control input channels
  stamped into sample time (the sensors' path, #312). **Sound → image:** levels and FFT through the snapshot ring by
  heard frame (§3.3).
- **Several screens:** each deck shows the frame of the show time; with HAP every frame is a keyframe, so a late screen
  jumps, never catches up. The show-clock target (±5 ms p95 LAN) is under half a frame at 60 fps; consumer screens'
  refresh phases can still differ by one refresh (only genlock removes that). Drift: drop or repeat a frame, never
  stretch the video.
- **Live input** (camera, mic) is stamped at capture and only roughly alignable – the `live` flag (§2.6a).

### 2.9 Assets and media – the core asks, the adapter fetches (owner 2026-11-02)
Owner: "we will use our own video playback in the deck without browser api to play video. since we keep both worlds
separate for device independence" · "we use the real hap and of course we pre encode all media in our asset library."
- **Documents reference assets by id, never a URL.** With a scene the API delivers an **asset manifest**: every asset
  its graph needs, resolved for its context (character, mood, set), each with its **derivatives** (format, resolution,
  size, hash, CDN URL).
- **All media is pre-encoded in the asset library:** the creator uploads one original (lossless / high bitrate); a media
  job on the server makes the derivatives, like image resolutions: **real HAP** (HAP, HAP Alpha, HAP Q – ffmpeg encodes
  them; HAP R / BC7 support in ffmpeg unverified) for native decks, our one own variant **ASTC frames** for GPUs without
  BC (ARM boards, Android-based decks), **AV1 + H.264** for phones (chosen by the phone's decoder), FLAC / Vorbis / MP3
  for audio, GPU-compressed texture sequences (KTX2 / Basis Universal) for loops and sprites. Encoder licences: AV1
  royalty-free; for H.264 the phone's decoder carries the licence, our encoder needs care (x264 GPL, openh264 BSD).
- **The deck chooses** (deck-media, core logic) by capability: the GPU's texture formats (wgpu reports BC / ASTC /
  ETC2), the hardware decoders (`caps()`), the disk, the quality setting, the network; and what to load when (the show's
  horizon, 256 MB ahead – EVENTS §16). The adapter only fetches bytes; the native deck caches on a disk chosen in deck
  settings (SD card, external SSD – HAP 1080p is ≈ 30–60 MB/s per layer, ≈ 2–4 GB per minute: one layer from a good
  SD card, several layers or 4K from an SSD).
- **Decoding is the core's** (deck-media, both targets – no `<video>`, no `decodeAudioData`): symphonia (audio),
  HAP = Snappy + GPU-texture frames straight to the GPU, rav1d (AV1). The codecs define their output exactly, so the
  goldens need no tolerance for decoded media. Decoded audio lands in a sample pool allocated before, never on the
  audio thread.
- **Hardware decoding, decode-only, built right away** (owner: "i would recommend we built it right away"): a
  `DecodeBackend` trait in deck-media – `caps()`, `configure(stream)`, `submit(packet)`, `poll()` → planes + the media
  time echoed, `reset()`; backends: software (always – the reference and the fallback), WebCodecs `VideoDecoder` in
  the media worker (≈ 100 lines of TS, rings both ways), VideoToolbox / MediaCodec / VA-API natively. The core keeps
  demux, the choice, decode-ahead, seeking, the frame queue and the timing; the adapter decides nothing. Frames come
  back as copied planes (exact; ≈ 90 MB/s at 1080p30); zero copy (external textures) only where the copy measurably
  costs too much (wgpu support unverified). A decoder whose planes differ from software on a test clip is marked and
  not used; a decoder error falls back to software from the next keyframe; deck setting "hardware decoding: auto /
  off". Audio always decodes in software.
- **The GPU does everything after decoding** (wgpu, one source native and browser): YUV → RGB in a shader, decode small
  and upscale (FSR 1, MIT), compositing, effects.
- **Samplers are instruments:** a built-in `sampler` kind (buffer, one-shot / loop / slices, pitch, envelope);
  `chapter_music` is one such player; a drum kit is an instrument of sampler nodes. A module declares asset
  parameters (sample, sequence, image) in its descriptor, the host hands it read-only data (ABI v2); a module never
  fetches. Sequences and MIDI files arrive as bytes and are parsed in the core.
- **Goldens use fixture assets:** small test files with the tests, identified by content hash – a replaced file fails
  as "fixture changed", not as an engine difference; production assets never stand in.

## 3. The hard problems, argued

### 3.1 The base: an audio library, or our own engine? (survey, read 2026-11-02)
The owner asked whether fundsp is the right low-level library and what a more stable base with multichannel and
routing would be. What the core needs from a base: builds native + wasm32 from one source; renders without allocation
or locks; **deterministic** (bit-exact across targets, §3.4); embeds as a graph we build from documents, with exposed
parameters, `send` / `set_base`, cv and audio-rate links; batches voices (the `F8` loops); N channels with routing;
hosts wasm module nodes under a budget; fits the one-`Descriptor` catalog (§2.4).

| Candidate (latest release) | What it is | Channels + routing | wasm | RT-safe / deterministic | Health, licence | Fit |
|---|---|---|---|---|---|---|
| **fundsp** 0.23.0 (2026-01-07) [1][2] | DSP library with an inline combinator notation (`>>`, `&`, `^`), `Net` for runtime graphs | channel count in the type (static nodes), `BufferVec` at run time; `Net` routes any arity | `no_std` + `alloc`; wasm not documented | `allocate()` before RT; std math (libm: native ≠ wasm) | active, essentially one author; 0.21–0.23 in one week of Jan 2026 with breaking renames (`prelude32/64`, noise seeds `u64`, removed phase opcodes); MIT/Apache | its value is the notation, which we would not use; its graph is not ours |
| **Firewheel** 0.14.0 (2026-09-05) [3][4][5] | game audio graph engine (planned as Bevy's default) | per-node `ChannelConfig {inputs, outputs}`, DAG, one-to-many / many-to-one | a WebAssembly backend (README); `no_std` partly | no mutexes on the audio path, silence masks; determinism not stated; "does NOT aim to be a complete DAW engine" | active, MIT/Apache, development follows Bevy's needs | the closest to multichannel routing out of the box – a design reference (channel config per node, silence masks), not our base: its node / parameter model instead of ours, no voice batching, no bit-exact promise |
| **kira** 0.12.6 (2026-10-28) [6][7] | game audio: sounds, mixer tracks, sends, tweens | **stereo only** (`Frame {left, right}`) | yes | game-oriented | very active, MIT/Apache | no: stereo, not a patch graph |
| **knyst** 0.5.1 (2024-11-25) [8][9] | real-time synthesis graph, desktop multithreaded first | flexible | single-threaded mode | sample-accurate scheduling | README: the API "can vary wildly"; 133 recent downloads – stale | no |
| **glicol_synth** 0.13.5 (2024-04-23) [10][11] | live-coding language + engine (dasp_graph roots) | stereo-oriented | yes (AudioWorklet + SAB) | "garbage-collection-free" claim | "highly experimental", no release since 2024; crates.io licence "non-standard" | no |
| **dasp** 0.11.0 (2020-05-29) [12] | PCM primitives (`Sample`, `Frame`, `dasp_graph`) | generic frames | yes | primitives only | frozen since 2020 | nothing we lack |
| **web-audio-api** 1.7.0 (2026-08-08) [13][14] | the W3C Web Audio API in Rust, for non-browser use | the Web Audio model: channel count / mode / interpretation, up / down-mix, 128-frame quantum | experimental only | not stated; std math | active, MIT | no: our routing is discrete by default (§2.6); the down-mix matrices are tested against the spec's tables, no dependency |
| **synfx-dsp** 0.5.6 [17] | DSP units from HexoSynth (PolyBLEP, biquads, …) | units only | – | – | GPL-3.0, needs nightly (`std::simd`) | no: licence and nightly |
| **Faust → Rust** [15] | DSP language that emits Rust | fixed I/O per DSP | via Rust | allocation-free by design; libm FFI can be switched off | mature compiler, Rust backend "WIP"; a second language | no for the core (modules are Rust by decision); a source of reviewed algorithms |
| **Cmajor** (crate 0.7) [16] | JIT DSP language | per processor | no Rust wasm path | – | bindings GPLv3, need the Cmajor shared library | no |
| cpal / rodio | device I/O / playback | – | cpal yes | – | active | I/O only – the native deck already uses cpal; not a base |
| **the native deck's engine → deck-engine** | level-ordered steps, planar pool, `F8` batches, blocks | stereo today → **N-channel links, buses, `out` nodes** (§2.6) | P0 proves it | proven no-alloc; AVX2 ≡ baseline bit for bit; libm-free math | ours, no outside churn | built for exactly this |

**Recommendation: no third-party engine as the base.** The core keeps its own engine (the native deck's, moved into
deck-engine – plan C below), generalised to N channels with the routing model of §2.6 (Max's nodes, Reaper's buses), and
a small set of DSP units written in deck-core (the six fundsp units today, more as kinds need them). Reasons: none of
the candidates is deterministic across native and wasm; the graph, the batching, cv, `send` / `set_base`, the module
node and the catalog are ours in any case – a library would only supply oscillators and filters, a few hundred lines;
the maintained general graphs (Firewheel, kira) are game engines whose non-goals are exactly our goals; the synthesis
graphs (knyst, glicol) are stale. What we take from outside are **specifications, not dependencies**: the speaker
down-mix matrices (ITU / Web Audio) and well-known algorithms (PolyBLEP / wavetables, Simper SVF, Kellet pink, VBAP /
DBAP), with their sources cited in the code. The one dependency allowed is the pure-Rust `libm` crate, for coefficient
math that must give the same bits native and wasm. Faust stays a possible source of reviewed algorithms later (reverbs,
ambisonic decoders): generated Rust checked in, the licence of each Faust library function noted – never a runtime.

Sources (read 2026-11-02): [1] crates.io/crates/fundsp · [2] github.com/SamiPerttu/fundsp (README, CHANGES.md) ·
[3] crates.io/crates/firewheel · [4] github.com/BillyDM/Firewheel (README, DESIGN_DOC.md) · [5] docs.rs/firewheel-core
0.14 `ChannelConfig` · [6] crates.io/crates/kira · [7] docs.rs/kira `Frame` · [8] crates.io/crates/knyst ·
[9] github.com/ErikNatanael/knyst · [10] crates.io/crates/glicol_synth · [11] github.com/chaosprint/glicol ·
[12] crates.io/crates/dasp · [13] crates.io/crates/web-audio-api · [14] github.com/orottier/web-audio-api-rs; W3C Web
Audio API 1.1, w3.org/TR/webaudio-1.1 ("MUST support at least 32 channels", render quantum 128) · [15] faust.grame.fr,
docs.rs/faust-types, the faust(1) man page · [16] docs.rs/crate/cmajor (README) · [17] docs.rs/crate/synfx-dsp (README).
The owner's routing references: Max's `mc.` cords and `dac~`, Reaper's tracks as buses (sends with channel offsets,
folders, pin connector).

**Where the engine's code comes from:**

| Option | For | Against |
|---|---|---|
| A. Grow `BatchGraph` (#310's wording) to every kind | small, clean start | re-writes ≈ 2,300 lines the native deck already has and tests; mono, no audio-rate links, no cv / envelope, slower than the native deck's `F8` voices in wasm without simd; every kind must re-earn the native deck's sound |
| B. Adopt a library engine (fundsp `Net`, Firewheel) | someone else's graph | survey above: not deterministic, different goals, outside API churn, our features re-built on top anyway |
| **C. Move the native deck's engine into deck-engine, merge `BatchGraph`'s control API into it, generalise it to N channels** | the more complete engine becomes the shared one; built on deck-core pieces (`Col`, `levels`, `F8`, `Ramp`, limiter); the move is identity for the native deck (hashes prove it); the browser gains every kind at once | threading seams (Params, seqlock, rtrb) must become platform-free; one large move ticket; the N-channel generalisation is a second step |

**Decision: C** – the result is still "deck-core's graph" in the owner's sense (one engine in the universal core; the
native deck keeps device I/O only); only the *source* of the code changes. `BatchGraph`'s good parts survive: `send` with an
amount (#189 force / blend), `set_base` (#188), `input()`, the `nodes()` paths table, `smooth` / `mul` / `add`. P0
falsifies C cheaply (does the moved engine build for wasm32, does the N-channel pool cost nothing for stereo, is it
bit-exact and fast in a browser); if not, fall back to A with the native deck's `voices.rs` loops ported into it.

### 3.2 fundsp – judged, and what replaces it
Fairly: fundsp is a capable, actively released library (MIT / Apache, `no_std`, a real-time-safe `Net`, any channel
count). Its main asset is the inline notation for writing DSP as expressions – which we would never use, since our
graphs come from documents and our modules are Rust crates. In the native deck it supplies six units (band-limited saw /
square / triangle, white and pink noise, the lowpass behind noise) and one constant (the sines' start phase `rnd1(0)`), driven through `dyn AudioUnit::tick` once per
sample per node – the slowest way to use it. Against keeping it: its API breaks often (0.21 removed the phase
oscillators and changed noise seeds, 0.22 renamed the preludes, the next version changes `Sequencer`) and rests on essentially one author; it uses std math, so native and wasm differ in the last bits; it has
no documented wasm story; it adds a second graph model beside ours. **It does not earn a place – not in the runtime,
and not as a test oracle either.** The six units are written in deck-core (wavetables built at load or PolyBLEP – P0
measures both; seeded xorshift white; Kellet pink; a Simper SVF with deck-core's own `tan`), written during the engine move
(P4) with no switch (§2.0); the owner listens on the X1 afterwards. **The quality bar is analytic, not "sounds like
fundsp":** harmonic levels of a band-limited saw within 0.1 dB of 1/n, aliasing below −60 dB for fundamentals up to
4 kHz, pink slope −3 dB / octave ± 0.5, the lowpass's −3 dB point within 2 % and its Q – each an FFT test in deck-core.
Today's native-deck renders (P0) keep the old sound for comparison by ear. The start phase becomes a literal
constant (today's hashes stay), and the patcher build's sine uses deck-core's `sin_turns`. Quality risk is bounded: six
units, each measurable.

### 3.3 Real-time constraints
- **No allocation, no lock, no wait in the render** – the native deck's `no_alloc` test moves with the engine, and live-core
  gets one (its `resize` / `extend` / `sort` go to preallocated buffers).
- **One engine quantum: 128 frames.** The engine always renders whole 128-frame blocks; the control half steps once per
  block inside the render. The native deck's render thread already runs ahead into a ring (ring.rs), so device buffer sizes
  (ASIO 64, PipeWire 256, 1024) no longer cut the engine's blocks. Result: output independent of the device, ramps
  independent of segmentation, the same cadence as the browser. Latency: unchanged (the adaptive lead governs it).
  Events land on their sample inside a block where a node reads them (click, seq, notes), else at the block start
  (≤ 2.67 ms).
- **Control at block rate** costs ≈ 6× today's control CPU on the native deck (300 nodes 3–8 µs per step → ≈ 0.1–0.3 % of a
  core) – negligible. The picture reads a **snapshot ring** of control values indexed by audio frame (≈ 64 blocks deep,
  preallocated) and picks the heard frame through the time base, so what is seen still matches what is heard.
- **Threads are adapters:** the engine is single-threaded; the native deck may run deck-events' patches on its sequencer
  thread ahead of the render (the same `Core` that runs in-block; the native deck already proves both give the same
  samples).
- **Behaviour changes the owner will hear** (no flags – §2.0; each golden-diffed, the owner listens after): control at block rate; FFT → control from the render (causal) instead of the heard window; in the browser:
  envelopes, stereo, real waveforms.
- **N channels cost what they carry:** the pool is sized at build from the graph's channel counts (allocation at build only);
  a stereo scene renders exactly the buffers it renders today; silent buffers carry a flag so mixers and limiters skip
  them (Firewheel's silence masks are the model). Worst case planned for: 32 channels × 128 frames × 4 B = 16 KB per
  bus – nothing for memory, and a native-deck case only.

### 3.4 Determinism and golden renders
- **Target: bit-exact across native baseline, native AVX2, wasm, wasm + simd128** for everything the engine computes.
  It is plausible: the native deck already proves AVX2 ≡ baseline bit for bit; wasm f32 arithmetic is IEEE; deck-core avoids
  libm. Rules to make it hold: no std `exp` / `sin` / `powf` / `tan` on engine paths (deck-core approximations or the
  pure-Rust `libm` crate) – today `BatchGraph`'s `smooth` calls `exp` per step and `env_coefs` once; no relaxed-SIMD, no
  FMA contraction; every random source seeded (the seed rule; `Math.random` leaves).
- **A golden = scene + event log (frame-stamped) + seed → a hash per target** of the rendered f32 stream; the files hold
  hashes, not audio. No tolerance for decoded media: the core decodes on every target and the codecs define their
  output (§2.9); goldens use fixture assets identified by hash. The DSP units' quality tests (§3.2) are analytic, the speaker down-mix matrices are checked
  against the spec's tables, channel maps and sends by exact sums.
- **How it runs in CI:** the conformance crate renders natively and loads `live_core.wasm` (built earlier in deck:ci)
  under wasmtime – no browser needed. V8 is checked by the owner's browser run once per phase that changes live-core.

### 3.5 wasm size and start-up
Today 249 KB gz for live-core. Expected growth: the engine's kinds + voices + event host + clock ≈ +60–120 KB gz; wasmi,
if chosen, ≤ +150 KB gz (#306 gate). Kept out of live-core: fundsp, schemars, any audio library (§3.1); media decoding is
deck-media, its own wasm loaded on need (§2.9). Wavetables are computed at load (≈ 1–3 ms, not in
the binary). **simd128 on** (`-C target-feature=+simd128` for live-core; all target browsers have it; Safari since
16.4 – older ones are ignored by the owner's rule). Gate per phase: ≤ 500 KB gz, compile + instantiate ≤ 10 ms on the
PC (5.4 ms today).

### 3.6 Module ABI: control outputs, groups, capabilities, channels
- **Control outputs (#289):** host work only (§1.4.7) – routes from `Out.control` into the graph.
- **Channels:** ABI v1's audio ports are fixed stereo × 1,024 frames (`[[[f32; 1024]; 2]; 4]` each way ≈ 64 KB).
  ABI v2 makes them a planar pool of 128-frame channel buffers (the engine quantum) with per-port offsets and channel
  counts from the descriptor (`channels`, §2.6), at most 32 channels in all – about half today's `Io` size even at
  32 channels each way.
- **Capabilities and sensors (#312), the character's parameters (#314):** need new `In` fields → the `Io` layout
  changes (`audio` would move). Do it **once, as ABI v2**: multichannel ports and the 128-frame block (above), sensor
  channels (motion, position, camera features with a present flag), character parameters (energy, feelings), and room
  for later. v2 replaces v1 (no backwards compatibility,
  §2.0): the build service recompiles the stored module sources, the host reads v2 only. `needs` is a
  descriptor field (additive, §2.4).
- **Assets:** a module's descriptor declares asset parameters (sample, sequence, image); the host hands decoded,
  read-only data through handles in v2 (§2.9).
- **Groups (#314):** a rig module's descriptor lists its children (instruments, modules) as addressable targets; it
  drives them through control outputs by index – no further ABI change. The declarative rig is the same shape without
  code, so validator, bake and editor treat both alike.

### 3.7 The show clock over the network
The deck's time model is §2.8; this is its network part. "Sample-accurate over the network" is not physically available; what is achievable and enough: **locally
sample-accurate, across decks within a few ms.** Design: the conductor sends a tempo **anchor** `{ bpm, per_bar, beat,
at, running }` (relay ms) only on change, scheduled with the relay's lead like a cue; every deck keeps a **clock-offset
filter** over ping / pong (min-RTT window + least-squares drift – deck-core's `fit` exists) and maps relay ms → local
audio frame through its time base; between anchors each deck runs its own `TempoMap` (sample-accurate); a corrected
offset is absorbed by a gentle tempo nudge, never a jump. Targets: ≤ ±5 ms p95 on the LAN, ≤ ±20 ms hosted
(three speakers 3 m apart already differ by ~9 ms acoustically). Link and MIDI clock stay native sources where a venue
has them.

### 3.8 What I am unsure about, and the cheap probe for each
| Unsure | Probe | Cost |
|---|---|---|
| the native deck engine builds for wasm32 without its platform bits, at an acceptable size / speed | P0b: lift `voices.rs` + `mod.rs` steps into a `deck/lab` crate, wasm32 + simd128, FM16 in node (V8) + size | ½ worker day |
| native ≡ wasm bit for bit | P0c: live-core native vs `live_core.wasm` under wasmtime, per kind | in P0 |
| how far the engines differ per kind | P0a (#309) | in P0 |
| wasmi / gas speed on phones | #306 (owner on S22 / iPhone / X1) | owner hour |
| the own-engine-with-N-channels recommendation (§3.1) and the routing model (§2.6) hold: stereo through the N-channel pool is bit-identical and no slower, sends / folders / `out` nodes / panning cost nothing extra, native ≡ wasm | P0d: in the P0b lab crate, `Stereo` → planar N-channel buffers; render today's stereo scene (hash equal, time within 5 %) and one 8-channel graph – a send with the channel map 1/2 → 3/4, a folder summing two buses, an `out` on device channels 3–4, a mono source panned (VBAP) to four speaker positions, a 5.1 bus folded to stereo by the spec's matrix – native and wasm32 + simd128, bit for bit | ½ worker day, in P0 |
| PolyBLEP vs wavetable quality and cost | P0d also renders both saws through the analytic FFT test | in P0 |
| multichannel output in desktop browsers | the owner opens a test page with a multichannel interface: `maxChannelCount`, a 6-channel worklet | owner 10 min |
| show clock accuracy | X1 + S22 + PC on the LAN, click on, phone-mic recording | owner ½ hour |
| `/validate` latency through the bunny pod | time the call in #313's first step | minutes |
| media in the core holds (§2.9): wasm size and speed, phones, disks, hardware planes ≡ software | media probe: rav1d + symphonia + HAP for wasm32 + simd128 (size, native ≡ wasm); 540p AV1 + GPU upscale vs a KTX2 sequence on the S22 for 10 min (fps, CPU, heat, battery); HAP 1080p from an SD card and an SSD on the X1 / a laptop (layers until frames drop); the hardware decoders' planes vs software on a test clip | ½–1 worker day + owner ½ hour |
| frame-accurate screens | two native decks, HAP, the show clock; a camera films both screens | owner ½ hour |

## 4. Phases with exit criteria
Every phase: one worker in its own worktree on `<kind>/<issue>-<title>`, deck CI `npm run deck:ci -- --no-release`
green, merge on the owner's word, push on the owner's word, the rule of §0 in every brief. Models per
`agents/models.json`: M = Sonnet 5.5 medium (cap 100 turns / $15), L = Sonnet 5.5 high (200 / $40), S = Haiku 5.5 (30 /
$3), design = Opus 5.5 only where the owner names it.

**P0 – The probe that can falsify the plan (#309, M).** Goal: evidence per kind, per target. Scope: new
`deck/conformance` crate (dev only): (a) each kind's scene through the native deck's engine (in-block, control stepped per 128
frames) and through `lower()` + live-core, 2 s, block patterns 128 / 37 / 1024 – max abs, RMS, first differing
sample, bit-exact, "missing" / "lowered away"; (b) live-core native vs `live_core.wasm` in wasmtime; (c) the lab spike
of §3.8; (d) **the base check (§3.1, §3.8 P0d)**: the lab copy with planar N-channel buffers – today's stereo scene
hash-equal and within 5 % of its time; the 8-channel routing graph of §3.8 (send 1/2 → 3/4, folder, `out` on 3–4,
VBAP to four speakers, 5.1 folded by the spec's matrix) native ≡ wasm32 + simd128 bit for bit – plus PolyBLEP vs wavetable saws through the analytic FFT test. Accept: the report table goes into ENGINE-ALIGNMENT.md §2; the native deck's current renders are recorded as hashes
(the guard of P4). Falsifies: C (if the spike fails or grows the wasm > +150 KB gz or is slower than `BatchGraph` with
simd) → A; bit-exactness (if (b) differs for osc / fm) → tolerance goldens; the own-engine recommendation (if (d)
shows the N-channel pool costing stereo scenes more than 5 % or not bit-exact) → revisit §3.1 before P4. Rollback: none needed (tests only). Demo:
the table. ≈ $3–5.

**P1 – Module host crate, step one (#305, M).** Extract `module_host.rs` into `deck-module-host` with the `Backend`
trait and the wasmtime backend; the native deck keeps a thin wrapper; hostile / runaway fixtures; **add the module tests to
deck:ci** (closes the CI gap). Accept: `tests/modules.rs` green inside deck:ci; no behaviour change. Rollback: revert.
Demo: none (internal). Engine-agnostic, so it goes first. ≈ $3–4.

**P2 – The kind catalog (new, M).** One `Descriptor` for built-ins and modules (§2.4); the five Rust kind tables read
from it; `kinds.json`, JSON Schema, TS types and `NodeKinds.php` generated, `--check` in deck:ci; the API serves
`kinds.json` (static file + published modules' descriptors). Accept: NodeKinds.php byte-identical in content (same
kinds / ports), node_kinds test replaced by the generator check, deck:ci green, `kinds.json` readable from the MCP.
Rollback: revert (no stored format changes). Demo: the catalog JSON in the creator's Nodes tab later; now a file. ≈ $3–5.

**P3 – Tempo controller into deck-core (#308 step 1, M).** `Tempo`, `Config`, `Source`, tap, quantize, the seqlock
`Shared` / `BeatReader` into `deck_core::tempo`; time and events injected (`now_ns` argument, a push API instead of
`mpsc`); `Config::from_graph` stays with the scene side. Stage sources unchanged. Accept: the native deck's tempo tests moved
and green; owner on the X1: click, tap, MIDI clock out as before. Rollback: revert. Demo: unchanged clock on the X1.
≈ $3.

**P4 – The engine move (new, under #310, L).** `native/src/audio/{mod (engine part), blocks, voices, click, walk core,
module}` + `control.rs` + the FFT half → `deck-engine`; platform traits: module runtime (from P1), PCM feed for
`chapter_music`, beat reader (from P3); the six fundsp units rewritten in deck-core (§3.2 – P7 folded in, fundsp leaves the
workspace); the 128-frame quantum on the native deck's render thread. Behaviour: identical except the six units – still
stereo, control still per frame in this phase. Accept: P0's
recorded native-deck hashes reproduce for every kind and block pattern except the six units (their analytic FFT tests pass); `no_alloc`, `the_batches_sound_like_the_
patcher`, `avx2_and_baseline_are_bit_identical` green in deck-engine; deck:ci green; the X1 plays the default scenes
(owner listens, deck-update bundle). Risk: the largest single change; seams around `Param` / seqlock / rtrb. Rollback:
one revert commit (a move). Demo: the X1 sounds the same, saws and noise from the core. ≈ $10–15 (owner may name it for Opus: ≈ $18–28).
**Built (#317):** `deck/engine` = crate `deck-engine`, features `events` · `modules` · `space`. Moved by `git mv`:
`native/src/audio/mod.rs` (engine part) → `engine/src/lib.rs`; `audio/{blocks, voices, click, walk, module, sequencer,
dispatch}.rs` → `engine/src/`; `control.rs`, `fft.rs` → `engine/src/`. Stays native (`deck-native`, `src/audio/`): backend,
ring, lead, priority, glitch, the sequencer's and the dispatcher's **threads** (the windows, tables and counters are the engine's),
`Engine` / `Handles` as thin wrappers (the core's + the glitch monitor), `modules.rs` (the wasmtime library as the engine's
`ModuleSource`), `DECK_SIMD` / `DECK_EVENTS`. Seams: `Seams { modules }`, the PCM feed is the existing `MusicFeed` ring, the beat
reader is deck-core's. New in deck-core: `dsp.rs` (wavetable saw / square / triangle – a table per quarter octave, 2048 points –,
xorshift64* white, Kellet pink, Simper SVF). fundsp is gone from the workspace; the patcher oracle is the scalar blocks and the
batches equal it exactly; `fm` is true FM (D18); transcendentals on libm; the engine renders fixed 128-frame quanta. Hashes: the
move kept every one; re-recorded (own commits) the six units, fm, walk, the stereo scene, seq, click (libm) and the two LFO scenes at
1024 (quantum). `deck:ci` builds `deck-engine-wasm` for wasm32 + simd128 (940 KB raw, 329 KB gz with the test harness, gate 500 KB gz);
`wasm.rs::engine_native_vs_wasm` proves native ≡ wasm32 ≡ wasm32 + simd128 for every kind and block pattern. The P0 `engine-lab` is deleted.

**P4b – Channels and routing (new, under #310, L).** §2.6: the `Stereo` pool becomes planar N-channel buffers;
multichannel links (sum at inputs, `any` follows the widest input) with channel maps; the kinds `bus` (sends with
channel maps, folders, inserts), `out` (device channels or a named group), `pan` (equal-power, VBAP / DBAP over
speaker positions), `downmix`, `feedback` in the catalog; layouts and speaker positions as data; the device's channels
and their names from the deck configuration (the native deck's backends report the count); the limiter per device channel
after the `out` sum, linked per group; meters per channel. Accept: stereo hashes reported (a change must be explained, §2.0); exact tests for sends, maps, folders and the spec's down-mix tables; a 5.1 scene on a 6-channel
device (the Fireface or HDMI) and folded on the X1's two; a cue `out` on channels 3–4; no_alloc with 32 channels.
Rollback: revert. Demo: one scene on a stereo and a surround output without a second mix.
≈ $8–10.
**Built (#318, `deck/engine`, `deck/space`, `deck/catalog`, `deck/core`):** the pool is `Buf`, planar channels per port (≤ 32; mechanical commit first – all 32 stereo hashes unchanged, the P0 base scene 84–91 ms for 10 s against 89.5 ms). Links: `map = [[1, 3], [2, 4]]` (source, target, counted from 1; a copy, a swap, a sum all fall out of it) and `scale` = the send level on an audio link; several links into one audio input add in the scene's order; a lone unscaled unmapped link of the port's width is read in place, anything else is a gather step; an `any` port is as wide as its widest link. A send is simply a link out of a bus's `pre` or `out`; a folder is a bus that sums its children; an insert chain is the nodes between a source and `in`. Kinds in the catalog: `bus` (fader, mute, solo – mute / solo gate `out` only, `pre` is the tap before them, soloing leaves the buses downstream of the soloed one audible, Reaper's rule, and a soloed folder keeps its children playing into it), `out` (an outlet: `group` default main, `first` or a `channels` list, `layout`; several allowed, an instrument may carry its own), `pan` (mono in; equal power between two speakers, VBAP – 2-D pairs on a ring, 3-D triples for an array with height – or DBAP; the LFE slot stays silent), `downmix` (the Web Audio / ITU-R BS.775 tables mono · stereo · quad · 5.1, or a `matrix` n → m), `feedback` (one block late; a link into it orders nothing, so it is the only way to close a loop; any other cycle is refused and the error names the loop). Layouts are data (`deck_engine::layout`: name + speakers {az, el, dist, lfe}; the deck configuration adds its own, a node may carry `az` / `el` / `dist` lists). `audio_out` stays as the stereo main outlet with its `volume`. **Deck configuration** (`deck_engine::DeckConfig`, serde): device channel count, names, group → device channels, layouts, `fold`; `Seams::deck`; nothing configured = main on channels 1.. , any other group silent (the build's `notes` say so in words). An outlet with fewer device channels than it has is folded by the spec's table between its layout and the layout of the channels it gets (5.1 on two = BS.775), else the first channels play (`Fold::Drop` always). After the outlets are summed per device channel: master ramp → stage environment hook (`StageEnvironment`, pass-through; its documents are #330's) → one `LinkedLimiter` per group of device channels (groups that share a channel are one; two channels = the old limiter bit for bit) → per-channel meters (`Handles::device`; `Meters::channel_level` per pool channel). The native deck's backends report the channel count (cpal; PipeWire and the timer thread stay stereo), the render ring carries it. Tests: exact sends / maps / folders / sums / mute / solo / feedback / cycles / 5.1 on 6 channels and folded on 2 / downmix node / custom matrix / VBAP / outlets / group limiter (`engine/tests/routing.rs`), the pan laws and tables in their modules, `no_alloc` with 32 channels, conformance hashes for the routing scenes and native = wasm32 = wasm32 + simd128 bit for bit (`conformance/tests/routing.rs`, `wasm.rs::routing_native_vs_wasm`).
**Where it differs from §2.6:** (1) no `in` nodes and no sidechain port yet (no kind has a second audio input; a send into one is the same link) and no latency compensation (no kind has a latency); (2) the fold is per `out` node, before the limiter, not on a summed group; (3) an unmapped `main` plays on device channels 1..width – the deck configuration only has to name what differs; (4) `pan` reads channel 1 of its input (a stereo source needs a `downmix` or two pans); a mono source into a stereo outlet plays on channel 1 only – discrete, no implicit up-mix; (5) the spec has no 7.1 table: 7.1 mixes need a `matrix`; (6) the deck configuration is a struct the adapter fills (a saved document, the venue and the cascade are #330); module nodes keep ABI v1's stereo ports.

**P5 – One control half at block rate (new, under #310, M).** `BatchGraph`'s control API (`send` with amount,
`set_base`, `input`, `nodes`, `smooth` / `mul` / `add`) moves onto the engine; control steps per 128-frame block, switched
directly (§2.0); the time model of §2.8; the snapshot ring for the picture. Accept: `BatchGraph`'s tests pass on the
engine; per-scene golden diff frame vs block reported; owner listens on the X1. Rollback: revert.
Demo: LFO scenes on the X1, smoother. ≈ $3–4.
**Built (#319, `deck/engine/src/control.rs`):** `Control` – the one control half – is built with the engine and stepped at the
start of every quantum, before the audio steps: the FFT analysers (fed in the engine, causal: what was rendered, no look-back;
`Bands` keeps its FFT scratch, `Fft::process` allocated per call), the tempo node at the block's first frame (the beat
reader, sample time), the computed nodes (`lfo`, `smooth`, `mul`, `add`) in levels × operation, then every control input of
an audio node into its `Param` (linked and unlinked: one column). Per-sample interpolation is the existing one: each audio
block glides each input across its block (`smooth::Ramp`), so a value that moves every block makes a piecewise-linear,
continuous trajectory. Every block's values go into `Snapshots` (64 blocks, a per-entry seqlock on the frame stamp). The
frame thread's `Controls` is a face: its own copy of the input table, the values of the heard block (`frames − lead`), and
commands (`Base`, `Send`, `Shape`) through an rtrb ring; without an engine it steps a `Control` of its own (the space's
mirror, tests). BatchGraph's API on `Control` / `Controls`: `send` (amount), `set_base`, `base`, `input_at`, `nodes`, the
helpers. **Golden diff** (`conformance/tests/control_rate.rs`, 4 s, block rate vs the old 60 Hz via `Engine::control_at`;
diff / level = RMS of the difference against the block render's RMS; the loudness moved by ≤ 0.21 dB everywhere):

| scene | control → sound | max abs | diff / level | why |
|---|---|---|---|---|
| lfo sine / saw → osc.freq, trill, helpers → osc.freq | a pitch | 0.40–0.46 | +2.6…+3.0 dB | the pitch follows another trajectory (the 60 Hz one lagged ≤ 17 ms and stepped), the phase drifts apart – uncorrelated after a few cycles, the same sound |
| deck: lfo-glide (the demo) | cutoff, pitch, gain | 0.19 | −1.2 dB | the same phase drift (the vibrato), plus the sweep |
| two LFO levels → fm.index | an FM index | 0.34 | −9.2 dB | the index's timing; FM magnifies it |
| deck: madness | fft → cutoff, fm.index | 0.21 | −18.9 dB | the FFT drives sound: a block-rate analysis reacts up to 17 ms sooner |
| tempo.pulse → osc.gain | a gain | 0.11 | −15.2 dB | the beat's pulse decays sharply: at 60 Hz its attack landed up to a frame late |
| tremolo (5 Hz → gain) | a gain | 0.033 | −23.5 dB | the gain's timing, nothing else |
| fft → smooth → gain, cathedral, conductor | gain, cutoff, FM index | 0.04–0.12 | −27…−32 dB | slow LFOs and the FFT on gains: timing only |
| idle, outlet, strobe, pulse-thunderwash, lamps-off, test-pattern, cue-34, surround-51, click, walk | none | 0 | – | no control value reaches the sound (bit-identical) |
| channel-walk | an azimuth | 0 | – | its left channel is silent on a stereo device (the walk starts at the front) |

**Zipper**, the 1 kHz carrier's sidebands (dB under it): tremolo ±60 Hz −113.6 → −159.4, ±375 Hz −112.8 → −178.4; vibrato
(±10 Hz at 5 Hz) ±60 Hz **−57.2 → −134.1**, ±120 Hz −76.8 → −145.6, ±375 Hz −136 → −135 (the floor). The gain path was
already smooth at 60 Hz (the 25 ms glide); the pitch path was not – that is the audible difference. Hashes: every one kept but
lfo sine / saw at 1024 (now equal to their 128 ones); new control scenes, native ≡ wasm32 ≡ wasm32 + simd128.
**Decisions the plan did not state:** (1) the LFO stays unipolar, `(0.5 + 0.5·wave)·depth` (the native deck's; BatchGraph's
was `depth·sin`, ±depth): the catalog's doc said ±depth and was wrong – P6 changes the browser's LFOs to 0..depth (a rig
that needs ±depth adds an offset); channel-walk's scale was written for ±depth and is fixed. (2) The helpers stay the
catalog's `internal` scope (the rig lowering writes them, authors don't – not in `Kind::ALL`, not in NodeKinds.php) but their
feature is `core`: the native deck refused them as unknown before. (3) An unset helper input reads the catalog's default
(`smooth.time` 0.1 s – BatchGraph read 0); the smoother starts at 0 (BatchGraph's). (4) `lfo.phase` is a portless knob (the
start phase, BatchGraph's). (5) BatchGraph's `input(node, port)` is `input_at` (the native `input(node, name, default)` keeps
its name); `set` is `set_base` (Knobs keeps `set`). (6) `Controls::update` keeps its signature: `lead` now picks the
heard block. (7) The face's tempo (`Controls::tempo`, quantised cuts) is still the frame thread's at the heard moment; the
tempo node's outputs are the engine's at the block. (8) The FFT analyses every block (375 / s instead of 60), no look-back.
Not done (out of scope): BatchGraph's body (P12), live-core on the engine (P6), modules reading time (ABI v2), the show clock (#321).

**P6 – The browser on the engine (new, under #310 / #288 steps 1 + 4, L).** live-core holds `deck-engine`; the
device channels are the destination's `maxChannelCount` (2 on phones), the worklet's `outputChannelCount` and the SAB
ring follow it, `out` nodes fold by the deck setting; walks / events through the engine's event host; simd128; `lower()` keeps only flattening (mixers, cv links,
waveforms pass through); a per-kind allowlist (`ENGINE_KINDS`) keeps not-yet kinds on the page; live-core `no_alloc`.
Then retire `noise.ts`, the melody / LFO / `Math.random` of `deck.ts`, the fixed patch of `scene.ts`. Accept: P0 (a)
reports browser ≡ native deck for every kind the wasm build carries (bit-exact by (b)); live-core and live-deck TS tests;
viewer + creator check and build; size gate §3.5; owner on the S22: catpope's rig in stereo on /live/deck and in the
Sound design workspace. Risk: deploys with the viewer (no push freeze – owner 2026-11-02: "i am aware of the danger"). Rollback: the
allowlist back to `BatchGraph` kinds, or revert. Demo: the same rig sounds the same on the X1 and the phone. ≈ $8–12.

**P7 – folded into P4** (no backwards compatibility, §2.0).

**P8 – The show clock (#308 steps 2–4 → new ticket, M/L).** `tempo` message in `schema.json` (regenerated), relay
scheduling with the lead, deck-core offset filter + anchor mapping, the browser source (client sends real `t0`, bytes
into the core, `getOutputTimestamp` stamps), the native deck's `Source::Relay` real; `space.ts`'s fixed BPM goes. Accept:
filter unit tests on synthetic jitter; relay tests; conformance: the same anchor gives the same beat positions in both
targets; owner LAN measurement ≤ ±5 ms p95. Rollback: the conductor's source picker stays on internal. Demo: X1 + phone
clicking together, strobe on the beat in the browser. ≈ $5–8.

**P9 – Rigs and modules everywhere (M each).** (a) The native deck plays rigs (`flatten` → engine, the same path as the
browser; new ticket). (b) After #306's decision (owner bench): the browser backend of `deck-module-host`, `module:*`
through `lower()`, Play of a compiled module in the Sound design workspace (#290) – **the proof of concept**. (c) #289
control outputs and events. (d) #307 `ModuleStats` → the Nodes tab. Accept per ticket: conformance (module fixtures
equal on every backend), no_alloc with module nodes, creator check. Demo: a module written in the creator plays on the
phone and on the X1 unchanged. ≈ $3–5 each.

**P10 – Validation by the core (#313, M), after #302 (M) and the owner's bunny step of #281.** `/validate` in
scutoons-build (deck-space checks, the same wording); the API forwards on **publish** (authority) and the creator runs
the same checks in its wasm core as hints on drafts (§8 D6); `RigValidator.php` + `NodeKinds.php` + their fixtures go.
Accept: PHPUnit for the forwarding and the 503 path, deck:ci, a creator publish with a broken instrument shows the
core's message. ≈ $3–4 + $2–3 (#302).

**P11 – Designs, then ABI v2 (#312, #314 design notes, then one build ticket, M each).** Design notes first (owner
review), Opus only if named. Then ABI v2 (§3.6) replacing v1, the stored modules recompiled. ≈ $3–5 each.

**P12 – Retirements (S).** `ControlGraph`, deck-core `SceneDef`, `synth.rs` blocks, `batch.rs` (its tests live on in
the engine); `deck/lab` pinned to the last commit that builds it (a note in its README, it is outside the workspace);
`modules.ts` deleted once the browser backend passes its fixtures (P9b). ≈ $1.

**P3b – Names (chore, S), before P4 – done (#316).** The native deck moved to `deck/native/`, crate `deck-native`, binary
`deck` (§2.0); the deck-update bundle and the X1 kiosk scripts name the binary `deck` now.

**P13 – Assets and media (owner: built right away, M each).** (a) The asset manifest with derivatives in the API's
scene response, and the server media job (ffmpeg: HAP / HAP Alpha / HAP Q, the ASTC variant, AV1, H.264, audio); (b)
deck-media: the derivative choice, load-ahead, demux, symphonia, HAP, rav1d, the `DecodeBackend` trait with the
software backend, the sample pool, the `sampler` kind; (c) the hardware decode-only adapters (WebCodecs in the media
worker; VideoToolbox / MediaCodec / VA-API); (d) the native deck's asset cache (disk in deck settings, preload per
show); (e) the picture side – planes → wgpu (YUV → RGB, FSR 1 upscale), the frame for the heard time – with #311's
video leg. Accept per ticket: fixture goldens native ≡ wasm, the media probe's numbers (§3.8), the X1 plays HAP from
disk, the S22 plays its phone derivative. ≈ $4–8 each.

## 5. Dependency graph, parallelism, cost
```
P0 ──► P1 ──► P2 ──► P3 ──► P3b ──► P4 ──► P4b ──► P5 ──► P6
 (probe)  (host)  (catalog) (clock) (names) (move+dsp) (channels) (ctl) (browser)
 P0 ──► P13a ─► P13b ─► P13c / P13d (media; P13e with #311's video leg)
                                      │              │
                                      │              ├──► P8 (show clock; needs P3, P6)
                                      │              └──► P9a native deck plays rigs (needs P4, P6)
 #306 owner bench ───────────────────────────────────────► P9b modules in both (needs P1, P6)
                                                            P9c #289 ─► P11 ABI v2 ─► #314 build
 #302 ─► owner bunny step (#281) ─► P10 #313
 P11 design notes (#312, #314): any time, docs only
```
- **Critical path:** P0 → P1 → P3 → P4 → P5 → P6 (≈ 6 worker days). P2 sits on it only because P4 reads the catalog;
  it could follow P4, at the cost of moving the five tables twice.
- **Parallelism under the limits:** two local sessions across all projects, one worker at a time – so the worker lane
  is **serial**; the orchestrator is the second session. What runs in parallel is the **owner lane**: #306 on the S22 /
  iPhone / X1 (any time after P1), X1 listening after P4 / P4b / P5, the LAN clock check after P8, the bunny step that
  unblocks P10. Design notes (P11) can fill a worker slot while the owner reviews something else.
- **Cost from the ledger** (`agents/costs.json`, Sonnet 5.5 workers): S-like $0.7–1.3 (#257, #253), M $2.2–2.9 (#250,
  the engine-signals session), L $4.7–11.2 (#280, #298, #281, #282). Orchestrator days were the big spend ($42–232 on
  Opus); on Sonnet with the $20 session cap they stay small.

| Phase | Class | Worker sessions | Est. worker cost | Owner time |
|---|---|---|---|---|
| P0 | M | 1 | $2–4 | read the table |
| P1 | M | 1 | $3–4 | – |
| P2 | M | 1 | $3–5 | – |
| P3 | M | 1 | $3 | X1 10 min |
| P3b | S | 1 | $1–2 | X1 5 min |
| P4 | L | 1–2 | $10–15 | X1 15 min |
| P4b | L | 1 | $8–10 | X1 + Fireface / HDMI 15 min |
| P5 | M | 1 | $3–4 | X1 10 min |
| P6 | L | 1–2 | $8–12 | S22 15 min |
| P8 | M/L | 1 | $5–8 | LAN 30 min |
| P9 a–d | 4 × M | 4 | $12–20 | S22 + X1 |
| P10 + #302 | 2 × M | 2 | $5–7 | bunny step |
| P11 | 3 × M | 3 | $9–15 | 2 reviews |
| P12 | S | 1 | $1 | – |
| P13 a–e | 5 × M | 5 | $20–40 | S22 + X1 + laptop |
| **Total** | | **≈ 25–27** | **≈ $95–148** | |
Plus orchestrator sessions (≈ 10, one per two or three merges, ≈ $5–10 each on Sonnet): **≈ $145–250 list overall**.

## 6. What stays frozen, what is retired and when
| Thing | Now | Retired in | How |
|---|---|---|---|
| `shared/live-core/src/modules.ts` (TS module host) | frozen (test-only) | P9b | deleted when the browser backend passes its fixtures |
| `server/.../RigValidator.php` + `NodeKinds.php` | in use (CatalogValidator) | P10 | `NodeKinds.php` generated from the catalog from P2; both deleted with #313 |
| parked TS validator (`parked/269-rig-editor`) | parked | never revived | branch deletion is the owner's |
| stage `control.rs`, `audio/{blocks,voices,click,walk,module}`, engine part of `mod.rs` | in use | P4 (moved, not copied) | the native deck keeps backend, ring, lead (until moved), priority, glitch, sequencer / dispatch threads |
| `BatchGraph`, `ControlGraph`, `SceneDef`, `synth.rs` blocks | in use (live-core, lower, lab) | P6 / P12 | API merged in P5, live-core switched in P6, bodies removed in P12 |
| live-core `walks.rs`; deck-space `lower()` page nodes + aliases | in use | P6 / P9b | the engine's event host; `module:*` last |
| TS `noise.ts`, `deck.ts` melody / LFO / `Math.random`, `scene.ts` fixed patch and `Spec` twin | in use | P6 (+ P2 for `Spec`: read `kinds.json`) | removed after the browser plays every kind |
| duplicate tempo code: `space.ts` fixed BPM, walks' own bpm | in use | P8 | the core's clock |
| `lead.ts` (twin of `lead.rs`) | in use | after P8 (a #311 line) | the rule in deck-core, the worker passes underruns |
| fundsp | runtime dep of the native deck | P4 – **done** (#317) | left the workspace (no oracle; the analytic tests of §3.2 replace it) |
| `feature/282-designer-panel`, `backup/270-with-269`, `wip/live-night-75b685`, `feature/269-rig-editor` | parked | after P6 | the owner deletes; agents never do |
| the member save layer (#302) | dormant | before P10 | its own ticket |

## 7. Risks and where the owner's decisions could collide
1. **Chapter 3 (2026-11-15, plan #64).** The release needs **nothing** from this leg – the reader does not load the
   deck engine. What it needs is that the leg stays out of its way: (a) the two-session limit – the release's agent
   work (read-through, validate 0 / 0, smoke) takes priority for a slot from **Nov 9 to Nov 15**; the refactor's worker
   lane pauses then (design notes only, if a slot is free); (b) **no push freeze for live-core** – a live-core push is
   a viewer deploy; the owner removed the guard (2026-11-02: "i am aware of the danger"); (c) **53 commits sit unpushed on `main`** today (deck work, the
   module API and the `2026-11-02-modules` migration) – they should be pushed and checked by **Nov 8**, or held until
   Nov 16, so the release push is not their first. **No deploy waits** (owner 2026-11-02: "we just keep
   going"): P6, P8 (relay image and viewer) and P10 (the API's catalog saves) are pushed when ready. Live performance
   mode restarts Nov 16 for Top Toy (2027-02-17): P6–P9 land Nov 16–30, leaving December for the workshop's content.
2. **Quality loss vs fundsp** – six units, measured by the analytic tests and heard on the X1 before switching (§3.2).
3. **Phones:** the full engine in wasm on the S22 – FM16 with simd is 29 µs on the PC (1 % of the budget); a phone at
   5× is still small. Without simd128 the batch path is slower than the patcher – simd128 is not optional.
4. **Safari:** simd128 (16.4+), SharedArrayBuffer with isolation (15.2+), no Web MIDI (ignored by decision),
   `Atomics.waitAsync` missing in older versions (the 2 ms timer fallback exists). Low risk.
5. **The build service is not deployed** (the owner's bunny step of #281): it blocks P10's authority and the creator's
   compile → Play proof of concept (P9b). It is the owner's step, not code.
6. **Compile / validate in a shared pod:** compiles yield to live shows (#281's rule); `/validate` must not – it is cheap
   (ms) and runs in its own request path. If the pod is down, publishing waits (503), drafts stay editable.
7. **Collision: "the native deck must run BatchGraph" vs plan C.** C keeps the decision's meaning (one engine in the core,
   device I/O in the native deck) and changes only which code becomes it – the owner confirms (D1).
8. **Collision: "migrate kind by kind behind a flag" vs one move.** P4 is one move checked against P0's hashes, so
   the native deck never runs a half-migrated engine; no flags (§2.0) – only the browser's allowlist (P6) stages kinds.
9. **Collision: "browser is UI" vs the worker's scheduling in TS** (when to render, when to analyse, the lead rule):
   these are adapter timing except the lead rule, which moves to the core (§6). The creator's authoring logic stays
   TS by the owner's narrowing (2026-11-02).
10. **Collision: baked rigs (#296) vs code rigs (#314):** a bake folds data; a code rig decides at run time. The bake
    applies to the declarative part (values, mappings, mood diffs) and leaves the code module as a node. Settle in #314's
    design note.
11. **Merge conflicts with deck work in parallel** (the X1 kiosk line, #288's scenes): P2 and P4 touch many imports –
    run them when no other deck session is open.
12. **Determinism may not be bit-exact** on some kind (a hidden libm call, a decoder): P0 (b) finds it on day one; the
    fallback is a stated tolerance for that kind, not a silent one.
13. **Scope:** the native deck's video graph is the next "universal core" gap (§1.4.5) and is deliberately not in this plan.

## 8. Decisions needed from the owner
1. **The engine's source: move the native deck's engine into a new core crate `deck-engine` and merge `BatchGraph`'s API into
   it (plan C).** Recommended. Alternatives: grow `BatchGraph` (A) – ≈ +2 L tickets and a re-earned sound per kind;
   wrap fundsp (B) – no bit-exact goldens, larger wasm. Putting it literally inside deck-core costs moving deck-events in
   too (2,560 lines, a cycle otherwise).
2. **Control at block rate (128 frames) in both targets.** Recommended (deterministic, one cadence). Alternative: keep
   per-frame on the native deck – then native deck and browser can never match for modulated sounds.
3. **fundsp leaves completely – runtime and tests; no third-party engine** (§3.1, §3.2): the six units are written in
   deck-core with analytic tests, the only new dependency is the pure-Rust `libm`. Recommended. Alternatives: fundsp in
   the core for both targets – ≈ +100 KB gz wasm, libm differences between targets, its API churn; Firewheel as the
   engine – the closest to multichannel routing, but its node model, no voice batching, no bit-exact promise.
4. **The determinism bar: bit-exact native ≡ wasm for the engine.** Recommended (cheap to keep once true, catches
   every drift). Alternative: tolerances – cheaper now, silent drift later.
5. **The kind catalog as one `Descriptor` for built-ins and modules, Rust-first, exported to JSON Schema, `kinds.json`,
   TS and (until #313) PHP.** Recommended (your 2026-11-02 suggestion, §2.4). Cost: one M ticket now; it saves the five
   tables' upkeep and the TS / PHP twins. Alternative: export only, keep the tables – cheaper by a day, keeps the drift.
6. **Validation authority at publish:** the API calls the core (`/validate`) on publish; drafts save without the
   service, with the core's checks as hints from the creator's wasm. Recommended (saves keep working when the pod is
   down). Alternatives: every save through the service (503 when down); a native validator binary on the API host
   (unknown whether all-inkl allows `exec`).
7. **Show-clock accuracy target: ±5 ms p95 LAN, ±20 ms hosted, anchor + offset filter.** Recommended. Alternative:
   Link-grade continuous sync over the relay – an L ticket for a gain no audience hears through speakers.
8. **ABI v2 once** (sensors, character parameters, multichannel ports, asset handles), replacing v1 (§2.0).
   Recommended. Alternative: append-only v1 extensions – not possible without moving `audio` in `Io`.
9. **Chapter 3 guard:** push the 53 pending commits by Nov 8 (or hold them to Nov 16); the refactor yields its slot to release work Nov 9–15. Recommended.
10. **Model for P4 and P6 (L):** Sonnet 5.5 high per `models.json` (≈ $8–12 each), or name them for Opus (code.design,
    ≈ $15–25 each). Recommended: Sonnet; Opus only if P0 shows surprises.
11. **Decided (owner 2026-11-02): the routing model of §2.6** – Max's nodes (multichannel links, `out` nodes on chosen
    device channels) and Reaper's buses (every instrument / rig track a bus, sends with channel maps, folders), discrete
    by default, the device's channels from the deck configuration; ambisonics and binaural are panner / decoder kinds or
    modules on it, not engine types; sidechains (§2.6), `in` nodes, serial and parallel chains (selectors designed
    later with the modules).
12. **Decided: the names of §2.0** – deck (native / browser), stage, stage environment, outlet, deck configuration, venue, deck
    settings, folder bus, main; no master / slave wording; the rename is P3b.
13. **Decided: no backwards compatibility for the engine (§2.0)** – no A/B flags, P7 folded into P4, ABI v2 replaces
    v1, stored documents migrated once or recreated.
14. **Decided: latency compensation automatic with a `live` flag, built when a kind needs it (§2.6a).**
15. **Decided: media (§2.9)** – own playback in the core on every deck; real HAP (+ our ASTC variant) for native
    decks, phone derivatives (AV1 + H.264), all pre-encoded in the asset library, the deck chooses by capability;
    hardware decoding as a decode-only adapter, built right away (P13); no deploy waits for P8 / P10.
16. **Decided: fitting a show to the room (§2.6b)** – outlets, the stage environment (the room, every outlet kind),
    the deck configuration (a device's binding, saved, assignable remotely), engine safety per kind, the venue as its
    own document that shows reference; assigned by the cascade show runner → conductors → deck: capability may
    always reject, a lock from above wins, otherwise the nearest wins; every deck reports what it runs and why.
17. **Open: derivatives pin their parent's version (§2.7)** – recommended (an update is an explicit step); the
    alternative is "current" like sets.

18. **Decided (owner 2026-11-02): `fm` is true frequency modulation** – "i want true fm." The carrier's phase accumulates the instantaneous frequency `f + index · fm · sin(modulator)` (fm = f · ratio); a negative instantaneous frequency runs the phase backward, so the stage's `abs()` fold goes; no phase-modulation variant in the browser. One formula in `deck-engine`, both targets. P0 measured the two formulas 0.4 apart (§2a of ENGINE-ALIGNMENT.md); the engine move (#317) takes the native hashes for every kind except the six fundsp units **and `fm` at index × ratio > 1** (the fold's range), and the P0 fm fixtures are regenerated with the analytic Bessel-sideband test.

## 9. Ticket changes (for the owner to approve; agents do not set the board's order)
**Re-scope**
- **#309** – add (b) native ≡ wasm under wasmtime, (c) the lab spike (§3.8), (d) the 8-channel routing graph and the
  saw comparison (§3.8 P0d), the recorded native-deck hashes; drop "glitch" from the kinds; it is P0, first.
- **#310** – becomes the engine tracker: plan C (`deck-engine`), the 128-frame quantum, the bit-exact goldens; its
  steps are P4, P4b, P5, P6 (the new tickets below); "grow BatchGraph" and "glitch" leave its text.
- **#308** – steps 1 only (P3); steps 2–4 move to the new show-clock ticket.
- **#305** – step one adds the module tests to deck:ci; the browser backend follows #306.
- **#289** – note: ABI v1 already has control outputs; host-only work.
- **#290** – depends on P6 and #306 (no longer on "a BatchGraph hook").
- **#313** – the authority at publish (D6), depends on #302 and #281's owner step; `NodeKinds.php` generated from the
  catalog until then.
- **#312** – its descriptor field `needs` lives in the catalog's `Descriptor`; the `Io` change is ABI v2.
- **#311** – add lines: the native deck's control domain (P4/P5), the native deck's video graph (later leg), the lead rule,
  `space.ts`'s fixed BPM, live-core's unproven no-alloc.
- **#288** – steps 1 and 4 are P6; it keeps steps 2, 3, 5.
- **#64** – no change; a comment that the deck leg needs no release work and yields Nov 9–15.

**Close:** none now (#290 stays on hold; #306 waits for the owner's devices).

**Create** (labels: `area:deck` unless said; complexity as given)
1. *The kind catalog: one Descriptor for built-in kinds and modules, exported as JSON Schema, kinds.json, TS and PHP* –
   feature, M: five Rust kind tables and the TS / PHP twins become one `Descriptor` table in deck-space; generated
   files `--check`ed in deck:ci; the API serves kinds.json (owner 2026-11-02 "batchgraph can document their schema in
   rust structs").
2. *The engine move: the native deck's audio engine into the new core crate deck-engine, fundsp's six units rewritten in
   deck-core* – feature, L: checked against the recorded hashes; the native deck keeps device I/O only.
3. *One control half at block rate: BatchGraph's API on the engine, the time model, snapshot ring for the picture* –
   feature, M.
4. *The browser runs deck-engine: every kind in live-core, stereo, simd128, lower() reduced, TS DSP retired* – feature,
   L.
5. *(folded into 2 – no backwards compatibility)*
10. *Channels and routing: multichannel links with channel maps, bus / out / pan / downmix / feedback kinds, device
   channels from the deck configuration* – feature, L (P4b; owner 2026-11-02: Max's nodes, Reaper's buses).
11. *Names: deck/native → deck/native, crate deck-native, binary deck* – chore, S (P3b).
12. *Asset manifest with derivatives and the server media job (HAP, ASTC variant, AV1, H.264, audio)* – feature, M
    (P13a; `area:api`).
13. *deck-media: derivative choice, demux, symphonia, HAP, rav1d, DecodeBackend, sampler kind* – feature, M (P13b).
14. *Hardware decode-only adapters: WebCodecs VideoDecoder, VideoToolbox, MediaCodec, VA-API* – feature, M (P13c).
15. *The native deck's asset cache on a chosen disk, preload per show* – feature, M (P13d).
16. *Video frames on the GPU: planes → wgpu, FSR 1 upscale, frame for the heard time* – feature, M (P13e, with #311).
17. *Venues: stage environment and deck configuration for every outlet kind (audio, video, DMX, MIDI, OSC), the
    cascade, the venue document referenced by shows* – design note first, then features (`area:live`; revises LIVE.md
    B.5's venue patches).
6. *Show clock over the relay: a tempo anchor message, the clock-offset filter in deck-core, the browser source* –
   feature, M/L (from #308 steps 2–4).
7. *The native deck plays rigs (flatten → engine), the same path as the browser* – feature, M.
8. *Module ABI v2: sensor channels and the character's parameters; replaces v1; stored modules
   recompiled* – feature, M; after #312 / #314's design notes.
9. *Retire the reference evaluators (ControlGraph, SceneDef, synth.rs blocks, batch.rs) and pin deck/lab* – chore, S.

**Recommended order** (top down, for the owner to set on the board): #309 → #305 → catalog → #308 (step 1) → names →
engine move → channels and routing → control half → (release pause Nov 9–15; #312 / #314 design notes if a slot is free) → browser on the engine →
show clock → native deck plays rigs → #306 decision → #290 → #289 → #307 → #302 → #313 → ABI v2 → retire.
