# wasmtime variants (pc-loops, #396) – median of 3 rounds; mean µs (p50) per sample

| variant | modules | analog x2 | fm16 | fm16 ×1024 | walk | fm16 / analog vs deck | bits = deck | bits = Chrome |
|---|---|---|---|---|---|---|---|---|
| deck | plain | 825.3 (762.5) | 100.5 (85.4) | 806.5 (709.5) | 10.7 (11.1) | 1.00× / 1.00× | yes | yes |
| nan-off | plain | 352.1 (306.2) | 47.3 (39.7) | 379.5 (327.2) | 4.4 (4.7) | 0.47× / 0.43× | yes | yes |
| nan-off+epoch-off | plain | 313.8 (277.8) | 39.9 (32.6) | 276.6 (246.9) | 5.1 (5.4) | 0.40× / 0.38× | yes | yes |
| nan-pass | nan | 452.5 (391.8) | 58.0 (51.3) | 441.3 (409.0) | 5.7 (5.8) | 0.58× / 0.55× | yes | yes |
| loops | loops-max | 816.0 (752.2) | 104.0 (97.4) | 738.6 (639.3) | 10.1 (10.8) | 1.03× / 0.99× | yes | yes |
| nan-pass+loops | nan-loops | 436.7 (380.2) | 54.4 (44.8) | 379.0 (328.5) | 6.8 (6.9) | 0.54× / 0.53× | yes | yes |
| native Rust (no wasm) | rlib | 300.6 (260.9) | 33.8 (29.1) | 263.0 (235.5) | – | 0.34× / 0.36× | yes | yes |
| Chrome (pc-chrome.json, web) | plain | 291.4 (250.0) | 39.1 (30.0) | 296.8 (245.0) | 6.9 (5.0) | 0.39× / 0.35× | | |
| Node (pc-node.json, web) | plain | 304.2 (259.0) | 46.7 (40.4) | 337.4 (314.5) | 6.8 (6.8) | 0.46× / 0.37× | | |

- **deck**: the deck's config (NaN canonicalization, epoch)
- **nan-off**: cranelift_nan_canonicalization off
- **nan-off+epoch-off**: both off
- **nan-pass**: NaN pass in the wasm, cranelift canonicalization off
- **loops**: the loop counter in the wasm instead of the epoch (canonicalization on)
- **nan-pass+loops**: NaN pass + loop counter in the wasm; Cranelift canonicalization and epoch off (option C + the counter)
- **native Rust**: the module crates compiled for this machine, run by `deck_module::testing::Rig` (src/native.rs)
