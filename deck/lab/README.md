# deck/lab – the #182 / #184 experiments (its own Cargo workspace, not built by CI)

> **Pinned (#324, 2026-11-04): the lab builds at commit `96f1ae9943a40e0965268ac2501b86771546a26b` (2026-11-03, the P5 merge)
> and not on `main`.** It is a snapshot of experiments, no longer maintained. It broke in two steps: `0bab7559` (live-core holds
> deck-engine, #320) gave `Core::load` a `channels` argument that `core-wasm` no longer fits, and #324 deleted what the
> lab measured (deck-core's patcher `ControlGraph`, `BatchGraph`, the `synth` and `control` blocks, `bench_scene`,
> `deck-scene-features --core`). To run it: `git worktree add ../lab-pin 96f1ae99` and follow "Run" below from its
> `deck/lab` (checked there with `cargo check --workspace --all-targets`). The results in `results/` stay valid as what
> was measured then. The engine's own numbers are `deck/engine/tests/bench.rs` and `deck/conformance`.

Results and what they mean: agents/research/LIVE-NIGHT-2026-10-29.md §4; the wasm boundary's exchange format (#184):
`results/formats.md`. Raw numbers: `results/`.

| Crate / folder | What |
|---|---|
| `core-wasm` | deck-core (`deck/core`) + the relay protocol for a browser Web Worker (wasm-bindgen): `Graph` (patcher), `Batch` (data-oriented), `Analyzer` (FFT bands), the codec |
| `core-raw` | deck-core behind a C ABI – for wasm32 a module wasmtime hosts, natively a dylib libloading loads |
| `bench` | the native paths: compiled in, dylib, wasmtime (`BENCH_LABEL` names a build with other RUSTFLAGS) |
| `ts/suite.ts` | the same workloads in TypeScript (patcher + batch) and through core-wasm; `ts/bench-node.ts` runs it in Node |
| `www/` | the worker page: `main.ts` (postMessage round trips), `worker.ts` (the suite in a worker) |
| `core-wasm/src/formats.rs`, `ts/formats.ts` | #184: the exchange formats (JSON, postcard, live-core's wire records copied / in place) into and out of wasm; `ts/bench-formats.ts` runs them in Node |
| `www/formats.html` | #184: the same in a worker + page → worker (postMessage vs the SAB ring) |
| `www/deck.html` | #197: a phone joins the Cathedral of decks as a being – since #198 the dev harness of `@scutoons/live-deck` (shared/live-deck: the relay client, the trimmed cathedral, the deck's state and sound, the being in 3D); the real page is the reader's `/live/deck`. `?space=1` draws the being with deck/space-web's wasm |
| `www/demo.html` | #184: `@scutoons/live-core` playing a scene, the FFT bands, a float vector sent live; `?transport=transfer` / `?coi=0` = the fallback |
| `ts/run-browser.ts` | formats.html + demo.html in a headless Edge / Chrome over plain CDP (own profile, port `LAB_DEBUG_PORT`, default 9531) → `results/formats-chromium.*`, screenshots |

## Run (from deck/lab)

```bash
cargo build --release -p core-raw && cargo build --release -p core-raw --target wasm32-unknown-unknown && cargo run --release -p bench
```

```bash
RUSTFLAGS="-C target-cpu=x86-64-v3" CARGO_TARGET_DIR=target-v3 cargo build --release -p bench && BENCH_LABEL=x86-64-v3 ./target-v3/release/bench
```

```bash
cargo build --release -p core-wasm --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir www/pkg target/wasm32-unknown-unknown/release/core_wasm.wasm
```

```bash
RUSTFLAGS="-C target-feature=+simd128" CARGO_TARGET_DIR=target-simd cargo build --release -p core-wasm --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir www/pkg-simd target-simd/wasm32-unknown-unknown/release/core_wasm.wasm
```

```bash
node --experimental-transform-types ts/bench-node.ts && node --experimental-transform-types ts/build.ts && node --experimental-transform-types ts/serve.ts 7790
```

Then open http://127.0.0.1:7790/ – the table appears after ~1 min (`window.__results`). Needs `rustup target add
wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.129`.

### #184: formats + the live-core demo

```bash
npm run build:wasm -w shared/live-core          # deck/target/wasm32-unknown-unknown/release/live_core.wasm
cargo build --release -p core-wasm --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir www/pkg target/wasm32-unknown-unknown/release/core_wasm.wasm
node --experimental-transform-types ts/bench-formats.ts 5      # Node → results/formats-node.json
node --experimental-transform-types ts/build.ts && node --experimental-transform-types ts/serve.ts 4131
node --experimental-transform-types ts/run-browser.ts http://127.0.0.1:4131 <profile dir> <shots dir> all
```

`serve.ts` sends COOP / COEP (`?coi=0` on a page: without). Under playwright (its launch or connectOverCDP) `audioWorklet.addModule` never
settles – most likely its auto-attach holding the worklet's new target; the same browser over plain CDP runs it, hence
run-browser.ts.

### #197: the browser deck on a phone (USB)

```bash
node --experimental-transform-types ts/build.ts && node --experimental-transform-types ts/serve.ts 4142
adb -s <device> reverse tcp:4142 tcp:4142 && adb -s <device> reverse tcp:7070 tcp:7070   # the phone's localhost = this PC (a secure context)
adb -s <device> shell am start -a android.intent.action.VIEW -d "'http://localhost:4142/deck.html?relay=ws://localhost:7070/live/CATH'"
```

It joins at once (`?relay=`, default `ws://<host>:7070/live/CATH`; `?character=<id>`, default yellowmouse or the last
pick); Start plays it; `?space=1` adds the 3D (`npm run build:wasm -w shared/live-deck` first – ts/build.ts copies it;
`?gl=1` = WebGL2 only). `LAB_DIST=<dir>` builds elsewhere (a check while www/dist is being served). Tests:
`npm run check -w shared/live-deck` (the relay test starts `deck/target/debug/relay.exe` on `LAB_RELAY_PORT`, default
7072); the conductor's side: `cargo test -p deck-space the_browser_decks_being` (`fixtures/deck-graph.json`).

### #198: the being in 3D on a phone – the reader's `/live/deck`

```bash
npm run build:wasm -w shared/live-deck        # space-web (+ live-core) → viewer/static/live/ (git-ignored)
cd viewer && npx vite dev --port 4143 --host 127.0.0.1   # VITE_API_URL / VITE_CDN_URL = production for the set + portraits
adb -s <device> reverse tcp:4143 tcp:4143 && adb -s <device> reverse tcp:7070 tcp:7070
adb -s <device> shell am start -a android.intent.action.VIEW -d "'http://localhost:4143/live/deck?relay=ws://localhost:7070/live/CATH'"
```

WebGPU where the browser has an adapter, else WebGL2 (the S22's Chrome: `navigator.gpu` without an adapter → WebGL2);
the status line at the bottom names the backend. `www/gpu.html` prints what a browser offers (WebGPU adapter, WebGL2).
