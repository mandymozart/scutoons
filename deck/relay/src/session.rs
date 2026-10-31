//! One Live session (LIVE.md B.6): its clients, the replay log, the snapshot state (the last `state`, the current cell
//! and the last media preload per outlet, the last instruction per group, the open vote, every deck's being), presence.
//! Synchronous and clock-free – the caller passes `now` (relay time, ms) – so every rule here is a unit test; lib.rs
//! owns the sockets and the timers.
//!
//! The decks' stream (#188 phase 4): `deck.graph` (a deck's being – kept per client, logged, to the conductor) and
//! `deck.values` (only what changed, ≤ 30 / s per deck – more is dropped; folded into the deck's state and never
//! logged – a stream would push the cues out of the replay buffer; to the conductor, and to `to` as well when a deck
//! sets one). Presence names the decks and every sender → receiver route their values took.
//!
//! The scene sync (#195): `scene.sync` (conductor only) goes to the group `decks` – or only the client ids in its
//! `decks` – is logged and kept, so a deck that joins later finds it in its snapshot; a new one forgets the decks'
//! answers to the old one. `deck.synced` (a deck's progress and result) goes to the conductor, is kept per deck for the
//! conductor's snapshot (`synced`), and a deck whose state is ready counts as ready in presence.
use std::collections::{BTreeSet, HashMap, VecDeque};

