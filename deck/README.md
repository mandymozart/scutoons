# deck/ – the SCUTOONS deck suite (Rust)

The native half of the platform: the Live relay and device bridge today, audio / video / 3D later
(agents/research/CYBERDECKS.md – the plan; LIVE.md B.3, B.4, B.6, B.7 – what relay and bridge do; #172 – the work).
Owner 2026-10-29: Rust first, no Node + Electron for anything native.

## Crates

| Crate | Kind | Does |
|---|---|---|
| `protocol` | lib | the B.6 envelope + message kinds as serde types, **generated** from `protocol/schema.json` (below) |
| `adapters/core` | lib | the `Adapter` trait (start / send / stop / ports), `DeviceConfig`, the bus events the default maps translate |
| `adapters/midi` | lib | `midir` in / out + the **default map** of B.3 (slice enter = note on ch 16, event types per channel, variables as CC) |
| `relay` | bin | WebSocket server: sessions by code, roles + groups from the URL / `hello`, stamps `seq` + `from`, routes by `to`, `ping` → `pong`, `GET /status`. Not yet: snapshot / replay, votes, clock lead, HMAC tokens, rate limits (each a `TODO(B.6 …)` in `relay/src/lib.rs`) |
| `bridge` | bin | loads a TOML venue patch (B.5.4), builds + starts the adapters, joins the relay as `bridge`, sends `devices { ports }`, plays `device` messages at their `at`, forwards device input as `device.in`. Not yet: the status window, osc / dmx adapters, a measured clock offset |

`osc`, `dmx-*`, `gpio`, `audio`, `video-in`, `display` (CYBERDECKS.md §4) do not exist yet – the bridge names them
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
`welcome`. Conductor, bridge and uplink need a `token` in `hello` (any non-empty string until the HMAC check lands).
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
- [ ] every acceleration path has a software fallback that is chosen, never crashed into (none yet – audio / video come
      with phase 3)
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
  relay/              the relay binary + tests/routing.rs (a real server, three roles, two groups)
  bridge/             the bridge binary + venue.example.toml
shared/live/          @scutoons/live: src/protocol.ts (generated), test/ (fixture + generator tests)
```
