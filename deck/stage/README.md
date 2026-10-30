# deck/stage – the native AV app the deck boots into

`deck-stage` (agents/research/CYBERDECKS.md §8, #179): one window, a node graph from a TOML scene – video passes on wgpu
(Vulkan on the deck), an audio graph on PipeWire (cpal elsewhere), control values per frame – and chapter content from
scutoons.com's library API with a disk cache. On the deck `deck-session` runs it under `cage` instead of Chromium
(deck/os/README.md "What happens at boot"). Built so far: S1 (window, wgpu, graph core, test pattern, shader passes,
HUD), S2 (audio graph, synths, FFT, chapter music) and S3 (the chapter's slices with their dialog, the caption overlay,
the splash), and the 3D space of #188 phases 1–3 (the Cathedral of decks: a glTF room, the deck's being, its 3D
buttons). Open: S4–S6 (#179), #188 phases 4–5 (several decks around the conductor over the relay, the portrait head).

## Run

```bash
cd deck && cargo run --release -p deck-stage -- --windowed
```

`--scene FILE` (default: the built-in `scenes/default.toml`, "AV madness"; `--scene cathedral` = the built-in
`scenes/cathedral.toml`) · `--rooms DIR` (a space node's rooms, `DECK_ROOMS`, default `<cache>/rooms`) · `--audio auto|pipewire|cpal|null` (auto =
PipeWire → cpal → null; null runs the engine on a timer, no device – the clock, the music and the FFT still move) ·
`--volume 0–1` · `--cache DIR` (default `/var/lib/deck/cache` on a deck, `DECK_CACHE`, else the user cache folder) ·
`--api` / `--cdn` (or `DECK_API` / `DECK_CDN`) · `--windowed` · `--frames N` (quit after N frames and print the numbers –
a check run) · `--screenshot FILE` (the last frame of a `--frames` run as a PNG) · `--pointer X,Y` (the pointer at a
window pixel for a check run – a hovered button shows in the HUD) · `--no-hud` · `--dev` (hot reload, below) ·
`--modules` (what this build carries). Logs: `RUST_LOG`. `DECK_SIMD=baseline` – the audio batches without AVX2 (to compare).
The graph's workloads: `cargo bench -p deck-stage` (patcher | batch | batch AVX2, µs per block).

Keys: **F1** the HUD · **F2** the browser · **Ctrl+Q** quit (on the deck the stage starts again) · **Ctrl+Alt+Del** power off (`systemctl
poweroff`, the logo stays up; not in a `--windowed` stage) · Esc quits a `--windowed` stage · the tempo clock: **T** tap ·
**+ / −** BPM · **Space** start / stop · **F3** the next source (internal → MIDI → Link → OSC → relay) · **K** the click. In a scene with a `space` node: a **mouse drag** turns the camera around the being (or, on a 3D
button, its value – up = more), the **wheel** / **↑ ↓** zoom, **← →** turn, **R** the camera's home; the pointer stays
visible in the kiosk then. The HUD names the keys. F2 with `DECK_BROWSER` set (deck-session sets it to `deck-session browser`) starts that command on
top of the running stage: the logo comes up until the browser's window takes the focus (cage shows the newest window),
then the stage stops drawing and mutes; when the command exits the stage is back at once. Without `DECK_BROWSER` F2
quits with exit code 10.

**The splash** (owner 2026-10-29: "das scutoons logo gross in weiss"): the official logo (viewer/static/SCUTOONS-logo.svg
with a white fill – `assets/SCUTOONS-logo-white.svg`, rendered with resvg) at half the width on black, a loading line at
the top. The window's first frame is the splash (before the scene's shaders compile); it fades once every chapter
node shows a slice and every music track is loaded (or failed), at the latest after 30 s.

## Modules (#185)

Modules are cargo features (DECISIONS 2026-10-30: compiled in, scene-driven builds; wasm generators later, no dylib
plugins). The **core** is always built: the graph, control (`lfo`, `tempo` with the internal clock and the relay stub),
the basic audio nodes (`osc`, `noise`, `fm`, `seq`, `click`, `mixer`, `fft`, `audio_out`), the render passes
(`test_pattern`, `shader`, `overlay`, `screen`), the HUD, cpal.

