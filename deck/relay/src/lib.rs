//! The Live relay (agents/research/LIVE.md B.4, protocol B.6): sessions by code, clients with a role and a group,
//! messages routed by their `to`. The rules of a session – authority, rate limits, snapshot, replay, votes, the clock's
//! lead, presence – live in `session.rs` (synchronous, unit-tested); this file owns the sockets (ws, and wss on the same
//! port when a deck has a certificate – `tls.rs`, #201), the session tokens (`token.rs`) and the vote timer. Not here: the uplink to the hosted relay (B.4 linked mode) and pickups checked
//! against the API (both need the session API, `POST /api/live/sessions`).
//!
//! Dogma (CYBERDECKS.md §2): one task per connection waiting on its socket, one writer task per client woken by
//! its outbox (outbox.rs, #223: the latest value wins, ordered messages stay in order, a client too far behind is
//! dropped), one timer task per open vote – no polling anywhere.

// tungstenite's handshake callback must return its (large) `ErrorResponse` – not ours to shrink.
#![allow(clippy::result_large_err)]

pub mod cert;
pub mod doors;
pub mod link;
pub mod outbox;
pub mod roles;
pub mod session;
pub mod tls;
pub mod token;
pub mod tree;
pub mod vote;

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Error as ProtocolError, Message, Role, Snapshot, Welcome};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{Message as Ws, Utf8Bytes};

pub use outbox::Outbox;
pub use session::{targets, After, Client, Session};

/// B.6: at most 1000 clients per session.
pub const MAX_CLIENTS: usize = 1000;
/// A client that sends no `hello` within this time is dropped.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// the relay's WebSocket ping to every client – the keep-alive, and its pong the link's round trip (#223)
pub const PING_EVERY: Duration = Duration::from_secs(1);
/// no frame at all (not even the pong) for this long = the client is gone
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
/// B.6: the vote tally goes to the conductor every 250 ms.
pub const TALLY_EVERY: Duration = Duration::from_millis(250);
/// a client's WebSocket read buffer. tungstenite's default is 128 KiB, allocated and zeroed per connection (~128 MB at
/// `MAX_CLIENTS`) for bulk reads the relay's small frames never make (#437); a larger frame just takes more reads.
pub const READ_BUFFER: usize = 4 * 1024;

/// How this relay runs: on a deck in the venue LAN or hosted (B.4).
#[derive(Debug, Clone)]
pub struct Config {
    /// B.6 clock: ms added to `at` for effects sent without one – 50 on the LAN, 150 hosted (`RELAY_LEAD_MS`)
    pub lead_ms: i64,
    /// the HMAC secret shared with the API (`LIVE_TOKEN_SECRET`); none = dev mode, any non-empty token passes
    pub token_secret: Option<Vec<u8>>,
    /// nickname blocklist, lower case substrings (`RELAY_BLOCKLIST`, comma-separated)
    pub blocklist: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config { lead_ms: 50, token_secret: None, blocklist: vec![] }
    }
}

