| device | runtime | workload | frames | compile | inst. | first | p50 µs | p99 µs | max µs | mean µs | p99 % budget | units / sample | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | fm16 | 128 | 57.9 ms | 143 µs | 132 | 87.8 | 165.3 | 361 | 102.9 | 6.2 % | – | 1088 | 1b3d03e4 |
| pc | wasmtime | fm16 | 1024 | 51.1 ms | 29 µs | 688 | 931.9 | 1690.4 | 2759 | 887.3 | 7.9 % | – | 1088 | 1b3d03e4 |
| pc | wasmtime | walk | 128 | 54.2 ms | 43 µs | 26 | 12.4 | 26.6 | 54 | 11.2 | 1.0 % | – | 1088 | d074122c |
| pc | wasmtime | analog x2 | 128 | 133.7 ms | 40 µs | 771 | 845.9 | 1948.4 | 7897 | 881.7 | 73.1 % | – | 1088 | 2fa5da63 |
| pc | wasmi | fm16 | 128 | 2.0 ms | 362 µs | 393 | 377.9 | 619.9 | 1505 | 396.3 | 23.2 % | 2410407 | 1088 | 1b3d03e4 |
| pc | wasmi | fm16 | 1024 | 1.1 ms | 306 µs | 3203 | 3262.4 | 4841.2 | 6366 | 3290.7 | 22.7 % | 19283260 | 1088 | 1b3d03e4 |
| pc | wasmi | walk | 128 | 1.4 ms | 331 µs | 70 | 32.5 | 128.2 | 438 | 30.7 | 4.8 % | 60273 | 1088 | d074122c |
| pc | wasmi | analog x2 | 128 | 2.8 ms | 253 µs | 2148 | 2149.0 | 4569.9 | 9152 | 2195.5 | 171.4 % | 12975422 | 1088 | 2fa5da63 |

| device | runtime | timer µs | runaway stopped by | budget | stopped after | past budget | show restart | grow results (pages / -1) | memory KB before → after | grow fault |
|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | 0.1 | epoch | 2 ms | 1.6 ms | -394 µs | 116 µs | -1 -1 -1 -1 -1 -1 -1 -1 | 1088 → 1088 | none |
| pc | wasmi | 0.1 | fuel | 2 ms | 12.6 ms | 10.6 ms | 396 µs | -1 -1 -1 -1 -1 -1 -1 -1 | 1088 → 1088 | none |

Bit equality (every output sample's bits hashed):
- analog x2 / 128: equal on all 2 runs
- fm16 / 1024: equal on all 2 runs
- fm16 / 128: equal on all 2 runs
- walk / 128: equal on all 2 runs

| file | bytes | gz |
|---|---|---|
| analog.gas.wasm | 123908 | 35802 |
| analog.loops.wasm | 113242 | 33379 |
| analog.wasm | 112289 | 33395 |
| bench-web-wasmi.wasm | 1795456 | 468842 |
| bench-web.wasm | 454163 | 167444 |
| fm16.gas.wasm | 35750 | 14440 |
| fm16.loops.wasm | 31573 | 13156 |
| fm16.wasm | 30697 | 13108 |
| stress.gas.wasm | 28350 | 11236 |
| stress.loops.wasm | 25010 | 10212 |
| stress.wasm | 24365 | 10181 |
| walk.gas.wasm | 38935 | 15804 |
| walk.loops.wasm | 34439 | 14527 |
| walk.wasm | 33733 | 14472 |
| live_core.wasm | 1433693 | 509804 |

Gates (#306, suggested):
- [open] FM16 p99 ≤ 25 % of the block budget on the S22 (browser WebAssembly): to be filled from the S22
- [open]   FM16 p99 % on pc / wasmtime: 6.2 % of 2667 µs
- [open]   FM16 p99 % on pc / wasmi: 23.2 % of 2667 µs
- [open] gas instrumentation ≤ 1.5×: not measured
- [FAIL] wasmi's bundle growth ≤ 150 KB gz: 294 KB gz
