//! The Cathedral of decks over the relay (#188 phase 4, LIVE.md B.6): one thread with a blocking WebSocket, like the
//! outlet's client (live/client.rs).
//!
//! - **A deck** joins as `musician` of the group `decks`, sends its being (`deck.graph`) and then what changed of its
//!   values (`deck.values`, stream.rs – ≤ 30 / s). It folds what it sent: after a reconnect the graph and every value
//!   go out again at once (the relay dropped its being when it left).
//! - **The conductor's deck** joins as `conductor` (a session token; any non-empty one on a dev relay) and hands the
//!   frame thread the decks' beings: the snapshot's `decks` at the welcome, then each `deck.graph`, `deck.values` and
//!   `presence` (who is there, who sends to whom). It never resumes – a fresh join brings the whole snapshot.
//! - **The scene sync (#195):** the conductor pushes `scene.sync` (`Wire::push` – sent once joined, and again at a
//!   welcome whose snapshot lacks it: the relay restarted) and hands the frame each deck's `deck.synced`. A deck hands
//!   a push – the welcome snapshot's `sync`, or a `scene.sync` – to its sync (`Syncs`, sync.rs) and sends its answer
//!   back whenever it is new (progress at most every 200 ms; the final state at once).
//! - **The show clock (#321):** pongs, the relay's `tempo` anchors and the welcome go to the clock (`ShowLink`, stamped
//!   when read); the pings it asks for go out with their real `t0` (µs of `now_ns`), and so does the anchor of a deck
//!   that leads. A link for the clock alone (`Join::Deck` without a being) sends no graph.
use crate::tempo::relay::ShowLink;
use protocol::{DeckGraph, DeckMidi, DeckQuality, DeckSynced, DeckValues, Envelope, Hello, Message, Ping, Role, SceneSync, SyncState};
use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

use deck_space::space::wire::{In, Wire};
use deck_space::space::Join;

const PING_EVERY: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(20);
const BACKOFF: [u64; 5] = [1, 2, 4, 8, 10];
/// a deck's sync progress at most this often (the final state goes at once)
const PROGRESS: Duration = Duration::from_millis(200);
/// the decks' group
pub const GROUP: &str = "decks";

/// #195: a deck's sync as the link sees it (sync.rs's DeckSync): the conductor's push in, the answer out
pub trait Syncs: Send {
    fn push(&self, sync: SceneSync);
    /// the answer, if one newer than `seen` is there: its number and the answer
    fn synced(&self, seen: u64) -> Option<(u64, DeckSynced)>;
}

/// what the frame hands the link
enum Out {
    Values(DeckValues),
    Push(SceneSync),
    Quality(DeckQuality),
    /// #211: the deck's MIDI outputs for the conductor (only while joined – stale MIDI is dropped, not sent later)
    Midi(DeckMidi),
}

/// the running client: its events, its outbox, its HUD line; stops when dropped
pub struct Link {
    pub rx: Receiver<In>,
    tx: Sender<Out>,
    pub status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Link {
    /// Join `relay` (`ws://host:port/live/<code>`, or `wss://` – the hosted relay, #199) as `join`; a deck announces
    /// `graph` and hands pushes to `sync`.
    pub fn start(relay: &str, join: Join, token: Option<String>, nick: Option<String>, graph: Option<DeckGraph>, sync: Option<Box<dyn Syncs>>, show: Option<Arc<ShowLink>>) -> Result<Link, String> {
        let cfg = crate::live::config::Config::new(relay, GROUP, "de").map_err(|e| e.to_string())?;
        let (tx_in, rx) = mpsc::channel();
        let (tx, outbox) = mpsc::channel();
        let status = Arc::new(Mutex::new(format!("JOINING {}", cfg.code)));
        let stop = Arc::new(AtomicBool::new(false));
        let client = Client {
            base: cfg.base,
            code: cfg.code,
            join,
            token,
            nick,
            graph,
            events: tx_in,
            outbox,
            status: status.clone(),
            stop: stop.clone(),
            sent: HashMap::new(),
            mood: None,
            push: None,
            push_due: false,
            sync,
            seen: 0,
            answered: None,
            quality: None,
            quality_due: false,
            show,
        };
        std::thread::Builder::new().name("cathedral".into()).spawn(move || client.run()).map_err(|e| e.to_string())?;
        Ok(Link { rx, tx, status, stop })
    }
}

impl Wire for Link {
    fn send(&self, values: DeckValues) {
        let _ = self.tx.send(Out::Values(values));
    }

