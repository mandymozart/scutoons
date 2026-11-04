Status 2026-11-04: design note for #374 (the deck UI model crate); the first slice (the HUD lines, the stats bar, the HUD mode) is built with it. The remaining slices are listed at the end; the deck's own menu / preferences UI comes after all engine work (#386).

# DECK-UI-MODEL – the deck's UI model as a platform-free crate

**Why (#374, child of #362, Part of #311).** The deck's own UI is the only UI ([authority] in decisions/deck.md). Its model
– what each HUD line says, the bar's items, which mode shows – sat in deck-native's main.rs (`hud_lines`, `bar_items`,
`source_line`) and hud.rs (`Mode`, `wrap`, the sizes). None of it compiled to wasm, so a browser or headless deck could
not show the same layer. RULES A1 / A6 / AUDIT §3.8: logic in a platform crate is the same defect as logic in TS.

**The crate: `deck-ui` (deck/ui).** Plain data in, words out. Dependencies: deck-core only (the monitor's `Report`,
`Item`, `Level`, #364). **No wgpu, winit, threads, files, sockets, clocks**; registered in scripts/deck-deps.json and
built for wasm32-unknown-unknown in deck:ci and .github/workflows/deck.yml (a build check, like deck-outlet's, #378).

**The first slice (this ticket) moves:**
- `Status` – what the HUD reads, as plain values: the scene, the error, the outlet's lines and note, the GPU, the
  governor (`Quality`), the audio device (`Audio`), deck-core's monitor `Report` (#364), the hosts' event counters, the
  tempo line (`Controller::tempo_line`, #370) and the source (`Source`: name, status, MIDI out), the outputs, the sync
  and push lines (#195 / #371), the space (`Space`: mood, its line, the session lines – the relay status, #366), the FFT,
  the music and slice lines.
- `hud_lines(&Status)` – every line, its order and wording, the alert row, the wrap and the 13-line cut.
- `bar_items(&Status)` – the collapsed bar: the monitor's items, the quality (amber below the ceiling), the relay, the GPU, F1.
- `source_line(&Source)`, `Mode` (F1: Full → Bar → Off), `Item`, `wrap`, `LINES`, `COLS`, `ALERT`, `alert_row`, `beat_row`.
- The thresholds and the monitor's words stay deck-core's (#364); deck-ui only places them.

**What stays in deck-native (platform glue):**
- main.rs: winit, the renderer, when the HUD refreshes (every 0.25 s), the volume overlay's borrowing of the bar,
  forcing Full for an error / page / switcher, the F1 key → `Mode::next`.
- hud_status.rs (a child module of main.rs, adapter only): reads the platform objects (renderer, governor, audio
  backend, glitch counters, outlet, sync, space, outputs, the tempo's I/O) into a `deck_ui::Status`. No words, no
  decisions – a line there that formats text is a defect.
- hud.rs: the font8x8 raster of the lines and the bar, the frame-time graph, the beat lights, the HUD's integer scale.

**The test.** `deck/ui/tests/golden.rs`: scripted `Status` values → `tests/golden/hud.txt` (lines + bar); it was
checked byte-identical against main.rs's own builders before they left (a differential run, see the commit). The
headless host's `hud` step (scenarios/25-hud.toml, formerly x-hud) calls `deck_ui` with the headless deck's clock and
monitor – so tests/wasm.rs proves the same lines native ≡ wasm32.

**Remaining slices (#374, the orchestrator files them):**
1. The F4 settings page (`quality.rs` `Page`: its keys, option tables and lines over deck-space `Presets`, with #375).
2. The F5 scene switcher (`switcher.rs`: the list, its cursor, the transition's steps).
3. The volume overlay (`volume.rs`: its text and timing, the bar it borrows while the HUD is off).
4. The pairing card (`pairing.rs`: the code + QR card's model and raster).
5. A CPU raster or draw list both targets consume (hud.rs's font8x8 raster) – then #362 removes the Svelte overlays.
