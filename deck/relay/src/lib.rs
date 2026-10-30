//! The Live relay (agents/research/LIVE.md B.4, protocol B.6): sessions by code, clients with a role and a group,
//! messages routed by their `to`. The rules of a session – authority, rate limits, snapshot, replay, votes, the clock's
//! lead, presence – live in `session.rs` (synchronous, unit-tested); this file owns the sockets, the session tokens
//! (`token.rs`) and the vote timer. Not here: the uplink to the hosted relay (B.4 linked mode) and pickups checked
//! against the API (both need the session API, `POST /api/live/sessions`).
//!
//! Dogma (CYBERDECKS.md §2): one task per connection waiting on its socket, one writer task per client fed by a
//! channel, one timer task per open vote – no polling anywhere.

// tungstenite's handshake callback must return its (large) `ErrorResponse` – not ours to shrink.
#![allow(clippy::result_large_err)]

pub mod session;
pub mod token;
pub mod vote;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Error as ProtocolError, Message, Role, Snapshot, Welcome};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message as Ws;

pub use session::{targets, After, Client, Session};

/// B.6: at most 1000 clients per session.
pub const MAX_CLIENTS: usize = 1000;
/// A client that sends no `hello` within this time is dropped.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// the relay's WebSocket ping to every client
pub const PING_EVERY: Duration = Duration::from_secs(5);
/// no frame at all (not even the pong) for this long = the client is gone
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
/// B.6: the vote tally goes to the conductor every 250 ms.
pub const TALLY_EVERY: Duration = Duration::from_millis(250);

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

    /// Run `f` on the session `code` (if it exists).
    pub fn with_session<R>(&self, code: &str, f: impl FnOnce(&mut Session) -> R) -> Option<R> {
        self.sessions.lock().unwrap().get_mut(code).map(f)
    }

    /// The token check of B.6 for conductor / bridge / uplink; other roles need none.
    fn check_token(&self, code: &str, role: Role, token: Option<&str>) -> Result<(), String> {
        if !matches!(role, Role::Conductor | Role::Bridge | Role::Uplink) {
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

    /// Join (or resume) and send the welcome: the snapshot, or – on a resume inside the replay buffer – an empty
    /// snapshot followed by the missed messages.
    fn join(&self, hello: &protocol::Hello, url_group: Option<String>, tx: mpsc::UnboundedSender<String>) -> Result<String, &'static str> {
        let now = now_ms();
        let mut sessions = self.sessions.lock().unwrap();
        // empty sessions are kept for a resume (the conductor's laptop restarts), then dropped
        sessions.retain(|_, s| s.empty_since.map_or(true, |t| t >= now - session::REPLAY_MS));
        let s = sessions.entry(hello.code.clone()).or_default();
        if s.clients.len() >= MAX_CLIENTS {
            return Err("session full");
        }
        let resumed = hello.resume.as_ref().and_then(|r| s.resume(r, hello.role).map(|replay| (r, replay)));
        let id = match resumed {
            Some((r, _)) => r.client.clone(),
            None => format!("c{}", self.next_client.fetch_add(1, Ordering::Relaxed)),
        };
        let group = hello.group.clone().or(url_group).or_else(|| resumed.and_then(|_| s.departed_group(&id)));
        let nick = session::clean_nick(hello.nick.as_deref(), &self.config.blocklist);
        s.join(id.clone(), Client::new(hello.role, group.clone(), nick, tx, now));
        let snapshot = match resumed {
            Some((_, true)) => Snapshot { seq: Some(s.seq as i64), ..Default::default() },
            _ => s.snapshot_for(hello.role, group.as_deref()),
        };
        s.send_to(&id, Message::Welcome(Welcome { client: id.clone(), group, snapshot, now }));
        if let Some((r, true)) = resumed {
            s.replay(&id, r.seq as u64);
        }
        s.tell_presence();
        Ok(id)
    }

    fn leave(&self, code: &str, id: &str) {
        self.with_session(code, |s| s.leave(id, now_ms()));
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

/// Accept connections until the listener fails.
pub async fn serve(listener: TcpListener, relay: Arc<Relay>) -> anyhow::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let relay = relay.clone();
        tokio::spawn(async move {
            if let Err(e) = connection(stream, relay).await {
                tracing::debug!(%peer, "connection ended: {e}");
            }
        });
    }
}

