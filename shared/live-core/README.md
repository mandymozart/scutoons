# @scutoons/live-core – the browser deck's sound (#184, #320)

deck-engine – the native deck's engine, the one engine of the universal core – as wasm in a Web Worker, its audio (the
device's channels, interleaved) played by an AudioWorklet through a SharedArrayBuffer ring. TypeScript only moves bytes;
what plays and how is the core's. The relay codec stays `@scutoons/live`. The wasm is `deck/live-core` (Rust, no
wasm-bindgen, no imports), built for wasm32 + simd128 (`npm run build:wasm -w shared/live-core` → `scripts/wasm.ts`'s path;
its gz size is gated at 500 KB in `npm run deck:ci`).

```ts
import { LiveCore } from '@scutoons/live-core';
import workletUrl from '@scutoons/live-core/worklet?worker&url';           // vite: the worklet as its own module
const core = await LiveCore.start({                                        // from a click (the AudioContext)
	worker: new Worker(new URL('@scutoons/live-core/worker', import.meta.url), { type: 'module' }),
	wasm: wasmUrl,                                                          // live_core.wasm, served same-origin
	worklet: workletUrl
});
const table = await core.load(sceneToml, { bpm: 84 }); // the deck's TOML (deck-core BROWSER_MODULES); bpm: no clock of its own
const lead = core.param('lead.freq');               // resolve once (node + input) …
core.set(lead, new Float32Array([440, 2, 1, 0.1]), { mode: 'blend', amount: 0.5 }); // … send often (#189)
const off = core.onAnalysis(({ level, bass, mid, high, spectrum }) => draw(level, bass, mid, high, spectrum));
core.value('sway.out');                             // the heard block's value of a path (inputs, outputs, meters)
core.stats(); // transport, render µs per block (avg, peak, budget), blocks/s, sends/s, underruns, dropped, lead, gapMs
await core.stop();
```

## The exchange format (measured: deck/lab/results/formats.md)

No JSON across the boundary. A value is a **wire record** (`src/wire.ts` is GENERATED from the layout tables in `deck/live-core/src/wire.rs` (#372: `npm run generate`); the
fixtures `deck/live-core/fixtures/wire.json` are the cross-check both sides run): 12 bytes `op · mode · n · node · port · amount` + `f32 × n`, little-endian,
4-byte aligned. Node and port are indices from the load table – names are resolved once, at load.

| path | shared transport (cross-origin isolated) | transfer transport (fallback) |
|---|---|---|
| page → worker: records | SAB byte ring (`Ring`), no message | one transferred ArrayBuffer per `set` |
| worker → wasm | the ring copied straight into the wasm inbox, `lc_apply(len)` | the same, from the message |
| wasm → worklet: audio | `Float32Array` view → SAB audio ring; the worklet `Atomics.notify`s, the worker `Atomics.waitAsync`s | 512-frame chunks over a MessagePort, played counts back |
| wasm → page: analysis | view → the `Latest` slot (a sequence lock), read on animation frames | one transferred frame per ≈ 16 ms |

- Edges only: the scene in (TOML), the load table out (JSON: the device channels, per node its inputs, outputs and`n  where their heard values are in the analysis frame, the build's notes), errors – once each.`n- **Channels** (plan 2.6): `LiveCore.start` renders the destination's `maxChannelCount` (2 on a phone; `channels` overrides)`n  – the worklet's `outputChannelCount`, the audio ring and the transfer chunks carry them interleaved; the core folds or`n  places a scene's outlets on them. The control half steps inside every 128-frame quantum (no step export).
- `transport: 'auto'` (default) picks shared when `crossOriginIsolated`, else transfer – it works without the headers,
  at one postMessage per send, per analysis frame and per 512 audio frames (≈ 6 µs each vs 0.5 µs in the ring).
- `device` (`'phone'` | `'desktop'`, the page's fact) picks the core's **lead** – how far the worker renders ahead;
  the analysis looks back by it, so the bands are what is heard. The rule is deck-core's `lead` (#365, the native
  deck's too): a generous start that shrinks once stable (the warm-up lead), a step up per dropout, a step down after a
  calm minute, the class's floor and cap (`lc_lead_class`, `lc_lead`). The worklet counts dropouts into a shared status
  word (shared transport) or its feed messages (transfer); `stats()` carries them.
- **Monitor** (#364): the worker times each block, the analysis and its own pauses and hands the readings to the core
  (`lc_note_*`); once a second the core's audio window goes to the page as words (`audioTo(monitor)` – space-web's
  `DeckMonitor`, deck-core `monitor`), which counts, judges and words everything the stats bar and the sound report show.
- **Warm-up** (#356): before the first sound the worker has the core play the scene silently in a throwaway twin
  (`lc_warm`, deck/live-core `Core::warm`, 256 quanta) – a wasm engine compiles each function on its first call and
  optimises it only when hot, so cold the first load and the first blocks take several budgets on a phone. The audible
  core never sees the twin: what is heard is bit for bit the same.
- The main thread is not in the audio path: the worker is woken by the worklet (`Atomics.waitAsync`, shared) or by
  its MessagePort (transfer), never by the page; the page only writes records into the control ring and reads the
  analysis on animation frames. A busy main thread (the 3D) can only compete for CPU – the lead covers that – and the
  worker renders without garbage per block (the render view is reused), so its GC does not pause it.

## Files

| file | job |
|---|---|
| `src/wire.ts` | GENERATED (#372) – the records (`writeSend` in place, `readRecords`), the analysis frame (`analysisOf`: bands, spectrum, limiter reduction, the per-channel peaks – #282, the heard values – #320) |
| `src/signals.ts` | GENERATED (#372) – the scene's `[outputs]` as device messages (MIDI / DMX / OSC records, `readSignals`; `LiveCore.onSignal`) – #282 |
| `src/clock-state.ts` | GENERATED (#372) from clock.rs + deck-core's tempo – the clock's state block (`SLOT`, `FLAG`, `clockOf`), `SOURCES`, `CLOCK_MESSAGES` (`clock.ts` re-exports it and keeps the page's window) |
| `src/module-abi.ts` | GENERATED from deck-module's `abi.rs` – the module ABI's numbers |
| `src/ring.ts` | `Ring` (one writer, one reader, Atomics counters, SAB or plain buffer), `Latest` (sequence lock) |
| `src/engine.ts` | `WasmCore` (the exports behind views on wasm memory), `Engine` (the pump: drain, render per block, analysis) |
| `src/worker.ts` | the worker entry: init (wasm compiled there), load, the shared pump / the transfer feed |
| `src/worklet.ts` | the `live-core` AudioWorkletProcessor: plays the ring (de-interleaves the channels), silence + an underrun when empty |
| `src/index.ts` | `LiveCore`: start, load, param, set, value, onAnalysis, audioTo, stats, stop; `deviceChannels`, `valueIndex` |
| `scripts/wasm.ts` | where and how live_core.wasm is built (wasm32 + simd128, its own target folder) |

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
