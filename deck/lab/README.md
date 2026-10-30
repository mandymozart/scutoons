# deck/lab – the #182 / #184 experiments (its own Cargo workspace, not built by CI)

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