impl Config {
    /// From the environment (main.rs); unset values keep the defaults.
    pub fn from_env() -> Self {
        let mut c = Config::default();
        if let Some(ms) = std::env::var("RELAY_LEAD_MS").ok().and_then(|v| v.parse().ok()) {
            c.lead_ms = ms;
        }
        c.token_secret = std::env::var("LIVE_TOKEN_SECRET").ok().filter(|s| !s.is_empty()).map(String::into_bytes);
        c.blocklist = std::env::var("RELAY_BLOCKLIST")
            .unwrap_or_default()
            .split(',')
            .map(|w| w.trim().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();
        c
    }
}

/// All sessions, in memory – B.4: exactly one instance.
pub struct Relay {
    sessions: Mutex<HashMap<String, Session>>,
    next_client: AtomicU64,
    pub config: Config,
}

impl Default for Relay {
    fn default() -> Self {
        Self::new()
    }
}

/// Relay time in ms (B.6 `now`, `at`).
pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl Relay {
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Self {
        Relay { sessions: Mutex::new(HashMap::new()), next_client: AtomicU64::new(1), config }
    }

    /// `{ sessions, live }` for `GET /status` – sessions with someone connected; live = with a conductor (B.4 freeze).
    pub fn status(&self) -> (usize, usize) {
        let sessions = self.sessions.lock().unwrap();
        let open = sessions.values().filter(|s| !s.clients.is_empty()).count();
        let live = sessions.values().filter(|s| s.clients.values().any(|c| c.role == Role::Conductor)).count();
        (open, live)
    }

    /// Run `f` on the session `code`, made if it does not exist yet (tree.rs: a child's relay opens its session before anyone joined).
    pub fn with_or_new_session<R>(&self, code: &str, f: impl FnOnce(&mut Session) -> R) -> R {
        f(self.sessions.lock().unwrap().entry(code.to_owned()).or_default())
    }

    /// Run `f` on the session `code` (if it exists).
    pub fn with_session<R>(&self, code: &str, f: impl FnOnce(&mut Session) -> R) -> Option<R> {
        self.sessions.lock().unwrap().get_mut(code).map(f)
    }

    /// The token check of B.6 for conductor / bridge / uplink / seat, and #469 the handheld (its token is the API's answer
    /// to its one-time pairing code); other roles need none.
    fn check_token(&self, code: &str, role: Role, token: Option<&str>) -> Result<(), String> {
        if !matches!(role, Role::Conductor | Role::Bridge | Role::Uplink | Role::Seat | Role::Handheld) {
            return Ok(());
        }
        let token = token.unwrap_or("");
        if token.is_empty() {
            return Err("this role needs a session token".into());
        }
        match &self.config.token_secret {
            None => Ok(()),
            Some(secret) => token::verify(secret, token, code, role, now_ms() / 1000).map(|_| ()).map_err(str::to_owned),
        }
    }

    /// #478: what a client's proof states for session `code` (a role's filter asks it). None, a bad or an expired one: no
    /// facts, a guest – never refused for it. Without a secret (dev mode) the facts are taken as stated (main.rs says so).
    /// The relay learns facts, never who it is.
    fn facts(&self, code: &str, proof: Option<&str>) -> token::Facts {
        let Some(proof) = proof.filter(|p| !p.is_empty()) else { return token::Facts::default() };
        token::proof(self.config.token_secret.as_deref(), proof, code, now_ms() / 1000).unwrap_or_else(|why| {
            tracing::info!(code, "a proof not taken ({why}): a guest");
            token::Facts::default()
        })
    }

    /// Join (or resume) and send the welcome: the snapshot, or – on a resume inside the replay buffer – an empty
    /// snapshot followed by the missed messages.
    fn join(&self, hello: &protocol::Hello, url_group: Option<String>, outbox: Arc<Outbox>) -> Result<String, &'static str> {
        let now = now_ms();
        let fresh = new_key();
        let mut sessions = self.sessions.lock().unwrap();
        // empty sessions are kept for a resume (the conductor's laptop restarts), then dropped
        sessions.retain(|_, s| s.empty_since.is_none_or(|t| t >= now - session::REPLAY_MS));
        let s = sessions.entry(hello.code.clone()).or_default();
        if let Some(r) = &hello.resume {
            s.take_over(r, hello.role, now);
        }
        if s.clients.len() >= MAX_CLIENTS {
            return Err("session full");
        }
        let resumed = hello.resume.as_ref().and_then(|r| s.resume(r, hello.role).map(|replay| (r, replay)));
        let (id, key) = match resumed {
            Some((r, _)) => (r.client.clone(), r.key.clone().unwrap_or_default()),
            None => (format!("c{}", self.next_client.fetch_add(1, Ordering::Relaxed)), fresh),
        };
        let group = hello.group.clone().or(url_group).or_else(|| resumed.and_then(|_| s.departed_group(&id)));
        let nick = session::clean_nick(hello.nick.as_deref(), &self.config.blocklist);
        let mut client = Client::new(hello.role, group.clone(), nick, outbox, now);
        client.key.clone_from(&key);
        client.device.clone_from(&hello.device);
        client.facts = self.facts(&hello.code, hello.proof.as_deref());
        s.join(id.clone(), client);
        // #478: a role it may not take is not given
        let group = s.clients.get(&id).and_then(|c| c.group.clone());
        let snapshot = match resumed {
            Some((_, true)) => Snapshot { seq: Some(s.seq as i64), ..Default::default() },
            _ => {
                let mut snap = s.snapshot_for(hello.role, group.as_deref());
                // #195: a scene.sync for listed decks reaches a client by its id (a resume past the replay buffer)
                snap.sync = snap.sync.or_else(|| s.sync_for(Some(&id), hello.role, group.as_deref()));
                snap
            }
        };
        s.send_to(&id, Message::Welcome(Welcome { client: id.clone(), group, snapshot, now, key: Some(key) }));
        if let Some((r, true)) = resumed {
            s.replay(&id, r.seq as u64);
        }
        s.tell_presence();
        s.tell_roles();
        Ok(id)
    }

