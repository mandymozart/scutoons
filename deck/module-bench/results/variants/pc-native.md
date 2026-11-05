# wasmtime variants (pc-native, #396) – median of 3 rounds; mean µs (p50) per sample

| variant | modules | analog x2 | fm16 | fm16 ×1024 | walk | fm16 / analog vs deck | bits = deck | bits = Chrome |
|---|---|---|---|---|---|---|---|---|
| deck | plain | 816.0 (745.7) | 96.2 (83.7) | 815.6 (742.6) | 11.4 (12.6) | 1.00× / 1.00× | yes | yes |
| nan-off | plain | 350.1 (303.3) | 48.1 (39.8) | 371.3 (324.6) | 5.2 (5.7) | 0.50× / 0.43× | yes | yes |
| nan-off+epoch-off | plain | 330.4 (284.7) | 36.1 (30.5) | 306.6 (251.4) | 4.7 (4.5) | 0.38× / 0.40× | yes | yes |
| nan-pass | nan | 448.5 (390.8) | 57.3 (51.7) | 483.2 (447.8) | 5.7 (6.0) | 0.60× / 0.55× | yes | yes |
| native Rust (no wasm) | rlib | 285.9 (247.0) | 35.6 (28.7) | 281.5 (233.7) | – | 0.37× / 0.35× | yes | yes |
| Chrome (pc-chrome.json, web) | plain | 291.4 (250.0) | 39.1 (30.0) | 296.8 (245.0) | 6.9 (5.0) | 0.41× / 0.36× | | |
| Node (pc-node.json, web) | plain | 304.2 (259.0) | 46.7 (40.4) | 337.4 (314.5) | 6.8 (6.8) | 0.49× / 0.37× | | |

- **deck**: the deck's config (NaN canonicalization, epoch)
- **nan-off**: cranelift_nan_canonicalization off
- **nan-off+epoch-off**: both off
- **nan-pass**: NaN pass in the wasm, cranelift canonicalization off
- **native Rust**: the module crates compiled for this machine, run by `deck_module::testing::Rig` (src/native.rs)
