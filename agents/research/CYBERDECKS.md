Status 2026-10-29: research brief, nothing built (#171, milestone "Cyberdecks"). Decided (owner 2026-10-29, DECISIONS): **Rust first** – the Live relay + bridge in Rust from the 2026-11-16 kickoff, no Node + Electron. Dates (owner 2026-10-29): beta at the Top Toy workshop 2027-02-17, public for a tour in summer 2027. Still open in #171: repo place, first hardware. Builds on research/LIVE.md (B.4 relay, B.6 protocol, B.7 bridge) – the Live kickoff 2026-11-16 is where this plan first meets code.

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
| **A x86** | an old ThinkPad, a NUC / mini PC, a Steam-Deck-like handheld | Intel / AMD iGPU, Mesa (anv / radv) | VA-API (H.264, HEVC, AV1 on newer) | VA-API | conductor laptop, editing, 3D – **first target** |
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
| `dmx-usb` | `serialport` (Enttec Open DMX, DMX USB Pro) | A B C | the Open DMX bit-bangs: its own thread, never async |
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
| Outputs (projection) | a fullscreen browser per outlet | **both**: web first (the reader's slices are `@scutoons/reader`, Svelte), the **video / media layer native** (`display` adapter: hardware-decoded video, live camera, black, vsync) composited under or over the browser surface; a fully native slice renderer only if the browser's scroll ever drops frames on class B | throughput where it matters, no second reader implementation |
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

## 8. Open work
Lives in the issues, not here: the decisions in #171 (repo place, the first class-A machine), the phases #172–#176.
LIVE.md B.4 / B.7 point here since 2026-10-29.