    /// The link with `outbox` is gone. One a resume replaced leaves nothing behind: the id is the new link's now.
    fn leave(&self, code: &str, id: &str, outbox: &Arc<Outbox>) {
        self.with_session(code, |s| {
            if s.clients.get(id).is_some_and(|c| Arc::ptr_eq(&c.outbox, outbox)) {
                s.leave(id, now_ms());
            }
        });
    }

    /// B.6: a connection's first message, whichever door it came through (`None`: it said nothing readable). The client's
    /// id and its hello, or the refusal to send.
    fn admit(&self, first: Option<&str>, join: Join, outbox: Arc<Outbox>) -> Result<(String, protocol::Hello), Utf8Bytes> {
        let hello = first.and_then(|text| serde_json::from_str::<Envelope>(text).ok());
        let Some(Envelope { message: Message::Hello(hello), .. }) = hello else {
            return Err(refusal("hello", "the first message must be hello"));
        };
        if join.code.as_deref().is_some_and(|c| c != hello.code) {
            return Err(refusal("code", "hello.code differs from the URL"));
        }
        self.check_token(&hello.code, hello.role, hello.token.as_deref()).map_err(|why| refusal("token", &why))?;
        let id = self.join(&hello, join.group, outbox).map_err(|why| refusal("full", why))?;
        Ok((id, hello))
    }

    /// A client's message from the reliable lane, whichever door it came through.
    fn receive(self: &Arc<Relay>, code: &str, id: &str, envelope: Result<Envelope, serde_json::Error>) {
        let lead = self.config.lead_ms;
        let after = self.with_session(code, |s| match envelope {
            Ok(mut envelope) => {
                // B.6: the conductor's rename passes the same cleaning as a hello
                if let Message::Rename(r) = &mut envelope.message {
                    r.nick = session::clean_nick(Some(&r.nick), &self.config.blocklist).unwrap_or_default();
                }
                s.handle(id, envelope, now_ms(), lead)
            }
            Err(e) => {
                s.send_to(id, Message::Error(ProtocolError { code: "parse".into(), message: e.to_string() }));
                After::Nothing
            }
        });
        if after == Some(After::VoteOpened) {
            spawn_vote_timer(self.clone(), code.to_owned());
        }
    }
}

/// The URL a client connects with: `/live/<code>?role=<role>&group=<group>` (B.2: the role from the URL / join code).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Join {
    pub code: Option<String>,
    pub role: Option<Role>,
    pub group: Option<String>,
}

pub fn parse_join(uri: &str) -> Join {
    let (path, query) = uri.split_once('?').unwrap_or((uri, ""));
    let mut join = Join::default();
    if let Some(code) = path.strip_prefix("/live/") {
        let code = code.trim_matches('/');
        if !code.is_empty() {
            join.code = Some(code.to_owned());
        }
    }
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "role" => join.role = serde_json::from_value(serde_json::Value::String(v.to_owned())).ok(),
            "group" if !v.is_empty() => join.group = Some(v.to_owned()),
            _ => {}
        }
    }
    join
}

/// Accept connections until the listener fails – plain ws (the hosted relay: bunny's edge ends TLS).
pub async fn serve(listener: TcpListener, relay: Arc<Relay>) -> anyhow::Result<()> {
    serve_with(listener, relay, None).await
}

/// One client on any byte stream – what an accepted socket gets, for an in-process client (the bench's lossy links,
/// examples/bench.rs). Ends when the client leaves.
pub async fn serve_stream<S>(stream: S, relay: Arc<Relay>) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    connection(stream, relay).await
}

