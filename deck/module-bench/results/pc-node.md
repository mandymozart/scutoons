| device | runtime | workload | frames | compile | inst. | first | p50 µs | p99 µs | max µs | mean µs | p99 % budget | units / sample | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc-node | web | fm16 | 128 | 423 µs | 837 µs | 743 | 40.4 | 89.4 | 743 | 46.7 | 3.4 % | – | 1088 | 1b3d03e4 |
| pc-node | web | fm16 | 1024 | 105 µs | 162 µs | 476 | 314.5 | 480.2 | 730 | 337.4 | 2.3 % | – | 1088 | 1b3d03e4 |
| pc-node | web | walk | 128 | 297 µs | 252 µs | 597 | 6.8 | 17.1 | 597 | 6.8 | 0.6 % | – | 1088 | d074122c |
| pc-node | web | analog x2 | 128 | 413 µs | 401 µs | 1592 | 259.0 | 519.4 | 1592 | 304.2 | 19.5 % | – | 1088 | 2fa5da63 |
| pc-node | web-gas | fm16 | 128 | 421 µs | 910 µs | 1306 | 204.1 | 323.0 | 1392 | 224.3 | 12.1 % | 557427 | 1088 | 1b3d03e4 |
| pc-node | web-gas | fm16 | 1024 | 94 µs | 218 µs | 1891 | 1606.1 | 2273.4 | 2444 | 1693.7 | 10.7 % | 4459418 | 1088 | 1b3d03e4 |
| pc-node | web-gas | walk | 128 | 282 µs | 200 µs | 592 | 17.6 | 40.9 | 592 | 17.3 | 1.5 % | 41299 | 1088 | d074122c |
| pc-node | web-gas | analog x2 | 128 | 423 µs | 302 µs | 2859 | 1109.1 | 2975.5 | 9220 | 1280.2 | 111.6 % | 2872651 | 1088 | 2fa5da63 |
| pc-node | web-loops | fm16 | 128 | 675 µs | 1.7 ms | 969 | 50.6 | 202.1 | 2123 | 62.9 | 7.6 % | 10268 | 1088 | 1b3d03e4 |
| pc-node | web-loops | fm16 | 1024 | 74 µs | 188 µs | 562 | 336.4 | 732.6 | 1289 | 388.3 | 3.4 % | 82144 | 1088 | 1b3d03e4 |
| pc-node | web-loops | walk | 128 | 289 µs | 177 µs | 452 | 6.4 | 14.8 | 452 | 6.2 | 0.6 % | 209 | 1088 | d074122c |
| pc-node | web-loops | analog x2 | 128 | 448 µs | 346 µs | 2200 | 274.6 | 544.5 | 3660 | 313.3 | 20.4 % | 17208 | 1088 | 2fa5da63 |
| pc-node | web-wasmi | fm16 | 128 | 29.9 ms | 3.8 ms | 3931 | 2810.9 | 4762.0 | 8119 | 2902.0 | 178.6 % | 2410407 | 1088 | 1b3d03e4 |
| pc-node | web-wasmi | fm16 | 1024 | 2.8 ms | 474 µs | 25623 | 23046.8 | 31492.2 | 39084 | 23430.8 | 147.6 % | 19283260 | 1088 | 1b3d03e4 |
| pc-node | web-wasmi | walk | 128 | 4.6 ms | 156 µs | 1495 | 219.1 | 465.8 | 1495 | 206.7 | 17.5 % | 60273 | 1088 | d074122c |
| pc-node | web-wasmi | analog x2 | 128 | 9.6 ms | 433 µs | 14976 | 13584.3 | 17350.2 | 23710 | 13648.4 | 650.6 % | 12975422 | 1088 | 2fa5da63 |

| device | runtime | timer µs | runaway stopped by | budget | stopped after | past budget | show restart | grow results (pages / -1) | memory KB before → after | grow fault |
|---|---|---|---|---|---|---|---|---|---|---|
| pc-node | web | 0.3 | watchdog (the parent terminates the worker) | 2 ms | 28.4 ms | 26.4 ms | 81.6 ms |  | 1088 → 2112 | dm_process: the memory grew past its freeze (1114112 → 2162688 bytes; the browser cannot refuse memory.grow) |
| pc-node | web-gas | 0.4 | gas | 2 ms | 6.8 ms | 4.8 ms | 213 µs |  | 1088 → 2112 | dm_process: the memory grew past its freeze (1114112 → 2162688 bytes; the browser cannot refuse memory.grow) |
| pc-node | web-loops | 0.4 | loop counter | 2 ms | 982 µs | -1018 µs | 138 µs |  | 1088 → 2112 | dm_process: the memory grew past its freeze (1114112 → 2162688 bytes; the browser cannot refuse memory.grow) |
| pc-node | web-wasmi | 0.4 | fuel | 2 ms | 15.7 ms | 13.7 ms | 132 µs | -1 -1 -1 -1 -1 -1 -1 -1 | 1088 → 1088 | none |

Bit equality (every output sample's bits hashed):
- analog x2 / 128: equal on all 4 runs
- fm16 / 1024: equal on all 4 runs
- fm16 / 128: equal on all 4 runs
- walk / 128: equal on all 4 runs

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
- [open]   FM16 p99 % on pc-node / web: 3.4 % of 2667 µs
- [open]   FM16 p99 % on pc-node / web-gas: 12.1 % of 2667 µs
- [open]   FM16 p99 % on pc-node / web-loops: 7.6 % of 2667 µs
- [open]   FM16 p99 % on pc-node / web-wasmi: 178.6 % of 2667 µs
- [FAIL] gas instrumentation ≤ 1.5× (pc-node web-gas / web, fm16 mean): 4.80×
- [PASS] gas instrumentation ≤ 1.5× (pc-node web-loops / web, fm16 mean): 1.35×
- [FAIL] wasmi's bundle growth ≤ 150 KB gz: 294 KB gz