| Feature | Adds | Dependencies |
|---|---|---|
| `space` | the `space` kind (3D room + the being, #188) | glam, image/webp, serde_json |
| `chapter` | `chapter`, `chapter_music` (the library API + cache) | ureq, symphonia, ab_glyph, image/webp |
| `midi` | the tempo source `midi`, `midi_out` (MIDI clock in / out) | midir |
| `osc` | the tempo source `osc` (`/tempo/*`) | rosc |
| `link` | the tempo source `link` (Ableton Link) | rusty_link (cmake, libclang) |
| `pipewire` | the PipeWire backend (Linux) | pipewire (clang) |

Default = `space`, `chapter`, `midi`, `osc` – a plain build plays every bundled scene. The table lives in deck-core
(`core/src/stage.rs`, data only); `src/modules.rs` says which modules this build carries. A scene that needs a module
the build lacks is refused at load with the feature's name (`this build of the stage does not carry the module 'space'
(node space) – build deck-stage with --features space`): the stage plays the test pattern and shows that in the HUD –
it never exits for it (deck-session would fall back to the browser); a `--frames` check run exits 2. F3 onto a source
that isn't built says `NOT IN THIS BUILD`. **Scene-driven builds:** `cargo run -p deck-scene-features -- scenes/*.toml`
prints the features a set of scenes needs (`chapter,space` for the bundled ones; the core alone prints nothing), so a
deck build is `cargo build --release -p deck-stage --no-default-features --features "pipewire,$(deck-scene-features …)"`
– the platform's features (pipewire) come from the build, not from scenes. Checked: the default, the core alone, each
feature alone, all with `link` – tests and clippy `-D warnings`.

**Hot reload** (`--dev`): the scene file and its shader nodes' `wgsl` files are watched (`src/watch.rs`, their
modification times polled every 300 ms). A changed WGSL file recompiles only its pass – the audio plays on; one that
doesn't compile keeps the running pipeline, the error in the HUD. A changed scene is checked first (a broken one keeps
the running show and says why), then the show is rebuilt: audio device, tempo sources, loaders, renderer.

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
| `chapter` | in (video), next, hold, zoom, pulse (control) | out (video), text (video) | `toon`, `chapter`, `quality` = low · medium · high, `dialog` (true: set the dialog in), `start` (slice index), `threshold` (0.6), `min_hold` (1.5 s), `quantize` = off · beat · bar (a due cut waits for the tempo clock's next beat / bar); `hold` default 6 s |
| `overlay` | in, top (video), opacity, pulse, shake (control) | out (video) | – |
| `screen` | in (video) | – | the window (one per scene) |
| `lfo` | rate (Hz), depth | out (control, 0–depth) | `shape` = sine · saw · square |
| `tempo` | – | bpm, beat, bar (counts), phase, bar_phase (0–1), pulse (1 on the beat, gone by its middle) | `bpm` (120), `beats` per bar (4), `source` = internal · midi · link · osc · relay, `click` (false), `midi_in` / `midi_out` (a part of a port name; out = MIDI clock out), `midi_out_lead` (ms), `osc_port` (9001) – one per scene |
| `click` | gain | out | `freq` (1000 Hz), `accent` (1500 Hz, the bar's first beat), `decay` (0.03 s) – ticks on the tempo clock while the click is on |
| `osc` | cv (audio), freq, gain | out (audio) | `wave` = sine · saw · square · triangle, `attack`, `release` |
| `noise` | cutoff, gain | out | `color` = white · pink, `q` |
| `fm` | cv, freq, ratio, index, gain | out | `attack`, `release` |
| `seq` | rate | cv (audio: pitch Hz left, gate 0 / 1 right) | `steps` = MIDI notes (0 = rest), `division` (steps per beat), `gate`, `transpose`; on the tempo clock (step k from beat k / division, `rate` scales the division) unless it has a `bpm` of its own |
| `chapter_music` | gain | out | `toon`, `chapter`, `track` (default: the chapter's default track), `quality` = low · medium · high, `loop` |
| `mixer` | in1–in4 (audio), gain1–gain4 | out | – |
| `fft` | in (audio) | level, bass, mid, high (control, 0–1) | `release` (s) |
| `audio_out` | in (audio), volume | – | the device (one per scene) |
| `space` | hue, glow, light, pulse (control) | out (video) | `room` (a name → `<rooms>/<room>.<quality>.glb`, or a path with its extension), `quality` = low · medium · high (low, the X1's), `at` = [x, y, z] (where the being floats, room metres) or `spot` (a node of the room, `spot_aisle`) + `lift` (1.5 m), `yaw` / `pitch` (°) / `distance` (m) – the camera's start, `max_texture` (2048) – one per scene |

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

**space** (#188 "Cathedral of decks", phases 1–3) draws a glTF room and this deck's **being** – the running graph as a
floating body – through an orbit camera. The body: a translucent core (the space node's own point sits in it), one node
point per graph node (sources low, sinks high: data rises toward where the head will be), a hose per link (a curved
tube; audio thick, video middle, control thin) and thin tendrils from the core to every point. **Bubbles** run along a
hose from its source to its target when its value changes – control links: the change against the value's recent
range, more and bigger the more it moves (a steady value: none); audio links: the source buffer's block peak; video
links: a steady trickle. Every point carries its node's control inputs as **3D buttons** (a lit band shows the value
in its range, `space/params.rs`) and an LFO also its waveform (sine · saw · square); dragging one sets the input like
a MIDI CC (`Controls::set`), a button on a linked input has a dark collar, shows the hose's value and does not turn.
The HUD's last line names the hovered button (`VOICE.INDEX 2.00 [0.00-10.0]`, `SPACE.PULSE 0.58 < FFT`) or node. The
body drifts, sways and breathes on the tempo clock (a swell on each beat, a wave over the bar; `pulse` kicks it); `hue`
/ `glow` colour it, `light` dims the room. The scene's `[being]` table is its shape: `size` (body radius, 0.5 m),
`drift` (0.06 m), `breath` (0.035), `wobble` (0.025), `hose` / `bubbles` (thickness / rate factors), `expose` (the
buttons: `"node"` or `"node.port"`, empty = all) – with the graph that is what a being preset saves (#191).
`scenes/cathedral.toml` ("Cathedral of decks") – the room from Cat Pope chapter 2 (`cathedral`, low), the being ahead of
the reader's view in the nave; catpope 2's music, a choir line (seq → FM), a drone (its gain on the bass), pink-noise
incense swayed by an LFO → mixer → FFT (its level kicks the breath) → out. An audio oscillator's `wave` can't change
while it plays yet (an engine rebuild); MIDI / OSC learn onto the same `Controls::set` is S6.

`scenes/default.toml` ("AV madness (S3)") – the tempo clock at 96 BPM; catpope 1's music + an FM voice sequenced on
the beat + pink noise + the click (off until K) → mixer → FFT; test pattern (pulsing on the beat) → kaleidoscope → the
chapter's slices (cut on a bass hit snapped to the beat, every 6 s at the latest) → RGB shift → the caption overlay →
screen. `scenes/test-pattern.toml` – the picture alone (no sound, no network). Both land in
`/usr/share/deck/scenes` on the deck; `scene = "…"` in `/var/lib/deck/deck.toml` picks one.

## How it is built

- **Threads:** the event loop (winit) renders per frame at vsync (Fifo – no busy loop); the audio callback (PipeWire's
  realtime data thread, or cpal's) runs the audio nodes per block; one loader thread per `chapter_music` node.
- **The graph runs on the batch model** (#183 – the engine model of ECS systems, TouchDesigner's channels, gen~, over
  the patcher's one object per node): compiled at load into dependency levels × kinds, struct-of-arrays loops.
  **Control** (`src/control.rs`): one value slot per control output, every control input a column (deck-core's `Col`:
  the linked slot × scale + offset, or the param as a constant), the LFOs one batch per level and shape. **Audio**
  (`src/audio/`): levels by the audio links; a level's sine oscillators and FM voices run as one batch each
  (`voices.rs`: groups of eight voices in deck-core's `F8` lanes, the group's state in registers for the block),
  every other node as a block of its own (`blocks.rs`, the patcher model for one-off nodes: saw / square / triangle
  oscillators, noise and the lowpass on fundsp; sequencer, click, player with linear resampling, mixer, FFT tap,
  output). One preallocated stereo buffer per output port, blocks of ≤ 1024 frames. `Build::patcher()` runs every node
  as a block (the stage before #183) – the reference the tests compare with: two seconds of the same scene agree
  within 2e-7 (sines), 1e-5 (the default scene), 2e-4 (16 FM voices at index 4.5 – FM multiplies deck-core's sine
  error by the modulation). Control values reach the callback as atomics (`Param`), tracks through a lock-free ring
  (rtrb), the FFT's samples leave through another; a hard limit before the device. `tests/no_alloc.rs` counts
  allocations around 2000 callbacks of the default scene (the click on, the tempo changing), AVX2 and baseline: zero.
  Each block is stamped with the moment it is heard – the time base (`src/clock.rs`).
- **x86-64-v3** (owner 2026-10-30): one baseline x86-64 binary; the voice batches' loops are compiled twice (once with
  `#[target_feature(avx2, fma)]`) and picked at start by CPU detection, bit-identical results – the start log says
  `audio graph: batch avx2 …`, a `--frames` run's summary too; `DECK_SIMD=baseline` keeps the baseline loops. A
  whole-program v3 binary (`cargo stage-v3`, deck/.cargo/config.toml) is faster again on the one-off nodes (the default
  scene 10.0 → 7.5 µs per block); the ISO may ship it beside the baseline one (#183).
- **Shared with deck-core** (#183, #185): the maths, `F8` lanes and batch columns, the FFT analyzer (`analysis::Analyzer`
  – `src/fft.rs` only drains the audio ring into it) and the tempo map (`tempo`: map, quantize, tap); the same code
  builds for wasm32 (the browser's Live mode).
- **Tempo** (`src/tempo/`, #181): a tempo map over the audio frame position (beat = f(audio position)); the frame thread
  follows the master source, re-anchors the map at the audio thread's frontier and publishes it through a seqlock – the
  click and the sequencer read it per block (sample-accurate), MIDI clock out (`midi_out`, its own thread) per tick, the
  `tempo` node per frame. Sources stamp their messages on arrival; MIDI clock in is fitted by least squares over two
  beats (±1 ms of jitter → < 0.1 BPM, < 1 ms of phase), Link answers its beat for the frame's moment, OSC listens for
  `/tempo/bpm f`, `/tempo/beat f`, `/tempo/start`, `/tempo/stop`, `/tempo/tap`; the relay is a stub (no show clock yet).
  Ableton Link is the cargo feature `link` (rusty_link 0.4.9: cmake, a C++ compiler and libclang at build time; on
  Windows a debug build needs `--config 'profile.dev.package.rusty_link.opt-level=1'`, else its debug CRT doesn't link).
- **Video** (`src/render.rs`): one fullscreen pass per video node into its own texture (Rgba8), the screen blits to the
  surface, the HUD (`src/hud.rs`, font8x8 into a small texture, integer scale by the screen height) on top. No hardware
  adapter → the software one (llvmpipe / lavapipe / WARP) at half the render size; a shader that does not compile →
  `pass` with the error in the log.
- **Space** (`src/space/`, #188): `glb.rs` reads a room (.glb) – meshes with POSITION, COLOR_0 (the rooms' build
  bakes Unreal's lights into the vertex colours), TEXCOORD_0, unlit base-colour materials, WebP / PNG images
  (`EXT_texture_webp`), quantized attributes (`KHR_mesh_quantization`), **`EXT_meshopt_compression`** (decoded by
  `meshopt.rs` in plain Rust – the vertex codec v0 and the index codec v0 / v1 of the extension's bitstream spec; the
  decoded positions are checked against the accessors' min / max), named empty nodes as spots – flattened at load into
  one world-space vertex / index list with a draw per texture, the images halved to `max_texture` and their mip chains
  made on the loader's thread (the frame that uploads only copies). Not read: the INDICES mode, meshopt filters, Draco,
  KTX2 / Basis, external buffers – a room with those is refused with the extension's name (the rooms' build encodes
  with meshopt's FILTER method, which filters only normals / rotations – the rooms carry none); a plain variant: the
  rooms' build (tools/rooms/pipeline/build.mjs §5) without its `EXTMeshoptCompression`. `camera.rs` the orbit camera (glam; it turns by itself after 4 s without input, eased in; the ray through a
  pixel for picking), `being.rs` the body (layout from the graph, the hoses' Béziers, the bubble pools – 12 per hose –,
  the buttons, picking by ray / sphere), `gpu.rs` + `shaders/space.wgsl` the passes into the node's texture with its
  own Depth32 buffer: the room unlit (texture × vertex light × `light`, fog), the solid spheres (nuclei, buttons), then
  the tubes and the ghost spheres (core, shells, bubbles) additive and depth-tested only. The being's buffers are sized
  once (`Being::capacity`) and rewritten per frame; `Being::update` / `write` never allocate (tests/no_alloc.rs). The
  values come from the control domain's slots and the audio meters (`audio::Meters`: every output buffer's block peak,
  only built for a scene with a space node).
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
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-30, #188) | cathedral (null audio) – room low / medium / high | 60.4 / 60.4 / 60.3 fps (vsync), CPU per frame 0.37 / 0.39 / 0.40 ms (the default scene: 0.31), first frame 719 / 802 / 827 ms; the room loads on its thread in 174 / 449 / 1574 ms (114,788 / 116,163 / 129,205 triangles, 4 WebP + their mips), its upload 2 / 4 / 9 ms; p99 ~50 ms in every scene = the start |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | cathedral | open – the owner's stick |
| Windows 10, Ryzen 5 3600X, `cargo bench -p deck-stage` (2026-10-30, #183) | the graph, µs per 128-frame block: before #183 → patcher → batch → batch AVX2 | default scene 11.0 → 9.9 → 11.1 → 10.0 (whole v3 binary 7.5) · 1 FM voice – → 2.2 → 3.5 → 2.2 · 16 FM voices 37.8 → 34.1 → 10.2 → 7.7 · 16 sines 20.1 → 15.4 → 7.3 → 5.3 · 64 FM voices – → 125 → 41 → 27 · control step, 300 LFOs: 58.4 → 2.1 (3 levels × 100), 69.6 → 5.5 (300 levels) |