/// Accept connections until the listener fails; with a TLS acceptor (tls.rs, #201) the same port speaks ws and wss.
pub async fn serve_with(listener: TcpListener, relay: Arc<Relay>, tls: Option<TlsAcceptor>) -> anyhow::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        if let Err(e) = tune(&stream) {
            tracing::debug!(%peer, "socket options: {e}");
        }
        let relay = relay.clone();
        let tls = tls.clone();
        tokio::spawn(async move {
            let ended = match tls {
                Some(tls) => plain_or_tls(stream, relay, tls).await,
                None => connection(stream, relay).await,
            };
            if let Err(e) = ended {
                tracing::debug!(%peer, "connection ended: {e}");
            }
        });
    }
}

/// #223: an accepted socket sends each frame at once – no Nagle (a value would wait for the ACK of the one before, tens
/// of ms on Wi-Fi). Applies to ws and wss alike (TLS wraps this socket).
///
/// And the kernel keeps little unsent data (Linux: `TCP_NOTSENT_LOWAT`, which leaves the bytes in flight alone; elsewhere
/// a small send buffer), so a stalled client's backlog waits in its outbox – where the latest value wins (outbox.rs) –
/// instead of in the kernel, where stale values can no longer be replaced.
pub fn tune(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let socket = socket2::SockRef::from(stream);
    #[cfg(any(target_os = "linux", target_os = "android"))]
    socket.set_tcp_notsent_lowat(UNSENT as u32)?;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    socket.set_send_buffer_size(2 * UNSENT)?;
    Ok(())
}

/// #223: the unsent bytes the kernel may hold per client (≈ 25 frames of values)
pub const UNSENT: usize = 4 * 1024;

/// TLS record type "handshake": the first byte of every ClientHello (an HTTP request starts with a letter)
const TLS_HANDSHAKE: u8 = 0x16;

/// One port, both schemes: a TLS handshake gets TLS, anything else the plain path. Both steps within HELLO_TIMEOUT.
async fn plain_or_tls(stream: TcpStream, relay: Arc<Relay>, tls: TlsAcceptor) -> anyhow::Result<()> {
    let mut first = [0u8; 1];
    let n = tokio::time::timeout(HELLO_TIMEOUT, stream.peek(&mut first)).await??;
    if n == 1 && first[0] == TLS_HANDSHAKE {
        let stream = tokio::time::timeout(HELLO_TIMEOUT, tls.accept(stream)).await??;
        connection(stream, relay).await
    } else {
        connection(stream, relay).await
    }
}

/// Read into `buf` until `done(buf)` or the peer stops sending.
async fn read_until<S: AsyncRead + Unpin>(stream: &mut S, buf: &mut Vec<u8>, done: impl Fn(&[u8]) -> bool) -> io::Result<()> {
    let mut chunk = [0u8; 512];
    while !done(buf) {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(())
}

/// A stream that first gives back the bytes already read from it (the request line the status check looked at), then
/// reads on – what `peek` was on a bare TCP socket, for a TLS stream too.
struct Prefixed<S> {
    head: Vec<u8>,
    at: usize,
    inner: S,
}

impl<S: AsyncRead + Unpin> AsyncRead for Prefixed<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = &mut *self;
        if this.at < this.head.len() {
            let n = (this.head.len() - this.at).min(buf.remaining());
            buf.put_slice(&this.head[this.at..this.at + n]);
            this.at += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Prefixed<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// While the session's vote is open: the tally every 250 ms, the close at the deadline (session.rs `tick`).
fn spawn_vote_timer(relay: Arc<Relay>, code: String) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(TALLY_EVERY);
        every.tick().await; // the first tick is immediate
        loop {
            every.tick().await;
            if relay.with_session(&code, |s| s.tick(now_ms())) != Some(true) {
                break;
            }
        }
    });
}

const STATUS: &[u8] = b"GET /status ";

