Status 2026-10-29: research brief, nothing built (#171, milestone "Cyberdecks"). Decided (owner 2026-10-29, DECISIONS): **Rust first** – the Live relay + bridge in Rust from the 2026-11-16 kickoff, no Node + Electron. Dates (owner 2026-10-29): beta at the Top Toy workshop 2027-02-17, public for a tour in summer 2027. Hardware (owner 2026-10-29): the owner's ThinkPad X1 first, a Framework 13 AMD mainboard for the built deck; the code lives in `deck/` (#172). Builds on research/LIVE.md (B.4 relay, B.6 protocol, B.7 bridge) – the Live kickoff 2026-11-16 is where this plan first meets code.

# CYBERDECKS — Arch Linux decks running the SCUTOONS suite natively (Rust)

Owner 2026-10-29: a series of cyberdecks, all on Arch Linux, running our own suite: performance (Live), audio, video
editing, 3D. Coded in Rust, hardware acceleration wherever the hardware has it, minimal resource waste so the suite
runs on really any hardware. A series of adapters like the Live plan's devices; which parts stay web UI for good reason,
which run natively and gain from the low abstraction layers.

## 1. What a deck is here
A small computer we assemble and own, booted straight into our suite – no desktop, no app store, no update prompts.
Three hardware classes, one image each (§3), the same binaries:

| Class | Example | GPU / Vulkan | Video decode | Video encode | Fit |
|---|---|---|---|---|---|
| **A x86** | an old ThinkPad, a NUC / mini PC, a Steam-Deck-like handheld | Intel / AMD iGPU, Mesa (anv / radv) | VA-API (H.264, HEVC, AV1 on newer) | VA-API | conductor laptop, editing, 3D – **first target**: the owner's ThinkPad X1 (today), a Framework 13 AMD mainboard (the built deck) |
| **B ARM big** | Rockchip RK3588 boards (Orange Pi 5, Rock 5, Radxa) | Mali G610, Mesa Panthor (Vulkan since kernel 6.10) | V4L2 stateless + rkmpp (4K HEVC / AV1) | rkmpp (H.264 / HEVC) | a fanless output / bridge deck, 8–16 GB |
| **C ARM small** | Raspberry Pi 5 | VideoCore VII, Mesa V3D (Vulkan 1.2) | HEVC only (V4L2) | **none** – software | the cheapest bridge: MIDI / DMX / OSC / GPIO, no video |

Rule: every feature degrades per class (C has no video layer; B and C no editing); the suite reads the class from the
image and hides what the deck cannot do. Phones, tablets and the audience never run the deck stack: they stay web (§5).

## 2. Why Rust, and the dogma
- One language for the timing-critical parts (MIDI clock, DMX refresh at 40–44 Hz, audio buffers of 64–256 frames),
  hardware access (serial, UDP, V4L2, GPU), and throughput (decode, render) – without a runtime, a GC or Electron.
- Cross-compiles to all three classes and to macOS / Windows from the same code: the Rust bridge **replaces** the
  Electron app of LIVE B.7 everywhere, not only on decks (owner 2026-10-29: "we can port better from rust to others").
- **Dogma** (checked per release, like the viewer's size checks): idle RAM of relay + bridge < 150 MB; cold start < 2 s;
  no busy loops (timers by `tokio` / the audio callback, never polling); release builds with LTO, `panic = "abort"`,
  `opt-level = "z"` for the bridge; every acceleration path has a software fallback that is chosen, never crashed into.
- Sizes in the UI follow DESIGN.md as far as the toolkit allows (rem-like scaling by the display's DPI).

## 3. The OS image
- **A live USB stick first** (owner 2026-10-29: "hijack any hardware around"): the x86 image boots any machine from the
  stick without touching its disk – first screen: pick the Wi-Fi (or the deck's own hotspot), then `deck-check` runs:
  GPU + Vulkan driver, video decode / encode paths, audio device + latency, MIDI / USB / serial devices, RAM, and writes
  the result as the machine's class and feature set (what §1 calls "the suite reads the class"); then the suite starts.
  Persistence on the stick (a second partition: venue patches, pairing, the pacman cache). **Install to a partition is
  optional** – the same image, `deck-install` next to the check (a guided partition pick, never the whole disk unasked).
  The ARM images (B, C) are SD / eMMC images of the same package set; the check and the installer are the same scripts.
- **Arch Linux** (x86) and **Arch Linux ARM** (B, C) – the same package set, built reproducibly from the repo:
  `deck/os/` with an `archiso`-style profile for x86 and a `pacstrap` image builder for ARM; the suite as PKGBUILDs in
  `deck/pkg/`, published to our own pacman repository (a bunny storage zone, like the CDN) – a deck updates with
  `pacman -Syu`, offline-safe (nothing breaks while a show runs: updates only outside a session, LIVE B.4's freeze idea).
- **Boot to the suite**: `cage` (a Wayland kiosk compositor) running one app; `sway` only on class A editing decks.
  No display manager, no desktop, no X11.
- **Audio**: PipeWire with a realtime config (quantum 64–128 at 48 kHz, `rtkit`), the `linux-rt` kernel on class A/B.
  Jack clients work through PipeWire's Jack layer – the DAW case stays possible (Ableton is not Linux; Bitwig /
  Reaper / Ardour are).
- **Network**: NetworkManager with a hotspot profile (the deck is the venue's LAN if there is none, LIVE B.4 local) and
  mDNS (`scutoons-<deck>.local`); the hosted uplink as in B.4 linked mode.
- **GPIO / panels** (class B/C): a `gpio` adapter (§4) for the deck's own buttons, encoders and LEDs (`gpiocdev`).
- First boot: a QR / code pairs the deck with the SCUTOONS account (the API's device id, the member's token) – the deck
  is a device of a member, no password typing on a deck.

## 4. The adapters (`deck/adapters/`)
One trait, one crate each, loaded by the venue patch (LIVE B.5, a TOML file on the deck or from the show document):

| Adapter | Crate basis | Class | Notes |
|---|---|---|---|
| `midi` | `midir` (ALSA / CoreMIDI / WinMM) | A B C | in + out, default map of LIVE B.3, clock from the bridge's own timer |
| `dmx-artnet`, `dmx-sacn` | UDP via `tokio`, `sacn` | A B C | refresh at 40 Hz even when nothing changes (fixtures expect it) |
| `dmx-usbpro` | `serialport` | A B C | **built** (#187): the Enttec DMX USB Pro widget protocol (Enttec Pro, DMXKing ultraDMX); the widget keeps the refresh, the adapter sends on change ≤ 40 Hz + a keep-alive, fades, reconnects; its own thread, never async. No Open DMX bit-banging (owner 2026-10-30) – deck/adapters/dmx-usbpro/README.md |
| `osc` | `rosc` | A B C | in + out, Ableton / TouchDesigner / Resolume |
| `laser` | `ether-dream` / `helios-dac` (nannou laser), own frame + path optimiser | A B | owner 2026-10-29: ILDA DACs (Ether Dream over the LAN, Helios over USB); frames at 20–30k pps with blanking + corner dwell; the **laser tracing** effect: edges of the live camera (`video-in`) or a slice's outlines → vector paths → the beam. Safety: audience scanning is the venue's licence, never ours by default |
| `gpio` | `gpiocdev` | B C | the deck's own buttons, encoders, LEDs; a class-A deck has none |
| `audio` | `pipewire-rs` (Linux), `cpal` elsewhere | A B | samples, the FX cues (research/FX.md), the score synth of SCORES §4 natively; the clock master if the DAW is not |
| `video-in` | V4L2 via `libcamera` / `v4l` | A B | cameras, capture cards; the onion camera (parked branch) gets a native home |
| `display` | `wgpu` + `winit` on Wayland | A B | the output renderer (§5): per outlet one fullscreen surface, vsync-locked |
| `relay-uplink` | `tokio-tungstenite` | A B C | the hosted relay (B.4), the protocol of B.6 unchanged |

The protocol types of `@scutoons/live` (B.6) are generated from one schema (JSON Schema → TypeScript + `serde`
types) so the web clients and the Rust relay never drift. The relay itself (sessions, roles, groups, snapshot + replay,
votes, clock) is `deck/relay`, one binary that is also the hosted relay's container (bunny Magic Containers, B.4) –
the same code on the deck and in the cloud.

## 5. Native or web – the split and the reasons

Native when at least one holds: **timing** (sub-frame, MIDI / DMX / audio), **hardware** (serial, UDP, V4L2, GPU
compute, GPIO), **throughput** (decode, encode, 3D), **offline first boot**. Web when at least one holds: **already
built and iterated** (the creator's editors, the reader), **joined by URL on someone else's device** (phones), **must
always be the deployed version** (the conductor, LIVE B.7).

| Part | Today / planned | On a deck | Why |
|---|---|---|---|
| Relay | Node 22 + `ws` (LIVE B.4, not started) | **native** `deck/relay` | timing, one binary everywhere, the hosted container too |
| Device bridge | Electron + node drivers (B.7) | **native** `deck/bridge` | hardware; Electron is the resource waste the goal names |
| Conductor UI | web, in the Electron window | **web** in the deck's kiosk browser or a `wry` webview | always the deployed version, iterated in the creator's toolkit |
| Outputs (projection) | a fullscreen browser per outlet | **native on the deck since 2026-10-29: the stage (§8)**; before that: **both**: web first (the reader's slices are `@scutoons/reader`, Svelte), the **video / media layer native** (`display` adapter: hardware-decoded video, live camera, black, vsync) composited under or over the browser surface; a fully native slice renderer only if the browser's scroll ever drops frames on class B | throughput where it matters, no second reader implementation |
| Audio engine (cues, samples, score synth) | Web Audio in the browser | **native** `audio` adapter on the deck; Web Audio stays on phones | latency, PipeWire, the DAW clock |
| Performer / musician / audience | web (phones, tablets) | **web** – unchanged | joined by URL, not our hardware |
| Reader, creator, admin, embassy, shop | web / PWA | **web** – unchanged, installed on class A decks like any PWA | built, iterated, deployed |
| Storyboard studio + generators | web tool + `comfy.mjs` (local GPU) | web tool; a **native** worker only if a deck gets a GPU worth it (class A dGPU) | the GPU work is remote (RunPod) by design |
| 3D cast viewer (CHARACTERS3D, storyboard 10.95) | planned in the browser (three / WebGPU) | **native** `deck/view3d` (`wgpu`, glTF via `gltf`), the browser version for the creator | throughput on class B; same glTF assets |
| Video editing | nothing yet | **native** `deck/cut` – an assemble / cut / export tool on GStreamer (`gstreamer-rs`, VA-API / rkmpp through its plugins): chapter videos, storyboard previews, show recordings. Not a Premiere clone (§7 risk) | throughput, hardware encode |
| Score rendering (Verovio) | web, lazy | **web** (Verovio is C++ with a JS build; a native build is possible later via its C API) | built, good enough |

## 6. Phases (owner 2026-10-29: starts now, parallel to Chapter 3 – agents build while the owner edits content)
Two dated targets: **beta = the Top Toy workshop, 2027-02-17** (phases 1–3: the workshop runs on a deck – the same idea
as the workshop, refined); **public = a tour in summer 2027** (phases 4–5 as far as the tour needs them; milestone due
2027-06-30 as a placeholder). The work items are the issues of milestone "Cyberdecks"; the plan and its decisions: #171.
1. **Relay + bridge in Rust** – #172 (= LIVE phase 1 on the deck stack; merges with the Live kickoff 2026-11-16).
2. **The class-A image** – #173 (boot to kiosk, PipeWire RT, pairing, `deck/HARDWARE.md`).
3. **Native audio + display adapters, class-B image** – #174.
4. **3D viewer** – #175.
5. **Cut tool** – #176 (only when a concrete export need exists).

## 7. Risks and what to verify first
- **Double work on Live** – resolved 2026-10-29: Rust from the start for relay + bridge (the protocol is designed, B.6;
  the scope of phase 1 is the same). Cost: the first week is slower; agents write Rust as well as TypeScript.
- **Class B drivers**: Panthor Vulkan and rkmpp encode are mainline-recent; verify on real hardware before promising
  video on ARM. Class C has no hardware encode at all – never plan recording on a Pi.
- **The video editor** is the only part that is a product of its own; it is phase 5 and may stay a CLI (`deck cut
  --edl show.json`) for a long time. Say so before anyone expects a timeline UI.
- **Wayland kiosk + a browser surface + a native surface** (outputs, §5): verify compositing order and vsync on class A
  and B in phase 3's first week; the fallback is "video inside the browser" (what the plan has today).
- **Signing / distribution** on macOS / Windows stays as in B.7 (unsigned for now); Linux decks need none.

## 8. The stage – the native AV app the deck boots into (#179)
Owner 2026-10-29: "start building a ui … some hardware accelerated … amazing tool … which the machine boots right into …
and connect directly to our scutoons.com ecosystem. … a default [that reads] some 3d content, some audio synth
generators and from some chapter some audio files and display an interactive scene with a video and some overlays. so
we can play av madness" · "a concept called oss which uses modular architecture which we can run close to the hardware".
Proven on the X1 first: Vulkan on the iGPU (anv), VA-API decode h264 / hevc / vp9, the SOF card (HARDWARE.md).

- **What:** `deck/stage`, one Rust binary – the deck's kiosk app (cage runs it instead of Chromium; the browser stays
  for the conductor page, on demand or on a second output). Changes §5's "Outputs" row: the deck renders natively.
- **Modular (the owner's "oss" – read as ossia score, ossia.io, the open-source intermedia sequencer; to confirm):** a
  node graph with typed ports. Sources: chapter slices, chapter music / samples, slice videos, a camera, a glTF model,
  synth generators. Processors: shader passes, audio FX, mixers, an FFT that turns sound into control values. Sinks: a
  screen outlet, the audio out, MIDI / DMX / OSC through the adapters (§4). A scene is a graph document (TOML now,
  later an entity on scutoons.com like the show document). ossia score itself (C++ / Qt, GPL) is not embedded: we take
  its model and can talk OSC to it.
- **Render:** wgpu on Vulkan, winit on Wayland under cage, one surface per outlet, vsync; a pass list per frame; 2D
  overlays (slices, text boxes, titles) by our own sprite + text pass (`glyphon`). No toolkit, no browser engine.
- **Video:** GStreamer (`gstreamer-rs`) with the VA-API plugin (vah264dec / vah265dec / vavp9dec) → appsink → a GPU
  texture; step two zero-copy (dmabuf import into Vulkan). Software decode is the fallback, chosen by deck-check's feature set.
- **3D:** the `space` node (#188): rooms as .glb read by the stage itself (meshopt, quantization, WebP – no `gltf`
  crate: it refuses meshopt), drawn with a depth buffer through an orbit camera, and the deck's **being** (its graph as a
  floating body, its parameters as 3D buttons, its head the character's portrait; the decks' beings around the
  conductor's over the relay – deck/stage/README.md). The artists' cast models (research/CHARACTERS3D.md) come later.
- **Audio:** our own graph on PipeWire (`pipewire-rs`, `cpal` fallback), a realtime thread that never allocates; DSP
  blocks from `fundsp` (oscillators, noise, FM, filters; granular on chapter samples); chapter music decoded with
  `symphonia`; out through the default sink (the RME UCX II in Class Compliant mode, #174). The audio clock is the master
  clock for video, MIDI and the bus.
- **Graph runtime** (#183, owner 2026-10-30: "Architecture is key for this"): the **batch model** – the graph compiled at
  load into dependency levels × kinds, struct-of-arrays loops (ECS systems, TouchDesigner channels, gen~) – with the
  **patcher** (one block per node) for one-off nodes. Control: every input a column, the LFOs batched per level and
  shape. Audio: a level's sine oscillators and FM voices as one batch in deck-core's 8-lane `F8`; sequencer, player,
  noise, mixer, FFT tap stay blocks. **x86-64-v3:** one baseline binary whose voice loops also exist in AVX2, picked at
  start; a whole v3 binary is optional (the one-off nodes gain too). Numbers: deck/stage/README.md "Measured".
- **Modules** (#185, DECISIONS 2026-10-30): cargo features of deck-stage – the core (graph, control, the basic audio
  nodes, render, HUD) + `space`, `chapter`, `midi`, `osc`, `link`, `pipewire`. A deck build carries what its scenes
  need (`deck-scene-features scenes/*.toml`); a scene that needs a module the build lacks is refused with the feature's
  name, on screen. Visuals as data reload (`--dev`: WGSL, scenes). Later: sandboxed wasm generators (wasmtime – no GPU,
  they may hand over WGSL as data); no dylib plugins. deck/stage/README.md "Modules".
- **Two clocks** (#181, owner 2026-10-29: "ein richtiger Klick … 60 BPM … Oder eine Clock, die von extern kommt"): the
  **time base** (`clock.rs`) = the exact audio position – each audio block stamped with the monotonic moment it is heard
  (PipeWire's pw_time, cpal's playback instant), a DLL on the frame thread interpolates per video frame, never
  backwards; the picture and the FFT show what is heard. On top the **tempo clock** (`tempo/`) = a tempo map over that
  frame axis (beat = f(audio position), never a second clock): the `tempo` node (bpm, beat, bar, phase, pulse) per
  frame, the `click` and the sequencer per sample, chapter cuts quantised to beat / bar. One master at a time (the
  scene's `source`, F3): internal (+ / −, tap), MIDI clock in (least squares over two beats), Ableton Link (cargo
  feature `link`), OSC (`/tempo/*`), the relay's show clock (a stub until the relay has one, #172). MIDI clock out runs
  from the audio clock (each tick when its beat is heard). The map, quantize and tap tempo are deck-core's `tempo`
  module (#185), like the FFT analyzer (`analysis`). Link on the ISO: `cmake` in os/container/build.sh and the PKGBUILD,
  `--features deck-stage/link` on their cargo lines.
- **Live outlet** (#186, DECISIONS 2026-10-30, research/LIVE-LENSES.md): a deck is a screen outlet – the `live`
  module's `outlet` node joins the relay as `output` of the outlet set in deck.toml, preloads the show media with the
  deck token (`media.ready` = the conductor's ready light) and draws the cells natively (layers → stage nodes, a 3D set
  through the space node, crossfades); a reconnect shows the snapshot's cell at once. deck/stage/README.md "Live outlet".
- **scutoons.com:** the public library API read-only (`/api/library/toons/<toon>/chapters/<n>`: slices, slice videos,
  music paths; files from cdn.scutoons.com), cached on DECKDATA (`/var/lib/deck/cache`) so a venue runs offline; later
  the paired member's token for drafts. The relay bus (LIVE B.1: slice enter, VN events, FX cues) feeds node parameters.
- **Control:** MIDI / OSC learnable onto any parameter (the UCX II's MIDI, a controller), the deck's keyboard and
  touchpad, a small on-screen HUD.
- **The default scene "AV madness"** (what the deck boots into): a chapter (catpope 1 unless configured) – its slices as
  a moving layer, its music plus a synth generator that reacts to it (FFT → visuals), a slice video or a local clip on a
  plane, a 3D object in the middle, overlays (title, text boxes); everything modulated by MIDI.
- **Dogma:** 60 fps at 1080p on the X1's UHD 620 with video + 3D + synth running; first frame < 3 s from a warm cache;
  idle CPU near zero when nothing moves. Measured per step in HARDWARE.md.
- **Steps:** S1 window + wgpu + the graph core + a test pattern, booted by cage on the stick · S2 the audio graph,
  synth generators, chapter music from the API with the cache · S3 slices + overlays from the chapter · S4 video via
  VA-API · S5 glTF · S6 MIDI / OSC mapping + the default scene. Each step runs on the X1 before the next.

## 9. Deck identity and access (#180, owner 2026-10-29)
Handshake, then approval, then data – a deck touches no SCUTOONS data before a member approved it.
1. **Identity:** at first boot the deck makes an Ed25519 key pair on the persistence partition; the private key never
   leaves it. No shared secret in the image – anything in the ISO is public.
2. **Request:** the deck sends its public key, build, class and hostname; the splash shows a short code ("K7-42").
   Pending requests expire after minutes and are rate-limited.
3. **Approval:** the conductor / the account page lists waiting decks; a member compares the code, approves and maps
   the deck to a member, a role or a Live session. The approval's lifetime is set there, like the MCP keys (MCP.md §3):
   default six months, renewable on the account page with one reminder mail; or shorter – N idle days, the end of the
   Live session. Revocable any time.
4. **Access:** within its approval the deck signs a server challenge and gets a short token (~1 h) with the mapped
   rights, renewed by itself while it runs – an internal detail, not a setting. Read-only for content; cached content keeps playing offline.
5. **Revoke / timeout:** the next renewal fails → the splash with a new code, a new request. The build never becomes
   useless.
**Who:** every member can pair a deck, readers included (owner 2026-10-29: gadgets people buy and link to their
account) – a device gets exactly its owner's rights, so pairing sits on the viewer's account page, phone-first (scan
the QR on the splash). Open: pairing a gadget without a screen (a code printed on it, or the phone over the local
network). Every deck is a row of its own (build, last seen, mapping), even when all map to one user. Before approval the stage
plays built-in scenes only. **Code:** server `Domain\Decks\Decks` (server/docs/API.md "Decks", migration `2026-10-29-decks`), the
account page's "Decks" (`/account/decks?deck=<code>` = the splash's QR), the deck's `deck/link` (`deck-link`). Master-quality media (stems, high-res video) are not part of this: they are show media,
assigned to outlets in the Live conductor (LIVE.md B.5.6) – the concept: research/LIVE-LENSES.md (#186); the deck
fetches what its outlet is given at `media.preload`, with its access token (deck/stage/README.md "Live outlet").

## 10. Open work
Lives in the issues, not here: the phases #172–#176, the stage #179 (#171 holds the plan and the decisions, all taken 2026-10-29).
LIVE.md B.4 / B.7 point here since 2026-10-29.
