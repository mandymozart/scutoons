The small audio modules, each alone and busy (src/factory.rs), 5000 calls of 128 frames at 48 kHz (budget 2667 µs). AMD64 Family 23 Model 113 Stepping 0, AuthenticAMD · 2026-11-22 08:31 UTC

| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pc | wasmtime | env | 128 | 76.2 ms | 154 µs | 9 | 2.4 | 14.1 | 2626 | 4.0 | 0.53 % | 1088 | 6142333b |
| pc | wasmtime | vca | 128 | 66.0 ms | 73 µs | 14 | 5.7 | 33.6 | 868 | 7.0 | 1.26 % | 1088 | 8c63258c |
| pc | wasmtime | filter | 128 | 144.8 ms | 182 µs | 28 | 21.1 | 70.1 | 1127 | 23.4 | 2.63 % | 1088 | aa735601 |
| pc | wasmtime | noise | 128 | 74.1 ms | 41 µs | 7 | 3.8 | 5.7 | 110 | 3.9 | 0.21 % | 1088 | 8d09631f |
| pc | wasmtime | osc | 128 | 78.9 ms | 46 µs | 39 | 37.0 | 128.9 | 2126 | 41.8 | 4.83 % | 1088 | 2b669c72 |
| pc | wasmtime | mixer | 128 | 71.5 ms | 61 µs | 32 | 29.7 | 47.0 | 165 | 29.5 | 1.76 % | 1088 | 89b4935b |
| pc | wasmtime | transpose | 128 | 53.3 ms | 44 µs | 4 | 0.6 | 1.2 | 74 | 0.7 | 0.04 % | 1088 | ebbc8117 |
| pc | wasmtime | delay | 128 | 115.1 ms | 732 µs | 19 | 15.5 | 30.7 | 168 | 15.8 | 1.15 % | 3264 | f0a91488 |
| pc | wasmtime | reverb | 128 | 127.9 ms | 202 µs | 69 | 66.2 | 148.8 | 905 | 70.2 | 5.58 % | 1536 | db60ac50 |
| pc | wasmtime | arp | 128 | 85.1 ms | 56 µs | 3 | 0.5 | 0.7 | 30 | 0.5 | 0.03 % | 1088 | 7cb0d9c9 |
| pc | wasmtime | drumseq | 128 | 69.3 ms | 72 µs | 6 | 0.6 | 1.4 | 263 | 0.9 | 0.05 % | 1088 | 0c3016e3 |
| pc | wasmtime | convolver | 128 | 173.4 ms | 62437 µs | 281 | 620.5 | 1636.2 | 14098 | 613.7 | 61.36 % | 10432 | 99cc8ac6 |
| pc | wasmtime | cloud | 128 | 95.6 ms | 46 µs | 14 | 8.7 | 14.1 | 91 | 8.6 | 0.53 % | 1088 | 0212283c |
| pc | wasmtime | bamboo | 128 | 140.3 ms | 96 µs | 158 | 87.6 | 519.3 | 199900 | 275.2 | 19.47 % | 1152 | 56fe1a46 |

The cloud row: 2026-11-22 ~07:40 UTC, the same PC – the storm preset now raining (0.8): 72 drops after the puffs (#780 slice 2; its other rows of that run are not taken).

The bamboo row: 2026-11-22 ~10:06 UTC, the same PC under load from another session (every row of that run had a high p99, e.g. osc 476 µs) – the storm preset: 50 stalks (600 instances), two 64-frame ticks a call; p50 is the honest number (#780; its other rows of that run are not taken).
