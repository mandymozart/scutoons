# deck/stage – the native AV app the deck boots into

`deck-stage` (agents/research/CYBERDECKS.md §8, #179): one window, a node graph from a TOML scene – video passes on wgpu
(Vulkan on the deck), an audio graph on PipeWire (cpal elsewhere), control values per frame – and chapter content from
scutoons.com's library API with a disk cache. On the deck `deck-session` runs it under `cage` instead of Chromium
(deck/os/README.md "What happens at boot"). Built so far: S1 (window, wgpu, graph core, test pattern, shader passes,
HUD), S2 (audio graph, synths, FFT, chapter music) and S3 (the chapter's slices with their dialog, the caption overlay,
the splash), the 3D space of #188 (the Cathedral of decks: a set's glTF, the deck's being with its portrait head and
3D buttons, the decks around the conductor over the relay) and the Live outlet of #186 (the relay's cells for this
deck's outlet, natively – below) and the sync of #195 (the scene's sets and portraits from the API into the cache –
below). Open: S4–S6 (#179), #188's body parts and the browser scene.

## Run

```bash
cd deck && cargo run --release -p deck-stage -- --windowed
```

`--scene FILE` (default: the built-in `scenes/default.toml`, "AV madness"; `--scene cathedral` / `conductor` = the
built-in `scenes/cathedral.toml` / `conductor.toml`) · `--sets DIR` (a local folder of sets instead of the synced
`<cache>/sets`, `DECK_SETS` – e.g. `../media/scutoons/rooms`, tools/rooms' output) · `--portraits DIR` (a local folder
of the heads' sprites instead of the synced `<cache>/portraits`, `DECK_PORTRAITS` – e.g. `../media/characters`) ·
`DECK_TOKEN` (a fixed deck token for the API and show media – a dev deck; a deck reads deck-link's, #180) · `--token T`
(the conductor's session token, `LIVE_TOKEN`) · `--audio auto|pipewire|cpal|null` (auto =
PipeWire → cpal → null; null runs the engine on a timer, no device – the clock, the music and the FFT still move) ·
`--audio-host` · `--audio-device` · `--audio-buffer N` · `--audio-thread on|off` · `--audio-list` ("Audio – live-safe" below) ·
`--volume 0–1` · `--cache DIR` (default `/var/lib/deck/cache` on a deck, `DECK_CACHE`, else the user cache folder) ·
`--api` / `--cdn` (or `DECK_API` / `DECK_CDN`) · `--windowed` · `--frames N` (quit after N frames and print the numbers –
a check run) · `--screenshot FILE` (the last frame of a `--frames` run as a PNG) · `--pointer X,Y` (the pointer at a
window pixel for a check run – a hovered button shows in the HUD; or `--pointer voice.index`: on that button) ·
`--drag voice.index,0.3` (a check run's drag: pressed after 60 frames, pulled 0.3 of its range out over 30, held) · `--no-hud` · `--hud-bar` (start with the collapsed stats bar) · `--dev` (hot reload, below) ·
`--modules` (what this build carries). Logs: `RUST_LOG`. `DECK_SIMD=baseline` – the audio batches without AVX2 (to compare).
The graph's workloads: `cargo bench -p deck-stage` (patcher | batch | batch AVX2, µs per block).

Keys: **F1** the HUD → the collapsed stats bar (#209: one line across the whole top edge – FPS, CPU / frame, the audio's
glitches / DSP load / limiter, the relay, the GPU backend; red when out of the green) → nothing · **F2** the browser · **Ctrl+Q** quit (on the deck the stage starts again) · **Ctrl+Alt+Del** power off (`systemctl
poweroff`, the logo stays up; not in a `--windowed` stage) · Esc quits a `--windowed` stage · the tempo clock: **T** tap ·
**+ / −** BPM · **Space** start / stop · **F3** the next source (internal → MIDI → Link → OSC → relay) · **K** the click. In a scene with a `space` node: a **mouse drag** turns the camera around the being (or, on a 3D
button, pulls it along its tether – out = more; a double click: back to the scene's value; a finger the same), the **wheel** / **↑ ↓** zoom, **← →** turn, **R** the camera's home; the pointer stays
visible in the kiosk then. The HUD names the keys. F2 with `DECK_BROWSER` set (deck-session sets it to `deck-session browser`) starts that command on
top of the running stage: the logo comes up until the browser's window takes the focus (cage shows the newest window),
then the stage stops drawing and mutes; when the command exits the stage is back at once. Without `DECK_BROWSER` F2
quits with exit code 10.

**The splash** (owner 2026-10-29: "das scutoons logo gross in weiss"): the official logo (viewer/static/SCUTOONS-logo.svg
with a white fill – `assets/SCUTOONS-logo-white.svg`, rendered with resvg) at half the width on black, a loading line at
the top. The window's first frame is the splash (before the scene's shaders compile); it fades once every chapter
node shows a slice and every music track is loaded (or failed), at the latest after 30 s.

**Pairing** (#180, CYBERDECKS.md §9): while deck-link's daemon says the deck waits for a member
(`/run/deck-link/state` = `pending <code>`, read every 2 s – `DECK_LINK_DIR` elsewhere), a card in the lower right
corner – over the splash and the scene – shows the code (`K7Q-42M`) and the QR of
`https://scutoons.com/account/decks?deck=<code>` (`DECK_APP` for another viewer); paired, offline or no daemon = no
card (`pairing.rs`, rastered like the HUD, integer scale). Built-in scenes play before approval; once approved the
deck token (`/run/deck-link/token`, renewed by the daemon) goes to the API – the library's chapters, the sync's
scene-content, the outlet's show media.

## Modules (#185)

Modules are cargo features (DECISIONS 2026-10-30: compiled in, scene-driven builds; wasm generators later, no dylib
plugins). The **core** is always built: the graph, control (`lfo`, `tempo` with the internal clock and the relay stub),
the basic audio nodes (`osc`, `noise`, `fm`, `seq`, `click`, `mixer`, `fft`, `audio_out`), the render passes
(`test_pattern`, `shader`, `overlay`, `screen`), the HUD, cpal.

| Feature | Adds | Dependencies |
|---|---|---|
| `space` | the `space` kind (a set's 3D + the being, #188) | glam, image/webp, serde_json |
| `sync` | the scene's sets and portraits from the API into the cache (#195 – below); no node kind | ureq, serde_json |
| `chapter` | `chapter`, `chapter_music` (the library API + cache) | ureq, symphonia, ab_glyph, image/webp |
| `live` | `outlet` (a Live outlet, #186 – below) | `chapter`, protocol, tungstenite |
| `midi` | the tempo source `midi`, `midi_out` (MIDI clock in / out) | midir |
| `osc` | the tempo source `osc` (`/tempo/*`) | rosc |
| `link` | the tempo source `link` (Ableton Link) | rusty_link (cmake, libclang) |
| `pipewire` | the PipeWire backend (Linux) | pipewire (clang) |

Default = `space`, `sync`, `chapter`, `live`, `midi`, `osc` – a plain build plays every bundled scene. The table lives in deck-core
(`core/src/stage.rs`, data only); `src/modules.rs` says which modules this build carries. A scene that needs a module
the build lacks is refused at load with the feature's name (`this build of the stage does not carry the module 'space'
(node space) – build deck-stage with --features space`): the stage plays the test pattern and shows that in the HUD –
it never exits for it (deck-session would fall back to the browser); a `--frames` check run exits 2. F3 onto a source
that isn't built says `NOT IN THIS BUILD`. **Scene-driven builds:** `cargo run -p deck-scene-features -- scenes/*.toml`
prints the features a set of scenes needs (`chapter,live,space` for the bundled ones; the core alone prints nothing), so a
deck build is `cargo build --release -p deck-stage --no-default-features --features "pipewire,$(deck-scene-features …)"`
– the platform's features (pipewire) come from the build, not from scenes. Checked: the default, the core alone, each
feature alone, all with `link` – tests and clippy `-D warnings`.

**Hot reload** (`--dev`): the scene file and its shader nodes' `wgsl` files are watched (`src/watch.rs`, their
modification times polled every 300 ms). A changed WGSL file recompiles only its pass – the audio plays on; one that
doesn't compile keeps the running pipeline, the error in the HUD. A changed scene is checked first (a broken one keeps
the running show and says why), then the show is rebuilt: audio device, tempo sources, loaders, renderer.

The PipeWire backend is the cargo feature `pipewire` (Linux; needs libpipewire + clang at build time) – the ISO and
the PKGBUILD build with it. Without it the stage plays through cpal (ALSA on Linux, so PipeWire's ALSA plugin still works).

## Audio – live-safe (#208)

A live tool may not crackle unnoticed, and should not crackle at all:
- **The glitch indicator** (`src/audio/glitch.rs`): every device callback is measured – the interval since the last
  one against its deadline, the time spent in it against its budget. A glitch is an **underrun** (the device ran dry:
  the ring was short, a gap in the heard stamps, or – WASAPI – a callback that finds the whole buffer free), a **long
  render** (over the budget) or **late** (later than the deadline + half of it + 2 ms; the deadline is a period, or
  WASAPI's whole buffer – its callbacks jitter 10 / 20 ms by design). Counts in atomics, each glitch's numbers through
  a preallocated ring to the frame thread, which logs them (`audio glitch: underrun – 480 frames, interval …`). The HUD
  flashes **`AUDIO! UNDERRUN - n IN 10 MIN`** red under the title for 5 s; its audio lines: `AUDIO WASAPI 48K BUF 1056
  22MS LEAD 5MS` (the host, the buffer the device really runs = its largest callback, the ring's lead) and `OUT 44MS DSP
  3% GLITCH 0/10M LIM 0` (rendered ahead of the ear, the peak callback / block load, glitches in 10 minutes, the
  limiter). A `--frames` run's summary: `glitches=N (late a long b underrun c) buffer=… lead=…`.
- **The render thread** (`src/audio/ring.rs`, `--audio-thread on` = the default; `off` renders in the callback): the
  graph runs in blocks of 256 frames on a thread of its own into a lock-free SPSC ring (rtrb), the callback only
  copies. The ring holds the device's buffer + the **lead**: one block (5 ms) at first, a block more per glitch up to
  16 (85 ms), a block less per calm minute. The time base stays exact (the callback's (frames taken, heard at) through
  a seqlock; each block is stamped with it + the frames between). `tests/no_alloc.rs` runs both halves: no allocation.
- **Priority** (`src/audio/priority.rs`): the render thread – or the callback's thread when it renders (WASAPI, ALSA;
  not ASIO's, that is the driver's) – at Windows MMCSS "Pro Audio" (high), Linux SCHED_FIFO 70 (the deck's user is in
  `realtime`; PipeWire's own threads at 88 through rtkit).
- **The buffer** (`--audio-buffer N` · deck.toml `audio_buffer` · the scene's `[stage] audio_buffer`, in that order):
  cpal's `BufferSize::Fixed(N)` clamped to the device's range (a log line says so), a refused one falls back to the
  default; PipeWire's latency request (default 128). **WASAPI shared can only grow it**: on the RME the engine runs a
  1056-frame (22 ms) buffer whatever is asked below that – a smaller buffer needs ASIO.
- **Host and device**: `--audio-host default|wasapi|asio` (deck.toml `audio_host`), `--audio-device NAME` (a part of
  the name, any case; deck.toml `audio_device`), `--audio-list` prints the hosts, devices, formats and buffer ranges.
  **WASAPI exclusive** is not possible: cpal (0.15 and 0.16) opens WASAPI in shared mode only.
- **ASIO** (the RME Fireface's reference driver on Windows) is the cargo feature `asio` (cpal's asio host; output in
  f32, i16 or i32 – RME's driver talks i32). Its `asio-sys` builds bindings to Steinberg's ASIO SDK at build time:
  1. get the ASIO SDK from steinberg.net (a free download after accepting Steinberg's licence) and unpack it, e.g. to
     `C:\SDK\asiosdk` (the folder holding `common\`, `host\`);
  2. LLVM for bindgen (`winget install LLVM.LLVM`; `LIBCLANG_PATH=C:\Program Files\LLVM\bin`) and the MSVC build tools;
  3. `set CPAL_ASIO_DIR=C:\SDK\asiosdk` (without it, or with a folder that doesn't exist, asio-sys downloads the SDK
     itself into %TEMP% – set it), then `cargo build --release -p deck-stage --features asio`;
  4. run `deck-stage --audio cpal --audio-host asio --audio-device fireface --audio-buffer 128` (the RME's driver is
     `ASIO Fireface USB`; the buffer is also the driver's setting in RME's Settings dialog – cpal asks the driver for it).
  ASIO hands one client the interface's driver (RME's is multi-client); the HUD then says `AUDIO ASIO …`.
- **Measured** (2026-10-31, Windows 10, RME Fireface UCX II on WASAPI shared, the cathedral at volume 0 – silent, real
  device timing –, release build, 2 min per setting; "load" = 12 busy threads on the 12 logical cores):

  | buffer asked → run | render | no load | load |
  |---|---|---|---|
  | 128 / 256 / 512 → 1056 (22 ms) | in the callback (raised to MMCSS) | 0 · 0 · 0 | 0 · 0 · 0, and 0 in 8 min |
  | 128 / 256 / 512 → 1056 | render thread, callback not raised | 0 · 0 · 0 | 1 · 0 · 1, and 3 in 8 min |
  | 128 / 256 / 512 → 1056 | render thread, callback raised (now) | – | 0 in 8 min |
  | 2048 → 2048 (43 ms) | callback / render thread | 0 / 0 | 0 / 0 |

  Every glitch was the same: cpal's WASAPI thread (its own TIME_CRITICAL priority) woke 32–38 ms after the last
  callback and found the 22 ms buffer drained – the ring can't help a late device thread, MMCSS for that thread does;
  so the callback's thread is raised in both modes. The DSP load stays at ~3 % of a block, so the render itself never
  came near its budget. What this can't see: a glitch inside Windows' audio engine (audiodg) or the USB driver (DPC
  latency) after our buffer – crackles heard while the indicator stays at 0 point there (LatencyMon; ASIO bypasses
  audiodg).

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
| `outlet` | in (video: the backdrop), opacity | out (video) | – (one per scene; the relay and the outlet from deck.toml / the CLI – "Live outlet") |
| `space` | hue, glow, light, pulse; the post knobs bloom, threshold, lift, gamma, gain, pixel, levels (control) | out (video) | `post` = the post stack's passes in order (bloom · curves · pixelate · fxaa, #204 – below), `palette` (pixelate's colours, ≤ 16 "#rrggbb"), `glow_range` (7 m) / `glow_light` (1, 0 = off) – the core's light in the room, `set` (a set id → its 3D at `quality`, `<sets>/<id>/<id>.<quality>.glb`, synced; or a path with its extension, relative to `--sets`), `quality` = low · medium · high (low, the X1's), `at` = [x, y, z] (where the being floats, the set's metres) or `spot` (a node of the set's glTF, `spot_aisle`) + `lift` (1.5 m), `yaw` / `pitch` (°) / `distance` (m) – the camera's start, `auto_turn` (false – the camera turns only by hand; true: slowly by itself after 4 s idle), `max_texture` (2048), `hide` (glTF nodes whose mesh is left out, by name), `join` = deck · conductor · alone (a session with a relay set), `ring` (3.2 m, the decks' circle around a conductor) – one per scene |

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

**space** (#188 "Cathedral of decks") draws a set's glTF and this deck's **being** – the running graph as a
floating body – through an orbit camera. The body: a translucent core (the space node's own point sits in it), one node
point per graph node (sources low, sinks high: data rises toward the head), a hose per link (a curved
tube; audio thick, video middle, control thin) and thin tendrils from the core to every point. **Bubbles** run along a
hose from its source to its target when its value changes – control links: the change against the value's recent
range, more and bigger the more it moves (a steady value: none); audio links: the source buffer's block peak; video
links: a steady trickle. Every point carries its node's control inputs as **3D buttons** (deck-space `params.rs` ranges) and an LFO also its
waveform (sine · saw · square), each on a **tether** (owner 2026-10-31): the core's as spokes, any other node's as a
chain, each button hanging from the one before it; its value is its distance from that point (low = close, high = far,
clamped – `tether_len`), so a deck turned up wide sprawls and a quiet one stays compact, on the conductor's mirrored
beings too. The tether is a thin hose in the parameter's own colour (`param_hue`: freq red, glow amber, index blue …),
thicker and brighter with the value; while the value moves, bubbles run along it to the node, the node flashes and its
hoses glow (what the button reaches). A colour control (a hue) wears the colour it is set to – knob, tether, ring and
its node point. **No numbers** (owner 2026-10-31): no labels, no range ends; instead **every knob always wears a thin
270° ring lit up to its value** (`kind::GAUGE` – the value at rest), wider and brighter under the hand. **A linked
knob turns as its base** (#188): a linked input is `source · scale + base`, the knob sets the base (the link's offset;
`Controls::set` / deck-core's `BatchGraph::set_base` / live-core's wire mode `base`) and the modulation adds on top –
its ring shows the base lit, the swing the modulation covered lately as a band and the value now as a white tick; a
double click eases it back to the link's offset. Only a mirrored deck's linked knob (the stream carries no base) is
locked: pale, a dashed ring, it shakes when grabbed. **The knobs face the camera** (#209, owner 2026-10-31: "always in sight of the camera … never hidden behind the body or the head"): the core's
spokes fan as a crown around the view axis toward the eye, a chain leans to the eye's side (one turned away rounds the body along its point's way
out), and every knob is moved sideways out of the core's and the head's shadow – eased as the camera orbits (`being.rs` `crown` / `lean` / `clear_of`).
**A tap selects** a knob or the being (a white ring; empty space deselects – `Space::take_tap`): what the browser's corner sticks drive
(deck-space `controls.rs`: the selection → the sticks' pairs, MIDI learn, the mappings as text for a preset). **Feedback:** hover swells a button with a thin ring (the pointer a
hand); a drag pulls it along its tether like a MIDI CC (`Controls::set`); a double click eases it back to the scene's
value (0.5 s). **Level and overload:** a node point glows with its audio level; the head flushes red with the output
limiter's gain reduction (`Being::strain`, full at 9 dB) and the feelings' `strain` makes it dizzy. **The camera
focuses like a photographer** (`camera::focus_target`): far out (7 sizes) at the whole being's centre, close up (2.2
sizes) at its head, smoothly between. Bubbles stay well inside their tubes (`BUBBLE_FILL` 0.6 of the radius). A finger on a touch screen (winit's touch events) does the same with twice the reach. No click
sound: the audio graph has no cheap one-shot path from the frame thread (the click node only ticks the tempo map). The HUD's last line names the hovered button (`VOICE.INDEX 2.00`, `INCENSE.CUTOFF 600.0 < SWAY` – a linked one's base) or node. The
body drifts, sways and breathes on the tempo clock (a swell on each beat, a wave over the bar; `pulse` kicks it); `hue`
/ `glow` colour it, `light` dims the set. The scene's `[being]` table is its shape: `size` (body radius, 0.5 m),
`drift` (0.06 m), `breath` (0.035), `wobble` (0.025), `hose` / `bubbles` (thickness / rate factors), `expose` (the
buttons: `"node"` or `"node.port"`, empty = all), `character` + `moods` (the head), `feel` + `react` (its feelings,
below) – with the graph that is what a being preset saves (#191).

**The head** (phase 5; owner 2026-10-30: the portrait *is* the head, organic on synthetic, Ghost in the Shell): the
character's portrait mood sprite (`<character>_portrait-<mood>.png|webp`) on a curved face plate turned to the eye,
over a dark machine layer behind it (circuit lines lit by the energy – the parallax of a layered head), inside a
translucent synthetic skin (a ghost sphere with scan lines); below the chin the sprite dissolves into the neck, where
cables from the core and the body's highest points plug into sockets, their bands rising into the head. Without a
feeling firing the **mood follows the energy** (0.8 × the FFT level + 0.2 × the tempo): `moods` calm → wild (default
neutral · thinking · happy · laughing · surprised), a new mood must hold 1.2 s, then cross-fades in 0.5 s. The mood set
(production 2026-10-30): acceptance · angry · bargaining · confused · crying · denial · disgusted · embarrassed · furious
· happy · laughing · neutral · sad · scared · smug · surprised · thinking in every full set, + dizzy (the cat popes,
the helmet mouse), dying (billthefrog), tripping (yellowmouse); a missing sprite shows its nearest moods
(`portrait::NEAR`: dizzy → confused, tripping …), then toward calm; a character without sprites a faceless visor. Sprites: `space/portrait.rs` – a `Source`, today
`LocalDir` (`--portraits`: `<dir>/<char>/<char>_portrait-<mood>.*`, the media/characters layout, or flat), loaded and
mipmapped on their own thread; **#180's deck-token fetch** becomes a second `Source` caching into that folder.

**Feelings** (`space/feel.rs`; owner 2026-10-30: "a few mappings for certain emotions … the main thing is that we get a
change. It has to become more exploratory again" – arbitrary on purpose, the whole mood set). Two lists in `[being]`,
one inline table per line:
- `feel = [{ mood, when, <test>, for, hold, weight, chance, cooldown }]` – feedback → emotion. `when` = `node.port` (the
  node by id, else the first of that kind: `fft.bass`, `fm.index`, `lfo.shape`, an audio output = its peak) · `bpm` ·
  `touch` (1 while a hand turns a button, fading 0.8 s) · `energy` · `phrase` (0..8 through an 8-bar phrase). The test:
  `above` / `below` (both = between), `rising` / `falling` (that far above / below its last half second), `steady = s`
  (moved less than `by`, 0.03, for s seconds). True for `for` s (0), it fires with `chance` (1) and its mood shows at
  once, for at least `hold` s (1.5). The heaviest `weight` (1) wins once the shown one has had 0.6 s; a mood shown 6 s,
  or one that stops, rests `cooldown` s (4). None firing = the energy.
- `react = [{ mood, set, by | to, attack, release }]` – emotion → sound: when the head turns to `mood`, the unlinked
  control input `set` is kicked `by` an amount or `to` a value over `attack` s (0.1) and eases back over `release` s (2)
  through `Controls::set`; a hand on it meanwhile becomes its new rest, kicks on one input add up, its range bounds them.
- The defaults (cathedral.toml, conductor.toml): every mood has a rule and a nudge – touch → smug, letting go →
  embarrassed, an LFO's waveform switched → dizzy, a bass jump → surprised, a level jump → scared, a jump in the highs →
  angry, silence → dying, a fast LFO → tripping, a high FM index → furious, a high FM ratio → confused, the level falling
  → crying, a fast clock → laughing, loud mids → happy, the incense's cutoff low (the conductor: the swell's trough) →
  disgusted, the phrase's last bar → denial, its fourth → bargaining, no bass → sad, a level standing still → thinking,
  the energy standing still → acceptance, a phrase's start → neutral. Back into the sound: furious / angry / sad /
  disgusted / embarrassed paint the hue (red · rust · blue · bile · blush), surprised flashes the glow and blips the
  drone's pitch, laughing doubles the choir (`seq.rate`) and speeds the LFO, thinking drops the FM index, denial ducks
  the music, dying darkens the set and sinks the drone … – several chain (surprised → furious, laughing → tripping,
  smug / disgusted → confused). `--dev` reloads them with the scene.
- The HUD's mood line: `MOOD FURIOUS < VOICE.INDEX +3` (the rule, the nudges at work), `MOOD HAPPY < ENERGY 0.62`, or `<
  STREAM`. The conductor runs its own `feel` rules on every deck's mirrored values (by kind, so they resolve on any
  graph; a deck's nudges stay with the deck), the deck's streamed mood their fallback; `deck.graph` carries the whole
  palette.

**The session** (phase 4) – the space node's `join` with the relay of deck.toml `relay` or `--relay`, else the hosted
`wss://live.scutoons.com/live/CATH` (#199):
- `join = "deck"` (`scenes/cathedral.toml`): joins as `musician` of the group `decks`, sends `deck.graph` (its graph,
  character, size, moods, exposed parameters), then `deck.values` – every value its body shows (the buttons' inputs, the
  hoses' sources; paths `node.port` / `node.shape` → float vectors, `values.rs`) that moved past its epsilon, at most
  30 / s, and its mood when it changes (`stream.rs`, `link.rs`); after a reconnect the graph and every value again. It
  shows only its own being.
- `join = "conductor"` (`scenes/conductor.toml`; `--token`): gathers every deck's being from the snapshot and the stream
  – its graph rebuilt from the wire, the same body code on a `Mirror` of its values, its own head and mood (`decks.rs`)
  – into a circle (`ring`, 3 m) around its own bigger being in the middle of the nave, a deck drifting down into its
  place; presence's `decks` drops the ones that left, its `routes` (who sent values to whom) become hoses sagging between
  the beings' cores, bubbling while values come. Only this stage's own buttons turn.
- The HUD: `CONDUCTOR c6 @ CATH 5 DECKS 5 ROUTES` / `DECK c1 @ CATH`, then `FACE plf 0/5 MOODS` (or why there is none).
- Without the relay: stand-in decks, `cargo run --release -p deck-stage --example decks -- ws://127.0.0.1:7700/live/CATH 4`
  (the cathedral scene as n decks, each another character, values and moods moving by themselves; no window).

`scenes/cathedral.toml` ("Cathedral of decks", a deck: catpope's head) – the set from Cat Pope chapter 2 (`cathedral`,
low), the being ahead of the reader's view in the nave; catpope 2's music, a choir line (seq → FM), a drone (its gain on
the bass), pink-noise incense swayed by an LFO → mixer → FFT (its level kicks the breath) → out. `scenes/conductor.toml`
– PLF's head on the conductor's being (size 0.8) in the middle of the nave, the sermon's music, an organ drone on the
bass, a voice swelling with a slow LFO. An audio oscillator's `wave` can't change while it plays yet (an engine
rebuild); MIDI / OSC learn onto the same `Controls::set` is S6.

`scenes/default.toml` ("AV madness (S3)") – the tempo clock at 96 BPM; catpope 1's music + an FM voice sequenced on
the beat + pink noise + the click (off until K) → mixer → FFT; test pattern (pulsing on the beat) → kaleidoscope → the
chapter's slices (cut on a bass hit snapped to the beat, every 6 s at the latest) → RGB shift → the caption overlay →
screen. `scenes/test-pattern.toml` – the picture alone (no sound, no network). Both land in
`/usr/share/deck/scenes` on the deck; `scene = "…"` in `/var/lib/deck/deck.toml` picks one.

## Sync (#195)

Owner 2026-10-30: "Baked into the image is not what I want … The API should deliver this." The `sync` module
(`src/sync.rs`) brings what a scene needs into the cache on DECKDATA; the ISO ships no sets and no portraits.

- **At a scene's load** (start, `--dev` reload) with a `space` or `outlet` node: one request,
  `GET <api>/library/scene-content?sets=<the space node's set>&characters=<the being's character>` (server/docs/API.md
  "Library") – each set's 3D per quality and each character's portraits per mood, CDN paths with sizes. Fetched: the
  set at the node's `quality` and the portraits of the being's whole palette (`look.moods` + its feelings' moods).
- **Later wants** – a deck joining the conductor with another character, a Live cell's set – are asked for when the
  portraits thread or the set loader wants them (`--example decks`' stand-ins: catpope-biker, yellowmouse …).
- **Only what is missing or changed:** `<cache>/sync/index.json` records the CDN path each file came from (paths are
  immutable – a new path is a new version); same path + size = skipped. A download is checked (the answer's size, the
  glTF / WebP / PNG magic) before it replaces anything – a broken one is refused, the old file stays.
- **Where:** `<cache>/sets/<id>/<id>.<quality>.glb`, `<cache>/portraits/<char>/<char>_portrait-<mood>.<ext>` – read by the
  space node and the heads as before. `--sets` / `--portraits` (or `DECK_SETS` / `DECK_PORTRAITS`) name local folders
  instead; that kind is then not synced.
- **Never on the frame:** one worker thread does the network; the set loader (space::Gate) and the portraits thread
  (`sync::Portraits`, a portrait::Source next to LocalDir) wait on their own threads until their file is settled.
- **Offline:** the request fails → `SYNC OFFLINE - FROM THE CACHE`, everything settles at once, the cache plays; asked
  again after 30 s for new wants and at the next scene load.
- **HUD** (while it fetches, is offline or failed): `SYNC 2/7 3.1 MB` · `SYNC OFFLINE - FROM THE CACHE` · `SYNC ERR …`.
- **Token:** the deck token (deck-link's run directory, read at each request; `DECK_TOKEN` on a dev deck) goes to the
  API as `Bearer` (never to the CDN) once the deck is paired (#180) – the content is public, nothing waits for it.
- **The conductor's push** (owner 2026-10-30: "conductor sync. start yes"): the conductor sends `scene.sync` (LIVE.md
  B.6) – once at its start, again on **S**; `--sync set:cathedral,character:catpope` names what, else the scene's set
  and the characters on stage (its own + every deck's); the push's `scene` is the scene file's name (`conductor`). A deck's link hands the push (a `scene.sync`, or the welcome
  snapshot's when it joins later) to its sync as an extra need: the same request, cache and skipping; a pushed set at
  the space node's `quality`, a pushed character with every portrait the API lists. It answers `deck.synced` (items
  `set:<id>` / `character:<id>` ready / missing + why, files done / total, bytes; progress ≤ 5 / s, then the final
  state) – an item is ready only when its files are in the cache. The deck's HUD: `PUSH 3/7 CATHEDRAL` ·
  `PUSH READY CATHEDRAL` · `PUSH MISSING 1 …` · `PUSH OFFLINE …`. The conductor's HUD: `SYNC 2/3 READY CATHEDRAL`,
  and over each deck's being `SYNC 3/7` · `READY` · `MISSING 1` · `OFFLINE` · `SYNC WAITING`; `Space::decks_ready()` is
  the cue's wait (every deck the push is for is ready).
- Tests: `tests/sync.rs` against a stub API + CDN on 127.0.0.1 (needs → one request, into the cache, cached skipped, a
  changed path fetched again, a broken file refused, offline from the cache, a later character, the space loaders);
  `tests/scene_sync.rs` with the real relay too (a push → every deck fetches → ready at the conductor, a late deck syncs
  from its welcome, a push again skips the cache, missing / offline answers, a deck's own push refused). The stub:
  `tests/common/`.

## Live outlet (#186)

A deck is a screen outlet of a Live show (research/LIVE-LENSES.md, DECISIONS 2026-10-30): the scene's `outlet` node
joins the relay as `output` of this deck's outlet and draws the cells the conductor sends – no browser. Settings:
`relay = "ws://<host>:7700/live/<code>"`, `outlet = "left"`, `locale` in `/var/lib/deck/deck.toml`, or `--relay`,
`--outlet`, `--locale`; with an outlet set and no `--scene` the stage plays `scenes/outlet.toml` (a space node → the
outlet → the screen). No relay set = the hosted `wss://live.scutoons.com/live/CATH` (#199); both clients speak ws:// and
wss:// (`config::dial` – tungstenite with rustls + the webpki roots, no OpenSSL). `src/live/`:

- **client.rs** – tungstenite on its own thread: `hello { role: output, group: <outlet> }`; the outlet's `cell`,
  `media.preload`, `state.slice`, `panic` in; `media.ready` out; a ping every 20 s; a lost connection joins again with
  `resume` (the relay replays what it missed) or gets the snapshot (the outlet's cell and last preload) – shown at once.
- **media.rs** – the preload: each item fetched with the deck token (`DECK_TOKEN`, else deck-link's run directory on a
  deck, #180; not paired = the code as the reason) into `<cache>/live/<hash>-<name>` (immutable URLs: once), then
  `media.ready { ready, missing, why }`; what is missing is tried again after 2, 5, 10, 20, then every 30 s. Sets
  (#193: set + spot + mode, never a file) resolve through the `Resolve` trait – today `LocalSets`: the set's 3D
  (`cache::set_file`, low; with the sync on, where the sync puts it – the space node's gate fetches it),
  `<sets>/<set>/<spot>.<still|video>.<ext>`, else the item's `url`.
- **plan.rs** – a cell's layers → stage nodes: `scroll` / `slice` / `panel` → chapter (a slice by its reading-order
  index, a crop), `image` and a set's still → an image, `video` → video (S4 – a set spot's still stands in), `model3d` /
  a set in 3D → the space node (its set and spot), `text` / `surtitles` → text, `colour` → a solid, `camera` →
  video-in (S4). A missing module or node is a problem with the layer's id – the HUD shows it, the rest plays.
- **compose.rs** – the flat layers painted into one plane at the render size on the composer's thread (fit cover /
  contain, opacity, text in Bangers, the surtitles of the show's slice; a scroll steps every 6 s); the 3D set goes to
  the space node (`Space::show`: another set loads on its thread, the being and the camera move to the spot).
- **render.rs** – the `outlet` pass (`shaders/outlet.wgsl`) uploads each plane and cross-fades it over its `in` by the
  cell's longest `enter` / `exit` (default 0.6 s); the backdrop shows only where the cell has a 3D set. The frame path
  only polls the channels and computes the fade – no allocation (tests/no_alloc.rs).
- **HUD** – under the title: `LIVE <outlet>  OUTPUT c4 OF left @ CODE` (or joining / offline and why), `MEDIA n / m
  READY – <id>: <why>`, `CELL n LAYERS + SPACE`, then `! <layer>: <reason>` for everything the cell can't show.

Try it without the conductor app: `cargo run --release -p relay -- 127.0.0.1:7070`, the stage with `--scene outlet
--relay ws://127.0.0.1:7070/live/DECK --outlet left --windowed`, then the stand-in conductor
`cargo run --release -p deck-stage --example live-cue -- ws://127.0.0.1:7070/live/DECK stage/examples/sermon.json`
(Cat Pope s1c2's sermon, LIVE-LENSES §5: comic → the live cathedral → comic; 60 fps, 0.3 ms CPU per frame on the RTX
2070 desktop). Tests: `tests/live_relay.rs` (the real relay on 127.0.0.1, ports 7000–7099, a local file server: preload
with the token, ready, the cell, resume, the snapshot after a restart). Not yet: video / video-in (S4), audio layers,
`mirror` (the conductor resolves it), cells at their `at`, the API set resolver, signed show-media URLs (#186 "Open").

## How it is built

- **Threads:** the event loop (winit) renders per frame at vsync (Fifo – no busy loop); the render thread (`audio-render`,
  MMCSS / SCHED_FIFO) runs the audio nodes per block into the ring the audio callback (PipeWire's realtime data thread,
  or cpal's) copies from – or, `--audio-thread off`, the callback runs them itself ("Audio – live-safe"); one loader
  thread per `chapter_music` node.
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
  (rtrb), the FFT's samples leave through another. **No zipper** (#188): every control input glides across its
  block – deck-core's `smooth::Ramp`, a one-pole per block spread linearly per sample (gains, frequencies 25 ms, the
  rest 12 ms), the same in the voices, the blocks and the browser's batch. **The soft limiter** (deck-core's
  `limiter.rs`, also live-core's) instead of a hard clamp, last before the device: 64 frames look-ahead (the stamp
  accounts for it), held to −1 dBFS, a tanh bend above 0.95 – never 1; its gain reduction goes to the HUD (`LIM 3`), the device's
  underruns (a block heard later than the last one's end, `Engine::mark`) into the glitch indicator ("Audio –
  live-safe"). `examples/render-wav.rs` renders 10 s of the cathedral with a hand on the knobs into a WAV and prints the
  largest sample step (2026-10-31: 0.142 → 0.060, second difference 0.145 → 0.015). `tests/no_alloc.rs` counts
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
- **Space** (#188; the crate **deck-space**, `deck/space/src/space/` – lifted out in #198 so the browser draws the same
  being, deck/space-web; the scene model `graph.rs` / `params.rs` / `scene.rs` lives there too, deck-stage re-exports it
  and keeps `src/space/link.rs`, the relay link, and `modules::check` before a scene's graph is built): `glb.rs` reads a set's 3D (.glb, tools/rooms' build) – meshes with POSITION, COLOR_0 (the rooms' build
  bakes Unreal's lights into the vertex colours), TEXCOORD_0, unlit base-colour materials, WebP / PNG images
  (`EXT_texture_webp`), quantized attributes (`KHR_mesh_quantization`), **`EXT_meshopt_compression`** (decoded by
  `meshopt.rs` in plain Rust – the vertex codec v0 and the index codec v0 / v1 of the extension's bitstream spec; the
  decoded positions are checked against the accessors' min / max), named empty nodes as spots – flattened at load into
  one world-space vertex / index list with a draw per texture, the images halved to `max_texture` and their mip chains
  made on the loader's thread (the frame that uploads only copies). Not read: the INDICES mode, meshopt filters, Draco,
  KTX2 / Basis, external buffers – a file with those is refused with the extension's name (the rooms' build encodes
  with meshopt's FILTER method, which filters only normals / rotations – the rooms carry none); a plain variant: the
  rooms' build (tools/rooms/pipeline/build.mjs §5) without its `EXTMeshoptCompression`. `camera.rs` the orbit camera (glam; it stands still unless touched – the space node's `auto_turn = true` turns it slowly after 4 s without input; the ray through a
  pixel for picking), `being.rs` the body (layout from the graph, the hoses' Béziers, the bubble pools – 12 per hose –,
  the buttons, picking by ray / sphere), `gpu.rs` + `shaders/space.wgsl` the passes into the node's texture with its
  own Depth32 buffer: the set unlit (texture × vertex light × `light`, fog), the solid spheres (nuclei, buttons,
  sockets) of every being, the heads far to near (a 16 × 16 plate bulged toward the eye, the machine layer then the
  portrait – the old mood under the new –, premultiplied alpha, depth written, a texture per sprite), then the tubes and
  the ghost spheres (core, shells, bubbles, skin) additive and depth-tested only. The buffers are sized from all beings
  (`Space::capacity`) – anew only when a deck comes or goes (`Space::generation`) – and rewritten per frame;
  `Being::update` / `write` never allocate (tests/no_alloc.rs, a deck's being on a `Mirror` too). The values come
  through `values.rs`: this deck's control slots and audio meters (`audio::Meters`: every output buffer's block peak,
  only built for a scene with a space node), or a deck's stream (`Mirror`).
- **Post stack + the glow's light** (#204; deck-space `space/post.rs` – the data – and `space/post_gpu.rs`, the WGSL in
  `deck/space/src/shaders/post/`, so the browser runs the same passes): the space node's `post` list picks and orders
  hard-coded fragment passes after the space is drawn – **bloom** (soft-knee threshold into half size, the dual filter
  down to a quarter and an eighth and back up, each level added onto the one above, laid over the picture as a screen
  blend: six small draws), **curves** (lift / gamma / gain, one draw), **pixelate** (blocks of `pixel` render pixels by
  `textureLoad`, optionally `levels` per channel or the nearest `palette` colour, one draw), **fxaa** (the console
  variant, nine taps, one draw). Their knobs are the space node's control inputs (links, the feelings' `react`, the
  conductor's sends), no buttons on the core unless `expose` names them (`"space.bloom"`). Two Rgba8 textures at the
  render size take turns, the last pass writes the node's target (no copy); bloom adds three at ½, ¼, ⅛. A pass not
  listed has no pipeline and no texture, a listed one at neutral (bloom 0, curves 0 / 1 / 1, pixel 1 without levels or
  palette) is skipped that frame (`Chain::plan`), and with nothing to run the space draws straight into its target.
  `--dev` watches the passes' files (and their prelude `common.wgsl`) in the checkout: a changed one recompiles only
  that pass, one that doesn't compile keeps the running pipeline (the error in the HUD). **The glow lights the room:**
  every being's core is a point light in the room shader – its hue, `glow` + its level `pulse` × `glow_light`,
  inverse-square (+1) windowed to zero at `glow_range`, on the face's normal from the world position's screen
  derivatives (the rooms carry no normals) – added to the baked light; at most 8, the strongest at the eye first
  (`Space::write_lights`). The plan and the lights never allocate (tests/no_alloc.rs).
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
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-30, #188) | cathedral (null audio) – set low / medium / high | 60.4 / 60.4 / 60.3 fps (vsync), CPU per frame 0.37 / 0.39 / 0.40 ms (the default scene: 0.31), first frame 719 / 802 / 827 ms; the set loads on its thread in 174 / 449 / 1574 ms (114,788 / 116,163 / 129,205 triangles, 4 WebP + their mips), its upload 2 / 4 / 9 ms; p99 ~50 ms in every scene = the start |
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-30, #188 phases 4–5) | cathedral with its head · conductor + 5 stand-in decks (`--example decks`, same PC) | 60.8 fps, CPU per frame 0.42 ms (0.37 without the head) · 60.2 fps, 1.12 ms (0.39 alone) |
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-31, #188 tethered buttons, labels) | cathedral · conductor alone · conductor + 5 stand-in decks (30 s each, null audio) | 60.1 fps each, CPU per frame 0.51 · 0.49 · 1.54 ms (before the tethers: 0.42 · 0.39 · 1.12) |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | cathedral, conductor | open – the owner's stick |
| Windows 10, Ryzen 5 3600X, `cargo bench -p deck-stage` (2026-10-30, #183) | the graph, µs per 128-frame block: before #183 → patcher → batch → batch AVX2 | default scene 11.0 → 9.9 → 11.1 → 10.0 (whole v3 binary 7.5) · 1 FM voice – → 2.2 → 3.5 → 2.2 · 16 FM voices 37.8 → 34.1 → 10.2 → 7.7 · 16 sines 20.1 → 15.4 → 7.3 → 5.3 · 64 FM voices – → 125 → 41 → 27 · control step, 300 LFOs: 58.4 → 2.1 (3 levels × 100), 69.6 → 5.5 (300 levels) |
