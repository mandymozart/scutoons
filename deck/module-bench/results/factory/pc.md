The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-21 18:16 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 248.2 ms | 171 µs | 10 | 2.5 | 3.7 | 122 | 2.7 | 0.14 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 102.3 ms | 66 µs | 14 | 5.6 | 43.8 | 25408 | 13.5 | 1.64 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 187.8 ms | 84 µs | 29 | 20.9 | 64.2 | 5096 | 24.6 | 2.41 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 74.1 ms | 46 µs | 7 | 3.8 | 5.9 | 34459 | 13.7 | 0.22 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 157.8 ms | 45 µs | 40 | 36.6 | 117.9 | 13345 | 45.8 | 4.42 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 81.1 ms | 61 µs | 34 | 29.3 | 204.7 | 14019 | 40.0 | 7.68 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 63.0 ms | 40 µs | 3 | 0.5 | 0.7 | 6 | 0.5 | 0.03 % | 1088 | ebbc8117 |
