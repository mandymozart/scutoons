//! The relay client (#366): one sans-IO state machine every client of the relay drives – the native deck's link and
//! the Live outlet (deck-native: a tungstenite thread), the browser deck and the vote screens (deck/relay-web as wasm
//! under live-deck client.ts: a `WebSocket` and a timer) and the headless host. The core decides; an adapter moves bytes.
//!
//! - **In:** the socket opened, closed (or a dial failed), a message's text; `tick` at `next_wake`; what the app hands
//!   over (a deck's values, its being, render quality, sync answer, a leader's tempo, a vote's moves). Time is the
//!   adapter's clock in ms (the browser: ms since the epoch; the deck: `now_ns` in ms) – pings carry it in µs.
//! - **Out** (`take`): dial a URL, send a text, close the socket; the status (`Status`, its HUD line `line`); what
//!   the app hears – joined, a pong stamped on arrival, the show clock's anchor, the conductor's push, the session's
//!   decks (`In`), what a vote screen hears, every envelope (`messages`, the outlet's composer).
//! - **The join:** `hello` first (`resume { client, seq }` for a client that resumes – the outlet); on `welcome` a deck
//!   sends its being (`deck.graph`), a keyframe of every value it sent so far, its render quality and its last sync
//!   answer – the relay forgot them when it left; then only changes. A conductor's push goes again when the
//!   snapshot lacks it.
//! - **Keeping it:** a ping every `PING_MS` (the keepalive; the show clock asks for more, `ping`); a lost joined link
//!   joins again after `REJOIN_MS`, a failed one backs off `BACKOFF_S`; a refused token closes it.
//! - **The stream** (`Values`): a deck's values at most `RATE` a second, a value only when it moved more than its
//!   epsilon (`rig::Stream::epsilon`), the mood when it changed.
//!
//! `script` runs scripted socket events through a `Client` – the outs every adapter's test compares against
//! (shared/live-deck/test/fixtures/relay.json, the headless scenario `relay_join`).
use crate::rig::Stream;
use protocol::{
    DeckGraph, DeckMidi, DeckQuality, DeckSynced, DeckValues, Envelope, Hello, LinkQuality, Message, Ping, Resume, Role, SceneSync, Snapshot, SyncState, Tempo, VoteCast, VoteClose,
    VoteHold, VoteOpen,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// the keepalive: a ping at least this often (ms)
pub const PING_MS: f64 = 20_000.0;
/// a link that never got a welcome dials again after these waits (s), the last one from then on
pub const BACKOFF_S: [u32; 5] = [1, 2, 4, 8, 10];
/// a joined link that dropped dials again after this (ms) – the relay frees the client id for a resume
pub const REJOIN_MS: f64 = 250.0;
/// a deck's sync progress at most this often (ms); the final state goes at once
pub const PROGRESS_MS: f64 = 200.0;
/// at most this many `deck.values` a second (the relay drops more than 30 / s)
pub const RATE: f64 = 30.0;
/// the decks' group
pub const DECKS: &str = "decks";

/// `ws://host:port/live/<code>` (or `wss://`, a path before `/live/` – the LAN viewer's `…/relay/live/<code>`) → the
/// base and the session code; a query is dropped (the client adds its own)
pub fn parse_relay(url: &str) -> Result<(String, String), String> {
    let url = url.trim();
    let rest = url.strip_prefix("wss://").or_else(|| url.strip_prefix("ws://")).ok_or_else(|| format!("{url}: the relay is a ws:// or wss:// URL"))?;
    let path = rest.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
    let no_code = || format!("{url}: no session code (ws://host:7700/live/<code>)");
    let (base, code) = path.rsplit_once("/live/").ok_or_else(no_code)?;
    if base.split('/').next().unwrap_or("").is_empty() {
        return Err(format!("{url}: no host"));
    }
    if code.is_empty() || code.contains('/') {
        return Err(no_code());
    }
    let scheme = &url[..url.len() - rest.len()];
    Ok((format!("{scheme}{base}"), decode(code)))
}

/// %XX → the byte (a session code in a URL)
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |k: usize| b.get(k).and_then(|c| (*c as char).to_digit(16));
        match (b[i], hex(i + 1), hex(i + 2)) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (c, ..) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// a URL component: everything but the unreserved characters as %XX
fn encode(s: &str) -> String {
    s.bytes().map(|c| if c.is_ascii_alphanumeric() || b"-_.~".contains(&c) { (c as char).to_string() } else { format!("%{c:02X}") }).collect()
}

/// the role's name on the wire
fn role_name(r: Role) -> String {
    serde_json::to_value(r).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

/// How a client joins.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Options {
    /// `ws://host:port/live/<code>`
    pub relay: String,
    pub role: Role,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub nick: Option<String>,
    /// the session token (a conductor without one says `dev` – a dev relay takes any)
    #[serde(default)]
    pub token: Option<String>,
    /// join again with `resume { client, seq }` (the relay replays what was missed)
    #[serde(default)]
    pub resume: bool,
    /// hand the app every envelope as it came (`Out::Message`)
    #[serde(default)]
    pub messages: bool,
}

impl Options {
    pub fn new(relay: &str, role: Role, group: Option<&str>) -> Options {
        Options { relay: relay.into(), role, group: group.map(String::from), nick: None, token: None, resume: false, messages: false }
    }
}

/// What the conductor's frame (and any deck that looks) learns of the session.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum In {
    /// this stage's client id
    Me(String),
    /// a deck's being (joined, or its scene changed); its values and mood so far
    Deck { client: String, graph: DeckGraph, values: HashMap<String, Vec<f64>>, mood: Option<String> },
    Values { client: String, values: HashMap<String, Vec<f64>>, mood: Option<String> },
    /// the decks there are and who sends to whom; each client's link (#223) and each deck's render quality (#220)
    Presence { decks: Vec<String>, routes: Vec<(String, String)>, links: HashMap<String, LinkQuality>, quality: HashMap<String, DeckQuality> },
    /// #195: the session's scene sync (the relay's snapshot, another conductor's push)
    Pushed(SceneSync),
    /// #195: a deck's answer to it
    Synced { client: String, synced: DeckSynced },
}

/// where the link stands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Joining,
    Connected,
    Offline,
    Refused,
}

/// The link's state for a page and the HUD (`Client::line` words it).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub state: State,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    /// why it is offline or refused
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// offline without a welcome: the seconds until the next try
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