async fn connection<S>(mut stream: S, relay: Arc<Relay>) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // `GET /status` is plain HTTP (B.4 deploy freeze): tungstenite refuses a request without the upgrade headers
    // before any callback runs, so read the start of the request line first and answer it ourselves; anything else
    // goes to tungstenite with those bytes put back in front (`Prefixed`). Silent past HELLO_TIMEOUT = dropped.
    let mut head = Vec::with_capacity(512);
    let first_line = |b: &[u8]| b.len() >= STATUS.len() || b.contains(&b'\n');
    tokio::time::timeout(HELLO_TIMEOUT, read_until(&mut stream, &mut head, first_line)).await??;
    if head.starts_with(STATUS) {
        // Consume the request before answering: a socket closed with unread bytes in its receive buffer is reset
        // (RST, not FIN) on macOS / BSD, and the client's read then fails with ConnectionReset (CI 2026-10-29).
        let whole = |b: &[u8]| b.windows(4).any(|w| w == b"\r\n\r\n") || b.len() > 8192;
        tokio::time::timeout(HELLO_TIMEOUT, read_until(&mut stream, &mut head, whole)).await??;
        let (sessions, live) = relay.status();
        let body = format!(r#"{{"sessions":{sessions},"live":{live}}}"#);
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await?;
        stream.shutdown().await.ok();
        return Ok(());
    }

    let mut uri = String::new();
    let stream = Prefixed { head, at: 0, inner: stream };
    let callback = |req: &Request, res: Response| {
        uri = req.uri().to_string();
        Ok::<_, ErrorResponse>(res)
    };
    let config = WebSocketConfig::default().read_buffer_size(READ_BUFFER);
    let ws = tokio_tungstenite::accept_hdr_async_with_config(stream, callback, Some(config)).await?;
    let join = parse_join(&uri);
    let (mut sink, mut source) = ws.split();

    // B.6: the first message is `hello`.
    let first = match tokio::time::timeout(HELLO_TIMEOUT, source.next()).await {
        Ok(Some(Ok(Ws::Text(text)))) => Some(text),
        _ => None,
    };
    let outbox = Outbox::new();
    let (id, hello) = match relay.admit(first.as_deref(), join, outbox.clone()) {
        Ok(admitted) => admitted,
        Err(refusal) => {
            sink.send(Ws::Text(refusal)).await.ok();
            return Ok(());
        }
    };
    let code = hello.code.clone();
    tracing::info!(code, client = id, role = hello.role.as_str(), resume = hello.resume.is_some(), "joined");
    let writer = tokio::spawn(write(sink, outbox.clone(), relay.clone(), code.clone()));

    loop {
        let next = tokio::select! {
            next = tokio::time::timeout(IDLE_TIMEOUT, source.next()) => next,
            _ = outbox.dropped() => break, // too far behind (outbox.rs): it reconnects and resyncs from its snapshot
        };
        let frame = match next {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(_) => {
                tracing::info!(code, client = id, "silent for {IDLE_TIMEOUT:?} – dropped");
                break;
            }
        };
        let text = match frame {
            Ok(Ws::Text(t)) => t,
            Ok(Ws::Close(_)) | Err(_) => break,
            // a pong proves the client is there and measures its link (#223)
            Ok(Ws::Pong(payload)) => {
                if outbox.pong(&payload, std::time::Instant::now()) {
                    relay.with_session(&code, |s| s.tell_presence());
                }
                continue;
            }
            Ok(_) => continue, // binary / ping frames: nothing in B.6
        };
        relay.receive(&code, &id, serde_json::from_str(&text));
    }

    relay.leave(&code, &id, &outbox);
    outbox.close();
    writer.abort();
    tracing::info!(code, client = id, "left");
    Ok(())
}

/// One client's writer: what its outbox lets go goes out as one batch (fed, then one flush – one write for many small
/// frames), timed for the link's rate (outbox.rs, link.rs); a WebSocket ping every PING_EVERY, numbered, so its pong
/// measures the round trip. Browsers and tungstenite answer pings themselves, so a client that stays silent past
/// IDLE_TIMEOUT is gone even when its TCP side never closed (a phone reloading behind `adb reverse` left a ghost deck at
/// the conductor, 2026-10-31). A changed rate tells the conductor (presence).
pub(crate) async fn write<S>(mut sink: S, outbox: Arc<Outbox>, relay: Arc<Relay>, code: String)
where
    S: futures_util::Sink<Ws> + Unpin,
{
    let mut batch: Vec<Utf8Bytes> = Vec::with_capacity(64);
    let mut next_ping = tokio::time::Instant::now() + PING_EVERY;
    let mut pings = 0u64;
    'out: loop {
        let now = tokio::time::Instant::now();
        let mut fed = false;
        if now >= next_ping {
            next_ping = now + PING_EVERY;
            pings += 1;
            outbox.pinged(pings, now.into_std());
            if sink.feed(Ws::Ping(pings.to_be_bytes().to_vec().into())).await.is_err() {
                break;
            }
            fed = true;
        }
        let took = outbox.take(now.into_std(), &mut batch);
        let texts = !batch.is_empty();
        for text in batch.drain(..) {
            if sink.feed(Ws::Text(text)).await.is_err() {
                break 'out;
            }
            fed = true;
        }
        if fed {
            if sink.flush().await.is_err() {
                break;
            }
            let done = tokio::time::Instant::now();
            if texts && outbox.sent(done.into_std(), done - now, took.waited) {
                relay.with_session(&code, |s| s.tell_presence());
            }
            continue;
        }
        let wake = took.until.map_or(next_ping, |t| next_ping.min(t.into()));
        tokio::select! {
            _ = outbox.pushed() => {}
            _ = tokio::time::sleep_until(wake) => {}
        }
    }
    outbox.close();
    sink.close().await.ok();
}

