Draft for owner review, 2026-11-02 – the plan for converging the deck onto one engine. Nothing in it is built; the phases need the owner's go (and the decisions of §8) before the first worker starts.

# ENGINE-REFACTOR-PLAN – one engine in the universal core

Owner 2026-11-02: "deck-core is everything. with built I mean only lightweight adapters for other browser services and
the crate compiler target. really need to stick to one universal core." · on the stage running its own audio engine:
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
| `deck/stage/src/audio` | 4,081 total: **engine ≈ 1,960** (mod 500, blocks 428, voices 248, dispatch 241, walk 194, sequencer 151, module 116, click 80) · **device + diagnostics ≈ 1,320** (backend 717, glitch 262, ring 196, priority 92, lead 49) · tests ≈ 800 | the stage's own engine: level-ordered steps over a stereo buffer pool, audio-rate links, `F8` voice batches (AVX2 copy), fundsp blocks, envelopes, cv (pitch L / gate R) | native stage |
| `deck/stage/src/control.rs` | 362 / 205 | **a third duplicate, not in the report:** the stage's control domain (LFO sine / saw / square, the tempo node, FFT → control), stepped once per **video frame** on the frame thread, handed to audio through atomic `Param`s | native stage |
| `deck/stage/src/tempo/` | mod 418 / 128, map 73, midi 477, osc 139, link 68 | the clock controller (sources, priority, tap, quantized start, seqlock) and the sources | native stage |
| `deck/stage/src/module_host.rs` + `audio/module.rs` | 576 + 116 | wasmtime module host + the module node | native stage, feature `modules` only |
| `deck/live-core` | 1,492 (core 575, walks 247, signals 238, wire 233) | the browser's engine: `BatchGraph` + walks driving a voice's `freq` + limiter + analyzer + outputs, raw wasm exports | browser worker |
| `deck/space` (deck-space) | 12,900 (rig/ ≈ 1,400 code + 644 tests; graph 419, params 151, scene 198, modreg 124) | scene model, the `Kind` table, `Spec` ranges, rigs (`play`, `flatten`, `lower`, checks), the being | stage (model, being), space-web (rigs, being) |
| `deck/events`, `deck/outputs` | 2,560, 2,474 | generators / patches / routing; the outputs engine | both (already one code base) |
| `deck/module` (+ macros) | 1,045 + 290 | the SDK and ABI v1 (`Io` with audio, control in **and control out**, events, vars, tempo, character) | modules, both hosts |
| `deck/build` | 1,875 | compile service (#281); not deployed (owner's bunny step open) | server |
| `shared/live-core/src` | 1,276 TS | worker, worklet, rings, `lead.ts`, `modules.ts` (346, **imported by its test only**) | browser |
| `shared/live-deck/src` | 2,100 TS | `deck.ts` melody via `Math.random`, `noise.ts` (Web Audio + `Math.random`), `scene.ts` hand twin of `Spec` ranges, `space.ts:416` a **fixed BPM** fed to the being | browser |
| `server/src/Domain/Rigs` | RigValidator 704 + generated NodeKinds 38 | second implementation of deck-space's checks | API |

### 1.2 What plays where today
- **The native stage plays scenes, not rigs** (`main.rs:9`: "the stage plays scenes, not rigs, yet"). Rigs are played
  only in the browser: space-web `rig_play` → `Rig::flatten` → `lower()` → live-core. "Rigs are universal" is not true
  yet on the deck that matters most for a show.
- **Control rate differs:** the browser steps its control graph once per 128-frame block (`engine.ts` `BLOCK`), the
  stage once per video frame (~60 Hz). Any LFO-driven sound differs by design, before any DSP difference.
- **Block segmentation leaks into the sound:** `Ramp::block` glides per call, and live-core splits its render at every
  walk note – the output depends on where blocks are cut. The stage's tests compare builds with the same cut pattern
  only.
- **Clock sync is missing:** the relay answers `ping {t0}` with `pong {t0, t1}`, but no client computes an offset (the
  TS client sends `t0: 0`). The relay's `at` (ms) cannot be placed on a local audio frame today.
- **CI gap:** `npm run deck:ci` runs `cargo test --workspace` with default features; the stage's `modules` feature is
  not default, so `module_host.rs` and `tests/modules.rs` are never built or tested by it.
- **wasm:** `live_core.wasm` is 706 KB (249 KB gz), built **without simd128**; space-web 3.6 MB (1.2 MB gz). The lab's
  Chromium numbers: 16 FM voices × 128 frames – patcher 49.8 µs, BatchGraph without simd **64.0 µs (slower)**, with
  simd128 29.4 µs. Budget per 128-frame block: 2,667 µs.
- **Allocation:** the stage's render is proven allocation-free (`tests/no_alloc.rs`). live-core's render calls
  `resize`, `extend` and `sort_unstable` on its hot path and has no such test.

### 1.3 Node kinds – what each engine does today (read from code; P0 measures the numbers)
| Kind | Stage (native) | Browser (live-core after `lower()`) | Parity expected |
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
| `mixer` | 4 stereo inputs, gains | folded into `mul`s, **mono** master | differs (stereo) |
| `fft` | analyses its input (a tap), frame thread, heard window | analyses the limited master output | differs (where) |
| `audio_out` + limiter | stereo, master ramp, deck-core limiter | mono, deck-core limiter | close for mono material |
| `module:*` | wasmtime (feature `modules`) | pushed to page nodes, not played | missing |
| `tempo` (control) | the clock's outputs | – | missing |

### 1.4 Corrections to ENGINE-ALIGNMENT.md
1. **"glitch" is not a node kind.** `audio/glitch.rs` is the device glitch monitor (#208: underrun, long render, late
   callback) – platform diagnostics, it stays with the adapter. #309's and #310's kind lists name it wrongly.
2. **The stage's control domain (`control.rs`) is a third duplicate** of the batch model, missing from §2 of the report.
3. **"~4,081 lines vs 561"** compares unlike things: ≈ 1,320 of the 4,081 are device I/O and diagnostics that stay, and
   ≈ 800 are tests; the engine logic to converge is ≈ 1,960 (audio) + 362 (control) lines against `BatchGraph`'s 418
   (+ live-core's ≈ 600 around it). The **stage's engine is the bigger and more capable one** (stereo, audio-rate links,
   cv, envelopes, `F8` + AVX2, no-alloc proof).
4. **The browser does not play `osc` / `fm` at parity:** `lower()` keeps only a kind's ports, so `wave`, `shape`,
   `attack` / `release` vanish silently.
5. **"Picture side: genuinely one code base"** holds for deck-space (the being, sets, rigs) only. The stage's video graph
   (`render.rs` 1,036, shader / chapter / overlay / outlet nodes, `live/` ≈ 1,500, `text.rs`) is native-only. Out of
   this plan; it belongs on #311 as a later leg.
6. **The stage does not play rigs at all** (§1.2); the report's open question "how the stage plays rigs natively" has
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

## 2. Target architecture

### 2.1 The crates after the refactor
| Crate | Holds | Builds for |
|---|---|---|
| **deck-core** | foundations: math, `F8` lanes, `Ramp`, limiter, FFT analyzer, the DSP units (band-limited oscillators, noise, SVF), **the tempo clock** (map + controller + relay-sync filter), the lead rule | native, wasm32 |
| **deck-space** (no features) | the scene / instrument / rig model, **the kind catalog** (§2.4), rig logic (`play`, `flatten`, checks, later `bake`) | native, wasm32, build service |
| **deck-events**, **deck-outputs** | generators, patches, routing; the outputs engine (unchanged) | native, wasm32 |
| **deck-module** | SDK + ABI (v1, later v2) + **the one descriptor type** (built-ins and modules) | wasm32 (modules), native (hosts) |
| **deck-module-host** (new, #305) | descriptor / layout, `Io` staging, log limiter, budget, `ModuleStats`, `Backend` trait; backends: `wasmtime` (feature, native), `web` or `wasmi` (feature, wasm – #306 decides) | native, wasm32 |
| **deck-engine** (new) | **the one engine**: graph build, control half (block rate), audio half (stereo pool, audio links, `F8` batches, blocks), every kind, the in-block event host, the module node, analysis, `send` / `set_base` / paths table | native (baseline, AVX2 copy), wasm32 (+simd128) |
| deck-stage | adapter + app: device backends (cpal, PipeWire, ASIO), render thread + ring, priority, glitch monitor, time base (`clock.rs`), tempo sources (Link, MIDI, OSC), the sequencer thread as a scheduling wrapper, window / GPU, video graph (later leg), sync, files | native |
| live-core | adapter: the worker's raw exports (inbox / outbox, pointers), nothing else | wasm32 |
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
   │ deck-stage (adapters)      │  │ live-core (exports)     │  │ space-web        │  │ scutoons-build      │
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
| events, routing | deck-events (+ engine's in-block host) | the stage's sequencer / dispatcher threads (scheduling only) |
| lead rule (when to grow / shrink) | deck-core (from `lead.rs`; `lead.ts` goes) | measuring underruns |
| kind metadata and its exports | deck-space catalog + deck-module `Descriptor` | – |

### 2.4 The kind catalog – one descriptor for built-ins and modules (owner 2026-11-02)
The node kinds, their ports and parameters and the graph documents are **plain Rust types that derive their schema**;
everything else is exported from them.
- **One type, `deck_module::Descriptor` (v2 shape):** `name`, `version`, `domain`, `doc`, `feature` (the build module
  that carries it), `scene_only`, `needs` (capabilities, #312), `abi` (modules only), `ports[]` {`name`, `dir`,
  `type` audio · control · cv · video · event, `doc`}, `params[]` {`name`, `min`, `max`, `default`, `curve` lin · log,
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
| stage, x86-64 baseline | all + wasmtime backend | AVX2 loops chosen at start (as today) |
| stage, x86-64-v3, aarch64 | same | aarch64 cross on GitHub by hand |
| live-core, wasm32 + simd128 | deck-core, deck-space (no features), deck-events, deck-outputs, deck-module-host (web/wasmi), deck-engine | size gate ≤ 350 KB gz (249 today); no fundsp, no symphonia, no schemars |
| space-web, wasm32 | deck-space (gpu) | unchanged |
| scutoons-build | deck-module, deck-space | gains `/validate`, later `/bake` |
| conformance (tests) | deck-engine, live-core, wasmtime (dev) | runs `live_core.wasm` under wasmtime to compare targets bit for bit |

## 3. The hard problems, argued

### 3.1 Where the one engine's code comes from (the central call)
| Option | For | Against |
|---|---|---|
| A. Grow `BatchGraph` (#310's wording) to every kind | small, clean start | re-writes ≈ 2,300 lines the stage already has and tests; mono, no audio-rate links, no cv / envelope, slower than the stage's `F8` voices in wasm without simd; every kind must re-earn the stage's sound |
| B. deck-engine wraps fundsp for both targets | quality for free | fundsp is a per-node `dyn AudioUnit` patcher (against batching), calls std `sin` / `tan` → libm differs native vs wasm (no bit-exact goldens), size in wasm; only 5 units are used |
| **C. Move the stage's engine into deck-engine, merge `BatchGraph`'s control API into it** | the more complete engine becomes the shared one; already built on deck-core pieces (`Col`, `levels`, `F8`, `Ramp`, limiter); the move is identity for the stage (goldens prove it); the browser gains every kind at once | the move touches threading seams (Params, seqlock, rtrb) that must become platform-free; one large move ticket |
**Decision: C** – the result is still "deck-core's graph" in the owner's sense (one engine in the universal core; the
stage keeps device I/O only); only the *source* of the code changes. `BatchGraph`'s good parts survive: `send` with an
amount (#189 force / blend), `set_base` (#188), `input()`, the `nodes()` paths table, `smooth` / `mul` / `add`. P0
falsifies C cheaply (does the moved engine build for wasm32, is its size and speed in a browser acceptable); if not,
fall back to A with the stage's `voices.rs` loops ported into it.

### 3.2 fundsp parity
fundsp is used for exactly five things: band-limited **saw, square, triangle** (wavetables), **white** and **pink
noise**, and the **lowpass** behind noise. (Sine and FM already run on deck-core's polynomial sine.) Plan: re-implement
the five in deck-core (wavetables built at load, outside the audio thread; seeded xorshift white; Kellet pink; a
Simper SVF whose `tan` comes from a deck-core approximation), each behind a per-kind switch (`DECK_DSP=fundsp|core`) on
the stage; **fundsp stays only as a dev-dependency – the test oracle**: harmonic levels within 0.1 dB, aliasing below
−60 dB under 4 kHz fundamentals, pink slope −3 dB / octave ± 0.5, the lowpass's −3 dB point within 2 % and its Q. The
owner listens on the X1 before the default flips. Quality risk is bounded: five units, each measurable.

### 3.3 Real-time constraints
- **No allocation, no lock, no wait in the render** – the stage's `no_alloc` test moves with the engine, and live-core
  gets one (its `resize` / `extend` / `sort` go to preallocated buffers).
- **One engine quantum: 128 frames.** The engine always renders whole 128-frame blocks; the control half steps once per
  block inside the render. The stage's render thread already runs ahead into a ring (ring.rs), so device buffer sizes
  (ASIO 64, PipeWire 256, 1024) no longer cut the engine's blocks. Result: output independent of the device, ramps
  independent of segmentation, the same cadence as the browser. Latency: unchanged (the adaptive lead governs it).
  Events land on their sample inside a block where a node reads them (click, seq, notes), else at the block start
  (≤ 2.67 ms).
- **Control at block rate** costs ≈ 6× today's control CPU on the stage (300 nodes 3–8 µs per step → ≈ 0.1–0.3 % of a
  core) – negligible. The picture reads a **snapshot ring** of control values indexed by audio frame (≈ 64 blocks deep,
  preallocated) and picks the heard frame through the time base, so what is seen still matches what is heard.
- **Threads are adapters:** the engine is single-threaded; the stage may run deck-events' patches on its sequencer
  thread ahead of the render (the same `Core` that runs in-block; the stage already proves both give the same
  samples).
- **Behaviour changes the owner will hear** (each behind a flag, each golden-diffed, each switched only after he
  listens): control at block rate; FFT → control from the render (causal) instead of the heard window; in the browser:
  envelopes, stereo, real waveforms.

### 3.4 Determinism and golden renders
- **Target: bit-exact across native baseline, native AVX2, wasm, wasm + simd128** for everything the engine computes.
  It is plausible: the stage already proves AVX2 ≡ baseline bit for bit; wasm f32 arithmetic is IEEE; deck-core avoids
  libm. Rules to make it hold: no std `exp` / `sin` / `powf` / `tan` on engine paths (deck-core approximations or the
  pure-Rust `libm` crate) – today `BatchGraph`'s `smooth` calls `exp` per step and `env_coefs` once; no relaxed-SIMD, no
  FMA contraction; every random source seeded (the seed rule; `Math.random` leaves).
- **A golden = scene + event log (frame-stamped) + seed → a hash per target** of the rendered f32 stream; the files hold
  hashes, not audio. Tolerance only where the input itself differs: decoded chapter music (decoders differ – tested
  with a WAV fixture), and the fundsp oracle of §3.2.
- **How it runs in CI:** the conformance crate renders natively and loads `live_core.wasm` (built earlier in deck:ci)
  under wasmtime – no browser needed. V8 is checked by the owner's browser run once per phase that changes live-core.

### 3.5 wasm size and start-up
Today 249 KB gz for live-core. Expected growth: the engine's kinds + voices + event host + clock ≈ +60–120 KB gz; wasmi,
if chosen, ≤ +150 KB gz (#306 gate). Kept out: fundsp, symphonia (decoding is an adapter: the browser's
`decodeAudioData`, native symphonia – the core gets PCM), schemars. Wavetables are computed at load (≈ 1–3 ms, not in
the binary). **simd128 on** (`-C target-feature=+simd128` for live-core; all target browsers have it; Safari since
16.4 – older ones are ignored by the owner's rule). Gate per phase: ≤ 350 KB gz, compile + instantiate ≤ 10 ms on the
PC (5.4 ms today).

### 3.6 Module ABI: control outputs, groups, capabilities
- **Control outputs (#289):** host work only (§1.4.7) – routes from `Out.control` into the graph.
- **Capabilities and sensors (#312), the character's parameters (#314):** need new `In` fields → the `Io` layout
  changes (`audio` would move). Do it **once, as ABI v2**: sensor channels (motion, position, camera features with a
  present flag), character parameters (energy, feelings), and room for later. The host accepts v1 and v2 (two offset
  tables from the same type), the build service recompiles every stored module source to v2, then v1 goes. `needs` is a
  descriptor field (additive, §2.4).
- **Groups (#314):** a rig module's descriptor lists its children (instruments, modules) as addressable targets; it
  drives them through control outputs by index – no further ABI change. The declarative rig is the same shape without
  code, so validator, bake and editor treat both alike.

### 3.7 The show clock over the network
"Sample-accurate over the network" is not physically available; what is achievable and enough: **locally
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
| the stage engine builds for wasm32 without its platform bits, at an acceptable size / speed | P0b: lift `voices.rs` + `mod.rs` steps into a `deck/lab` crate, wasm32 + simd128, FM16 in node (V8) + size | ½ worker day |
| native ≡ wasm bit for bit | P0c: live-core native vs `live_core.wasm` under wasmtime, per kind | in P0 |
| how far the engines differ per kind | P0a (#309) | in P0 |
| wasmi / gas speed on phones | #306 (owner on S22 / iPhone / X1) | owner hour |
| wavetable quality vs fundsp | the oracle test of P7 | in P7 |
| show clock accuracy | X1 + S22 + PC on the LAN, click on, phone-mic recording | owner ½ hour |
| `/validate` latency through the bunny pod | time the call in #313's first step | minutes |

## 4. Phases with exit criteria
Every phase: one worker in its own worktree on `<kind>/<issue>-<title>`, deck CI `npm run deck:ci -- --no-release`
green, merge on the owner's word, push on the owner's word, the rule of §0 in every brief. Models per
`agents/models.json`: M = Sonnet 5.5 medium (cap 100 turns / $15), L = Sonnet 5.5 high (200 / $40), S = Haiku 5.5 (30 /
$3), design = Opus 5.5 only where the owner names it.

**P0 – The probe that can falsify the plan (#309, M).** Goal: evidence per kind, per target. Scope: new
`deck/conformance` crate (dev only): (a) each kind's scene through the stage's engine (in-block, control stepped per 128
frames) and through `lower()` + live-core, 2 s, block patterns 128 / 37 / 1024 – max abs, RMS, first differing
sample, bit-exact, "missing" / "lowered away"; (b) live-core native vs `live_core.wasm` in wasmtime; (c) the lab spike
of §3.8. Accept: the report table goes into ENGINE-ALIGNMENT.md §2; the stage's current renders are recorded as hashes
(the guard of P4). Falsifies: C (if the spike fails or grows the wasm > +150 KB gz or is slower than `BatchGraph` with
simd) → A; bit-exactness (if (b) differs for osc / fm) → tolerance goldens. Rollback: none needed (tests only). Demo:
the table. ≈ $2–4.

**P1 – Module host crate, step one (#305, M).** Extract `module_host.rs` into `deck-module-host` with the `Backend`
trait and the wasmtime backend; the stage keeps a thin wrapper; hostile / runaway fixtures; **add the module tests to
deck:ci** (closes the CI gap). Accept: `tests/modules.rs` green inside deck:ci; no behaviour change. Rollback: revert.
Demo: none (internal). Engine-agnostic, so it goes first. ≈ $3–4.

**P2 – The kind catalog (new, M).** One `Descriptor` for built-ins and modules (§2.4); the five Rust kind tables read
from it; `kinds.json`, JSON Schema, TS types and `NodeKinds.php` generated, `--check` in deck:ci; the API serves
`kinds.json` (static file + published modules' descriptors). Accept: NodeKinds.php byte-identical in content (same
kinds / ports), node_kinds test replaced by the generator check, deck:ci green, `kinds.json` readable from the MCP.
Rollback: revert (no stored format changes). Demo: the catalog JSON in the creator's Nodes tab later; now a file. ≈ $3–5.

**P3 – Tempo controller into deck-core (#308 step 1, M).** `Tempo`, `Config`, `Source`, tap, quantize, the seqlock
`Shared` / `BeatReader` into `deck_core::tempo`; time and events injected (`now_ns` argument, a push API instead of
`mpsc`); `Config::from_graph` stays with the scene side. Stage sources unchanged. Accept: the stage's tempo tests moved
and green; owner on the X1: click, tap, MIDI clock out as before. Rollback: revert. Demo: unchanged clock on the X1.
≈ $3.

**P4 – The engine move (new, under #310, L).** `stage/src/audio/{mod (engine part), blocks, voices, click, walk core,
module}` + `control.rs` + the FFT half → `deck-engine`; platform traits: module runtime (from P1), PCM feed for
`chapter_music`, beat reader (from P3); fundsp as an optional native-only feature (`fundsp`) until P7; the 128-frame
quantum on the stage's render thread. Behaviour: **identical** – control still per frame in this phase. Accept: P0's
recorded stage hashes reproduce bit for bit for every kind and block pattern; `no_alloc`, `the_batches_sound_like_the_
patcher`, `avx2_and_baseline_are_bit_identical` green in deck-engine; deck:ci green; the X1 plays the default scenes
(owner listens, deck-update bundle). Risk: the largest single change; seams around `Param` / seqlock / rtrb. Rollback:
one revert commit (a move). Demo: the X1 sounds the same. ≈ $8–12 (owner may name it for Opus: ≈ $15–25).

**P5 – One control half at block rate (new, under #310, M).** `BatchGraph`'s control API (`send` with amount,
`set_base`, `input`, `nodes`, `smooth` / `mul` / `add`) moves onto the engine; control steps per 128-frame block behind
`DECK_CONTROL=frame|block` on the stage; the snapshot ring for the picture. Accept: `BatchGraph`'s tests pass on the
engine; per-scene golden diff frame vs block reported; owner listens on the X1 → default `block`. Rollback: the flag.
Demo: LFO scenes on the X1, smoother. ≈ $3–4.

**P6 – The browser on the engine (new, under #310 / #288 steps 1 + 4, L).** live-core holds `deck-engine`; stereo
master; walks / events through the engine's event host; simd128; `lower()` keeps only flattening (mixers, cv links,
waveforms pass through); a per-kind allowlist (`ENGINE_KINDS`) keeps not-yet kinds on the page; live-core `no_alloc`.
Then retire `noise.ts`, the melody / LFO / `Math.random` of `deck.ts`, the fixed patch of `scene.ts`. Accept: P0 (a)
reports browser ≡ stage for every kind the wasm build carries (bit-exact by (b)); live-core and live-deck TS tests;
viewer + creator check and build; size gate §3.5; owner on the S22: catpope's rig in stereo on /live/deck and in the
Sound design workspace. Risk: deploys with the viewer – **not pushed between 2026-11-09 and 2026-11-15**. Rollback: the
allowlist back to `BatchGraph` kinds, or revert. Demo: the same rig sounds the same on the X1 and the phone. ≈ $8–12.

**P7 – Core DSP replaces fundsp (new, M).** §3.2: the five units in deck-core, `DECK_DSP` per kind, fundsp as oracle.
Accept: oracle tests; owner listens on the X1; then fundsp leaves the runtime dependencies and the browser plays
saw / square / triangle / noise in the engine. Rollback: the flag. Demo: noise and saws on the phone. ≈ $3–5.

**P8 – The show clock (#308 steps 2–4 → new ticket, M/L).** `tempo` message in `schema.json` (regenerated), relay
scheduling with the lead, deck-core offset filter + anchor mapping, the browser source (client sends real `t0`, bytes
into the core, `getOutputTimestamp` stamps), the stage's `Source::Relay` real; `space.ts`'s fixed BPM goes. Accept:
filter unit tests on synthetic jitter; relay tests; conformance: the same anchor gives the same beat positions in both
targets; owner LAN measurement ≤ ±5 ms p95. Rollback: the conductor's source picker stays on internal. Demo: X1 + phone
clicking together, strobe on the beat in the browser. ≈ $5–8.

**P9 – Rigs and modules everywhere (M each).** (a) The stage plays rigs (`flatten` → engine, the same path as the
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
review), Opus only if named. Then ABI v2 (§3.6) with the v1 / v2 host and the recompile of stored modules. ≈ $3–5 each.

**P12 – Retirements (S).** `ControlGraph`, deck-core `SceneDef`, `synth.rs` blocks, `batch.rs` (its tests live on in
the engine); `deck/lab` pinned to the last commit that builds it (a note in its README, it is outside the workspace);
`modules.ts` deleted once the browser backend passes its fixtures (P9b). ≈ $1.

## 5. Dependency graph, parallelism, cost
```
P0 ──► P1 ──► P2 ──► P3 ──► P4 ──► P5 ──► P6 ──► P7
 (probe)  (host)  (catalog) (clock)  (move)  (ctl)   (browser)  (dsp)
                                      │              │
                                      │              ├──► P8 (show clock; needs P3, P6)
                                      │              └──► P9a stage plays rigs (needs P4, P6)
 #306 owner bench ───────────────────────────────────────► P9b modules in both (needs P1, P6)
                                                            P9c #289 ─► P11 ABI v2 ─► #314 build
 #302 ─► owner bunny step (#281) ─► P10 #313
 P11 design notes (#312, #314): any time, docs only
```
- **Critical path:** P0 → P1 → P3 → P4 → P5 → P6 (≈ 6 worker days). P2 sits on it only because P4 reads the catalog;
  it could follow P4, at the cost of moving the five tables twice.
- **Parallelism under the limits:** two local sessions across all projects, one worker at a time – so the worker lane
  is **serial**; the orchestrator is the second session. What runs in parallel is the **owner lane**: #306 on the S22 /
  iPhone / X1 (any time after P1), X1 listening after P4 / P5 / P7, the LAN clock check after P8, the bunny step that
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
| P4 | L | 1–2 | $8–12 | X1 15 min |
| P5 | M | 1 | $3–4 | X1 10 min |
| P6 | L | 1–2 | $8–12 | S22 15 min |
| P7 | M | 1 | $3–5 | X1 10 min |
| P8 | M/L | 1 | $5–8 | LAN 30 min |
| P9 a–d | 4 × M | 4 | $12–20 | S22 + X1 |
| P10 + #302 | 2 × M | 2 | $5–7 | bunny step |
| P11 | 3 × M | 3 | $9–15 | 2 reviews |
| P12 | S | 1 | $1 | – |
| **Total** | | **≈ 19–21** | **≈ $65–100** | |
Plus orchestrator sessions (≈ 8, one per two or three merges, ≈ $5–10 each on Sonnet): **≈ $105–180 list overall**.

## 6. What stays frozen, what is retired and when
| Thing | Now | Retired in | How |
|---|---|---|---|
| `shared/live-core/src/modules.ts` (TS module host) | frozen (test-only) | P9b | deleted when the browser backend passes its fixtures |
| `server/.../RigValidator.php` + `NodeKinds.php` | in use (CatalogValidator, MemberRigs) | P10 | `NodeKinds.php` generated from the catalog from P2; both deleted with #313 |
| parked TS validator (`parked/269-rig-editor`) | parked | never revived | branch deletion is the owner's |
| stage `control.rs`, `audio/{blocks,voices,click,walk,module}`, engine part of `mod.rs` | in use | P4 (moved, not copied) | the stage keeps backend, ring, lead (until moved), priority, glitch, sequencer / dispatch threads |
| `BatchGraph`, `ControlGraph`, `SceneDef`, `synth.rs` blocks | in use (live-core, lower, lab) | P6 / P12 | API merged in P5, live-core switched in P6, bodies removed in P12 |
| live-core `walks.rs`; deck-space `lower()` page nodes + aliases | in use | P6 / P9b | the engine's event host; `module:*` last |
| TS `noise.ts`, `deck.ts` melody / LFO / `Math.random`, `scene.ts` fixed patch and `Spec` twin | in use | P6 (+ P2 for `Spec`: read `kinds.json`) | removed after the browser plays every kind |
| duplicate tempo code: `space.ts` fixed BPM, walks' own bpm | in use | P8 | the core's clock |
| `lead.ts` (twin of `lead.rs`) | in use | after P8 (a #311 line) | the rule in deck-core, the worker passes underruns |
| fundsp | runtime dep of the stage | P7 | dev-dependency (oracle) only |
| `feature/282-designer-panel`, `backup/270-with-269`, `wip/live-night-75b685`, `feature/269-rig-editor` | parked | after P6 | the owner deletes; agents never do |
| the member save layer (#302) | dormant | before P10 | its own ticket |

## 7. Risks and where the owner's decisions could collide
1. **Chapter 3 (2026-11-15, plan #64).** The release needs **nothing** from this leg – the reader does not load the
   deck engine. What it needs is that the leg stays out of its way: (a) the two-session limit – the release's agent
   work (read-through, validate 0 / 0, smoke) takes priority for a slot from **Nov 9 to Nov 15**; the refactor's worker
   lane pauses then (design notes only, if a slot is free); (b) **deploy freeze Thu Nov 12** – P6 and anything that
   changes live-core's sound is not pushed before Nov 16; (c) **53 commits sit unpushed on `main`** today (deck work, the
   module API and the `2026-11-02-modules` migration) – they should be pushed and checked by **Nov 8**, or held until
   Nov 16, so the release push is not their first. **Safe before the release:** P0–P5 (native, or identity for the
   browser: P2 rebuilds the wasm with unchanged behaviour). **Must not start before Nov 16:** P6's push, P8's protocol
   change (deploys the relay image and the viewer), P10 (touches the API's catalog saves). Live performance mode
   restarts Nov 16 for Top Toy (2027-02-17): P6–P9 land Nov 16–30, leaving December for the workshop's content.
2. **Quality loss vs fundsp** – five units, measured against fundsp itself and heard on the X1 before switching (§3.2).
3. **Phones:** the full engine in wasm on the S22 – FM16 with simd is 29 µs on the PC (1 % of the budget); a phone at
   5× is still small. Without simd128 the batch path is slower than the patcher – simd128 is not optional.
4. **Safari:** simd128 (16.4+), SharedArrayBuffer with isolation (15.2+), no Web MIDI (ignored by decision),
   `Atomics.waitAsync` missing in older versions (the 2 ms timer fallback exists). Low risk.
5. **The build service is not deployed** (the owner's bunny step of #281): it blocks P10's authority and the creator's
   compile → Play proof of concept (P9b). It is the owner's step, not code.
6. **Compile / validate in a shared pod:** compiles yield to live shows (#281's rule); `/validate` must not – it is cheap
   (ms) and runs in its own request path. If the pod is down, publishing waits (503), drafts stay editable.
7. **Collision: "the stage must run BatchGraph" vs plan C.** C keeps the decision's meaning (one engine in the core,
   device I/O in the stage) and changes only which code becomes it – the owner confirms (D1).
8. **Collision: "migrate kind by kind behind a flag" vs one move.** P4 is an identity move guarded by bit-exact
   hashes, so the stage never runs a half-migrated engine; the kind-by-kind flags live where sound actually changes:
   the browser's allowlist (P6), `DECK_DSP` (P7), `DECK_CONTROL` (P5).
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
13. **Scope:** the stage's video graph is the next "universal core" gap (§1.4.5) and is deliberately not in this plan.

## 8. Decisions needed from the owner
1. **The engine's source: move the stage's engine into a new core crate `deck-engine` and merge `BatchGraph`'s API into
   it (plan C).** Recommended. Alternatives: grow `BatchGraph` (A) – ≈ +2 L tickets and a re-earned sound per kind;
   wrap fundsp (B) – no bit-exact goldens, larger wasm. Putting it literally inside deck-core costs moving deck-events in
   too (2,560 lines, a cycle otherwise).
2. **Control at block rate (128 frames) in both targets.** Recommended (deterministic, one cadence). Alternative: keep
   per-frame on the stage – then stage and browser can never match for modulated sounds.
3. **fundsp out of the runtime, kept as the test oracle.** Recommended. Alternative: fundsp in the core for both
   targets – ≈ +100 KB gz wasm and libm differences between targets.
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
8. **ABI v2 once** (sensors + character parameters), the host reading v1 and v2 until stored modules are recompiled.
   Recommended. Alternative: append-only v1 extensions – not possible without moving `audio` in `Io`.
9. **Chapter 3 guard:** push the 53 pending commits by Nov 8 (or hold them to Nov 16); no live-core-changing push
   Nov 9–15; the refactor yields its slot to release work Nov 9–15. Recommended.
10. **Model for P4 and P6 (L):** Sonnet 5.5 high per `models.json` (≈ $8–12 each), or name them for Opus (code.design,
    ≈ $15–25 each). Recommended: Sonnet; Opus only if P0 shows surprises.

## 9. Ticket changes (for the owner to approve; agents do not set the board's order)
**Re-scope**
- **#309** – add (b) native ≡ wasm under wasmtime, (c) the lab spike (§3.8), the recorded stage hashes; drop "glitch"
  from the kinds; it is P0, first.
- **#310** – becomes the engine tracker: plan C (`deck-engine`), the 128-frame quantum, the bit-exact goldens; its
  steps are P4, P5, P6, P7 (the new tickets below); "grow BatchGraph" and "glitch" leave its text.
- **#308** – steps 1 only (P3); steps 2–4 move to the new show-clock ticket.
- **#305** – step one adds the module tests to deck:ci; the browser backend follows #306.
- **#289** – note: ABI v1 already has control outputs; host-only work.
- **#290** – depends on P6 and #306 (no longer on "a BatchGraph hook").
- **#313** – the authority at publish (D6), depends on #302 and #281's owner step; `NodeKinds.php` generated from the
  catalog until then.
- **#312** – its descriptor field `needs` lives in the catalog's `Descriptor`; the `Io` change is ABI v2.
- **#311** – add lines: the stage's control domain (P4/P5), the stage's video graph (later leg), the lead rule,
  `space.ts`'s fixed BPM, live-core's unproven no-alloc.
- **#288** – steps 1 and 4 are P6; it keeps steps 2, 3, 5.
- **#64** – no change; a comment that the deck leg needs no release work and yields Nov 9–15.

**Close:** none now (#290 stays on hold; #306 waits for the owner's devices).

**Create** (labels: `area:deck` unless said; complexity as given)
1. *The kind catalog: one Descriptor for built-in kinds and modules, exported as JSON Schema, kinds.json, TS and PHP* –
   feature, M: five Rust kind tables and the TS / PHP twins become one `Descriptor` table in deck-space; generated
   files `--check`ed in deck:ci; the API serves kinds.json (owner 2026-11-02 "batchgraph can document their schema in
   rust structs").
2. *The engine move: the stage's audio engine into the new core crate deck-engine (no behaviour change)* – feature, L:
   bit-exact against the recorded stage hashes; the stage keeps device I/O only.
3. *One control half at block rate: BatchGraph's API on the engine, DECK_CONTROL flag, snapshot ring for the picture* –
   feature, M.
4. *The browser runs deck-engine: every kind in live-core, stereo, simd128, lower() reduced, TS DSP retired* – feature,
   L; not pushed Nov 9–15.
5. *Core DSP replaces fundsp: band-limited saw / square / triangle, seeded white / pink noise, SVF lowpass; fundsp as
   the test oracle* – feature, M.
6. *Show clock over the relay: a tempo anchor message, the clock-offset filter in deck-core, the browser source* –
   feature, M/L (from #308 steps 2–4).
7. *The native stage plays rigs (flatten → engine), the same path as the browser* – feature, M.
8. *Module ABI v2: sensor channels and the character's parameters; the host reads v1 and v2; stored modules
   recompiled* – feature, M; after #312 / #314's design notes.
9. *Retire the reference evaluators (ControlGraph, SceneDef, synth.rs blocks, batch.rs) and pin deck/lab* – chore, S.

**Recommended order** (top down, for the owner to set on the board): #309 → #305 → catalog → #308 (step 1) → engine
move → control half → (release pause Nov 9–15; #312 / #314 design notes if a slot is free) → browser on the engine →
core DSP → show clock → stage plays rigs → #306 decision → #290 → #289 → #307 → #302 → #313 → ABI v2 → retire.