/// Read one HTTP request head (up to and including the blank line) and discard it; bounded to 8 KiB.
async fn drain_http_request(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf = Vec::with_capacity(512);
    let mut chunk = [0u8; 512];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 8192 {
            return Ok(());
        }
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

async fn connection(mut stream: TcpStream, relay: Arc<Relay>) -> anyhow::Result<()> {
    // `GET /status` is plain HTTP (B.4 deploy freeze): tungstenite refuses a request without the upgrade headers
    // before any callback runs, so peek at the request line first and answer it ourselves.
    let mut head = [0u8; 512];
    let n = stream.peek(&mut head).await?;
    if head[..n].starts_with(b"GET /status ") {
        // Consume the request before answering: a socket closed with unread bytes in its receive buffer is reset
        // (RST, not FIN) on macOS / BSD, and the client's read then fails with ConnectionReset (CI 2026-10-29).
        drain_http_request(&mut stream).await?;
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
    let ws = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, res: Response| {
        uri = req.uri().to_string();
        Ok::<_, ErrorResponse>(res)
    })
    .await?;
    let join = parse_join(&uri);
    let (mut sink, mut source) = ws.split();

    // B.6: the first message is `hello`.
    let hello = tokio::time::timeout(HELLO_TIMEOUT, source.next()).await;
    let hello = match hello {
        Ok(Some(Ok(Ws::Text(text)))) => serde_json::from_str::<Envelope>(&text).ok(),
        _ => None,
    };
    let Some(Envelope { message: Message::Hello(hello), .. }) = hello else {
        sink.send(Ws::Text(refusal("hello", "the first message must be hello"))).await.ok();
        return Ok(());
    };
    if join.code.as_deref().is_some_and(|c| c != hello.code) {
        sink.send(Ws::Text(refusal("code", "hello.code differs from the URL"))).await.ok();
        return Ok(());
    }
    if let Err(why) = relay.check_token(&hello.code, hello.role, hello.token.as_deref()) {
        sink.send(Ws::Text(refusal("token", &why))).await.ok();
        return Ok(());
    }

    let code = hello.code.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let id = match relay.join(&hello, join.group, tx) {
        Ok(id) => id,
        Err(why) => {
            sink.send(Ws::Text(refusal("full", why))).await.ok();
            return Ok(());
        }
    };
    tracing::info!(code, client = id, role = hello.role.as_str(), resume = hello.resume.is_some(), "joined");

    // a WebSocket ping every PING_EVERY: browsers and tungstenite answer it themselves, so a client that stays silent
    // past IDLE_TIMEOUT is gone even when its TCP side never closed (a phone reloading behind `adb reverse` left a
    // ghost deck at the conductor, 2026-10-31)
    let writer = tokio::spawn(async move {
        let mut ping = tokio::time::interval_at(tokio::time::Instant::now() + PING_EVERY, PING_EVERY);
        ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            let sent = tokio::select! {
                text = rx.recv() => match text {
                    Some(text) => sink.send(Ws::Text(text)).await,
                    None => break,
                },
                _ = ping.tick() => sink.send(Ws::Ping(Default::default())).await,
            };
            if sent.is_err() {
                break;
            }
        }
        sink.close().await.ok();
    });

    loop {
        let frame = match tokio::time::timeout(IDLE_TIMEOUT, source.next()).await {
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
            Ok(_) => continue, // binary / ping / pong frames: nothing in B.6 (a pong only proves the client is there)
        };
        let lead = relay.config.lead_ms;
        let after = relay.with_session(&code, |s| match serde_json::from_str::<Envelope>(&text) {
            Ok(envelope) => s.handle(&id, envelope, now_ms(), lead),
            Err(e) => {
                s.send_to(&id, Message::Error(ProtocolError { code: "parse".into(), message: e.to_string() }));
                After::Nothing
            }
        });
        if after == Some(After::VoteOpened) {
            spawn_vote_timer(relay.clone(), code.clone());
        }
    }

    relay.leave(&code, &id);
    writer.abort();
    tracing::info!(code, client = id, "left");
    Ok(())
}

fn refusal(code: &str, message: &str) -> String {
    let e = Envelope::new("relay", Message::Error(ProtocolError { code: code.into(), message: message.into() }));
    serde_json::to_string(&e).expect("envelope serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::To;

    fn client(role: Role, group: Option<&str>) -> Client {
        let (tx, _rx) = mpsc::unbounded_channel();
        Client::new(role, group.map(str::to_owned), None, tx, 0)
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
    }
}
