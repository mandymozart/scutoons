# @scutoons/live-core – the browser Live mode's engine (#184)

deck-core (the deck's graph, FFT analysis and DSP) as wasm in a Web Worker, its audio played by an AudioWorklet through
a SharedArrayBuffer ring. For the conductor, rehearsal and browser outputs; phones stay TS + Web Audio, the relay codec
stays `@scutoons/live` (DECISIONS 2026-10-30). The wasm is `deck/live-core` (Rust, no wasm-bindgen, no imports).

```ts
import { LiveCore } from '@scutoons/live-core';
import workletUrl from '@scutoons/live-core/worklet?worker&url';           // vite: the worklet as its own module
const core = await LiveCore.start({                                        // from a click (the AudioContext)
	worker: new Worker(new URL('@scutoons/live-core/worker', import.meta.url), { type: 'module' }),
	wasm: wasmUrl,                                                          // live_core.wasm, served same-origin
	worklet: workletUrl
});
const table = await core.load(sceneToml);           // deck/stage's TOML, batch kinds: lfo smooth mul add osc fm
const lead = core.param('lead');                    // resolve once (node + first input) …
core.set(lead, new Float32Array([440, 2, 1, 0.1]), { mode: 'blend', amount: 0.5 }); // … send often (#189)
const off = core.onAnalysis(({ level, bass, mid, high, spectrum }) => draw(level, bass, mid, high, spectrum));
core.stats(); // transport, render µs per block (avg, peak, budget), blocks/s, sends/s, underruns, dropped, lead, gapMs
await core.stop();
```

## The exchange format (measured: deck/lab/results/formats.md)

No JSON across the boundary. A value is a **wire record** (`src/wire.ts` = `deck/live-core/src/wire.rs`, kept equal by
`deck/live-core/fixtures/wire.json`): 12 bytes `op · mode · n · node · port · amount` + `f32 × n`, little-endian,
4-byte aligned. Node and port are indices from the load table – names are resolved once, at load.

| path | shared transport (cross-origin isolated) | transfer transport (fallback) |
|---|---|---|
| page → worker: records | SAB byte ring (`Ring`), no message | one transferred ArrayBuffer per `set` |
| worker → wasm | the ring copied straight into the wasm inbox, `lc_apply(len)` | the same, from the message |
| wasm → worklet: audio | `Float32Array` view → SAB audio ring; the worklet `Atomics.notify`s, the worker `Atomics.waitAsync`s | 512-frame chunks over a MessagePort, played counts back |
| wasm → page: analysis | view → the `Latest` slot (a sequence lock), read on animation frames | one transferred frame per ≈ 16 ms |

- Edges only: the scene in (TOML), the load table out (JSON), errors – once each.
- `transport: 'auto'` (default) picks shared when `crossOriginIsolated`, else transfer – it works without the headers,
  at one postMessage per send, per analysis frame and per 512 audio frames (≈ 6 µs each vs 0.5 µs in the ring).
- `lead` (default 1024 frames ≈ 21 ms at 48 kHz) = how far the worker renders ahead; the analysis looks back by it,
  so the bands are what is heard. It **adapts** (#208, `src/lead.ts` – the rule of deck-stage's render thread): a
  dropout grows it 512 frames up to 6144 (128 ms), a calm minute shrinks it 512 back towards the start. The worklet
  counts dropouts into a shared status word (shared transport) or its feed messages (transfer); `stats()` carries
  `lead`, `renderPeakUs` / `budgetUs` (the longest block of the second against a block's 2.67 ms) and `gapMs` (the
  longest pause between the worker's pumps – a stalled worker shows there); `glitching()` is true for 5 s after a
  dropout (the page's warning).
- The main thread is not in the audio path: the worker is woken by the worklet (`Atomics.waitAsync`, shared) or by
  its MessagePort (transfer), never by the page; the page only writes records into the control ring and reads the
  analysis on animation frames. A busy main thread (the 3D) can only compete for CPU – the lead covers that – and the
  worker renders without garbage per block (the render view is reused), so its GC does not pause it.

## Files

| file | job |
|---|---|
| `src/wire.ts` | the records (`writeSend` in place, `readRecords`), the analysis frame (`analysisOf`: bands, spectrum, limiter reduction, the per-output peaks – #282) |
| `src/signals.ts` | the scene's `[outputs]` as device messages (MIDI / DMX / OSC records, `readSignals`; `LiveCore.onSignal`) – #282 |
| `src/ring.ts` | `Ring` (one writer, one reader, Atomics counters, SAB or plain buffer), `Latest` (sequence lock) |
| `src/engine.ts` | `WasmCore` (the exports behind views on wasm memory), `Engine` (the pump: drain, step + render per block, analysis) |
| `src/worker.ts` | the worker entry: init (wasm compiled there), load, the shared pump / the transfer feed |
| `src/worklet.ts` | the `live-core` AudioWorkletProcessor: plays the ring, silence + an underrun when empty |
| `src/lead.ts` | `AdaptiveLead` (#208): grow on a dropout, shrink after a calm minute |
| `src/index.ts` | `LiveCore`: start, load, param, set, onAnalysis, stats, stop |

## Production headers (set: the viewer's `/live/…` in static/.htaccess, the creator's `/engine` and `/edit/…` in its static/.htaccess)

The shared transport needs the page cross-origin isolated:
- the Live page documents: `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`
  (or `credentialless` – Chromium / Firefox only; the conductor runs there);
- the worker script and the wasm (`/_app/immutable/…`, same origin): the same COEP header (a dedicated worker's own
  response decides its isolation);
- every cross-origin subresource those pages load under `require-corp`: `Cross-Origin-Resource-Policy: cross-origin`
  on cdn.scutoons.com (a bunny edge rule) or a CORS fetch; the API is CORS already;
- only on the Live paths: COOP `same-origin` cuts `window.opener` (popup sign-ins) and COEP blocks iframes without
  COEP – the creator's viewer iframe would need it too. The viewer serves headers from `viewer/static/.htaccess`
  (mod_headers, `<If "%{REQUEST_URI} =~ …">`).

## Checks

`npm run check -w shared/live-core` (tsc + node tests: wire fixtures, the rings incl. two threads over a SAB, the
engine with the real wasm – skipped, saying so, until `npm run build:wasm -w shared/live-core` built it). The Rust side:
`cargo test -p live-core` in `deck/`. The demo: `deck/lab/www/demo.html` (deck/lab/README.md).
