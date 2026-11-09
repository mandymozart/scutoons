Status: built 2026-11-08 on branch feature/438-relay-transports (the owner chose QUIC for every client, WebSocket last); off until a relay is started with --quic and a client names the door; tested on one Linux host under `tc netem`, on Windows localhost and in a real Chromium, not over the internet or on bunny (#438).

# Relay transports: TCP, QUIC, RUDP

The owner asked for RUDP for the Rust applications, QUIC for browsers, a bridge between them, and what running them in parallel costs (agents/decisions/deck.md, 2026-11-08 `[live]`). Short answer: one relay can serve all of them in the same session, it costs almost nothing, and QUIC does the job for native decks as well, so a separate RUDP door does not earn its place.

## Decision

The owner chose QUIC for every client, native and browser, through one WebTransport door, WebSocket last (2026-11-08, "Quic it is"). RUDP measured no better than QUIC (below), and QUIC is needed for browsers anyway.

| Path | Now | Before it goes live |
|---|---|---|
| Hosted (bunny) | built, off | an owner-run test on a real Anycast endpoint: UDP through Anycast, TLS at the relay and certificate renewal are untested |
| Venue LAN | built, off | a test on real venue Wi-Fi: with no loss every transport gives p99 ≈ 11 ms; `netem` loss is random, Wi-Fi loss comes in bursts |

## How it was measured

Linux 6.18 (Docker Desktop's WSL 2 kernel), everything on `lo` in a throw-away container: `netem delay 5ms` each way (RTT 10 ms), loss per packet, real TCP and real UDP. The load is that of `examples/bench.rs`: 20 decks at 30 Hz to the conductor, 100 phones getting 5 Hz cues, or 50 · 500. Medians of 3 × 30 s. Rerun: `deck/relay/netem/run.sh` (`baseline`, `doors`, `mixed`, `control`, `cost`, `limits`, `udp-blocked`, `ladder`). `examples/doors_bench.rs` counts only runs in which the joins are done, and CPU only in the measured window.

Limits of this method: one host, no real Wi-Fi or mobile network, random rather than bursty loss, loopback MTU (a 1.3 KB datagram fits here). The first matrix overlapped another session's CI; the control rows on the idle machine gave the same numbers. The cue p99 sits at a knee (1–2 % of cues are delayed by a loss), so it swings between runs; the differences below that are under about 10 ms are noise.

## Results at 1 % loss

Pass mark: p99 < 50 ms. 20 decks · 100 phones, idle-machine control rows.

| Transport | values p50 / p99 / max | arrived | hold p99 | cues p50 / p99 / max | relay CPU | wire KB/s |
|---|---|---|---|---|---|---|
| TCP / WebSocket | 10.2 / 29.0 / 263.4 | 98.9 % | 86 | 10.7 / **220.6** / 439.7 | 5.4 % | 414 |
| QUIC raw (`quinn`) | 10.2 / 10.8 / 19.4 | 97.9 % | 77 | 10.7 / **57.3** / 138.1 | 11.0 % | 432 |
| QUIC WebTransport (`wtransport`) | 10.3 / 14.6 / 19.5 | 98.0 % | 77 | 10.8 / **58.9** / 109.8 | 10.1 % | 419 |
| RUDP (`renet2`, 40 ms resend) | 10.3 / 10.9 / 14.6 | 98.0 % | 77 | 11.2 / **51.2** / 92.3 | 12.5 % | 504 |

`hold` = how old the value on the conductor's screen is when the next one replaces it; it is the fair comparison, because an unreliable lane never counts what it lost. All ms.

- **Real TCP against the old model:** values p99 is 26–31 ms in the untouched `bench`, not the modelled 190 ms, and `doors_bench` agrees (29.0). So TCP passes on values. The problem is cues: 21 / 221 / 223 ms in three runs of the untouched `bench`, thin streams waiting for the 200 ms RTO. Its one-off stalls are visible in the max (263 ms).
- **Every UDP transport** passes easily on values and removes TCP's quarter-second stalls. Cues are still just over the mark at default settings (51–59 ms, against 221 on TCP) and under it when tuned: `renet2` with a 20 ms resend 33.6 ms (CPU 19.7 %), `quinn` with a 5 ms ack delay 38.8 ms (CPU 18.3 %). A browser controls its own ack delay, so WebTransport's relay → phone direction probably cannot be tuned this way (not tested). `renet2` at its default resend class of 300 ms: cues 311.8 ms.
- **5 % loss** (values / cues p99): TCP 184.5 / 245.1 ms (the untouched `bench`); `quinn` 22.8 / 62.5, WebTransport 17.0 / 61.7, `renet2` 12.7 / 65.0.
- **Real browser** (Chromium 156 headless, serverCertificateHashes, in the same netem'd namespace, as an extra conductor and phone beside the load): WebTransport values p99 14.2, cues 59.0 (151 cues, so p99 is about the max); same browser on WebSocket values 32.6, cues 220.8.

## Running them in parallel: what it costs

One relay, one session, 1 % loss. "mixed" = decks and phones spread evenly over RUDP / QUIC / TCP, conductor on QUIC. "target" = native decks on RUDP, browser decks and phones on QUIC. Relay and clients in separate processes; egress counted by iptables, IP headers and retransmits included. Cost for a 3-hour show at bunny's rates ($0.02 per CPU-hour, $0.005 per GB-hour, $0.01 per GB egress in Europe).

| Layout | size | values p99 | cues p99 | CPU | RAM / client | egress KB/s | GB / 3 h | $ / show |
|---|---|---|---|---|---|---|---|---|
| TCP alone | 20 · 100 | 30.0 | 220.6 | 5.9 % | 52 kB | 242 | 2.5 | 0.029 |
| QUIC alone | 20 · 100 | 15.0 | 60.1 | 10.2 % | 122 kB | 255 | 2.6 | 0.033 |
| RUDP alone | 20 · 100 | 10.9 | 51.8 | 12.8 % | 43 kB | 239 | 2.5 | 0.032 |
| mixed | 20 · 100 | 11.6 | 51.0 | 11.9 % | 74 kB | 251 | 2.6 | 0.033 |
| target | 20 · 100 | 11.4 | 58.5 | 11.0 % | 117 kB | 246 | 2.5 | 0.032 |
| TCP alone | 50 · 500 | 25.5 | 221.1 | 12.9 % | 27 kB | 840 | 8.7 | 0.095 |
| QUIC alone | 50 · 500 | 16.4 | 43.1 | 25.5 % | 99 kB | 920 | 9.5 | 0.111 |
| RUDP alone | 50 · 500 | 29.6 | 55.1 | **52.1 %** | 20 kB | 831 | 8.6 | 0.117 |
| mixed | 50 · 500 | 17.1 | 51.6 | 36.1 % | 51 kB | 876 | 9.0 | 0.112 |
| target | 50 · 500 | 16.3 | 49.4 | 19.6 % | 96 kB | 845 | 8.7 | 0.100 |

Plus the Anycast IP, $2 a month even when idle. Egress is 75–85 % of a show's bill; RAM is under a tenth of a cent.

**One door does not hurt another.** Each door's values p99 in the mixed run against the same door alone: RUDP 11.2 (alone 10.9), QUIC 11.1 (15.0), TCP 20.3 (30.0) at 20 · 100; RUDP 16.3 (29.6), QUIC 16.0 (16.4), TCP 20.3 (25.5) at 50 · 500. The conductor is on QUIC in the mixed run and on the door itself when alone, so TCP's better number there comes from the conductor's link. Parallel doors cost a little CPU and no latency. The relay's CPU limit is what should size the hosted container: `renet2`'s single task at 50 · 500 takes half a core.

## RUDP against QUIC

| | RUDP (`renet2`) | QUIC |
|---|---|---|
| values / cues p99, 20 · 100 | 10.9 / 51.2 | 10.8 / 57.3 (`quinn`) |
| CPU, 50 · 500 | 52.1 % | 25.5 % |
| wire, 20 · 100 | 504 KB/s (+17 %) | 432 |
| join, nothing blocked (p50) | 38 ms | 43 ms |
| 2 Mbit bottleneck: cues p99 | 4270 ms | 2440 ms (TCP: 1220) |
| congestion control | none: a fixed retransmit timer per path | yes, per connection |
| simultaneous join under loss | **1–7 % of clients time out** (below) | none in any run |
| adapter size | 287 lines | 266 lines (both QUIC doors) |
| new crates | 111, of which 34 not shared with QUIC | 88 |
| UDP mappings | one more port | one |

The cue difference is inside the noise, and RUDP loses on CPU, bytes and behaviour at a bottleneck. It does not beat QUIC by enough to justify a separate door.

**Open defect:** when 121 or 551 clients connect at the same moment, some `renet2` clients end with "connection timed out during response step" (netcode's handshake): 1–2 % of phones at 1 % loss, 5–7 % at 5 %. It did not happen with 31 simultaneous clients nor in 40 of 40 sequential joins at 1 %. Root cause not found; it is either in `renetcode2` or in how this prototype drives it. The ladder would carry such a client to the next rung.

**Maintenance:** `renet2` 1.0.0 was last updated 2026-10-16, with about 5 000 recent downloads on crates.io; `quinn` about 148 million, `wtransport` about 540 000. `renet2_netcode` still pins `wtransport` 0.6, while the current release is 0.7.2, and its default path pulls in `aws-lc-rs` next to the relay's `ring`, which needs `install_default` to avoid a rustls panic.

## One library for both worlds?

- `renet2`'s server really does hold UDP and WebTransport sockets on one netcode server (test `renet2_runs_udp_and_webtransport_sockets_on_one_server`). But its WebTransport client exists only for wasm32, so a browser has to run the renet2 wasm client; that path was not measured. `WebTransportServer::new` blocks and may not be called on a runtime thread.
- `wtransport` (on `quinn`) serves browsers and native Rust clients over the same door, one UDP port, one code path. That is the recommended single library. Raw `quinn` would be a second port: `wtransport`'s endpoint cannot share one with another ALPN.
- ENet and KCP were not built: neither protocol defines encryption of its own, neither can be opened from a browser, and the Rust bindings are small (`rusty_enet` 0.4.1, about 19 000 recent downloads; `kcp` 0.6.0; `tokio_kcp` 0.9.8 last updated 2024).

## Security

The browser's constraint is that it cannot open raw UDP, so WebTransport is required for that reason, not for encryption: RUDP over the internet has to be encrypted as well.

- **QUIC / WebTransport:** TLS 1.3 from the first flight. `hello.token` travels unchanged inside the encrypted stream. Needs a certificate the client trusts.
- **`renet2` / netcode:** `ServerAuthentication::Secure` seals a connect token with a key shared with whoever mints it, and encrypts every packet with ChaCha20-Poly1305. The benches ran in this mode. `Unsecure` sends everything in clear and must never go near the internet. It requires a new step: every client first fetches a connect token over HTTPS, so the API needs the netcode key and token format.
- A `resume` is authenticated only by the client id (visible to others in presence). This already applies to the WebSocket path; a resume secret belongs in the adoption work.

## What was built

The owner chose QUIC for every client on 2026-11-08 ("Quic it is"). The RUDP and raw `quinn` doors of the comparison are gone; they are in git at `a31148de` (`deck/relay/src/doors/renet.rs`, `ladder.rs`). Every number above comes from that comparison.

- **Relay:** `src/doors/` joins the same `Relay` as WebSocket, with the same `hello` checks. `--quic ADDR` / `RELAY_QUIC_ADDR` opens one WebTransport door under the `--tls-*` certificate, reloaded when the files change like the wss port. Without it no UDP port is opened.
- **Lanes:** `deck.values` go unreliable, latest value wins (`Outbox::take_lanes`). Everything else stays reliable and ordered. The door's writer waits for the stream as the WebSocket writer waits for its flush, so what a stalled stream cannot take stays in the outbox under `MAX_WAIT`, `MAX_QUEUED` and the rate steps; while it waits the datagrams wait too, which only a client past a full flow-control window ever sees. A datagram carrying anything but `deck.values` is ignored. The framing and `DATAGRAM_MAX` are in `protocol/src/lines.rs`, once.
- **Settle:** every second each door conductor gets the values of the decks that changed since the last settle on the reliable lane (a conductor new to the door: all of them). A deck does the same upward, in the place of its next change (the relay allows a deck 30 `deck.values` a second), numbered by its last datagram: the door keeps a key from a late settle only if no newer datagram has said it. A receiver applies a key only if the frame's `seq` is not older. A datagram never moves the resume's `seq`.
- **The choice of socket** is in the core client (`deck/space/src/relay.rs`, RULES A1); the native adapter (`deck/native/src/live/quic.rs`, `socket.rs`) and the browser adapter (`shared/live-deck/src/client.ts`) only dial what it says.
- **The HUD line** ends in `QUIC` on QUIC; the relay logs `through the quic door`.

The door as built, under the same `tc netem` (20 decks, 100 phones, median of 3 × 30 s, ms; its clients are the door's own `wtransport`, which honour the 5 ms ack delay the door asks for; a browser does not):

| loss | door | values p50 / p99 / max | arrived | cues p50 / p99 / max | relay CPU |
|---|---|---|---|---|---|
| 0 % | WebSocket | 10.2 / 10.4 / 11.9 | 100 % | 10.9 / 11.5 / 11.8 | 4.1 % |
| 0 % | QUIC | 10.3 / 10.6 / 11.0 | 99.9 % | 10.8 / 11.3 / 11.8 | 7.9 % |
| 1 % | WebSocket | 10.2 / 29.1 / 265.7 | 98.9 % | 10.9 / 220.6 / 440.9 | 4.2 % |
| 1 % | QUIC | 10.3 / 14.3 / 19.4 | 97.9 % | 10.7 / **11.4** / 76.7 | 9.7 % |
| 5 % | WebSocket | 15.6 / 199.5 / 755.7 | 89.7 % | 10.9 / 249.3 / 1426.8 | 4.1 % |
| 5 % | QUIC | 10.3 / 16.7 / 27.5 | 90.1 % | 10.8 / 60.6 / 185.7 | 10.5 % |

The writer waits for the stream since the first run of this table (11.4 ms was 40.7, 60.6 was 51.6). At 1 % loss about 1 % of the cues are delayed, so the p99 sits on the edge between delayed and not: read the 11.4 with the max (77 both times), not as a gain from the change.

Checked end to end as well: the deck's real socket thread against the real relay (`deck/native/tests/live_quic.rs`), and Chromium 152 through `client.ts` and the wasm core (a conductor and a deck on QUIC, values as datagrams, a dial refused to the WebSocket in 61 ms).

Size: the relay's doors 335 lines, the native QUIC adapter 123, the framing 80 (with tests). The relay's build goes from 77 crates to 151. Retransmission, congestion and crypto stay in the libraries.

## The fallback

`deck/space/src/relay.rs`. QUIC gets 250 ms, then a WebSocket is dialled beside it; only the winner says `hello`, so a race leaves no ghost client. A browser with no WebTransport or a refused dial goes to the WebSocket at once (61 ms in Chromium). When QUIC dies mid-show the connection's idle timeout (3 s) ends it, the client resumes on the WebSocket as the same client, values are gone for about 3 s and no cue is lost. On the WebSocket it tries QUIC again after 60 s, doubling to 5 minutes; a step up closes the WebSocket and says `hello` at once. A `resume` takes over a link the relay has not seen go yet: the old link is ended, and its own leave finds the new one in its place and leaves it alone.

The numbers below are from the prototype's ladder (RUDP > QUIC > WebSocket and QUIC > WebSocket); the built one is the second.

Time to `welcome`, 1 % loss, 40 joins:

| Stack | Blocked | race p50 / max (ms) | in turn p50 / max (ms) |
|---|---|---|---|
| native (RUDP → QUIC → WS) | nothing | 38 / 289 | 38 / 290 |
| | RUDP's port only | 296 / 364 | 1045 / 1066 |
| | all UDP | 534 / 2040 | 2034 / 2245 |
| browser (QUIC → WS) | nothing | 44 / 56 | 44 / 1032 |
| | all UDP | 283 / 509 | 1032 / 1242 |

**Race, with a 250 ms head start per rung.** Trying in turn makes a blocked rung cost a second; once in 40 joins it also pushed a healthy client down to WebSocket.

**The middle rung** (for native decks, only if RUDP is kept) earns its place when RUDP's port is filtered and UDP 443 passes: joined in 296 ms and on QUIC instead of TCP. With the recommended design the ladder is just QUIC → WebSocket.

**UDP blocked outright** (`-p udp -j DROP`): joining as above; then the WebSocket numbers: values p99 29.0, cues 220.6, max 444.4 ms at 1 % loss. That is the known worst case.

**Stepping down mid-show** (UDP dropped after a good start, 3 runs): a phone and a deck on the ladder ended on WebSocket, the phone with the **same client id and no missing cue** (replay through the existing resume), the conductor's values gone for **2.2 s**. The design gives 2.1–3.1 s: 2 s of silence, then the hello waits until the relay has let the old connection go (3 s door idle + 0.1 s), because `Session::resume` refuses a client still attached. Letting a resume take over a silent connection would cut this to about 2.3 s at most, and needs a small change on the WebSocket path. A door idle of 3 s is short for a Wi-Fi hiccup; with resume that costs little.

**Stepping back up:** after 60 s on a lower rung the client dials the rung above in the background; if it comes up, it switches by resume, otherwise it backs off, doubling up to 5 minutes. Only the state machine is tested; the switch over a real network is not.

## The adaptive rate on UDP

`link.rs` reads backpressure, which a datagram never gives: `flush` is always zero on a door. What is left is the smoothed round trip, and a door feeds the transport's own RTT (QUIC's estimator, netcode's) into `Link::rtt` once a second, the same input as the numbered ping's. Enough for a queue that builds: on a shared 2 Mbit path every transport stepped to 5 values / s. It would not see loss without queueing; `quinn`'s `stats().path.lost_packets` and renet's `packet_loss` are the obvious second signal (not built). In that test the reliable lane still suffered on every transport (cues p99: TCP 1.2 s, QUIC 2.4 s, RUDP 4.3 s; the TCP clients of this row still had Nagle on), so a saturated uplink stays a problem the rate alone does not solve.

## `deck.values` and the datagram limit

A datagram holds a little under 1200 bytes on any path (QUIC's guaranteed minimum; path discovery may raise it). A key costs roughly 21 bytes, so **about 50 keys fit one datagram**. A frame that does not fit goes on the reliable lane, with TCP-like behaviour for that frame: at 100 keys (≈ 2.1 KB) `quinn` values p99 is 50.1 ms instead of 10.8; WebTransport 30.0. `renet2` fragments it and loses twice as often (95.9 % arrived). Merged frames from a paced link are the ones at risk. On adoption: split a frame by key into several self-contained datagrams (each carries its `seq`). Not built.

## The bunny path

From bunny's documentation, not tested:

- **Anycast endpoint:** IPv4 only, $2 a month, billed while attached. `portMappings` take `containerPort`, `exposedPort` and `protocols` (`tcp`, `udp`, `sctp`), several per endpoint. So one endpoint can map UDP 443 → the WebTransport door. It "reaches your container without the edge in the path".
- **TLS moves to the relay:** today bunny's CDN endpoint terminates TLS. WebTransport needs a certificate at the relay: a hostname with an A record on the Anycast IP (say `live-udp.scutoons.com`), and a certificate with renewal. Renewal needs DNS-01 (HTTP-01 and TLS-ALPN-01 need TCP on that IP). The sessions are in memory, so the certificate has to be reloaded, not the container restarted: `quinn::Endpoint::set_server_config` and `wtransport`'s `reload_config` exist for that (not tried).
- **WebSocket stays on the CDN endpoint** as it is, so the fallback does not depend on the new path.
- **Open, to be tested on a real endpoint:** whether a one-region app (min = max = 1) gets the same address from every PoP, what happens to UDP flows at a container restart, NAT timeouts, MTU through Anycast encapsulation, whether the client source address is preserved (QUIC and netcode key on it; QUIC can migrate).
- **Venues that block UDP:** the ladder reaches WebSocket in about 0.3–0.5 s, numbers above.

**`owner` steps** (none done here): create an Anycast endpoint on `scutoons-live` with UDP 443 → the door's port; the hostname and its certificate; put the door into the deployed image; test from a phone and a venue network.

## Open

- Branch `chore/437-tungstenite-0-30` changes `Frame.text` and `Outbox::take` to `Utf8Bytes`; whichever merges second has to follow, and `Cargo.lock` will conflict.
- Frames over about 50 keys take the reliable lane; no per-key splitting.
- The adaptive rate on QUIC sees the round trip only, no loss signal.
- A `resume` is authenticated only by the client id; every QUIC client now resumes.
- Safari's WebTransport was not tried; a browser without it goes to the WebSocket.
- Owner steps before the hosted path (issue #438): an Anycast endpoint with UDP 443 to the door's port, a hostname and a certificate at the relay (DNS-01 renewal), `RELAY_QUIC_ADDR`, `VITE_LIVE_QUIC`, a test from a phone and a venue network.
