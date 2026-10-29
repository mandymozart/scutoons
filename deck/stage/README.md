# deck/stage – the native AV app the deck boots into

`deck-stage` (agents/research/CYBERDECKS.md §8, #179): one window, a node graph from a TOML scene – video passes on wgpu
(Vulkan on the deck), an audio graph on PipeWire (cpal elsewhere), control values per frame – and chapter content from
scutoons.com's library API with a disk cache. On the deck `deck-session` runs it under `cage` instead of Chromium
(deck/os/README.md "What happens at boot"). Built so far: S1 (window, wgpu, graph core, test pattern, shader passes,
HUD), S2 (audio graph, synths, FFT, chapter music) and S3 (the chapter's slices with their dialog, the caption overlay,
the splash). Open: S4–S6 (#179).

## Run

```bash
cd deck && cargo run --release -p deck-stage -- --windowed
```

`--scene FILE` (default: the built-in `scenes/default.toml`, "AV madness") · `--audio auto|pipewire|cpal|null` (auto =
PipeWire → cpal → null; null runs the engine on a timer, no device – the clock, the music and the FFT still move) ·
`--volume 0–1` · `--cache DIR` (default `/var/lib/deck/cache` on a deck, `DECK_CACHE`, else the user cache folder) ·
`--api` / `--cdn` (or `DECK_API` / `DECK_CDN`) · `--windowed` · `--frames N` (quit after N frames and print the numbers –
a check run) · `--no-hud`. Logs: `RUST_LOG`.

Keys: **F1** the HUD · **F2** the browser · **Ctrl+Q** quit (on the deck the stage starts again) · Esc quits a
`--windowed` stage. F2 with `DECK_BROWSER` set (deck-session sets it to `deck-session browser`) starts that command on
top of the running stage: the logo comes up until the browser's window takes the focus (cage shows the newest window),
then the stage stops drawing and mutes; when the command exits the stage is back at once. Without `DECK_BROWSER` F2
quits with exit code 10.

**The splash** (owner 2026-10-29: "das scutoons logo gross in weiss"): the official logo (viewer/static/SCUTOONS-logo.svg
with a white fill – `assets/SCUTOONS-logo-white.svg`, rendered with resvg) at half the width on black, a loading line at
the top. The window's first frame is the splash (before the scene's shaders compile); it fades once every chapter
node shows a slice and every music track is loaded (or failed), at the latest after 30 s.

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
| `chapter` | in (video), next, hold, zoom, pulse (control) | out (video), text (video) | `toon`, `chapter`, `quality` = low · medium · high, `dialog` (true: set the dialog in), `start` (slice index), `threshold` (0.6), `min_hold` (1.5 s); `hold` default 6 s |
| `overlay` | in, top (video), opacity, pulse, shake (control) | out (video) | – |
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

**chapter** shows the chapter's slices one after the other in reading order (looping), the slice fitted to the height
over `in` (darkened; without an input a dark stretched copy of the slice), a slow push-in over the hold, a kick by
`pulse`, a short cross-fade with a flash at each cut. It cuts when `hold` is over, or when `next` rises past
`threshold` after `min_hold` (a bass hit). The dialog is set into the slice the way the reader places it (rect, size
in % of the slice width, shrunk to fit, paper boxes for captions / narration / framed bubbles, a rounded paper field
behind a plain bubble, coloured + stroked lettering, the rotation) in the reader's fonts – Bangers and IBM Plex Mono
Bold (`assets/`, SIL OFL); bubble tails, wobbly shapes and gradients are not drawn. `text` = the slice's caption: its
shortest painted lettering (a sound, a title), else its shortest line of dialog (≤ 48 characters), big in Bangers,
white with a black stroke on transparent – no caption, nothing. **overlay** puts `top` over `in` in the lower part of
the frame (80 % of the width, at most 40 % of the height), popping in when it changes.

With a `cv` link an oscillator / FM voice takes its pitch and gate from the sequencer and ignores `freq`. A control link
out of the FFT or into an audio node crosses threads a frame late, so feedback through it (FFT → synth → mix → FFT) is
allowed; every other cycle is an error.

`scenes/default.toml` ("AV madness (S3)") – catpope 1's music + a sequenced FM voice + pink noise → mixer → FFT; test
pattern → kaleidoscope → the chapter's slices (cut on a bass hit, every 6 s at the latest) → RGB shift → the caption
overlay → screen. `scenes/test-pattern.toml` – the picture alone (no sound, no network). Both land in
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
- **Slices** (`src/chapter.rs`, `src/text.rs`): one loader thread per chapter node – the chapter (retried every 10 s
  while there is neither a network nor a cached copy), then slice after slice: the image from the cache, decoded
  (`image`: webp, png), the dialog set in and the caption drawn on the CPU (ab_glyph), handed over through a channel of
  one (the next slice is always ready, at most two in memory); the renderer uploads it at the cut. A pass reads up to
  three textures (bindings 1, 3, 4: `in`, the current / previous slice or an overlay's `top`).

Dependencies are pinned to the APIs this was written against (wgpu 25, winit 0.30, cpal 0.15, pipewire 0.10 (0.8's libspa no longer builds against Arch's PipeWire headers), fundsp
0.20, symphonia 0.5, ureq 2, image 0.25, ab_glyph 0.2, resvg 0.45); `Cargo.lock` points gpu-allocator at windows 0.58 (cpal 0.15 pulls 0.54, which breaks
wgpu-hal's DX12 build on Windows). A bump is its own step.

## Measured

| Where | Scene | Numbers |
|---|---|---|
| Windows 10, RTX 2070 SUPER, Vulkan, release | default (null audio) | 60.3 fps, 16.6 ms avg frame, 0.29 ms CPU per frame, first frame 845 ms, catpope 1 music online in 1.1 s (cached 0.4 s) |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | default | open – the owner's stick (deck/HARDWARE.md) |
