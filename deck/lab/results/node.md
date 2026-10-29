| path | workload | µs |
|---|---|---|
| TypeScript (Node) | control step (300 nodes, deep) | 13.819 |
| TypeScript (Node) · batch | control step (300 nodes, deep) | 19.466 |
| TypeScript (Node) | audio block (16 FM voices, 128 frames) | 125.022 |
| TypeScript (Node) · batch | audio block (16 FM voices, 128 frames) | 114.651 |
| TypeScript (Node) | control step (300 nodes, wide) | 13.207 |
| TypeScript (Node) · batch | control step (300 nodes, wide) | 5.293 |
| TypeScript (Node) | FFT bands (1024) | 41.003 |
| TypeScript (Node) | codec decode + encode (cue, 330 B) | 7.074 |
| wasm (Node) | control step (300 nodes, deep) | 7.895 |
| wasm (Node) · batch | control step (300 nodes, deep) | 5.233 |
| wasm (Node) | audio block (16 FM voices, 128 frames) | 40.777 |
| wasm (Node) · batch | audio block (16 FM voices, 128 frames) | 55.419 |
| wasm (Node) | control step (300 nodes, wide) | 8.057 |
| wasm (Node) · batch | control step (300 nodes, wide) | 3.389 |
| wasm (Node) | FFT bands (1024) | 14.447 |
| wasm (Node) | codec decode + encode (cue, 330 B) | 7.828 |
| wasm (Node) (loop inside) | codec decode + encode (cue, 330 B) | 6.670 |
| wasm simd128 (Node) | control step (300 nodes, deep) | 7.922 |
| wasm simd128 (Node) · batch | control step (300 nodes, deep) | 6.745 |
| wasm simd128 (Node) | audio block (16 FM voices, 128 frames) | 40.716 |
| wasm simd128 (Node) · batch | audio block (16 FM voices, 128 frames) | 30.959 |
| wasm simd128 (Node) | control step (300 nodes, wide) | 8.576 |
| wasm simd128 (Node) · batch | control step (300 nodes, wide) | 2.733 |
| wasm simd128 (Node) | FFT bands (1024) | 12.112 |
| wasm simd128 (Node) | codec decode + encode (cue, 330 B) | 8.018 |
| wasm simd128 (Node) (loop inside) | codec decode + encode (cue, 330 B) | 6.439 |
