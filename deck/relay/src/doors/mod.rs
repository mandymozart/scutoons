//! #438: the QUIC door beside the WebSocket path, into the same sessions. There is no bridge process: the relay is the
//! hub, a door is only an adapter that moves bytes between a transport and the session's outbox (RULES A1, A4 – `Envelope`
//! stays the one wire format, nothing is decided here that the session does not decide).
//!
//! Two lanes (`outbox::Lane`): `deck.values` may be lost, latest value wins; everything else is reliable and in order, so
//! cues, votes, `scene.sync` and snapshots wait for nothing but their own lane. WebSocket stays the fallback (`lib.rs`,
//! unchanged) and joins the very same `Relay`.
//!
//! `deck.values` carry only what changed, so a lost datagram could leave a value wrong for as long as it does not change
//! again. The **settle** repairs that: every `SETTLE` the relay sends each door conductor what changed since the last
//! one, on the reliable lane (a conductor new to the door: everything). A receiver applies a key only when the frame's
//! `seq` is not older than the key's. The other way, a deck's own settle is numbered by its last datagram, so one that
//! the stream delayed does not take back what a newer datagram already said.

pub mod quic;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use protocol::{DeckValues, Envelope, Message, Role, To};
use tokio_tungstenite::tungstenite::Utf8Bytes;

use crate::outbox::{Frame, Outbox};
use crate::{now_ms, session, Join, Relay};

/// how often a door member gets every deck's current values on the reliable lane
pub const SETTLE: Duration = Duration::from_secs(1);
/// A door lets a silent connection go after this, and the client's QUIC notices at the same moment.
pub const DOOR_IDLE: Duration = Duration::from_secs(3);
/// The doors of one relay: what the transports share.
pub struct Doors {
    pub relay: Arc<Relay>,
    /// session code → its members that came through a door (the settle's addressees); an entry lives as long as that
    /// session's one settle task
    members: Mutex<HashMap<String, HashMap<String, DoorMember>>>,
}

struct DoorMember {
    outbox: Arc<Outbox>,
    /// has had the settle once: from then on only what changes
    settled: bool,
}

/// what a client's datagrams said last: the newest number in all and per value (`MOOD`: its mood)
#[derive(Default)]
struct Uplink {
    last: u64,
    keys: HashMap<String, u64>,
}

const MOOD: &str = "";

/// One client that came through a door.
pub struct Member {
    doors: Arc<Doors>,
    pub code: String,
    pub id: String,
    pub outbox: Arc<Outbox>,
    /// datagrams can overtake each other, and a settle on the stream
    up: Mutex<Uplink>,
}

impl Doors {
    pub fn new(relay: Arc<Relay>) -> Arc<Doors> {
        Arc::new(Doors { relay, members: Mutex::new(HashMap::new()) })
    }

    /// The first reliable message of a connection. Err = the refusal to send.
    pub fn admit(self: &Arc<Doors>, first: &str, join: Join) -> Result<Member, Utf8Bytes> {
        let outbox = Outbox::new();
        let (id, hello) = self.relay.admit(Some(first), join, outbox.clone())?;
        let code = hello.code;
        let first_of_session = {
            let mut members = self.members.lock().unwrap();
            let first = !members.contains_key(&code);
            members.entry(code.clone()).or_default().insert(id.clone(), DoorMember { outbox: outbox.clone(), settled: false });
            first
        };
        if first_of_session {
            spawn_settle(self.clone(), code.clone());
        }
        Ok(Member { doors: self.clone(), code, id, outbox, up: Mutex::default() })
    }
}

impl Member {
    /// A message from the reliable lane.
    pub fn text(&self, text: &str) {
        let mut envelope = serde_json::from_str::<Envelope>(text);
        if envelope.as_mut().is_ok_and(|e| self.overtaken(e)) {
            return;
        }
        self.doors.relay.receive(&self.code, &self.id, envelope);
    }

