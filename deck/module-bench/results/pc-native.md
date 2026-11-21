| device | runtime | workload | frames | compile | inst. | first | p50 µs | p99 µs | max µs | mean µs | p99 % budget | units / sample | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | fm16 | 128 | 48.5 ms | 113 µs | 90 | 86.2 | 151.9 | 961 | 103.4 | 5.7 % | – | 1088 | 1b3d03e4 |
| pc | wasmtime | fm16 | 1024 | 46.9 ms | 40 µs | 992 | 971.7 | 1249.9 | 1756 | 892.8 | 5.9 % | – | 1088 | 1b3d03e4 |
| pc | wasmtime | walk | 128 | 68.4 ms | 42 µs | 25 | 10.7 | 25.2 | 126 | 10.3 | 0.9 % | – | 1088 | d074122c |
| pc | wasmtime | analog x2 | 128 | 123.8 ms | 54 µs | 948 | 808.5 | 1232.2 | 2264 | 868.9 | 46.2 % | – | 1088 | 2fa5da63 |
| pc | wasmi | fm16 | 128 | 1.4 ms | 283 µs | 388 | 430.1 | 535.0 | 1623 | 419.5 | 20.1 % | 2410427 | 1088 | 1b3d03e4 |
| pc | wasmi | fm16 | 1024 | 1.2 ms | 354 µs | 3691 | 3476.9 | 3895.3 | 4195 | 3368.1 | 18.3 % | 19283420 | 1088 | 1b3d03e4 |
| pc | wasmi | walk | 128 | 1.3 ms | 280 µs | 62 | 32.3 | 59.5 | 81 | 28.2 | 2.2 % | 60295 | 1088 | d074122c |
| pc | wasmi | analog x2 | 128 | 2.6 ms | 280 µs | 3255 | 2617.5 | 4965.7 | 10710 | 2679.9 | 186.2 % | 10916857 | 1088 | 2fa5da63 |

| device | runtime | timer µs | runaway stopped by | budget | stopped after | past budget | show restart | grow results (pages / -1) | memory KB before → after | grow fault |
|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | 0.1 | epoch | 2 ms | 3.2 ms | 1.2 ms | 115 µs | -1 -1 -1 -1 -1 -1 -1 -1 | 1088 → 1088 | none |
| pc | wasmi | 0.1 | fuel | 2 ms | 11.1 ms | 9.1 ms | 383 µs | -1 -1 -1 -1 -1 -1 -1 -1 | 1088 → 1088 | none |

Bit equality (every output sample's bits hashed):
- analog x2 / 128: equal on all 2 runs
- fm16 / 1024: equal on all 2 runs
- fm16 / 128: equal on all 2 runs
- walk / 128: equal on all 2 runs

| file | bytes | gz |
|---|---|---|
| analog.gas.wasm | 129117 | 36125 |
| analog.loops-max.wasm | 115372 | 33627 |
| analog.loops.wasm | 115363 | 33623 |
| analog.nan-light.wasm | 111877 | 33500 |
| analog.nan-loops.wasm | 120219 | 33985 |
| analog.nan.wasm | 116019 | 33743 |
| analog.simd.wasm | 112932 | 33383 |
| analog.wasm | 114372 | 33635 |
| bench-web-wasmi.wasm | 1818532 | 476516 |
| bench-web.wasm | 477266 | 173787 |
| chain-merged.wasm | 27277 | 10914 |
| chain-stage.wasm | 26361 | 10703 |
| fm16.gas.wasm | 35902 | 14485 |
| fm16.loops-max.wasm | 31722 | 13198 |
| fm16.loops.wasm | 31713 | 13197 |
| fm16.nan-light.wasm | 29529 | 13082 |
| fm16.nan-loops.wasm | 32545 | 13309 |
| fm16.nan.wasm | 29915 | 13126 |
| fm16.simd.wasm | 32258 | 13777 |
| fm16.wasm | 30838 | 13153 |
| stress.gas.wasm | 28489 | 11275 |
| stress.loops-max.wasm | 25146 | 10243 |
| stress.loops.wasm | 25137 | 10243 |
| stress.nan-light.wasm | 23233 | 10114 |
| stress.nan-loops.wasm | 25568 | 10293 |
| stress.nan.wasm | 23273 | 10125 |
| stress.simd.wasm | 26685 | 10953 |
| stress.wasm | 24493 | 10208 |
| walk.gas.wasm | 39091 | 15841 |
| walk.loops-max.wasm | 34592 | 14561 |
| walk.loops.wasm | 34583 | 14559 |
| walk.nan-light.wasm | 32823 | 14456 |
| walk.nan-loops.wasm | 35706 | 14711 |
| walk.nan.wasm | 33250 | 14523 |
| walk.simd.wasm | 34844 | 15058 |
| walk.wasm | 33878 | 14506 |
| live_core.wasm | 1784501 | 604230 |

Gates (#306, suggested):
- [open] FM16 p99 ≤ 25 % of the block budget on the S22 (browser WebAssembly): to be filled from the S22
- [open]   FM16 p99 % on pc / wasmtime: 5.7 % of 2667 µs
- [open]   FM16 p99 % on pc / wasmi: 20.1 % of 2667 µs
- [open] gas instrumentation ≤ 1.5×: not measured
- [FAIL] wasmi's bundle growth ≤ 150 KB gz: 296 KB gz
