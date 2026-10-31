Status: research 2026-10-31 – issue #222 (owner: would a real-time game-state backend beat our WebSocket relay once it is bidirectional and decentralised, #221?) – game state only, assets stay on our DB + CDN; nothing built.

# Real-time shared state: market backends vs our relay

**The system the owner saw** is almost certainly **SpacetimeDB** (Clockwork Labs): "a database that is the server" –
Rust / C# / TS / C++ modules (reducers = transactions) run inside an in-memory DB with a commit log, clients
subscribe to SQL queries and get row diffs pushed; BitCraft runs on it.

## Baseline: what we have (deck/relay, LIVE.md B.4 / B.6)
- One Rust process (tokio + tungstenite), sessions by code, ~3 MB static image; on a deck in the venue LAN (ws, or wss
  with a `*.lan.scutoons.com` cert, #201) and hosted on bunny Magic Containers (`wss://live.scutoons.com`, #199).
- **Authoritative router:** roles + groups, `seq` + `from` stamped by the relay, routed by `to`, conductor-only kinds,
  audience 5 msg/s, ≤ 1000 clients per session.
- **State:** a snapshot per role (last `state`, cell, `deck.graph`, `scene.sync`, `deck.synced`, folded `deck.values`),
  replay on `resume` (500 messages / 5 min), votes with a 250 ms tally, the clock's lead (`at` + 50 ms LAN / 150 hosted).
- **Streams:** `deck.values` ≤ 30 / s per deck, only what changed, folded into the snapshot and never replayed.
- **Wire:** JSON text frames (envelope `v, seq, from, kind, p`), ping 1 s (its pong: the round trip, #223), idle 15 s.
- **Not yet:** the uplink LAN ↔ hosted (linked mode), several conductors (#221).
- **Found while reading, fixed by #223:** the relay set no `TCP_NODELAY` on accepted sockets, and each client's writer
  was an unbounded queue of strings (under Wi-Fi loss stale values piled up behind the stall) – now NODELAY, an outbox
  where the latest value wins, an adaptive rate per client (deck/relay/README.md, with the bench).

## The market
| System | State model | Transport / latency | Offline LAN show | Self-host (deck / bunny) | Rust | License / cost | Fit for #221 |
|---|---|---|---|---|---|---|---|
| **SpacetimeDB 2.x** | authoritative DB; reducers = ACID transactions; query subscriptions (row diffs) | WebSocket, binary (BSATN) or JSON; every write is a transaction (group-commit fsync; "confirmed reads" can be turned off) | yes, standalone binary runs locally | yes (single node; Docker / binary) – no built-in replication between instances | server modules in Rust; Rust client SDK (native); TS client for browsers | **BSL 1.1**: free only with **≤ 1 production instance** and no DB-as-a-service; AGPL from 2031-10-25. A LAN instance per venue + a hosted one = an enterprise licence | strong for "who owns which key" (tables + reducers enforce it); the LAN ↔ hosted link we'd still build ourselves |
| **Nakama** (Heroic Labs) | authoritative match loop (Go / TS / Lua) or relayed; plus accounts, chat, storage | WebSocket or rUDP | yes | yes, but needs Postgres / CockroachDB beside it | no Rust SDK | Apache-2.0; Heroic Cloud paid | a whole game backend (accounts, leaderboards) we don't need; heavy for a deck |
| **Colyseus** | authoritative rooms; `@colyseus/schema` binary delta sync | WebSocket | yes | yes (Node.js) | no Rust | MIT; cloud paid | good delta model, wrong runtime (Node; owner: Rust for native parts) |
| **Photon** (Realtime / Fusion / Quantum) | relay rooms / state-transfer with prediction / deterministic lockstep | UDP (rUDP), WebSocket for WebGL | Photon Server only, Windows, licence file **must reach the licence server at start** | SDK not redistributable | no | per-CCU subscription | Unity / Unreal-centric; licence model rules out a deck |
| **Convex** | reactive DB, TS functions, subscriptions | WebSocket | self-hosted yes | yes (FSL → Apache after 2 y) | Rust client exists; server TS | FSL-1.1-Apache-2.0 | app sync, not 30 Hz values; JS runtime |
| **Supabase Realtime** | broadcast + presence + Postgres changes | WebSocket (Phoenix) | with a full Supabase stack | Elixir + Postgres | community | Apache-2.0 | fan-out only, no authority |
| **Firebase RTDB / Ably / Pusher / Liveblocks** | JSON tree / pub-sub channels / CRDT rooms | WebSocket | **no** (SaaS; Liveblocks' AGPL server (Feb 2026) is a dev server) | no | partial | per message / connection | fail "the show runs with only the local relay" |
| **PartyKit / Durable Objects** | one stateful object per room | WebSocket (hibernation) | no (Cloudflare only) | no | Workers in Rust/wasm | usage-based | a good hosted-room model, but SaaS – our bunny relay already is that |
| **Yjs (yrs) / Automerge / Loro** | CRDTs: every peer merges, no authority | any (we'd carry them on the relay) | yes | library, not a server | **Rust cores** (yrs, automerge, loro), wasm | MIT | for shared *documents* (setlists, patches edited by several conductors); overkill for 30 Hz knobs (LWW per key is enough) |
| **NATS** (+ JetStream) | subjects pub/sub, optional log / KV | TCP / WebSocket for browsers | yes; **leaf nodes** keep a site working when the uplink drops, then reconnect | Go binary ~20 MB, fine on a deck or a container | async-nats | Apache-2.0 | a ready mesh LAN ↔ cloud; no roles / snapshot / votes – we'd keep our relay logic on top |
| **Redis Streams** | log + pub/sub | TCP | yes | yes | yes | AGPL / RSAL / SSPL (8+; Valkey = BSD) | a broker for servers, not for browsers |

**Transports** (independent of the backend):
- **WebTransport** (QUIC, HTTP/3): reliable streams + unreliable datagrams, no head-of-line blocking across streams.
  Baseline in all browsers since Safari 26.4 (March 2026); Safari may fall back to HTTP/2 without datagrams
  (`requireUnreliable`). Rust server: `wtransport` (author: "not completely production-ready"). Needs a real cert
  (we have `*.lan.scutoons.com`); on bunny only via an Anycast UDP endpoint (QUIC unverified).
- **WebRTC data channels:** unordered / unreliable, peer to peer or deck ↔ relay; Rust `matchbox` (signalling + wasm)
  / `str0m`. Needs signalling (our relay could do it) and ICE.
- **Binary encoding** (MessagePack / CBOR / bincode, or schema-delta like Colyseus): 2–5× fewer bytes than JSON; matters
  for fan-out to hundreds of phones, not for a few decks.

**Netcode patterns that apply:** authoritative server (we have it); state over events for continuous values (latest
value per key, drop the stale ones – we fold already); snapshot + delta (our snapshot / replay); interpolation of
received values over ~1–2 frames on the outputs (smooths jitter, costs ~30–60 ms); client prediction only for the
local performer's own knob (show it at once, reconcile with the relay's echo). Lockstep (Quantum) doesn't fit: decks
are not deterministic simulations.

## What actually limits us
- **Throughput is not it.** 20 decks × 30 values/s × ~300 B JSON ≈ 180 KB/s – trivial for one tokio process. Fan-out of
  values to 1000 phones at 30 Hz (30 k msg/s, ~9 MB/s) would hurt – but phones need cues and votes, not raw knobs.
- **Latency on a wired LAN:** WebSocket over TCP is sub-millisecond per hop; the database systems add a transaction /
  fsync on top. No backend makes the relay *faster* – SpacetimeDB is fast *for a database*, our relay isn't one.
- **The real risk is Wi-Fi loss:** one lost TCP segment stalls everything behind it until the retransmit
  (Linux min RTO 200 ms, more under bursts) – head-of-line blocking. With an unbounded writer queue, stale knob values
  then arrive in a burst. Plus Nagle (no `TCP_NODELAY`) can add tens of ms to small frames.
- **The hard part of #221 is semantics, not transport:** who owns which key, overrides, a time base across relays,
  LAN ↔ hosted when the internet drops. No product solves that for us; SpacetimeDB gives a nice way to *write* the rules,
  NATS gives the *mesh*, neither gives both, and SpacetimeDB's licence forbids the per-venue instance model.

## Recommendation
1. **Keep the relay as the authority** and extend it (it already is a small, purpose-built game server in Rust, runs
   offline, ~3 MB). Cheap fixes first: `TCP_NODELAY` on accept; per-client writer that **coalesces** `deck.values`
   (latest per deck / key) instead of queueing; drop a client whose backlog stays over a bound.
2. **State model for #221:** an **owner per key / group** (a conductor of group X owns `X.*`; the relay rejects or routes
   others as requests), last-writer-wins with the relay's `seq` as the tiebreak, an explicit `baton` hand-over message.
   CRDTs (yrs / Loro) only for shared documents several conductors edit (setlists, patches), carried as relay messages.
3. **Relay ↔ relay:** build the uplink as our own federation (the relay is the authority for its LAN; the hosted one
   mirrors the audience) – keep NATS leaf nodes in mind if the mesh grows beyond LAN + one hosted relay.
4. **Fast values later, only if the bench says so:** an unreliable side channel for continuous values – WebTransport
   datagrams (browser + native, one server) before WebRTC (ICE, signalling). Cues, votes, snapshot stay on the
   reliable WebSocket. Binary envelope (MessagePack) only if the hosted fan-out grows.
5. **SpacetimeDB:** not as the live backend (BSL ≤ 1 production instance, a DB transaction per 30 Hz value, no
   LAN ↔ hosted replication). A **½-day spike is optional** as design inspiration (tables + reducers + subscriptions as the
   shape for "who owns what") – not a dependency.

## Measurements before deciding (a relay bench)
- `deck/relay` bench binary: N simulated decks (10 / 50 / 200) at 30 Hz + 1 conductor + M audience (100 / 1000),
  on the LAN relay (X1 deck) and the hosted one. Record **p50 / p99 / max** deck → conductor latency (relay timestamps,
  clock offset as the bridge measures it), CPU / RAM, bytes / s.
- The same over Wi-Fi with **induced loss** (`tc netem loss 1% / 5%`, delay 20 ms ± 10) – with and without
  `TCP_NODELAY` and coalescing; then a WebTransport-datagram prototype for `deck.values` against it.
- Pass mark (proposal): p99 deck → conductor < 50 ms on the LAN Wi-Fi at 1 % loss, no stale bursts.

## Sources
[SpacetimeDB repo / README](https://github.com/clockworklabs/SpacetimeDB) ·
[SpacetimeDB licence (BSL 1.1, ≤ 1 production instance, AGPL 2031)](https://raw.githubusercontent.com/clockworklabs/SpacetimeDB/master/LICENSE.txt) ·
[SpacetimeDB pricing](https://spacetimedb.com/pricing) ·
[SpacetimeDB standalone](https://spacetimedb.com/docs/deploying/spacetimedb-standalone) ·
[SpacetimeDB WebSocket protocol](https://spacetimedb.com/docs/ws) ·
[Nakama](https://github.com/heroiclabs/nakama) · [Nakama vs Photon](https://heroiclabs.com/comparison/photon/) ·
[Colyseus schema](https://docs.colyseus.io/state/schema) ·
[Photon Server licences](https://doc.photonengine.com/server/v5/operations/licenses) ·
[Convex self-hosting](https://docs.convex.dev/self-hosting) ·
[Liveblocks open-sources its server (AGPL, Feb 2026)](https://liveblocks.io/blog/open-sourcing-the-liveblocks-sync-engine-and-dev-server) ·
[Durable Objects WebSockets](https://developers.cloudflare.com/durable-objects/learning/websockets) ·
[PartyKit hibernation](https://blog.partykit.io/posts/partyserver-api/) ·
[NATS WebSocket](https://docs.nats.io/learn/websocket/) · [NATS leaf nodes](https://docs.nats.io/reference/protocols/leafnode) ·
[WebTransport baseline (2026)](https://webrtc.ventures/2026/04/webtransport-is-now-baseline-what-it-means-for-real-time-media/) ·
[wtransport](https://github.com/BiagioFesta/wtransport) · [matchbox](https://github.com/johanhelsing/matchbox) ·
[bunny Magic Containers endpoints (Anycast UDP)](https://bunny.net/docs/magic-containers/endpoints)
