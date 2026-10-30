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
| `relay` | bin | WebSocket server: sessions by code, roles + groups from the URL / `hello`, stamps `seq` + `from`, routes by `to`, `GET /status`; B.6 rules in `relay/src/session.rs`: conductor-only kinds, audience 5 / s, the clock lead on effects without `at` (`RELAY_LEAD_MS`, 50 / 150), snapshot per role, replay on `resume` (500 messages / 5 min, the same client id), votes (`vote.rs`: majority / groups / first-to-n, fallback, override, tally every 250 ms, closes itself), cue split per outlet, presence, `assign`, nick trim + `RELAY_BLOCKLIST`; HMAC session tokens (`token.rs`, `LIVE_TOKEN_SECRET`; unset = dev mode). Not yet: the uplink, pickups via the API |
| `core` | lib | `deck-core`, the platform-free core (#182, #185): maths, `F8` lanes, batch columns, the patcher graph, its modules as features (control, synth, analysis = the stage's FFT, tempo = the tempo map), the stage's module table (`stage.rs`); builds for wasm32 too |
| `stage` | bin + lib | `deck-stage`, the AV app the deck boots into (CYBERDECKS.md §8, #179): winit + wgpu, a node graph from a TOML scene, an audio graph on PipeWire / cpal with synths + an FFT, chapter music from the library API with a disk cache; modules = cargo features (#185) – stage/README.md |
| `scene-features` | bin | `deck-scene-features scenes/*.toml`: the deck-stage features a set of scenes needs (a scene-driven build, #185) |
| `link` | bin + lib | `deck-link`, the deck's identity and access (CYBERDECKS.md §9, #180): an Ed25519 key made at first boot in `/var/lib/deck/identity`, the request with the code for the splash, polling until a member approves it on scutoons.com/account/decks, ~1 h tokens from a signed challenge, renewed by itself; `deck-link id` / `deck-link pair` for the boot flow and tests. Server side: server/docs/API.md "Decks" |
| `bridge` | bin | loads a TOML venue patch (B.5.4), builds + starts the adapters, joins the relay as `bridge`, measures the clock offset (median of 5 pings), sends `devices { ports }`, queues `device` messages until their `at`, `panic` = queue cleared + every adapter silenced (MIDI CC 120 / 123), forwards device input as `device.in`. Not yet: the status window, dmx adapters |

`dmx-*`, `gpio`, `audio`, `video-in`, `display`, `laser` (CYBERDECKS.md §4, #178) do not exist yet – the bridge names them
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
  relay/              the relay binary + tests/routing.rs (a real server, three roles, two groups)
  bridge/             the bridge binary + venue.example.toml
  link/               deck-link: src/ (identity, link, http) · tests/link.rs (a fake server)
  core/               deck-core: the platform-free core (also wasm32) · src/stage.rs (the stage's modules)
  scene-features/     deck-scene-features: scenes → cargo features
  stage/              deck-stage: src/ (graph, modules, audio, render, library) · scenes/ · tests/no_alloc.rs
shared/live/          @scutoons/live: src/protocol.ts (generated), test/ (fixture + generator tests)
```
