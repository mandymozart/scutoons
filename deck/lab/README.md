# deck/lab – the #182 experiments (its own Cargo workspace, not built by CI)

Results and what they mean: agents/research/LIVE-NIGHT-2026-10-29.md §4. Raw numbers: `results/`.

| Crate / folder | What |
|---|---|
| `core-wasm` | deck-core (`deck/core`) + the relay protocol for a browser Web Worker (wasm-bindgen): `Graph` (patcher), `Batch` (data-oriented), `Analyzer` (FFT bands), the codec |
| `core-raw` | deck-core behind a C ABI – for wasm32 a module wasmtime hosts, natively a dylib libloading loads |
| `bench` | the native paths: compiled in, dylib, wasmtime (`BENCH_LABEL` names a build with other RUSTFLAGS) |
| `ts/suite.ts` | the same workloads in TypeScript (patcher + batch) and through core-wasm; `ts/bench-node.ts` runs it in Node |
| `www/` | the worker page: `main.ts` (postMessage round trips), `worker.ts` (the suite in a worker) |

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
