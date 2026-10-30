Status 2026-10-30: the morning report of the #182 night run (merged to main). #183 decided and built (the stage runs on the batch model: deck/stage/README.md); decisions waiting: #184–#187.

# LIVE night run 2026-10-29 – the full picture, the Live stage on a deck, a modular core in Rust and wasm

## 0. One screen

**What came out**
- **Built (tested, 4 commits on the branch):**
  - The relay does everything B.6 decided. It refuses messages from the wrong role, rate-limits the audience, gives each role its snapshot, replays what a reconnecting phone missed, runs votes by itself (majority / per group / first to n, fallback, override, tally every 250 ms), adds the clock lead, splits cues per outlet, sends presence, checks HMAC session tokens.
  - The bridge: the clock offset is now the median of 5 pings, device messages wait in a queue until their `at` (a far one no longer blocks), and `panic` silences every device.
  - `deck/core`: a small core with modules as cargo features, the same crate on the deck and in a browser worker.
- **Measured** (§4, one machine, numbers in `deck/lab/results`): **the architecture matters more than the language.**
  - 16 FM voices: from 133 µs to 5 µs native. Fixing the maths gave 4×, batch + vectorising another 6.5×.
  - In a browser worker, the Rust → wasm core is 2.5–5.7× faster than the same algorithms in TypeScript. JSON is the exception: TypeScript wins there.
  - Dynamic modules: a dylib costs nothing, a wasm module in wasmtime 1.0–2.4×, but nothing can draw on the GPU from inside wasm.
- **The picture** (§1): the 2026-10-07 Live plan holds. 2026-10-29 replaced how it runs (Rust, decks), not what it does. The open contradictions are listed with their fix.
- **The deck** (§2): one deck = relay + bridge + one native stage per outlet. The conductor and the phones stay web. The two clocks map onto relay time.

**Look at first:** §4's tables, then `deck/core/src/batch.rs` (the engine model, 300 lines), then the relay's `session.rs`.
The lab page: `deck/lab/README.md` (run it on the X1 too – these numbers are a Ryzen desktop).

**Decisions waiting (all `decide`, assigned to you):**
- #183 graph architecture: batch (recommended) or patcher; x86-64-v3 builds.
- #184 what runs as wasm in the browser Live mode.
- #185 modules: features + scene-driven builds now, wasm modules later, no dylibs.
- #186 show media on a deck + one concept for storyboard / slice editor / Live.
- #187 the USB-DMX interface for the workshop.

## 1. The concept picture

### 1.1 Decided (what holds)
| Since | Decision | Where |
|---|---|---|
| 2026-10-07 | A show is its own entity `show:<toon>:<n>`. The toon's flow decides everything; the show only adds its stage layer, in-between rows and performance overrides. Rows are never launched ("are we rebuilding Ableton?" – no) | LIVE B.5.1, B.11 (9) |
| 2026-10-07 | Logical outlets (screens, groups, devices) + venue patches; audience guest-first; groups by QR; votes = rule + time + fallback; DMX = fixtures + looks; the `live` role; rehearsal with a simulated room, bots, jump + record | LIVE B.5, B.11 |
| 2026-10-07 | The relay protocol B.6: envelope, roles, `to`, snapshot / replay, clock (ping median, `at` + lead) | LIVE B.6 |
| 2026-10-14 | The score view (Verovio) is the musicians' view; parts on / off and annotations come with Live (SCORES §6a, §7) | SCORES |
| 2026-10-19 | One shared Timeline for creator, scores and Live, read-only for a conductor; musical time only as tick functions | TIMELINE §1, §2.4 |
| 2026-10-20 | Live restarts 2026-11-16 for the Top Toy workshop 2027-02-17 (the full Must list; code complete Feb 5, freeze Feb 12) | KANON §8 |
| 2026-10-29 | Rust first: relay + bridge in `deck/`, no Node + Electron; decks on Arch (live stick first); beta = the workshop, public = a summer 2027 tour; lasers are a deck output; deck access = handshake → approval → data; two clocks (audio time base + tempo clock) | DECISIONS 2026-10-29, CYBERDECKS §1–§9 |

