The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-22 03:20 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 72.9 ms | 156 µs | 10 | 2.7 | 3.4 | 992 | 3.0 | 0.13 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 63.4 ms | 54 µs | 13 | 6.1 | 12.9 | 1409 | 6.9 | 0.48 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 168.7 ms | 63 µs | 27 | 22.5 | 49.9 | 1584 | 24.8 | 1.87 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 69.6 ms | 44 µs | 7 | 4.1 | 8.7 | 13183 | 7.8 | 0.33 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 83.8 ms | 41 µs | 116 | 38.5 | 171.8 | 2456 | 45.0 | 6.44 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 68.5 ms | 53 µs | 34 | 32.3 | 111.0 | 2243 | 35.8 | 4.16 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 69.4 ms | 37 µs | 3 | 0.5 | 0.7 | 186 | 0.6 | 0.03 % | 1088 | ebbc8117 |
| pc | wasmtime | delay | 128 | 186.6 ms | 834 µs | 19 | 15.9 | 74.0 | 2305 | 19.6 | 2.78 % | 3264 | f0a91488 |
| pc | wasmtime | reverb | 128 | 200.7 ms | 223 µs | 67 | 66.5 | 204.2 | 29200 | 113.0 | 7.66 % | 1536 | db60ac50 |
| pc | wasmtime | arp | 128 | 77.2 ms | 54 µs | 3 | 0.5 | 0.8 | 441 | 0.7 | 0.03 % | 1088 | 7cb0d9c9 |
| pc | wasmtime | drumseq | 128 | 107.1 ms | 80 µs | 6 | 0.7 | 1.5 | 103 | 0.8 | 0.06 % | 1088 | 0c3016e3 |
