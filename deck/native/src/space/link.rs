//! The Cathedral of decks over the relay (#188 phase 4, LIVE.md B.6): the deck's side of deck-space's relay client
//! (relay.rs, #366 – it decides the join, the keyframe, the pushes and answers, the pings and the status) on the
//! socket thread (live/socket.rs). What is the deck's alone: the frame's outbox in, the session's events, the sync and
//! the show clock out.
//!
//! - **A deck** joins as `musician` of the group `decks` with its being (`deck.graph`) and its values (`deck.values`,
//!   stream.rs – ≤ 30 / s); the conductor's push goes to its sync (`Syncs`, sync.rs), whose answers go back.
//! - **The conductor's deck** joins as `conductor` (a session token; a dev relay takes any) and hands the frame thread
//!   the decks' beings (`In`); its push (`Wire::push`) goes out once joined and again when the relay forgot it.
//! - **The show clock (#321):** pongs, the relay's `tempo` anchors and the welcome go to the clock (`ShowLink`, stamped
//!   when read); the pings it asks for go out with the client's clock (µs of `now_ns`), and so does the anchor of a deck
//!   that leads. A link for the clock alone (`Join::Deck` without a being) sends no graph.
use crate::live::socket::{self, Side};
use crate::tempo::relay::ShowLink;
use deck_space::far::{FarTable, Subscriptions};
use deck_space::relay::{Client, Options, Out, DECKS};
use protocol::{DeckGraph, DeckMidi, DeckQuality, DeckSynced, DeckValues, Pong, Role, SceneSync};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deck_space::space::wire::{In, Wire};
use deck_space::space::Join;

const POLL: Duration = Duration::from_millis(20);
/// the decks' group
pub const GROUP: &str = DECKS;

/// #195: a deck's sync as the link sees it (sync.rs's DeckSync): the conductor's push in, the answer out
pub trait Syncs: Send {
    fn push(&self, sync: SceneSync);
    /// the answer, if one newer than `seen` is there: its number and the answer
    fn synced(&self, seen: u64) -> Option<(u64, DeckSynced)>;
}

/// what the frame hands the link
enum Give {
    Values(DeckValues),
    Push(SceneSync),
    Quality(DeckQuality),
    /// #211: the deck's MIDI outputs for the conductor (only while joined – stale MIDI is dropped, not sent later)
    Midi(DeckMidi),
    /// #442: the engine's far world nodes and event outputs
    Far(FarTable),
}

/// the running client: its events, its outbox, its HUD line; stops when dropped
pub struct Link {
    pub rx: Receiver<In>,
    tx: Sender<Give>,
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
        let conductor = join == Join::Conductor;
        let mut o = Options::new(relay, if conductor { Role::Conductor } else { Role::Musician }, Some(GROUP));
        o.nick = nick;
        o.token = token.filter(|_| conductor);
        let mut client = Client::new(o)?;
        if let Some(g) = graph {
            client.set_graph(g);
        }
        let (events, rx) = mpsc::channel();
        let (tx, outbox) = mpsc::channel();
        let status = Arc::new(Mutex::new(client.line()));
        let stop = Arc::new(AtomicBool::new(false));
        let mut side = Deck { outbox, events, status: status.clone(), sync, seen: 0, show, far: None, subs: Subscriptions::default(), sending: Vec::new() };
        let halt = stop.clone();
        std::thread::Builder::new().name("cathedral".into()).spawn(move || socket::run(client, &mut side, &halt, None, POLL)).map_err(|e| e.to_string())?;
        Ok(Link { rx, tx, status, stop })
    }
}

impl Wire for Link {
    fn send(&self, values: DeckValues) {
        let _ = self.tx.send(Give::Values(values));
    }

    fn push(&self, sync: SceneSync) {
        let _ = self.tx.send(Give::Push(sync));
    }

    fn quality(&self, quality: DeckQuality) {
        let _ = self.tx.send(Give::Quality(quality));
    }

    fn midi(&self, midi: DeckMidi) {
        let _ = self.tx.send(Give::Midi(midi));
    }

