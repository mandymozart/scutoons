# deck/native – the native AV app the deck boots into

`deck-native` (agents/research/CYBERDECKS.md §8, #179): one window, a node graph from a TOML scene – video passes on wgpu
(Vulkan on the deck), an audio graph on PipeWire (cpal elsewhere), control values per 128-frame audio block – and chapter content from
scutoons.com's library API with a disk cache. On the deck `deck-session` runs it under `cage` instead of Chromium
(deck/os/README.md "What happens at boot"). Built so far: S1 (window, wgpu, graph core, test pattern, shader passes,
HUD), S2 (audio graph, synths, FFT, chapter music) and S3 (the chapter's slices with their dialog, the caption overlay,
the splash), the 3D space of #188 (the Cathedral of decks: a set's glTF, the deck's being with its portrait head and
3D buttons, the decks around the conductor over the relay) and the Live outlet of #186 (the relay's cells for this
deck's outlet, natively – below) and the sync of #195 (the scene's sets and portraits from the API into the cache –
below). Open: S4–S6 (#179), #188's body parts and the browser scene.

## Run

```bash
cd deck && cargo run --release -p deck-native -- --windowed
```

`--scene FILE` (default: the system scene `scenes/idle.toml` – fireflies on black, no sound, no network (#252); the system scenes
`idle`, `silent`, `outlet`, `test-pattern` by name; a sandbox of `scenes/sandbox/` (`madness`, `cathedral`, `conductor` …) by name only
in dev mode, #427; `--scene rig:catpope[@mood]` = a core rig – `rig:item.incense_burner` an
item's –, played by deck-space's `Deck` as on every deck (#322, src/rig.rs: flattened, in the staging – its character's home set, none yet: no set, #427 –, replayed when
the being's mood turns; no such rig = the silent scene: the being, no sound, #425)) · `--sets DIR` (a local folder of sets instead of the synced
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
The graph's workloads: `cargo bench -p deck-native` (patcher | batch | batch AVX2, µs per block).

Keys: **F1** the HUD → the collapsed stats bar (#209: one line across the whole top edge – FPS, CPU / frame, the audio's
glitches / DSP load / limiter, the relay, the GPU backend; red when out of the green) → nothing · **F12** the conductor page (a locked-down browser window; #268) · **Ctrl+Q** quit (on the deck the stage starts again; a stop signal – SIGTERM, Ctrl+C – ends the same way, outputs dark, #263) · **Ctrl+Alt+Del** power off (`systemctl
poweroff`, the logo stays up; not in a `--windowed` stage) · Esc quits a `--windowed` stage · the tempo clock: **T** tap ·
**+ / −** BPM · **Space** start / stop · **F3** the next source (internal → MIDI → Link → OSC → relay) · **K** the click · **F4** the
quality settings page, **Shift+F4** the governor on / off ("Quality" below) · **F5** the scene list ("Scenes" below). **Volume up / down / mute** (the hardware keys, `volume.rs`, #253): the PipeWire default sink's volume in 5 % steps up to 100 % (`wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+ -l 1.0`, `set-mute … toggle`) – so Chromium (F2) follows too – and `VOL ######.... 60%` / `VOL MUTED 60%` for about a second on the HUD, also with the HUD off (the bar appears for that second); WirePlumber keeps the level across reboots. `--show-volume` keeps the overlay on (a check run's screenshot). In a scene with a `space` node: a **mouse drag** turns the camera around the being (or, on a 3D
button, pulls it along its tether – out = more; a double click: back to the scene's value; a finger the same), the **wheel** / **↑ ↓** zoom, **← →** turn, **R** the camera's home; the pointer stays
visible in the kiosk then. The HUD names the keys. F12 with `DECK_BROWSER` set (deck-session sets it to `deck-session browser`) starts that command on
top of the running stage: the logo comes up until the browser's window takes the focus (cage shows the newest window),
then the stage stops drawing and mutes; when the command exits the stage is back at once. Without `DECK_BROWSER` F12
quits with exit code 10. If the display connection dies (cage went away) the stage logs "display lost" and exits with
code **75** at once; `deck-session` ends on it with a non-zero exit, so deck-kiosk (Restart=on-failure) brings the
kiosk back – no browser on a dead display (#257).

**Gamepad** (#228: the feature `gamepad`, `cargo run --release -p deck-native --features gamepad -- --windowed`; gilrs – a
pad system that can't start is logged and the stage plays without one): the left stick drives the left stick of the
selection, the right stick the right one – the same pairs as a tap (the being: voice ratio / index on the left, cutoff /
LFO rate on the right; a knob: its pair) – and a stick is down while it is deflected past the dead zone, its values
moving from where they were. **A** (south) selects the next – the master section → the being → each knob → … – **Y**
(north) the previous; the space rings what is selected, as for a tap. Logic: `src/pad.rs` over deck-space's `controls`; the layout is gilrs's standard one (its stick up =
positive – not yet checked on a pad).

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
| `events` | the `walk` and `events` kinds – generators that talk in events, their sequencer and dispatcher threads (#236, research/EVENTS.md) | deck-events |
| `sync` | the scene's sets and portraits from the API into the cache (#195 – below); no node kind | ureq, serde_json |
| `chapter` | `chapter`, `chapter_music` (the library API + cache) | ureq, symphonia, ab_glyph, image/webp |
| `live` | `outlet` (a Live outlet, #186 – below) | `chapter`, protocol, tungstenite |
| `midi` | the tempo source `midi`, `midi_out` (MIDI clock in / out) | midir |
| `outputs` | the scene's `[outputs]` (#211): parameters as outputs – MIDI out (a port, or the conductor as `deck.midi`), DMX fixtures through a USB-DMX widget, the clock-synced strobe; no node kind (below) | deck-outputs, adapters-dmx-usbpro, adapters-midi |
| `osc` | the tempo source `osc` (`/tempo/*`) | rosc |
| `link` | the tempo source `link` (Ableton Link) | rusty_link (cmake, libclang) |
| `pipewire` | the PipeWire backend (Linux) | pipewire (clang) |
| `gamepad` | a gamepad on the space's sticks (#228 – below) | gilrs (libudev at build time on Linux; the deck image build carries it) |

Default = `space`, `sync`, `chapter`, `live`, `midi`, `osc`, `outputs`, `events` – a plain build plays every bundled scene. The table lives in deck-core
(`core/src/stage.rs`, data only); `src/modules.rs` says which modules this build carries. A scene that needs a module
the build lacks is refused at load with the feature's name (`this build of the stage does not carry the module 'space'
(node space) – build deck-native with --features space`): the stage plays the test pattern and shows that in the HUD –
it never exits for it (deck-session would fall back to the browser); a `--frames` check run exits 2. F3 onto a source
that isn't built says `NOT IN THIS BUILD`. **Scene-driven builds:** `cargo run -p deck-scene-features -- scenes/*.toml`
prints the features a set of scenes needs (`chapter,live,space` for the bundled ones; the core alone prints nothing), so a
deck build is `cargo build --release -p deck-native --no-default-features --features "pipewire,$(deck-scene-features …)"`
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
  copies. The ring holds the device's buffer + the **lead**: one block (5 ms) at first, a block more per **underrun**
  up to 16 (85 ms), a block less per calm minute – a late or long callback is the device's own thread stalling, which
  no lead can help (at ASIO 64 they pushed the lead to its cap for nothing, #230). The time base stays exact (the callback's (frames taken, heard at) through
  a seqlock; each block is stamped with it + the frames between). `tests/no_alloc.rs` runs both halves: no allocation.
- **Priority** (`src/audio/priority.rs`): the render thread – or the callback's thread when it renders (WASAPI, ALSA;
  not ASIO's, that is the driver's) – at Windows MMCSS "Pro Audio" (high), Linux SCHED_FIFO 70 (the deck's user is in
  `realtime`; PipeWire's own threads at 88 through rtkit).
- **Events** (#236, `src/audio/sequencer.rs`, `dispatch.rs`; research/EVENTS.md §16): no generator code on the
  render thread. The `walk` / `events` hosts' patches run on the **sequencer** thread (`events-seq`) in windows of 256
  frames, ahead of the render by the device's buffer + the lead + a block; each host's notes go into its own SPSC ring
  (rtrb) at absolute frames (the engine's frame counter – one timebase), and the host's block only takes the events of
  the frames it renders and writes the cv. A window the render already passed is dropped (counted late); a full ring
  makes the sequencer wait. The **dispatcher** thread (`events-dispatch`) owns the routing table (deck-events
  route.rs): adapters push into a bounded MPSC queue (`Inlet::send`, never blocks), it fans each event out by name or
  domain to the patches' `listen = [{ to = "arp.notes", name = "music.note" }]` inputs (one SPSC ring each); a patch
  change swaps the immutable table through the same queue – no lock on any hot path. Both threads at high, not audio,
  priority (Windows THREAD_PRIORITY_HIGHEST, Linux nice −10). Drops (capped, hops, refused compound events, late): the
  HUD's `EVENTS IN n DROP m`, a `--frames` run's `events_dropped=`. `DECK_EVENTS=block` runs the patches inside the
  block instead (the fallback; `Engine::build_with` in tests) – the same samples (tests/walk.rs), both allocation-free
  (tests/no_alloc.rs). Measured: send → receiver's ring ≈ 1.4 µs median, 41 µs p99 (dispatch.rs test).
- **The buffer** (`--audio-buffer N` · deck.toml `audio_buffer` · the scene's `[stage] audio_buffer`, in that order):
  cpal's `BufferSize::Fixed(N)` clamped to the device's range (a log line says so), a refused one falls back to the
  default; PipeWire's latency request (default 128). **WASAPI shared can only grow it**: on the RME the engine runs a
  1056-frame (22 ms) buffer whatever is asked below that – a smaller buffer needs ASIO.
- **Which channels an outlet plays on** (#428): a scene names outlet groups (`out` nodes, `group = "cue"`), never hardware; the
  machine's deck.toml binds them, counted from 1, after its plain keys: `[[audio_outlet]]` / `group = "cue"` / `channels = [3, 4]`.
  `main` without an entry plays on 1.., any other group without one is silent. The log prints each outlet's device channels
  ("audio: outlet 'cue' (cue): device channels [3, 4]"). Who assigns this at a venue: #330.
- **Host and device**: `--audio-host default|wasapi|asio` (deck.toml `audio_host`), `--audio-device NAME` (a part of
  the name, any case; deck.toml `audio_device`), `--audio-list` prints the hosts, devices, formats and buffer ranges.
  On PipeWire the name picks the **sink** (`target.object`, #285): the first `Audio/Sink` node whose name or description
  holds it – the X1's RME in the Pro Audio profile is `…pro-output-0`, so `audio_device = "Fireface"` finds it; no match
  logs the sinks there are and the default sink plays.
  The stream opens with the sink's channels (#318: `audio.position`, else `audio.channels` as AUX; at most 32) – the RME's
  Pro Audio sink is 20 channels, AUX0…AUX19, analog 1/2 first; a sink that says nothing is stereo. In class-compliant mode
  the RME plays the mixer preset stored in the unit: routing is fixed in TotalMix on Windows (driver mode), then back to CC
  mode with the USB cable out (owner 2026-11-03): **Options → Reset Mix → Straight Playback** (playback n → output n:
  1/2 Main, 3–6 Analog, 7/8 Phones, 9/10 SPDIF, 11/12 AES, 13–20 ADAT), stored as the snapshot `SCUTOONSDECKX1`. Check scenes: `scenes/channel-walk`, `surround-51`, `cue-34`.
  A dev machine's deck.toml: the user variable `DECK_TOML` names it (the PC: `%LOCALAPPDATA%\scutoons\deck.toml` with
  `audio_host = "asio"`, `audio_device = "Fireface"`, `audio_buffer = 128` – owner 2026-11-02, #230).
  **WASAPI exclusive** is not possible: cpal (0.15 and 0.16) opens WASAPI in shared mode only.
- **ASIO** (the RME Fireface's reference driver on Windows) is the cargo feature `asio` (cpal's asio host; output in
  f32, i16 or i32 – RME's driver talks i32). Its `asio-sys` builds bindings to Steinberg's ASIO SDK at build time:
  1. get the ASIO SDK from steinberg.net (a free download after accepting Steinberg's licence) and unpack it, e.g. to
     `C:\SDK\asiosdk` (the folder holding `common\`, `host\`);
  2. LLVM for bindgen (`winget install LLVM.LLVM`; `LIBCLANG_PATH=C:\Program Files\LLVM\bin`) and the MSVC build tools;
  3. `set CPAL_ASIO_DIR=C:\SDK\asiosdk` (without it, or with a folder that doesn't exist, asio-sys downloads the SDK
     itself into %TEMP% – set it), then `cargo build --release -p deck-native --features asio`;
  4. run `deck --audio cpal --audio-host asio --audio-device fireface --audio-buffer 128` (the RME's driver is
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
  | **ASIO** 128 → 128 (2.7 ms), 10 min | render thread, lead 256 (5 ms) | 0 | – |
  | ASIO 128, 2 min | in ASIO's callback (`--audio-thread off`) | 0 | – |
  | ASIO 64 asked → 128 (RME's Settings at 128) | render thread | 0 | – |
  | ASIO 64 → 64 (1.3 ms), 10 min, Live + a browser stage open, memory under pressure | render thread, the lead ran to its cap (85 ms) | 40 (long 39, late 1, underrun 0) | – |
  | ASIO 64, 10 min, the lead on underruns only (the fix) | render thread, lead stays 256 | 3 (long 3) | – |

  Every glitch was the same: cpal's WASAPI thread (its own TIME_CRITICAL priority) woke 32–38 ms after the last
  callback and found the 22 ms buffer drained – the ring can't help a late device thread, MMCSS for that thread does;
  so the callback's thread is raised in both modes. The DSP load stays at ~3 % of a block, so the render itself never
  came near its budget. What this can't see: a glitch inside Windows' audio engine (audiodg) or the USB driver (DPC
  latency) after our buffer – crackles heard while the indicator stays at 0 point there (LatencyMon; ASIO bypasses
  audiodg).
  **ASIO** (2026-11-01, #230, the same PC, `ASIO Fireface USB` 48 kHz i32 20 ch): the HUD `AUDIO ASIO 48K BUF 128 3MS
  LEAD 5MS`, OUT 31 ms; the lead stays at one block. The driver offers exactly the buffer set in RME's Settings dialog
  (`--audio-list`: `buffer 128–128`); another `--audio-buffer` is clamped to it with a WARN line – a smaller buffer is
  set in RME's dialog first. The walk scene's sequencer runs at THREAD_PRIORITY_HIGHEST, `events_dropped=0`. At 64
  (1.3 ms periods) a "long" glitch is the callback's copy taking ~2 ms: the device's thread preempted, not compute (DSP
  3 %), the ring never dry – Windows' scheduling jitter against a 1.3 ms period; 128 is the safe setting under load.
  **Linux, the X1** (2026-11-02, #285: deck/os r2548, kernel 7.2 PREEMPT_RT, PipeWire 1.6 at 48 kHz, the RME Fireface
  UCX II in class-compliant mode on the Pro Audio profile – deck/os/README.md –, the cathedral at volume 0, release
  build; the render thread at SCHED_FIFO 70, PipeWire's data thread in the stage at 83 through rtkit, the sequencer and
  dispatcher at nice −10):

  | setting | run | glitches | HUD |
  |---|---|---|---|
  | `--audio pipewire --audio-buffer 128`, 10 min | render thread, lead 256 | **0** | `AUDIO PIPEWIRE 48K BUF 128 3MS LEAD 5MS`, OUT 31 ms, DSP 6 % |
  | pipewire 128, 2 min, audible (volume 0.2) | render thread | 0 | – |
  | pipewire 64 (1.3 ms), 10 min | render thread, the lead stays 256 | 4 (late 4, underrun 0) | `BUF 64 1MS LEAD 5MS`, OUT 29 ms, DSP 2 % |
  | pipewire 64, 2 min, audible | render thread | 6 (late 6, underrun 0) | – |
  | the walk scene (tempo + walk + fm), pipewire 128, 2 min | sequencer thread | 0, `events_dropped=0` | `EVENTS IN 0 DROP 0`, DSP 2 % |
  | `--audio cpal --audio-device II24322536` 128, 2 min | cpal ALSA `sysdefault:CARD=…` | 0 | `AUDIO ALSA 44K BUF 4704 107MS`, OUT 131 ms, DSP 4 % |

  At 64 every glitch was a **late** callback – PipeWire's cycle came 4.5–9.4 ms after the last one instead of 1.3 ms,
  in bursts of three within 10 ms (while a cable was plugged in, once at the start) – with the ring never dry (underrun
  0) and the callback itself at 0.00 ms: the USB / graph scheduling at 1.3 ms periods, not compute (DSP 2 %). 128 ran 10
  min + 2 min audible without one. **cpal on ALSA is no live path on the deck:** cpal enumerates only the card's
  `sysdefault` (the plug + dmix layer: 44.1 kHz, a 170-frame minimum, `Fixed` refused → ALSA's default 4704 = 107 ms,
  the callback thread at 35 % CPU for the conversion), never `hw:`; PipeWire on the Pro Audio profile is the way.

## Scenes

A scene is `[stage]` (`name`, `render_scale` 0.25–1 – times the quality rung's scale) + `[[node]]` (`id`, `kind`, the kind's params) + `[[link]]`
(`from = "node.port"`, `to = "node.port"`; a bare id = its port `out`; control links map `value * scale + offset`).
An input takes one link; an output feeds any number. Every control input can also be set as a param of the same name.

| Kind | Inputs | Outputs | Params |
|---|---|---|---|
| `test_pattern` | speed, zoom, hue, pulse (control) | out (video) | – |
| `shader` | in (video), a, b, c, d (control) | out (video) | `effect` = `kaleido` · `rgb_shift` · `invert` · `pass` · `fireflies` (draws on its own – a brightness, b drift speed, c size, d how many; the idle scene, #252), or `wgsl` = a file defining `fn effect(uv: vec2<f32>) -> vec4<f32>` (bindings: shaders/common.wgsl) |
| `chapter` | in (video), next, hold, zoom, pulse (control) | out (video), text (video) | `toon`, `chapter`, `quality` = low · medium · high, `dialog` (true: set the dialog in), `start` (slice index), `threshold` (0.6), `min_hold` (1.5 s), `quantize` = off · beat · bar (a due cut waits for the tempo clock's next beat / bar); `hold` default 6 s |
| `overlay` | in, top (video), opacity, pulse, shake (control) | out (video) | – |
| `screen` | in (video) | – | the window (one per scene) |
| `lfo` | rate (Hz), depth | out (control, 0–depth) | `shape` = sine · saw · square, `phase` (0–1, where it starts) |
| `tempo` | – | bpm, beat, bar (counts), phase, bar_phase (0–1), pulse (1 on the beat, gone by its middle) | `bpm` (120), `beats` per bar (4), `source` = internal · midi · link · osc · relay, `click` (false), `midi_in` / `midi_out` (a part of a port name; out = MIDI clock out), `midi_out_lead` (ms), `osc_port` (9001), `lead` (false: this deck sends the show clock's anchor; MIDI clock in leads anyway, #321) – one per scene |
| `click` | gain | out | `freq` (1000 Hz), `accent` (1500 Hz, the bar's first beat), `decay` (0.03 s) – ticks on the tempo clock while the click is on |
| `osc` | cv (audio), freq, gain | out (audio) | `wave` = sine · saw · square · triangle, `attack`, `release` |
| `noise` | cutoff, gain | out | `color` = white · pink, `q` |
| `fm` | cv, freq, ratio, index, gain | out | `attack`, `release` |
| `seq` | rate | cv (audio: pitch Hz left, gate 0 / 1 right) | `steps` = MIDI notes (0 = rest), `division` (steps per beat), `gate`, `transpose`; on the tempo clock (step k from beat k / division, `rate` scales the division) unless it has a `bpm` of its own |
| `walk` | spread, rate, glide | cv (audio: pitch Hz left, gate 0 / 1 right – seq's) | module `events` (#236, deck-events): a seeded random walk on a continuous pitch scale on the tempo clock – `seed` (the same seed, the same melody on every deck), `base` (Hz, 147), `octaves` (2), `length` (of a step, 1 = legato); `spread` 0 = the base held, 1 = ±`octaves`; `rate` = steps a beat (2); `glide` rides along |
| `world` | – | events (event) | module `events` (#430): a source from outside as a node, cabled like a module's event output (`[[link]]`, a rig's `route`). `source` = `midi` · `generator`; `listen` = the names it lets through (`["midi.note"]`, a generator's `["music.note"]`); a generator's `patch` = a deck-events patch run by the node, its notes going down its cables only. An event input with no cable hears nothing |
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
Bold (deck-outlet's `text.rs` and `assets/`, SIL OFL); bubble tails, wobbly shapes and gradients are not drawn. `text` = the slice's caption: its
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
`Controls::set` / the engine's `Control::set_base` (once deck-core's `BatchGraph`) / live-core's wire mode `base`) and the modulation adds on top –
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
buttons: `"node"` or `"node.port"`, empty = all; the list's order is the knobs' order on a node – the first named hangs nearest its point; a counted group `"seq.step1..16 <= seq.steps"` (#424) shows the first N of those ports, N = the value of `seq.steps` now (rounded, clamped 0..16), live – a hidden knob is not drawn, not picked, has no tether and keeps its value; the shown ones keep their place as N changes), `character` + `moods` (the head), `feel` + `react` (its feelings,
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
- Without the relay: stand-in decks, `cargo run --release -p deck-native --example decks -- ws://127.0.0.1:7700/live/CATH 4`
  (the cathedral scene as n decks, each another character, values and moods moving by themselves; no window).

`scenes/cathedral.toml` ("Cathedral of decks", a deck: catpope's head) – the set from Cat Pope chapter 2 (`cathedral`,
low), the being ahead of the reader's view in the nave; catpope 2's music, a choir line (seq → FM), a drone (its gain on
the bass), pink-noise incense swayed by an LFO → mixer → FFT (its level kicks the breath) → out. `scenes/conductor.toml`
– PLF's head on the conductor's being (size 0.8) in the middle of the nave, the sermon's music, an organ drone on the
bass, a voice swelling with a slow LFO. An audio oscillator's `wave` can't change while it plays yet (an engine
rebuild); MIDI / OSC learn onto the same `Controls::set` is S6.

`scenes/catpope-analog.toml` ("Catpope plays analog", #418, `--scene catpope-analog`, in the F5 list) – the being in the
cathedral's room with the `stepseq` module playing the `analog` module (its `lead` preset; the event host routes the
notes): the being's `expose` makes the loop length `steps` (the first knob), the step sliders, the on/off switches (each a plain
0..1 button: the module reads ≥ 0.5 as on – the knob widget has no two-state look yet) – both only as many as the sequence
has steps (`seq.step1..16 <= seq.steps`, #424: turn `steps` and the knobs come and go) –, rate, gate and eight synth controls
(volume, cutoff, resonance, filter envelope, amp decay / release, oscillator 2 detune, LFO rate) its 3D buttons.
**Hover name** (owner 2026-11-08, "for now": the one exception to the 2026-10-31 no-numbers-no-labels rule): the knob under the
pointer (else the selected one) wears its name beside it – a rig's `expose` label in the locale, else the parameter's name
(`step3`, `f1_cutoff`); never a number. Drawn by the space's own label pass (deck-space, so native ≡ browser).
Modules play only on a deck built with `--features modules` and their wasm in `DECK_MODULES`; from `deck/`:
`cargo build --release --target wasm32-unknown-unknown -p analog-module -p stepseq-module --target-dir target/wasm-modules`,
PowerShell `$env:DECK_MODULES = "$PWD\target\wasm-modules\wasm32-unknown-unknown\release"`, then
`cargo run -p deck-native --features modules --release -- --scene catpope-analog --windowed` (the folder's other `*.wasm`
files load too and are ignored unless a scene names them). `rig:` scenes stage the being too now: `Player::scene` adds the
`screen` node the flattened rig lacks (it showed only the HUD on black).

`scenes/module-walk.toml` (#406, `--scene module-walk`, in the F5 list) is the smallest module demo: `module:walk` (a seeded melody on the tempo
clock, 96 BPM) → its `cv` (pitch Hz left, gate right) → an `fm` voice → the device, quiet by default (voice gain 0.2, device volume 0.5), a pulsing
test pattern as the picture. The X1 update ships the wasm of `walk`, `stepseq` and `analog` to `/usr/share/deck/modules` (`DECK_MODULES`; `deck/os/modules.sh`)
and builds the stage with `--features modules`. Test: `cargo test -p deck-native --features modules --test modules`.

`scenes/madness.toml` ("AV madness (S3)", `--scene madness`) – the tempo clock at 96 BPM; catpope 1's music + an FM voice sequenced on
the beat + pink noise + the click (off until K) → mixer → FFT; test pattern (pulsing on the beat) → kaleidoscope → the
chapter's slices (cut on a bass hit snapped to the beat, every 6 s at the latest) → RGB shift → the caption overlay →
screen. `scenes/test-pattern.toml` – the picture alone (no sound, no network). Both land in
`/usr/share/deck/scenes` on the deck; `scene = "…"` in `/var/lib/deck/deck.toml` picks one.
`scenes/module-walk.toml` and `scenes/module-analog.toml` (#406) are the module demos – the first scenes whose sound comes from
compiled rig modules (`--features modules`, the wasm in `DECK_MODULES`; the X1 update ships `walk` and `analog` to
`/usr/share/deck/modules`): **module-walk** = `module:walk` (a seeded melody on the tempo clock) → its `cv` (pitch Hz left, gate right) →
an `fm` voice → the device, 96 BPM; **module-analog** = `module:walk`'s note events play `module:analog` over an event cable (#430: `walk.notes` → `synth.notes`), 84 BPM. Both quiet by default (voice gain 0.2, device volume 0.5), a pulsing test pattern as the
picture. Built in like `madness`: `--scene module-walk` or F5; a build without the feature, or with the module missing, refuses them with the reason.
Test: `cargo test -p deck-native --features modules --test modules` (`the_module_walk_scene_plays_the_walk_module`, `the_module_analog_scene_plays_walk_into_analog`).

**The scene switcher (#250, `switcher.rs`):** **F5** lists the system scenes (`idle`, `silent`, `outlet`, `test-pattern`) and, in dev
mode only, under "SANDBOX (DEV)" every `*.toml` of the sandbox folder (#427: `--scenes DIR`, else the dev image's
`/usr/share/deck/scenes/sandbox`, else with `--dev` the repo's `scenes/sandbox`; a file named like a system scene is skipped; a
release image has none); ↑ ↓ move, **Enter** loads the scene in place – the same
path as a `--dev` reload, no process restart: the old show stops (audio graph dropped), the new one is built, the window
stays (about 0.6 s on a PC) – **Esc** / F5 closes; the playing one is starred. A scene that doesn't build (a module this build lacks,
a broken file) keeps the old one playing and the list says why. Enter only loads; **S** makes the scene under the cursor the
start scene (#427): one `scene = "<name or path>"` line in deck.toml (`DECK_TOML` names another file; replaced like `deck-set`
does, never a second line), so the next boot plays it – unset, the deck starts with `idle`. **Scene transitions (#264):** a switch is a transition, a stop is a stop. The incoming scene's `[stage]` says how the lamps (DMX / MIDI outputs) hand over: `transition = "fade"` (default) – the lamps hold the last look, then cross over to the new scene's values over `transition_secs` (default 0.4, the picture's fade-in); channels the new scene doesn't drive fade to 0 – a scene without outputs fades them out the same way (`idle` → `pulse-thunderwash` → `idle` never goes dark in between); `"cut"` – the new values at once, unused channels to 0; `"blackout"` – the old outputs end dark, the new start anew (a reset). Needs the same DMX widget / MIDI port in both scenes (the runner and the open port stay); a different device = blackout. The stop paths below stay dark. The change goes through black, no logo: the picture and the PipeWire stream fade out (0.25 s), the new show loads behind the black, then the picture fades in (0.4 s; `switcher.rs` `Transition`, the splash shader's veil mode). Check runs: `--switcher` starts with the list open, `--switch NAME` loads that scene after 30 frames.

**Quality** (#220, deck-space `quality.rs` – the browser deck runs the same; the stage's `src/quality.rs`): four
**presets** – low · medium · high · ultra – each with its knobs: on / off (an off preset is skipped and can't be chosen),
two render sizes (`min` = the step under load, `max` = its own; a scale of the display ≤ 4, or a height in pixels ≥ 100 –
a 4K screen can render the 3D at 1080 to save power), FXAA, bloom, the lights (0–8) and a frame rate cap (`fps` – 30 draws
half the frames; an early frame waits, the loop sleeps). Defaults: 0.5 / 0.6 (low, no FXAA / bloom, 2 lights) · 0.7 / 0.8
(medium, 4) · 0.9 / 1.0 (high, 8) · 1.25 (ultra, supersampled), 60 fps; curves and pixelate always run (the scene's look).
The enabled presets make the governor's **ladder**. The **preset file** (`/var/lib/deck/quality.toml` on a deck,
`--quality-file`, `DECK_QUALITY_FILE`; TOML, only what changes is needed – the defaults fill the rest) also holds the
**level** (the ceiling; missing = by the GPU: software low, integrated – the X1's UHD 620 – high, discrete ultra; the
browser: WebGL2 medium, WebGPU high), `auto` and the **display** mode (`"3840x2160"` – exclusive fullscreen at that size;
missing or one the monitor lacks = the desktop's; under cage, Wayland, winit has no exclusive mode); `--quality
low|medium|high|ultra[,fixed]` (`DECK_QUALITY`) wins over it. The stage reloads the file when it changes (every 2 s).
**F4** opens the settings page in the HUD: the level, Auto, the display mode (its modes from the monitor, the largest as
the hardware's MAX) and the presets' table – arrows move, + / − change, Enter / Space flip, **P** turns a size into a
height in pixels and back, **R** resets the presets; each change applies at once and is saved. **Shift+F4** the
governor. The **governor** (on unless `,fixed` / `auto = false`) works like the audio lead and the relay's link rate: the
drawn frames' interval in 0.5 s windows; over budget (the cap's interval × 1.2 – 20 ms at 60 fps – or an audio glitch) for
1 s steps a rung down (one window twice over), at most one step a second; 8 s calm (≤ × 1.08) probes a rung up, and a
probe that runs over within 6 s steps back and doubles that rung's wait (up to 2 min) – no flicker. Stalls over 250 ms and
the first 2 s are no load. A step only resizes the offscreen targets; the HUD stays native. The HUD bar shows `Q <level>
<scale>` (red below the level chosen), a deck sends `deck.quality` to the session (at once on a change, else ≤ every 2 s)
and the conductor shows each deck's quality and link under its being (`HIGH 0.90 17MS` / `LINK 3MS 30HZ`, amber while
held lower, under 50 fps or paced, red under 25 fps). `--settings` starts with the page open (a check run's screenshot).
The browser deck's sheet (/live/deck → Settings) edits the same presets (+ its pixel ratio cap) and copies them as this
file's text. Not yet scaled: the set's 3D level, shadows, cloth, the beings' detail, bloom's resolution.

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
`--outlet`, `--locale`; `name = "left"` there names the deck for other decks' far world nodes (`/deck:left/…`, #442,
deck-space far.rs – a deck without a name answers none), `join = "deck"` makes a rig scene (`--scene rig:…`, whose space
node has no `join` of its own) join the relay (`run-far-demo.ps1`: two decks on this PC over a local relay); with an outlet set and no `--scene` the stage plays `scenes/outlet.toml` (a space node → the
outlet → the screen). No relay set = the hosted `wss://live.scutoons.com/live/CATH` (#199); both clients speak ws:// and
wss:// (`config::dial` – tungstenite with rustls + the webpki roots, no OpenSSL). `src/live/`:

- **client.rs** – tungstenite on its own thread: `hello { role: output, group: <outlet> }`; the outlet's `cell`,
  `media.preload`, `state.slice`, `panic` in; `media.ready` out; a ping every 20 s; a lost connection joins again with
  `resume` (the relay replays what it missed) or gets the snapshot (the outlet's cell and last preload) – shown at once.
- **media.rs** – the preload: each item fetched with the deck token (`DECK_TOKEN`, else deck-link's run directory on a
  deck, #180; not paired = the code as the reason) into `<cache>/live/<hash>-<name>` (immutable URLs: once), then
  `media.ready { ready, missing, why }`; what is missing is tried again after 2, 5, 10, 20, then every 30 s (deck-outlet's `preload.rs`, #391: the schedule and the HUD's line; here the fetching). Sets
  (#193: set + spot + mode, never a file) resolve through the `Resolve` trait – today `LocalSets`: the set's 3D
  (`cache::set_file`, low; with the sync on, where the sync puts it – the space node's gate fetches it),
  `<sets>/<set>/<spot>.<still|video>.<ext>`, else the item's `url`.
- **plan.rs** (deck-outlet, #378) – a cell's layers → stage nodes: `scroll` / `slice` / `panel` → chapter (a slice by its reading-order
  index, a crop), `image` and a set's still → an image, `video` → video (S4 – a set spot's still stands in), `model3d` /
  a set in 3D → the space node (its set and spot), `text` / `surtitles` → text, `colour` → a solid, `camera` →
  video-in (S4). A missing module or node is a problem with the layer's id – the HUD shows it, the rest plays.
- **compose.rs** (deck-outlet; here the shelf it reads: the library, the preloaded media, decoding) – the flat layers painted into one plane at the render size on the composer's thread (fit cover /
  contain, opacity, text in Bangers, the surtitles of the show's slice; a scroll steps every 6 s); the 3D set goes to
  the space node (`Space::show`: another set loads on its thread, the being and the camera move to the spot).
- **render.rs** – the `outlet` pass (`shaders/outlet.wgsl`) uploads each plane and cross-fades it over its `in` by the
  cell's longest `enter` / `exit` (default 0.6 s); the backdrop shows only where the cell has a 3D set. The frame path
  only polls the channels and computes the fade – no allocation (tests/no_alloc.rs).
- **HUD** (the words are deck-outlet's `status.rs`, `preload.rs` and `recompose.rs`, #391; when the composer composes again is `recompose.rs`) – under the title: `LIVE <outlet>  OUTPUT c4 OF left @ CODE` (or joining / offline and why), `MEDIA n / m
  READY – <id>: <why>`, `CELL n LAYERS + SPACE`, then `! <layer>: <reason>` for everything the cell can't show.

Try it without the conductor app: `cargo run --release -p relay -- 127.0.0.1:7070`, the stage with `--scene outlet
--relay ws://127.0.0.1:7070/live/DECK --outlet left --windowed`, then the stand-in conductor
`cargo run --release -p deck-native --example live-cue -- ws://127.0.0.1:7070/live/DECK native/examples/sermon.json`
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
  **The engine is deck-engine's** (`deck/engine`, #317 – the universal core's crate, native and wasm; this crate keeps
  the device I/O around it: `src/audio/` = backend, ring, lead, priority, glitch, the sequencer's and the dispatcher's
  threads, the module library as its `ModuleSource`). **Control** (`engine/src/control.rs`, #319): ONE control half,
  stepped by the engine once per 128-frame quantum before the audio steps (the screen no longer drives the sound): one
  value slot per control output, every control input a column (deck-core's `Col`: the linked slot × scale + offset, or
  the param as a constant), the computed nodes (`lfo`, the helpers `smooth` / `mul` / `add`) one batch per level and
  operation; the tempo node reads the clock at the block's first frame, the FFT analyses what was rendered (causal).
  The frame thread's `Controls` is a face: it reads the values of the heard block from a snapshot ring (64 blocks,
  by the time base's lead – the picture shows what is heard) and sends bases, sends and LFO shapes as commands through a
  lock-free ring. **Audio** (`engine/src/`): levels by the audio links; a level's
  sine oscillators and FM voices run as one batch each (`voices.rs`: groups of eight voices in deck-core's `F8` lanes,
  the group's state in registers for the block; `fm` is true frequency modulation), every other node as a block of its
  own (`blocks.rs`, the patcher model for one-off nodes: saw / square / triangle on deck-core's mip-mapped wavetables,
  noise on seeded xorshift / Kellet pink and the lowpass on a state-variable filter – `core/src/dsp.rs`, no fundsp;
  sequencer, click, player with linear resampling, mixer, FFT tap, output). One preallocated stereo buffer of 128 frames
  per output port; the engine renders whole 128-frame quanta whatever the device asks for. `Build::patcher()` runs every
  node as a block (the engine before #183) – the reference the tests compare with: the batches agree with it bit for
  bit. Control values reach the callback as atomics (`Param`), tracks through a lock-free ring
  (rtrb), the FFT's samples leave through another. **No zipper** (#188): every control input glides across its
  block – deck-core's `smooth::Ramp`, a one-pole per block spread linearly per sample (gains, frequencies 25 ms, the
  rest 12 ms), the same in the voices, the blocks and the browser's batch. **The soft limiter** (deck-core's
  `limiter.rs`, also live-core's) instead of a hard clamp, last before the device: 64 frames look-ahead (the stamp
  accounts for it), held to −1 dBFS, a tanh bend above 0.95 – never 1; its gain reduction goes to the HUD (`LIM 3`), the device's
  underruns (a block heard later than the last one's end, `Engine::mark`) into the glitch indicator ("Audio –
  live-safe"). `examples/render-wav.rs` renders 10 s of the cathedral with a hand on the knobs into a WAV and prints the
  largest sample step (2026-10-31: 0.142 → 0.060, second difference 0.145 → 0.015). deck-engine's `tests/no_alloc.rs` counts
  allocations around 2000 callbacks of the default scene (the click on, the tempo changing), AVX2 and baseline: zero;
  this crate's keeps the ring, the sequencer and dispatcher path and the being.
  Each block is stamped with the moment it is heard – the time base (`src/clock.rs`).
- **x86-64-v3** (owner 2026-10-30): one baseline x86-64 binary; the voice batches' loops are compiled twice (once with
  `#[target_feature(avx2, fma)]`) and picked at start by CPU detection, bit-identical results – the start log says
  `audio graph: batch avx2 …`, a `--frames` run's summary too; `DECK_SIMD=baseline` keeps the baseline loops. A
  whole-program v3 binary (`cargo stage-v3`, deck/.cargo/config.toml) is faster again on the one-off nodes (the default
  scene 10.0 → 7.5 µs per block); the ISO may ship it beside the baseline one (#183).
- **Shared with deck-core** (#183, #185): the maths, `F8` lanes, the FFT analyzer (`analysis::Analyzer`
  – `src/fft.rs` only drains the audio ring into it) and the tempo map (`tempo`: map, quantize, tap); the same code
  builds for wasm32 (the browser's Live mode).
- **Tempo** (`src/tempo/`, #181): a tempo map over the audio frame position (beat = f(audio position)); the frame thread
  follows the master source, re-anchors the map at the audio thread's frontier and publishes it through a seqlock – the
  click and the sequencer read it per block (sample-accurate), MIDI clock out (`midi_out`, its own thread) per tick, the
  `tempo` node per frame. Sources stamp their messages on arrival; MIDI clock in is fitted by least squares over two
  beats (±1 ms of jitter → < 0.1 BPM, < 1 ms of phase), Link answers its beat for the frame's moment, OSC listens for
  `/tempo/bpm f`, `/tempo/beat f`, `/tempo/start`, `/tempo/stop`, `/tempo/tap`; the relay: the show clock (#321) – the deck follows the leader's anchor through its offset filter (HUD `SHOW ±x MS`), a deck that leads (`lead = true`, or on MIDI clock in) sends it.
  Ableton Link is the cargo feature `link` (rusty_link 0.4.9: cmake, a C++ compiler and libclang at build time; on
  Windows a debug build needs `--config 'profile.dev.package.rusty_link.opt-level=1'`, else its debug CRT doesn't link).
- **Video** (`src/render.rs`; the window's device and surface are `src/gpu.rs`, the window itself and the frame-rate
  cap's wait `src/host.rs` – #443: the native editor's proof (#444) makes its window and draws the stage on the same
  pieces): one fullscreen pass per video node into its own texture (Rgba8), the screen blits to the
  surface, the HUD (`src/hud.rs`, font8x8 into a small texture, integer scale by the screen height) on top – what its
  lines and the bar's items say and the F1 mode are deck-ui's (#374, deck/ui; `src/hud_status.rs` reads the deck's
  objects into its `Status`). No hardware
  adapter → the software one (llvmpipe / lavapipe / WARP) at half the render size; a shader that does not compile →
  `pass` with the error in the log.
- **Space** (#188; the crate **deck-space**, `deck/space/src/space/` – lifted out in #198 so the browser draws the same
  being, deck/space-web; the scene model `graph.rs` / `params.rs` / `scene.rs` lives there too, deck-native re-exports it
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
- **Slices** (`src/chapter.rs`, deck-outlet's `text.rs`): one loader thread per chapter node – the chapter (retried every 10 s
  while there is neither a network nor a cached copy), then slice after slice: the image from the cache, decoded
  (`image`: webp, png), the dialog set in and the caption drawn on the CPU (ab_glyph), handed over through a channel of
  one (the next slice is always ready, at most two in memory); the renderer uploads it at the cut. A pass reads up to
  three textures (bindings 1, 3, 4: `in`, the current / previous slice or an overlay's `top`).

Dependencies are pinned to the APIs this was written against (wgpu 30 since #485, winit 0.30, cpal 0.15, pipewire 0.10 (0.8's libspa no longer builds against Arch's PipeWire headers), symphonia 0.5, ureq 2, image 0.25, ab_glyph 0.2, resvg 0.45); `Cargo.lock` points gpu-allocator at windows 0.62, wgpu-hal 30's (cpal 0.15 pulls 0.54, which a fresh resolve picks and
which breaks wgpu-hal's DX12 build on Windows). A bump is its own step.

## Measured

| Where | Scene | Numbers |
|---|---|---|
| Windows 10, RTX 2070 SUPER, Vulkan, release | default (null audio) | 60.3 fps, 16.6 ms avg frame, 0.29 ms CPU per frame, first frame 845 ms, catpope 1 music online in 1.1 s (cached 0.4 s) |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | default | open – the owner's stick (deck/HARDWARE.md) |
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-30, #188) | cathedral (null audio) – set low / medium / high | 60.4 / 60.4 / 60.3 fps (vsync), CPU per frame 0.37 / 0.39 / 0.40 ms (the default scene: 0.31), first frame 719 / 802 / 827 ms; the set loads on its thread in 174 / 449 / 1574 ms (114,788 / 116,163 / 129,205 triangles, 4 WebP + their mips), its upload 2 / 4 / 9 ms; p99 ~50 ms in every scene = the start |
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-30, #188 phases 4–5) | cathedral with its head · conductor + 5 stand-in decks (`--example decks`, same PC) | 60.8 fps, CPU per frame 0.42 ms (0.37 without the head) · 60.2 fps, 1.12 ms (0.39 alone) |
| Windows 10, RTX 2070 SUPER, Vulkan, release, 1280 × 720 window (2026-10-31, #188 tethered buttons, labels) | cathedral · conductor alone · conductor + 5 stand-in decks (30 s each, null audio) | 60.1 fps each, CPU per frame 0.51 · 0.49 · 1.54 ms (before the tethers: 0.42 · 0.39 · 1.12) |
| ThinkPad X1 Carbon Gen 7, UHD 620 (anv) | cathedral, conductor | open – the owner's stick |
| Windows 10, Ryzen 5 3600X, `cargo bench -p deck-native` (2026-10-30, #183) | the graph, µs per 128-frame block: before #183 → patcher → batch → batch AVX2 | default scene 11.0 → 9.9 → 11.1 → 10.0 (whole v3 binary 7.5) · 1 FM voice – → 2.2 → 3.5 → 2.2 · 16 FM voices 37.8 → 34.1 → 10.2 → 7.7 · 16 sines 20.1 → 15.4 → 7.3 → 5.3 · 64 FM voices – → 125 → 41 → 27 · control step, 300 LFOs: 58.4 → 2.1 (3 levels × 100), 69.6 → 5.5 (300 levels) |

## Outputs (#211)

Every parameter (`node.port`, a control input or output) can drive MIDI and DMX: the scene's `[outputs]` table – its
format and every check: deck/outputs/src/config.rs; the math, profiles and the strobe: deck/outputs. The stage opens the
bridge's adapter crates in-process (the USB-DMX widget, a MIDI out port – no relay hop for the hardware) and runs
deck-outputs' engine on its own thread: continuous targets smoothed on the universe's 40 Hz, the strobe's flashes on the
tempo clock's grid, each sent `lead` ms before its beat is heard, in its own DMX frame (`send_now`). MIDI for the
conductor goes over the relay link as `deck.midi`. A wrong table or a missing device never stops the scene – the HUD's
`OUTPUTS` line says what runs or why not. The strobe test: `scenes/strobe.toml` (the generic strobe at address 1, the
click on, an LFO sweeping the divisions):

```bash
cd deck/native && cargo run --release -p deck-native -- --scene scenes/strobe.toml --windowed
```

**The USB-DMX widget:** the adapter (adapters/dmx-usbpro) speaks the Enttec DMX USB Pro protocol and opens the port at
**250000 baud**: the Eurolite USB-DMX512-PRO MK2 (FTDI FT232R, a clone) puts nothing on the line at 115200; an original
Enttec and the CDC widgets ignore the baud. The MK2 never answers the device check – one info line, frames go out anyway.

**Fixture profiles** are built in by name (`profile = "strobe"`, `"thunderwash-600-rgbw"`) or a file next to the scene.
`thunderwash-600-rgbw` = the Cameo Thunder Wash 600 RGBW in DMX mode **4CH_1** (lamp menu: DMX Mode 4CH_1, confirm with
ENTER, address 001): CH1–4 = red, green, blue, white – no strobe channel, so nothing the deck sends can start the lamp's
own strobe (confirmed on the lamp 2026-11-01; its 7CH_1 does not follow the manual and is not profiled). A colour from a
**hue parameter** (0–1 = a turn of the colour wheel): one map target per channel with `hue = "red"` / `"green"` /
`"blue"` (applied before the curve; `max` caps the brightness). A **gain** on a map multiplies its shaped value by more
parameters – `gain = ["tempo.pulse", { from = "level.out", in = [-1, 1] }]`, `gain_floor = 0.1` (what stays between
pulses), `smooth` for a soft attack – so colour channels pulse on the beat without a flash channel. The scene
`pulse-thunderwash` (built in; the file `scenes/pulse-thunderwash.toml`): a slowly turning colour that pulses on the
tempo's beat, every channel capped at 15 % (`max = 0.15` – raise the three together for a show). A scene that ends
in the stage – **Ctrl+Q**, or a **stop signal** (SIGTERM / SIGHUP / SIGINT, Ctrl+C on Windows: `deck-set
--restart`, `systemctl restart deck-kiosk`, an update's restart, #263; `src/stop.rs`) – blacks its DMX out (the widget
repeats its last frame for ever otherwise) and fades the audio out (a scene *switch* hands the lamps over instead: "Scenes", #264), then the stage exits 0, within the kiosk's 5 s stop
(a watchdog leaves after 3.5 s if the loop hangs). Only a SIGKILL leaves the lamp lit. The built-in scene `lamps-off`
(the Thunder Wash's channels held at 0) stays for switching a lit lamp off by hand.