/// A client's resume key (#438): 128 random bits that only it and the relay ever know.
fn new_key() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 16];
    rustls::crypto::ring::default_provider().secure_random.fill(&mut bytes).expect("the system's random source");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn refusal(code: &str, message: &str) -> Utf8Bytes {
    let e = Envelope::new("relay", Message::Error(ProtocolError { code: code.into(), message: message.into() }));
    serde_json::to_string(&e).expect("envelope serializes").into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::To;

    fn client(role: Role, group: Option<&str>) -> Client {
        Client::new(role, group.map(str::to_owned), None, Outbox::new(), 0)
    }

    fn room() -> HashMap<String, Client> {
        HashMap::from([
            ("cond".to_owned(), client(Role::Conductor, None)),
            ("outA".to_owned(), client(Role::Output, Some("a"))),
            ("outB".to_owned(), client(Role::Output, Some("b"))),
            ("audA".to_owned(), client(Role::Audience, Some("a"))),
            ("bridge".to_owned(), client(Role::Bridge, None)),
        ])
    }

    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[test]
    fn no_to_reaches_everyone_but_the_sender() {
        let r = room();
        assert_eq!(sorted(targets(r.iter(), "cond", None)), ["audA", "bridge", "outA", "outB"]);
    }

    #[test]
    fn role_and_group_narrow() {
        let r = room();
        assert_eq!(sorted(targets(r.iter(), "cond", Some(&To::role(Role::Output)))), ["outA", "outB"]);
        assert_eq!(targets(r.iter(), "cond", Some(&To::role_group(Role::Output, "a"))), ["outA"]);
        assert_eq!(sorted(targets(r.iter(), "cond", Some(&To::group("a")))), ["audA", "outA"]);
        // the conductor has no group: a group filter never reaches it
        assert_eq!(targets(r.iter(), "outA", Some(&To::group("none"))), Vec::<String>::new());
    }

    #[test]
    fn client_is_exact() {
        let r = room();
        assert_eq!(targets(r.iter(), "cond", Some(&To::client("bridge"))), ["bridge"]);
        // a sender may address itself explicitly
        assert_eq!(targets(r.iter(), "cond", Some(&To::client("cond"))), ["cond"]);
    }

    #[test]
    fn join_url() {
        assert_eq!(
            parse_join("/live/A7K2?role=output&group=left"),
            Join { code: Some("A7K2".into()), role: Some(Role::Output), group: Some("left".into()) }
        );
        assert_eq!(parse_join("/live/A7K2"), Join { code: Some("A7K2".into()), role: None, group: None });
        assert_eq!(parse_join("/?role=teleporter"), Join::default());
    }

    /// #223: an accepted socket has Nagle off (the ws and the wss path both go through `serve_with`'s `tune`)
    #[tokio::test]
    async fn accepted_sockets_have_nodelay() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _client = TcpStream::connect(addr).await.unwrap();
        let (accepted, _) = listener.accept().await.unwrap();
        assert!(!accepted.nodelay().unwrap(), "the OS default is Nagle on");
        tune(&accepted).unwrap();
        assert!(accepted.nodelay().unwrap());
        // and little unsent data in the kernel (the backlog waits in the outbox)
        #[cfg(any(target_os = "linux", target_os = "android"))]
        assert_eq!(socket2::SockRef::from(&accepted).tcp_notsent_lowat().unwrap(), UNSENT as u32);
        #[cfg(windows)]
        assert_eq!(socket2::SockRef::from(&accepted).send_buffer_size().unwrap(), 2 * UNSENT);
    }

    #[test]
    fn tokens_with_and_without_a_secret() {
        let dev = Relay::new();
        assert!(dev.check_token("A", Role::Conductor, Some("anything")).is_ok());
        assert!(dev.check_token("A", Role::Conductor, None).is_err());
        assert!(dev.check_token("A", Role::Audience, None).is_ok());
        let secret = b"s3cret".to_vec();
        let prod = Relay::with_config(Config { token_secret: Some(secret.clone()), ..Config::default() });
        assert!(prod.check_token("A", Role::Conductor, Some("anything")).is_err());
        let claims = token::Claims { session: "A".into(), role: Role::Conductor, exp: now_ms() / 1000 + 60, show: None };
        assert!(prod.check_token("A", Role::Conductor, Some(&token::sign(&secret, &claims))).is_ok());
        // #469: a handheld joins only with the API's token for its one session
        assert!(dev.check_token("H", Role::Handheld, None).is_err());
        let hand = token::Claims { session: "H".into(), role: Role::Handheld, exp: now_ms() / 1000 + 60, show: None };
        assert!(prod.check_token("H", Role::Handheld, Some(&token::sign(&secret, &hand))).is_ok());
        assert!(prod.check_token("OTHER", Role::Handheld, Some(&token::sign(&secret, &hand))).is_err(), "another session");
        assert!(prod.check_token("H", Role::Conductor, Some(&token::sign(&secret, &hand))).is_err(), "not the editor's");
    }

    /// #478: a role that asks member, live or an approved deck is offered and taken with the API's proof, and not without;
    /// a tampered, an expired or another session's proof makes a guest – never a refusal. Presence says the proven fact.
    #[test]
    fn proofs_open_the_roles_that_ask_for_them() {
        use protocol::{Account, Admits, Assign, Hello, Roles, ShowRole};
        let secret = b"relay-test-secret".to_vec();
        let relay = Relay::with_config(Config { token_secret: Some(secret.clone()), ..Config::default() });
        let now = now_ms() / 1000;
        let proof = |account: Vec<Account>, approved: bool, session: &str, exp: i64| {
            token::sign(&secret, &token::Proof { session: session.into(), facts: token::Facts { account, approved }, exp })
        };
        let join = |role: Role, group: Option<&str>, token: Option<String>, proof: Option<String>| {
            let hello = Hello { code: "P".into(), role, group: group.map(Into::into), nick: None, device: None, token, proof, resume: None };
            let text = serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap();
            relay.admit(Some(&text), Join::default(), Outbox::new()).map(|(id, _)| id).unwrap()
        };
        let cond_token = token::sign(&secret, &token::Claims { session: "P".into(), role: Role::Conductor, exp: now + 60, show: None });
        let cond = join(Role::Conductor, None, Some(cond_token), None);
        let role = |id: &str, admits: Admits| ShowRole { id: id.into(), label: None, r#for: Some(admits), here: None };
        let roles = vec![
            role("soprano", Admits { account: Some(vec![Account::Member]), ..Default::default() }),
            role("click", Admits { account: Some(vec![Account::Live]), ..Default::default() }),
            role("stage", Admits { approved: Some(true), ..Default::default() }),
        ];
        relay.with_session("P", |s| s.handle(&cond, Envelope::new("", Message::Roles(Roles { roles })), now_ms(), 50));

        let guest = join(Role::Output, None, None, None);
        let member = join(Role::Output, None, None, Some(proof(vec![Account::Member], false, "P", now + 60)));
        let live = join(Role::Audience, Some("click"), None, Some(proof(vec![Account::Live], false, "P", now + 60)));
        let deck = join(Role::Output, Some("stage"), None, Some(proof(vec![], true, "P", now + 60)));
        let signed = proof(vec![Account::Member], false, "P", now + 60);
        let sig = signed.split_once('.').unwrap().1;
        let forged = format!("{}.{sig}", base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, format!(r#"{{"session":"P","facts":{{"account":["live"],"approved":true}},"exp":{}}}"#, now + 60)));
        let tampered = join(Role::Output, None, None, Some(forged));
        let expired = join(Role::Output, Some("soprano"), None, Some(proof(vec![Account::Member], false, "P", now - 1)));
        let elsewhere = join(Role::Output, None, None, Some(proof(vec![Account::Member], true, "OTHER", now + 60)));

        let offered = |id: &str| relay.with_session("P", |s| roles::offer(s, &s.clients[id]).unwrap().into_iter().map(|r| r.id).collect::<Vec<_>>()).unwrap();
        assert_eq!(offered(&guest), Vec::<String>::new(), "no proof: a guest, none of these roles");
        assert_eq!(offered(&member), ["soprano"]);
        assert_eq!(offered(&live), ["click"]);
        assert_eq!(offered(&deck), ["stage"]);
        for (who, id) in [("tampered", &tampered), ("expired", &expired), ("another session's", &elsewhere)] {
            assert_eq!(offered(id), Vec::<String>::new(), "a {who} proof: a guest");
        }
        let group = |id: &str| relay.with_session("P", |s| s.clients[id].group.clone()).unwrap();
        assert_eq!((group(&deck), group(&expired)), (Some("stage".into()), None), "a hello's role: given with the proof, not with an expired one");

        // taken with the proof, refused without
        let take = |id: &str, g: &str| relay.with_session("P", |s| s.handle(id, Envelope::new("", Message::Assign(Assign { client: id.into(), group: g.into(), lock: None })), now_ms(), 50));
        take(&member, "soprano");
        take(&guest, "soprano");
        assert_eq!((group(&member), group(&guest)), (Some("soprano".into()), None));

        // presence: the strongest proven account fact, guest without one
        let who = relay.with_session("P", |s| s.presence().who.unwrap()).unwrap();
        let fact = |id: &str| who[id].account.map(Account::as_str);
        assert_eq!((fact(&member), fact(&live), fact(&deck), fact(&guest), fact(&tampered)), (Some("member"), Some("live"), Some("guest"), Some("guest"), Some("guest")));
    }

    /// #478: a deck whose proof came after its first hello joins again with resume, the role it names and the proof in one
    /// hello (deck-space relay.rs `set_proof`): the relay gives it the role with the proven facts it refused the guest
    #[test]
    fn a_resume_with_a_proof_takes_the_role_a_guest_was_refused() {
        use protocol::{Account, Admits, Hello, Resume, Roles, ShowRole};
        let secret = b"relay-test-secret".to_vec();
        let relay = Relay::with_config(Config { token_secret: Some(secret.clone()), ..Config::default() });
        let now = now_ms() / 1000;
        let hello = |role: Role, group: Option<&str>, token: Option<String>, proof: Option<String>, resume: Option<Resume>| {
            let hello = Hello { code: "R".into(), role, group: group.map(Into::into), nick: None, device: None, token, proof, resume };
            serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap()
        };
        let cond_token = token::sign(&secret, &token::Claims { session: "R".into(), role: Role::Conductor, exp: now + 60, show: None });
        let (cond, _) = relay.admit(Some(&hello(Role::Conductor, None, Some(cond_token), None, None)), Join::default(), Outbox::new()).unwrap();
        let soprano = ShowRole { id: "soprano".into(), label: None, r#for: Some(Admits { account: Some(vec![Account::Member]), ..Default::default() }), here: None };
        relay.with_session("R", |s| s.handle(&cond, Envelope::new("", Message::Roles(Roles { roles: vec![soprano] })), now_ms(), 50));

        let first = Outbox::new();
        let (id, _) = relay.admit(Some(&hello(Role::Output, Some("soprano"), None, None, None)), Join::default(), first.clone()).unwrap();
        let (group, key) = relay.with_session("R", |s| (s.clients[&id].group.clone(), s.clients[&id].key.clone())).unwrap();
        assert_eq!(group, None, "a guest: not given the members' role");
        // the client drops the link and joins again at once – the relay may not have seen the old link go (`take_over`)
        let proof = token::sign(&secret, &token::Proof { session: "R".into(), facts: token::Facts { account: vec![Account::Member], approved: false }, exp: now + 60 });
        let resume = Resume { client: id.clone(), seq: 0, key: Some(key) };
        let (again, _) = relay.admit(Some(&hello(Role::Output, Some("soprano"), None, Some(proof), Some(resume))), Join::default(), Outbox::new()).unwrap();
        assert_eq!(again, id, "the same client");
        assert_eq!(relay.with_session("R", |s| s.clients[&id].group.clone()).unwrap(), Some("soprano".into()), "the role, with the proven facts");
    }
}
