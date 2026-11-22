The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-22 00:46 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 115.7 ms | 164 µs | 11 | 2.5 | 3.1 | 105 | 2.5 | 0.12 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 69.9 ms | 84 µs | 13 | 5.6 | 14.3 | 419 | 6.3 | 0.54 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 179.9 ms | 220 µs | 29 | 21.0 | 127.9 | 42721 | 39.9 | 4.80 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 84.4 ms | 44 µs | 8 | 3.8 | 4.9 | 483 | 4.2 | 0.18 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 116.6 ms | 116 µs | 41 | 36.9 | 241.5 | 17644 | 55.1 | 9.06 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 91.9 ms | 53 µs | 33 | 28.9 | 149.5 | 16514 | 41.5 | 5.61 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 72.8 ms | 41 µs | 4 | 0.5 | 0.6 | 4 | 0.5 | 0.02 % | 1088 | ebbc8117 |
| pc | wasmtime | arp | 128 | 77.2 ms | 54 µs | 3 | 0.5 | 0.8 | 441 | 0.7 | 0.03 % | 1088 | 7cb0d9c9 |
