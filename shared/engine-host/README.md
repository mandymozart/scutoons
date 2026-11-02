# @scutoons/engine-host (#298)

The deck's browser engine as one component – `Deck` + `SpaceStage` (deck-space as wasm: the being, `rig_play`, `core_rig`)
over live-core's worker and worklet. The viewer's `/live/deck` brings its engine up through it; the creator's editor mounts it
(research/EDITOR.md: the Sound design workspace – the designer on one side, this deck on the other; later the scene editor).
The engine stays Rust: nothing of the sound or the being is done in TS here.

```ts
const host = createEngineHost({ api, cdn, wasmBase: '/live/', character: 'catpope', rigs: true }); // relay: none = alone
host.hooks = { rig, mood, tap, tick, space, threeD, spaceStatus, value };  // the app listens in (all optional)
await host.mount(element);   // the deck + the being (init() / attach(el) when the app sets itself up in between)
await host.start();          // the sound – needs a user gesture (the AudioContext); false = see snapshot().error
host.playCoreRig('character.catpope', '', 'happy');  host.playRig(bundleJson, persona, mood);  host.setMood('happy' | null);
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
viewer isolates `/live/…`; the creator only `/engine` and `/edit/…` (creator/src/lib/isolation.ts, vite.config.ts, static/.htaccess):
app-wide COEP blocks the Reader panel's viewer iframe (measured, #298). A document keeps its headers until a full load, so
`isolateThisPage()` (creator) loads the page afresh when it arrives without them and on the way out.
