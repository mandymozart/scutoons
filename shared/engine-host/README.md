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

**Output meters** – `snapshot().outputs: OutputReading[]`, one entry per audio output the engine renders: `{ level, position, zone }` – the held
peak (0..1), its place on the meter's dB scale (0..1 from the bottom) and its zone (`ok` / `hot` / `clip`). Today the master is mono and
plays on both channels of the speaker, so two equal entries; `[]` while no sound runs. All of it is the core's (#373): live-core's analysis
frame carries each output's largest sample HELD, falling ×1/e per 0.25 s (deck-core `meters`), with its position and zone; the scale
(-48…+3 dB, zone edges -12 / -3 dB, the limiter's bar full at 12 dB) is deck-core's constants, and the marks the page draws
(`METER_MARKS`, `METER_EDGES_DB`, `METER_ZONES`) are generated into live-core's wire.ts. The page draws what the frame says (`Analysis.outputs`,
`.positions`, `.zones`, `.limiterBar` – views on the frame). For a canvas drawn every animation frame, `host.peaks` is the newest frame's
held peaks as a `Float32Array` view (no allocation; its length is the output count, 0 without sound – read the property each frame, don't keep it).

**Device messages** – the scene's `[outputs]` (deck-outputs, #211: MIDI CC / note / bend, DMX fixtures, the strobe) run inside the engine
on the audio clock, and what they send is *shown*, not played (the browser has no MIDI port or DMX widget):

```ts
host.hooks.signal = (msg) => { … };      // msg: DeviceSignal – the protocol's DeviceMsg kinds, plus `at`
snapshot().signals;                      // DeviceRow[]: per protocol that carried a message { kind, count, text, activity, active }
```

| `msg.kind` | fields | notes |
|---|---|---|
| `midi` | `port` (`'out'` the deck's MIDI port · `'conductor'` over the relay), `bytes` (status first) | |
| `dmx` | `ch` (1-based first channel), `values`, `flash` | consecutive channels of one frame come as one range; `flash` = a strobe flash, sent at once |
| `osc` | `address`, `args` (`{ kind: 'int' \| 'float' \| 'string' \| 'bool', p }`) | the wire and the hook carry it; deck-outputs produces no OSC yet |

`at` is the engine clock in ms since the scene loaded (the audio rendered, not wall time). The hook runs for every message (a few a
second at most: 40 Hz DMX frames carry only what changed), inside the host's own try/catch. The host passes each message on to deck-core's
device monitor (deck/space-web `DeckMonitor.signal`, #373), which words it (`CC 74 = 127 · ch 1`, `DMX 20–21 = 255 255`, `/fx/level 0.5`),
counts it and keeps an activity that is 1 at a message and falls (×1/e per 0.4 s; the lamp is lit at ≥ 0.25) – the same words and lamp a native
HUD gets from the same crate (headless scenario 18-device-text). Limits today: the browser has no tempo clock yet, so a
strobe stays dark (its rest values go out); profile files other than the built-in `strobe` are not available (the load fails with the
reason); a mapping may read any `node.<input>` or a node's output (`node`, `node.out`) of the *batch* graph live-core plays – the
rig's lowered scene decides whether an `[outputs]` table reaches it.

The path from Rust: live-core's analysis frame (`FRAME` = bands + spectrum + reduction + output count + `MAX_OUTPUTS` held peaks, their dB positions and zones, the limiter's bar, wire.ts / wire.rs)
and the signal records (`lc_signals`, signals.ts / signals.rs, fixtures/signals.json read by both sides' tests) → the worker posts them
to the page → `LiveCore.onSignal` → `Deck.onSignal` → this host. Additive: older readers of the frame never look past the reduction.
