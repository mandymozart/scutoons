//! The Cathedral of decks over the relay (#188 phase 4, LIVE.md B.6): one thread with a blocking WebSocket, like the
//! outlet's client (live/client.rs).
//!
//! - **A deck** joins as `musician` of the group `decks`, sends its being (`deck.graph`) and then what changed of its
//!   values (`deck.values`, stream.rs – ≤ 30 / s). It folds what it sent: after a reconnect the graph and every value
//!   go out again at once (the relay dropped its being when it left).
//! - **The conductor's deck** joins as `conductor` (a session token; any non-empty one on a dev relay) and hands the
//!   frame thread the decks' beings: the snapshot's `decks` at the welcome, then each `deck.graph`, `deck.values` and
//!   `presence` (who is there, who sends to whom). It never resumes – a fresh join brings the whole snapshot.
use protocol::{DeckGraph, DeckLink, DeckNode, DeckValues, Envelope, Hello, Message, Ping, Role};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

use super::being::Look;
use super::Join;
use crate::graph::Graph;

const PING_EVERY: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(20);
const BACKOFF: [u64; 5] = [1, 2, 4, 8, 10];
/// the decks' group
pub const GROUP: &str = "decks";

/// what the conductor's frame thread learns
#[derive(Debug, Clone, PartialEq)]
pub enum In {
    /// this stage's client id
    Me(String),
    /// a deck's being (joined, or its scene changed); its values and mood so far
    Deck { client: String, graph: DeckGraph, values: HashMap<String, Vec<f64>>, mood: Option<String> },
    Values { client: String, values: HashMap<String, Vec<f64>>, mood: Option<String> },
    /// the decks there are and who sends to whom
    Presence { decks: Vec<String>, routes: Vec<(String, String)> },
}

