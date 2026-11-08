# deck/relay – the Live relay

Sessions, roles, groups, the B.6 envelope (agents/research/LIVE.md B.4, B.6). Run, wss, the hosted image:
deck/README.md "Relay". The rules of a session: `src/session.rs`; the sockets: `src/lib.rs`.

## Send path (#223 – "performance and stability is everything")

- **TCP_NODELAY** on every accepted socket (ws and wss): no Nagle wait for the ACK of the previous frame.
- **Little unsent data in the kernel** (`UNSENT` 4 KB: `TCP_NOTSENT_LOWAT` on Linux, an 8 KB send buffer elsewhere), so
  a stalled client's backlog waits in its outbox, where it can still be replaced.
- **The latest value wins** (`src/outbox.rs`): ordered messages (cues, `scene.sync`, `deck.graph`, `deck.synced`,
  snapshots, welcome, votes, errors) stay complete and in order; state-like ones replace the one of their kind still
  waiting – `deck.values` per sending deck (merged key by key), `presence`, `vote.tally`, `state` – never across an
  ordered message. Serialized once per message, the writer sends a batch with one flush.
- **One buffer per frame, small reads** (#437): a frame's text is shared by every client it goes to, not copied per
  client; each connection reads through 4 KB (`READ_BUFFER`, tungstenite's default 128 KB is zeroed on every read).
- **A client too far behind is dropped** (2048 ordered messages, 8 MB, or its oldest waited 5 s): it reconnects and
  resyncs from its snapshot.
- **Adaptive rate per client** (`src/link.rs`): a link that stays behind (the socket full / messages waiting for 500 ms
  without a break) or whose smoothed ping round trip stays 80 ms over its best steps down 30 → 15 → 10 → 5 values / s
  (one step per 500 ms; values merge meanwhile); 5 s calm steps it up one level. A one-off stall (a lost segment) costs
  nothing – pacing would only add delay. Level 0 is unpaced. Pings every 1 s, numbered (the pong = the round trip).
- **The conductor sees each link:** `presence.links` = `{ <client>: { rtt, hz, skipped } }` (not the audience), sent again
  when a rate changes.

## Bench

`cargo run --release -p relay --example bench -- --decks 20 --listeners 100 --seconds 30 [--loss 1 [--rtt 10] [--lossy down]]`

The relay in-process (its own runtime – its CPU is measured alone), N decks sending `deck.values` at 30 Hz to the
conductor, M audience phones getting the conductor's cues at 5 Hz. Latency = receive − send on one clock. Without
`--loss` the clients use TCP on 127.0.0.1:7070. With `--loss` every client gets its own in-process link (Windows'
loopback buffers ~2.6 MB per socket, which would hide any backpressure): one-way `rtt / 2`, a lost segment arrives
after its recovery (`2.5 × rtt`, RACK / TLP), a lost retransmit after the 200 ms RTO, in order behind it; 16 KB in flight
per direction, then the relay's socket backs up (`--unsent` = what its kernel keeps unsent). Nothing is dropped –
real loss can't be made in-process; **the real test is Linux `tc netem`** with real TCP (RACK, TLP, the RTO):
`sudo tc qdisc add dev lo root netem delay 5ms loss 1%`, the bench without `--loss`, `sudo tc qdisc del dev lo root`
after (run 2026-11-08, below).

Results (2026-10-31, Windows 10 workstation, release build, median of 3 × 30 s; "before" = c6dc54c2 with only
`serve_stream` added, its kernel buffer modelled as 64 KB unsent – Linux autotuning without NOTSENT_LOWAT):

| decks · phones | link | values p50 / p99 / max (ms) before → after | cues p99 (ms) | values arrived (after) | lowest rate |
|---|---|---|---|---|---|
| 20 · 100 | TCP 127.0.0.1 | 0.1 / 0.3 / 2.2 → 0.1 / 0.3 / 1.2 | 1.2 → 1.3 | 100 % | 30 |
| 20 · 100 | 10 ms, 0 % | 12.8 / 15.0 / 18 → 12.7 / 14.9 / 18 | 15.6 → 15.1 | 100 % | 30 |
| 20 · 100 | 10 ms, 1 % | 13.0 / 190 / 250 → 13.0 / 188 / 255 | 38.5 → 38.7 | 99.9 % | 30 |
| 20 · 100 | 10 ms, 5 % | 28.6 / 238 / 455 → 29.6 / 235 / 265 | 39.4 → 39.2 | 82–98 % | 15 |
| 50 · 500 | 10 ms, 1 % | 13.1 / 196 / 252 → 13.1 / 196 / 257 | 39.1 → 39.1 | 97.8 % | 30 |

Relay CPU < 1 % of one core in every row (2 worker threads); the clients received 130 KB/s (20 decks) / 490 KB/s (50).

**Pass mark (p99 < 50 ms at 1 %):** cues to phones pass (39 ms); values to the conductor don't, before or after – its
downlink carries 600–1500 small frames / s, so the rare lost retransmit (p² per segment → the 200 ms RTO) lands in
almost every run and everything behind it waits. Loss on the decks' uplinks alone: p99 37 ms; on the downlinks alone:
202 ms (`--lossy up|down`, 10 s runs). What #223 buys: no stale backlog beyond the bytes already in flight (5 %: the
max 455 → 265 ms, up to 18 % of stale values never sent, the rate steps to 15 / s), drops instead of an unbounded queue,
the link in presence. What it can't: TCP's head-of-line blocking – bytes in flight can't be replaced. The model's lost
retransmit is pessimistic (RACK finds most without the RTO), so the next step is the `tc netem` run on Linux; if it
shows the same tail, the issue's "later" item (an unreliable side channel for values, WebTransport datagrams) is due.

**Real loss (2026-11-08, #438):** Linux `tc netem` on lo in a container (`deck/relay/netem/run.sh baseline`, delay 5 ms each way, 20 decks · 100 phones,
median of 3 × 30 s). The model above was pessimistic for values and optimistic for cues:

| loss | values p50 / p99 / max (ms) | cues p50 / p99 / max (ms) | arrived | lowest rate |
|---|---|---|---|---|
| 0 % | 10.3 / 10.9 / 47.9 | 10.8 / 11.3 / 14.3 | 100 % | 30 |
| 1 % | 10.3 / 28.7 / 275.7 | 10.8 / 220.8 / 420.9 | 98.9 % | 30 |
| 5 % | 16.0 / 184.5 / 766.2 | 10.7 / 245.1 / 1166.7 | 85–91 % | 5 |

At 1 % values pass the 50 ms mark; cues fail in two runs of three (21 / 221 / 223 ms): a lost segment of a thin stream waits for the RTO. Other transports,
the costs and the recommendation: agents/research/RELAY-TRANSPORTS.md.