use protocol::{
    Assign, Cell, CellMode, Cue, DeckState, DeckSynced, Envelope, Error as ProtocolError, Loose, MediaPreload, Message,
    Presence, Resume, Role, Route, SceneSync, Snapshot, State, SyncState, To, VoteTally,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use crate::outbox::{Frame, Outbox};
use crate::vote::OpenVote;

/// B.6: `resume` within the last 500 messages / 5 minutes replays; older gets the snapshot.
pub const REPLAY_MESSAGES: usize = 500;
pub const REPLAY_MS: i64 = 5 * 60 * 1000;
/// B.6: audience messages are rate-limited to 5 / s (a bucket of 5, refilled at 5 / s).
pub const AUDIENCE_RATE: f64 = 5.0;
/// B.6: nicknames are trimmed to 24 characters.
pub const NICK_MAX: usize = 24;
/// #188: a deck's `deck.values` at most 30 / s (a bucket of 15 – a frame's jitter is no reason to drop one).
pub const VALUES_RATE: f64 = 30.0;
const VALUES_BURST: f64 = 15.0;
/// the decks' group (deck-stage's space/link.rs, @scutoons/live-deck's client.ts): where a `scene.sync` goes
pub const DECKS: &str = "decks";

/// What the relay knows about one connected client.
#[derive(Debug, Clone)]
pub struct Client {
    pub role: Role,
    pub group: Option<String>,
    pub nick: Option<String>,
    pub ready: bool,
    /// #223: its send queue (outbox.rs) – the latest value wins, ordered messages stay in order
    pub outbox: Arc<Outbox>,
    bucket: Bucket,
    /// #188: the deck's values stream
    values: Bucket,
}

impl Client {
    pub fn new(role: Role, group: Option<String>, nick: Option<String>, outbox: Arc<Outbox>, now: i64) -> Self {
        let bucket = Bucket { tokens: AUDIENCE_RATE, at: now, rate: AUDIENCE_RATE, burst: AUDIENCE_RATE };
        let values = Bucket { tokens: VALUES_BURST, at: now, rate: VALUES_RATE, burst: VALUES_BURST };
        Client { role, group, nick, ready: false, outbox, bucket, values }
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    at: i64,
    /// tokens per second, at most `burst`
    rate: f64,
    burst: f64,
}

impl Bucket {
    fn take(&mut self, now: i64) -> bool {
        let refill = (now - self.at).max(0) as f64 / 1000.0 * self.rate;
        self.tokens = (self.tokens + refill).min(self.burst);
        self.at = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// A routed message kept for `resume`: who sent it to whom, and the message as it went out.
#[derive(Debug)]
struct Logged {
    seq: u64,
    t: i64,
    from: String,
    to: Option<To>,
    frame: Arc<Frame>,
}

/// A client that left – its id can be taken again by a `resume` (its vote stays counted).
#[derive(Debug)]
struct Departed {
    role: Role,
    group: Option<String>,
    at: i64,
}

/// Does a message from `from` addressed `to` reach the client `id` (role, group)? B.6: `to` narrows by role, group,
/// client; missing = everyone the sender may reach – everyone else in the session. A client without a group never
/// matches a group filter; a sender may address itself only by its client id.
pub fn reaches(id: &str, role: Role, group: Option<&str>, from: &str, to: Option<&To>) -> bool {
    let Some(to) = to else { return id != from };
    if let Some(client) = &to.client {
        return client == id;
    }
    if id == from {
        return false;
    }
    to.role.map_or(true, |r| r == role) && to.group.as_deref().map_or(true, |g| group == Some(g))
}

/// Who a message reaches, as client ids.
pub fn targets<'a>(clients: impl Iterator<Item = (&'a String, &'a Client)>, from: &str, to: Option<&To>) -> Vec<String> {
    clients.filter(|(id, c)| reaches(id, c.role, c.group.as_deref(), from, to)).map(|(id, _)| id.clone()).collect()
}

/// The kinds only the conductor (or the uplink, which carries the conductor's messages to the hosted relay) may send –
/// B.6: "only conductor-role messages can change state".
fn conductor_only(m: &Message) -> bool {
    matches!(
        m,
        Message::State(_)
            | Message::Cue(_)
            | Message::VoteOpen(_)
            | Message::VoteClose(_)
            | Message::Assign(_)
            | Message::Panic(_)
            | Message::MediaPreload(_)
            | Message::Cell(_)
            | Message::Say(_)
            | Message::Line(_)
            | Message::Fx(_)
            | Message::Colour(_)
            | Message::Device(_)
            | Message::SceneSync(_)
    )
}

/// The relay's own kinds: no client sends them, except the uplink (the hosted relay's messages come down through it).
fn relay_only(m: &Message) -> bool {
    matches!(
        m,
        Message::Welcome(_)
            | Message::Pong(_)
            | Message::Snapshot(_)
            | Message::Presence(_)
            | Message::VoteTally(_)
            | Message::VoteResult(_)
            | Message::Error(_)
    )
}

/// Trim a nickname to 24 characters; empty or blocklisted (a case-insensitive substring) → none.
pub fn clean_nick(nick: Option<&str>, blocklist: &[String]) -> Option<String> {
    let nick: String = nick?.trim().chars().filter(|c| !c.is_control()).take(NICK_MAX).collect();
    let lower = nick.to_lowercase();
    if nick.is_empty() || blocklist.iter().any(|b| !b.is_empty() && lower.contains(b.as_str())) {
        return None;
    }
    Some(nick)
}

/// The string field `outlet` of a loose payload (cell, say, line, fx, colour, a B.5.2 action's body).
fn outlet_of(p: &Loose) -> Option<String> {
    p.get("outlet").and_then(Value::as_str).map(str::to_owned)
}

/// What `handle` asks the caller to do besides delivering: start the vote timer.
#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Nothing,
    VoteOpened,
}

#[derive(Default)]
pub struct Session {
    pub seq: u64,
    pub clients: HashMap<String, Client>,
    log: VecDeque<Logged>,
    /// the highest seq dropped from the log – a resume from before it gets the snapshot
    trimmed: u64,
    departed: HashMap<String, Departed>,
    state: Option<State>,
    cells: HashMap<String, Cell>,
    /// the last `media.preload` per outlet (#186: a reconnecting output resolves its cell's media ids with it)
    media: HashMap<String, MediaPreload>,
    say: HashMap<String, Loose>,
    pub vote: Option<OpenVote>,
    /// relay time when the last client left (the session is kept for a resume, then pruned)
    pub empty_since: Option<i64>,
    /// #188: every deck's being (its graph, its latest values) by client id
    decks: HashMap<String, DeckState>,
    /// #188: who sent values to whom (sender, receiver)
    routes: BTreeSet<(String, String)>,
    /// `deck.values` dropped over the rate
    pub dropped: u64,
    /// #195: the last `scene.sync` (a deck that joins later gets it in its snapshot)
    sync: Option<SceneSync>,
    /// #195: each deck's last `deck.synced` for it, by client id
    synced: HashMap<String, DeckSynced>,
}

impl Session {
    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    /// Stamp `seq` + `from`, keep it for replay, deliver by `to`. Returns how many clients got it.
    pub fn publish(&mut self, from: &str, mut envelope: Envelope, now: i64) -> usize {
        envelope.seq = self.next_seq();
        envelope.from = from.to_owned();
        let (seq, to) = (envelope.seq, envelope.to.clone());
        let frame = Frame::of(envelope);
        let t = Instant::now();
        let mut delivered = 0;
        for (id, c) in &self.clients {
            if reaches(id, c.role, c.group.as_deref(), from, to.as_ref()) && c.outbox.push(frame.clone(), t) {
                delivered += 1;
            }
        }
        self.log.push_back(Logged { seq, t: now, from: from.to_owned(), to, frame });
        while self.log.len() > REPLAY_MESSAGES || self.log.front().is_some_and(|l| l.t < now - REPLAY_MS) {
            let dropped = self.log.pop_front().expect("non-empty");
            self.trimmed = dropped.seq;
        }
        delivered
    }

    /// A relay-authored message routed like any other (and kept for replay): vote.open, vote.result.
    fn relay_publish(&mut self, message: Message, to: Option<To>, now: i64) -> usize {
        let mut e = Envelope::new("relay", message);
        e.to = to;
        self.publish("relay", e, now)
    }

    /// A relay-authored message to exactly one client, not kept (welcome, pong, error, snapshot, presence, tally).
    pub fn send_to(&mut self, id: &str, message: Message) {
        let seq = self.next_seq();
        let envelope = Envelope { seq, ..Envelope::new("relay", message).to(To::client(id)) };
        if let Some(c) = self.clients.get(id) {
            c.outbox.push(Frame::of(envelope), Instant::now());
        }
    }

    fn error(&mut self, id: &str, code: &str, message: &str) {
        self.send_to(id, Message::Error(ProtocolError { code: code.into(), message: message.into() }));
    }

    pub fn join(&mut self, id: String, client: Client) {
        self.departed.remove(&id);
        self.clients.insert(id, client);
        self.empty_since = None;
    }

    pub fn leave(&mut self, id: &str, now: i64) {
        if let Some(c) = self.clients.remove(id) {
            self.departed.insert(id.to_owned(), Departed { role: c.role, group: c.group, at: now });
        }
        // a deck that left takes its being and its routes along (it sends its graph again when it comes back)
        self.decks.remove(id);
        self.synced.remove(id);
        self.routes.retain(|(a, b)| a != id && b != id);
        self.departed.retain(|_, d| d.at >= now - REPLAY_MS);
        if self.clients.is_empty() {
            self.empty_since = Some(now);
        } else {
            self.tell_presence();
        }
    }

    /// A `resume { client, seq }`: may the client take its old id again (it left, same role, nobody holds it)? Then
    /// `Some(replay)` – `replay` = whether the missed messages are still in the log (else it gets the snapshot).
    pub fn resume(&self, resume: &Resume, role: Role) -> Option<bool> {
        let d = self.departed.get(&resume.client)?;
        if d.role != role || self.clients.contains_key(&resume.client) {
            return None;
        }
        Some(resume.seq >= 0 && resume.seq as u64 >= self.trimmed)
    }

    /// The group a resumed client had (its QR's group stays unless the hello names another).
    pub fn departed_group(&self, id: &str) -> Option<String> {
        self.departed.get(id).and_then(|d| d.group.clone())
    }

    /// The logged messages after `seq` that reach the client `id`, in order (B.6 resume).
    pub fn missed(&self, id: &str, after: u64) -> Vec<String> {
        let Some(c) = self.clients.get(id) else { return vec![] };
        self.log
            .iter()
            .filter(|l| l.seq > after && reaches(id, c.role, c.group.as_deref(), &l.from, l.to.as_ref()))
            .map(|l| l.frame.text.clone())
            .collect()
    }

    /// Replay `missed` to the client (after its welcome).
    pub fn replay(&mut self, id: &str, after: u64) {
        let Some(c) = self.clients.get(id) else { return };
        let t = Instant::now();
        for l in self.log.iter().filter(|l| l.seq > after && reaches(id, c.role, c.group.as_deref(), &l.from, l.to.as_ref())) {
            c.outbox.push(l.frame.clone(), t);
        }
    }

    fn is_voter(&self, role: Role, group: Option<&str>) -> bool {
        let Some(v) = &self.vote else { return false };
        match &v.open.groups {
            None => role == Role::Audience,
            Some(_) => matches!(role, Role::Audience | Role::Performer | Role::Musician) && v.admits(group),
        }
    }

    /// What a client shows now (B.6 snapshot): the conductor (and the uplink) everything; an output / phone its own
    /// outlet's cell and its group's last instruction; the audience the position without the story variables (no
    /// spoilers); the open vote to its voters.
    pub fn snapshot_for(&self, role: Role, group: Option<&str>) -> Snapshot {
        let all = matches!(role, Role::Conductor | Role::Uplink);
        let state = self.state.clone().map(|mut s| {
            if role == Role::Audience {
                s.vars.clear();
                s.inventory = None;
            }
            s
        });
        fn pick<T: Clone>(m: &HashMap<String, T>, all: bool, group: Option<&str>) -> Option<HashMap<String, T>> {
            let picked: HashMap<String, T> = m
                .iter()
                .filter(|(outlet, _)| all || group == Some(outlet.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            (!picked.is_empty()).then_some(picked)
        }
        let bridge = role == Role::Bridge;
        Snapshot {
            state,
            cells: if bridge { None } else { pick(&self.cells, all, group) },
            media: if bridge { None } else { pick(&self.media, all, group) },
            vote: if all || self.is_voter(role, group) { self.vote.as_ref().map(|v| v.open.clone()) } else { None },
            say: if bridge { None } else { pick(&self.say, all, group) },
            decks: if all && !self.decks.is_empty() { Some(self.decks.clone()) } else { None },
            sync: self.sync_for(None, role, group),
            synced: if all && !self.synced.is_empty() { Some(self.synced.clone()) } else { None },
            seq: Some(self.seq as i64),
        }
    }

    /// #195: the last `scene.sync`, if it reaches this client – the conductor (and the uplink) always, a deck of the
    /// group `decks` when it went to every deck, a listed client by its id (`id`: known at the welcome).
    pub fn sync_for(&self, id: Option<&str>, role: Role, group: Option<&str>) -> Option<SceneSync> {
        let sync = self.sync.as_ref()?;
        let reaches = match &sync.decks {
            _ if matches!(role, Role::Conductor | Role::Uplink) => true,
            None => group == Some(DECKS) && matches!(role, Role::Musician | Role::Performer | Role::Output),
            Some(list) => id.is_some_and(|id| list.iter().any(|d| d == id)),
        };
        reaches.then(|| sync.clone())
    }

    /// B.6 presence to the conductor(s): clients per group (phones and outputs; `-` = no group) and who is ready.
    pub fn presence(&self) -> Presence {
        let mut groups: HashMap<String, i64> = HashMap::new();
        let mut ready = vec![];
        for (id, c) in &self.clients {
            if !matches!(c.role, Role::Conductor | Role::Bridge | Role::Uplink) {
                *groups.entry(c.group.clone().unwrap_or_else(|| "-".into())).or_default() += 1;
            }
            if c.ready {
                ready.push(id.clone());
            }
        }
        ready.sort();
        let mut decks: Vec<String> = self.decks.keys().cloned().collect();
        decks.sort();
        let routes = self.routes.iter().map(|(from, to)| Route { from: from.clone(), to: to.clone() }).collect();
        Presence { groups, ready, decks: Some(decks), routes: Some(routes) }
    }

    pub fn tell_presence(&mut self) {
        let p = self.presence();
        for id in self.ids_with_role(Role::Conductor) {
            self.send_to(&id, Message::Presence(p.clone()));
        }
    }

    fn ids_with_role(&self, role: Role) -> Vec<String> {
        let mut ids: Vec<String> = self.clients.iter().filter(|(_, c)| c.role == role).map(|(id, _)| id.clone()).collect();
        ids.sort();
        ids
    }

    /// One message from client `id`: authority, rate limit, fold into the snapshot, votes, the clock's lead, routing.
    /// `lead` = ms added to `at` for timed kinds that come without one (B.6: 50 LAN, 150 hosted).
    pub fn handle(&mut self, id: &str, mut e: Envelope, now: i64, lead: i64) -> After {
        let Some(sender) = self.clients.get_mut(id) else { return After::Nothing };
        let role = sender.role;
        if role == Role::Audience && !sender.bucket.take(now) {
            self.error(id, "rate", "5 messages per second");
            return After::Nothing;
        }
        let uplink = role == Role::Uplink;
        if conductor_only(&e.message) && !(role == Role::Conductor || uplink) {
            self.error(id, "authority", &format!("only the conductor sends {}", e.message.kind()));
            return After::Nothing;
        }
        if relay_only(&e.message) && !uplink {
            self.error(id, "authority", &format!("{} comes from the relay", e.message.kind()));
            return After::Nothing;
        }
        if matches!(e.message, Message::Devices(_) | Message::DeviceIn(_)) && role != Role::Bridge {
            self.error(id, "authority", "only the bridge sends device input");
            return After::Nothing;
        }
        if matches!(e.message, Message::DeckGraph(_) | Message::DeckValues(_) | Message::DeckSynced(_)) && matches!(role, Role::Audience | Role::Bridge) {
            self.error(id, "authority", "only a deck streams a being");
            return After::Nothing;
        }

        // B.6 clock: an effect without `at` happens `lead` ms from now, so every client can play it together
        let timed = matches!(
            e.message,
            Message::Cue(_)
                | Message::Cell(_)
                | Message::Say(_)
                | Message::Line(_)
                | Message::Fx(_)
                | Message::Colour(_)
                | Message::Panic(_)
                | Message::Device(_)
        );
        if timed && e.at.is_none() {
            e.at = Some(now + lead);
        }
        if let Message::Device(d) = &mut e.message {
            d.at = d.at.or(e.at);
        }

        match &e.message {
            Message::Ping(p) => {
                let pong = protocol::Pong { t0: p.t0, t1: now };
                self.send_to(id, Message::Pong(pong));
            }
            Message::Hello(_) => self.error(id, "hello", "already joined"),
            Message::State(s) => {
                self.state = Some(s.clone());
                self.publish(id, e, now);
            }
            Message::Cell(c) => {
                self.cells.insert(c.outlet.clone(), c.cell.clone());
                if e.to.is_none() {
                    e.to = Some(To::group(c.outlet.clone()));
                }
                self.publish(id, e, now);
            }
            Message::Say(p) | Message::Line(p) | Message::Fx(p) | Message::Colour(p) => {
                let outlet = outlet_of(p);
                if let (Message::Say(p), Some(o)) = (&e.message, &outlet) {
                    self.say.insert(o.clone(), p.clone());
                }
                if e.to.is_none() {
                    e.to = outlet.map(To::group);
                }
                self.publish(id, e, now);
            }
            Message::MediaPreload(m) => {
                self.media.insert(m.outlet.clone(), m.clone());
                if e.to.is_none() {
                    e.to = Some(To::group(m.outlet.clone()));
                }
                self.publish(id, e, now);
            }
            Message::Cue(c) if e.to.is_none() => {
                let c = c.clone();
                self.split_cue(id, &e, c, now);
            }
            Message::Panic(p) => {
                if p.black == Some(true) {
                    for cell in self.cells.values_mut() {
                        *cell = Cell { mode: CellMode::Black, layers: None, of: None, send: None };
                    }
                    self.say.clear();
                }
                self.publish(id, e, now);
            }
            Message::Device(_) => {
                if e.to.is_none() {
                    e.to = Some(To::role(Role::Bridge));
                }
                self.publish(id, e, now);
            }
            Message::VoteOpen(v) => {
                if v.options.is_empty() || self.vote.is_some() {
                    let why = if v.options.is_empty() { "a vote needs options" } else { "a vote is open" };
                    self.error(id, "vote", why);
                    return After::Nothing;
                }
                let v = v.clone();
                let groups = v.groups.clone();
                self.vote = Some(OpenVote::new(v.clone(), now));
                match groups {
                    Some(gs) => {
                        for g in gs {
                            self.relay_publish(Message::VoteOpen(v.clone()), Some(To::group(g)), now);
                        }
                    }
                    None => {
                        self.relay_publish(Message::VoteOpen(v), Some(To::role(Role::Audience)), now);
                    }
                }
                return After::VoteOpened;
            }
            Message::VoteCast(c) => {
                let group = self.clients.get(id).and_then(|c| c.group.clone());
                let voter = self.is_voter(role, group.as_deref());
                let Some(vote) = self.vote.as_mut().filter(|v| v.open.vote == c.vote) else {
                    self.error(id, "vote", "no such vote open");
                    return After::Nothing;
                };
                if !voter || c.option < 0 || c.option as usize >= vote.open.options.len() {
                    self.error(id, "vote", "not a voter / no such option");
                    return After::Nothing;
                }
                vote.casts.insert(id.to_owned(), (c.option as usize, group));
                if vote.reached().is_some() {
                    self.close_vote(None, now);
                }
            }
            Message::VoteClose(c) => {
                if self.vote.as_ref().is_some_and(|v| v.open.vote == c.vote) {
                    let o = c.r#override.filter(|&o| o >= 0).map(|o| o as usize);
                    self.close_vote(o, now);
                } else {
                    self.error(id, "vote", "no such vote open");
                }
            }
            Message::Assign(a) => {
                let a = a.clone();
                self.assign(&a);
            }
            Message::Ready(_) => {
                if let Some(c) = self.clients.get_mut(id) {
                    c.ready = true;
                }
                self.tell_presence();
            }
            Message::MediaReady(m) => {
                // #186: an output's preload answer – ready (for presence) once nothing is missing; the details to the
                // conductor (its ready light per outlet)
                let complete = m.missing.is_empty();
                if let Some(c) = self.clients.get_mut(id) {
                    c.ready = complete;
                }
                if e.to.is_none() {
                    e.to = Some(To::role(Role::Conductor));
                }
                self.publish(id, e, now);
                self.tell_presence();
            }
            Message::DeckGraph(g) => {
                self.decks.insert(id.to_owned(), DeckState { graph: g.clone(), values: HashMap::new(), mood: None });
                if e.to.is_none() {
                    e.to = Some(To::role(Role::Conductor));
                }
                self.publish(id, e, now);
                self.tell_presence();
            }
            Message::DeckValues(_) => self.deck_values(id, e, now),
            Message::SceneSync(sync) => {
                let sync = sync.clone();
                self.scene_sync(id, e, sync, now);
            }
            Message::DeckSynced(d) => {
                // #195: a deck's answer – kept for the conductor's snapshot (when it answers the sync the relay keeps),
                // ready in presence once everything is there; to the conductor
                let ready = d.state == SyncState::Ready;
                if self.sync.as_ref().map_or(true, |s| s.scene == d.scene) {
                    self.synced.insert(id.to_owned(), d.clone());
                }
                if e.to.is_none() {
                    e.to = Some(To::role(Role::Conductor));
                }
                self.publish(id, e, now);
                let changed = self.clients.get_mut(id).is_some_and(|c| std::mem::replace(&mut c.ready, ready) != ready);
                if changed {
                    self.tell_presence();
                }
            }
            Message::React(_) | Message::Pickup(_) | Message::Devices(_) | Message::DeviceIn(_) => {
                // to the conductor (B.6: pickups are checked against the API there, like the reader does – TODO with
                // the session API; reacts feed its meter; device input is mapped to actions there)
                if e.to.is_none() {
                    e.to = Some(To::role(Role::Conductor));
                }
                self.publish(id, e, now);
            }
            _ => {
                self.publish(id, e, now);
            }
        }
        After::Nothing
    }

    /// #188: a deck's values – over the rate: dropped; folded into its state; delivered (not logged) to the
    /// conductor(s) and to `to`; a new sender → receiver pair is a new route (presence).
    fn deck_values(&mut self, id: &str, mut e: Envelope, now: i64) {
        let Some(c) = self.clients.get_mut(id) else { return };
        if !c.values.take(now) {
            self.dropped += 1;
            return;
        }
        let Message::DeckValues(v) = &e.message else { return };
        let Some(deck) = self.decks.get_mut(id) else {
            self.error(id, "deck", "deck.graph first");
            return;
        };
        for (path, x) in &v.values {
            deck.values.insert(path.clone(), x.clone());
        }
        if v.mood.is_some() {
            deck.mood.clone_from(&v.mood);
        }
        e.seq = self.next_seq();
        e.from = id.to_owned();
        let to = e.to.clone();
        let frame = Frame::of(e);
        let t = Instant::now();
        let mut new_route = false;
        for (cid, c) in &self.clients {
            let addressed = to.is_some() && reaches(cid, c.role, c.group.as_deref(), id, to.as_ref());
            if (addressed || (c.role == Role::Conductor && cid != id)) && c.outbox.push(frame.clone(), t) {
                new_route |= self.routes.insert((id.to_owned(), cid.clone()));
            }
        }
        if new_route {
            self.tell_presence();
        }
    }

    /// #195: the conductor's push – logged and kept (the snapshot of a deck that joins later), the decks' answers to the
    /// one before forgotten; to the group `decks`, or one copy per listed client id. An explicit `to` is kept as it is.
    fn scene_sync(&mut self, from: &str, mut e: Envelope, sync: SceneSync, now: i64) {
        let list = |v: &Option<Vec<String>>, none: &str| v.as_ref().map_or_else(|| none.to_string(), |v| v.join(","));
        tracing::info!(
            scene = sync.scene.as_str(),
            sets = list(&sync.sets, "").as_str(),
            characters = list(&sync.characters, "").as_str(),
            decks = list(&sync.decks, "all").as_str(),
            "scene.sync from {from}"
        );
        for id in self.synced.keys() {
            if let Some(c) = self.clients.get_mut(id) {
                c.ready = false;
            }
        }
        let had = !self.synced.is_empty();
        self.synced.clear();
        self.sync = Some(sync.clone());
        match (&e.to, &sync.decks) {
            (None, Some(ids)) => {
                for id in ids {
                    let mut part = Envelope::new(from, Message::SceneSync(sync.clone()));
                    part.to = Some(To::client(id.clone()));
                    self.publish(from, part, now);
                }
            }
            _ => {
                if e.to.is_none() {
                    e.to = Some(To::group(DECKS));
                }
                self.publish(from, e, now);
            }
        }
        if had {
            self.tell_presence();
        }
    }

    /// B.6 cue: "the relay splits actions per outlet → `to`". A B.5.2 action is `{ <verb>: { outlet, … } }` (or a
    /// flat `{ outlet, … }`); actions per outlet go to that outlet's group, actions without one to everyone.
    fn split_cue(&mut self, from: &str, e: &Envelope, cue: Cue, now: i64) {
        let mut by_outlet: Vec<(Option<String>, Vec<Loose>)> = vec![];
        for action in cue.actions {
            let outlet = outlet_of(&action).or_else(|| action.values().find_map(|v| v.as_object().and_then(outlet_of)));
            if let Some(Value::Object(say)) = action.get("say") {
                if let Some(o) = &outlet {
                    self.say.insert(o.clone(), say.clone());
                }
            }
            match by_outlet.iter_mut().find(|(o, _)| *o == outlet) {
                Some((_, list)) => list.push(action),
                None => by_outlet.push((outlet, vec![action])),
            }
        }
        if by_outlet.is_empty() {
            by_outlet.push((None, vec![]));
        }
        for (outlet, actions) in by_outlet {
            let mut part = Envelope::new(from, Message::Cue(Cue { id: cue.id.clone(), actions }));
            part.at = e.at;
            part.to = outlet.map(To::group);
            self.publish(from, part, now);
        }
    }

    fn close_vote(&mut self, override_option: Option<usize>, now: i64) {
        if let Some(v) = self.vote.take() {
            let result = v.decide(override_option);
            self.relay_publish(Message::VoteResult(result), None, now);
        }
    }

    /// B.6 assign: move a client to another group – it gets the assign and its new snapshot; the conductor presence.
    fn assign(&mut self, a: &Assign) {
        let Some(c) = self.clients.get_mut(&a.client) else { return };
        c.group = Some(a.group.clone());
        let role = c.role;
        self.send_to(&a.client, Message::Assign(a.clone()));
        let snapshot = self.snapshot_for(role, Some(&a.group));
        self.send_to(&a.client, Message::Snapshot(snapshot));
        self.tell_presence();
    }

    /// Every 250 ms while a vote is open: the tally to the conductor (and the voters when the vote shows it); closes the
    /// vote at its deadline. Returns whether a vote is still open.
    pub fn tick(&mut self, now: i64) -> bool {
        let Some(v) = &self.vote else { return false };
        if now >= v.deadline {
            self.close_vote(None, now);
            return false;
        }
        let tally = VoteTally {
            vote: v.open.vote.clone(),
            counts: v.counts(),
            voters: v.casts.len() as i64,
            left: v.deadline - now,
        };
        let show = v.open.show == Some(true);
        let mut ids = self.ids_with_role(Role::Conductor);
        if show {
            let voters: Vec<String> = self
                .clients
                .iter()
                .filter(|(_, c)| self.is_voter(c.role, c.group.as_deref()))
                .map(|(id, _)| id.clone())
                .collect();
            ids.extend(voters);
        }
        for id in ids {
            self.send_to(&id, Message::VoteTally(tally.clone()));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{CellChange, MediaItem, MediaReady, Panic, VoteCast, VoteClose, VoteHow, VoteOpen, VoteRule};

    struct Room {
        s: Session,
        rx: HashMap<String, Arc<Outbox>>,
    }

    impl Room {
        fn new() -> Room {
            Room { s: Session::default(), rx: HashMap::new() }
        }

        fn add(&mut self, id: &str, role: Role, group: Option<&str>) {
            let outbox = Outbox::new();
            self.s.join(id.into(), Client::new(role, group.map(str::to_owned), None, outbox.clone(), 0));
            self.rx.insert(id.into(), outbox);
        }

        fn inbox(&mut self, id: &str) -> Vec<Envelope> {
            self.rx[id].drain().iter().map(|t| serde_json::from_str(t).unwrap()).collect()
        }

        fn kinds(&mut self, id: &str) -> Vec<&'static str> {
            self.inbox(id).iter().map(|e| e.message.kind()).collect()
        }

        fn send(&mut self, id: &str, m: Message, now: i64) -> After {
            self.s.handle(id, Envelope::new("", m), now, 50)
        }

        fn drain(&mut self) {
            let ids: Vec<String> = self.rx.keys().cloned().collect();
            for id in ids {
                self.inbox(&id);
            }
        }
    }

    fn room() -> Room {
        let mut r = Room::new();
        r.add("cond", Role::Conductor, None);
        r.add("outL", Role::Output, Some("left"));
        r.add("outR", Role::Output, Some("right"));
        r.add("a1", Role::Audience, Some("left"));
        r.add("a2", Role::Audience, Some("left"));
        r.add("a3", Role::Audience, Some("right"));
        r.add("bridge", Role::Bridge, None);
        r
    }

    fn cell(json: &str) -> Cell {
        serde_json::from_str(json).unwrap()
    }

    fn loose(json: &str) -> Loose {
        serde_json::from_str(json).unwrap()
    }

    fn vote(rule: VoteRule, groups: Option<Vec<&str>>) -> VoteOpen {
        VoteOpen {
            vote: "v1".into(),
            options: vec!["stay".into(), "go".into()],
            rule,
            n: Some(2),
            seconds: 10,
            fallback: None,
            groups: groups.map(|g| g.into_iter().map(str::to_owned).collect()),
            show: None,
        }
    }

    #[test]
    fn only_the_conductor_changes_state() {
        let mut r = room();
        r.send("a1", Message::Panic(Panic { black: Some(true), stop: None }), 0);
        let e = r.inbox("a1");
        assert!(matches!(&e[0].message, Message::Error(x) if x.code == "authority"), "{e:?}");
        assert!(r.inbox("outL").is_empty());
        // relay kinds come from nobody but the uplink
        r.send("outL", Message::Snapshot(Snapshot::default()), 0);
        assert_eq!(r.kinds("outL"), ["error"]);
        // device input only from the bridge
        r.send("cond", Message::Devices(protocol::Devices { ports: vec![] }), 0);
        assert_eq!(r.kinds("cond"), ["error"]);
    }

    #[test]
    fn audience_is_rate_limited() {
        let mut r = room();
        for _ in 0..5 {
            r.send("a1", Message::React(protocol::React { kind: "applause".into() }), 1_000);
        }
        assert_eq!(r.kinds("cond"), ["react"; 5], "reacts go to the conductor");
        r.send("a1", Message::React(protocol::React { kind: "applause".into() }), 1_000);
        assert_eq!(r.kinds("a1"), ["error"]);
        // 200 ms later one token is back
        r.send("a1", Message::React(protocol::React { kind: "applause".into() }), 1_200);
        assert_eq!(r.kinds("cond"), ["react"]);
        // the conductor is never limited
        for _ in 0..20 {
            r.send("cond", Message::Ping(protocol::Ping { t0: 1 }), 1_200);
        }
        assert_eq!(r.inbox("cond").len(), 20);
    }

    #[test]
    fn timed_kinds_get_the_lead() {
        let mut r = room();
        r.send("cond", Message::Cell(CellChange { outlet: "left".into(), cell: cell(r#"{"mode":"black"}"#) }), 1_000);
        let e = r.inbox("outL");
        assert_eq!(e[0].at, Some(1_050));
        assert!(r.inbox("outR").is_empty(), "a cell goes to its outlet only");
        r.drain();
        // an explicit `at` stays
        let mut env = Envelope::new("", Message::Say(loose(r#"{"outlet":"right","text":"stand up"}"#)));
        env.at = Some(5_000);
        r.s.handle("cond", env, 1_000, 50);
        let e = r.inbox("a3");
        assert_eq!(e[0].at, Some(5_000));
        assert!(r.inbox("a1").is_empty());
    }

    #[test]
    fn cue_splits_per_outlet() {
        let mut r = room();
        let actions = vec![
            loose(r#"{"say":{"outlet":"left","text":{"de":"Aufstehen"}}}"#),
            loose(r#"{"stage":{"outlet":"right","layer":"l1","show":true}}"#),
            loose(r#"{"say":{"outlet":"left","text":{"de":"Klatschen"}}}"#),
        ];
        r.send("cond", Message::Cue(Cue { id: "c1".into(), actions }), 0);
        let left = r.inbox("a1");
        let Message::Cue(c) = &left[0].message else { panic!() };
        assert_eq!(c.actions.len(), 2, "both left actions in one message");
        assert_eq!(r.inbox("outR").len(), 1);
        assert_eq!(r.inbox("a3").len(), 1, "the right audience is group right too");
        // the last say per group is in the snapshot
        let s = r.s.snapshot_for(Role::Audience, Some("left"));
        assert_eq!(s.say.unwrap()["left"]["text"]["de"], "Klatschen");
    }

    #[test]
    fn snapshots_per_role() {
        let mut r = room();
        let mut vars = HashMap::new();
        vars.insert("mood".to_owned(), 2.0);
        r.send("cond", Message::State(State { row: 3, slice: 12, vars, inventory: Some(vec!["lp".into()]) }), 0);
        r.send("cond", Message::Cell(CellChange { outlet: "left".into(), cell: cell(r#"{"mode":"own"}"#) }), 0);
        r.send("cond", Message::Cell(CellChange { outlet: "right".into(), cell: cell(r#"{"mode":"black"}"#) }), 0);
        let c = r.s.snapshot_for(Role::Conductor, None);
        assert_eq!(c.cells.as_ref().unwrap().len(), 2);
        assert_eq!(c.state.as_ref().unwrap().vars["mood"], 2.0);
        let o = r.s.snapshot_for(Role::Output, Some("left"));
        assert_eq!(o.cells.unwrap().keys().collect::<Vec<_>>(), ["left"]);
        let a = r.s.snapshot_for(Role::Audience, Some("right"));
        assert!(a.state.as_ref().unwrap().vars.is_empty(), "no story variables on phones");
        assert_eq!(a.state.unwrap().slice, 12);
        // panic black turns every known cell black
        r.send("cond", Message::Panic(Panic { black: Some(true), stop: None }), 0);
        let o = r.s.snapshot_for(Role::Output, Some("left"));
        assert_eq!(o.cells.unwrap()["left"].mode, CellMode::Black);
    }

    /// #186: the outlet's preload stays for its snapshot (a deck that reconnects resolves its cell's media ids); the
    /// output's `media.ready` reaches the conductor, and the output counts as ready once nothing is missing
    #[test]
    fn media_preload_and_ready() {
        let mut r = room();
        let item = |id: &str| MediaItem { id: id.into(), url: Some(format!("https://cdn/{id}.webm")), set: None, spot: None, mode: None, bytes: None };
        let preload = protocol::MediaPreload { outlet: "left".into(), items: vec![item("choir"), item("organ")] };
        r.send("cond", Message::MediaPreload(preload.clone()), 0);
        assert_eq!(r.kinds("outL"), ["media.preload"]);
        assert!(r.kinds("outR").is_empty(), "another outlet's preload");
        assert_eq!(r.s.snapshot_for(Role::Output, Some("left")).media.unwrap()["left"], preload);
        assert!(r.s.snapshot_for(Role::Output, Some("right")).media.is_none());
        r.drain();

        let ready = |missing: &[&str]| MediaReady {
            outlet: "left".into(),
            ready: vec!["organ".into()],
            missing: missing.iter().map(|m| m.to_string()).collect(),
            why: None,
        };
        r.send("outL", Message::MediaReady(ready(&["choir"])), 0);
        let inbox = r.inbox("cond");
        assert!(inbox.iter().any(|e| matches!(&e.message, Message::MediaReady(m) if m.missing == ["choir"])));
        assert!(r.s.presence().ready.is_empty(), "something is missing");
        assert!(r.kinds("outR").is_empty(), "the answer goes to the conductor only");
        r.send("outL", Message::MediaReady(ready(&[])), 0);
        assert_eq!(r.s.presence().ready, ["outL"]);
        // an audience phone may not preload for others
        assert_eq!(r.send("a1", Message::MediaPreload(preload), 0), After::Nothing);
        assert!(r.kinds("a1").contains(&"error"));
    }

    #[test]
    fn vote_majority_closes_at_the_deadline() {
        let mut r = room();
        assert_eq!(r.send("cond", Message::VoteOpen(vote(VoteRule::Majority, None)), 1_000), After::VoteOpened);
        assert_eq!(r.kinds("a1"), ["vote.open"]);
        assert!(r.inbox("outL").is_empty(), "outputs do not vote");
        r.send("a1", Message::VoteCast(VoteCast { vote: "v1".into(), option: 1 }), 2_000);
        r.send("a2", Message::VoteCast(VoteCast { vote: "v1".into(), option: 1 }), 2_000);
        r.send("a3", Message::VoteCast(VoteCast { vote: "v1".into(), option: 0 }), 2_000);
        r.send("outL", Message::VoteCast(VoteCast { vote: "v1".into(), option: 0 }), 2_000);
        assert_eq!(r.kinds("outL"), ["error"], "an output is no voter");
        assert!(r.s.tick(2_250));
        let t = r.inbox("cond");
        let Message::VoteTally(t) = &t[0].message else { panic!("{t:?}") };
        assert_eq!((t.counts.clone(), t.voters, t.left), (vec![1, 2], 3, 8_750));
        assert!(r.inbox("a1").is_empty(), "the tally is hidden from the audience by default");
        assert!(!r.s.tick(11_000));
        let res = r.inbox("outL");
        let Message::VoteResult(v) = &res[0].message else { panic!("{res:?}") };
        assert_eq!((v.option, v.how), (1, VoteHow::Rule));
        assert!(r.s.vote.is_none());
    }

    #[test]
    fn vote_first_to_n_and_override() {
        let mut r = room();
        r.send("cond", Message::VoteOpen(vote(VoteRule::First, None)), 0);
        r.drain();
        r.send("a1", Message::VoteCast(VoteCast { vote: "v1".into(), option: 0 }), 10);
        assert!(r.s.vote.is_some());
        r.send("a3", Message::VoteCast(VoteCast { vote: "v1".into(), option: 0 }), 10);
        assert!(r.s.vote.is_none(), "closed when option 0 reached n = 2");
        assert_eq!(r.kinds("cond"), ["vote.result"]);
        r.drain();

        r.send("cond", Message::VoteOpen(vote(VoteRule::Majority, Some(vec!["right"]))), 20);
        assert_eq!(r.kinds("a3"), ["vote.open"]);
        assert!(r.inbox("a1").is_empty(), "only the right group votes");
        r.send("a1", Message::VoteCast(VoteCast { vote: "v1".into(), option: 1 }), 30);
        assert_eq!(r.kinds("a1"), ["error"]);
        r.send("cond", Message::VoteClose(VoteClose { vote: "v1".into(), r#override: Some(1) }), 40);
        let res = r.inbox("a1");
        let Message::VoteResult(v) = &res[0].message else { panic!() };
        assert_eq!((v.option, v.how), (1, VoteHow::Override));
    }

    #[test]
    fn resume_replays_what_the_client_missed() {
        let mut r = room();
        r.send("cond", Message::Say(loose(r#"{"outlet":"left","text":"one"}"#)), 0);
        let seen = r.s.seq;
        r.drain();
        r.s.leave("a1", 100);
        r.send("cond", Message::Say(loose(r#"{"outlet":"left","text":"two"}"#)), 200);
        r.send("cond", Message::Say(loose(r#"{"outlet":"right","text":"not yours"}"#)), 200);
        assert_eq!(r.s.resume(&Resume { client: "a1".into(), seq: seen as i64 }, Role::Audience), Some(true));
        assert_eq!(r.s.resume(&Resume { client: "a1".into(), seq: seen as i64 }, Role::Conductor), None, "same role only");
        assert_eq!(r.s.departed_group("a1").as_deref(), Some("left"));
        r.add("a1", Role::Audience, Some("left"));
        let missed = r.s.missed("a1", seen);
        assert_eq!(missed.len(), 1);
        assert!(missed[0].contains("two"));
    }

    #[test]
    fn replay_log_is_bounded() {
        let mut r = room();
        r.s.leave("a1", 0);
        for i in 0..(REPLAY_MESSAGES as i64 + 10) {
            r.send("cond", Message::Say(loose(r#"{"outlet":"left","text":"x"}"#)), i);
        }
        assert_eq!(r.s.resume(&Resume { client: "a1".into(), seq: 1 }, Role::Audience), Some(false), "too old: snapshot");
        // and by time: one message 5 min later drops the rest
        r.send("cond", Message::Say(loose(r#"{"outlet":"left","text":"late"}"#)), REPLAY_MS + 1_000);
        assert_eq!(r.s.log.len(), 1);
    }

    #[test]
    fn presence_and_assign() {
        let mut r = room();
        r.send("a1", Message::Ready(protocol::Empty {}), 0);
        let p = r.inbox("cond");
        let Message::Presence(p) = &p[0].message else { panic!() };
        assert_eq!(p.groups["left"], 3, "two phones + the left output");
        assert_eq!(p.ready, ["a1"]);
        r.send("cond", Message::Assign(Assign { client: "a2".into(), group: "right".into() }), 0);
        assert_eq!(r.kinds("a2"), ["assign", "snapshot"]);
        assert_eq!(r.s.presence().groups["right"], 3);
    }

    fn graph() -> protocol::DeckGraph {
        serde_json::from_str(r#"{"nodes":[{"id":"space","kind":"space"},{"id":"lfo","kind":"lfo"}],"links":[{"from":"lfo.out","to":"space.hue"}],"character":"catpope"}"#).unwrap()
    }

    fn values(pairs: &[(&str, f64)], mood: Option<&str>) -> Message {
        let values = pairs.iter().map(|(k, v)| (k.to_string(), vec![*v])).collect();
        Message::DeckValues(protocol::DeckValues { values, mood: mood.map(str::to_owned) })
    }

    /// #188 phase 4: a deck's graph reaches the conductor and stays for its snapshot; its values go to the conductor
    /// only (and to `to`), are folded, never logged; presence names the decks and the routes; a deck that leaves takes
    /// its being and its routes along
    #[test]
    fn decks_stream_their_beings() {
        let mut r = room();
        r.add("d1", Role::Musician, Some("decks"));
        r.add("d2", Role::Musician, Some("decks"));
        r.drain();
        r.send("d1", values(&[("lfo.rate", 2.0)], None), 0);
        assert!(matches!(&r.inbox("d1")[0].message, Message::Error(e) if e.code == "deck"), "the graph first");
        r.send("d1", Message::DeckGraph(graph()), 0);
        let c = r.inbox("cond");
        assert!(matches!(&c[0].message, Message::DeckGraph(g) if g.character.as_deref() == Some("catpope")));
        let Message::Presence(p) = &c[1].message else { panic!("{c:?}") };
        assert_eq!(p.decks.as_deref(), Some(&["d1".to_owned()][..]));
        assert!(r.inbox("d2").is_empty() && r.inbox("outL").is_empty() && r.inbox("a1").is_empty(), "to the conductor only");
        let logged = r.s.log.len();
        r.send("d1", values(&[("lfo.rate", 2.0), ("space.hue", 0.3)], Some("happy")), 10);
        let c = r.inbox("cond");
        assert!(matches!(&c[0].message, Message::DeckValues(v) if v.values["lfo.rate"] == [2.0] && c[0].from == "d1"));
        let Message::Presence(p) = &c[1].message else { panic!("a new route: {c:?}") };
        assert_eq!(p.routes.as_ref().unwrap(), &[Route { from: "d1".into(), to: "cond".into() }]);
        assert!(r.inbox("d2").is_empty());
        assert_eq!(r.s.log.len(), logged, "values are not kept for replay");
        r.send("d1", values(&[("space.hue", 0.4)], None), 20);
        assert_eq!(r.kinds("cond"), ["deck.values"], "a known route: no presence");
        // the snapshot folds every value and the last mood – the conductor's only
        let snap = r.s.snapshot_for(Role::Conductor, None);
        let d1 = &snap.decks.as_ref().unwrap()["d1"];
        assert_eq!((d1.values["lfo.rate"].clone(), d1.values["space.hue"].clone(), d1.mood.as_deref()), (vec![2.0], vec![0.4], Some("happy")));
        assert!(r.s.snapshot_for(Role::Output, Some("left")).decks.is_none());
        // a deck may send its values to another deck as well: a route between decks
        r.send("d2", Message::DeckGraph(graph()), 30);
        r.drain();
        r.s.handle("d2", Envelope::new("", values(&[("lfo.depth", 0.5)], None)).to(To::client("d1")), 40, 50);
        assert_eq!(r.kinds("d1"), ["deck.values"]);
        assert!(r.kinds("cond").contains(&"deck.values"), "the conductor sees every stream");
        let routes = r.s.presence().routes.unwrap();
        assert_eq!(routes.len(), 3, "{routes:?}");
        assert!(routes.contains(&Route { from: "d2".into(), to: "d1".into() }));
        // the rate: a burst of 15, then 30 / s
        r.drain();
        let skipped = r.rx["cond"].skipped();
        for i in 0..40 {
            r.send("d1", values(&[("space.hue", i as f64 / 40.0)], None), 1_000);
        }
        assert_eq!(r.s.dropped, 25);
        // #223: the 15 that passed wait in the conductor's outbox as one – the newest (14 / 40) wins
        let got = r.inbox("cond");
        assert!(matches!(&got[..], [e] if matches!(&e.message, Message::DeckValues(v) if v.values["space.hue"] == [14.0 / 40.0])), "{got:?}");
        assert_eq!(r.rx["cond"].skipped() - skipped, 14);
        r.send("d1", values(&[("space.hue", 0.9)], None), 1_100);
        assert_eq!(r.inbox("cond").len(), 1, "100 ms later: three tokens back");
        // audience phones and the bridge don't stream beings
        r.send("a1", Message::DeckGraph(graph()), 2_000);
        assert_eq!(r.kinds("a1"), ["error"]);
        // a deck that leaves
        r.s.leave("d1", 3_000);
        let p = r.s.presence();
        assert_eq!(p.decks.unwrap(), ["d2"]);
        assert!(p.routes.unwrap().iter().all(|x| x.from != "d1" && x.to != "d1"));
        assert!(!r.s.snapshot_for(Role::Conductor, None).decks.unwrap().contains_key("d1"));
    }

    fn push(decks: Option<Vec<&str>>) -> SceneSync {
        SceneSync {
            scene: "cathedral".into(),
            sets: Some(vec!["cathedral".into()]),
            characters: Some(vec!["catpope".into()]),
            beings: None,
            decks: decks.map(|d| d.into_iter().map(str::to_owned).collect()),
        }
    }

    fn synced(state: SyncState, missing: &[&str]) -> DeckSynced {
        DeckSynced {
            scene: "cathedral".into(),
            state,
            done: Some(2),
            total: Some(2),
            bytes: Some(10),
            ready: vec!["set:cathedral".into()],
            missing: missing.iter().map(|m| m.to_string()).collect(),
            why: None,
        }
    }

    /// #195: only the conductor pushes; the push reaches every deck (the group `decks`), not the outlets or the
    /// audience; it is kept for the snapshot of a deck that joins later, and for the conductor's
    #[test]
    fn scene_sync_reaches_the_decks_and_later_ones() {
        let mut r = room();
        r.add("d1", Role::Musician, Some(DECKS));
        r.add("d2", Role::Musician, Some(DECKS));
        r.drain();
        r.send("d1", Message::SceneSync(push(None)), 0);
        assert!(matches!(&r.inbox("d1")[0].message, Message::Error(x) if x.code == "authority"), "a deck may not push");
        assert!(r.inbox("d2").is_empty());
        assert!(r.s.snapshot_for(Role::Musician, Some(DECKS)).sync.is_none(), "a refused push is not kept");

        r.send("cond", Message::SceneSync(push(None)), 0);
        for d in ["d1", "d2"] {
            let e = r.inbox(d);
            assert!(matches!(&e[0].message, Message::SceneSync(s) if s.scene == "cathedral"), "{d}: {e:?}");
            assert_eq!(e[0].from, "cond");
        }
        for other in ["outL", "a1", "bridge"] {
            assert!(r.kinds(other).is_empty(), "{other}");
        }
        assert_eq!(r.s.log.back().map(|l| l.frame.text.contains("scene.sync")), Some(true), "logged for a resume");
        // a deck that joins later: in its snapshot; an outlet's screen and the audience: not
        assert_eq!(r.s.snapshot_for(Role::Musician, Some(DECKS)).sync, Some(push(None)));
        assert!(r.s.snapshot_for(Role::Output, Some("left")).sync.is_none());
        assert!(r.s.snapshot_for(Role::Audience, Some(DECKS)).sync.is_none());
        assert_eq!(r.s.snapshot_for(Role::Conductor, None).sync, Some(push(None)));
    }

    /// #195: a push to a `decks` list reaches only those decks (one copy each); a later deck isn't listed
    #[test]
    fn scene_sync_to_a_list() {
        let mut r = room();
        for d in ["d1", "d2", "d3"] {
            r.add(d, Role::Musician, Some(DECKS));
        }
        r.drain();
        r.send("cond", Message::SceneSync(push(Some(vec!["d1", "d3"]))), 0);
        assert_eq!(r.kinds("d1"), ["scene.sync"]);
        assert!(r.kinds("d2").is_empty());
        assert_eq!(r.kinds("d3"), ["scene.sync"]);
        assert!(r.s.snapshot_for(Role::Musician, Some(DECKS)).sync.is_none(), "a new client id is not on the list");
        assert!(r.s.sync_for(Some("d3"), Role::Musician, Some(DECKS)).is_some(), "a listed one by its id");
    }

    /// #195: a deck's answers go to the conductor, the last one is kept for its snapshot, ready shows in presence; a
    /// new push forgets the answers to the old one; a deck that leaves takes its answer along
    #[test]
    fn deck_synced_folds_for_the_conductor() {
        let mut r = room();
        r.add("d1", Role::Musician, Some(DECKS));
        r.add("d2", Role::Musician, Some(DECKS));
        r.send("cond", Message::SceneSync(push(None)), 0);
        r.drain();
        r.send("d1", Message::DeckSynced(synced(SyncState::Syncing, &[])), 0);
        let got = r.inbox("cond");
        assert!(got.iter().any(|e| matches!(&e.message, Message::DeckSynced(d) if d.state == SyncState::Syncing) && e.from == "d1"));
        assert!(r.kinds("d2").is_empty(), "to the conductor only");
        assert!(r.s.presence().ready.is_empty());
        r.send("d1", Message::DeckSynced(synced(SyncState::Ready, &[])), 0);
        r.send("d2", Message::DeckSynced(synced(SyncState::Missing, &["character:catpope"])), 0);
        assert_eq!(r.s.presence().ready, ["d1"]);
        let snap = r.s.snapshot_for(Role::Conductor, None).synced.unwrap();
        assert_eq!((snap["d1"].state, snap["d2"].missing.clone()), (SyncState::Ready, vec!["character:catpope".to_string()]));
        assert!(r.s.snapshot_for(Role::Musician, Some(DECKS)).synced.is_none(), "the conductor's view only");
        // the audience can't answer
        r.send("a1", Message::DeckSynced(synced(SyncState::Ready, &[])), 0);
        assert!(r.kinds("a1").contains(&"error"));
        // a deck leaves: its answer goes
        r.s.leave("d2", 1);
        assert!(!r.s.snapshot_for(Role::Conductor, None).synced.unwrap().contains_key("d2"));
        // a new push: the old answers are forgotten, nobody is ready for it yet
        r.send("cond", Message::SceneSync(SceneSync { scene: "sermon".into(), ..push(None) }), 2);
        assert!(r.s.snapshot_for(Role::Conductor, None).synced.is_none());
        assert!(r.s.presence().ready.is_empty());
        // a late answer to the old scene still reaches the conductor but isn't kept
        r.drain();
        r.send("d1", Message::DeckSynced(synced(SyncState::Ready, &[])), 3);
        assert_eq!(r.kinds("cond").first(), Some(&"deck.synced"));
        assert!(r.s.snapshot_for(Role::Conductor, None).synced.is_none());
    }

    #[test]
    fn nicknames() {
        let block = vec!["badword".to_owned()];
        assert_eq!(clean_nick(Some("  Mo  "), &block).as_deref(), Some("Mo"));
        assert_eq!(clean_nick(Some("abcdefghijklmnopqrstuvwxyz"), &block).unwrap().chars().count(), 24);
        assert_eq!(clean_nick(Some("xBadWordx"), &block), None);
        assert_eq!(clean_nick(Some("   "), &block), None);
        assert_eq!(clean_nick(None, &block), None);
    }
}