/// the running client: its events, its outbox, its HUD line; stops when dropped
pub struct Link {
    pub rx: Receiver<In>,
    tx: Sender<DeckValues>,
    pub status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Link {
    /// Join `relay` (`ws://host:port/live/<code>`) as `join`; a deck announces `graph`.
    pub fn start(relay: &str, join: Join, token: Option<String>, nick: Option<String>, graph: Option<DeckGraph>) -> Result<Link, String> {
        let cfg = crate::live::config::Config::new(relay, GROUP, "de").map_err(|e| e.to_string())?;
        let (tx_in, rx) = mpsc::channel();
        let (tx, outbox) = mpsc::channel();
        let status = Arc::new(Mutex::new(format!("JOINING {}", cfg.code)));
        let stop = Arc::new(AtomicBool::new(false));
        let client = Client { base: cfg.base, code: cfg.code, join, token, nick, graph, events: tx_in, outbox, status: status.clone(), stop: stop.clone(), sent: HashMap::new(), mood: None };
        std::thread::Builder::new().name("cathedral".into()).spawn(move || client.run()).map_err(|e| e.to_string())?;
        Ok(Link { rx, tx, status, stop })
    }

    /// a deck's changes (the client folds them, sends them in order)
    pub fn send(&self, values: DeckValues) {
        let _ = self.tx.send(values);
    }

    pub fn status(&self) -> String {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// a deck's being for the wire: its graph's nodes and links and the look's head – `moods` = the head's whole palette
/// (the energy's ladder + the feelings' moods), so the conductor knows every mood the deck may stream
pub fn deck_graph(graph: &Graph, look: &Look, moods: &[String]) -> DeckGraph {
    let port = |(n, p): (usize, usize), out: bool| {
        let node = &graph.nodes[n];
        let name = if out { node.kind.outputs()[p].name } else { node.kind.inputs()[p].name };
        format!("{}.{name}", node.id)
    };
    DeckGraph {
        nodes: graph.nodes.iter().map(|n| DeckNode { id: n.id.clone(), kind: n.kind.name().into() }).collect(),
        links: graph
            .links
            .iter()
            .map(|l| DeckLink {
                from: port(l.from, true),
                to: port(l.to, false),
                scale: (l.scale != 1.0).then_some(l.scale as f64),
                offset: (l.offset != 0.0).then_some(l.offset as f64),
            })
            .collect(),
        character: (!look.character.is_empty()).then(|| look.character.clone()),
        size: Some(look.size as f64),
        moods: Some(moods.to_vec()),
        expose: (!look.expose.is_empty()).then(|| look.expose.clone()),
    }
}

/// a deck's being from the wire: its graph (refused when this build lacks one of its kinds) and its look
pub fn from_wire(g: &DeckGraph) -> anyhow::Result<(Graph, Look)> {
    use crate::scene::{LinkDef, NodeDef, Scene};
    let mut scene = Scene::parse("")?;
    scene.nodes = g.nodes.iter().map(|n| NodeDef { id: n.id.clone(), kind: n.kind.clone(), params: toml::Table::new() }).collect();
    scene.links = g
        .links
        .iter()
        .map(|l| LinkDef { from: l.from.clone(), to: l.to.clone(), scale: l.scale.unwrap_or(1.0) as f32, offset: l.offset.unwrap_or(0.0) as f32 })
        .collect();
    let mut look = Look { size: g.size.unwrap_or(0.5).clamp(0.1, 2.0) as f32, character: g.character.clone().unwrap_or_default(), ..Look::default() };
    if let Some(m) = g.moods.as_ref().filter(|m| !m.is_empty()) {
        look.moods = m.clone();
    }
    look.expose = g.expose.clone().unwrap_or_default();
    Ok((Graph::build(&scene)?, look))
}

struct Client {
    base: String,
    code: String,
    join: Join,
    token: Option<String>,
    nick: Option<String>,
    graph: Option<DeckGraph>,
    events: Sender<In>,
    outbox: Receiver<DeckValues>,
    status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    /// everything the deck sent (the keyframe after a reconnect)
    sent: HashMap<String, Vec<f64>>,
    mood: Option<String>,
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

impl Client {
    fn set(&self, s: String) {
        if let Ok(mut st) = self.status.lock() {
            *st = s;
        }
    }

    fn run(mut self) {
        let mut tries = 0usize;
        while !self.stop.load(Ordering::Relaxed) {
            match self.connect() {
                Ok(mut ws) => {
                    tries = 0;
                    let why = self.serve(&mut ws);
                    let _ = ws.close(None);
                    let _ = ws.flush();
                    if self.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    tracing::info!("cathedral: the relay connection ended ({why}) – joining again");
                    self.set(format!("OFFLINE {why}"));
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(e) => {
                    let wait = BACKOFF[tries.min(BACKOFF.len() - 1)];
                    tries += 1;
                    self.set(format!("NO RELAY {e} - AGAIN IN {wait} S"));
                    let until = Instant::now() + Duration::from_secs(wait);
                    while Instant::now() < until && !self.stop.load(Ordering::Relaxed) {
                        // the outbox keeps folding while offline
                        self.fold();
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        }
    }

    fn connect(&self) -> Result<Socket, String> {
        let host = self.base.trim_start_matches("ws://");
        let addr = host.to_socket_addrs().map_err(|e| format!("{host}: {e}"))?.next().ok_or_else(|| format!("{host}: no address"))?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("{host}: {e}"))?;
        tcp.set_nodelay(true).ok();
        let role = if self.join == Join::Conductor { "conductor" } else { "musician" };
        let url = format!("{}/live/{}?role={role}&group={GROUP}", self.base, self.code);
        let (ws, _) = tungstenite::client(url, MaybeTlsStream::Plain(tcp)).map_err(|e| format!("handshake: {e}"))?;
        if let MaybeTlsStream::Plain(s) = ws.get_ref() {
            s.set_read_timeout(Some(POLL)).map_err(|e| e.to_string())?;
        }
        Ok(ws)
    }

    fn send(ws: &mut Socket, m: Message) -> Result<(), String> {
        let text = serde_json::to_string(&Envelope::new("", m)).expect("an envelope serializes");
        ws.send(Ws::Text(text)).map_err(|e| e.to_string())
    }

    /// the outbox into `sent` (offline: nothing goes out, the latest values wait)
    fn fold(&mut self) -> Vec<DeckValues> {
        let mut out = Vec::new();
        while let Ok(v) = self.outbox.try_recv() {
            for (k, x) in &v.values {
                self.sent.insert(k.clone(), x.clone());
            }
            if v.mood.is_some() {
                self.mood.clone_from(&v.mood);
            }
            out.push(v);
        }
        out
    }

    fn serve(&mut self, ws: &mut Socket) -> String {
        let conductor = self.join == Join::Conductor;
        let hello = Hello {
            code: self.code.clone(),
            role: if conductor { Role::Conductor } else { Role::Musician },
            group: Some(GROUP.into()),
            nick: self.nick.clone(),
            token: if conductor { Some(self.token.clone().unwrap_or_else(|| "dev".into())) } else { None },
            resume: None,
        };
        if let Err(e) = Self::send(ws, Message::Hello(hello)) {
            return e;
        }
        let mut pinged = Instant::now();
        let mut joined = false;
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return "stopping".into();
            }
            // a deck's values: only once welcomed (the graph goes first)
            let out = self.fold();
            if joined {
                for v in out {
                    if let Err(e) = Self::send(ws, Message::DeckValues(v)) {
                        return e;
                    }
                }
            }
            if pinged.elapsed() >= PING_EVERY {
                pinged = Instant::now();
                if let Err(e) = Self::send(ws, Message::Ping(Ping { t0: 0 })) {
                    return e;
                }
            }
            let text = match ws.read() {
                Ok(Ws::Text(t)) => t,
                Ok(Ws::Close(_)) => return "the relay closed it".into(),
                Ok(_) => continue,
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => continue,
                Err(e) => return e.to_string(),
            };
            let Ok(e) = serde_json::from_str::<Envelope>(&text) else { continue };
            let from = e.from;
            match e.message {
                Message::Welcome(w) => {
                    tracing::info!("cathedral: {} as {} (session {})", w.client, if conductor { "the conductor" } else { "a deck" }, self.code);
                    self.set(format!("{} {} @ {}", if conductor { "CONDUCTOR" } else { "DECK" }, w.client, self.code));
                    let _ = self.events.send(In::Me(w.client));
                    for (client, d) in w.snapshot.decks.unwrap_or_default() {
                        let _ = self.events.send(In::Deck { client, graph: d.graph, values: d.values, mood: d.mood });
                    }
                    if let Some(g) = self.graph.clone() {
                        // the being, then every value it showed so far (a keyframe)
                        if let Err(e) = Self::send(ws, Message::DeckGraph(g)) {
                            return e;
                        }
                        let all = DeckValues { values: self.sent.clone(), mood: self.mood.clone() };
                        if !all.values.is_empty() || all.mood.is_some() {
                            if let Err(e) = Self::send(ws, Message::DeckValues(all)) {
                                return e;
                            }
                        }
                    }
                    joined = true;
                }
                Message::DeckGraph(graph) => {
                    let _ = self.events.send(In::Deck { client: from, graph, values: HashMap::new(), mood: None });
                }
                Message::DeckValues(v) => {
                    let _ = self.events.send(In::Values { client: from, values: v.values, mood: v.mood });
                }
                Message::Presence(p) => {
                    let routes = p.routes.unwrap_or_default().into_iter().map(|r| (r.from, r.to)).collect();
                    let _ = self.events.send(In::Presence { decks: p.decks.unwrap_or_default(), routes });
                }
                Message::Error(err) => {
                    tracing::warn!("cathedral: the relay says {}: {}", err.code, err.message);
                    self.set(format!("RELAY {}: {}", err.code, err.message));
                    if err.code == "token" {
                        return format!("refused: {}", err.message);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};

    #[test]
    fn a_being_survives_the_wire() {
        let scene = crate::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        let g = Graph::build(&scene).unwrap();
        let wire = deck_graph(&g, &scene.being, &scene.being.moods);
        let json = serde_json::to_string(&Message::DeckGraph(wire.clone())).unwrap();
        let Message::DeckGraph(back) = serde_json::from_str(&json).unwrap() else { panic!() };
        let (g2, look) = from_wire(&back).unwrap();
        assert_eq!(g2.nodes.iter().map(|n| (&n.id, n.kind)).collect::<Vec<_>>(), g.nodes.iter().map(|n| (&n.id, n.kind)).collect::<Vec<_>>());
        assert_eq!(g2.links.len(), g.links.len());
        for (a, b) in g.links.iter().zip(&g2.links) {
            assert_eq!((a.from, a.to, a.scale, a.offset), (b.from, b.to, b.scale, b.offset));
        }
        assert_eq!((look.character.as_str(), look.size, &look.moods), (scene.being.character.as_str(), scene.being.size, &scene.being.moods));
        // a kind this stage doesn't know: refused with its name
        let mut odd = wire;
        odd.nodes[0].kind = "teleporter".into();
        assert!(from_wire(&odd).unwrap_err().to_string().contains("teleporter"));
    }
}
