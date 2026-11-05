# wasmtime variants (pc-light, #396) – median of 3 rounds; mean µs (p50) per sample

| variant | modules | analog x2 | fm16 | fm16 ×1024 | walk | fm16 / analog vs deck | bits = deck | bits = Chrome |
|---|---|---|---|---|---|---|---|---|
| deck | plain | 1551.3 (983.9) | 239.2 (121.7) | 1489.1 (1021.7) | 17.6 (13.4) | 1.00× / 1.00× | yes | yes |
| nan-off | plain | 762.9 (470.6) | 87.0 (58.4) | 719.3 (488.8) | 8.7 (6.6) | 0.36× / 0.49× | yes | yes |
| nan-pass | nan | 946.1 (563.4) | 98.6 (67.2) | 821.7 (558.4) | 7.9 (7.0) | 0.41× / 0.61× | yes | yes |
| nan-light | nan-light | 727.2 (470.6) | 99.0 (64.8) | 801.1 (555.0) | 7.9 (6.9) | 0.41× / 0.47× | yes | yes |
| nan-pass+loops | nan-loops | 877.6 (547.6) | 93.7 (61.2) | 826.5 (520.0) | 8.5 (7.3) | 0.39× / 0.57× | yes | yes |
| native Rust (no wasm) | rlib | 565.9 (368.7) | 76.3 (46.8) | 631.6 (396.7) | – | 0.32× / 0.36× | yes | yes |
| Chrome (pc-chrome.json, web) | plain | 291.4 (250.0) | 39.1 (30.0) | 296.8 (245.0) | 6.9 (5.0) | 0.16× / 0.19× | | |
| Node (pc-node.json, web) | plain | 304.2 (259.0) | 46.7 (40.4) | 337.4 (314.5) | 6.8 (6.8) | 0.20× / 0.20× | | |

- **deck**: the deck's config (NaN canonicalization, epoch)
- **nan-off**: cranelift_nan_canonicalization off
- **nan-pass**: NaN pass in the wasm, cranelift canonicalization off
- **nan-light**: the NaN pass without the store points (reinterpret, copysign); canonicalization off
- **nan-pass+loops**: NaN pass + loop counter in the wasm; Cranelift canonicalization and epoch off (option C + the counter)
- **native Rust**: the module crates compiled for this machine, run by `deck_module::testing::Rig` (src/native.rs)

**Measured under load** (another process on ~2 cores, 2026-11-05): every row 1.5–2× slower than pc.md / pc-loops.md (native 566 instead of 286) – compare ratios within this table only.
