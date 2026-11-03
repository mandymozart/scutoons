Status 2026-11-03: the bridge audit, first pass (#357, read-only; a worker's report, every claim checked in the code at merge 3436e9f5 / 1fc7732f). The moves are tickets #363–#378 (Part of #311), plus the existing #360 #359 #358 #362 #343. Rerun: §5.

# The browser bridge and its twins – what is logic, where it lives, what native would have to redo

**Scope read:** shared/live-core, live-deck, engine-host, live (generated – types only); viewer `lib/components/live-deck`,
`lib/live-deck*.ts`, `lib/live-vote*.ts`, `routes/live/*`; creator `lib/components/sound`, `lib/editor/signal-strip.ts`; deck/live-core,
space-web, bridge, adapters, native (headers of every file, the HUD / stats / lead / glitch / pad / sync / quality code in full). About
9,100 lines of bridge TS / Svelte (src, without tests and generated files).

**In one paragraph:** the hot path is clean – sound, the tempo clock (P8), the render-quality governor, the rig player, the controls'
selection / sticks / MIDI learn are Rust (deck-engine, deck-core tempo, deck-space via space-web). Around it the browser carries a second,
smaller deck in TS: the cathedral scene and its parameter table (#360), the mood ladder, the relay client (twice), the adaptive lead,
the rig's derived facts and lifecycle, the controls' value maths, the gamepad rules, the scene-sync state machine, and **its own monitoring**
– every number on the stats bar is computed and judged in TS, by different rules than the native HUD's. The native side has the mirror-image
problem: the HUD lines, thresholds, the F4 settings page and the F5 switcher are logic in the *platform* crate (main.rs 2066 lines), so the
wasm build cannot show them and the browser rebuilt them in Svelte. That pair is where "one UI authority" (#362) fails today.

## 1. The table

Verdicts: **P** periphery (only this platform can do it – keep) · **W** logic in the wrong place · **D** duplicated (a twin exists in Rust) ·
**U** UI authority (a second copy of state or a number the core has) · cost to move: S / M / L.

| Feature | Where it lives (lines) | Verdict | Native today | Cost | Ticket |
|---|---|---|---|---|---|
| Cathedral scene, PARAMS / MELODY ranges + labels, OUTPUTS / STREAMED, `deckGraph` | live-deck scene.ts (162) | D W | cathedral.toml + `carried_part` in deck-native modules.rs | M | #360 |
| Mood energy ladder `Moods`, `LADDER`, `ladderFor`, the tick's dt clamp | scene.ts, deck.ts `tick` | D | deck-space being.rs `step_head` | M | #367 |
| Relay deck client: join, keyframe, backoff 1 2 4 8 10 s, ping 20 s, status texts, `ValueStream` / epsilon / RATE | live-deck client.ts (262) | D | space/link.rs (446), live/client.rs (184, + resume) | L | #366 |
| Vote clients (audience / conductor) – the same join / backoff / ping a 4th time | viewer live-vote-link.ts (172) | D | relay vote.rs tallies (Rust) | in L | #366 |
| Adaptive lead (rule + bounds 2048/3072 → 6144, step 512; phone guess) | live-core lead.ts (47), worker.ts, deck.ts `defaultLead` | D | audio/lead.rs: 256 → 4096, step 256 – **different numbers** | S | #365 |
| Ring fill / pump (render until `lead` queued) | live-core engine.ts `fill` | P (scheduling per platform) | audio/ring.rs render thread | – | – |
| Block timing histogram p50 / p99 / max, analysis peak | live-core timing.ts, engine.ts `stats` | W (aggregation) / P (the stopwatch) | glitch.rs `note_load` (peak only) | in M | #364 |
| Underruns per minute, dropout alert 5 s | live-core index.ts `perMinute`, `dropoutAlert` | D | hud.rs `PerMinute` over **10 min**, `ALERT_S` 5 s | in M | #364 |
| Frame stats FPS / CPU per frame | live-deck controls.ts `FrameStats`, engine-host controller | D W (+ a bug, §2) | main.rs `Stats` | in M | #364 |
| Stats bar thresholds and wording, the copy report, `statusLine`, `loadPercent`, `leadMs`, `meterPercent`, the quality string ×3 | deck-stats.svelte, engine-host status.ts / report.ts, +page.svelte, creator signal-strip.ts `stripStatus` | U D | main.rs `bar_items` / `hud_lines` | M | #364, #374 |
| Rig facts: streamed paths (`/^mix\d+$/`), knob target, `sameShape` reload decision, `rigDiff`, `rigParams` | live-deck rig.ts (81), deck.ts `useRig` / `moodRig` | W | deck-space `rig::Played` (partly) ; #322 | M | #368 |
| Rig lifecycle: fallback to the fixed patch, replay on mood, pinned moods, pick order | engine-host controller.ts (436) | W U | main.rs (scene switch path) | in M | #368 |
| Engine phases idle → running, start gate | engine-host status.ts, gate.ts | P (AudioContext needs a gesture) | – | – | – |
| Knob turn (drag → value, fine ×0.1), value ↔ position (log), `Hands.norm/set` clamps, keyboard steps | controls.ts, scene.ts `toValue/toPos`, deck-knob.svelte, creator signal-strip `knobPosition` (3rd copy) | D W | deck-space params / 3D knobs | M | #369 |
| Gamepad press 0.3 / release 0.15, A edge cycles the selection | live-deck gamepad.ts (89) | D | native pad.rs ("the same rules") | in M | #369 |
| Selection line, which knobs the panel shows (`'voice.ratio' in params`) | viewer live-deck.ts `sticksLine` / `panelKnobs` | W | – (no native panel) | in M | #369 |
| Selection, sticks, MIDI learn, mappings | deck-space `DeckControls` via space-web | ✔ core | the same | – | – |
| Mappings persisted per character | controls.ts `localStore` | P (storage) | – (preset #191 later) | – | #300 |
| Clock: source picker ×3 copies (page `SOURCE` map, `Deck.choice`, core), MIDI-out flag ×3, strobe beat edge + 60 ms | +page.svelte, deck.ts, deck-clock.svelte | U W | HUD beat lights, F3 source | S | #370 |
| `beatAt` – the core's line extrapolated between 25 ms clock frames | live-core clock.ts | P with an escape (the wasm is in the worker) | tempo map in deck-core | S | #370 |
| Clock rules, status line, MIDI clock in / out | deck-core tempo via live-core | ✔ core (P8) | the same | – | – |
| Tempo line wording (`BPM x BAR y BEAT z` vs `x BPM <line>`) | main.rs `tempo_line` vs deck-clock.svelte | D U | main.rs | in S | #370 |
| Conductor scene sync: items, progress, states, `pushLine` | live-deck sync.ts (121) | D | sync.rs (817, mostly threads / cache) | M | #371 |
| Wire codecs: records / analysis frame, device signals, clock state block, clock-message filter | live-core wire.ts (158), signals.ts (86), clock.ts flags, deck.ts `CLOCK_MESSAGES` | D (fixture-tested hand twins) | live-core wire.rs / signals.rs / clock.rs, tempo/midi.rs | S | #372 |
| Device-message text (MIDI status parsing), activity decay, held peaks, dB scale and zones | engine-host monitor.ts (117), meters.ts (48), creator signal-strip.ts (161) | W D | outputs.rs `summary` only, no meters | S | #373 |
| Pinch-to-zoom, two-finger state machine, the camera's 0.88 step | live-deck space.ts `bind` | W D | winit mapping in main.rs, `Space::touch` | S | #376 |
| Image decode, set / portrait fetch, canvas, resize, DPR | space.ts | P | the native loaders | – | – |
| Quality presets and governor | deck-space quality.rs via space-web | ✔ core | the same | – | – |
| Quality option tables (fps caps 24…120, ratios, steps), level labels | deck-quality-sheet.svelte, deck-quality.svelte | D | native quality.rs `Page` (F4) | S | #375 |
| Story → event adapter (name rule, MAX_PROPS 6, MAX_DATA 1024) – **unused** outside tests | live-deck events.ts (76) | D | deck-events adapters/story.rs | S | #377 |
| TS module host (ABI v1 Io marshalling, log limiter) – **unused** outside tests | live-core modules.ts (346) | D | deck-module-host crate | – | #343 (deletes it) |
| Web MIDI access, Gamepad poll, permissions, WebSocket, fetch, AudioWorklet, SharedArrayBuffer rings | midi.ts, worklet.ts, ring.ts, worker.ts | P | the native adapters | – | – |
| URL options `?relay ?rig ?gl ?character`, default code CATH, default character | viewer live-deck.ts | P (+ defaults that belong to deck config) | deck.toml / live/config.rs | – | #330, #300 |
| HUD lines, stats bar, F1 mode, F4 settings page, F5 switcher, volume overlay | **native** main.rs, hud.rs, quality.rs, switcher.rs | W (platform crate holds UI model; wasm can't draw it) | – | L | #374 (child of #362) |
| Live outlet plan, composer, crossfade, dialog text layout | **native** live/plan.rs, compose.rs, frame.rs, text.rs (≈1,200) | W (pure, but in the platform crate) | native only | M | #378 |
| Device bridge, adapters (DMX, MIDI, OSC) | deck/bridge, deck/adapters | P (adapters behind one trait; the queue / timing is the bridge's own process) | – | – | – |
| space-web | deck/space-web (733) | P (wasm-bindgen glue, delegates to deck-space) | – | – | – |

**Native layer, the question "does wasm need it too?":** yes for the HUD / page / switcher model (#374, #375), the pad rules (#369), the
lead rule (#365), the sync state machine (#371), `carried_part` (#360) and the Live outlet's pure parts (#378). No for volume.rs (wpctl),
stop.rs, watch.rs, splash.rs, pairing.rs's daemon poll, the cache / loaders, the device backends and audio threads – genuine periphery.

### 1b. UI authority (#362): every control and readout on `/live/deck` and the creator's deck half

| Control / readout | Who owns the state | Who computes the number / text | Verdict |
|---|---|---|---|
| Start / Play | page `started` / `starting` + host phase (two copies) | – | U (one: the host phase) |
| Knobs (panel), 3D knobs, sticks, MIDI | `Deck.value` (TS) → pushed to the core (base record) and to the 3D every frame; page mirror `values` | `read()` = the core's heard value, else `Deck.value` | U – three copies of a parameter value; the core's base should be the one |
| Knob ranges / labels | scene.ts PARAMS or rig knobs (core) | TS `toPos/toValue` | D (#360, #369) |
| Selection line | DeckControls (core) | viewer `sticksLine` | W (#369) |
| Sticks on / off | page + localStorage | – | P now, a stage setting later (#300) |
| MIDI learn, mapped label | DeckControls (core) | core | ✔ |
| Copy report | – | engine-host report.ts | U (#364) |
| Quality row, level / Auto | deck-space governor (core) | `${level} ${scale}×` formatted in 3 places | ✔ state, U text (#364) |
| Quality sheet | deck-space presets (core); stored per device in localStorage (P) | option lists in Svelte | D (#375) |
| Clock picker, MIDI OUT, click | page + `Deck` + core (3 copies each, the click only in the core) | core line + page BPM text | U (#370) |
| Strobe | page only | page (beat edge, 60 ms) | W – a browser-only debug feature (#370) |
| Stats bar FPS / Q / CPU / AUDIO / RELAY / GPU | engine-host + page intervals | TS (FrameStats, BlockTimes, thresholds), relay text from client.ts | U D (#364, #366) |
| Dropout alert | core count → TS window | TS (`dropoutAlert`) | D (#364) |
| Status line (sync · 3D · hud) | page | `pushLine` (TS) + core `hud_line` | D (#371) |
| Character chip, mood, face | `Deck.character`, TS `Moods` | page picks the portrait fallback | U (#367) |
| Creator: mood chip, perf line, signal strip (bus meters, limiter, knob columns, lamps) | engine-host snapshot | creator signal-strip.ts (dB, zones, positions), meters.ts, monitor.ts | W D (#364, #373, #369) |
| Conductor / vote pages | relay (Rust) | `percent` display only | P / UI |

## 2. Monitoring parity

| Number | Browser (viewer bar / creator strip) – from | Native HUD (F1 full / bar) – from | Gap |
|---|---|---|---|
| FPS | `FrameStats` (controls.ts) per **animation frame**, also the frames the fps cap skips | `Stats.window` per **drawn** frame (main.rs) | **Bug:** with a preset cap of 30 on a 60 Hz screen the browser shows 60 (space.ts `frame` returns before drawing; controller.ts still pushes) |
| Frame time | – (not shown) | mean of the last 60 intervals, + a 20 px frame-time graph | browser has none |
| CPU per frame | JS wall time of tick + 3D frame, avg / peak per second | frame-thread work, mean of 60 | different definitions |
| Thresholds | fps <30 bad, <50 warn; peak >16 bad, >10 warn | fps <50 bad (after 120 frames); cpu >14 bad | two rule sets |
| Audio load | block render p50 / p99 / max % of budget (timing.ts histogram, worker) | DSP = peak callback load per second (glitch.rs) | different measures |
| Glitches | worklet underruns, per **1 min** | late / long / underrun, per **10 min**, with cause | different windows and causes |
| Lead | adaptive, shown `+N MS` | `LEAD` / `OUT N MS` | different bounds (§1) |
| Limiter | only in the creator strip (bar full at 12 dB) | `LIM` in bar and lines | viewer lacks it |
| Quality | deck-space governor (core) – parity | same | text formatted separately |
| Clock | core `Controller::status` – parity (P8) | same, + own `tempo_line` | wording of BPM / bar / beat differs |
| Warm-up, first-seconds table, sound report | browser only (#356) | – | fine (no JIT natively), but no native report |
| Events in / dropped | – | `EVENTS IN x DROP y` | browser lacks it |
| Debug overlay | Svelte bar + fold-out panel; core `hud_line` in the status line | F1 cycles full → bar → off (CPU raster) | two UIs; neither shared |

All of it is computed outside the core on both sides. #364 makes one `stats` report in deck-core (samples in, numbers + judgement + text
out); #374 moves the native HUD's model out of main.rs so the wasm build can draw the same thing (then #362 removes the Svelte copy).

## 3. The check:core-only baseline, entry by entry (59 hits, 15 files)

| File · pattern (count) | What it is | Class | Ticket |
|---|---|---|---|
| live-core signals.ts · scene-literal 14 | `kind: 'midi'/'dmx'/'osc'/'int'…` – type declarations and the wire decoder | false positive (wire kinds, not catalog kinds); the codec itself is a hand twin | #372, #363 |
| live-deck scene.ts · scene-literal 9 | the cathedral's node list | **logic** | #360 |
| live-deck scene.ts · clamp 1 | `Moods.update` tempo weight | **logic** | #367 |
| live-deck client.ts · scene-literal 3 | protocol envelopes `hello / ping / tempo` | false positive | #363 |
| live-deck client.ts · kind-branch 5 | dispatch on envelope kinds | pattern false positive, but **logic** (the client's state machine) | #366 |
| engine-host monitor.ts · kind-branch 3 | `s.kind === 'osc' / 'dmx'` (DeviceSignal) | false positive | #363 |
| engine-host monitor.ts · midi-bytes 3 | `status & 0xf0` switch for the text | **logic** | #373 |
| live-deck controls.ts · clamp 4 | `knobTurn`, `Hands.norm`, `Hands.set` (3) · `arcPoint` (1) | **logic** 3 · periphery (drawing) 1 | #369 |
| live-core worker.ts · lead-rule 3, lead.ts · lead-rule 1 | `AdaptiveLead` | **logic** | #365 |
| creator sound-workspace.svelte · kind-branch 3 | `target.kind === 'module'` (an edit target, not a catalog kind) | false positive | #363 |
| live-core clock.ts · tempo-math 2 | `beatAt` extrapolation (one expression counted twice) | periphery, needs `core-only-ok: #370` | #370 |
| live-core wire.ts · clamp 2 | slice bounds of a decoded frame | false positive | #372 |
| viewer deck-knob.svelte · clamp 2 | arc position (drawing) 1 · keyboard step 1 | periphery 1 · **logic** 1 | #369 |
| live-deck deck.ts · clamp 1 | tick dt clamp feeding `Moods` | **logic** | #367 |
| live-deck rig.ts · kind-branch 1 | last `mixer` named `mix\d+` | **logic** | #368 |
| engine-host meters.ts · clamp 1 | `clamp01` of a peak | periphery (sanitising); moves with the ballistics | #373 |
| engine-host status.ts · clamp 1 | `meterPercent` | false positive (display) | #364 |
| core-only-ok deck.ts:18 | clock-message filter | periphery, but a twin of tempo/midi.rs → generated | #372 |
| core-only-ok space.ts:219 | image magic bytes | periphery | – |

**Totals:** 28 hits are real logic, 26 are false positives (the patterns match protocol / wire / edit-target kinds and display maths),
5 are periphery that needs a documented escape. The baseline's tickets should be re-pointed from #357 to the move tickets above when #363
rebuilds it. **Not in the baseline at all** (outside the scanned roots or no pattern matches): `Moods`' thresholds, `ValueStream`, the
relay backoff, `FrameStats` / thresholds, `sameShape`, `PadSticks`, `pushLine`, the pinch, the creator's signal-strip.ts, viewer
`lib/live-deck*.ts` and `routes/live/*`. The grep caught the shapes; most of the twins are shapeless.

## 4. Ranked moves

Rank = drift already seen × what a native or third device would have to redo ÷ cost. Existing tickets keep their place in the order.

1. **#360** – FIXED_SCENE and the rest of scene.ts' constants (PARAMS / MELODY / OUTPUTS / `deckGraph`) – already drifted. M.
2. **#363** – guard v2 (the leverage move; it keeps 3–16 from coming back while they wait). M.
3. **#364** – one monitor in the core (the owner's own observation; includes the FPS bug). M.
4. **#365** – the lead rule once (constants already differ). S.
5. **#366** – one sans-IO relay client (four copies of one state machine). L.
6. **#367** – the mood ladder into deck-space. M.
7. **#368** – rig facts and lifecycle from deck-space. M.
8. **#369** – controls rules (gamepad, knob turn, value ↔ position). M.
9. **#370** – clock choices / flash as core state (child of #362). S.
10. **#371** – scene sync state machine. M.
11. **#372** – generated wire codecs. S.
12. **#373** – device-message text and meter ballistics. S.
13. **#374** – deck UI model crate out of main.rs (child of #362; after #364). L.
14. **#375** – quality option tables in deck-space (child of #362). S.
15. **#376** – stage gestures in deck-space. S.
16. **#377** – story adapter rules (unused – delete or wire). S.
17. **#378** – Live outlet pure parts to a platform-free crate. M.

Also in the same picture: #343 deletes the unused TS module host; #359 (transport) and #358 (clock domains) must land in deck-core, not in
`Deck` / the page – their briefs should carry #363's check.

## 5. Strengthening the guard, and the rerun

**What an agent can still do today without failing `check:core-only`** (each tested against `scan()` – 19 of 23 pass unseen):
a TOML scene in a template literal (`kind = "fm"`) – exactly how `FIXED_SCENE` is emitted; `kind : 'fm'`; `'kind': 'fm'`; a JSON scene
string; a `node('fm', …)` helper; `clamp(x, 0.5, 4)`; a ternary clamp; `Math.max(Math.min(x, 1), 0)`; min / max with variable bounds;
`(status & 0xf0) === 144`; `crypto.getRandomValues`; an LCG; a status text joined from parts or in lower case; `switch (n.kind)`;
`[...].includes(n.kind)`; `{mixer: a}[n.kind]`; mood thresholds; a renamed lead class. Caught: bpm under another name with `/ 60000`,
`60 / t.bpm`, `60000 / bpm`, a named MIDI constant. Plus the structural gap: the check scans a fixed path list, so a file in
`viewer/src/lib/` or `creator/src/lib/editor/` is never read (signal-strip.ts is that case).

**The proposal (#363):**
- **Scope by import graph**, not by path: any file that imports live-core / live-deck / engine-host / live or the space wasm is a bridge file.
- **Catalog-aware literals:** match against deck/catalog kinds.json, not `kind:` in general – no catalog kind name in an object, string,
  JSON or TOML literal; no branch / switch / includes / map lookup on catalog kinds. Removes the 26 false positives and catches the template.
- **AST, not regex** (typescript is in the repo): clamps in any form, tempo / beat arithmetic by identifier, randomness incl. LCG constants,
  MIDI status as hex, decimal or named const, `{min, max, value}` tables.
- **Size budgets** per bridge file (lines, functions, `Math.` calls) in the baseline: growth > 5 % needs `Owner-approved-baseline:`; a new
  file in a bridge root needs a line in `scripts/bridge-files.json` ("adapter: …").
- **Twin detector:** (a) numeric-constant clones – the same array or distinctive literal in a bridge file and a deck Rust file
  ([1, 2, 4, 8, 10], [24, 30, 40, 48, 60, 120], 0.88, 20_000 …); (b) the "trimmed copy" phrases – a TS comment that names a Rust twin
  ("twin of", "as the native", "same rule", ".rs", "trimmed", "kept in TS because", "for now in the page") needs a ticket or fails
  (46 such comments today; they are an honest index of the twins).
- **Data files vs literals:** a scene / params / patch JSON or TOML under a bridge root or `static/` must be generated or a byte copy of its
  deck/ source (hash check); the browser's default scene comes from the same file the X1 boots with (#360).
- **Conformance in the other direction:** native ≡ wasm golden tests for each moved piece (graph hash #360, stats report #364, relay client
  script #366, lead script #365) – a twin that reappears cannot pass both.
- **Reports:** the rerun greps worker reports and commit messages for the phrases above.

**Cadence (owner 2026-11-03):** `npm run audit:bridge` (#363: check + report + deltas against the last `bridge-audit-N` tag) at the end of
every night shift and after every 5 merged deck / shared / viewer tickets; a full hand pass like this one after each leg (rig editor
#282 #283 #314, ABI v2 #339–#346, media, the live epic's UI) – a hand pass reads what the grep cannot: shapeless twins, state copies,
native platform-crate logic. CI runs the cheap check on every PR.

## 6. Not judged here

- Whether the native HUD thresholds or the browser's are the right ones (the owner's call in #364), and the phone lead value (#356's).
- Behaviour on hardware (S22, X1): nothing was run; `deck:ci` on local main fails at "live-deck tests" after the P8 merge (the
  orchestrator is on it) – `check:core-only` itself ran green (59 hits, 2 escapes).
- shared/rig-designer and the creator's authoring code (allowed by the rule; not audited), deck/lab, the relay itself.
- native text.rs as a twin of the reader's dialog layout – real, but the reader is not the deck (noted in #378).
- The board status of #363–#378: `npm run backlog -- set N "Backlog"` needs `gh auth refresh -s project` (interactive) – not set.