/// What the client asks of its adapter and tells its app, in order.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Out {
    /// open a socket at this URL (then `opened`, or `closed` when it fails)
    Dial(String),
    /// send this text on the open socket
    Send(String),
    /// close the socket (then `closed`)
    Close {},
    Status(Status),
    /// welcomed as `client` at the adapter's ms
    Joined { client: String, at: f64 },
    /// #321: a pong – its ping's `t0` (µs), the relay's `t1` (ms), back at the adapter's `t2` ms
    Pong { t0: i64, t1: i64, t2: f64 },
    /// #321: the show clock's anchor, arrived at the adapter's ms
    Tempo { tempo: Tempo, arrived: f64 },
    /// #195: the conductor's push for this deck (the welcome snapshot's, then each new one)
    Sync(SceneSync),
    /// the session's decks as a conductor sees them
    Deck(In),
    /// what a vote screen hears (#231): the open vote and the group's instruction from the welcome, the votes, errors
    Vote(Envelope),
    /// every envelope as it came (`Options::messages`)
    Message(Envelope),
}

/// Where a value's change is due: at most `RATE` samples a second; a frame may come a hair early, a long gap starts the
/// schedule anew (native stream.rs and the browser's values alike).
#[derive(Debug, Clone, Default)]
pub struct Pace {
    next: f64,
}

impl Pace {
    /// a sample is due at `time` (s): the next slot is set
    pub fn due(&mut self, time: f64) -> bool {
        if time + 1e-3 < self.next {
            return false;
        }
        self.next = if time - self.next > 1.0 / RATE { time + 1.0 / RATE } else { self.next + 1.0 / RATE };
        true
    }
}

/// A deck's values by path (the browser reads them from its engine): one sample's changes at the pace, past each path's
/// epsilon, the mood when it changed.
#[derive(Debug, Clone, Default)]
pub struct Values {
    paths: Vec<String>,
    eps: Vec<f64>,
    last: Vec<f64>,
    pace: Pace,
    mood: Option<String>,
}

impl Values {
    pub fn new(streamed: &[(String, Stream)]) -> Values {
        Values {
            paths: streamed.iter().map(|(p, _)| p.clone()).collect(),
            eps: streamed.iter().map(|(_, s)| f64::from(s.epsilon())).collect(),
            last: vec![f64::NAN; streamed.len()],
            pace: Pace::default(),
            mood: None,
        }
    }

