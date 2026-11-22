The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-22 05:58 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 80.6 ms | 156 µs | 9 | 2.5 | 6.9 | 4961 | 3.9 | 0.26 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 122.6 ms | 70 µs | 13 | 5.8 | 56.2 | 7939 | 12.6 | 2.11 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 194.6 ms | 78 µs | 26 | 20.9 | 145.4 | 15538 | 34.5 | 5.45 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 119.2 ms | 45 µs | 7 | 3.8 | 12.9 | 4770 | 7.1 | 0.48 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 115.6 ms | 53 µs | 39 | 36.9 | 1075.7 | 18436 | 88.4 | 40.34 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 101.4 ms | 73 µs | 32 | 30.2 | 176.2 | 5964 | 39.7 | 6.61 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 80.1 ms | 45 µs | 6 | 0.5 | 0.6 | 329 | 0.6 | 0.02 % | 1088 | ebbc8117 |
| pc | wasmtime | delay | 128 | 139.0 ms | 971 µs | 18 | 15.6 | 53.4 | 920 | 17.5 | 2.00 % | 3264 | f0a91488 |
| pc | wasmtime | reverb | 128 | 162.3 ms | 195 µs | 68 | 66.5 | 130.7 | 829 | 69.6 | 4.90 % | 1536 | db60ac50 |
| pc | wasmtime | arp | 128 | 73.1 ms | 52 µs | 2 | 0.6 | 0.9 | 13 | 0.6 | 0.03 % | 1088 | 7cb0d9c9 |
| pc | wasmtime | convolver | 128 | 157.2 ms | 54364 µs | 245 | 599.0 | 1627.3 | 27049 | 597.9 | 61.02 % | 10432 | 99cc8ac6 |
