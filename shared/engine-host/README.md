# @scutoons/engine-host (#298)

The deck's browser engine as one component – `Deck` + `SpaceStage` (deck-space as wasm: the being, and `DeckState` – the mood, what plays, the rig's lifecycle)
over live-core's worker and worklet. The viewer's `/live/deck` brings its engine up through it; the creator's editor mounts it
(research/EDITOR.md: the Sound design workspace – the designer on one side, this deck on the other; later the scene editor).
The engine stays Rust: nothing of the sound or the being is done in TS here.

```ts
const host = createEngineHost({ api, cdn, wasmBase: '/live/', character: 'catpope', rigs: true }); // relay: none = alone
host.hooks = { rig, mood, tap, tick, space, threeD, spaceStatus, value };  // the app listens in (all optional)
await host.mount(element);   // the deck + the being (init() / attach(el) when the app sets itself up in between)
await host.start();          // the sound – needs a user gesture (the AudioContext); false = see snapshot().error
host.playCoreRig('character.catpope');  host.playRig(bundleJson, persona);  host.setMood('happy' | null);  // the rig plays at the deck's mood
host.snapshot();  host.subscribe(fn);  host.log.subscribe(fn);   // phase, transport, levels, fps, cpu, dropouts · log lines
await host.stop();  host.dispose();
```

`<EngineStage {options} bind:host onmounted>` draws the being's canvas (fills its parent) and hands the host out; Start / Stop and
the readouts are the page's. `gate` (opt-in, default off – the viewer keeps its own Start button): a round Play button centred over the canvas
and the first user gesture on the element (pointerdown / touchend / keydown, observed in the capture phase, never swallowed) start the sound –
the browser blocks the first sound without a gesture; gone once it runs (gate.ts: idle → starting → running | blocked). No app state, routing or paraglide in here. The wasm (`space_web.js`, `space_web_bg.wasm`,
`live_core.wasm`) is served by the app from `wasmBase` (`npm run build:wasm -w shared/live-deck` writes viewer/ and creator/ `static/live/`).

Pure parts (phases, status line, transport note, log buffer, options): `npm run check -w @scutoons/engine-host`.

**Transport:** with the page cross-origin isolated (COOP `same-origin` + COEP `credentialless`) live-core runs on SharedArrayBuffer
rings; without it, on postMessage (`transfer`) – it works, slower; `snapshot().transport` / `transportNote()` say which. The
viewer isolates `/live/…`; the creator only `/edit/…` (creator/src/lib/isolation.ts, vite.config.ts, static/.htaccess):
app-wide COEP blocks the Reader panel's viewer iframe (measured, #298). A document keeps its headers until a full load, so
`isolateThisPage()` (creator) loads the page afresh when it arrives without them and on the way out.

## Signals: output peaks and device messages (#282)

What the Sound design workspace's signal strip reads. Both come from the Rust engine (deck/live-core); nothing of the
sound or the scene's outputs is computed in TS.

**Output peaks** – `snapshot().outputs: number[]`, one entry per audio output the engine renders, 0..1. Today the master is mono and
plays on both channels of the speaker, so `[l, r]` with two equal values; `[]` while no sound runs. live-core puts each output's largest sample
since the last analysis frame (≈ 60 a second, after the limiter) into the frame; `OutputMeters` (meters.ts) holds a peak and lets it fall
(×1/e per 0.25 s) so the 250 ms snapshot sample does not miss the transients. A multi-channel engine only raises the count (≤ 8
slots, `MAX_OUTPUTS`); the snapshot's array grows with it. The raw per-frame peaks are `Analysis.outputs` (`LiveCore.onAnalysis`).
For a canvas drawn every animation frame, `host.peaks` is the same readings as **one `Float32Array` the host updates in place with every analysis
frame** (no allocation per frame; its length is the output count, 0 without sound, and the object changes only when the count does – read the
property each frame, don't keep it): `const p = host.peaks; for (let i = 0; i < p.length; i++) draw(p[i])`, no snapshot / subscribe in between.
`host.monitor` is allocation-free per message too (`count(kind)`, `last(kind)`, `activity(kind, now)`; the wording `text(kind)` is made once per message, on request).

**Device messages** – the scene's `[outputs]` (deck-outputs, #211: MIDI CC / note / bend, DMX fixtures, the strobe) run inside the engine
on the audio clock, and what they send is *shown*, not played (the browser has no MIDI port or DMX widget):

```ts
host.hooks.signal = (msg) => { … };      // msg: DeviceSignal – the protocol's DeviceMsg kinds, plus `at`
host.monitor.view(performance.now());    // also snapshot().signals: per kind { count, activity, last, text, at }
```

| `msg.kind` | fields | notes |
|---|---|---|
| `midi` | `port` (`'out'` the deck's MIDI port · `'conductor'` over the relay), `bytes` (status first) | |
| `dmx` | `ch` (1-based first channel), `values`, `flash` | consecutive channels of one frame come as one range; `flash` = a strobe flash, sent at once |
| `osc` | `address`, `args` (`{ kind: 'int' \| 'float' \| 'string' \| 'bool', p }`) | the wire and the hook carry it; deck-outputs produces no OSC yet |

`at` is the engine clock in ms since the scene loaded (the audio rendered, not wall time). The hook runs for every message (a few a
second at most: 40 Hz DMX frames carry only what changed), inside the host's own try/catch. `SignalMonitor` (monitor.ts, pure) keeps
the last message per kind, a count and an activity that is 1 at a message and falls (×1/e per 0.4 s); `describeSignal(msg)` words
one message (`CC 74 = 127 · ch 1`, `DMX 20–21 = 255 255`, `/fx/level 0.5`). Limits today: the browser has no tempo clock yet, so a
strobe stays dark (its rest values go out); profile files other than the built-in `strobe` are not available (the load fails with the
reason); a mapping may read any `node.<input>` or a node's output (`node`, `node.out`) of the *batch* graph live-core plays – the
rig's lowered scene decides whether an `[outputs]` table reaches it.

The path from Rust: live-core's analysis frame (`FRAME` = bands + spectrum + reduction + output count + `MAX_OUTPUTS` peaks, wire.ts / wire.rs)
and the signal records (`lc_signals`, signals.ts / signals.rs, fixtures/signals.json read by both sides' tests) → the worker posts them
to the page → `LiveCore.onSignal` → `Deck.onSignal` → this host. Additive: older readers of the frame never look past the reduction.