    /// the paths in the order `sample` reads them
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// At `time` (s), `read` = each path's value now (a missing or non-finite one is skipped): None between two slots
    /// or when nothing moved, else what moved since it was last sent (the first sample: all).
    pub fn sample(&mut self, time: f64, read: &[f64], mood: Option<&str>) -> Option<DeckValues> {
        if !self.pace.due(time) {
            return None;
        }
        let mut values = HashMap::new();
        for (k, path) in self.paths.iter().enumerate() {
            let Some(&v) = read.get(k).filter(|v| v.is_finite()) else { continue };
            if self.last[k].is_nan() || (v - self.last[k]).abs() > self.eps[k] {
                self.last[k] = v;
                values.insert(path.clone(), vec![v]);
            }
        }
        let mood = mood.filter(|m| self.mood.as_deref() != Some(*m)).map(String::from);
        if mood.is_some() {
            self.mood.clone_from(&mood);
        }
        (!values.is_empty() || mood.is_some()).then_some(DeckValues { values, mood })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Dialing,
    Open,
    /// dial again at this ms
    Waiting(f64),
    Stopped,
}

/// The relay client of one app. Drive it: `start`, then the socket's `opened` / `closed` / `text`, `tick` at
/// `next_wake`; after each call `take` what it asks and says.
pub struct Client {
    o: Options,
    base: String,
    code: String,
    phase: Phase,
    tries: usize,
    joined: bool,
    client: String,
    /// the highest `seq` seen (the resume's)
    seq: i64,
    resume: Option<Resume>,
    last_ping: f64,
    status: Status,
    /// the relay's last error while joined (the line shows it)
    note: Option<String>,
    out: Vec<Out>,
    /// what went out per message kind (tests, the HUD)
    counts: BTreeMap<&'static str, u32>,
    // a deck: its being, everything it sent (the keyframe), its mood, render quality and sync answer
    graph: Option<DeckGraph>,
    sent: BTreeMap<String, Vec<f64>>,
    mood: Option<String>,
    stream: Values,
    quality: Option<DeckQuality>,
    quality_due: bool,
    answer: Option<DeckSynced>,
    answer_due: bool,
    answered: f64,
    // a conductor: its push and whether it still has to go out
    push: Option<SceneSync>,
    push_due: bool,
    // a vote screen: the vote open now
    vote: Option<VoteOpen>,
}

impl Client {
    pub fn new(o: Options) -> Result<Client, String> {
        let (base, code) = parse_relay(&o.relay)?;
        Ok(Client {
            o,
            base,
            code,
            phase: Phase::Idle,
            tries: 0,
            joined: false,
            client: String::new(),
            seq: 0,
            resume: None,
            last_ping: 0.0,
            status: Status { state: State::Joining, client: None, why: None, wait: None },
            note: None,
            out: Vec::new(),
            counts: BTreeMap::new(),
            graph: None,
            sent: BTreeMap::new(),
            mood: None,
            stream: Values::default(),
            quality: None,
            quality_due: false,
            answer: None,
            answer_due: false,
            answered: f64::NEG_INFINITY,
            push: None,
            push_due: false,
            vote: None,
        })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn client(&self) -> &str {
        &self.client
    }

    pub fn joined(&self) -> bool {
        self.joined
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// messages sent per kind
    pub fn counts(&self) -> &BTreeMap<&'static str, u32> {
        &self.counts
    }

    /// what to do and say since the last call
    pub fn take(&mut self) -> Vec<Out> {
        std::mem::take(&mut self.out)
    }

    /// the URL a join dials (the role and group in the query; the hello repeats them)
    pub fn url(&self) -> String {
        let group = self.o.group.as_deref().map(|g| format!("&group={}", encode(g))).unwrap_or_default();
        format!("{}/live/{}?role={}{group}", self.base, encode(&self.code), role_name(self.o.role))
    }

    /// The status as one HUD line: `JOINING CATH` · `DECK c3 @ CATH` · `OFFLINE … - JOINING AGAIN` · `NO RELAY … - AGAIN IN 2 S`
    /// · `RELAY token: …`
    pub fn line(&self) -> String {
        let why = |w: &Option<String>| w.as_deref().filter(|w| !w.is_empty()).map(|w| format!(" {w}")).unwrap_or_default();
        let s = &self.status;
        match s.state {
            State::Joining => format!("JOINING {}", self.code),
            State::Connected if self.note.is_some() => self.note.clone().unwrap_or_default(),
            State::Connected => {
                let (c, code) = (s.client.as_deref().unwrap_or(""), &self.code);
                match self.o.role {
                    Role::Conductor => format!("CONDUCTOR {c} @ {code}"),
                    Role::Output => format!("OUTPUT {c} OF {} @ {code}", self.o.group.as_deref().unwrap_or("")),
                    Role::Audience => format!("AUDIENCE {c} @ {code}"),
                    _ => format!("DECK {c} @ {code}"),
                }
            }
            State::Offline => match s.wait {
                Some(n) => format!("NO RELAY{} - AGAIN IN {n} S", why(&s.why)),
                None => format!("OFFLINE{} - JOINING AGAIN", why(&s.why)),
            },
            State::Refused => format!("RELAY token: {}", s.why.as_deref().unwrap_or("")),
        }
    }

    /// the earliest ms `tick` has something to do
    pub fn next_wake(&self) -> Option<f64> {
        match self.phase {
            Phase::Waiting(at) => Some(at),
            Phase::Open => {
                let ping = self.last_ping + PING_MS;
                let answer = (self.joined && self.answer_due).then_some(self.answered + PROGRESS_MS);
                Some(answer.map_or(ping, |a| a.min(ping)))
            }
            _ => None,
        }
    }

    fn set_status(&mut self, s: Status) {
        self.status = s.clone();
        self.out.push(Out::Status(s));
    }

    /// join (once)
    pub fn start(&mut self, now: f64) {
        if self.phase == Phase::Idle {
            self.dial(now);
        }
    }

    fn dial(&mut self, _now: f64) {
        self.phase = Phase::Dialing;
        self.joined = false;
        self.set_status(Status { state: State::Joining, client: None, why: None, wait: None });
        self.out.push(Out::Dial(self.url()));
    }

    /// the socket is open: hello first
    pub fn opened(&mut self, now: f64) {
        if self.phase != Phase::Dialing {
            return;
        }
        self.phase = Phase::Open;
        self.tries = 0;
        self.last_ping = now;
        let conductor = self.o.role == Role::Conductor;
        let hello = Hello {
            code: self.code.clone(),
            role: self.o.role,
            group: self.o.group.clone(),
            nick: self.o.nick.clone(),
            token: self.o.token.clone().or_else(|| conductor.then(|| "dev".into())),
            resume: self.resume.clone().filter(|_| self.o.resume),
        };
        self.send(Message::Hello(hello));
    }

    /// The socket closed, or the dial failed (`why`: what the adapter knows, may be empty): a joined link joins again
    /// soon, another backs off.
    pub fn closed(&mut self, now: f64, why: &str) {
        if !matches!(self.phase, Phase::Dialing | Phase::Open) {
            return;
        }
        let was = std::mem::replace(&mut self.joined, false);
        let why = (!why.is_empty()).then(|| why.to_string());
        let (wait_ms, wait) = if was {
            (REJOIN_MS, None)
        } else {
            let s = BACKOFF_S[self.tries.min(BACKOFF_S.len() - 1)];
            self.tries += 1;
            (f64::from(s) * 1000.0, Some(s))
        };
        self.phase = Phase::Waiting(now + wait_ms);
        self.note = None;
        self.set_status(Status { state: State::Offline, client: None, why, wait });
    }

    /// Time passes: a due dial, the keepalive, a sync answer that waited.
    pub fn tick(&mut self, now: f64) {
        match self.phase {
            Phase::Waiting(at) if now >= at => self.dial(now),
            Phase::Open => {
                if now - self.last_ping >= PING_MS {
                    self.ping(now);
                }
                self.flush(now);
            }
            _ => {}
        }
    }

    /// leave for good: the socket closes, nothing more happens
    pub fn stop(&mut self) {
        if matches!(self.phase, Phase::Dialing | Phase::Open) {
            self.out.push(Out::Close {});
        }
        self.phase = Phase::Stopped;
        self.joined = false;
    }

    /// drop the connection now and join again (with resume) – a network change, tests
    pub fn rejoin(&mut self) {
        if matches!(self.phase, Phase::Dialing | Phase::Open) {
            self.out.push(Out::Close {});
        }
    }

    fn send(&mut self, m: Message) -> bool {
        if self.phase != Phase::Open {
            return false;
        }
        *self.counts.entry(m.kind()).or_default() += 1;
        // through a Value: its maps are sorted – the same text natively and in wasm, whatever a HashMap's order
        let text = serde_json::to_value(Envelope::new("", m)).map(|v| v.to_string()).unwrap_or_default();
        self.out.push(Out::Send(text));
        true
    }

    /// any message on the open socket (the outlet's `media.ready`)
    pub fn send_message(&mut self, m: Message) -> bool {
        self.send(m)
    }

    /// #321: a ping now (the keepalive and the clock's pings alike), its `t0` the adapter's clock in µs
    pub fn ping(&mut self, now: f64) -> bool {
        let sent = self.send(Message::Ping(Ping { t0: (now * 1000.0).round() as i64 }));
        if sent {
            self.last_ping = now;
        }
        sent
    }

    /// #321: a leading deck's anchor (dropped while not joined – the clock sends again after a join)
    pub fn tempo(&mut self, t: Tempo) -> bool {
        self.joined && self.send(Message::Tempo(t))
    }

    /// the deck's being (another character, another scene): kept for every join
    pub fn set_graph(&mut self, g: DeckGraph) {
        self.graph = Some(g.clone());
        if self.joined {
            self.send(Message::DeckGraph(g));
        }
    }

    /// a deck's changes: folded always (offline the latest wait for the keyframe), sent once welcomed
    pub fn values(&mut self, v: DeckValues) {
        for (k, x) in &v.values {
            self.sent.insert(k.clone(), x.clone());
        }
        if v.mood.is_some() {
            self.mood.clone_from(&v.mood);
        }
        if self.joined {
            self.send(Message::DeckValues(v));
        }
    }

    /// the paths a deck streams, with how each moves (the played scene's)
    pub fn set_stream(&mut self, streamed: &[(String, Stream)]) {
        self.stream = Values::new(streamed);
    }

    pub fn stream_paths(&self) -> &[String] {
        self.stream.paths()
    }

    /// one frame of the deck's values (`stream_paths`' order) and its mood: what moved goes out at the stream's pace
    pub fn sample(&mut self, time: f64, read: &[f64], mood: Option<&str>) {
        if let Some(v) = self.stream.sample(time, read, mood) {
            self.values(v);
        }
    }

    /// #220: the render quality – kept, sent again after every join
    pub fn quality(&mut self, q: DeckQuality) {
        self.quality = Some(q);
        self.quality_due = true;
        self.flush_quality();
    }

    /// #195: this deck's answer to the push – kept (a join sends it again); progress at most every `PROGRESS_MS`
    pub fn synced(&mut self, now: f64, r: DeckSynced) {
        self.answer = Some(r);
        self.answer_due = true;
        self.flush(now);
    }

    /// #195: the conductor's push – sent once joined, and again at a welcome whose snapshot lacks it
    pub fn push(&mut self, s: SceneSync) {
        self.push = Some(s);
        self.push_due = true;
        self.flush_push();
    }

    /// #211: the deck's MIDI outputs for the conductor (only while joined – stale MIDI is dropped)
    pub fn midi(&mut self, m: DeckMidi) {
        if self.joined {
            self.send(Message::DeckMidi(m));
        }
    }

    fn flush_quality(&mut self) {
        if self.joined && self.quality_due {
            if let Some(q) = self.quality.clone() {
                self.send(Message::DeckQuality(q));
            }
            self.quality_due = false;
        }
    }

    fn flush_push(&mut self) {
        if self.joined && self.push_due {
            if let Some(s) = self.push.clone() {
                self.send(Message::SceneSync(s));
            }
            self.push_due = false;
        }
    }

    fn flush(&mut self, now: f64) {
        self.flush_quality();
        self.flush_push();
        if !(self.joined && self.answer_due) {
            return;
        }
        let Some(r) = self.answer.clone() else { return };
        if r.state == SyncState::Syncing && now - self.answered < PROGRESS_MS {
            return;
        }
        self.answered = now;
        self.answer_due = false;
        self.send(Message::DeckSynced(r));
    }

    /// #231: a conductor opens a vote
    pub fn vote_open(&mut self, p: VoteOpen) -> bool {
        self.send(Message::VoteOpen(p))
    }

    /// a vote for `option` of the vote open now (none open: nothing)
    pub fn vote_cast(&mut self, option: i64) -> bool {
        let Some(vote) = self.vote.as_ref().map(|v| v.vote.clone()) else { return false };
        self.send(Message::VoteCast(VoteCast { vote, option }))
    }

    /// the conductor closes the open vote (`over`: the option it picks instead)
    pub fn vote_close(&mut self, over: Option<i64>) -> bool {
        let Some(vote) = self.vote.as_ref().map(|v| v.vote.clone()) else { return false };
        self.send(Message::VoteClose(VoteClose { vote, r#override: over }))
    }

    pub fn vote_hold(&mut self, hold: bool) -> bool {
        let Some(vote) = self.vote.as_ref().map(|v| v.vote.clone()) else { return false };
        self.send(Message::VoteHold(VoteHold { vote, hold }))
    }

    /// the vote open now as a screen hears it (a screen that comes late gets it at once)
    pub fn current_vote(&self) -> Option<Envelope> {
        self.vote.clone().map(|v| relay_says(Message::VoteOpen(v)))
    }

    /// A message's text arrived.
    pub fn text(&mut self, now: f64, text: &str) {
        if self.phase != Phase::Open {
            return;
        }
        let Ok(e) = serde_json::from_str::<Envelope>(text) else { return };
        self.seq = self.seq.max(e.seq as i64);
        if self.o.messages {
            self.out.push(Out::Message(e.clone()));
        }
        let conductor = self.o.role == Role::Conductor;
        let from = e.from.clone();
        match e.message.clone() {
            Message::Welcome(w) => self.welcome(now, w.client, w.snapshot),
            Message::Pong(p) => self.out.push(Out::Pong { t0: p.t0, t1: p.t1, t2: now }),
            Message::Tempo(t) => self.out.push(Out::Tempo { tempo: t, arrived: now }),
            Message::SceneSync(s) if conductor => self.out.push(Out::Deck(In::Pushed(s))),
            Message::SceneSync(s) => self.out.push(Out::Sync(s)),
            Message::DeckGraph(graph) => self.out.push(Out::Deck(In::Deck { client: from, graph, values: HashMap::new(), mood: None })),
            Message::DeckValues(v) => self.out.push(Out::Deck(In::Values { client: from, values: v.values, mood: v.mood })),
            Message::DeckSynced(synced) if conductor => self.out.push(Out::Deck(In::Synced { client: from, synced })),
            Message::Presence(p) => {
                let routes = p.routes.unwrap_or_default().into_iter().map(|r| (r.from, r.to)).collect();
                self.out.push(Out::Deck(In::Presence { decks: p.decks.unwrap_or_default(), routes, links: p.links.unwrap_or_default(), quality: p.quality.unwrap_or_default() }));
            }
            Message::VoteOpen(v) => {
                self.vote = Some(v);
                self.out.push(Out::Vote(e));
            }
            Message::VoteResult(_) => {
                self.vote = None;
                self.out.push(Out::Vote(e));
            }
            Message::VoteTally(_) | Message::Say(_) => self.out.push(Out::Vote(e)),
            Message::Error(err) => {
                self.out.push(Out::Vote(e));
                if err.code == "token" {
                    self.set_status(Status { state: State::Refused, client: None, why: Some(err.message), wait: None });
                    self.out.push(Out::Close {});
                } else {
                    self.note = Some(format!("RELAY {}: {}", err.code, err.message));
                    let s = self.status.clone();
                    self.set_status(s);
                }
            }
            _ => {}
        }
        if let Some(r) = self.resume.as_mut() {
            r.seq = self.seq;
        }
    }

    fn welcome(&mut self, now: f64, client: String, snap: Snapshot) {
        self.client.clone_from(&client);
        self.joined = true;
        self.note = None;
        if self.o.resume {
            self.resume = Some(Resume { client: client.clone(), seq: self.seq });
        }
        self.set_status(Status { state: State::Connected, client: Some(client.clone()), why: None, wait: None });
        self.out.push(Out::Joined { client, at: now });
        for (client, d) in snap.decks.unwrap_or_default() {
            self.out.push(Out::Deck(In::Deck { client, graph: d.graph, values: d.values, mood: d.mood }));
        }
        if self.o.role == Role::Conductor {
            // the session's sync and the decks' answers; this conductor's push again if the relay lacks it
            if let Some(s) = &snap.sync {
                self.out.push(Out::Deck(In::Pushed(s.clone())));
            }
            for (client, synced) in snap.synced.unwrap_or_default() {
                self.out.push(Out::Deck(In::Synced { client, synced }));
            }
            self.push_due = self.push.is_some() && snap.sync != self.push;
        } else if let Some(s) = snap.sync {
            self.out.push(Out::Sync(s));
        }
        if let Some(g) = self.graph.clone() {
            // the being, then every value it showed so far (a keyframe)
            self.send(Message::DeckGraph(g));
            if !self.sent.is_empty() || self.mood.is_some() {
                let all = DeckValues { values: self.sent.clone().into_iter().collect(), mood: self.mood.clone() };
                self.send(Message::DeckValues(all));
            }
        }
        // the relay forgot this deck's quality and answer when it left
        self.quality_due = self.quality.is_some();
        self.answer_due = self.answer.is_some();
        self.answered = f64::NEG_INFINITY;
        self.flush(now);
        if let Some(t) = snap.tempo {
            self.out.push(Out::Tempo { tempo: t, arrived: now });
        }
        if let Some(v) = snap.vote {
            self.vote = Some(v.clone());
            self.out.push(Out::Vote(relay_says(Message::VoteOpen(v))));
        }
        if let Some(say) = self.o.group.as_ref().and_then(|g| snap.say.as_ref()?.get(g).cloned()) {
            self.out.push(Out::Vote(relay_says(Message::Say(say))));
        }
    }
}

/// an envelope as if the relay said it (what the welcome's snapshot holds, handed to a screen)
fn relay_says(m: Message) -> Envelope {
    Envelope::new("relay", m)
}

/// one scripted moment: at `at` s, `do` with `with`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub at: f64,
    #[serde(rename = "do")]
    pub what: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub with: serde_json::Value,
}

/// A scripted link: how it joins, a deck's being and stream, and what happens.
#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub join: Options,
    #[serde(default)]
    pub graph: Option<DeckGraph>,
    #[serde(default)]
    pub stream: Vec<(String, Stream)>,
    pub steps: Vec<Step>,
}

/// what the client did at one moment: a step, or a timer it asked for (`wake`)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Turn {
    pub at: f64,
    #[serde(rename = "do")]
    pub what: String,
    pub outs: Vec<Out>,
}

/// One scripted step on a client at `now` ms (what an adapter calls for it).
pub fn step(c: &mut Client, now: f64, s: &Step) -> Result<(), String> {
    use serde_json::from_value;
    let w = s.with.clone();
    let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_string();
    match s.what.as_str() {
        "start" => c.start(now),
        "open" => c.opened(now),
        "close" | "fail" => c.closed(now, &text(&w)),
        "message" => {
            let mut e = w;
            if let Some(o) = e.as_object_mut() {
                o.entry("v").or_insert(1.into());
                o.entry("seq").or_insert(1.into());
                o.entry("from").or_insert("relay".into());
            }
            c.text(now, &e.to_string());
        }
        "sample" => {
            let read: Vec<f64> = c.stream_paths().iter().map(|p| w["values"][p].as_f64().unwrap_or(f64::NAN)).collect();
            c.sample(now / 1000.0, &read, w["mood"].as_str());
        }
        "graph" => c.set_graph(from_value(w).map_err(|e| e.to_string())?),
        "quality" => c.quality(from_value(w).map_err(|e| e.to_string())?),
        "synced" => c.synced(now, from_value(w).map_err(|e| e.to_string())?),
        "push" => c.push(from_value(w).map_err(|e| e.to_string())?),
        "tempo" => {
            c.tempo(from_value(w).map_err(|e| e.to_string())?);
        }
        "ping" => {
            c.ping(now);
        }
        "vote_open" => {
            c.vote_open(from_value(w).map_err(|e| e.to_string())?);
        }
        "vote_cast" => {
            c.vote_cast(w.as_i64().unwrap_or(0));
        }
        "vote_close" => {
            c.vote_close(w.as_i64());
        }
        "vote_hold" => {
            c.vote_hold(w.as_bool().unwrap_or(false));
        }
        "rejoin" => c.rejoin(),
        "stop" => c.stop(),
        "tick" => c.tick(now),
        other => return Err(format!("no step '{other}'")),
    }
    Ok(())
}

/// A scripted link through a `Client` as every adapter drives it: `start` at 0, then before each step the timers it
/// asked for (`tick` at `next_wake`, each a `wake` turn), then the step; the outs of each.
pub fn script(s: &Script) -> Result<Vec<Turn>, String> {
    let mut c = Client::new(s.join.clone())?;
    if let Some(g) = &s.graph {
        c.set_graph(g.clone());
    }
    c.set_stream(&s.stream);
    c.start(0.0);
    let mut turns = vec![Turn { at: 0.0, what: "start".into(), outs: c.take() }];
    for st in &s.steps {
        let now = st.at * 1000.0;
        while let Some(at) = c.next_wake().filter(|&at| at <= now) {
            c.tick(at);
            turns.push(Turn { at: at / 1000.0, what: "wake".into(), outs: c.take() });
        }
        step(&mut c, now, st)?;
        turns.push(Turn { at: st.at, what: st.what.clone(), outs: c.take() });
    }
    Ok(turns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const RELAY: &str = "ws://relay.test:7070/live/CATH";

    fn sent(turns: &[Turn]) -> Vec<Value> {
        turns.iter().flat_map(|t| &t.outs).filter_map(|o| if let Out::Send(s) = o { serde_json::from_str(s).ok() } else { None }).collect()
    }

    fn kinds(turns: &[Turn]) -> Vec<String> {
        sent(turns).iter().map(|e| e["kind"].as_str().unwrap_or("").to_string()).collect()
    }

    fn lines_of(s: &Script) -> Vec<String> {
        let mut c = Client::new(s.join.clone()).unwrap();
        c.start(0.0);
        let mut lines = vec![c.line()];
        for st in &s.steps {
            let now = st.at * 1000.0;
            while let Some(at) = c.next_wake().filter(|&at| at <= now) {
                c.tick(at);
            }
            step(&mut c, now, st).unwrap();
            if lines.last() != Some(&c.line()) {
                lines.push(c.line());
            }
        }
        lines
    }

    fn st(at: f64, what: &str, with: Value) -> Step {
        Step { at, what: what.into(), with }
    }

    fn welcome(client: &str, snapshot: Value) -> Value {
        json!({ "kind": "welcome", "p": { "client": client, "snapshot": snapshot, "now": 0 } })
    }

    fn graph() -> DeckGraph {
        serde_json::from_value(json!({ "nodes": [{ "id": "voice", "kind": "osc" }], "links": [], "character": "yellowmouse" })).unwrap()
    }

    fn stream() -> Vec<(String, Stream)> {
        vec![("voice.index".into(), Stream::Input([0.0, 8.0])), ("voice.out".into(), Stream::Audio)]
    }

    /// The scripted links every adapter's test replays (shared/live-deck/test/fixtures/relay.json, native's below):
    /// a deck's join, keyframe, quality and stream through a drop; backoff and keepalive; the push and the answer; the
    /// show clock; a vote screen; a conductor's push; the outlet's resume; a refused token.
    pub fn scripts() -> Vec<(&'static str, Script)> {
        let mk = |join: Value, deck: bool, steps: Vec<Step>| Script {
            join: serde_json::from_value(join).unwrap(),
            graph: deck.then(graph),
            stream: if deck { stream() } else { Vec::new() },
            steps,
        };
        let deck = json!({ "relay": RELAY, "role": "musician", "group": DECKS, "nick": "browser" });
        let q = json!({ "level": "medium", "scale": 0.8, "auto": true, "frame": 17.1, "ceiling": "medium" });
        vec![
            (
                "deck",
                mk(
                    deck.clone(),
                    true,
                    vec![
                        st(0.01, "sample", json!({ "values": { "voice.index": 2.0, "voice.out": 0.5 }, "mood": "neutral" })),
                        st(0.02, "quality", q.clone()),
                        st(0.05, "open", Value::Null),
                        st(0.1, "message", welcome("c5", json!({}))),
                        st(0.2, "sample", json!({ "values": { "voice.index": 2.0001, "voice.out": 0.6 }, "mood": "happy" })),
                        st(0.3, "sample", json!({ "values": { "voice.index": 3.0, "voice.out": 0.6 }, "mood": "happy" })),
                        st(25.0, "close", json!("the relay went")),
                        st(25.3, "open", Value::Null),
                        st(25.4, "message", welcome("c6", json!({}))),
                        st(26.0, "stop", Value::Null),
                    ],
                ),
            ),
            (
                "backoff",
                mk(
                    json!({ "relay": "wss://192.168.1.116:4100/relay/live/CLOCK", "role": "musician", "group": DECKS }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.05, "message", welcome("c1", json!({}))),
                        st(30.0, "close", Value::Null),
                        st(31.0, "fail", json!("refused")),
                        st(33.0, "fail", Value::Null),
                        st(37.0, "fail", Value::Null),
                        st(45.0, "fail", Value::Null),
                        st(55.0, "fail", Value::Null),
                        st(66.0, "fail", Value::Null),
                        st(77.0, "open", Value::Null),
                        st(78.0, "fail", Value::Null),
                    ],
                ),
            ),
            (
                "sync",
                mk(
                    json!({ "relay": "ws://relay.test/live/SYNC", "role": "musician", "group": DECKS }),
                    true,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.01, "synced", json!({ "scene": "x", "state": "ready", "ready": [], "missing": [] })),
                        st(0.02, "message", welcome("c7", json!({ "sync": { "scene": "cathedral", "sets": ["cathedral"], "characters": ["catpope"] } }))),
                        st(0.03, "message", json!({ "from": "c1", "kind": "scene.sync", "p": { "scene": "cathedral", "sets": ["cathedral"], "characters": [] } })),
                        st(0.1, "synced", json!({ "scene": "cathedral", "state": "syncing", "done": 1, "total": 3, "bytes": 4, "ready": [], "missing": [] })),
                        st(0.15, "synced", json!({ "scene": "cathedral", "state": "syncing", "done": 2, "total": 3, "bytes": 8, "ready": [], "missing": [] })),
                        st(0.4, "synced", json!({ "scene": "cathedral", "state": "ready", "done": 3, "total": 3, "bytes": 12, "ready": ["set:cathedral"], "missing": [] })),
                    ],
                ),
            ),
            (
                "clock",
                mk(
                    json!({ "relay": RELAY, "role": "musician", "group": DECKS }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.01, "tempo", json!({ "bpm": 126.0, "per_bar": 4, "beat": 32.0, "running": true })),
                        st(0.02, "message", welcome("c2", json!({ "tempo": { "bpm": 120.0, "per_bar": 4, "beat": 8.0, "running": true, "at": 1000 } }))),
                        st(0.5, "ping", Value::Null),
                        st(0.52, "message", json!({ "kind": "pong", "p": { "t0": 500000, "t1": 1700000000000_i64 } })),
                        st(1.0, "tempo", json!({ "bpm": 126.0, "per_bar": 4, "beat": 32.0, "running": true })),
                        st(1.1, "message", json!({ "kind": "tempo", "p": { "bpm": 90.0, "per_bar": 3, "beat": 1.0, "running": false, "at": 2000 } })),
                        st(5.0, "message", json!({ "kind": "error", "p": { "code": "rate", "message": "too many" } })),
                    ],
                ),
            ),
            (
                "vote",
                mk(
                    json!({ "relay": RELAY, "role": "audience", "group": "balkon", "nick": "Mia" }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.1, "vote_cast", json!(1)),
                        st(0.2, "message", welcome("c7", json!({ "vote": { "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20 }, "say": { "balkon": { "text": "Stand up" } } }))),
                        st(0.3, "vote_cast", json!(1)),
                        st(0.4, "message", json!({ "kind": "vote.tally", "p": { "vote": "v1", "counts": [1, 0], "voters": 1, "left": 10 } })),
                        st(0.5, "message", json!({ "kind": "vote.result", "p": { "vote": "v1", "option": 0, "counts": [1, 0], "how": "rule" } })),
                        st(0.6, "vote_cast", json!(0)),
                        st(0.7, "message", json!({ "kind": "say", "p": { "text": "Sit down" } })),
                    ],
                ),
            ),
            (
                "conductor-vote",
                mk(
                    json!({ "relay": RELAY, "role": "conductor", "token": "tok" }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.1, "message", welcome("c1", json!({}))),
                        st(0.2, "vote_close", Value::Null),
                        st(0.3, "vote_open", json!({ "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20 })),
                        st(0.4, "message", json!({ "kind": "vote.open", "p": { "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20 } })),
                        st(0.5, "vote_hold", json!(true)),
                        st(0.6, "vote_close", json!(1)),
                        st(0.7, "vote_cast", json!(1)),
                    ],
                ),
            ),
            (
                "refused",
                mk(
                    json!({ "relay": RELAY, "role": "conductor", "token": "bad" }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.1, "message", json!({ "kind": "error", "p": { "code": "token", "message": "bad signature" } })),
                        st(0.11, "close", Value::Null),
                        st(1.2, "open", Value::Null),
                    ],
                ),
            ),
            (
                "conductor-push",
                mk(
                    json!({ "relay": RELAY, "role": "conductor", "group": DECKS }),
                    false,
                    vec![
                        st(0.0, "push", json!({ "scene": "cathedral", "sets": ["cathedral"] })),
                        st(0.1, "open", Value::Null),
                        st(0.2, "message", welcome("c1", json!({ "decks": { "c2": { "graph": { "nodes": [], "links": [] }, "values": { "a.b": [1.0] } } }, "synced": { "c2": { "scene": "cathedral", "state": "ready", "ready": [], "missing": [] } } }))),
                        st(0.3, "message", json!({ "from": "c2", "kind": "deck.values", "p": { "values": { "a.b": [2.0] }, "mood": "happy" } })),
                        st(0.4, "message", json!({ "from": "c2", "kind": "deck.synced", "p": { "scene": "cathedral", "state": "missing", "ready": [], "missing": ["set:cathedral"] } })),
                        st(0.5, "message", json!({ "kind": "presence", "p": { "groups": {}, "ready": [], "decks": ["c2"], "routes": [{ "from": "c2", "to": "c1" }] } })),
                        st(1.0, "close", Value::Null),
                        st(1.3, "open", Value::Null),
                        st(1.4, "message", welcome("c1", json!({ "sync": { "scene": "cathedral", "sets": ["cathedral"] } }))),
                    ],
                ),
            ),
            (
                "outlet",
                mk(
                    json!({ "relay": RELAY, "role": "output", "group": "left", "resume": true, "messages": true }),
                    false,
                    vec![
                        st(0.0, "open", Value::Null),
                        st(0.1, "message", json!({ "seq": 4, "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0 } })),
                        st(0.2, "message", json!({ "seq": 9, "kind": "say", "p": { "text": "Lights" } })),
                        st(1.0, "rejoin", Value::Null),
                        st(1.0, "close", json!("asked to join again")),
                        st(1.3, "open", Value::Null),
                    ],
                ),
            ),
        ]
    }

    fn turns(name: &str) -> Vec<Turn> {
        script(&scripts().into_iter().find(|(n, _)| *n == name).unwrap().1).unwrap()
    }

    #[test]
    fn relay_urls() {
        assert_eq!(parse_relay("ws://localhost:7070/live/CATH").unwrap(), ("ws://localhost:7070".into(), "CATH".into()));
        assert_eq!(parse_relay("wss://live.scutoons.com/live/A7K2/").unwrap().1, "A7K2");
        assert_eq!(parse_relay("ws://x:7700/live/A7K2/?role=x").unwrap().1, "A7K2");
        assert_eq!(parse_relay("wss://192.168.1.116:4100/relay/live/CLOCK").unwrap(), ("wss://192.168.1.116:4100/relay".into(), "CLOCK".into()));
        assert_eq!(parse_relay("ws://h/live/A%20B").unwrap().1, "A B");
        assert!(parse_relay("http://x/live/CATH").is_err());
        assert!(parse_relay("ws://x:7700/").unwrap_err().contains("session code"));
        let mut o = Options::new("ws://h:1/live/A B", Role::Audience, Some("bal kon"));
        o.relay = "ws://h:1/live/A%20B".into();
        assert_eq!(Client::new(o).unwrap().url(), "ws://h:1/live/A%20B?role=audience&group=bal%20kon");
    }

    #[test]
    fn a_deck_joins_sends_its_being_and_a_keyframe_then_changes() {
        let t = turns("deck");
        assert_eq!(t[0].outs[1], Out::Dial("ws://relay.test:7070/live/CATH?role=musician&group=decks".into()));
        let s = sent(&t);
        assert_eq!(s[0], json!({ "v": 1, "seq": 0, "from": "", "kind": "hello", "p": { "code": "CATH", "role": "musician", "group": "decks", "nick": "browser" } }));
        assert_eq!(kinds(&t)[..5], ["hello", "deck.graph", "deck.values", "deck.quality", "deck.values"]);
        assert_eq!(s[2]["p"], json!({ "values": { "voice.index": [2.0], "voice.out": [0.5] }, "mood": "neutral" }), "the keyframe: what was sampled before the welcome");
        assert_eq!(s[4]["p"], json!({ "values": { "voice.out": [0.6] }, "mood": "happy" }), "only what moved past its epsilon");
        assert!(!kinds(&t[..t.len() - 5]).contains(&"ping".to_string()) || kinds(&t).iter().filter(|k| *k == "ping").count() == 1, "one keepalive in 25 s");
        // the drop at 25 s: joins again after REJOIN_MS – the being, the keyframe of everything and the quality again
        let again: Vec<String> = kinds(&t[t.iter().position(|x| x.at == 25.0).unwrap()..]);
        assert_eq!(again, ["hello", "deck.graph", "deck.values", "deck.quality"]);
        let last_values = sent(&t).into_iter().rev().find(|e| e["kind"] == "deck.values").unwrap();
        assert_eq!(last_values["p"]["values"], json!({ "voice.index": [3.0], "voice.out": [0.6] }));
        let dial_at: Vec<f64> = t.iter().filter(|x| x.outs.iter().any(|o| matches!(o, Out::Dial(_)))).map(|x| x.at).collect();
        assert_eq!(dial_at, [0.0, 25.25]);
        assert!(t.last().unwrap().outs.contains(&Out::Close {}), "stop closes");
    }

    #[test]
    fn backoff_keepalive_and_the_lines() {
        let s = &scripts()[1].1;
        let t = script(s).unwrap();
        let waits: Vec<u32> = t.iter().flat_map(|x| &x.outs).filter_map(|o| if let Out::Status(Status { wait: Some(w), .. }) = o { Some(*w) } else { None }).collect();
        assert_eq!(waits, [1, 2, 4, 8, 10, 10, 1], "backs off, caps at 10 s; an open socket starts over");
        assert_eq!(kinds(&t).iter().filter(|k| *k == "ping").count(), 1, "the keepalive at 20 s");
        let lines = lines_of(s);
        assert_eq!(lines[..4], ["JOINING CLOCK", "DECK c1 @ CLOCK", "OFFLINE - JOINING AGAIN", "NO RELAY refused - AGAIN IN 1 S"]);
    }

    #[test]
    fn the_push_reaches_the_deck_and_its_answer_goes_out_kept_for_a_join() {
        let t = turns("sync");
        let syncs: Vec<&Out> = t.iter().flat_map(|x| &x.outs).filter(|o| matches!(o, Out::Sync(_))).collect();
        assert_eq!(syncs.len(), 2, "the welcome's and a scene.sync");
        let answers: Vec<(f64, Value)> = t.iter().flat_map(|x| sent(std::slice::from_ref(x)).into_iter().filter(|e| e["kind"] == "deck.synced").map(move |e| (x.at, e["p"].clone()))).collect();
        let states: Vec<(f64, &str, Option<u64>)> = answers.iter().map(|(at, p)| (*at, p["state"].as_str().unwrap(), p["done"].as_u64())).collect();
        assert_eq!(states, [(0.02, "ready", None), (0.22, "syncing", Some(2)), (0.4, "ready", Some(3))], "the kept answer at the welcome; progress every 200 ms (the newest); the final at once");
    }

    #[test]
    fn the_show_clock_pings_pongs_and_anchors() {
        let t = turns("clock");
        let outs: Vec<&Out> = t.iter().flat_map(|x| &x.outs).collect();
        assert!(outs.contains(&&Out::Pong { t0: 500_000, t1: 1_700_000_000_000, t2: 520.0 }));
        let tempos: Vec<f64> = outs.iter().filter_map(|o| if let Out::Tempo { tempo, .. } = o { Some(tempo.bpm) } else { None }).collect();
        assert_eq!(tempos, [120.0, 90.0], "the welcome's anchor, then the relay's");
        let s = sent(&t);
        assert_eq!(s.iter().find(|e| e["kind"] == "ping").unwrap()["p"]["t0"], 500_000, "a ping's t0: the adapter's clock in µs");
        assert_eq!(s.iter().filter(|e| e["kind"] == "tempo").count(), 1, "a leader's anchor only once joined");
        assert!(lines_of(&scripts()[3].1).last().unwrap() == "RELAY rate: too many");
    }

    #[test]
    fn a_vote_screen_hears_the_vote_and_casts_into_it() {
        let t = turns("vote");
        let heard: Vec<String> = t.iter().flat_map(|x| &x.outs).filter_map(|o| if let Out::Vote(e) = o { Some(e.message.kind().to_string()) } else { None }).collect();
        assert_eq!(heard, ["vote.open", "say", "vote.tally", "vote.result", "say"]);
        assert_eq!(kinds(&t), ["hello", "vote.cast"], "a cast only into an open vote");
        assert_eq!(sent(&t)[1]["p"], json!({ "vote": "v1", "option": 1 }));
        let c = turns("conductor-vote");
        assert_eq!(sent(&c)[0]["p"]["token"], "tok");
        let moves: Vec<(String, Value)> = sent(&c).into_iter().skip(1).map(|e| (e["kind"].as_str().unwrap().to_string(), e["p"].clone())).collect();
        assert_eq!(
            moves,
            [
                ("vote.open".to_string(), json!({ "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20 })),
                ("vote.hold".to_string(), json!({ "vote": "v1", "hold": true })),
                ("vote.close".to_string(), json!({ "vote": "v1", "override": 1 })),
                ("vote.cast".to_string(), json!({ "vote": "v1", "option": 1 })),
            ]
        );
    }

    #[test]
    fn a_refused_token_closes_and_backs_off() {
        let t = turns("refused");
        let outs: Vec<&Out> = t.iter().flat_map(|x| &x.outs).collect();
        assert!(outs.contains(&&Out::Status(Status { state: State::Refused, client: None, why: Some("bad signature".into()), wait: None })));
        assert!(outs.contains(&&Out::Close {}));
        assert_eq!(lines_of(&scripts()[6].1)[1], "RELAY token: bad signature");
        assert_eq!(t.iter().filter(|x| x.outs.iter().any(|o| matches!(o, Out::Dial(_)))).map(|x| x.at).collect::<Vec<_>>(), [0.0, 1.11]);
    }

    #[test]
    fn a_conductor_sees_the_decks_and_pushes_again_when_the_relay_forgot() {
        let t = turns("conductor-push");
        assert_eq!(sent(&t)[0]["p"]["token"], "dev", "a conductor without a token: dev");
        let pushes: Vec<f64> = t.iter().filter(|x| kinds(std::slice::from_ref(x)).contains(&"scene.sync".to_string())).map(|x| x.at).collect();
        assert_eq!(pushes, [0.2], "once joined; not again when the snapshot has it");
        let decks: Vec<&In> = t.iter().flat_map(|x| &x.outs).filter_map(|o| if let Out::Deck(i) = o { Some(i) } else { None }).collect();
        assert!(matches!(decks[0], In::Deck { client, .. } if client == "c2"));
        assert!(matches!(decks[1], In::Synced { client, .. } if client == "c2"));
        assert!(matches!(decks[2], In::Values { mood: Some(m), .. } if m == "happy"));
        assert!(matches!(decks[4], In::Presence { routes, .. } if routes == &[("c2".to_string(), "c1".to_string())]));
        assert!(matches!(decks[5], In::Pushed(_)));
    }

    #[test]
    fn the_outlet_resumes_where_it_left() {
        let t = turns("outlet");
        let hellos: Vec<Value> = sent(&t).into_iter().filter(|e| e["kind"] == "hello").collect();
        assert_eq!(hellos[0]["p"], json!({ "code": "CATH", "role": "output", "group": "left" }));
        assert_eq!(hellos[1]["p"]["resume"], json!({ "client": "c3", "seq": 9 }));
        assert_eq!(t.iter().flat_map(|x| &x.outs).filter(|o| matches!(o, Out::Message(_))).count(), 2, "every envelope for the composer");
        let mut c = Client::new(scripts()[8].1.join.clone()).unwrap();
        c.start(0.0);
        c.opened(0.0);
        c.text(0.0, &json!({ "v": 1, "seq": 1, "from": "relay", "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0 } }).to_string());
        assert_eq!(c.line(), "OUTPUT c3 OF left @ CATH");
    }

    #[test]
    fn values_at_the_pace_past_their_epsilon() {
        let mut v = Values::new(&stream());
        assert_eq!(v.sample(0.0, &[2.0, 0.5], Some("neutral")).unwrap().values.len(), 2);
        assert_eq!(v.sample(0.01, &[3.0, 0.5], Some("neutral")), None, "between two slots");
        assert_eq!(v.sample(1.0 / RATE, &[2.0 + 0.004, 0.5], Some("neutral")), None, "under its epsilon");
        assert_eq!(v.sample(2.0 / RATE, &[2.0, f64::NAN], Some("happy")).unwrap().mood.as_deref(), Some("happy"), "a non-finite value is skipped");
        let mut n = 0;
        for f in 0..240 {
            if v.sample(10.0 + f64::from(f) / 120.0, &[0.0, f64::from(f) / 240.0], None).is_some() {
                n += 1;
            }
        }
        assert!((59..=61).contains(&n), "30 a second: {n}");
    }

    /// shared/live-deck/test/fixtures/relay.json: the scripted links and every turn's outs – the browser's adapter
    /// (client.ts over relay-web's wasm) must do and say the same.
    #[test]
    fn relay_fixture_for_the_browser_is_current() {
        let fixture: Vec<Value> = scripts()
            .into_iter()
            .map(|(name, s)| {
                let stream: Vec<Value> = s.stream.iter().map(|(p, k)| json!([p, k])).collect();
                json!({ "name": name, "join": serde_json::to_value(OptionsOut(&s.join)).unwrap(), "graph": s.graph, "stream": stream, "steps": s.steps, "turns": script(&s).unwrap() })
            })
            .collect();
        let text = serde_json::to_string_pretty(&json!({ "scripts": fixture })).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/relay.json");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(path).unwrap_or_default().replace('\r', "");
        assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space relay_fixture");
    }

    /// the options as the browser hands them in
    struct OptionsOut<'a>(&'a Options);

    impl Serialize for OptionsOut<'_> {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let o = self.0;
            json!({ "relay": o.relay, "role": o.role, "group": o.group, "nick": o.nick, "token": o.token, "resume": o.resume, "messages": o.messages }).serialize(s)
        }
    }
}
