//! #438 prototype (feature `doors`, off the default path): several transports into the same sessions. There is no bridge
//! process: the relay is the hub, a door is only an adapter that moves bytes between a transport and the session's outbox
//! (RULES A1, A4 – `Envelope` stays the one wire format, nothing is decided here that the session does not decide).
//!
//! Two lanes (`outbox::Lane`): `deck.values` may be lost, latest value wins; everything else is reliable and in order, so
//! cues, votes, `scene.sync` and snapshots wait for nothing but their own lane. WebSocket stays the fallback (`lib.rs`,
//! unchanged) and joins the very same `Relay`.
//!
//! `deck.values` carry only what changed, so a lost datagram could leave a value wrong for as long as it does not change
//! again. The **settle** repairs that: every `SETTLE` the relay sends each door member what every deck showed last, on
//! the reliable lane. A receiver applies a key only when the frame's `seq` is not older than the key's.

pub mod ladder;
pub mod quic;
pub mod renet;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use protocol::{DeckValues, Envelope, Error as ProtocolError, Message, Role, To};

use crate::outbox::{Frame, Outbox};
use crate::{now_ms, session, After, Join, Relay};

/// how often a door member gets every deck's current values on the reliable lane
pub const SETTLE: Duration = Duration::from_secs(1);
/// A door lets a silent connection go after this: the ladder steps down after 2 s and resumes once the relay is free of
/// the old connection (a resume while it is still attached is refused, session.rs `resume`).
pub const DOOR_IDLE: Duration = Duration::from_secs(3);
/// a reliable line longer than this ends the connection
pub const MAX_LINE: usize = 1 << 20;

/// rustls refuses to choose when two crypto providers are linked (`renet2_netcode` brings aws-lc-rs through its
/// wtransport 0.6, the relay uses ring): ring, once, before any door opens.
pub fn init_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// The doors of one relay: what the transports share.
pub struct Doors {
    pub relay: Arc<Relay>,
    /// session code → its members that came through a door (the settle's addressees)
    members: Mutex<HashMap<String, HashSet<String>>>,
}

/// One client that came through a door.
pub struct Member {
    doors: Arc<Doors>,
    pub code: String,
    pub id: String,
    pub outbox: Arc<Outbox>,
    /// the newest `seq` of a datagram from this client: datagrams can overtake each other
    last_up: AtomicU64,
}

impl Doors {
    pub fn new(relay: Arc<Relay>) -> Arc<Doors> {
        init_crypto();
        Arc::new(Doors { relay, members: Mutex::new(HashMap::new()) })
    }

    /// The first reliable message of a connection: the same checks as the WebSocket path (B.6). Err = the refusal to send.
    pub fn admit(self: &Arc<Doors>, first: &str, join: Join) -> Result<Member, String> {
        let hello = match serde_json::from_str::<Envelope>(first) {
            Ok(Envelope { message: Message::Hello(h), .. }) => h,
            _ => return Err(refusal("hello", "the first message must be hello")),
        };
        if join.code.as_deref().is_some_and(|c| c != hello.code) {
            return Err(refusal("code", "hello.code differs from the URL"));
        }
        self.relay.check_token(&hello.code, hello.role, hello.token.as_deref()).map_err(|why| refusal("token", &why))?;
        let outbox = Outbox::new();
        let id = self.relay.join(&hello, join.group, outbox.clone()).map_err(|why| refusal("full", why))?;
        let first_of_session = {
            let mut members = self.members.lock().unwrap();
            let set = members.entry(hello.code.clone()).or_default();
            let first = set.is_empty();
            set.insert(id.clone());
            first
        };
        if first_of_session {
            spawn_settle(self.clone(), hello.code.clone());
        }
        Ok(Member { doors: self.clone(), code: hello.code, id, outbox, last_up: AtomicU64::new(0) })
    }
}

impl Member {
    /// A message from the reliable lane.
    pub fn text(&self, text: &str) {
        let relay = &self.doors.relay;
        let envelope = serde_json::from_str::<Envelope>(text);
        let lead = relay.config.lead_ms;
        let after = relay.with_session(&self.code, |s| match envelope {
            Ok(envelope) => s.handle(&self.id, envelope, now_ms(), lead),
            Err(e) => {
                s.send_to(&self.id, Message::Error(ProtocolError { code: "parse".into(), message: e.to_string() }));
                After::Nothing
            }
        });
        if after == Some(After::VoteOpened) {
            crate::spawn_vote_timer(relay.clone(), self.code.clone());
        }
    }

    /// A datagram: only `deck.values`, never an older one than this client already sent. Anything else is ignored, so
    /// nothing that must arrive can be sent on a lane that may lose it.
    pub fn datagram(&self, bytes: &[u8]) {
        let Ok(e) = serde_json::from_slice::<Envelope>(bytes) else { return };
        if !matches!(e.message, Message::DeckValues(_)) || (e.seq != 0 && e.seq <= self.last_up.fetch_max(e.seq, Ordering::Relaxed)) {
            return;
        }
        let relay = &self.doors.relay;
        let lead = relay.config.lead_ms;
        relay.with_session(&self.code, |s| s.handle(&self.id, e, now_ms(), lead));
    }

    /// Feed the transport's round trip to the link (the adaptive rate, `presence.links`).
    pub fn rtt(&self, rtt: Duration) {
        if self.outbox.link_rtt(Instant::now(), rtt) {
            self.tell_presence();
        }
    }

    pub fn tell_presence(&self) {
        self.doors.relay.with_session(&self.code, |s| s.tell_presence());
    }

    pub fn leave(&self) {
        if let Some(set) = self.doors.members.lock().unwrap().get_mut(&self.code) {
            set.remove(&self.id);
        }
        self.doors.relay.leave(&self.code, &self.id);
        self.outbox.close();
    }
}

/// While a session has door members: every `SETTLE`, to each conductor among them, what every deck showed last.
fn spawn_settle(doors: Arc<Doors>, code: String) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(SETTLE);
        every.tick().await;
        loop {
            every.tick().await;
            let conductors: Vec<String> = {
                let members = doors.members.lock().unwrap();
                let Some(set) = members.get(&code).filter(|s| !s.is_empty()) else {
                    drop(members);
                    doors.members.lock().unwrap().remove(&code);
                    return;
                };
                set.iter().cloned().collect()
            };
            doors.relay.with_session(&code, |s| settle(s, &conductors));
        }
    });
}

fn settle(s: &mut session::Session, members: &[String]) {
    let snapshot = s.snapshot_for(Role::Conductor, None);
    let Some(decks) = snapshot.decks else { return };
    let seq = s.seq;
    let t = Instant::now();
    for (id, c) in s.clients.iter().filter(|(id, c)| c.role == Role::Conductor && members.contains(id)) {
        for (deck, state) in decks.iter().filter(|(deck, state)| *deck != id && !state.values.is_empty()) {
            let mut e = Envelope::new(deck, Message::DeckValues(DeckValues { values: state.values.clone(), mood: state.mood.clone() }));
            e.seq = seq;
            e.to = Some(To::client(id));
            // reliable lane: `ordered`, not state-like (a settle must not be merged into a datagram's slot)
            c.outbox.push(Frame::ordered(serde_json::to_string(&e).expect("envelope serializes")), t);
        }
    }
}

fn refusal(code: &str, message: &str) -> String {
    crate::refusal(code, message)
}

/// The reliable lane of a byte stream is newline-delimited `Envelope` JSON (serde_json escapes newlines, so a line is a
/// message). A message-based transport (netcode) needs no framing.
pub fn line(text: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(text.as_bytes());
    out.push(b'\n');
}