    fn push(&self, sync: SceneSync) {
        let _ = self.tx.send(Out::Push(sync));
    }

    fn quality(&self, quality: DeckQuality) {
        let _ = self.tx.send(Out::Quality(quality));
    }
    fn midi(&self, midi: DeckMidi) {
        let _ = self.tx.send(Out::Midi(midi));
    }

    fn recv(&self) -> Option<In> {
        self.rx.try_recv().ok()
    }

    fn status(&self) -> String {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

struct Client {
    base: String,
    code: String,
    join: Join,
    token: Option<String>,
    nick: Option<String>,
    graph: Option<DeckGraph>,
    events: Sender<In>,
    outbox: Receiver<Out>,
    status: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    /// everything the deck sent (the keyframe after a reconnect)
    sent: HashMap<String, Vec<f64>>,
    mood: Option<String>,
    /// the conductor: its push, and whether it still has to go out
    push: Option<SceneSync>,
    push_due: bool,
    /// a deck: its sync, the last answer's number it sent, and when it sent the last one
    sync: Option<Box<dyn Syncs>>,
    seen: u64,
    answered: Option<Instant>,
    /// #220: this deck's render quality, the newest; due = not sent yet
    quality: Option<DeckQuality>,
    quality_due: bool,
    /// #321: the show clock's side
    show: Option<Arc<ShowLink>>,
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
        let role = if self.join == Join::Conductor { "conductor" } else { "musician" };
        crate::live::config::dial(&format!("{}/live/{}?role={role}&group={GROUP}", self.base, self.code), POLL)
    }

    fn send(ws: &mut Socket, m: Message) -> Result<(), String> {
        let text = serde_json::to_string(&Envelope::new("", m)).expect("an envelope serializes");
        ws.send(Ws::Text(text)).map_err(|e| e.to_string())
    }

    /// the outbox into `sent` (offline: nothing goes out, the latest values wait); a push waits until joined
    fn fold(&mut self) -> (Vec<DeckValues>, Vec<DeckMidi>) {
        let mut out = Vec::new();
        let mut midi = Vec::new();
        while let Ok(o) = self.outbox.try_recv() {
            let v = match o {
                Out::Values(v) => v,
                Out::Push(s) => {
                    self.push = Some(s);
                    self.push_due = true;
                    continue;
                }
                Out::Quality(q) => {
                    self.quality = Some(q);
                    self.quality_due = true;
                    continue;
                }
                Out::Midi(m) => {
                    midi.push(m);
                    continue;
                }
            };
            for (k, x) in &v.values {
                self.sent.insert(k.clone(), x.clone());
            }
            if v.mood.is_some() {
                self.mood.clone_from(&v.mood);
            }
            out.push(v);
        }
        (out, midi)
    }

    /// a deck: its sync's answer when it is new – progress at most every PROGRESS, a final state at once
    fn answer(&mut self, ws: &mut Socket) -> Result<(), String> {
        let Some((n, synced)) = self.sync.as_ref().and_then(|y| y.synced(self.seen)) else { return Ok(()) };
        if synced.state == SyncState::Syncing && self.answered.is_some_and(|t| t.elapsed() < PROGRESS) {
            return Ok(());
        }
        self.seen = n;
        self.answered = Some(Instant::now());
        Self::send(ws, Message::DeckSynced(synced))
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
            let (out, midi) = self.fold();
            if joined {
                for v in out {
                    if let Err(e) = Self::send(ws, Message::DeckValues(v)) {
                        return e;
                    }
                }
                for m in midi {
                    if let Err(e) = Self::send(ws, Message::DeckMidi(m)) {
                        return e;
                    }
                }
            }
            if joined && self.push_due {
                if let Some(s) = self.push.clone() {
                    tracing::info!("cathedral: the sync of '{}' to the decks", s.scene);
                    if let Err(e) = Self::send(ws, Message::SceneSync(s)) {
                        return e;
                    }
                }
                self.push_due = false;
            }
            if joined {
                if let Err(e) = self.answer(ws) {
                    return e;
                }
            }
            // #220: this deck's render quality – the newest only
            if joined && self.quality_due {
                if let Some(q) = self.quality.clone() {
                    if let Err(e) = Self::send(ws, Message::DeckQuality(q)) {
                        return e;
                    }
                }
                self.quality_due = false;
            }
            // #321: the pings the clock asks for (and the keepalive), each with its real t0; a leader's anchor
            let asked = self.show.as_ref().and_then(|s| s.take_ping());
            if asked.is_some() || pinged.elapsed() >= PING_EVERY {
                pinged = Instant::now();
                let ping = asked.unwrap_or(Ping { t0: (crate::clock::now_ns() / 1000) as i64 });
                if let Err(e) = Self::send(ws, Message::Ping(ping)) {
                    return e;
                }
            }
            if let Some(t) = self.show.as_ref().filter(|_| joined).and_then(|s| s.take_tempo()) {
                if let Err(e) = Self::send(ws, Message::Tempo(t)) {
                    return e;
                }
            }
            let text = match ws.read() {
                Ok(Ws::Text(t)) => t,
                Ok(Ws::Close(_)) => return "the relay closed it".into(),
                Ok(_) => continue,
                Err(tungstenite::Error::Io(e)) if crate::live::config::timed_out(&e) => continue,
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
                    if conductor {
                        // the session's sync and the decks' answers; this conductor's push again if the relay lacks it
                        if let Some(s) = &w.snapshot.sync {
                            let _ = self.events.send(In::Pushed(s.clone()));
                        }
                        for (client, synced) in w.snapshot.synced.unwrap_or_default() {
                            let _ = self.events.send(In::Synced { client, synced });
                        }
                        self.push_due = self.push.is_some() && w.snapshot.sync != self.push;
                    } else if let (Some(y), Some(s)) = (&self.sync, w.snapshot.sync) {
                        y.push(s);
                        // the answer goes out again (the relay forgot it when this deck left)
                        self.seen = 0;
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
                    // the relay forgot this deck's quality when it left: again
                    self.quality_due = self.quality.is_some();
                    if let Some(s) = &self.show {
                        s.joined();
                        if let Some(t) = &w.snapshot.tempo {
                            s.tempo(t);
                        }
                    }
                    joined = true;
                }
                Message::Pong(p) => {
                    if let Some(s) = &self.show {
                        s.pong(&p);
                    }
                }
                Message::Tempo(t) => {
                    if let Some(s) = &self.show {
                        s.tempo(&t);
                    }
                }
                Message::DeckGraph(graph) => {
                    let _ = self.events.send(In::Deck { client: from, graph, values: HashMap::new(), mood: None });
                }
                Message::DeckValues(v) => {
                    let _ = self.events.send(In::Values { client: from, values: v.values, mood: v.mood });
                }
                Message::SceneSync(s) => {
                    if conductor {
                        let _ = self.events.send(In::Pushed(s));
                    } else if let Some(y) = &self.sync {
                        y.push(s);
                    }
                }
                Message::DeckSynced(synced) if conductor => {
                    let _ = self.events.send(In::Synced { client: from, synced });
                }
                Message::Presence(p) => {
                    let routes = p.routes.unwrap_or_default().into_iter().map(|r| (r.from, r.to)).collect();
                    let _ = self.events.send(In::Presence { decks: p.decks.unwrap_or_default(), routes, links: p.links.unwrap_or_default(), quality: p.quality.unwrap_or_default() });
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