    fn far(&self, table: FarTable) {
        let _ = self.tx.send(Give::Far(table));
    }

    fn recv(&self) -> Option<In> {
        self.rx.try_recv().ok()
    }

    fn status(&self) -> String {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// the deck's side of the client
struct Deck {
    outbox: Receiver<Give>,
    events: Sender<In>,
    status: Arc<Mutex<String>>,
    sync: Option<Box<dyn Syncs>>,
    /// the last sync answer handed to the client
    seen: u64,
    show: Option<Arc<ShowLink>>,
    /// #442: the engine's far world nodes and event outputs, once the frame handed them over
    far: Option<FarTable>,
    /// who asked this deck for which outputs
    subs: Subscriptions,
    /// scratch: (client, event) per poll
    sending: Vec<(String, protocol::DeckEvent)>,
}

impl Deck {
    /// #442, each poll: this deck's event outputs to their subscribers (one message per client, in order), lapsed
    /// subscriptions dropped
    fn far_out(&mut self, c: &mut Client, now: f64) {
        let Some(table) = self.far.as_mut() else { return };
        self.subs.expire(now);
        self.sending.clear();
        table.drain(&self.subs, &mut self.sending);
        while let Some((client, _)) = self.sending.first() {
            let client = client.clone();
            let events: Vec<protocol::DeckEvent> = self.sending.iter().filter(|(cl, _)| *cl == client).map(|(_, e)| e.clone()).collect();
            self.sending.retain(|(cl, _)| *cl != client);
            c.events(&client, events);
        }
    }
}

impl Side for Deck {
    fn feed(&mut self, c: &mut Client, now: f64) {
        while let Ok(g) = self.outbox.try_recv() {
            match g {
                Give::Values(v) => c.values(v),
                Give::Push(s) => c.push(s),
                Give::Quality(q) => c.quality(q),
                Give::Midi(m) => c.midi(m),
                Give::Far(table) => {
                    c.set_listen(now, table.addresses());
                    self.far = Some(table);
                    self.subs = Subscriptions::default();
                }
            }
        }
        self.far_out(c, now);
        if let Some((n, r)) = self.sync.as_ref().and_then(|y| y.synced(self.seen)) {
            self.seen = n;
            c.synced(now, r);
        }
        if let Some(s) = &self.show {
            // #321: the pings the clock asks for; a leader's anchor once joined
            if s.take_ping().is_some() {
                c.ping(now);
            }
            if c.joined() {
                if let Some(t) = s.take_tempo() {
                    c.tempo(t);
                }
            }
        }
    }

    fn hear(&mut self, out: Out, c: &Client) {
        match out {
            Out::Status(_) => {
                if let Ok(mut s) = self.status.lock() {
                    *s = c.line();
                }
            }
            Out::Joined { client, .. } => {
                tracing::info!("cathedral: {client} (session {})", c.code());
                let _ = self.events.send(In::Me(client));
                if let Some(s) = &self.show {
                    s.joined();
                }
            }
            Out::Pong { t0, t1, .. } => {
                if let Some(s) = &self.show {
                    s.pong(&Pong { t0, t1 });
                }
            }
            Out::Tempo { tempo, .. } => {
                if let Some(s) = &self.show {
                    s.tempo(&tempo);
                }
            }
            Out::Sync(s) => {
                if let Some(y) = &self.sync {
                    y.push(s);
                }
            }
            Out::Deck(i) => {
                let _ = self.events.send(i);
            }
            // #442: another deck asks for far addresses – what names this deck's outputs is served from now on
            Out::Listen { client, addresses } => {
                if let Some(t) = &self.far {
                    let offered = t.offers(&addresses);
                    if !offered.is_empty() {
                        tracing::info!("far events: {client} asks for {}", offered.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>().join(", "));
                    }
                    self.subs.listen(&client, offered, socket::now_ms());
                }
            }
            // #442: events for this deck's far world nodes, into their rings
            Out::Events { events, .. } => {
                if let Some(t) = self.far.as_mut() {
                    t.take(&events);
                }
            }
            _ => {}
        }
    }
}
