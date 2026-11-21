The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-21 22:22 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 88.7 ms | 160 µs | 10 | 2.5 | 2.7 | 11 | 2.5 | 0.10 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 86.2 ms | 70 µs | 16 | 5.6 | 11.0 | 13460 | 9.2 | 0.41 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 164.9 ms | 84 µs | 30 | 21.0 | 34.5 | 30126 | 35.5 | 1.29 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 68.7 ms | 42 µs | 8 | 3.8 | 4.6 | 15344 | 6.9 | 0.17 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 94.3 ms | 46 µs | 39 | 36.9 | 88.8 | 29791 | 53.0 | 3.33 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 93.5 ms | 60 µs | 31 | 29.8 | 55.9 | 1887 | 32.3 | 2.10 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 72.2 ms | 43 µs | 4 | 0.6 | 0.7 | 16284 | 3.8 | 0.03 % | 1088 | ebbc8117 |
| pc | wasmtime | reverb | 128 | 200.7 ms | 223 µs | 67 | 66.5 | 204.2 | 29200 | 113.0 | 7.66 % | 1536 | db60ac50 |
