//! The Live relay (agents/research/LIVE.md B.4, protocol B.6): sessions by code, clients with a role and a group,
//! messages routed by their `to`. This skeleton accepts connections, assigns roles from the URL / the `hello`,
//! stamps `seq` and `from`, answers `ping`, and broadcasts. Not yet (each a `TODO(B.6 …)` below): snapshot + replay,
//! vote tally, the clock's lead, session tokens (HMAC), rate limits, presence.
//!
//! Dogma (CYBERDECKS.md §2): one task per connection waiting on its socket, one writer task per client fed by a
//! channel – no polling anywhere.

// tungstenite's handshake callback must return its (large) `ErrorResponse` – not ours to shrink.
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Error as ProtocolError, Message, Pong, Role, To, Welcome};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message as Ws;

/// B.6: at most 1000 clients per session.
pub const MAX_CLIENTS: usize = 1000;
/// A client that sends no `hello` within this time is dropped.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// What the relay knows about one connected client.
#[derive(Debug, Clone)]
pub struct Client {
    pub role: Role,
    pub group: Option<String>,
    tx: mpsc::UnboundedSender<String>,
}

#[derive(Default)]
struct Session {
    seq: u64,
    clients: HashMap<String, Client>,
}

/// All sessions, in memory – B.4: exactly one instance.
pub struct Relay {
    sessions: Mutex<HashMap<String, Session>>,
    next_client: AtomicU64,
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

/// Who a message from `from` addressed `to` reaches (B.6: `to` narrows by role, group, client; missing = everyone
/// the sender may reach – here: everyone else in the session). A client without a group never matches a group filter.
pub fn targets<'a>(
    clients: impl Iterator<Item = (&'a String, &'a Client)>,
    from: &str,
    to: Option<&To>,
) -> Vec<String> {
    clients
        .filter(|(id, c)| {
            let Some(to) = to else { return id.as_str() != from };
            if let Some(client) = &to.client {
                return client == *id;
            }
            if id.as_str() == from {
                return false;
            }
            to.role.map_or(true, |r| r == c.role) && to.group.as_ref().map_or(true, |g| c.group.as_deref() == Some(g))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

impl Relay {
    pub fn new() -> Self {
        Relay { sessions: Mutex::new(HashMap::new()), next_client: AtomicU64::new(1) }
    }

    /// `{ sessions, live }` for `GET /status` – live = sessions with a conductor connected (B.4 deploy freeze).
    pub fn status(&self) -> (usize, usize) {
        let sessions = self.sessions.lock().unwrap();
        let live = sessions.values().filter(|s| s.clients.values().any(|c| c.role == Role::Conductor)).count();
        (sessions.len(), live)
    }

    fn join(&self, code: &str, role: Role, group: Option<String>, tx: mpsc::UnboundedSender<String>) -> Result<String, &'static str> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.entry(code.to_owned()).or_default();
        if session.clients.len() >= MAX_CLIENTS {
            return Err("session full");
        }
        let id = format!("c{}", self.next_client.fetch_add(1, Ordering::Relaxed));
        session.clients.insert(id.clone(), Client { role, group, tx });
        Ok(id)
    }

    fn leave(&self, code: &str, id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(code) {
            session.clients.remove(id);
            // TODO(B.6 presence): tell the conductor `presence { groups, ready }`.
            if session.clients.is_empty() {
                sessions.remove(code);
            }
        }
    }

    /// Stamp `seq` + `from`, route by `to`, deliver. Returns how many clients got it.
    pub fn publish(&self, code: &str, from: &str, mut envelope: Envelope) -> usize {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(code) else { return 0 };
        session.seq += 1;
        envelope.seq = session.seq;
        envelope.from = from.to_owned();
        // TODO(B.6 replay): keep the last 500 messages / 5 min for `resume`.
        let ids = targets(session.clients.iter(), from, envelope.to.as_ref());
        let text = serde_json::to_string(&envelope).expect("envelope serializes");
        let mut delivered = 0;
        for id in ids {
            if let Some(c) = session.clients.get(&id) {
                if c.tx.send(text.clone()).is_ok() {
                    delivered += 1;
                }
            }
        }
        delivered
    }

    /// A relay-authored message to exactly one client (welcome, pong, error).
    fn send_to(&self, code: &str, id: &str, message: Message) {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(code) else { return };
        session.seq += 1;
        let envelope = Envelope { seq: session.seq, ..Envelope::new("relay", message).to(To::client(id)) };
        if let Some(c) = session.clients.get(id) {
            let _ = c.tx.send(serde_json::to_string(&envelope).expect("envelope serializes"));
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
            "HTTP/1.1 200 OK
content-type: application/json
content-length: {}
connection: close

{body}",
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
        sink.send(Ws::Text(refusal("hello", "the first message must be hello").await)).await.ok();
        return Ok(());
    };
    if join.code.as_deref().is_some_and(|c| c != hello.code) {
        sink.send(Ws::Text(refusal("code", "hello.code differs from the URL").await)).await.ok();
        return Ok(());
    }
    let code = hello.code;
    let role = hello.role;
    let group = hello.group.or(join.group);
    // TODO(B.6 session token): verify the HMAC token of conductor / bridge / uplink against the API's secret.
    if matches!(role, Role::Conductor | Role::Bridge | Role::Uplink) && hello.token.as_deref().unwrap_or("").is_empty() {
        sink.send(Ws::Text(refusal("token", "this role needs a session token").await)).await.ok();
        return Ok(());
    }
    // TODO(B.6 nick): trim to 24 characters, blocklist; TODO(B.6 resume): replay instead of the snapshot.

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let id = match relay.join(&code, role, group.clone(), tx) {
        Ok(id) => id,
        Err(why) => {
            sink.send(Ws::Text(refusal("full", why).await)).await.ok();
            return Ok(());
        }
    };
    tracing::info!(code, client = id, role = role.as_str(), group = group.as_deref().unwrap_or("-"), "joined");
    relay.send_to(
        &code,
        &id,
        // TODO(B.6 snapshot): row, the current cell per outlet, the open vote, the last instruction per group.
        Message::Welcome(Welcome { client: id.clone(), group, snapshot: Default::default(), now: now_ms() }),
    );

    let writer = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink.send(Ws::Text(text)).await.is_err() {
                break;
            }
        }
        sink.close().await.ok();
    });

    while let Some(frame) = source.next().await {
        let text = match frame {
            Ok(Ws::Text(t)) => t,
            Ok(Ws::Close(_)) | Err(_) => break,
            Ok(_) => continue, // binary / ping / pong frames: nothing in B.6
        };
        let envelope = match serde_json::from_str::<Envelope>(&text) {
            Ok(e) => e,
            Err(e) => {
                relay.send_to(&code, &id, Message::Error(ProtocolError { code: "parse".into(), message: e.to_string() }));
                continue;
            }
        };
        // TODO(B.6 rate limit): audience 5 / s.
        match &envelope.message {
            Message::Ping(p) => {
                relay.send_to(&code, &id, Message::Pong(Pong { t0: p.t0, t1: now_ms() }));
            }
            Message::Hello(_) => {
                relay.send_to(&code, &id, Message::Error(ProtocolError { code: "hello".into(), message: "already joined".into() }));
            }
            // TODO(B.6 state): fold into the snapshot before routing.
            // TODO(B.6 cue): split `actions` per outlet → `to`; TODO(B.6 vote.open / vote.cast / vote.close): tally,
            // `vote.tally` to the conductor every 250 ms, `vote.result`; TODO(B.6 pickup): ask the API like the reader.
            // TODO(B.6 authority): only conductor-role messages may change state – refuse `state` / `cue` / `vote.*` /
            // `assign` / `panic` from other roles.
            _ => {
                relay.publish(&code, &id, envelope);
            }
        }
    }

    relay.leave(&code, &id);
    writer.abort();
    tracing::info!(code, client = id, "left");
    Ok(())
}

async fn refusal(code: &str, message: &str) -> String {
    let e = Envelope::new("relay", Message::Error(ProtocolError { code: code.into(), message: message.into() }));
    serde_json::to_string(&e).expect("envelope serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(role: Role, group: Option<&str>) -> Client {
        let (tx, _rx) = mpsc::unbounded_channel();
        Client { role, group: group.map(str::to_owned), tx }
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
}
