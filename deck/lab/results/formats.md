# The wasm boundary's exchange format (#184) – measured 2026-10-30, the choice

Ryzen 5 3600X, Windows 10. Node 22.14 and Edge 156 headless (Chromium 156, cross-origin isolated, timers 5 µs), the
suite in a Web Worker. Each number: the median of 7 runs inside a suite run, then the median of 5 suite runs (spread
in the raw files). **Noisy:** another project was encoding video on the same machine – single runs moved by up to
2×, the medians by ~10–20 %; the ratios are what holds. Raw: `formats-node.json`, `formats-chromium.json` / `.md`
(the 5-run spreads); an earlier run in playwright's Chromium 140 gave the same picture.

"Whole trip" = what a worker does per value: encode in TS → cross into wasm → decode → apply to a loaded scene
(live-core's `Core::send`, the same work in every row); for the analysis frame the way out (wasm → JS values).

## Into / out of wasm (µs, whole trip; Chromium worker · Node)

| workload | JSON | postcard | wire, copied (wasm-bindgen) | **wire, in place** |
|---|---|---|---|---|
| control: one scalar | 1.16 · 2.45 | 0.23 · 0.27 | 0.18 · 0.26 | **0.08 · 0.07** |
| parameter vector, 4 floats | 1.79 · 3.36 | 0.36 · 0.29 | 0.27 · 0.24 | **0.13 · 0.11** |
| parameter vector, 64 floats | 12.1 · 18.6 | 0.77 · 0.75 | 0.28 · 0.33 | **0.20 · 0.17** |
| analysis frame out (4 bands + 512 bins) | 38.6 · 71.6 | 3.7 · 4.0 | 1.4 · 1.5 | **0.10 · 0.11** |
| cue / cell (the 330 B relay envelope) | 5.4 · 9.1 | 7.3 · 3.0 | – | – (stays in TS: JSON.parse 1.9 · 4.7) |

- **JSON** – JSON.stringify → wasm-bindgen `&str` → serde_json (the cue as `protocol::Envelope`).
- **postcard** – the schema-driven binary: serde types, a TS encoder by its rules (what a generator would emit),
  bytes copied in by wasm-bindgen.
- **wire, copied** – live-core's fixed records (`deck/live-core/src/wire.rs` = `shared/live-core/src/wire.ts`), the
  bytes copied in by wasm-bindgen.
- **wire, in place** – the same records written straight into live_core.wasm's inbox (a `Uint8Array` view on wasm
  memory), then `lc_apply(len)`; the frame read through a `Float32Array` view and copied once.

## Page → worker (µs, Chromium)

| probe | µs |
|---|---|
| postMessage, an object { mode, node, port, amount, v }, one-way (4000 back to back) | 6.0 |
| postMessage, the 16 record bytes transferred, one-way | 5.4 |
| **SharedArrayBuffer ring, a 16-byte record, one-way** | **0.47** |
| postMessage round trip | 30–65 |
| SharedArrayBuffer ring round trip (Atomics.waitAsync both ways) | 30 |

The round trips are bound by waking the other thread (waitAsync resolves through the event loop) and the 5 µs timer;
the per-message cost is what scales with a show's sends – there the ring is 12× cheaper and sends no events.

## The choice: wire records, in place, over SharedArrayBuffer rings

1. **Hot path = live-core's fixed records** (12-byte header op / mode / n / node / port / amount + f32 × n,
   little-endian, 4-byte aligned): 14× (scalar) to 60× (64 floats) faster than JSON across the wasm boundary, 2–4×
   faster than postcard, and the analysis frame leaves wasm ~400× cheaper than as JSON. The page writes a record into
   the SAB control ring, the worker copies the ring straight into the wasm inbox – no allocation, no glue, no event.
2. **Not postcard / a generated codec from deck/protocol:** slower here (varints, a wasm-bindgen copy, TextEncoder
   per short string – 7 µs for the cue in Chromium), and the relay's own types do not fit a non-self-describing format
   (`#[serde(flatten)]` envelope, skipped optional fields, the loose `serde_json::Map` actions); it would need mirror
   types and a generator to keep two codecs in step. The boundary's messages are not the relay's anyway: names are
   resolved to indices once at load, so a record is numbers only. The fixture `deck/live-core/fixtures/wire.json`
   keeps the two codecs equal (both sides' tests).
3. **Cues and cells stay TypeScript** (DECISIONS 2026-10-30: the relay codec stays TS): JSON.parse in TS is faster
   than any form of the cue inside wasm; the worker turns their numeric effects into records.
4. **JSON only at the edges:** the scene in (TOML, once), the load table out (once), errors.
5. **Without COOP / COEP** (no SharedArrayBuffer) the same records, audio chunks and frames travel as transferred
   ArrayBuffers – it works (demo-not-isolated.png), at one message per send, per frame and per 512 audio frames.
