//! The relay client (#366): one sans-IO state machine every client of the relay drives – the native deck's link and
//! the Live outlet (deck-native: a tungstenite thread), the browser deck and the vote screens (deck/relay-web as wasm
//! under live-deck client.ts: a `WebSocket` and a timer) and the headless host. The core decides; an adapter moves bytes.
//!
//! - **In:** the socket opened, closed (or a dial failed), a message's text; `tick` at `next_wake`; what the app hands
//!   over (a deck's values, its being, render quality, sync answer, a leader's tempo, a vote cast). Time is the
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
//! - **QUIC first, WebSocket last** (#438, `?quic=` on the relay URL – none: only the WebSocket, as ever): QUIC gets
//!   `QUIC_HEAD_START_MS`, then a WebSocket is dialled beside it; the first one up says `hello`, so a race leaves no ghost.
//!   A QUIC link that dies falls to the WebSocket with a resume; a WebSocket link tries QUIC again after `STEP_UP_MS`.
//!   On QUIC `deck.values` go as datagrams (they may be lost; every `SETTLE_MS` the full values go on the stream, and
//!   what arrives is applied key by key, only when not older), everything else on the stream.
//! - **The handheld camera** (#469, research/HANDHELD-CAMERA.md): a phone (`handheld`) sends its pose (`hand_pose`: latest
//!   wins, a datagram on QUIC, the socket on the WebSocket), state, Record and Recenter; the editor (the session's
//!   conductor) its view. The receiver drops a pose that is malformed or not newer than that phone's last. `rung` says
//!   which socket a page is on.
//!
//! `script` runs scripted socket events through a `Client` – the outs every adapter's test compares against
//! (shared/live-deck/test/fixtures/relay.json, the headless scenario `relay_join`).
use crate::linked::{Choice, Driving, Place, Readers, Steer};
use crate::live_cameras::{Held, Holders, Holding, Seat};
use crate::rig::Stream;
use protocol::lines::{line, Lines, DATAGRAM_MAX};
use protocol::{
    Assign, CameraFlash, CameraForce, CameraLeave, Cameras, DeckEvent, DeckEvents, DeckGraph, DeckListen, DeckMidi, DeckQuality, DeckSynced, DeckValues, Empty, Envelope, HandPose, HandRecord, HandState, HandView, Hello,
    LinkQuality, LiveCamera, Loose, Message, Ping, Rename, Resume, Role, SceneSync, ShowRole, Snapshot, SyncState, Tempo, To, VoteCast, VoteOpen,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// the keepalive: a ping at least this often (ms)
pub const PING_MS: f64 = 20_000.0;
/// a link that never got a welcome dials again after these waits (s), the last one from then on
pub const BACKOFF_S: [u32; 5] = [1, 2, 4, 8, 10];
/// a joined link that dropped dials again after this (ms)
pub const REJOIN_MS: f64 = 250.0;
/// a deck's sync progress at most this often (ms); the final state goes at once
pub const PROGRESS_MS: f64 = 200.0;
/// at most this many `deck.values` a second (the relay drops more than 30 / s)
pub const RATE: f64 = 30.0;
/// the decks' group
pub const DECKS: &str = "decks";
/// how long QUIC tries alone before a WebSocket is dialled beside it (ms)
pub const QUIC_HEAD_START_MS: f64 = 250.0;
/// a QUIC dial nothing else is waiting for ends after this (ms): a path that drops UDP never says no
pub const QUIC_GIVE_UP_MS: f64 = 3000.0;
/// on the WebSocket this long, try QUIC again in the background (ms); doubles after each failure, up to `STEP_UP_MAX_MS`
pub const STEP_UP_MS: f64 = 60_000.0;
pub const STEP_UP_MAX_MS: f64 = 300_000.0;
/// on QUIC, the full values go on the stream this often while they change (ms): what a lost datagram would have said
pub const SETTLE_MS: f64 = 1000.0;
/// a due settle waits this long (ms, a frame and a bit) to take the place of the next change: the relay allows a deck 30
/// `deck.values` a second, a deck at full pace has none to spare
pub const SETTLE_GRACE_MS: f64 = 40.0;
/// #478: a client that asks for its proof (`Options::wait_proof`) dials at most this long after its start without the
/// answer (ms): long enough for the API's answer on a usual link, short enough that a deck nobody signed in to is not kept
pub const PROOF_WAIT_MS: f64 = 1500.0;
/// the relay a deck joins when nothing names another (#199; deck-native live/config.rs, space-web outlet.rs) – the hosted
/// one, the Cathedral's session for now; a linked reader's link names no relay on it (linked.rs `reader_link`, #530)
pub const HOSTED_RELAY: &str = "wss://live.scutoons.com/live/CATH";

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

/// a relay URL's query parameter, decoded (`?quic=`, `&quichash=`); None when the URL does not name it
pub fn relay_param(url: &str, name: &str) -> Option<String> {
    let query = url.trim().split('#').next().unwrap_or("").split_once('?').map_or("", |(_, q)| q);
    query.split('&').find_map(|kv| kv.strip_prefix(name)?.strip_prefix('=').map(decode))
}

/// The relay's QUIC door (#438), from its URL – the base (`https://host:port`) and the hash to pin. A `wss://` relay under
/// a name has one on its own host and port, a relay opens it with its certificate. `?quic=host:port` names another,
/// an empty `?quic=` asks for the relay's own, `?quic=off` for none, `&quichash=<sha-256 of the certificate, hex>` pins a
/// self-signed one.
pub fn parse_quic(url: &str) -> Option<(String, Option<Vec<u8>>)> {
    let url = url.trim();
    let (secure, rest) = match url.strip_prefix("wss://") {
        Some(rest) => (true, rest),
        None => (false, url.strip_prefix("ws://")?),
    };
    let param = |name: &str| relay_param(url, name);
    let own = rest.split(['/', '?', '#']).next().unwrap_or("");
    let door = match param("quic") {
        Some(off) if off == "off" => return None,
        Some(door) if !door.is_empty() => door,
        Some(_) => own.to_string(),
        None if secure && door_by_default(own) => own.to_string(),
        None => return None,
    };
    Some((format!("https://{door}"), param("quichash").and_then(|h| hex(&h))))
}

/// An address cannot have a public certificate, so a `wss://` by IP is a proxy or a pinned relay, which name their door.
fn door_by_default(authority: &str) -> bool {
    let host = authority.rsplit_once(':').map_or(authority, |(host, _)| host);
    !host.starts_with('[') && host.parse::<std::net::IpAddr>().is_err()
}

/// a hex string's bytes; None when it is empty or not hex
fn hex(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
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
pub(crate) fn encode(s: &str) -> String {
    s.bytes().map(|c| if c.is_ascii_alphanumeric() || b"-_.~".contains(&c) { (c as char).to_string() } else { format!("%{c:02X}") }).collect()
}

/// the role's name on the wire
fn role_name(r: Role) -> String {
    serde_json::to_value(r).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

/// How a client joins.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Options {
    /// `ws://host:port/live/<code>` or `wss://…`; which QUIC door it has: `parse_quic`
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
    /// #478: what this client says it is (`hello.device`: native, browser, phone …) – a role's filter may ask it
    #[serde(default)]
    pub device: Option<String>,
    /// #478: the API's signed facts of this client for its session (`hello.proof`; deck-outlet's proof.rs asks for it)
    #[serde(default)]
    pub proof: Option<String>,
    /// #478: the app asks for a proof (`proof_answered` brings the answer): the first dial waits for it, at most
    /// `PROOF_WAIT_MS` – a hello with the proof, not a guest's hello and a join again a moment later
    #[serde(default)]
    pub wait_proof: bool,
    /// a reader phone's own secret for the live cameras, kept in its browser: a drop or a reload is the same holder
    #[serde(default)]
    pub key: Option<String>,
    /// how long QUIC runs alone before a WebSocket is dialled beside it (`QUIC_HEAD_START_MS` when none). A test that must
    /// see QUIC win the race gives it all the time it needs – a loaded machine's handshake is no fixed 250 ms
    #[serde(default)]
    pub quic_head_start_ms: Option<f64>,
    /// how long a QUIC dial nothing else waits for may take (`QUIC_GIVE_UP_MS` when none)
    #[serde(default)]
    pub quic_give_up_ms: Option<f64>,
}

impl Options {
    pub fn new(relay: &str, role: Role, group: Option<&str>) -> Options {
        Options { relay: relay.into(), role, group: group.map(String::from), nick: None, token: None, resume: false, messages: false, device: None, proof: None, wait_proof: false, key: None, quic_head_start_ms: None, quic_give_up_ms: None }
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
    /// #517: an address action (`address`, SHOW.md §8) for this deck's playing scene – the core's scene_play.rs reads it;
    /// `beat`: the show clock's beat it plays on (a show cell's camera cut on its row's bar or phrase), none = now
    Address { address: String, action: String, beat: Option<f64> },
    /// #517: a reader's story event (the VNEvent's JSON), there because this deck asked (`set_story`)
    Story(Loose),
    /// a phone holding one of the session's live cameras (live_cameras.rs): the deck whose scene it is follows it
    Held(Held),
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
    /// #438: open a QUIC / WebTransport connection and its bidirectional stream (then `quic_opened`, or `quic_closed`);
    /// `hash`: the certificate's SHA-256 to pin instead of the usual trust
    DialQuic { url: String, hash: Option<Vec<u8>> },
    /// write these bytes (the framing included) to the QUIC stream
    Stream(String),
    /// send this as one QUIC datagram (it may be lost)
    Datagram(String),
    /// close the QUIC connection (then `quic_closed`)
    CloseQuic {},
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
    /// #442: a deck asks for the far addresses it names (far.rs: this deck answers those that name it)
    Listen { client: String, addresses: Vec<String> },
    /// #442: events a deck sends for this deck's far world nodes
    Events { client: String, events: Vec<DeckEvent> },
    /// #469: the editor (a conductor) hears a phone's pose (a late or malformed one never comes out), state, Record / Stop
    /// and Recenter; a phone the editor's view
    HandPose { client: String, pose: HandPose },
    HandState { client: String, state: HandState },
    HandRecord { client: String, on: bool },
    HandRecenter { client: String },
    HandView(HandView),
    /// #469: a phone that spoke is no longer in the session (the relay's presence lacks it); it may come back as the same
    /// client (a fall to the WebSocket): its state comes again then
    HandAway { client: String },
    /// #527: a reader's place in the session's story changed (linked.rs `Driving`: asked, drives, taken over; whether
    /// another reader drives)
    Driving(Driving),
    /// #552: a story question's result for the driving reader (linked.rs `Choice`): pick that option, or ignored and why
    Choice(Choice),
    /// a phone's hold on a live camera changed (live_cameras.rs `Holding`: asked, holds, taken over – who has it)
    Holding(Holding),
    /// the session's cameras as the relay shows them to this client, with who holds each
    Cameras(Vec<LiveCamera>),
    /// the conductor flashed the camera this phone holds: what it now is
    CameraFlash { camera: String, settings: Loose },
    /// #478: fetch this client's proof with the person's sign-in (`Client::ask_proof`), the answer to `proof`
    AskProof { url: String },
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

/// one of the two sockets
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sock {
    Idle,
    Dialing,
    Up,
}

/// The QUIC door and how the two sockets stand (#438).
struct Quic {
    base: String,
    hash: Option<Vec<u8>>,
    quic: Sock,
    ws: Sock,
    /// a WebSocket is due beside QUIC at this ms
    ws_at: Option<f64>,
    /// a dial nothing else waits for ends at this ms
    give_up_at: f64,
    /// the open link is QUIC
    on_quic: bool,
    /// it died: the next join goes straight to the WebSocket
    fell: bool,
    /// `rejoin`: the next join is on QUIC again
    keep: bool,
    /// on the WebSocket: try QUIC at this ms, after this wait
    step_up_at: Option<f64>,
    step_up_wait: f64,
    lines: Lines,
    /// our datagrams' number, so the door drops one that was overtaken
    up_seq: u64,
    /// values went as datagrams since the last settle
    dirty: bool,
    settle_at: f64,
    /// a settle is due: the next change goes as the full values, on the stream
    carry: bool,
    /// each deck's value: the newest `seq` applied
    seen: HashMap<(String, String), u64>,
}

impl Quic {
    fn new(base: String, hash: Option<Vec<u8>>) -> Quic {
        Quic {
            base,
            hash,
            quic: Sock::Idle,
            ws: Sock::Idle,
            ws_at: None,
            give_up_at: 0.0,
            on_quic: false,
            fell: false,
            keep: false,
            step_up_at: None,
            step_up_wait: STEP_UP_MS,
            lines: Lines::default(),
            up_seq: 0,
            dirty: false,
            settle_at: 0.0,
            carry: false,
            seen: HashMap::new(),
        }
    }

    /// `seq` is not older than what this deck's `key` showed last
    fn fresh(&mut self, from: &str, key: &str, seq: u64) -> bool {
        let at = self.seen.entry((from.to_string(), key.to_string())).or_default();
        let fresh = seq >= *at;
        if fresh {
            *at = seq;
        }
        fresh
    }
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
    // #556: the vote this client may cast in, as its ballot shows it (deck-ui ballot.rs), on this client's clock
    ballot: deck_ui::ballot::State,
    /// #438: the relay has a QUIC door
    door: Option<Quic>,
    // #442: the far addresses this deck asks for, and when it last asked
    listen: Vec<String>,
    listen_sent: f64,
    /// #517: this deck's playing scene listens to story events: it asks for them (`deck.story`)
    story: bool,
    story_sent: f64,
    /// #527: a reader's claim to drive the session's story and the relay's word on who does
    steer: Steer,
    /// #552: where a reader's story stands for a vote's result
    place: Place,
    /// #527: a conductor's view of the linked readers
    readers: Readers,
    // #469: a phone's state and the editor's view, sent again after every welcome; each phone's newest pose heard (its `t`)
    hand_state: Option<HandState>,
    hand_view: Option<HandView>,
    poses: HashMap<String, f64>,
    /// the phones heard and still in the session
    hands: std::collections::BTreeSet<String>,
    /// #478: the roles the relay says this client may take (with how many are in each), and whether the conductor locked it
    roles: Vec<ShowRole>,
    locked: bool,
    /// whether a conductor is in the session, as the relay last said (None: not said yet)
    conductor: Option<bool>,
    /// #478: the first dial waits for the proof's answer (`wait_proof`): until this ms (None: it does not wait); the answer
    /// came (`proof_answered`)
    proof_hold: Option<f64>,
    proof_heard: bool,
    /// a reader phone's hold on a live camera; the cameras as last heard; a conductor's cameras, sent at every welcome
    seat: Seat,
    cameras: Vec<LiveCamera>,
    camera_defs: Option<Vec<LiveCamera>>,
    /// a deck's: who holds which camera, so a holder's stream is that camera's
    holders: Holders,
    /// #478: a phone that asks its own proof (`ask_proof`): the API and when to ask again
    proof_ask: Option<(String, protocol::proof::Renew)>,
}

impl Client {
    pub fn new(mut o: Options) -> Result<Client, String> {
        let (base, code) = parse_relay(&o.relay)?;
        let door = parse_quic(&o.relay).map(|(base, hash)| Quic::new(base, hash));
        // moving between the two sockets keeps the client: what it missed is replayed
        o.resume |= door.is_some();
        let seat = Seat::new(o.key.as_deref().unwrap_or_default());
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
            ballot: Default::default(),
            door,
            listen: Vec::new(),
            listen_sent: f64::NEG_INFINITY,
            story: false,
            story_sent: f64::NEG_INFINITY,
            steer: Steer::default(),
            place: Place::default(),
            readers: Readers::default(),
            hand_state: None,
            hand_view: None,
            poses: HashMap::new(),
            hands: Default::default(),
            roles: Vec::new(),
            locked: false,
            conductor: None,
            proof_hold: None,
            proof_heard: false,
            seat,
            cameras: Vec::new(),
            camera_defs: None,
            holders: Holders::default(),
            proof_ask: None,
        })
    }

    /// #478: take a role (the conductor's, one the relay offered), or none – at once, at any time; the relay refuses it when
    /// the conductor locked this client there. Its group is the role, so a rejoin keeps it.
    pub fn take_role(&mut self, role: Option<&str>) -> bool {
        let a = Assign { client: self.client.clone(), group: role.unwrap_or_default().to_string(), lock: None };
        self.joined && self.send(Message::Assign(a))
    }

    /// #478: the name this client says (free text; the relay trims it) – in its next hello, and now when it is joined
    pub fn set_name(&mut self, name: &str) -> bool {
        let name = name.trim();
        self.o.nick = (!name.is_empty()).then(|| name.to_string());
        self.joined && self.send(Message::Rename(Rename { client: self.client.clone(), nick: name.to_string() }))
    }

    /// #478: the API's proof of this client's facts (`None`: none) – in every hello from now. The first one after a hello
    /// without it joins again at once (with resume), so the relay learns the facts; a renewed one waits for the next hello.
    pub fn set_proof(&mut self, proof: Option<String>) {
        let first = self.o.proof.is_none() && proof.is_some();
        self.o.proof = proof;
        if first && self.phase == Phase::Open {
            self.rejoin();
        }
    }

    /// #478: the API answered the ask for the proof – `Some(p)`: the proof from now (`set_proof`), `None`: the answer said
    /// nothing, the proof it has stays (deck-outlet proof.rs `Renew::answered`). A first dial that waits for it goes now.
    pub fn proof_answered(&mut self, proof: Option<Option<String>>) {
        self.proof_heard = true;
        if let Some(p) = proof {
            self.set_proof(p);
        }
        if self.proof_hold.take().is_some() {
            // the next `tick` dials (the adapter's timer: `next_wake` is due)
            self.phase = Phase::Waiting(f64::NEG_INFINITY);
        }
    }

    /// #478: a page that has the person's sign-in (a reader's camera phone) asks the API at `api` for this client's proof
    /// itself: `Out::AskProof` says when, `proof` takes the answer; the first hello waits for it. Call before `start`.
    pub fn ask_proof(&mut self, api: &str) {
        self.proof_ask = Some((api.trim().trim_end_matches('/').to_string(), protocol::proof::Renew::default()));
        self.o.wait_proof = true;
    }

    /// #478: the API's answer to `Out::AskProof` at `now` (`status` 0: no answer)
    pub fn proof(&mut self, now: f64, status: u16, body: &str) {
        let Some((_, renew)) = self.proof_ask.as_mut() else { return };
        let body = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        let answer = renew.answered(now / 1000.0, protocol::proof::read(status, &body));
        self.proof_answered(answer);
    }

    /// #478: who is signed in changed: the proof is asked again at once
    pub fn proof_again(&mut self, now: f64) {
        if let Some((_, renew)) = self.proof_ask.as_mut() {
            renew.again();
        }
        self.proof_due(now);
    }

    fn proof_due(&mut self, now: f64) {
        let Some((api, renew)) = self.proof_ask.as_mut() else { return };
        if renew.due(now / 1000.0) {
            let url = protocol::proof::request(api, &self.code);
            self.out.push(Out::AskProof { url });
        }
    }

    /// #478: what the Live page shows of this client's session (deck-ui live.rs)
    pub fn live(&self) -> deck_ui::live::Session {
        let offers = self.roles.iter().map(|r| deck_ui::live::Offer { id: r.id.clone(), label: r.label.clone().unwrap_or_else(|| r.id.clone()), here: r.here.unwrap_or(0) as u32 });
        deck_ui::live::Session {
            code: self.code.clone(),
            link: self.line(),
            joined: self.joined,
            offers: offers.collect(),
            role: self.o.group.clone(),
            name: self.o.nick.clone().unwrap_or_default(),
            locked: self.locked,
            conductor: self.conductor,
        }
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

    /// the socket is open (welcomed or not)
    pub fn open(&self) -> bool {
        self.phase == Phase::Open
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
        format!("{}{}", self.base, self.path())
    }

    fn path(&self) -> String {
        let group = self.o.group.as_deref().map(|g| format!("&group={}", encode(g))).unwrap_or_default();
        format!("/live/{}?role={}{group}", encode(&self.code), role_name(self.o.role))
    }

    /// the link in use is QUIC
    pub fn on_quic(&self) -> bool {
        self.door.as_ref().is_some_and(|q| q.on_quic)
    }

    /// #469: the rung of the ladder the joined link is on, as a page shows it (`QUIC`, `WebSocket`); None while not joined
    pub fn rung(&self) -> Option<&'static str> {
        self.joined.then(|| if self.on_quic() { "QUIC" } else { "WebSocket" })
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
                let via = if self.on_quic() { " QUIC" } else { "" };
                match self.o.role {
                    Role::Conductor => format!("CONDUCTOR {c} @ {code}{via}"),
                    Role::Output => format!("OUTPUT {c} OF {} @ {code}{via}", self.o.group.as_deref().unwrap_or("")),
                    Role::Audience => format!("AUDIENCE {c} @ {code}{via}"),
                    Role::Handheld => format!("HANDHELD {c} @ {code}{via}"),
                    Role::Reader => format!("READER {c} @ {code}{via}"),
                    _ => format!("DECK {c} @ {code}{via}"),
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
        let door = self.door.as_ref();
        match self.phase {
            Phase::Waiting(at) => Some(at),
            Phase::Dialing => door.and_then(|q| q.ws_at.or((q.quic == Sock::Dialing && q.ws == Sock::Idle).then_some(q.give_up_at))),
            Phase::Open => {
                let ping = self.last_ping + PING_MS;
                let answer = (self.joined && self.answer_due).then_some(self.answered + PROGRESS_MS);
                let settle = door.filter(|q| self.joined && q.on_quic && q.dirty).map(|q| q.settle_at);
                let step_up = door.and_then(|q| q.step_up_at.filter(|_| q.quic == Sock::Idle));
                Some([answer, settle, step_up].into_iter().flatten().fold(ping, f64::min))
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
        if self.phase != Phase::Idle {
            return;
        }
        self.proof_due(now);
        if self.o.wait_proof && !self.proof_heard {
            // #478: the first hello waits for the proof's answer, not longer than PROOF_WAIT_MS
            self.proof_hold = Some(now + PROOF_WAIT_MS);
            self.phase = Phase::Waiting(now + PROOF_WAIT_MS);
            return;
        }
        self.dial(now);
    }

    fn dial(&mut self, now: f64) {
        self.proof_hold = None;
        self.phase = Phase::Dialing;
        self.joined = false;
        self.set_status(Status { state: State::Joining, client: None, why: None, wait: None });
        let path = self.path();
        let head_start = self.o.quic_head_start_ms.unwrap_or(QUIC_HEAD_START_MS);
        let give_up = self.o.quic_give_up_ms.unwrap_or(QUIC_GIVE_UP_MS);
        if let Some(q) = self.door.as_mut() {
            q.on_quic = false;
            q.step_up_at = None;
            q.lines.clear();
            q.seen.clear();
            q.dirty = false;
            q.carry = false;
            if !q.fell {
                q.quic = Sock::Dialing;
                q.ws = Sock::Idle;
                q.ws_at = Some(now + head_start);
                q.give_up_at = now + give_up;
                self.out.push(Out::DialQuic { url: format!("{}{path}", q.base), hash: q.hash.clone() });
                return;
            }
            q.ws = Sock::Dialing;
            q.ws_at = None;
        }
        self.out.push(Out::Dial(self.url()));
    }

    /// the WebSocket beside the QUIC dial
    fn dial_ws(&mut self) {
        if let Some(q) = self.door.as_mut() {
            q.ws = Sock::Dialing;
            q.ws_at = None;
        }
        self.out.push(Out::Dial(self.url()));
    }

    /// the WebSocket is open: hello first
    pub fn opened(&mut self, now: f64) {
        if let Some(q) = self.door.as_mut() {
            if self.phase == Phase::Open && q.on_quic {
                // the race's loser got there late
                q.ws = Sock::Idle;
                self.out.push(Out::Close {});
                return;
            }
            if self.phase != Phase::Dialing {
                return;
            }
            q.ws = Sock::Up;
            q.ws_at = None;
            q.fell = false;
            q.step_up_at = Some(now + q.step_up_wait);
            if q.quic != Sock::Idle {
                q.quic = Sock::Idle;
                self.out.push(Out::CloseQuic {});
            }
        }
        if self.phase != Phase::Dialing {
            return;
        }
        self.won(now);
    }

    /// #438: the QUIC connection and its stream are up
    pub fn quic_opened(&mut self, now: f64) {
        let Some(q) = self.door.as_mut().filter(|q| q.quic == Sock::Dialing) else { return };
        match self.phase {
            Phase::Dialing => {
                q.quic = Sock::Up;
                q.on_quic = true;
                q.fell = false;
                q.ws_at = None;
                q.step_up_wait = STEP_UP_MS;
                q.settle_at = now + SETTLE_MS;
                if std::mem::replace(&mut q.ws, Sock::Idle) != Sock::Idle {
                    self.out.push(Out::Close {});
                }
                self.won(now);
            }
            Phase::Open => {
                // stepping up: the WebSocket goes, the hello at once (the relay gives a resume the old link's place)
                q.quic = Sock::Up;
                q.on_quic = true;
                q.ws = Sock::Idle;
                q.step_up_wait = STEP_UP_MS;
                q.settle_at = now + SETTLE_MS;
                self.joined = false;
                self.out.push(Out::Close {});
                self.last_ping = now;
                self.hello();
            }
            _ => {
                q.quic = Sock::Idle;
                self.out.push(Out::CloseQuic {});
            }
        }
    }

    /// a socket won the race: hello first
    fn won(&mut self, now: f64) {
        self.phase = Phase::Open;
        self.tries = 0;
        self.last_ping = now;
        self.hello();
    }

    /// whether a filter takes this client by the facts it knows of itself: its group and its device. Account facts and an
    /// approved deck come from the server's proof – the relay decides those, a deck can't prove them to itself, so they are
    /// left out here (else a row address naming an account would miss every deck)
    fn passes(&self, filter: Option<&protocol::Filter>) -> bool {
        let Some(f) = filter else { return true };
        let mine = protocol::Filter { group: f.group.clone(), device: f.device.clone(), account: None, approved: None };
        protocol::filter::passes(Some(&mine), &protocol::filter::Facts { group: self.o.group.as_deref(), device: self.o.device.as_deref(), ..Default::default() })
    }

    fn hello(&mut self) {
        let conductor = self.o.role == Role::Conductor;
        let hello = Hello {
            code: self.code.clone(),
            role: self.o.role,
            group: self.o.group.clone(),
            nick: self.o.nick.clone(),
            device: self.o.device.clone(),
            token: self.o.token.clone().or_else(|| conductor.then(|| "dev".into())),
            proof: self.o.proof.clone(),
            resume: self.resume.clone().filter(|_| self.o.resume),
        };
        self.send(Message::Hello(hello));
    }

    /// The WebSocket closed, or its dial failed (`why`: what the adapter knows, may be empty): a joined link joins again
    /// soon, another backs off.
    pub fn closed(&mut self, now: f64, why: &str) {
        if let Some(q) = self.door.as_mut() {
            let on_quic = q.on_quic;
            q.ws = Sock::Idle;
            if self.phase == Phase::Open && on_quic {
                return; // the race's loser, or the socket left behind by a step up
            }
            if self.phase == Phase::Dialing {
                q.ws_at = None;
                if q.quic == Sock::Dialing {
                    return; // QUIC may still come; `tick` ends the wait
                }
            }
            if q.quic != Sock::Idle {
                q.quic = Sock::Idle;
                self.out.push(Out::CloseQuic {});
            }
            q.fell = false;
        }
        self.lost(now, why);
    }

    /// #438: the QUIC connection ended, or its dial failed.
    pub fn quic_closed(&mut self, now: f64, why: &str) {
        let Some(q) = self.door.as_mut().filter(|q| q.quic != Sock::Idle) else { return };
        q.quic = Sock::Idle;
        match self.phase {
            Phase::Dialing if q.ws == Sock::Dialing => {}
            // no WebSocket yet: no reason to wait out the head start
            Phase::Dialing if q.ws_at.is_some() => self.dial_ws(),
            Phase::Dialing => self.lost(now, why),
            Phase::Open if q.on_quic => {
                q.fell = !std::mem::take(&mut q.keep);
                q.on_quic = false;
                self.lost(now, why);
            }
            Phase::Open => {
                q.step_up_wait = (q.step_up_wait * 2.0).min(STEP_UP_MAX_MS);
                q.step_up_at = Some(now + q.step_up_wait);
            }
            _ => {}
        }
    }

    /// #438: bytes of the QUIC stream, as they came
    pub fn quic_stream(&mut self, now: f64, bytes: &[u8]) {
        let Some(q) = self.door.as_mut().filter(|q| q.on_quic) else { return };
        match q.lines.push(bytes) {
            Ok(lines) => {
                for text in lines {
                    self.text(now, &text);
                }
            }
            Err(_) => self.out.push(Out::CloseQuic {}),
        }
    }

    /// #438: a QUIC datagram – only `deck.values` and #469 `hand.pose`, and it does not move the resume's `seq` (it can
    /// overtake the stream)
    pub fn quic_datagram(&mut self, now: f64, bytes: &[u8]) {
        if !self.on_quic() {
            return;
        }
        let Ok(e) = serde_json::from_slice::<Envelope>(bytes) else { return };
        if matches!(e.message, Message::DeckValues(_) | Message::HandPose(_)) {
            self.envelope(now, e, false);
        }
    }

    fn lost(&mut self, now: f64, why: &str) {
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
        self.proof_due(now);
        match self.phase {
            Phase::Waiting(at) if now >= at => self.dial(now),
            Phase::Dialing => {
                let Some(q) = self.door.as_mut() else { return };
                if q.ws_at.is_some_and(|at| now >= at) {
                    self.dial_ws();
                } else if q.quic == Sock::Dialing && q.ws == Sock::Idle && q.ws_at.is_none() && now >= q.give_up_at {
                    q.quic = Sock::Idle;
                    self.out.push(Out::CloseQuic {});
                    self.lost(now, "QUIC did not answer");
                }
            }
            Phase::Open => {
                let path = self.path();
                if let Some(q) = self.door.as_mut() {
                    if !q.on_quic && q.quic == Sock::Idle && q.step_up_at.is_some_and(|at| now >= at) {
                        q.step_up_at = None;
                        q.quic = Sock::Dialing;
                        self.out.push(Out::DialQuic { url: format!("{}{path}", q.base), hash: q.hash.clone() });
                    }
                }
                if now - self.last_ping >= PING_MS {
                    self.ping(now);
                }
                self.flush(now);
                self.settle(now);
                self.flush_listen(now);
                self.flush_story(now);
            }
            _ => {}
        }
    }

    /// on QUIC, while values change: every value this deck sent so far, on the stream, in the place of a change if one comes
    fn settle(&mut self, now: f64) {
        let joined = self.joined;
        let Some(q) = self.door.as_mut().filter(|q| joined && q.on_quic && q.dirty && now >= q.settle_at) else { return };
        if !q.carry {
            q.carry = true;
            q.settle_at = now + SETTLE_GRACE_MS;
            return;
        }
        q.settle_at = now + SETTLE_MS;
        self.send_settle();
    }

    fn send_settle(&mut self) {
        if let Some(q) = self.door.as_mut() {
            q.dirty = false;
            q.carry = false;
        }
        let all = DeckValues { values: self.sent.clone().into_iter().collect(), mood: self.mood.clone() };
        self.send(Message::DeckValues(all));
    }

    /// leave for good: the socket closes, nothing more happens
    pub fn stop(&mut self) {
        if matches!(self.phase, Phase::Dialing | Phase::Open) {
            match self.door.as_mut() {
                Some(q) => {
                    if std::mem::replace(&mut q.quic, Sock::Idle) != Sock::Idle {
                        self.out.push(Out::CloseQuic {});
                    }
                    if std::mem::replace(&mut q.ws, Sock::Idle) != Sock::Idle {
                        self.out.push(Out::Close {});
                    }
                }
                None => self.out.push(Out::Close {}),
            }
        }
        self.phase = Phase::Stopped;
        self.joined = false;
    }

    /// drop the connection now and join again (with resume) – a network change, tests
    pub fn rejoin(&mut self) {
        if !matches!(self.phase, Phase::Dialing | Phase::Open) {
            return;
        }
        match self.door.as_mut() {
            Some(q) if q.on_quic => {
                q.keep = true;
                self.out.push(Out::CloseQuic {});
            }
            _ => self.out.push(Out::Close {}),
        }
    }

    fn send(&mut self, m: Message) -> bool {
        self.send_on(m, false)
    }

    /// `may_lose`: a value change, which on QUIC goes as a datagram when it fits one
    fn send_on(&mut self, m: Message, may_lose: bool) -> bool {
        self.send_envelope(Envelope::new("", m), may_lose)
    }

    /// a message addressed to `to` (the relay's default otherwise); on QUIC it takes the stream
    fn send_to(&mut self, m: Message, to: To) -> bool {
        let mut e = Envelope::new("", m);
        e.to = Some(to);
        self.send_envelope(e, false)
    }

    fn send_envelope(&mut self, mut e: Envelope, may_lose: bool) -> bool {
        if self.phase != Phase::Open {
            return false;
        }
        *self.counts.entry(e.message.kind()).or_default() += 1;
        let on_quic = self.on_quic();
        let values = matches!(e.message, Message::DeckValues(_));
        // values on the stream say which datagram they follow: the door keeps what a newer one has said since
        if let Some(q) = self.door.as_mut().filter(|_| on_quic && values) {
            q.up_seq += u64::from(may_lose);
            e.seq = q.up_seq;
        }
        // through a Value: its maps are sorted – the same text natively and in wasm, whatever a HashMap's order
        let text = serde_json::to_value(e).map(|v| v.to_string()).unwrap_or_default();
        if !on_quic {
            self.out.push(Out::Send(text));
        } else if may_lose && text.len() <= DATAGRAM_MAX {
            // a pose says all of itself: only values need the settle
            if let Some(q) = self.door.as_mut().filter(|_| values) {
                q.dirty = true;
            }
            self.out.push(Out::Datagram(text));
        } else {
            self.out.push(Out::Stream(String::from_utf8_lossy(&line(&text)).into_owned()));
        }
        true
    }

    /// any message on the open socket (the outlet's `media.ready`)
    pub fn send_message(&mut self, m: Message) -> bool {
        self.send(m)
    }

    /// #91: an envelope as the app made it – a conductor's (deck-show): its moment (`at`) and its `to` kept
    pub fn send_whole(&mut self, e: Envelope) -> bool {
        self.send_envelope(e, false)
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
        if !self.joined {
            return;
        }
        if let Some(q) = self.door.as_mut().filter(|q| q.on_quic && q.carry) {
            q.settle_at += SETTLE_MS - SETTLE_GRACE_MS;
            self.send_settle();
        } else {
            self.send_on(Message::DeckValues(v), true);
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

    /// #442: the far addresses this deck's world nodes name – asked for once joined, at each welcome and every
    /// `far::LISTEN_MS` (far.rs); empty = nothing to ask
    pub fn set_listen(&mut self, now: f64, addresses: Vec<String>) {
        self.listen = addresses;
        self.listen_sent = f64::NEG_INFINITY;
        self.flush_listen(now);
    }

    fn flush_listen(&mut self, now: f64) {
        if self.joined && !self.listen.is_empty() && now - self.listen_sent >= crate::far::LISTEN_MS {
            self.listen_sent = now;
            self.send_to(Message::DeckListen(DeckListen { addresses: self.listen.clone() }), To { role: None, group: Some(DECKS.to_string()), client: None });
        }
    }

    /// #517: whether this deck's playing scene listens to story events (scene_play.rs `Playing::listens`) – asked for once
    /// joined, at each welcome and every `LISTEN_MS` while it does; an ask not renewed lapses at the relay
    pub fn set_story(&mut self, now: f64, on: bool) {
        if self.story != on {
            self.story = on;
            self.story_sent = f64::NEG_INFINITY;
            self.flush_story(now);
        }
    }

    fn flush_story(&mut self, now: f64) {
        if self.joined && self.story && now - self.story_sent >= protocol::LISTEN_MS {
            self.story_sent = now;
            self.send_to(Message::DeckStory(Empty {}), To { role: None, group: Some(DECKS.to_string()), client: None });
        }
    }

    /// #518: a story event the reader fired (the VNEvent as it is: `type`, `params`, `id`) as a `story` message, in the
    /// order fired. Which events matter is the receiver's (deck-events adapters/story.rs `STAYS_IN_THE_STORY`), not the
    /// reader's. #527: only while it drives the session's story (`drive`, granted by the relay) – before the answer, while
    /// away or after a take-over it is dropped, never held and sent later. False: dropped.
    /// #552: every event also moves where the reader's story stands for a vote's result (linked.rs `Place`), sent or not.
    pub fn story(&mut self, event: Loose) -> bool {
        self.place.fired(&event);
        self.joined && self.steer.drives() && self.send(Message::Story(event))
    }

    /// #552: the reader's runner says its choices closed (picked, or none was shown): it stands at no choice now
    pub fn choices_closed(&mut self) {
        self.place.closed();
    }

    /// #527: the person said yes – this reader drives the session's story from the relay's answer on (`reader.drive`; the
    /// newest claim wins, the reader before stops). Sent now when joined, else at the welcome; again after a drop unless
    /// another reader took the place meanwhile. False: this client is no reader.
    pub fn drive(&mut self) -> bool {
        if self.o.role != Role::Reader {
            return false;
        }
        self.steer.claim();
        if self.joined {
            self.send(Message::ReaderDrive(protocol::ReaderDrive::default()));
        }
        self.tell_driving();
        true
    }

    /// #527: a reader's place in the session's story (asked, drives, taken over) and whether another reader drives
    pub fn driving(&self) -> Driving {
        self.steer.now(&self.client)
    }

    /// #527: a conductor's view of the linked readers: who is in by name, which one drives, the last story heard
    pub fn readers(&self) -> &Readers {
        &self.readers
    }

    fn tell_driving(&mut self) {
        if let Some(d) = self.steer.change(&self.client) {
            self.out.push(Out::Driving(d));
        }
    }

    /// the person said yes to `camera` (or Take it back): this phone holds it from the relay's answer on (the newest
    /// claim wins). Sent now when joined, else at the welcome; taken again by itself after a drop or a reload while
    /// nobody else claimed it. False: no reader, or it has no key.
    pub fn camera_claim(&mut self, camera: &str) -> bool {
        if self.o.role != Role::Reader || self.seat.key().is_empty() {
            return false;
        }
        let claim = self.seat.claim(camera);
        if self.joined {
            self.send(Message::CameraClaim(claim));
        }
        self.tell_holding();
        true
    }

    /// After a reload: the page held `camera` as the client `was` (its browser kept both) – taken again by itself at the
    /// welcome while nobody else claimed it. False: no reader, or no key.
    pub fn camera_resume(&mut self, camera: &str, was: &str) -> bool {
        if self.o.role != Role::Reader || self.seat.key().is_empty() {
            return false;
        }
        self.seat.resume(camera, was);
        if self.joined {
            if let Some(claim) = self.seat.welcome(&self.client) {
                self.send(Message::CameraClaim(claim));
            }
        }
        self.tell_holding();
        true
    }

    /// Leave – the phone lets its camera go
    pub fn camera_leave(&mut self) -> bool {
        let Some(camera) = self.seat.leave() else { return false };
        if self.joined {
            self.send(Message::CameraLeave(CameraLeave { camera }));
        }
        self.tell_holding();
        true
    }

    /// a deck (a musician, an output, a performer): a camera a phone holds may be its own to follow
    fn looks(&self) -> bool {
        matches!(self.o.role, Role::Musician | Role::Output | Role::Performer)
    }

    /// a phone's hold on a camera as its page shows it
    pub fn holding(&self) -> Holding {
        self.seat.now(&self.client)
    }

    /// the session's cameras as the relay last showed them to this client (with who holds each)
    pub fn cameras(&self) -> &[LiveCamera] {
        &self.cameras
    }

    /// a conductor's cameras – kept, sent again at every welcome
    pub fn set_cameras(&mut self, cameras: Vec<LiveCamera>) {
        self.camera_defs = Some(cameras.clone());
        if self.joined {
            self.send(Message::Cameras(Cameras { cameras }));
        }
    }

    /// a conductor forces the phone `client` onto `camera` ('' frees what it holds) – only while joined
    pub fn camera_force(&mut self, client: &str, camera: &str) -> bool {
        self.joined && self.send(Message::CameraForce(CameraForce { client: client.to_string(), camera: camera.to_string(), r#where: None }))
    }

    /// a conductor flashes a camera's settings to the phone holding it – only while joined
    pub fn camera_flash(&mut self, camera: &str, settings: Loose) -> bool {
        self.joined && self.send(Message::CameraFlash(CameraFlash { camera: camera.to_string(), settings }))
    }

    fn tell_holding(&mut self) {
        if let Some(h) = self.seat.change(&self.client) {
            self.out.push(Out::Holding(h));
        }
    }

    /// #442: events for one subscriber's far world nodes (only while joined – stale events are dropped)
    pub fn events(&mut self, client: &str, events: Vec<DeckEvent>) -> bool {
        self.joined && !events.is_empty() && self.send_to(Message::DeckEvents(DeckEvents { events }), To { role: None, group: None, client: Some(client.to_string()) })
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

    /// #469: a phone's pose at `t` (its clock, ms) – only while joined (a stale pose is worth nothing); on QUIC a datagram
    pub fn hand_pose(&mut self, t: f64, p: &[f64], q: &[f64]) -> bool {
        self.joined && self.send_on(Message::HandPose(HandPose { t, p: p.to_vec(), q: q.to_vec() }), true)
    }

    /// #469: what the phone tracks – kept, sent again after every welcome (`ua` trimmed to 300 characters)
    pub fn hand_state(&mut self, mut s: HandState) {
        s.ua = s.ua.map(|ua| ua.chars().take(300).collect());
        self.hand_state = Some(s.clone());
        if self.joined {
            self.send(Message::HandState(s));
        }
    }

    /// #469: the phone's Record (`on`) or Stop – only while joined
    pub fn hand_record(&mut self, on: bool) -> bool {
        self.joined && self.send(Message::HandRecord(HandRecord { on }))
    }

    /// #469: the phone's Recenter – only while joined
    pub fn hand_recenter(&mut self) -> bool {
        self.joined && self.send(Message::HandRecenter(Empty {}))
    }

    /// #469: the editor's view for its phones – kept, sent again after every welcome
    pub fn hand_view(&mut self, v: HandView) {
        self.hand_view = Some(v.clone());
        if self.joined {
            self.send(Message::HandView(v));
        }
    }

    /// #469: a phone spoke – one new to this client (it joined after the welcome, or came back) gets the editor's view
    fn hand_heard(&mut self, from: &str) {
        if self.hands.insert(from.to_string()) {
            if let Some(v) = self.hand_view.clone() {
                self.send_to(Message::HandView(v), To { role: None, group: None, client: Some(from.to_string()) });
            }
        }
    }

    /// #469: a phone's pose as it came – false when it is malformed or not newer than that phone's last (a late one)
    fn fresh_pose(&mut self, from: &str, p: &HandPose) -> bool {
        let finite = |v: &[f64], n: usize| v.len() == n && v.iter().all(|x| x.is_finite());
        if !(p.t.is_finite() && finite(&p.p, 3) && finite(&p.q, 4)) || self.poses.get(from).is_some_and(|t| p.t <= *t) {
            return false;
        }
        self.poses.insert(from.to_string(), p.t);
        true
    }

    /// the ballot marks a cast only once it went out
    pub fn vote_cast(&mut self, option: i64) -> bool {
        let Some(vote) = self.vote.as_ref().map(|v| v.vote.clone()) else { return false };
        let sent = self.send(Message::VoteCast(VoteCast { vote, option }));
        if let Some(o) = usize::try_from(option).ok().filter(|_| sent) {
            self.ballot.cast(o);
        }
        sent
    }

    /// its times are this client's clock (the `now` of its calls), so ask `view` with that clock
    pub fn ballot(&self) -> &deck_ui::ballot::State {
        &self.ballot
    }

    pub fn current_vote(&self) -> Option<Envelope> {
        self.vote.clone().map(|v| relay_says(Message::VoteOpen(v)))
    }

    /// A message's text arrived.
    pub fn text(&mut self, now: f64, text: &str) {
        let Ok(e) = serde_json::from_str::<Envelope>(text) else { return };
        self.envelope(now, e, true);
    }

    /// `in_order`: it came on the reliable lane, so the resume may count it
    fn envelope(&mut self, now: f64, e: Envelope, in_order: bool) {
        if self.phase != Phase::Open {
            return;
        }
        if in_order {
            self.seq = self.seq.max(e.seq as i64);
        }
        if self.o.messages {
            self.out.push(Out::Message(e.clone()));
        }
        let conductor = self.o.role == Role::Conductor;
        let from = e.from.clone();
        match e.message.clone() {
            Message::Welcome(w) => self.welcome(now, w.client, w.snapshot, w.key),
            Message::Pong(p) => self.out.push(Out::Pong { t0: p.t0, t1: p.t1, t2: now }),
            Message::Tempo(t) => self.out.push(Out::Tempo { tempo: t, arrived: now }),
            Message::SceneSync(s) if conductor => self.out.push(Out::Deck(In::Pushed(s))),
            Message::SceneSync(s) => self.out.push(Out::Sync(s)),
            Message::DeckGraph(graph) => self.out.push(Out::Deck(In::Deck { client: from, graph, values: HashMap::new(), mood: None })),
            Message::DeckValues(mut v) => {
                if let Some(q) = self.door.as_mut().filter(|q| q.on_quic) {
                    v.values.retain(|key, _| q.fresh(&from, key, e.seq));
                }
                if !v.values.is_empty() || v.mood.is_some() {
                    self.out.push(Out::Deck(In::Values { client: from, values: v.values, mood: v.mood }));
                }
            }
            Message::HandPose(pose) => {
                if self.fresh_pose(&from, &pose) {
                    self.hand_heard(&from);
                    if let Some(camera) = self.holders.of(&from).filter(|_| self.looks()) {
                        self.out.push(Out::Deck(In::Held(Held::Pose { camera: camera.to_string(), pose: pose.clone() })));
                    }
                    self.out.push(Out::HandPose { client: from, pose });
                }
            }
            Message::HandState(state) => {
                self.hand_heard(&from);
                self.out.push(Out::HandState { client: from, state });
            }
            Message::HandRecord(r) => {
                self.hand_heard(&from);
                self.out.push(Out::HandRecord { client: from, on: r.on });
            }
            Message::HandRecenter(_) => {
                self.hand_heard(&from);
                if let Some(camera) = self.holders.of(&from).filter(|_| self.looks()) {
                    self.out.push(Out::Deck(In::Held(Held::Recenter { camera: camera.to_string() })));
                }
                self.out.push(Out::HandRecenter { client: from });
            }
            Message::HandView(v) => self.out.push(Out::HandView(v)),
            // #478: the roles it may take; a move (the conductor's or its own) is its group – a rejoin says it – and the lock
            Message::Roles(r) => {
                self.roles = r.roles;
                self.conductor = r.conductor.or(self.conductor);
            }
            Message::Assign(a) if a.client == self.client => {
                self.o.group = (!a.group.is_empty()).then_some(a.group);
                self.locked = a.lock == Some(true);
            }
            Message::Rename(r) if r.client == self.client => self.o.nick = (!r.nick.is_empty()).then_some(r.nick),
            Message::DeckListen(l) => self.out.push(Out::Listen { client: from, addresses: l.addresses }),
            Message::DeckEvents(ev) => self.out.push(Out::Events { client: from, events: ev.events }),
            // #517: what the deck's playing scene takes – the core reads it (scene_play.rs), the link only passes it
            // the address tests its filter here (show address.rs `passes`): a cell's camera cut names its column's group
            Message::Address(a) if self.passes(a.r#where.as_ref()) => self.out.push(Out::Deck(In::Address { address: a.address, action: a.action, beat: a.beat })),
            Message::Story(s) => {
                // #527: the conductor keeps the last story heard (the relay passes only the driving reader's)
                if conductor {
                    self.readers.heard(&from, &s);
                }
                self.out.push(Out::Deck(In::Story(s)));
            }
            // #527: who drives the session's story now – a reader's grant or its take-over
            Message::ReaderDrive(d) if self.o.role == Role::Reader => {
                self.steer.heard(&self.client, d.driver);
                self.tell_driving();
            }
            // the cameras this client sees and who holds each – a phone's hold follows from it
            Message::Cameras(c) => {
                if self.o.role == Role::Reader {
                    self.seat.heard(&self.client, c.cameras.clone());
                    self.tell_holding();
                }
                if self.looks() {
                    for h in self.holders.heard(&c.cameras) {
                        self.out.push(Out::Deck(In::Held(h)));
                    }
                }
                self.cameras.clone_from(&c.cameras);
                self.out.push(Out::Cameras(c.cameras));
            }
            Message::CameraFlash(f) => self.out.push(Out::CameraFlash { camera: f.camera, settings: f.settings }),
            Message::DeckSynced(synced) if conductor => self.out.push(Out::Deck(In::Synced { client: from, synced })),
            Message::Presence(p) => {
                // every client but the audience has a link in presence: a phone without one left
                let links = p.links.as_ref();
                let away: Vec<String> = self.hands.iter().filter(|c| !links.is_some_and(|l| l.contains_key(*c))).cloned().collect();
                for client in away {
                    self.hands.remove(&client);
                    self.out.push(Out::HandAway { client });
                }
                if conductor {
                    self.readers.presence(&p);
                }
                let routes = p.routes.unwrap_or_default().into_iter().map(|r| (r.from, r.to)).collect();
                self.out.push(Out::Deck(In::Presence { decks: p.decks.unwrap_or_default(), routes, links: p.links.unwrap_or_default(), quality: p.quality.unwrap_or_default() }));
            }
            Message::VoteOpen(v) => {
                self.ballot.open(&v, now);
                self.vote = Some(v);
                self.out.push(Out::Vote(e));
            }
            Message::VoteResult(r) => {
                self.ballot.result(&r, now);
                self.vote = None;
                // #552 (LIVE.md B.1): a story question's result is the driving reader's choice where it stands
                if let Some(story) = r.story.as_ref().filter(|_| self.o.role == Role::Reader && self.steer.drives()) {
                    let choice = self.place.result(story, r.option);
                    self.out.push(Out::Choice(choice));
                }
                self.out.push(Out::Vote(e));
            }
            Message::VoteTally(t) => {
                self.ballot.tally(&t, now);
                self.out.push(Out::Vote(e));
            }
            Message::Say(_) => self.out.push(Out::Vote(e)),
            Message::Error(err) => {
                self.out.push(Out::Vote(e));
                if err.code == "camera" && self.o.role == Role::Reader {
                    self.seat.refused(&err.message);
                    self.tell_holding();
                }
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

    fn welcome(&mut self, now: f64, client: String, snap: Snapshot, key: Option<String>) {
        self.client.clone_from(&client);
        self.joined = true;
        self.note = None;
        if self.o.resume {
            self.resume = Some(Resume { client: client.clone(), seq: self.seq, key });
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
        // #469: a handheld's state, the editor's view: the other end may have joined since
        if let Some(s) = self.hand_state.clone() {
            self.send(Message::HandState(s));
        }
        if let Some(v) = self.hand_view.clone() {
            self.send(Message::HandView(v));
        }
        // the relay forgot this deck's quality and answer when it left; the decks it asked for far events forget it too
        self.quality_due = self.quality.is_some();
        self.answer_due = self.answer.is_some();
        self.answered = f64::NEG_INFINITY;
        self.listen_sent = f64::NEG_INFINITY;
        self.story_sent = f64::NEG_INFINITY;
        self.flush(now);
        self.flush_listen(now);
        self.flush_story(now);
        // #527: a reader's claim goes out now (again after a drop, unless another reader took the place meanwhile)
        if self.o.role == Role::Reader {
            if self.steer.welcome(&self.client, snap.driver.clone()) {
                self.send(Message::ReaderDrive(protocol::ReaderDrive::default()));
            }
            self.tell_driving();
            // a held camera is taken again by itself (a yes still waiting goes again)
            if let Some(claim) = self.seat.welcome(&self.client) {
                self.send(Message::CameraClaim(claim));
            }
            self.tell_holding();
        }
        if let Some(c) = self.camera_defs.clone() {
            self.send(Message::Cameras(Cameras { cameras: c }));
        }
        if let Some(t) = snap.tempo {
            self.out.push(Out::Tempo { tempo: t, arrived: now });
        }
        if let Some(v) = snap.vote {
            self.ballot.open(&v, now);
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

/// a scripted message as the relay would stamp it
fn stamped(mut e: serde_json::Value) -> serde_json::Value {
    if let Some(o) = e.as_object_mut() {
        o.entry("v").or_insert(1.into());
        o.entry("seq").or_insert(1.into());
        o.entry("from").or_insert("relay".into());
    }
    e
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
        "quic_open" => c.quic_opened(now),
        "quic_close" | "quic_fail" => c.quic_closed(now, &text(&w)),
        "quic_stream" => {
            let e = stamped(w);
            c.quic_stream(now, e.to_string().as_bytes());
            c.quic_stream(now, b"\n");
        }
        "quic_datagram" => {
            let e = stamped(w);
            c.quic_datagram(now, e.to_string().as_bytes());
        }
        "message" => c.text(now, &stamped(w).to_string()),
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
        "vote_cast" => {
            c.vote_cast(w.as_i64().unwrap_or(0));
        }
        "hand_pose" => {
            let n = |k: &str| -> Vec<f64> { w[k].as_array().map(|a| a.iter().filter_map(serde_json::Value::as_f64).collect()).unwrap_or_default() };
            c.hand_pose(w["t"].as_f64().unwrap_or(0.0), &n("p"), &n("q"));
        }
        "hand_state" => c.hand_state(from_value(w).map_err(|e| e.to_string())?),
        "hand_record" => {
            c.hand_record(w.as_bool().unwrap_or(false));
        }
        "hand_recenter" => {
            c.hand_recenter();
        }
        "hand_view" => c.hand_view(from_value(w).map_err(|e| e.to_string())?),
        "story" => {
            c.story(from_value(w).map_err(|e| e.to_string())?);
        }
        "drive" => {
            c.drive();
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
    const QUIC_RELAY: &str = "wss://relay.test:7070/live/CATH?quic=udp.test:443";

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
        json!({ "kind": "welcome", "p": { "client": client, "snapshot": snapshot, "now": 0, "key": format!("key-{client}") } })
    }

    fn graph() -> DeckGraph {
        serde_json::from_value(json!({ "nodes": [{ "id": "voice", "kind": "osc" }], "links": [], "character": "yellowmouse" })).unwrap()
    }

    fn stream() -> Vec<(String, Stream)> {
        vec![("voice.index".into(), Stream::Input([0.0, 8.0])), ("voice.out".into(), Stream::Audio)]
    }

    /// every adapter's test replays these (shared/live-deck/test/fixtures/relay.json)
    pub fn scripts() -> Vec<(&'static str, Script)> {
        let mk = |join: Value, deck: bool, steps: Vec<Step>| Script {
            join: serde_json::from_value(join).unwrap(),
            graph: deck.then(graph),
            stream: if deck { stream() } else { Vec::new() },
            steps,
        };
        let deck = json!({ "relay": RELAY, "role": "musician", "group": DECKS, "nick": "browser" });
        let q = json!({ "level": "medium", "scale": 0.8, "auto": true, "frame": 17.1, "ceiling": "medium" });
        let quic_deck = json!({ "relay": QUIC_RELAY, "role": "musician", "group": DECKS, "nick": "browser" });
        let moved = |x: f64| json!({ "values": { "voice.index": x, "voice.out": 0.5 }, "mood": "neutral" });
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
                        st(0.1, "message", json!({ "seq": 4, "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0, "key": "key-c3" } })),
                        st(0.2, "message", json!({ "seq": 9, "kind": "say", "p": { "text": "Lights" } })),
                        st(1.0, "rejoin", Value::Null),
                        st(1.0, "close", json!("asked to join again")),
                        st(1.3, "open", Value::Null),
                    ],
                ),
            ),
            (
                "quic",
                mk(
                    quic_deck.clone(),
                    true,
                    vec![
                        st(0.05, "quic_open", Value::Null),
                        st(0.1, "quic_stream", welcome("c5", json!({}))),
                        st(0.2, "sample", moved(1.0)),
                        st(0.3, "sample", moved(2.0)),
                        st(0.4, "quic_stream", json!({ "seq": 9, "kind": "cue", "p": { "id": "a", "actions": [] } })),
                        // a cue can't come on the lane that may lose it
                        st(0.5, "quic_datagram", json!({ "seq": 10, "kind": "error", "p": { "code": "x", "message": "no" } })),
                        st(1.4, "tick", Value::Null),
                        st(2.0, "stop", Value::Null),
                    ],
                ),
            ),
            (
                "quic-blocked",
                mk(
                    quic_deck.clone(),
                    false,
                    vec![st(0.3, "open", Value::Null), st(0.4, "message", welcome("c6", json!({}))), st(0.5, "quic_fail", json!("late")), st(1.0, "stop", Value::Null)],
                ),
            ),
            (
                "quic-refused-at-once",
                mk(quic_deck.clone(), false, vec![st(0.01, "quic_fail", json!("no WebTransport here")), st(0.05, "open", Value::Null), st(0.1, "message", welcome("c7", json!({}))), st(0.2, "stop", Value::Null)]),
            ),
            (
                "quic-falls-and-steps-up",
                mk(
                    quic_deck,
                    false,
                    vec![
                        st(0.02, "quic_open", Value::Null),
                        st(0.03, "quic_stream", welcome("c8", json!({}))),
                        st(5.0, "quic_close", json!("timed out")),
                        st(5.3, "open", Value::Null),
                        st(5.35, "message", welcome("c8", json!({}))),
                        st(65.5, "quic_open", Value::Null),
                        st(65.6, "close", Value::Null),
                        st(65.9, "tick", Value::Null),
                        st(66.0, "quic_stream", welcome("c8", json!({}))),
                        st(67.0, "stop", Value::Null),
                    ],
                ),
            ),
            (
                // #469: the phone on QUIC – its state again after the welcome, poses as datagrams, Record on the stream,
                // the editor's view heard
                "handheld",
                mk(
                    json!({ "relay": "wss://relay.test:7070/live/H7K2Q9?quic=udp.test:443", "role": "handheld", "token": "t-hand" }),
                    false,
                    vec![
                        st(0.01, "hand_state", json!({ "mode": "none", "tracking": true, "ua": "S22" })),
                        st(0.02, "hand_pose", json!({ "t": 1.0, "p": [0.0, 1.5, 0.0], "q": [0.0, 0.0, 0.0, 1.0] })),
                        st(0.05, "quic_open", Value::Null),
                        st(0.06, "quic_stream", welcome("c2", json!({}))),
                        st(0.1, "hand_state", json!({ "mode": "xr", "tracking": true, "ua": "S22" })),
                        st(0.2, "hand_pose", json!({ "t": 200.0, "p": [0.1, 1.5, 0.0], "q": [0.0, 0.0, 0.0, 1.0] })),
                        st(0.3, "hand_record", json!(true)),
                        st(0.35, "hand_recenter", Value::Null),
                        st(0.4, "quic_stream", json!({ "from": "c1", "kind": "hand.view", "p": { "shot": "sb-1", "on": true, "recording": true, "seconds": 0.0 } })),
                        st(1.0, "stop", Value::Null),
                    ],
                ),
            ),
            (
                // #469: the editor on the WebSocket – a phone's poses, a late one dropped, its controls; the view kept for a rejoin
                "hand-editor",
                mk(
                    json!({ "relay": "ws://relay.test:7070/live/H7K2Q9", "role": "conductor", "token": "t-editor" }),
                    false,
                    vec![
                        st(0.01, "hand_view", json!({ "shot": "sb-1", "on": true, "recording": false, "seconds": 0.0 })),
                        st(0.05, "open", Value::Null),
                        st(0.1, "message", welcome("c1", json!({}))),
                        st(0.2, "message", json!({ "from": "c2", "kind": "hand.state", "p": { "mode": "xr", "tracking": true } })),
                        st(0.3, "message", json!({ "from": "c2", "kind": "hand.pose", "p": { "t": 300.0, "p": [0.0, 1.5, 0.0], "q": [0.0, 0.0, 0.0, 1.0] } })),
                        st(0.31, "message", json!({ "from": "c2", "kind": "hand.pose", "p": { "t": 280.0, "p": [9.0, 9.0, 9.0], "q": [0.0, 0.0, 0.0, 1.0] } })),
                        st(0.32, "message", json!({ "from": "c2", "kind": "hand.pose", "p": { "t": 316.0, "p": [0.0, 1.6], "q": [0.0, 0.0, 0.0, 1.0] } })),
                        st(0.4, "message", json!({ "from": "c2", "kind": "hand.record", "p": { "on": true } })),
                        st(0.45, "message", json!({ "kind": "presence", "p": { "groups": {}, "ready": [], "links": {} } })),
                        st(0.5, "hand_view", json!({ "shot": "sb-1", "on": true, "recording": true, "seconds": 0.0 })),
                        st(1.0, "stop", Value::Null),
                    ],
                ),
            ),
            (
                // #518: a phone reading the chapter in a session sends what it fires, in order. #527: only while it drives:
                // nothing before the claim, nothing between the claim and the relay's answer, nothing while away – dropped,
                // not held; back from a drop it claims again; another reader's claim takes the place over
                "reader",
                mk(
                    json!({ "relay": RELAY, "role": "reader" }),
                    false,
                    vec![
                        st(0.01, "story", json!({ "type": "label", "params": { "name": "start" }, "id": "e1" })),
                        st(0.02, "drive", Value::Null),
                        st(0.05, "open", Value::Null),
                        st(0.1, "message", welcome("c4", json!({}))),
                        st(0.15, "story", json!({ "type": "joinScene", "params": { "characters": [{ "character": "catpope" }] }, "id": "e2" })),
                        st(0.2, "message", json!({ "kind": "reader.drive", "p": { "driver": "c4" } })),
                        st(0.3, "story", json!({ "type": "showMessage", "params": { "character": "catpope", "text": { "de": "Hallo" } }, "id": "e3" })),
                        st(1.0, "close", json!("the relay went")),
                        st(1.1, "story", json!({ "type": "leaveScene", "params": { "characters": [{ "character": "catpope" }] }, "id": "e4" })),
                        st(1.3, "open", Value::Null),
                        st(1.4, "message", welcome("c5", json!({}))),
                        st(1.5, "message", json!({ "kind": "reader.drive", "p": { "driver": "c5" } })),
                        st(1.6, "story", json!({ "type": "joinScene", "params": { "characters": [{ "character": "catpope" }] }, "id": "e5" })),
                        st(1.7, "message", json!({ "kind": "reader.drive", "p": { "driver": "c9" } })),
                        st(1.8, "story", json!({ "type": "leaveScene", "params": { "characters": [{ "character": "catpope" }] }, "id": "e6" })),
                        st(2.0, "stop", Value::Null),
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

    /// #518: a reader's story events go out as `story` messages, the VNEvent as it is, in the order fired. #527: only
    /// while it drives – the claim goes at the welcome, what fired before the relay's answer, while away or after a
    /// take-over is dropped, never sent later; back from a drop it claims again
    #[test]
    fn a_reader_s_story_goes_out_only_while_it_drives() {
        let t = turns("reader");
        assert_eq!(t[0].outs[1], Out::Dial("ws://relay.test:7070/live/CATH?role=reader".into()));
        let s = sent(&t);
        assert_eq!(s[0]["kind"], "hello");
        assert_eq!(s[0]["p"]["role"], "reader");
        assert_eq!(kinds(&t), ["hello", "reader.drive", "story", "hello", "reader.drive", "story"], "a claim at each welcome");
        let told: Vec<&Value> = s.iter().filter(|e| e["kind"] == "story").map(|e| &e["p"]).collect();
        assert_eq!(told.iter().map(|p| p["id"].as_str().unwrap()).collect::<Vec<_>>(), ["e3", "e5"], "e1 before the claim, e2 before the answer, e4 away, e6 taken over: dropped");
        assert_eq!(*told[0], json!({ "type": "showMessage", "params": { "character": "catpope", "text": { "de": "Hallo" } }, "id": "e3" }), "as fired");
        let at = |what: &str, at: f64| t.iter().find(|x| x.what == what && x.at == at).unwrap();
        let driving = |turn: &Turn| turn.outs.iter().find_map(|o| if let Out::Driving(d) = o { Some(d.claim) } else { None });
        use crate::linked::Claim;
        assert_eq!(driving(at("drive", 0.02)), Some(Claim::Asked));
        assert_eq!(driving(at("message", 0.2)), Some(Claim::Drives));
        assert_eq!(driving(at("message", 1.4)), Some(Claim::Asked), "back from the drop: asks again");
        assert_eq!(driving(at("message", 1.7)), Some(Claim::TakenOver));
        assert!(at("story", 1.8).outs.is_empty(), "taken over: nothing more");
        // a reader that never claims sends nothing, joined or not; only a reader claims
        let mut c = Client::new(Options::new(RELAY, Role::Reader, None)).unwrap();
        c.start(0.0);
        c.opened(1.0);
        c.text(2.0, &stamped(welcome("c4", json!({ "driver": "c2" }))).to_string());
        assert_eq!(c.driving(), Driving { claim: Claim::None, other: true }, "the welcome says another drives: a claim would take over");
        assert!(!c.story(serde_json::from_value(json!({ "type": "label", "id": "e1" })).unwrap()));
        assert!(!c.take().iter().any(|o| matches!(o, Out::Send(t) if t.contains("\"story\"") || t.contains("reader.drive"))));
        let mut deck = Client::new(Options::new(RELAY, Role::Musician, Some(DECKS))).unwrap();
        assert!(!deck.drive());
    }

    /// #527: a conductor keeps the linked readers from presence and the last story event it heard
    #[test]
    fn a_conductor_sees_the_linked_readers() {
        let mut c = Client::new(Options::new(RELAY, Role::Conductor, None)).unwrap();
        c.start(0.0);
        c.opened(1.0);
        c.text(2.0, &stamped(welcome("c1", json!({}))).to_string());
        c.text(3.0, &stamped(json!({ "kind": "presence", "p": { "groups": {}, "ready": [], "readers": { "c4": { "nick": "Mira" } }, "driver": "c4" } })).to_string());
        c.text(4.0, &stamped(json!({ "from": "c4", "kind": "story", "p": { "type": "joinScene", "params": { "characters": [{ "character": "catpope" }] } } })).to_string());
        let r = c.readers();
        assert_eq!((r.linked.get("c4").cloned().flatten().as_deref(), r.driver.as_deref()), (Some("Mira"), Some("c4")));
        let last = r.last.as_ref().unwrap();
        assert_eq!((last.kind.as_str(), last.characters.as_slice(), last.from.as_str()), ("joinScene", &["character:catpope".to_string()][..], "c4"));
    }

    /// #478: a phone that asks its own proof says where, waits for the answer to dial, and its hello carries it
    #[test]
    fn a_phone_asks_its_proof_and_its_hello_carries_it() {
        let mut c = Client::new(Options { key: Some("k".into()), ..Options::new(RELAY, Role::Reader, None) }).unwrap();
        c.ask_proof("https://api.test/api/");
        c.start(0.0);
        let out = c.take();
        let asked: Vec<&str> = out.iter().filter_map(|o| if let Out::AskProof { url } = o { Some(url.as_str()) } else { None }).collect();
        assert_eq!(asked, [format!("https://api.test/api/live/proof?session={}", c.code())]);
        assert!(!out.iter().any(|o| matches!(o, Out::Dial(_))), "the first dial waits for the proof");
        c.proof(10.0, 200, r#"{"proof":"p.s","expires_in":3600}"#);
        c.tick(11.0);
        assert!(c.take().iter().any(|o| matches!(o, Out::Dial(_))));
        c.opened(12.0);
        let hello = c.take().into_iter().find_map(|o| if let Out::Send(t) = o { serde_json::from_str::<Value>(&t).ok() } else { None }).unwrap();
        assert_eq!(hello["p"]["proof"], "p.s");
        c.tick(1_900_000.0);
        assert!(c.take().iter().any(|o| matches!(o, Out::AskProof { .. })), "asked again at half its life");
    }

    /// a deck's Live session says whether its conductor is in, as the relay last told it (roles without the word keep it)
    #[test]
    fn a_deck_knows_whether_its_conductor_is_in() {
        let mut deck = Client::new(Options::new(RELAY, Role::Output, Some("left"))).unwrap();
        deck.start(0.0);
        deck.opened(1.0);
        deck.text(2.0, &stamped(welcome("c2", json!({}))).to_string());
        assert_eq!(deck.live().conductor, None, "not told yet");
        deck.text(3.0, &stamped(json!({ "kind": "roles", "p": { "roles": [], "conductor": true } })).to_string());
        assert_eq!(deck.live().conductor, Some(true));
        deck.text(4.0, &stamped(json!({ "kind": "roles", "p": { "roles": [{ "id": "left" }] } })).to_string());
        assert_eq!(deck.live().conductor, Some(true), "a roles word without it keeps it");
        deck.text(5.0, &stamped(json!({ "kind": "roles", "p": { "roles": [], "conductor": false } })).to_string());
        assert_eq!(deck.live().conductor, Some(false), "the conductor is gone");
    }

    /// a deck hears a camera's holder from the relay's list and that phone's stream as the camera's (`In::Held`); another
    /// phone's pose is no camera's; a reader hears no held word
    #[test]
    fn a_deck_hears_the_phone_holding_a_camera() {
        use crate::live_cameras::Held;
        let held = |c: &mut Client| -> Vec<Held> { c.take().into_iter().filter_map(|o| if let Out::Deck(In::Held(h)) = o { Some(h) } else { None }).collect() };
        let mut deck = Client::new(Options::new(RELAY, Role::Musician, Some(DECKS))).unwrap();
        deck.start(0.0);
        deck.opened(1.0);
        deck.text(2.0, &stamped(welcome("c2", json!({}))).to_string());
        deck.take();
        deck.text(3.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet", "holder": "c4" }] } })).to_string());
        assert_eq!(held(&mut deck), [Held::Holder { camera: "scene:duet".into(), holder: Some("c4".into()) }]);
        let pose = |from: &str, t: f64| stamped(json!({ "kind": "hand.pose", "from": from, "p": { "t": t, "p": [0.0, 1.5, 0.0], "q": [0.0, 0.0, 0.0, 1.0] } })).to_string();
        deck.text(4.0, &pose("c4", 10.0));
        deck.text(5.0, &pose("c9", 10.0));
        deck.text(6.0, &stamped(json!({ "kind": "hand.recenter", "from": "c4", "p": {} })).to_string());
        let h = held(&mut deck);
        assert!(matches!(&h[..], [Held::Pose { camera, pose }, Held::Recenter { .. }] if camera == "scene:duet" && pose.t == 10.0), "{h:?}");
        deck.text(7.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet" }] } })).to_string());
        assert_eq!(held(&mut deck), [Held::Holder { camera: "scene:duet".into(), holder: None }], "back to its owner");
        let mut reader = Client::new(Options { key: Some("k".into()), ..Options::new(RELAY, Role::Reader, None) }).unwrap();
        reader.start(0.0);
        reader.opened(1.0);
        reader.text(2.0, &stamped(welcome("c5", json!({}))).to_string());
        reader.text(3.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet", "holder": "c4" }] } })).to_string());
        assert!(held(&mut reader).is_empty());
    }

    /// a reader phone holds a live camera from the relay's word; back from a drop it claims again by its key (a
    /// resume), a newer claim takes it over; a refusal and a flash reach its page; a conductor's cameras go at each welcome
    #[test]
    fn a_reader_phone_holds_a_camera_and_takes_it_again_after_a_drop() {
        use crate::live_cameras::Hold;
        let claims = |c: &mut Client| -> Vec<Value> {
            c.take().into_iter().filter_map(|o| if let Out::Send(t) = o { serde_json::from_str::<Value>(&t).ok() } else { None }).filter(|e| e["kind"] == "camera.claim").map(|e| e["p"].clone()).collect()
        };
        let holding = |c: &mut Client| c.take().into_iter().filter_map(|o| if let Out::Holding(h) = o { Some(h) } else { None }).next_back();
        let mut no_key = Client::new(Options::new(RELAY, Role::Reader, None)).unwrap();
        assert!(!no_key.camera_claim("scene:duet"), "without its key a phone cannot come back: no claim");
        let mut c = Client::new(Options { key: Some("k1".into()), ..Options::new(RELAY, Role::Reader, None) }).unwrap();
        assert!(c.camera_claim("scene:duet"), "asked before the welcome");
        c.start(0.0);
        c.opened(1.0);
        c.text(2.0, &stamped(welcome("c4", json!({}))).to_string());
        assert_eq!(claims(&mut c), [json!({ "camera": "scene:duet", "key": "k1" })], "the yes goes at the welcome");
        c.text(3.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet", "holder": "c4" }] } })).to_string());
        assert_eq!(c.holding().hold, Hold::Held);
        c.take();
        // a drop: the new welcome claims again with resume, no question
        c.closed(4.0, "gone");
        c.tick(400.0);
        c.opened(500.0);
        c.text(600.0, &stamped(welcome("c9", json!({}))).to_string());
        assert_eq!(claims(&mut c), [json!({ "camera": "scene:duet", "key": "k1", "resume": true })]);
        c.text(7.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet", "holder": "c4", "away": true }] } })).to_string());
        assert_eq!(c.holding().hold, Hold::Held, "its old id: still its own");
        c.text(8.0, &stamped(json!({ "kind": "cameras", "p": { "cameras": [{ "id": "scene:duet", "holder": "c5", "nick": "Ben" }] } })).to_string());
        let h = holding(&mut c).unwrap();
        assert_eq!((h.hold, h.other.as_deref()), (Hold::TakenOver, Some("Ben")), "the warning names who took it");
        // the flash and a refusal
        c.text(9.0, &stamped(json!({ "kind": "camera.flash", "p": { "camera": "scene:duet", "settings": { "label": "Duet" } } })).to_string());
        assert!(c.take().iter().any(|o| matches!(o, Out::CameraFlash { camera, .. } if camera == "scene:duet")));
        c.camera_claim("scene:crew");
        c.text(10.0, &stamped(json!({ "kind": "error", "p": { "code": "camera", "message": "this camera is not open to you" } })).to_string());
        assert_eq!(c.holding().refused.as_deref(), Some("this camera is not open to you"));
        // a conductor's cameras go out at each welcome
        let mut cond = Client::new(Options { token: Some("dev".into()), ..Options::new(RELAY, Role::Conductor, None) }).unwrap();
        cond.set_cameras(vec![LiveCamera { id: "scene:duet".into(), label: None, r#for: None, release_ms: None, holder: None, nick: None, away: None }]);
        cond.start(0.0);
        cond.opened(1.0);
        cond.text(2.0, &stamped(welcome("c1", json!({}))).to_string());
        assert!(cond.take().iter().any(|o| matches!(o, Out::Send(t) if t.contains("\"cameras\""))));
        assert!(cond.camera_force("c4", "scene:duet"));
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
    }

    /// a show cell's camera cut names its column's group: the left deck takes it with its beat, the right one does not
    #[test]
    fn an_address_reaches_only_the_decks_its_filter_takes() {
        let cut = json!({ "kind": "address", "p": { "id": "duo/1", "address": "scene:nave", "action": "shot:close", "where": { "group": "left" }, "beat": 8.0 } });
        let heard = |group: &str| {
            let join = json!({ "relay": RELAY, "role": "output", "group": group, "device": "native" });
            let mut c = Client::new(serde_json::from_value(join).unwrap()).unwrap();
            c.start(0.0);
            let at = |c: &mut Client, s: f64, what: &str, with: Value| step(c, s * 1000.0, &st(s, what, with)).unwrap();
            at(&mut c, 0.0, "open", Value::Null);
            at(&mut c, 0.1, "message", welcome("c3", json!({})));
            at(&mut c, 1.0, "message", cut.clone());
            c.take().into_iter().filter_map(|o| if let Out::Deck(In::Address { action, beat, .. }) = o { Some((action, beat)) } else { None }).collect::<Vec<_>>()
        };
        assert_eq!(heard("left"), [("shot:close".to_string(), Some(8.0))]);
        assert!(heard("right").is_empty(), "the right column's camera is left alone");
    }

    /// account and approval facts are the relay's to check (the server's proof): a deck doesn't drop an address for them
    #[test]
    fn an_address_naming_an_account_still_reaches_the_deck() {
        let cut = json!({ "kind": "address", "p": { "id": "duo/2", "address": "scene:nave", "action": "shot:wide", "where": { "group": "left", "account": ["creator"], "approved": true } } });
        let join = json!({ "relay": RELAY, "role": "output", "group": "left", "device": "native" });
        let mut c = Client::new(serde_json::from_value(join).unwrap()).unwrap();
        c.start(0.0);
        let at = |c: &mut Client, s: f64, what: &str, with: Value| step(c, s * 1000.0, &st(s, what, with)).unwrap();
        at(&mut c, 0.0, "open", Value::Null);
        at(&mut c, 0.1, "message", welcome("c3", json!({})));
        at(&mut c, 1.0, "message", cut);
        assert!(c.take().into_iter().any(|o| matches!(o, Out::Deck(In::Address { .. }))), "the group fits; the account is the relay's to check");
    }

    /// #556: the ballot of a deck that hears the vote follows the relay client – the open vote, this deck's cast once
    /// it went out, the result; a deck that hears only the result (it did not pass the filter, #547) shows nothing
    #[test]
    fn a_decks_ballot_follows_what_the_relay_client_hears_and_casts() {
        let join = json!({ "relay": RELAY, "role": "output", "group": "left", "device": "native" });
        let mut c = Client::new(serde_json::from_value(join.clone()).unwrap()).unwrap();
        c.start(0.0);
        let at = |c: &mut Client, s: f64, what: &str, with: Value| step(c, s * 1000.0, &st(s, what, with)).unwrap();
        at(&mut c, 0.0, "open", Value::Null);
        at(&mut c, 0.1, "message", welcome("c3", json!({})));
        assert_eq!(c.ballot().view(100.0, "en"), None);
        at(&mut c, 1.0, "message", json!({ "kind": "vote.open", "p": { "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20, "where": { "device": "native" } } }));
        let v = c.ballot().view(3000.0, "en").expect("the ballot");
        assert_eq!((v.caption.1.as_str(), v.choices.iter().map(|c| c.mine).collect::<Vec<_>>()), ("18 s left", vec![false, false]));
        at(&mut c, 3.0, "vote_cast", json!(1));
        assert!(c.ballot().view(3000.0, "en").unwrap().choices[1].mine);
        at(&mut c, 4.0, "message", json!({ "kind": "vote.result", "p": { "vote": "v1", "option": 1, "counts": [0, 1], "how": "rule" } }));
        assert_eq!(c.ballot().view(4000.0, "en").and_then(|v| v.result).map(|r| r.winner).as_deref(), Some("No"));
        assert_eq!(c.ballot().view(4000.0 + deck_ui::ballot::RESULT_MS, "en"), None, "the result has shown");
        let cast: Vec<Value> = c.take().into_iter().filter_map(|o| if let Out::Send(s) = o { serde_json::from_str::<Value>(&s).ok() } else { None }).filter(|e| e["kind"] == "vote.cast").collect();
        assert_eq!(cast.iter().map(|e| e["p"].clone()).collect::<Vec<_>>(), [json!({ "vote": "v1", "option": 1 })]);
        // the deck the filter leaves out: only the result reaches it
        let mut other = Client::new(serde_json::from_value(join).unwrap()).unwrap();
        other.start(0.0);
        at(&mut other, 0.0, "open", Value::Null);
        at(&mut other, 0.1, "message", welcome("c4", json!({})));
        at(&mut other, 4.0, "message", json!({ "kind": "vote.result", "p": { "vote": "v1", "option": 1, "counts": [0, 1], "how": "rule" } }));
        assert_eq!(other.ballot().view(4000.0, "en"), None);
        at(&mut other, 4.1, "vote_cast", json!(0));
        assert!(!other.take().iter().any(|o| matches!(o, Out::Send(s) if s.contains("vote.cast"))), "nothing to cast into");
        // offline, a cast does not go out and is not marked
        let mut off = Client::new(serde_json::from_value(json!({ "relay": RELAY, "role": "output", "group": "left" })).unwrap()).unwrap();
        off.start(0.0);
        at(&mut off, 0.0, "open", Value::Null);
        at(&mut off, 0.1, "message", welcome("c5", json!({ "vote": { "vote": "v1", "options": ["Yes", "No"], "rule": "majority", "seconds": 20 } })));
        at(&mut off, 0.2, "close", json!("gone"));
        at(&mut off, 0.3, "vote_cast", json!(0));
        assert!(off.ballot().view(300.0, "en").is_some_and(|v| v.choices.iter().all(|c| !c.mine)));
    }

    #[test]
    fn a_refused_token_closes_and_backs_off() {
        let t = turns("refused");
        let outs: Vec<&Out> = t.iter().flat_map(|x| &x.outs).collect();
        assert!(outs.contains(&&Out::Status(Status { state: State::Refused, client: None, why: Some("bad signature".into()), wait: None })));
        assert!(outs.contains(&&Out::Close {}));
        assert_eq!(lines_of(&scripts()[5].1)[1], "RELAY token: bad signature");
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
        assert_eq!(hellos[1]["p"]["resume"], json!({ "client": "c3", "seq": 9, "key": "key-c3" }));
        assert_eq!(t.iter().flat_map(|x| &x.outs).filter(|o| matches!(o, Out::Message(_))).count(), 2, "every envelope for the composer");
        let mut c = Client::new(scripts()[7].1.join.clone()).unwrap();
        c.start(0.0);
        c.opened(0.0);
        c.text(0.0, &json!({ "v": 1, "seq": 1, "from": "relay", "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0 } }).to_string());
        assert_eq!(c.line(), "OUTPUT c3 OF left @ CATH");
    }

    /// #478: a proof that comes after the hello joins again with it (resume); a renewed one waits for the next hello
    #[test]
    fn a_proof_after_the_hello_joins_again_with_it() {
        let sent = |outs: Vec<Out>| -> Vec<Value> { outs.into_iter().filter_map(|o| if let Out::Send(t) = o { serde_json::from_str(&t).ok() } else { None }).collect() };
        let mut o = Options::new("ws://relay/live/CATH", Role::Output, Some("tenor"));
        o.resume = true;
        let mut c = Client::new(o).unwrap();
        c.start(0.0);
        c.opened(0.0);
        assert_eq!(sent(c.take())[0]["p"].get("proof"), None, "joined as a guest");
        c.text(0.0, &json!({ "v": 1, "seq": 1, "from": "relay", "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0, "key": "k" } }).to_string());
        c.take();
        c.set_proof(Some("p.s".into()));
        assert!(c.take().iter().any(|o| matches!(o, Out::Close {})), "the socket goes: join again");
        c.closed(1.0, "");
        c.tick(60_000.0);
        c.opened(60_000.0);
        let hello = sent(c.take()).into_iter().find(|e| e["kind"] == "hello").unwrap();
        assert_eq!((hello["p"]["proof"].as_str(), hello["p"]["resume"]["client"].as_str()), (Some("p.s"), Some("c3")));
        c.set_proof(Some("p.t".into()));
        assert!(!c.take().iter().any(|o| matches!(o, Out::Close {})), "a renewed proof waits for the next hello");
    }

    /// #478: a client that asks for its proof dials when the answer is there – its first hello carries it and the role it
    /// names – and not later than PROOF_WAIT_MS without one; an answer that says nothing lets it go too
    #[test]
    fn the_first_hello_waits_for_the_proof_not_longer_than_its_time() {
        let sent = |outs: &[Out]| -> Vec<Value> { outs.iter().filter_map(|o| if let Out::Send(t) = o { serde_json::from_str(t).ok() } else { None }).collect() };
        let dials = |outs: &[Out]| outs.iter().filter(|o| matches!(o, Out::Dial(_))).count();
        let waiting = || {
            let mut o = Options::new("ws://relay/live/CATH", Role::Output, Some("soprano"));
            o.resume = true;
            o.wait_proof = true;
            let mut c = Client::new(o).unwrap();
            c.start(1000.0);
            assert_eq!((dials(&c.take()), c.next_wake()), (0, Some(1000.0 + PROOF_WAIT_MS)), "no dial yet: it waits for the answer");
            c
        };
        let welcome = json!({ "v": 1, "seq": 1, "from": "relay", "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0, "key": "k" } }).to_string();
        // the answer in time: the dial at once, the one hello with the proof and the role
        let mut c = waiting();
        c.tick(1200.0);
        assert_eq!(dials(&c.take()), 0);
        c.proof_answered(Some(Some("p.s".into())));
        assert!(c.next_wake().is_some_and(|at| at <= 1300.0), "due now");
        c.tick(1300.0);
        assert_eq!(dials(&c.take()), 1);
        c.opened(1310.0);
        let hellos = sent(&c.take());
        assert_eq!((hellos.len(), hellos[0]["p"]["proof"].as_str(), hellos[0]["p"]["group"].as_str()), (1, Some("p.s"), Some("soprano")));
        c.text(1320.0, &welcome);
        assert!(!c.take().iter().any(|o| matches!(o, Out::Close {})), "joined with its facts: no join again");

        // nobody signed in (401: a guest) or no answer worth keeping: it dials at once, as a guest
        for answer in [Some(None), None] {
            let mut c = waiting();
            c.proof_answered(answer.clone());
            c.tick(1050.0);
            assert_eq!(dials(&c.take()), 1, "{answer:?}");
            c.opened(1060.0);
            assert_eq!(sent(&c.take())[0]["p"].get("proof"), None);
        }

        // no answer: the dial at PROOF_WAIT_MS, a guest's hello; the proof that comes later joins again with resume, the
        // role it names and the proof in one hello (the relay gives the role with the proven facts: deck-relay
        // `a_resume_with_a_proof_takes_the_role_a_guest_was_refused`)
        let mut c = waiting();
        c.tick(1000.0 + PROOF_WAIT_MS - 1.0);
        assert_eq!(dials(&c.take()), 0);
        c.tick(1000.0 + PROOF_WAIT_MS);
        assert_eq!(dials(&c.take()), 1, "not longer than PROOF_WAIT_MS");
        c.opened(2600.0);
        assert_eq!(sent(&c.take())[0]["p"].get("proof"), None, "a guest");
        c.text(2610.0, &welcome);
        c.take();
        c.proof_answered(Some(Some("p.s".into())));
        assert!(c.take().iter().any(|o| matches!(o, Out::Close {})), "a late proof: join again");
        c.closed(3000.0, "");
        c.tick(3000.0 + REJOIN_MS);
        c.opened(3300.0);
        let hello = sent(&c.take()).into_iter().find(|e| e["kind"] == "hello").unwrap();
        assert_eq!(hello["p"]["group"], "soprano");
        assert_eq!((hello["p"]["proof"].as_str(), hello["p"]["resume"]["client"].as_str()), (Some("p.s"), Some("c3")));

        // a client that asks for no proof dials at once, as ever
        let mut c = Client::new(Options::new("ws://relay/live/CATH", Role::Output, None)).unwrap();
        c.start(0.0);
        assert_eq!(dials(&c.take()), 1);
    }

    /// #442: a deck asks for its far addresses once joined, to the decks' group, and again every `far::LISTEN_MS`; its
    /// events go to one subscriber; other decks' asks and events come out typed
    #[test]
    fn far_asks_go_out_on_the_join_and_events_come_in_typed() {
        let sent = |outs: Vec<Out>| -> Vec<Value> { outs.into_iter().filter_map(|o| if let Out::Send(t) = o { serde_json::from_str(&t).ok() } else { None }).collect() };
        let address = "/deck:left/scene:nave/character:catpope/analog:seq.notes";
        let mut c = Client::new(Options::new("ws://h:7700/live/CATH", Role::Musician, Some(DECKS))).unwrap();
        c.start(0.0);
        c.opened(0.0);
        c.set_listen(0.0, vec![address.to_string()]);
        assert!(sent(c.take()).iter().all(|e| e["kind"] != "deck.listen"), "nothing before the welcome");
        c.text(0.0, &json!({ "v": 1, "seq": 1, "from": "relay", "kind": "welcome", "p": { "client": "c3", "snapshot": {}, "now": 0 } }).to_string());
        let listens: Vec<Value> = sent(c.take()).into_iter().filter(|e| e["kind"] == "deck.listen").collect();
        assert_eq!(listens.len(), 1);
        assert_eq!(listens[0]["to"], json!({ "group": "decks" }));
        assert_eq!(listens[0]["p"], json!({ "addresses": [address] }));
        c.tick(crate::far::LISTEN_MS - 1.0);
        assert!(sent(c.take()).iter().all(|e| e["kind"] != "deck.listen"));
        c.tick(crate::far::LISTEN_MS);
        assert_eq!(sent(c.take()).iter().filter(|e| e["kind"] == "deck.listen").count(), 1, "asked again");
        // this deck's events for one subscriber
        let ev = DeckEvent { address: address.into(), name: "music.note".into(), props: [("pitch".to_string(), 440.0)].into_iter().collect(), dur_s: Some(0.25) };
        assert!(c.events("c9", vec![ev.clone()]));
        let s = sent(c.take());
        assert_eq!(s.len(), 1);
        assert_eq!((s[0]["kind"].as_str(), &s[0]["to"]), (Some("deck.events"), &json!({ "client": "c9" })));
        assert_eq!(s[0]["p"]["events"][0]["props"]["pitch"], 440.0);
        assert!(!c.events("c9", vec![]), "nothing to send");
        // another deck's ask, and events for this deck, typed
        c.text(1.0, &json!({ "v": 1, "seq": 2, "from": "c8", "kind": "deck.listen", "to": { "group": "decks" }, "p": { "addresses": ["/deck:me/scene:x/character:boy/voice:voice.cv"] } }).to_string());
        c.text(1.0, &json!({ "v": 1, "seq": 3, "from": "c8", "kind": "deck.events", "to": { "client": "c3" }, "p": { "events": [ev] } }).to_string());
        let outs = c.take();
        assert!(matches!(&outs[0], Out::Listen { client, addresses } if client == "c8" && addresses == &["/deck:me/scene:x/character:boy/voice:voice.cv".to_string()]), "{outs:?}");
        assert!(matches!(&outs[1], Out::Events { client, events } if client == "c8" && events.len() == 1 && events[0].name == "music.note"), "{outs:?}");
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

    fn sent_on_stream(t: &[Turn]) -> Vec<Value> {
        t.iter().flat_map(|t| &t.outs).filter_map(|o| if let Out::Stream(s) = o { serde_json::from_str(s.trim_end()).ok() } else { None }).collect()
    }

    fn all_outs(t: &[Turn]) -> Vec<&Out> {
        t.iter().flat_map(|t| &t.outs).collect()
    }

    #[test]
    fn the_quic_door_comes_from_the_relay_url() {
        assert_eq!(parse_quic("ws://h:7700/live/A"), None);
        assert_eq!(parse_quic("wss://10-0-0-1.lan.scutoons.com:7700/live/A"), Some(("https://10-0-0-1.lan.scutoons.com:7700".into(), None)), "wss under a name: the door by default");
        assert_eq!(parse_quic("wss://h.test:7700/live/A?role=x"), Some(("https://h.test:7700".into(), None)));
        assert_eq!(parse_quic("wss://h.test:7700/live/A?quic=off"), None, "unless told otherwise");
        assert_eq!(parse_quic("wss://live.scutoons.com/live/A"), Some(("https://live.scutoons.com".into(), None)), "the hosted relay answers both on one name (#441)");
        assert_eq!(parse_quic("wss://192.168.1.116:4100/relay/live/A"), None, "by IP: no public certificate");
        assert_eq!(parse_quic("wss://[::1]:7700/live/A"), None);
        assert_eq!(parse_quic("wss://live.test/live/A?quic=udp.test:443"), Some(("https://udp.test:443".into(), None)));
        assert_eq!(parse_quic("wss://h:7700/live/A?quic="), Some(("https://h:7700".into(), None)), "empty: the relay's own host and port");
        assert_eq!(parse_quic("ws://h:7700/live/A?quic=h%3A7701&quichash=ab12"), Some(("https://h:7701".into(), Some(vec![0xab, 0x12]))));
        assert_eq!(parse_quic("ws://h/live/A?quic=&quichash=xyz"), Some(("https://h".into(), None)), "not hex: no pin");
        assert_eq!(parse_relay("wss://h/live/A?quic=x:1").unwrap().1, "A", "the code is untouched by it");
    }

    #[test]
    fn quic_gets_a_head_start_and_says_hello_alone() {
        let t = turns("quic");
        assert_eq!(t[0].outs[1], Out::DialQuic { url: "https://udp.test:443/live/CATH?role=musician&group=decks".into(), hash: None });
        assert!(!all_outs(&t).iter().any(|o| matches!(o, Out::Dial(_) | Out::Send(_))), "up inside the head start: no WebSocket at all");
        assert_eq!(sent_on_stream(&t)[0]["kind"], "hello");
    }

    #[test]
    fn values_go_as_datagrams_and_settle_on_the_stream() {
        let t = turns("quic");
        let grams: Vec<&String> = all_outs(&t).into_iter().filter_map(|o| if let Out::Datagram(d) = o { Some(d) } else { None }).collect();
        assert_eq!(grams.len(), 2);
        let seqs: Vec<u64> = grams.iter().map(|g| serde_json::from_str::<Envelope>(g).unwrap().seq).collect();
        assert_eq!(seqs, [1, 2], "numbered: the door drops one that was overtaken");
        let settles: Vec<Value> = sent_on_stream(&t).into_iter().filter(|e| e["kind"] == "deck.values").collect();
        assert_eq!(settles.len(), 1, "one settle in the 1.4 s after the first datagram");
        assert_eq!(settles[0]["p"]["values"]["voice.index"], json!([2.0]));
        assert_eq!(settles[0]["seq"], 2, "numbered by the datagram it follows");
    }

    #[test]
    fn a_due_settle_takes_the_place_of_the_next_change() {
        let mut c = Client::new(serde_json::from_value(json!({ "relay": QUIC_RELAY, "role": "musician", "group": DECKS })).unwrap()).unwrap();
        c.set_graph(graph());
        c.start(0.0);
        c.quic_opened(0.0);
        c.quic_stream(1.0, (stamped(welcome("c1", json!({}))).to_string() + "\n").as_bytes());
        let change = |x: f64, k: &str| DeckValues { values: HashMap::from([(k.to_string(), vec![x])]), mood: None };
        c.values(change(1.0, "a"));
        c.values(change(2.0, "b"));
        c.take();
        c.tick(SETTLE_MS);
        assert_eq!(c.take(), [], "first a frame's grace");
        c.values(change(3.0, "a"));
        let outs = c.take();
        assert_eq!(outs.len(), 1, "one message, not a datagram and a settle: {outs:?}");
        let Out::Stream(full) = &outs[0] else { panic!("{outs:?}") };
        let e: Value = serde_json::from_str(full.trim_end()).unwrap();
        assert_eq!(e["p"]["values"], json!({ "a": [3.0], "b": [2.0] }));
        c.values(change(4.0, "a"));
        assert!(matches!(c.take()[..], [Out::Datagram(_)]), "and back to datagrams");
    }

    #[test]
    fn a_blocked_quic_lets_the_websocket_win() {
        let t = turns("quic-blocked");
        let dials: Vec<&Out> = all_outs(&t).into_iter().filter(|o| matches!(o, Out::Dial(_))).collect();
        assert_eq!(dials.len(), 1);
        let turn = t.iter().find(|t| t.outs.iter().any(|o| matches!(o, Out::Dial(_)))).unwrap();
        assert_eq!(turn.at, 0.25, "after the head start");
        assert!(all_outs(&t).contains(&&Out::CloseQuic {}), "the slower one is closed at once");
        let wake = t.iter().find(|t| t.what == "open").unwrap();
        assert!(wake.outs.iter().any(|o| matches!(o, Out::Send(h) if h.contains("hello"))));
    }

    #[test]
    fn no_webtransport_dials_the_websocket_at_once() {
        let t = turns("quic-refused-at-once");
        let turn = t.iter().find(|t| t.what == "quic_fail").unwrap();
        assert!(turn.outs.iter().any(|o| matches!(o, Out::Dial(_))), "no reason to wait out the head start");
    }

    #[test]
    fn a_dead_quic_falls_to_the_websocket_with_a_resume_and_steps_up_later() {
        let t = turns("quic-falls-and-steps-up");
        let at = |what: &str, when: f64| t.iter().find(|x| x.what == what && (x.at - when).abs() < 1e-6).unwrap();
        assert!(at("quic_close", 5.0).outs.iter().any(|o| matches!(o, Out::Status(_))));
        let dialed = t.iter().find(|x| x.at > 5.0 && x.outs.iter().any(|o| matches!(o, Out::Dial(_) | Out::DialQuic { .. }))).unwrap();
        assert!(dialed.outs.iter().any(|o| matches!(o, Out::Dial(_))), "the WebSocket, not QUIC again");
        assert!((dialed.at - 5.25).abs() < 1e-6, "after REJOIN_MS");
        let hello = at("open", 5.3).outs.iter().find_map(|o| if let Out::Send(h) = o { Some(h.clone()) } else { None }).unwrap();
        assert!(hello.contains(r#""resume":{"client":"c8","key":"key-c8""#), "{hello}");

        let probe = t.iter().find(|x| x.at > 60.0 && x.outs.iter().any(|o| matches!(o, Out::DialQuic { .. }))).unwrap();
        assert!((probe.at - 65.3).abs() < 1e-6, "STEP_UP_MS after joining on the WebSocket, at {}", probe.at);
        assert!(at("quic_open", 65.5).outs.contains(&Out::Close {}), "the WebSocket goes");
        assert!(at("quic_open", 65.5).outs.iter().any(|o| matches!(o, Out::Stream(h) if h.contains(r#""resume":{"client":"c8","key":"key-c8""#))), "and the resume at once: the relay gives it the old link's place");
    }

    #[test]
    fn a_datagram_can_be_overtaken_key_by_key_and_never_moves_the_resume() {
        let mut c = Client::new(serde_json::from_value(json!({ "relay": QUIC_RELAY, "role": "conductor" })).unwrap()).unwrap();
        c.start(0.0);
        c.quic_opened(1.0);
        let frame = |e: Value| e.to_string() + "\n";
        c.quic_stream(2.0, frame(stamped(welcome("c1", json!({})))).as_bytes());
        c.take();
        let values = |seq: u64, k: &str, x: f64| json!({ "v": 1, "seq": seq, "from": "d1", "kind": "deck.values", "p": { "values": { k: [x] } } }).to_string();
        c.quic_datagram(3.0, values(50, "a", 1.0).as_bytes());
        c.quic_datagram(3.0, values(40, "a", 0.5).as_bytes()); // overtaken
        c.quic_stream(3.0, frame(serde_json::from_str(&values(45, "b", 2.0)).unwrap()).as_bytes());
        let heard: Vec<(String, f64)> = c
            .take()
            .into_iter()
            .filter_map(|o| if let Out::Deck(In::Values { values, .. }) = o { values.into_iter().next().map(|(k, v)| (k, v[0])) } else { None })
            .collect();
        assert_eq!(heard, [("a".to_string(), 1.0), ("b".to_string(), 2.0)]);
        c.quic_closed(4.0, "gone");
        c.tick(4.0 + REJOIN_MS);
        c.opened(4.5 + REJOIN_MS);
        let hello = c.take().into_iter().find_map(|o| if let Out::Send(h) = o { Some(h) } else { None }).unwrap();
        assert!(hello.contains(r#""seq":45"#), "the stream's newest, not the datagram's 50: {hello}");
    }

    #[test]
    fn a_silent_udp_path_ends_the_wait_when_the_websocket_fails_too() {
        let mut c = Client::new(serde_json::from_value(json!({ "relay": QUIC_RELAY, "role": "audience" })).unwrap()).unwrap();
        c.start(0.0);
        c.tick(QUIC_HEAD_START_MS);
        c.closed(300.0, "refused");
        c.take();
        assert_eq!(c.next_wake(), Some(QUIC_GIVE_UP_MS));
        c.tick(QUIC_GIVE_UP_MS);
        let outs = c.take();
        assert!(outs.contains(&Out::CloseQuic {}));
        assert_eq!(c.status().state, State::Offline);
    }

    #[test]
    fn a_handheld_sends_poses_as_datagrams_and_controls_on_the_stream() {
        let t = turns("handheld");
        let grams: Vec<Value> = all_outs(&t).into_iter().filter_map(|o| if let Out::Datagram(d) = o { serde_json::from_str(d).ok() } else { None }).collect();
        assert_eq!(grams.len(), 1, "the pose before the welcome is dropped, the one after is a datagram: {grams:?}");
        assert_eq!((grams[0]["kind"].as_str(), grams[0]["p"]["t"].as_f64(), grams[0]["seq"].as_u64()), (Some("hand.pose"), Some(200.0), Some(0)), "not numbered like values");
        let kinds: Vec<String> = sent_on_stream(&t).iter().map(|e| e["kind"].as_str().unwrap_or("").to_string()).collect();
        assert_eq!(kinds, ["hello", "hand.state", "hand.state", "hand.record", "hand.recenter"], "the state kept for the welcome, then the new one");
        assert!(sent_on_stream(&t)[0]["p"]["token"] == "t-hand" && sent_on_stream(&t)[1]["p"]["mode"] == "none");
        assert!(all_outs(&t).iter().any(|o| matches!(o, Out::HandView(v) if v.recording && v.shot.as_deref() == Some("sb-1"))), "the editor's view heard");
        assert!(!all_outs(&t).iter().any(|o| matches!(o, Out::Stream(s) if s.contains("deck.values"))), "no settle for poses");
        let mut c = Client::new(serde_json::from_value(json!({ "relay": "wss://h.test:7070/live/H?quic=udp.test:443", "role": "handheld", "token": "t" })).unwrap()).unwrap();
        assert_eq!(c.rung(), None);
        c.start(0.0);
        c.quic_opened(1.0);
        c.quic_stream(2.0, (stamped(welcome("c2", json!({}))).to_string() + "\n").as_bytes());
        assert_eq!((c.rung(), c.line().as_str()), (Some("QUIC"), "HANDHELD c2 @ H QUIC"));
    }

    #[test]
    fn the_editor_hears_a_phones_newest_pose_only() {
        let t = turns("hand-editor");
        let poses: Vec<f64> = all_outs(&t).into_iter().filter_map(|o| if let Out::HandPose { pose, .. } = o { Some(pose.t) } else { None }).collect();
        assert_eq!(poses, [300.0], "the late one (280) and the malformed one (2 numbers) never come out");
        let heard: Vec<&str> = all_outs(&t)
            .into_iter()
            .filter_map(|o| match o {
                Out::HandState { client, .. } if client == "c2" => Some("state"),
                Out::HandPose { client, .. } if client == "c2" => Some("pose"),
                Out::HandRecord { client, on: true } if client == "c2" => Some("record"),
                _ => None,
            })
            .collect();
        assert_eq!(heard, ["state", "pose", "record"]);
        assert!(all_outs(&t).contains(&&Out::HandAway { client: "c2".into() }), "the presence without the phone: it left");
        assert_eq!(kinds(&t), ["hello", "hand.view", "hand.view", "hand.view"], "the view kept for the welcome, again to the phone that joined after it, then the new one");
        assert_eq!(sent(&t)[2]["to"], json!({ "client": "c2" }));
        let mut c = Client::new(serde_json::from_value(json!({ "relay": "ws://h:7070/live/H", "role": "conductor" })).unwrap()).unwrap();
        c.start(0.0);
        c.opened(1.0);
        c.text(2.0, &stamped(welcome("c1", json!({}))).to_string());
        assert_eq!(c.rung(), Some("WebSocket"));
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
