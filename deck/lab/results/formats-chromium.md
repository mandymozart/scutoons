Edg/156.0.0.0 headless (Chromium 156), Windows 10, Ryzen 5 3600X, cross-origin isolated: true (timers 5 µs), 2026-10-30.

| page → worker | µs (median of 5) | the 5 runs |
|---|---|---|
| postMessage, an object { mode, node, port, amount, v } (one-way, 4000 back to back) | **6.02** | 34.02 / 4.81 / 5.78 / 6.86 / 6.02 |
| postMessage, 16 record bytes transferred (one-way, 4000 back to back) | **5.43** | 5.99 / 5.43 / 5.32 / 7.17 / 5.41 |
| SharedArrayBuffer ring, a 16-byte record (one-way, 4000 back to back) | **0.47** | 0.70 / 0.40 / 0.46 / 0.47 / 0.91 |
| postMessage round trip (median of 300) | **65.00** | 70.00 / 65.00 / 65.00 / 60.00 / 60.00 |
| SharedArrayBuffer ring round trip, Atomics.waitAsync both ways (median of 300) | **30.00** | 30.00 / 30.00 / 35.00 / 20.00 / 25.00 |

| workload | format | TS encode µs | whole trip µs | spread (min – max) |
|---|---|---|---|---|
| control: one scalar (force) | JSON | 0.536 | **1.157** | 1.072 – 1.711 |
| control: one scalar (force) | postcard | 0.086 | **0.233** | 0.198 – 0.282 |
| control: one scalar (force) | wire, copied | 0.041 | **0.175** | 0.170 – 0.260 |
| control: one scalar (force) | wire, in place | 0.043 | **0.081** | 0.072 – 0.112 |
| parameter vector, 4 floats (blend) | JSON | 0.751 | **1.792** | 1.627 – 2.339 |
| parameter vector, 4 floats (blend) | postcard | 0.110 | **0.359** | 0.259 – 0.387 |
| parameter vector, 4 floats (blend) | wire, copied | 0.067 | **0.273** | 0.201 – 0.308 |
| parameter vector, 4 floats (blend) | wire, in place | 0.058 | **0.128** | 0.107 – 0.185 |
| parameter vector, 64 floats (blend) | JSON | 4.897 | **12.060** | 9.790 – 14.557 |
| parameter vector, 64 floats (blend) | postcard | 0.465 | **0.767** | 0.623 – 1.048 |
| parameter vector, 64 floats (blend) | wire, copied | 0.142 | **0.277** | 0.262 – 0.465 |
| parameter vector, 64 floats (blend) | wire, in place | 0.147 | **0.198** | 0.162 – 0.283 |
| analysis frame out (4 bands + 512 bins) | JSON | – | **38.614** | 37.412 – 48.835 |
| analysis frame out (4 bands + 512 bins) | postcard | – | **3.730** | 3.559 – 5.316 |
| analysis frame out (4 bands + 512 bins) | wire, copied | – | **1.411** | 0.852 – 2.008 |
| analysis frame out (4 bands + 512 bins) | wire, in place | – | **0.098** | 0.076 – 0.152 |
| cue / cell in (the 330 B relay envelope) | JSON | 0.850 | **5.407** | 5.310 – 6.453 |
| cue / cell in (the 330 B relay envelope) | postcard | 7.048 | **7.320** | 7.072 – 8.413 |
| cue / cell in (the 330 B relay envelope) | TS only (JSON.parse – it stays in TS) | – | **1.944** | 1.747 – 2.245 |