    /// A deck's values on the stream, numbered by its last datagram before them, keep only what no newer datagram has
    /// said since. True: nothing is left of them.
    fn overtaken(&self, e: &mut Envelope) -> bool {
        let seq = e.seq;
        let Message::DeckValues(v) = &mut e.message else { return false };
        if seq == 0 {
            return false;
        }
        let up = self.up.lock().unwrap();
        v.values.retain(|k, _| up.keys.get(k).is_none_or(|n| *n <= seq));
        if up.keys.get(MOOD).is_some_and(|n| *n > seq) {
            v.mood = None;
        }
        v.values.is_empty() && v.mood.is_none()
    }

    /// A datagram: only `deck.values`, never an older one than this client already sent. Anything else is ignored, so
    /// nothing that must arrive can be sent on a lane that may lose it.
    pub fn datagram(&self, bytes: &[u8]) {
        let Ok(e) = serde_json::from_slice::<Envelope>(bytes) else { return };
        let Message::DeckValues(v) = &e.message else { return };
        {
            let mut up = self.up.lock().unwrap();
            if e.seq != 0 && e.seq <= up.last {
                return;
            }
            up.last = e.seq;
            let said = v.values.keys().map(String::as_str).chain(v.mood.as_ref().map(|_| MOOD));
            for key in said {
                match up.keys.get_mut(key) {
                    Some(n) => *n = e.seq,
                    None => {
                        up.keys.insert(key.to_owned(), e.seq);
                    }
                }
            }
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
            if set.get(&self.id).is_some_and(|m| Arc::ptr_eq(&m.outbox, &self.outbox)) {
                set.remove(&self.id);
            }
        }
        self.doors.relay.leave(&self.code, &self.id, &self.outbox);
        self.outbox.close();
    }
}

/// While a session has door members: every `SETTLE`, to each conductor among them, what the decks changed.
fn spawn_settle(doors: Arc<Doors>, code: String) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(SETTLE);
        every.tick().await;
        let mut said = Said::new();
        loop {
            every.tick().await;
            let members: Vec<(String, bool)> = {
                let mut all = doors.members.lock().unwrap();
                let Some(set) = all.get_mut(&code).filter(|set| !set.is_empty()) else {
                    all.remove(&code);
                    return;
                };
                set.iter_mut().map(|(id, m)| (id.clone(), !std::mem::replace(&mut m.settled, true))).collect()
            };
            doors.relay.with_session(&code, |s| settle(s, &members, &mut said));
        }
    });
}

/// each deck's values and mood as the last settle sent them
type Said = HashMap<String, (HashMap<String, Vec<f64>>, Option<String>)>;

/// `members`: the door's clients, each with whether it has yet to have everything.
fn settle(s: &mut session::Session, members: &[(String, bool)], said: &mut Said) {
    let conductors: Vec<(&String, bool, Arc<Outbox>)> = members
        .iter()
        .filter_map(|(id, new)| s.clients.get(id).filter(|c| c.role == Role::Conductor).map(|c| (id, *new, c.outbox.clone())))
        .collect();
    if conductors.is_empty() {
        return;
    }
    let decks = s.deck_states();
    said.retain(|deck, _| decks.contains_key(deck));
    let t = Instant::now();
    for (deck, state) in decks.iter().filter(|(_, state)| !state.values.is_empty()) {
        let changed = said.get(deck).is_none_or(|(values, mood)| *values != state.values || *mood != state.mood);
        for (id, _, outbox) in conductors.iter().filter(|(id, new, _)| *id != deck && (changed || *new)) {
            let mut e = Envelope::new(deck.as_str(), Message::DeckValues(DeckValues { values: state.values.clone(), mood: state.mood.clone() }));
            e.seq = s.seq;
            e.to = Some(To::client(id.as_str()));
            // reliable lane: `ordered`, not state-like (a settle must not be merged into a datagram's slot)
            outbox.push(Frame::ordered(serde_json::to_string(&e).expect("envelope serializes").into()), t);
        }
        if changed {
            said.insert(deck.clone(), (state.values.clone(), state.mood.clone()));
        }
    }
}
