//! #438: the QUIC doors – raw `quinn` (ALPN `scutoons-live`) and WebTransport (`wtransport`, what a browser opens). One
//! bidirectional stream carries the reliable lane (newline-delimited, `line`), datagrams the unreliable one. TLS 1.3 from
//! the first flight, so `hello.token` never crosses the internet in clear.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use super::{line, Doors, Member, DOOR_IDLE, MAX_LINE};
use crate::outbox::{Lane, MAX_BYTES};
use crate::{parse_join, Join, HELLO_TIMEOUT, PING_EVERY};

pub const ALPN: &[u8] = b"scutoons-live";

/// Prototype knob (0 = quinn's default, 25 ms): the ack delay asked of the peer through the ack-frequency extension.
/// It shortens the probe timeout, which is what a lost cue on an idle stream waits for.
pub static ACK_DELAY_MS: AtomicU64 = AtomicU64::new(0);

/// What the pump needs from a QUIC connection.
pub trait Conn: Clone + Send + Sync + 'static {
    /// false: too large for one datagram or not allowed – the caller sends it on the reliable lane
    fn send_dgram(&self, payload: &[u8]) -> bool;
    fn rtt(&self) -> Duration;
    fn recv_dgram(&self) -> impl Future<Output = Option<Vec<u8>>> + Send;
}

impl Conn for quinn::Connection {
    fn send_dgram(&self, payload: &[u8]) -> bool {
        self.send_datagram(payload.to_vec().into()).is_ok()
    }
    fn rtt(&self) -> Duration {
        quinn::Connection::rtt(self)
    }
    async fn recv_dgram(&self) -> Option<Vec<u8>> {
        self.read_datagram().await.ok().map(|b| b.to_vec())
    }
}

/// a WebTransport session (the same shape as a raw QUIC connection)
#[derive(Clone)]
pub struct Wt(pub Arc<wtransport::Connection>);

impl Conn for Wt {
    fn send_dgram(&self, payload: &[u8]) -> bool {
        self.0.send_datagram(payload).is_ok()
    }
    fn rtt(&self) -> Duration {
        self.0.rtt()
    }
    async fn recv_dgram(&self) -> Option<Vec<u8>> {
        self.0.receive_datagram().await.ok().map(|d| d.payload().to_vec())
    }
}

/// One line from `reader` into `buf` (without the newline). None: the stream ended or the line is too long.
async fn next_line<R: AsyncRead + Unpin>(reader: &mut BufReader<R>, buf: &mut Vec<u8>) -> Option<()> {
    let room = (MAX_LINE + 1).saturating_sub(buf.len()) as u64;
    let n = (&mut *reader).take(room).read_until(b'\n', buf).await.ok()?;
    if n == 0 || buf.last() != Some(&b'\n') {
        return None;
    }
    buf.pop();
    Some(())
}

