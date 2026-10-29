| path | workload | µs |
|---|---|---|
| native | control step (300 nodes, deep) | 3.885 |
| native · batch | control step (300 nodes, deep) | 3.744 |
| native | audio block (16 FM voices, 128 frames) | 34.467 |
| native · batch | audio block (16 FM voices, 128 frames) | 8.899 |
| native | control step (300 nodes, wide) | 3.615 |
| native · batch | control step (300 nodes, wide) | 2.025 |
| native | FFT bands (1024) | 9.214 |
| native | codec decode + encode (cue, 330 B) | 5.651 |
| native | graph build from TOML (348 nodes) | 730.200 |
| dylib | control step (300 nodes, deep) | 3.703 |
| dylib | audio block (16 FM voices, 128 frames) | 35.342 |
| dylib | FFT bands (1024) | 9.393 |
| wasmtime | module compile (Cranelift, cold) | 606402.300 |
| wasmtime | instantiate | 71.700 |
| wasmtime | control step (300 nodes, deep) | 9.202 |
| wasmtime (loop inside) | control step (300 nodes, deep) | 9.238 |
| wasmtime | audio block (16 FM voices, 128 frames) | 35.770 |
| wasmtime | FFT bands (1024) | 11.618 |
