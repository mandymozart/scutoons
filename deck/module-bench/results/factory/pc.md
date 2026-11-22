The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-22 03:20 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 63.8 ms | 158 µs | 11 | 1.6 | 3.2 | 42 | 2.1 | 0.12 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 59.9 ms | 67 µs | 13 | 5.5 | 8.0 | 70 | 5.3 | 0.30 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 107.8 ms | 78 µs | 27 | 21.1 | 44.1 | 244 | 22.2 | 1.65 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 79.6 ms | 39 µs | 7 | 3.6 | 5.7 | 143 | 3.4 | 0.21 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 56.9 ms | 45 µs | 41 | 36.9 | 50.2 | 526 | 35.8 | 1.88 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 53.3 ms | 60 µs | 35 | 29.0 | 97.7 | 1947 | 31.4 | 3.66 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 84.9 ms | 38 µs | 3 | 0.5 | 0.9 | 105 | 0.7 | 0.03 % | 1088 | ebbc8117 |
| pc | wasmtime | delay | 128 | 164.9 ms | 813 µs | 19 | 15.4 | 119.0 | 9699 | 28.4 | 4.46 % | 3264 | f0a91488 |
| pc | wasmtime | reverb | 128 | 181.4 ms | 225 µs | 68 | 66.9 | 764.8 | 13186 | 105.7 | 28.68 % | 1536 | db60ac50 |
| pc | wasmtime | drumseq | 128 | 107.1 ms | 80 µs | 6 | 0.7 | 1.5 | 103 | 0.8 | 0.06 % | 1088 | 0c3016e3 |
