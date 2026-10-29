# deck/stage – the native AV app the deck boots into

`deck-stage` (agents/research/CYBERDECKS.md §8, #179): one window, a node graph from a TOML scene – video passes on wgpu
(Vulkan on the deck), an audio graph on PipeWire (cpal elsewhere), control values per frame – and chapter content from
scutoons.com's library API with a disk cache. On the deck `deck-session` runs it under `cage` instead of Chromium
(deck/os/README.md "What happens at boot"). Built so far: S1 (window, wgpu, graph core, test pattern, shader passes,
HUD) and S2 (audio graph, synths, FFT, chapter music). Open: S3–S6 (#179).

## Run

```bash
cd deck && cargo run --release -p deck-stage -- --windowed
```

`--scene FILE` (default: the built-in `scenes/default.toml`, "AV madness") · `--audio auto|pipewire|cpal|null` (auto =
PipeWire → cpal → null; null runs the engine on a timer, no device – the clock, the music and the FFT still move) ·
`--volume 0–1` · `--cache DIR` (default `/var/lib/deck/cache` on a deck, `DECK_CACHE`, else the user cache folder) ·
`--api` / `--cdn` (or `DECK_API` / `DECK_CDN`) · `--windowed` · `--frames N` (quit after N frames and print the numbers –
a check run) · `--no-hud`. Logs: `RUST_LOG`.

Keys: **F1** the HUD · **F2** the browser (exit code 10; deck-session opens the conductor page, closing it returns) ·
**Ctrl+Q** quit (on the deck the stage starts again) · Esc quits a `--windowed` stage.

The PipeWire backend is the cargo feature `pipewire` (Linux; needs libpipewire + clang at build time) – the ISO and
the PKGBUILD build with it. Without it the stage plays through cpal (ALSA on Linux, so PipeWire's ALSA plugin still works).

## Scenes

A scene is `[stage]` (`name`, `render_scale` 0.25–1) + `[[node]]` (`id`, `kind`, the kind's params) + `[[link]]`
(`from = "node.port"`, `to = "node.port"`; a bare id = its port `out`; control links map `value * scale + offset`).
An input takes one link; an output feeds any number. Every control input can also be set as a param of the same name.

| Kind | Inputs | Outputs | Params |
|---|---|---|---|
| `test_pattern` | speed, zoom, hue, pulse (control) | out (video) | – |
| `shader` | in (video), a, b, c, d (control) | out (video) | `effect` = `kaleido` · `rgb_shift` · `invert` · `pass`, or `wgsl` = a file defining `fn effect(uv: vec2<f32>) -> vec4<f32>` (bindings: shaders/common.wgsl) |
| `screen` | in (video) | – | the window (one per scene) |
| `lfo` | rate (Hz), depth | out (control, 0–depth) | `shape` = sine · saw · square |
| `osc` | cv (audio), freq, gain | out (audio) | `wave` = sine · saw · square · triangle, `attack`, `release` |
| `noise` | cutoff, gain | out | `color` = white · pink, `q` |
| `fm` | cv, freq, ratio, index, gain | out | `attack`, `release` |
| `seq` | rate | cv (audio: pitch Hz left, gate 0 / 1 right) | `steps` = MIDI notes (0 = rest), `bpm`, `division` (steps per beat), `gate`, `transpose` |
| `chapter_music` | gain | out | `toon`, `chapter`, `track` (default: the chapter's default track), `quality` = low · medium · high, `loop` |
| `mixer` | in1–in4 (audio), gain1–gain4 | out | – |
| `fft` | in (audio) | level, bass, mid, high (control, 0–1) | `release` (s) |
| `audio_out` | in (audio), volume | – | the device (one per scene) |

With a `cv` link an oscillator / FM voice takes its pitch and gate from the sequencer and ignores `freq`. A control link
out of the FFT or into an audio node crosses threads a frame late, so feedback through it (FFT → synth → mix → FFT) is
allowed; every other cycle is an error.

`scenes/default.toml` – catpope 1's music + a sequenced FM voice + pink noise → mixer → FFT → test pattern →
kaleidoscope → RGB shift → screen. `scenes/test-pattern.toml` – the picture alone (no sound, no network). Both land in
`/usr/share/deck/scenes` on the deck; `scene = "…"` in `/var/lib/deck/deck.toml` picks one.

## How it is built

- **Threads:** the event loop (winit) renders per frame at vsync (Fifo – no busy loop); the audio callback (PipeWire's
  realtime data thread, or cpal's) runs the audio nodes per block; one loader thread per `chapter_music` node.
- **Audio** (`src/audio/`): the audio nodes in topological order, one preallocated stereo buffer per output port, blocks of
  ≤ 1024 frames. Oscillators, noise and the lowpass are fundsp units, warmed up at build time; envelopes, sequencer,
  player (linear resampling to the device rate), mixer and FFT tap are ours. Control values reach the callback as atomics
  (`Param`), tracks through a lock-free ring (rtrb), the FFT's samples leave through another; a hard limit before the
  device. `tests/no_alloc.rs` counts allocations around 2000 callbacks of the default scene: zero. The frames the device
  played are the stage's clock.
- **Video** (`src/render.rs`): one fullscreen pass per video node into its own texture (Rgba8), the screen blits to the
  surface, the HUD (`src/hud.rs`, font8x8 into a small texture, integer scale by the screen height) on top. No hardware
  adapter → the software one (llvmpipe / lavapipe / WARP) at half the render size; a shader that does not compile →
  `pass` with the error in the log.
- **Library** (`src/library.rs`): `GET {api}/library/toons/<toon>/chapters/<n>` → `<cache>/api/…json` (the cached copy
  when offline), CDN files `{cdn}/<toon>/<path>` → `<cache>/cdn/<toon>/<path>` once (CDN paths are immutable), decoded
  whole with symphonia (mp3, wav).

Dependencies are pinned to the APIs this was written against (wgpu 25, winit 0.30, cpal 0.15, pipewire 0.10 (0.8's libspa no longer builds against Arch's PipeWire headers), fundsp
0.20, symphonia 0.5, ureq 2); `Cargo.lock` points gpu-allocator at windows 0.58 (cpal 0.15 pulls 0.54, which breaks
wgpu-hal's DX12 build on Windows). A bump is its own step.

## Measured

| Where | Scene | Numbers |
|---|---|---|
| Windows 10, RTX 2070 SUPER, Vulkan, release | default (null audio) | 60.3 fps, 16.6 ms avg frame, 0.29 ms CPU per frame, first frame 845 ms, catpope 1 music online in 1.1 s (cached 0.4 s) |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | default | open – the owner's stick (deck/HARDWARE.md) |
