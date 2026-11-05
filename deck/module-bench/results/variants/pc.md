# wasmtime variants (pc, #396) – median of 3 rounds; mean µs (p50) per sample

| variant | modules | analog x2 | fm16 | fm16 ×1024 | walk | fm16 / analog vs deck | bits = deck | bits = Chrome |
|---|---|---|---|---|---|---|---|---|
| deck | plain | 779.9 (729.3) | 100.8 (86.1) | 708.4 (673.0) | 9.8 (10.6) | 1.00× / 1.00× | yes | yes |
| nan-off | plain | 345.4 (302.2) | 46.0 (39.4) | 344.6 (315.7) | 4.5 (4.7) | 0.46× / 0.44× | yes | yes |
| epoch-off | plain | 760.3 (719.4) | 86.2 (74.6) | 625.9 (587.0) | 11.2 (11.2) | 0.85× / 0.97× | yes | yes |
| spectre-off | plain | 798.7 (738.4) | 93.6 (83.4) | 697.6 (674.2) | 10.6 (11.0) | 0.93× / 1.02× | yes | yes |
| opt-size | plain | 760.1 (730.1) | 98.1 (83.6) | 704.7 (674.1) | 9.8 (10.4) | 0.97× / 0.97× | yes | yes |
| nan-off+epoch-off | plain | 300.4 (276.8) | 30.9 (30.2) | 262.4 (242.7) | 4.0 (4.3) | 0.31× / 0.39× | yes | yes |
| all-off | plain | 297.3 (275.8) | 33.9 (30.2) | 250.2 (242.2) | 4.0 (4.3) | 0.34× / 0.38× | yes | yes |
| nan-pass | nan | 402.2 (379.4) | 51.7 (47.4) | 393.0 (384.3) | 5.0 (5.3) | 0.51× / 0.52× | yes | yes |
| nan-pass+deck | nan | 826.7 (788.5) | 98.0 (86.7) | 765.2 (700.9) | 10.2 (10.7) | 0.97× / 1.06× | yes | yes |
| simd | simd | 717.8 (680.9) | 89.8 (83.1) | 725.6 (673.8) | 10.3 (10.6) | 0.89× / 0.92× | yes | yes |
| simd+nan-off | simd | 290.5 (271.7) | 47.7 (39.6) | 322.6 (315.8) | 4.8 (4.9) | 0.47× / 0.37× | yes | yes |
| Chrome (pc-chrome.json, web) | plain | 291.4 (250.0) | 39.1 (30.0) | 296.8 (245.0) | 6.9 (5.0) | 0.39× / 0.37× | | |
| Node (pc-node.json, web) | plain | 304.2 (259.0) | 46.7 (40.4) | 337.4 (314.5) | 6.8 (6.8) | 0.46× / 0.39× | | |

- **deck**: the deck's config (NaN canonicalization, epoch)
- **nan-off**: cranelift_nan_canonicalization off
- **epoch-off**: epoch_interruption off (no budget)
- **spectre-off**: heap + table spectre mitigation off
- **opt-size**: cranelift_opt_level SpeedAndSize
- **nan-off+epoch-off**: both off
- **all-off**: NaN, epoch, spectre off
- **nan-pass**: NaN pass in the wasm, cranelift canonicalization off
- **nan-pass+deck**: NaN pass and the deck's config (the pass's own cost)
- **simd**: modules with simd128, the deck's config
- **simd+nan-off**: modules with simd128, canonicalization off

## V8 (Node 2× – pc-node-r1.json, pc-node-r2.json), mean µs (p50), the same hashes as above on every row

| runtime | modules | fm16 | fm16 ×1024 | walk | analog x2 |
|---|---|---|---|---|---|
| web r1 / r2 | plain | 46.4 (39.9) / 42.5 (39.3) | 320.3 (310.2) / 362.0 (315.0) | 5.4 / 5.1 | 273.6 (253.1) / 307.4 (257.7) |
| web-nan r1 / r2 | nan pass | 43.7 (40.5) / 43.8 (40.2) | 475.7 (482.7) / 346.9 (316.2) | 7.8 / 5.7 | 333.2 (300.0) / 326.0 (299.0) |
| web-simd r1 / r2 | simd128 | 53.5 (57.4) / 41.8 (39.3) | 323.4 (311.6) / 322.1 (310.5) | 5.3 / 6.0 | 262.5 (237.5) / 269.4 (238.5) |