/// One connection, after the transport's handshake: admit on the first line, then both lanes until it ends.
pub async fn pump<C, W, R>(doors: Arc<Doors>, conn: C, send: W, recv: R, join: Join) -> anyhow::Result<()>
where
    C: Conn,
    W: AsyncWrite + Unpin + Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(recv);
    let mut buf = Vec::with_capacity(1024);
    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let queued = Arc::new(AtomicUsize::new(0));
    let stream = tokio::spawn(stream_writer(send, rx, queued.clone()));

    tokio::time::timeout(HELLO_TIMEOUT, next_line(&mut reader, &mut buf)).await?.ok_or_else(|| anyhow::anyhow!("no hello"))?;
    let member = match doors.admit(std::str::from_utf8(&buf)?, join) {
        Ok(m) => Arc::new(m),
        Err(refusal) => {
            let mut out = Vec::new();
            line(&refusal, &mut out);
            tx.send(out).ok();
            drop(tx);
            stream.await.ok();
            return Ok(());
        }
    };
    buf.clear();
    let writer = tokio::spawn(write(conn.clone(), member.clone(), tx, queued));

    loop {
        tokio::select! {
            got = next_line(&mut reader, &mut buf) => {
                if got.is_none() {
                    break;
                }
                match std::str::from_utf8(&buf) {
                    Ok(text) => member.text(text),
                    Err(_) => break,
                }
                buf.clear();
            }
            d = conn.recv_dgram() => match d {
                Some(d) => member.datagram(&d),
                None => break,
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
async fn stream_writer<W: AsyncWrite + Unpin>(mut send: W, mut rx: mpsc::UnboundedReceiver<Vec<u8>>, queued: Arc<AtomicUsize>) {
    while let Some(bytes) = rx.recv().await {
        let n = bytes.len();
        if send.write_all(&bytes).await.is_err() {
            break;
        }
        queued.fetch_sub(n, Ordering::Relaxed);
    }
    send.shutdown().await.ok();
}

/// One client's writer: what its outbox lets go, each message on its lane; the transport's round trip once a second.
async fn write<C: Conn>(conn: C, member: Arc<Member>, tx: mpsc::UnboundedSender<Vec<u8>>, queued: Arc<AtomicUsize>) {
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
                if lane == Lane::Reliable || !conn.send_dgram(text.as_bytes()) {
                    line(&text, &mut reliable);
                }
            }
            if !reliable.is_empty() {
                // the same limit as the outbox's own: a client this far behind is dropped
                if queued.fetch_add(reliable.len(), Ordering::Relaxed) + reliable.len() > MAX_BYTES || tx.send(reliable).is_err() {
                    outbox.close();
                    break;
                }
            }
            let done = Instant::now();
            if outbox.sent(done, Duration::ZERO, took.waited) {
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

fn transport() -> quinn::TransportConfig {
    let mut t = quinn::TransportConfig::default();
    t.max_idle_timeout(Some(DOOR_IDLE.try_into().expect("a small timeout")));
    t.keep_alive_interval(Some(PING_EVERY));
    let ms = ACK_DELAY_MS.load(Ordering::Relaxed);
    if ms > 0 {
        let mut ack = quinn::AckFrequencyConfig::default();
        ack.max_ack_delay(Some(Duration::from_millis(ms)));
        t.ack_frequency_config(Some(ack));
    }
    t
}

/// The raw QUIC door. Binds now (a port in use is an error at once); the returned future accepts until the endpoint closes.
pub fn quinn_door(doors: Arc<Doors>, addr: SocketAddr, chain: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> anyhow::Result<impl Future<Output = ()> + Send> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut crypto = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(chain, key)?;
    crypto.alpn_protocols = vec![ALPN.to_vec()];
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?));
    config.transport_config(Arc::new(transport()));
    let endpoint = quinn::Endpoint::server(config, addr)?;
    Ok(async move {
        while let Some(incoming) = endpoint.accept().await {
            let doors = doors.clone();
            tokio::spawn(async move {
                let ended: anyhow::Result<()> = async {
                    let conn = incoming.await?;
                    let (send, recv) = conn.accept_bi().await?;
                    pump(doors, conn, send, recv, Join::default()).await
                }
                .await;
                if let Err(e) = ended {
                    tracing::debug!("quinn door: {e}");
                }
            });
        }
    })
}

/// A client's end of the raw door (tests, the bench).
pub async fn quinn_client(addr: SocketAddr, trust: CertificateDer<'static>) -> anyhow::Result<(quinn::Connection, quinn::SendStream, quinn::RecvStream)> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(trust)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut crypto = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    crypto.alpn_protocols = vec![ALPN.to_vec()];
    let mut config = quinn::ClientConfig::new(Arc::new(quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?));
    config.transport_config(Arc::new(transport()));
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(config);
    let conn = endpoint.connect(addr, "localhost")?.await?;
    let (send, recv) = conn.open_bi().await?;
    Ok((conn, send, recv))
}

/// The WebTransport door: the URL's path is the join (`/live/<code>?role=…`), as on the WebSocket path.
pub fn wt_door(doors: Arc<Doors>, addr: SocketAddr, identity: wtransport::Identity) -> anyhow::Result<impl Future<Output = ()> + Send> {
    let config = wtransport::ServerConfig::builder()
        .with_bind_address(addr)
        .with_identity(identity)
        .keep_alive_interval(Some(PING_EVERY))
        .max_idle_timeout(Some(DOOR_IDLE))?
        .build();
    let endpoint = wtransport::Endpoint::server(config)?;
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
                    pump(doors, Wt(Arc::new(conn)), send, recv, join).await
                }
                .await;
                if let Err(e) = ended {
                    tracing::debug!("webtransport door: {e}");
                }
            });
        }
    })
}
