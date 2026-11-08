//! #438: the QUIC door – WebTransport (`wtransport`), what a browser opens and what the native decks dial too (the owner,
//! 2026-11-08: "Quic it is"). One bidirectional stream carries the reliable lane (`protocol::lines`), datagrams the
//! unreliable one. TLS 1.3 from the first flight, so `hello.token` never crosses the internet in clear.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use protocol::lines::{line, Lines};
use tokio::sync::mpsc;
use wtransport::quinn::{AckFrequencyConfig, TransportConfig};
use wtransport::{Connection, Endpoint, RecvStream, SendStream, ServerConfig};

use super::{Doors, Member, DOOR_IDLE};
use crate::outbox::{Lane, MAX_BYTES};
use crate::tls::TlsFiles;
use crate::{parse_join, Join, HELLO_TIMEOUT, PING_EVERY};

/// The ack delay asked of the peer (quinn's default is 25 ms): it shortens the probe timeout, which is what a lost cue on
/// an idle stream waits for (cues p99 at 1 % loss: 57 → 39 ms, a peer that honours it; browsers do not).
const ACK_DELAY: Duration = Duration::from_millis(5);

/// One connection, after the transport's handshake: admit on the first line, then both lanes until it ends.
async fn pump(doors: Arc<Doors>, conn: Arc<Connection>, send: SendStream, mut recv: RecvStream, join: Join) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let queued = Arc::new(AtomicUsize::new(0));
    let stream = tokio::spawn(stream_writer(send, rx, queued.clone()));
    let mut lines = Lines::default();
    let mut chunk = vec![0u8; 16 * 1024];

    let mut waiting = tokio::time::timeout(HELLO_TIMEOUT, async {
        loop {
            let n = recv.read(&mut chunk).await?.ok_or_else(|| anyhow::anyhow!("no hello"))?;
            let got = lines.push(&chunk[..n]).map_err(|_| anyhow::anyhow!("hello too long"))?;
            if !got.is_empty() {
                return anyhow::Ok(got.into_iter());
            }
        }
    })
    .await??;
    let first = waiting.next().expect("at least the hello");
    let member = match doors.admit(&first, join) {
        Ok(m) => Arc::new(m),
        Err(refusal) => {
            tx.send(line(&refusal)).ok();
            drop(tx);
            stream.await.ok();
            return Ok(());
        }
    };
    for text in waiting {
        member.text(&text);
    }
    let writer = tokio::spawn(write(conn.clone(), member.clone(), tx, queued));

    loop {
        tokio::select! {
            got = recv.read(&mut chunk) => {
                let Ok(Some(n)) = got else { break };
                let Ok(got) = lines.push(&chunk[..n]) else { break };
                for text in got {
                    member.text(&text);
                }
            }
            d = conn.receive_datagram() => match d {
                Ok(d) => member.datagram(d.payload().as_ref()),
                Err(_) => break,
            },
            _ = member.outbox.dropped() => break, // too far behind: it reconnects and resyncs from its snapshot
        }
    }
    member.leave();
    writer.abort();
    stream.abort();
    Ok(())
}

/// The reliable lane on its own task: a stalled stream must not hold up the datagrams behind it.
async fn stream_writer(mut send: SendStream, mut rx: mpsc::UnboundedReceiver<Vec<u8>>, queued: Arc<AtomicUsize>) {
    while let Some(bytes) = rx.recv().await {
        let n = bytes.len();
        if send.write_all(&bytes).await.is_err() {
            break;
        }
        queued.fetch_sub(n, Ordering::Relaxed);
    }
    send.finish().await.ok();
}

/// One client's writer: what its outbox lets go, each message on its lane; the transport's round trip once a second.
async fn write(conn: Arc<Connection>, member: Arc<Member>, tx: mpsc::UnboundedSender<Vec<u8>>, queued: Arc<AtomicUsize>) {
    let outbox = member.outbox.clone();
    let mut batch: Vec<(Lane, String)> = Vec::with_capacity(64);
    let mut next_rtt = tokio::time::Instant::now() + PING_EVERY;
    loop {
        let now = tokio::time::Instant::now();
        if now >= next_rtt {
            next_rtt = now + PING_EVERY;
            member.rtt(conn.rtt());
        }
        let took = outbox.take_lanes(now.into_std(), &mut batch);
        if !batch.is_empty() {
            let mut reliable = Vec::new();
            for (lane, text) in batch.drain(..) {
                if lane == Lane::Reliable || text.len() > protocol::lines::DATAGRAM_MAX || conn.send_datagram(text.as_bytes()).is_err() {
                    reliable.extend_from_slice(&line(&text));
                }
            }
            if !reliable.is_empty() {
                // the same limit as the outbox's own: a client this far behind is dropped
                if queued.fetch_add(reliable.len(), Ordering::Relaxed) + reliable.len() > MAX_BYTES || tx.send(reliable).is_err() {
                    outbox.close();
                    break;
                }
            }
            if outbox.sent(Instant::now(), Duration::ZERO, took.waited) {
                member.tell_presence();
            }
            continue;
        }
        let wake = took.until.map_or(next_rtt, |t| next_rtt.min(t.into()));
        tokio::select! {
            _ = outbox.pushed() => {}
            _ = tokio::time::sleep_until(wake) => {}
        }
    }
}

fn transport() -> anyhow::Result<TransportConfig> {
    let mut t = TransportConfig::default();
    t.max_idle_timeout(Some(DOOR_IDLE.try_into()?));
    t.keep_alive_interval(Some(PING_EVERY));
    let mut ack = AckFrequencyConfig::default();
    ack.max_ack_delay(Some(ACK_DELAY));
    t.ack_frequency_config(Some(ack));
    Ok(t)
}

/// The door on `addr` (UDP), under the certificate of `files` – loaded again when the files change, as on the wss port
/// (tls.rs). The URL's path is the join (`/live/<code>?role=…`), as on the WebSocket path. Binds now (a port in use is
/// an error at once); the returned future accepts until the relay stops.
pub fn door(doors: Arc<Doors>, addr: SocketAddr, files: TlsFiles) -> anyhow::Result<impl Future<Output = ()> + Send> {
    let tls = crate::tls::quic_config(files).map_err(anyhow::Error::msg)?;
    let config = ServerConfig::builder().with_bind_address(addr).with_custom_tls_and_transport(tls, transport()?).build();
    let endpoint = Endpoint::server(config)?;
    Ok(async move {
        loop {
            let incoming = endpoint.accept().await;
            let doors = doors.clone();
            tokio::spawn(async move {
                let ended: anyhow::Result<()> = async {
                    let request = incoming.await?;
                    let join = parse_join(request.path());
                    let conn = request.accept().await?;
                    let (send, recv) = conn.accept_bi().await?;
                    pump(doors, Arc::new(conn), send, recv, join).await
                }
                .await;
                if let Err(e) = ended {
                    tracing::debug!("quic door: {e}");
                }
            });
        }
    })
}