### 1.2 Built (before tonight and tonight)
- Before: `deck/protocol` (one schema → TS + Rust), `relay` skeleton, `bridge` + `midi` + `osc` adapters, the class-A live ISO (#173), `deck/stage` S1–S3 (#179), deck identity (#180), and FX cues, score view, cave, Timeline in the web apps.
- Tonight: §5.
- **Not built anywhere:** the reader's event bus (LIVE B.1 – the foundation of everything Live), the show entity and its schema, the conductor, the session API (`POST /api/live/sessions`, which signs tokens), outputs, the stage builder.

### 1.3 Contradictions and their fix
1. **Dates.** LIVE B.10 / B.11 (10) still say "Nov 2 → show-ready Feb 1", and B.10 phase 0 / 1 / 4 still say Electron, `live/` and serialport. Fix: B.10 is void; KANON §8 lane 2 + milestone "Cyberdecks" rule. LIVE.md now says so at B.10 (this branch).
2. **"No score rendering".** LIVE B.5.6 and B.11 (12) still say it; SCORES built the score view on 2026-10-14 for musicians. Fix: musicians get the score view where a score item exists, the free-form sheet otherwise. Noted at B.5.6.
3. **USB-DMX.** It is a Must for the workshop (B.11 (17)), yet the Rust plan names only `dmx-artnet`. → #187.
4. **Hosted relay.** B.4 says a Node relay in the bunny container; CYBERDECKS §4 says the same Rust binary. The decision (the bunny app) holds and the runtime changed – marked at B.4.
5. **Outputs.** LIVE B.5 has a browser per outlet with three.js and Web Audio; CYBERDECKS §5 / §8 has native on a deck; #182 adds wasm in a worker. They fit as three tiers (§2.3): a deck renders natively, any other machine renders in the browser, and the browser's heavy parts run as wasm in a worker. → #184, #186.
6. **Clocks.** B.6 has the relay's ms clock; TIMELINE has "musical time never as a second axis"; #181 has a tempo clock as nodes. They fit: ms is the axis everywhere, the tempo clock is a derived layer (beat = f(ms)), and the relay carries ms only (§2.4).
7. **"oss".** #179 read it as ossia score; tonight's owner note says "object oriented sound / audio / video generators". Both lead to the same thing: generators as objects with typed ports in a graph. ossia stays a model, not a dependency.

### 1.4 Superseded by 2026-10-29
The `live/` Node workspace, the Electron app "Scutoons Live" (RtMidi, serialport, electron-updater), the Node relay in the container, a browser per outlet on decks, Web Audio on decks, the B.10 dates. **Not superseded:** the protocol, the show model, the audience rules, the `live` role, DMX looks, the hosted bunny app as a deployment.

### 1.5 Owner wishes not captured yet
- Storyboard + slice editor + Live views as one concept (#186).
- The modular core (#183, #185).
- Decks as gadgets people buy (DECISIONS 2026-10-29, no plan yet).
- Laser tracing beyond CYBERDECKS §4 (#178).
- Score annotations "important feature" (SCORES §7; LIVE has no section yet).

## 2. The Live stage on a cyberdeck

### 2.1 Roles → where they run
| Live role / output | On a deck | Elsewhere | Why |
|---|---|---|---|
| Relay | native `deck/relay` – the venue's primary (LIVE B.4 linked) | the same binary in bunny (`scutoons-live`) | one code, timing |
| Bridge (MIDI, OSC, DMX, laser, GPIO) | native `deck/bridge` | the same binary on a mac / Windows laptop | hardware |
| **Screen outlet** | **native `deck-stage`**: joins the relay as `output` of its outlet, plays that outlet's cells as a stage scene | a fullscreen browser output (`/live/<code>/out/<outlet>`) with wasm in a worker (#184) | throughput, offline, one surface per outlet |
| Audio outlet / FX cues / chapter audio | `deck-stage`'s audio graph on PipeWire | Web Audio (+ AudioWorklet wasm) | latency, the clock master |
| Conductor | web, in the deck's Chromium (F2) or on a laptop / tablet | web | always the deployed version |
| Performer, musician, audience | – | web on their phones / tablets (unchanged) | joined by URL |
| Light (DMX), lasers | bridge adapters | – | hardware, 40 Hz refresh |

**One process or two on a deck?** Today stage and bridge are separate binaries. MIDI clock out and tempo-locked DMX need the audio time base (#181), so on a deck with a screen the bridge's adapters should load as a **module of deck-stage** (one process, one clock). The bridge binary stays for decks without a screen (class C) and for laptops. The adapters already share one trait (`adapters-core`), so this is a link-time choice, not a rewrite (#183).

### 2.2 How show media reaches a deck's outlet (LIVE B.5.6, CYBERDECKS §9) – proposal #186
1. The venue patch maps the outlet `left` to the deck `scutoons-k7.local` (a screen outlet with the deck's class).
2. At doors the conductor sends `media.preload { outlet: left, urls }` (the relay routes it to group `left`).
3. The deck fetches each URL with its deck token. Show media (stems, high-res video, glTF) live under `live/<toon>/` on the CDN and need the token; slices come from the public library. It caches them on DECKDATA and answers `ready` – the conductor's ready light.
4. On a row change the conductor sends `cell { outlet: left, cell }`. The stage maps the cell's layers to nodes: `scroll` / `slice` / `panel` → the `chapter` node, `video` → `video` (S4), `model3d` → `model3d` (S5), `text` / `surtitles` → `text`, `colour` → a solid, `camera` → `video-in`. It crossfades with the layer's `enter` / `exit`.
5. A deck that reconnects gets its outlet's snapshot (built tonight) and shows the right cell at once.

### 2.3 Storyboard, slice editor, Live – one concept (proposal, #186)
One flow, three lenses on the **same rows** (the scene keys of the creator's FlowIndex) and the **same Timeline**:

| Lens | Answers | Edits |
|---|---|---|
| Storyboard | what is drawn | frames → generated art → slices |
| Slice editor | what a reader sees | slices, text boxes, events |
| Stage builder (Live) | what every outlet and device does when the flow is here | cells, layers, cues, looks, device messages |

Moving between lenses keeps the selected row. A storyboard frame or a slice is a pickable layer source in the stage builder (`ToonSrc` already models slices / panels). A deck is a screen outlet with its class badge. Rehearsal (B.8) shows the decks' outlets as tiles, or as the real deck where one is patched. Needs a design draft before building (RULES "Working with the owner").

### 2.4 The clocks across the room
- **Relay time** (ms, B.6) is what the network agrees on. Every client keeps its offset (the median of 5 pings – the bridge does it since tonight).
- **The deck's time base** (#181) is the audio position. The stage maps relay `at` → local ms (offset) → sample time, so a cue lands on an audio block, not on a frame.
- **The tempo clock** (#181) is derived (BPM, beat = f(ms)). A cue's `wait` in beats converts to ms in the conductor (B.6). MIDI clock out runs from the deck's audio clock. One master per room: the conductor's choice (internal, Ableton Link, MIDI clock in).

## 3. Architecture – what the fast tools do and what we take
| System | Model | What we take |
|---|---|---|
| Max/MSP, Pure Data | patcher: objects + cords; a message domain (events, control) and a signal domain (64-sample blocks); `gen~` compiles a sub-patch to one tight function | **two domains** (control per frame, audio per block); compiling hot sub-graphs |
| TouchDesigner | operator families by data type (TOP GPU textures, CHOP channels, SOP geometry, DAT tables, COMP); pull-based cooking (only what a viewer / output needs, only when inputs change) | **typed ports by domain**; **cook on demand** (skip nodes nothing reads); channels as arrays (= our batches) |
| Ableton Live / Bitwig | tracks with device chains; sample-accurate automation; Link for tempo; Bitwig Grid compiles modular patches | the tempo clock + Link (#181); **devices as compiled modules** |
| Game engines (Bevy, Unity DOTS, Unreal) | ECS: data in arrays per component, systems loop over them in parallel; a frame / render graph schedules GPU passes | **struct-of-arrays batches** (`batch.rs`); a render graph for the stage's passes |
| vvvv gamma | visual patches compiled to .NET IL at edit time – the patch runs as compiled code | **compile the graph at load** (levels × kinds), never interpret per node |
| SuperCollider | server (scsynth, C++ unit generators, blocks) separate from the language; synth defs compiled once | the **core / UI split**: a fast server process (deck-stage) driven by messages (relay / OSC) |
| Faust | a DSP language compiled to C++ / wasm / LLVM | DSP as compiled code that also builds for the browser (= deck-core → wasm) |
| ossia score | intervals, processes, a scenario over time; a graph of typed ports | the time model next to the graph (the show's rows / cues) |

The lesson in one line: **the fast ones compile the patch.** Patchers interpret one object at a time; engines and compilers turn the graph into loops over arrays at load. Tonight's measurement confirms it for us (§4b).

## 4. The modular runtime – evaluation and measurements
Machine: Ryzen 5 3600X, Windows 10. Native with MSVC Rust 1.99. Node 22.14. Chrome 152 – the Browser pane, a background tab, cross-origin isolated. µs per call, median of 7 runs. **Not measured:** the X1 / Linux. Run `deck/lab` there before trusting the absolute numbers. The ratios are what this table is for.

### 4a. Language and host (the patcher architecture, the same code everywhere)
| Workload | native | dylib | wasmtime | wasm, Chromium worker | wasm simd128, worker | TypeScript, worker |
|---|---|---|---|---|---|---|
| Control step, 300 nodes (deep) | 3.9 | 3.7 | 9.2 | 8.4 | 6.7 | 21.3 |
| Audio block, 16 FM voices × 128 | 34.5 | 35.3 | 35.8 | 49.8 | 43.6 | 123.9 |
| FFT bands, 1024 | 9.2 | 9.4 | 11.6 | 10.6 | 12.6 | 38.9 |
| Relay codec, decode + encode 330 B | 5.7 | – | – | 6.9 | 6.6 | **3.4** |

Fixed costs:
- **wasm:** fetch + compile + instantiate 5–11 ms (300 KB gz; the wasm is 960 KB raw: the TOML parser, serde_json and rustfft's plans dominate – a binary scene format would cut it).
- **wasmtime:** cold compile 0.6 s (cacheable on disk), instantiate 72 µs.
- **Worker:** a postMessage round trip is 40 µs, copied or transferred (128 floats).

### 4b. Architecture (patcher vs batch) – the same scene, the same results
| Workload | native patcher | native batch | batch, x86-64-v3 (AVX2) | wasm patcher (worker) | wasm simd128 batch (worker) | TS patcher | TS batch |
|---|---|---|---|---|---|---|---|
| Control step, 300 nodes wide (3 levels × 100) | 3.6 | **2.0** | 2.1 | 9.0 | **2.8** | 15.6 | 5.1 |
| Control step, 300 nodes deep (300 levels) | 3.9 | 3.7 | 3.7 | 8.4 | 5.2 | 21.3 | 14.5 |
| Audio block, 16 FM voices × 128 | 34.5 | **8.9** | **5.1** | 49.8 | 29.4 | 123.9 | 111.0 |

Where the 133 → 5 µs came from (native audio, 16 voices):
- **133 µs:** the platform `sinf` and `fract`. MSVC's CRT is slow, and without SSE4.1 `round` / `fract` are library calls.
- **58 µs:** our own polynomial sine (`math.rs`, error < 1e-5).
- **34 µs:** branch-free, no int casts (magic-number rounding).
- **8.9 µs:** batch, with voices in arrays and the voice loop free of a carried sum, so the compiler vectorises it (4 lanes).
- **5.1 µs:** AVX2 (8 lanes).

The same trick gives wasm simd128 1.7× and TypeScript almost nothing (V8 doesn't vectorise). On a deck build this is "compile the patch" (§3) made concrete.

### 4c. Ways to load a module (for the deck)
| Way | Speed (vs compiled in) | Sandbox | Hot reload | GPU | Verdict |
|---|---|---|---|---|---|
| **Cargo features + scene-driven builds** (`deck-scene-features`) | 1.0× | – (our code) | restart (~1 s) | yes (wgpu) | **the default**: a class / a show gets exactly its modules; the pacman repo (#173) ships them |
| Data as modules: WGSL shaders, scenes, show documents | GPU speed | yes (it's data) | **yes, live** | yes | **the cheap module for visuals** – the stage already loads WGSL files |
| dylib, C ABI (libloading) | 1.0× | no | possible, unsafe | yes | no: no stable Rust ABI, a crash takes the stage down; abi_stable fixes the ABI, not the safety |
| **wasm in wasmtime** | 1.0–2.4× | yes | yes | **no** – compute only (control, audio, generators); WASI-gfx is not ready | **later**: third-party / user generators, safe hot reload |

Recommendation (#185): the core + compiled-in modules, visuals as data, wasm modules when someone outside writes generators. The registry (`module.rs`) already takes runtime modules (`Registry::add`), so this costs nothing now.

### 4d. The browser Live mode (Rust → wasm in a Web Worker)
| Part | Gain vs TypeScript | Use |
|---|---|---|
| Control graph (the show's control domain, the stage's node graph) | 2.5–5.7× | **yes**, in a worker, one step per frame |
| Analysis (FFT → control values) | 3.7× | **yes** |
| Generators / DSP | 2.5–4× | **yes**, in an AudioWorklet; the worker → worklet path is a SharedArrayBuffer ring, which needs COOP / COEP headers on the Live pages |
| Relay protocol codec | 0.5× (TS's JSON.parse is native) | **no** – keep the generated TS types + JSON |
| Text layout, slice compositing (wgpu / WebGPU in an OffscreenCanvas) | not measured | **no for now**: the reader renders slices; a WebGPU output only if a browser output drops frames |

"One crate, two targets" works: `deck-core` builds for the deck and for `wasm32` unchanged. Its FFT is the stage's FFT. Since #183 the stage uses deck-core's maths, `F8` lanes and batch columns, since #185 its FFT analyzer and the tempo map too.

## 5. What was built (branch `wip/live-night-75b685`)
| Commit | What | Tests |
|---|---|---|
| e1e601ab | relay: authority, rate limit, clock lead, snapshot per role, replay on resume (500 messages / 5 min, the same client id), votes (`vote.rs`), cue split, presence, assign, nicknames, HMAC tokens (`token.rs`); protocol: typed vote / snapshot / cell kinds | relay 25 + 5 WebSocket, protocol 4, shared/live 6 + tsc + generate --check |
| 6ec6c027 | bridge: clock offset from 5 pings, a timed queue, panic (MIDI CC 120 / 123) | bridge 5 + timing test against the real relay |
| 3c944c3a | `deck/core` (registry, modules, patcher graph, batch graph, math, FFT) + `deck/lab` (wasm, wasmtime, dylib, TS suite, worker page, results) | deck-core 19, clippy -D warnings |
| (this) | this report, LIVE.md stale lines, HANDOVER "Last run" | – |

## 6. Open (→ issues)
- #183–#187 (decide).
- #172 rest: the uplink to the hosted relay, pickups via the API, the dmx adapters, the status window, the dogma numbers.
- The session API that signs relay tokens (the format is in `deck/relay/src/token.rs`) – part of LIVE phase 2.
- The stage: join the relay as an output (§2.2), use `deck-core` (§4d), run the adapters in-process (§2.1) – after #183.
- The reader's event bus (LIVE B.1) – the first Live step on 2026-11-16, unchanged.
