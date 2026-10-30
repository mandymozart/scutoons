Chrome 152.0.7977.167 (the Claude desktop app's Browser pane, a background tab), Windows 10, Ryzen 5 3600X, cross-origin isolated, 2026-10-30.

| path | workload | µs |
|---|---|---|
| postMessage (Chromium) | round trip, 128 floats copied | 40.000 |
| postMessage (Chromium) | round trip, 128 floats transferred | 40.000 |
| wasm (Chromium worker) | fetch + compile + instantiate | 5435.000 |
| TypeScript (Chromium worker) | control step (300 nodes, deep) | 21.311 |
| TypeScript (Chromium worker) · batch | control step (300 nodes, deep) | 14.479 |
| TypeScript (Chromium worker) | audio block (16 FM voices, 128 frames) | 123.857 |
| TypeScript (Chromium worker) · batch | audio block (16 FM voices, 128 frames) | 110.971 |
| TypeScript (Chromium worker) | control step (300 nodes, wide) | 15.604 |
| TypeScript (Chromium worker) · batch | control step (300 nodes, wide) | 5.106 |
| TypeScript (Chromium worker) | FFT bands (1024) | 38.947 |
| TypeScript (Chromium worker) | codec decode + encode (cue, 330 B) | 3.417 |
| wasm (Chromium worker) | control step (300 nodes, deep) | 8.355 |
| wasm (Chromium worker) · batch | control step (300 nodes, deep) | 5.924 |
| wasm (Chromium worker) | audio block (16 FM voices, 128 frames) | 49.804 |
| wasm (Chromium worker) · batch | audio block (16 FM voices, 128 frames) | 64.033 |
| wasm (Chromium worker) | control step (300 nodes, wide) | 9.026 |
| wasm (Chromium worker) · batch | control step (300 nodes, wide) | 3.800 |
| wasm (Chromium worker) | FFT bands (1024) | 10.590 |
| wasm (Chromium worker) | codec decode + encode (cue, 330 B) | 6.851 |
| wasm (Chromium worker) (loop inside) | codec decode + encode (cue, 330 B) | 4.747 |
| wasm simd128 (Chromium worker) | control step (300 nodes, deep) | 6.729 |
| wasm simd128 (Chromium worker) · batch | control step (300 nodes, deep) | 5.188 |
| wasm simd128 (Chromium worker) | audio block (16 FM voices, 128 frames) | 43.576 |
| wasm simd128 (Chromium worker) · batch | audio block (16 FM voices, 128 frames) | 29.441 |
| wasm simd128 (Chromium worker) | control step (300 nodes, wide) | 7.266 |
| wasm simd128 (Chromium worker) · batch | control step (300 nodes, wide) | 2.757 |
| wasm simd128 (Chromium worker) | FFT bands (1024) | 12.643 |
| wasm simd128 (Chromium worker) | codec decode + encode (cue, 330 B) | 6.573 |
| wasm simd128 (Chromium worker) (loop inside) | codec decode + encode (cue, 330 B) | 5.305 |
