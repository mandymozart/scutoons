# deck/ – the SCUTOONS deck suite (Rust)

The native half of the platform: the Live relay and device bridge, the stage (audio + video; 3D later)
(agents/research/CYBERDECKS.md – the plan; LIVE.md B.3, B.4, B.6, B.7 – what relay and bridge do; #172 – the work).
Owner 2026-10-29: Rust first, no Node + Electron for anything native.

## Crates

| Crate | Kind | Does |
|---|---|---|
| `protocol` | lib | the B.6 envelope + message kinds as serde types, **generated** from `protocol/schema.json` (below) |
| `adapters/core` | lib | the `Adapter` trait (start / send / stop / ports), `DeviceConfig`, the bus events the default maps translate |
| `adapters/midi` | lib | `midir` in / out + the **default map** of B.3 (slice enter = note on ch 16, event types per channel, variables as CC) |
| `adapters/osc` | lib | OSC over UDP (`rosc`): `host` = out (Resolume, TouchDesigner, laser software), `listen` = in → `device.in`; typed args (`int` / `float` / `string` / `bool`), bundles flattened |
| `adapters/dmx-usbpro` | lib | USB-DMX over the Enttec DMX USB Pro widget protocol (Enttec Pro, DMXKing ultraDMX; #187): one universe, frames on change ≤ 40 Hz + keep-alive, fades, `panic` = black, reconnects, the widget detected or `port`; `--example dmx-probe` checks the hardware – adapters/dmx-usbpro/README.md |
| `relay` | bin | WebSocket server: sessions by code, roles + groups from the URL / `hello`, stamps `seq` + `from`, routes by `to`, `GET /status`; B.6 rules in `relay/src/session.rs`: conductor-only kinds, audience 5 / s, the clock lead on effects without `at` (`RELAY_LEAD_MS`, 50 / 150), snapshot per role, replay on `resume` (500 messages / 5 min, the same client id), votes (`vote.rs`: majority / groups / first-to-n, fallback, override, tally every 250 ms, closes itself), cue split per outlet, presence, `assign`, the decks' stream (#188: `deck.graph` kept for the conductor's snapshot, `deck.values` ≤ 30 / s folded and never replayed, presence `decks` + `routes`), the scene sync (#195: `scene.sync` conductor-only to the group `decks`, kept for a late deck's snapshot; `deck.synced` to the conductor, kept for its snapshot), nick trim + `RELAY_BLOCKLIST`; the send path (#223, relay/README.md: TCP_NODELAY, an outbox per client where the latest value wins, a client too far behind dropped, an adaptive value rate per link, `links` in presence; the bench); each deck's `deck.quality` (#220) kept and in presence `quality`; HMAC session tokens (`token.rs`, `LIVE_TOKEN_SECRET`; unset = dev mode); SIGTERM stops it (a container's stop); wss with a certificate (#201, `relay/src/tls.rs`: rustls, the same port then speaks ws and wss – the first byte decides –, new cert files are used from the next handshake on). Hosted at `wss://live.scutoons.com` (#199, LIVE.md B.4): `relay/Dockerfile` (static musl on `scratch`, ~3 MB) → `.github/workflows/live-image.yml` → bunny; the stage's clients and /live/deck join it when nothing names a relay. Not yet: the uplink, pickups via the API |
| `core` | lib | `deck-core`, the platform-free core (#182, #185): maths, `F8` lanes, batch columns, the patcher graph, its modules as features (control, synth, analysis = the stage's FFT, tempo = the tempo map), the stage's module table (`stage.rs`); builds for wasm32 too |
| `space` | lib (+ wasm) | `deck-space` (#198): the 3D space and the being lifted out of the stage – the set loader (glb, meshopt), the being (tethered buttons, hoses, bubbles, the head, labels), the camera, picking and drag, the feelings, the conductor's mirrored beings, the GPU passes (wgpu), the quality ladder and its governor (#220, `quality.rs`) – and the scene model it reads (graph, params, scene); features `being` · `gpu` · `native` (loaders on threads, images decoded in Rust); all but `native` build for wasm32 |
| `space-web` | wasm | deck-space in the browser (#198): the being and its set drawn with wgpu into a canvas – WebGPU where there is an adapter, else WebGL2 – behind a wasm-bindgen `SpaceView` the page drives (its values in, the being's buttons out: one state; the set and portraits fetched and decoded by the browser; the governor sizes the canvas to the rung's render size, #220); `npm run build:wasm -w shared/live-deck` (profile `wasm-web`, ≈ 1 MB gzip); natively empty |
| `stage` | bin + lib | `deck-stage`, the AV app the deck boots into (CYBERDECKS.md §8, #179): winit + wgpu, a node graph from a TOML scene, an audio graph on PipeWire / cpal with synths + an FFT, chapter music from the library API with a disk cache; modules = cargo features (#185) – stage/README.md |
| `live-core` | lib + wasm | deck-core for the browser Live mode (#184): the batch graph, render and FFT behind raw `lc_*` wasm exports, the boundary's wire records (`wire.rs` = shared/live-core `wire.ts`); hosted by `@scutoons/live-core` |
| `scene-features` | bin | `deck-scene-features scenes/*.toml`: the deck-stage features a set of scenes needs (a scene-driven build, #185) |
| `link` | bin + lib | `deck-link`, the deck's identity and access (CYBERDECKS.md §9, #180): an Ed25519 key made at first boot in `/var/lib/deck/identity`, the request with the code for the splash, polling until a member approves it on scutoons.com/account/decks, ~1 h tokens from a signed challenge, renewed by itself; on a deck one daemon (`deck-link run`, deck-link.service) keeps the code + the token in `/run/deck-link` for the stage and `deck-tls` (`pairing`, no dependencies – the stage uses it without the `net` feature); `deck-link id` / `once` / `status` / `pair` for the boot flow and tests. Server side: server/docs/API.md "Decks" |
| `bridge` | bin | loads a TOML venue patch (B.5.4), builds + starts the adapters, joins the relay as `bridge`, measures the clock offset (median of 5 pings), sends `devices { ports }`, queues `device` messages until their `at`, `panic` = queue cleared + every adapter silenced (MIDI CC 120 / 123, DMX black), forwards device input as `device.in`. Not yet: the status window, `dmx-artnet` / `dmx-sacn` |

`dmx-artnet`, `dmx-sacn`, `gpio`, `audio`, `video-in`, `display`, `laser` (CYBERDECKS.md §4, #178) do not exist yet – the bridge names them
in its error when a patch asks for one.

## Build and run

Toolchain: `rustup` stable (≥ 1.80; the workspace pins `rust-version`). Linux needs the ALSA headers for `midir`
(`libasound2-dev`); macOS and Windows need nothing extra.

```bash
cd deck && cargo build --release
```

```bash
cd deck && cargo test
```

Relay (default `0.0.0.0:7700`; `relay 127.0.0.1:7700` or `RELAY_ADDR`):

```bash
cd deck && cargo run --release -p relay
```

Bridge (copy `bridge/venue.example.toml` to `bridge/venue.toml`, set the MIDI port names your OS lists – the bridge logs
them at start; `venue.toml` is git-ignored):

```bash
cd deck && cargo run --release -p bridge -- bridge/venue.toml
```

A client joins at `ws://127.0.0.1:7700/live/<code>?role=output&group=left` and sends
`{"v":1,"seq":0,"from":"","kind":"hello","p":{"code":"<code>","role":"output","group":"left"}}`; the relay answers
`welcome`. Conductor, bridge and uplink need a `token` in `hello`: with `LIVE_TOKEN_SECRET` set an HMAC token from the API
(`relay/src/token.rs` – `base64url(payload).base64url(HMAC-SHA256)`), without it any non-empty string (dev mode).
`curl http://127.0.0.1:7700/status` → `{"sessions":n,"live":n}`.

wss (#201 – a venue's relay that phones on https pages may join): `relay 0.0.0.0:7700 --tls-cert fullchain.pem --tls-key
privkey.pem` (or `RELAY_TLS_CERT` / `RELAY_TLS_KEY`). The port takes `ws://` and `wss://` alike (a TLS ClientHello starts
with 0x16, an HTTP request with a letter), so the LAN decks keep `ws://<ip>:7700` while phones open
`wss://<lan-ip-with-dashes>.lan.scutoons.com:7700/live/<code>` (bunny DNS answers those names). Every handshake checks the two files' time and size: a renewed pair is
used from the next connection on, a half-written one (the key not yet the cert's) keeps the old certificate. A broken pair
at start stops the relay. On the deck image: `deck-tls` (deck/os/README.md "wss in the venue").

The hosted relay's image (#199; build context = `deck/`, `deck/.dockerignore` keeps the targets, lab and ISOs out):
`docker build -f deck/relay/Dockerfile -t scutoons-live deck`, then `docker run --rm -p 7700:7700 scutoons-live`.

Logs: `RUST_LOG=debug` (tracing, `info` by default).

## Protocol: one schema, two outputs

`protocol/schema.json` (JSON Schema draft-07) is the single source of the B.6 protocol. `protocol/generate.ts`
writes `shared/live/src/protocol.ts` (`@scutoons/live`, TypeScript) and `protocol/src/generated.rs` (serde). Both
outputs are committed; `npm run check -w shared/live` fails when they are stale (CI job "Protocol").

```bash
npm run generate -w shared/live
```

Why a 200-line script and not `typify` + `json-schema-to-typescript`: two generators from two ecosystems would
shape the same union differently (typify emits untagged enums for `oneOf`, the TS tool emits every object inline),
and we need exactly one shape on both sides – a `kind`-tagged union (`#[serde(tag = "kind", content = "p")]` ↔ a TS
discriminated union) and an envelope that flattens it. The script understands only the schema subset we use (listed in
the schema's description) and throws on anything else, so a new construct is a conscious change here, not a surprise
in the generated code. No build.rs, no proc-macro at compile time: the Rust output is plain, readable, diffable code.

The shared fixture `protocol/fixtures/messages.json` is parsed by both sides (`cargo test -p protocol`,
`npm test -w shared/live`).

## Dogma (CYBERDECKS.md §2) – the release checklist

Checked per release like the viewer's size checks; measured numbers go into `deck/HARDWARE.md` once it exists (§8).

- [ ] idle RAM relay + bridge < 150 MB
- [ ] cold start < 2 s (relay listening, bridge joined with its adapters started)
- [ ] no busy loops: every wait is a socket read, a tokio timer or a hardware callback – `grep -rn "loop {" deck/*/src` and
      justify each
- [x] release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip`; the bridge at `opt-level = "z"`
      (`Cargo.toml` `[profile.release]`)
- [ ] every acceleration path has a software fallback that is chosen, never crashed into – the stage: software adapter at
      half size, null audio backend, `pass` for a broken shader (stage/README.md); video decode comes with S4
- [x] builds for Linux x86_64 + aarch64 (`cross`, `Cross.toml`), macOS, Windows – `.github/workflows/deck.yml`, only on
      changes under `deck/` or `shared/live/`

## Layout

```
deck/
  Cargo.toml          workspace, release profile
  Cross.toml          aarch64 cross build (arm64 ALSA headers)
  protocol/           schema.json · generate.ts · fixtures/ · src/generated.rs (generated) · src/lib.rs (helpers, tests)
  adapters/core/      the Adapter trait
  adapters/midi/      midir + default map
  adapters/osc/       rosc over UDP, in + out
  relay/              the relay binary + tests/routing.rs (a real server, three roles, two groups) · tests/tls.rs (wss) · examples/bench.rs (#223)
  bridge/             the bridge binary + venue.example.toml
  link/               deck-link: src/ (identity, link, http, pairing) · tests/link.rs (a fake server) · tests/boot_flow.rs (deck/os)
  core/               deck-core: the platform-free core (also wasm32) · src/stage.rs (the stage's modules)
  live-core/          deck-core as the browser's wasm (#184) · src/wire.rs (the boundary's records)
  scene-features/     deck-scene-features: scenes → cargo features
  space/              deck-space: src/ (graph, params, scene) · src/space/ (being, set, camera, GPU) · src/shaders/space.wgsl
  space-web/          deck-space as wasm for the browser (the reader's /live/deck, shared/live-deck)
  stage/              deck-stage: src/ (modules, audio, render, library, space/link.rs) · scenes/ · tests/no_alloc.rs
shared/live/          @scutoons/live: src/protocol.ts (generated), test/ (fixture + generator tests)
```
