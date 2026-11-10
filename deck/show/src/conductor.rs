//! The bare conductor (#89, SHOW.md §5, §7): one show, in memory, driven by calls. The adapter passes the time in and moves
//! the bytes; every decision is here (RULES A1). No relay tree yet (#90): what a row says to a seat comes back as [`Out::Seat`].
//!
//! You write only what changes; the conductor resolves `keep` and `values` along the path really taken and sends a **full
//! cell**, so a deck that reboots mid-show is right at once from the snapshot. GO and the hand are always allowed.
use crate::document::{Cell, How, Moment, OutletKind, Show, Values, Word};
use deck_core::tempo::Quantize;
use deck_space::rig::address::Place;
use protocol::{CellChange, CellMode, Device, DeviceMsg, Envelope, Layer, LayerSrc, Message, PieceSrc, Role, To};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};

/// how many bars a phrase has
pub const BARS_PER_PHRASE: u32 = 4;
/// the log keeps this many entries; the oldest go (a show must not grow without bound, RULES A10)
const LOG_CAP: usize = 4096;
/// the layer's id of a piece in a full cell
const PIECE_LAYER: &str = "piece";

/// The time, passed in by the adapter on every call.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Now {
    /// relay time in ms
    pub ms: u64,
    /// the show clock, if one runs; without it a row's moment is now
    pub clock: Option<Clock>,
}

impl Now {
    pub fn at(ms: u64) -> Now {
        Now { ms, clock: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    pub bpm: f64,
    pub beat: f64,
    pub per_bar: u32,
}

/// Who fired a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Auto,
    Hand,
    Go,
    Parent,
}

/// What a row says to its child conductor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Says {
    Go(String),
    Free,
}

/// What the conductor sends.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// over the relay
    Send(Box<Envelope>),
    /// to a seat's child conductor; the relay tree carries it later (#90)
    Seat { seat: String, word: Word, says: Says, at: Option<i64> },
}

impl Out {
    /// what goes over the relay, if this does
    pub fn envelope(&self) -> Option<&Envelope> {
        match self {
            Out::Send(e) => Some(e),
            Out::Seat { .. } => None,
        }
    }
}

/// A row waiting for the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub row: String,
    /// the event that raised it; none = the parent's row
    pub event: Option<String>,
}

/// One launch: the row it belongs to, and when. A cell launched by hand off its row is marked.
#[derive(Clone, Copy)]
struct Launch<'a> {
    row: &'a str,
    off_row: bool,
    at: Option<i64>,
    ms: u64,
}

/// An event standing for its hold time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Standing {
    row: String,
    event: String,
    how: How,
    until_ms: u64,
}

/// What an outlet is playing.
#[derive(Debug, Clone, PartialEq)]
pub enum Playing {
    Piece { piece: String, values: Values },
    Mirror { of: String },
    Black,
    /// a device outlet that was sent its messages
    Sent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnStage {
    pub playing: Playing,
    /// the row it counts as (a kept cell counts as the row that kept it)
    pub row: String,
    /// launched by hand off its row
    pub off_row: bool,
}

/// One line of the show's log (SHOW.md §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Logged {
    Fired { row: String, origin: Origin },
    /// held as a suggestion
    Suggested { row: String },
    /// the reader moved on, the suggestion or the standing event is gone
    Withdrawn { row: String },
    /// the row is on `hand`: the event does nothing
    Ignored { row: String, event: String },
    /// no row listens to this event
    NoRow { event: String },
    /// a single cell, by hand
    CellLaunched { row: String, column: String, off_row: bool },
    /// a `values` cell met another piece, or nothing
    ValuesSkipped { row: String, outlet: String, expected: String, found: Option<String> },
    /// GO with no next row
    NoNextRow,
    /// the parent said something the word or the child's rows drop
    ParentIgnored { row: Option<String> },
    /// an address and how many it reached; 0 = it stayed silent
    Addressed { address: String, action: String, reached: usize },
    /// #90: an address down the tree, once the answers are in: 0 reached = "nobody"
    Reached { address: String, action: String, reached: u64, passed: u64, answered: u64 },
    /// #90: a lock from above came / went
    Locked,
    Unlocked,
    /// #90: the operator tried to change the word under a lock from above
    WordRefused,
    /// #90: this seat cannot be locked
    LockRefused { seat: String },
    /// #90: a child sent what the seat does not use of its face
    NotUsed { seat: String, what: String },
    /// #90: this subtree has its own clock, the parent's anchor is ignored
    ClockKept,
    /// #90: a subtree on its parent's time does not lead the clock
    ClockRefused,
    /// #90: the parent's relay went away; what plays keeps playing
    ParentLost,
    ParentBack,
    /// #90: a flash taken, and through how many conductors it came
    Flashed { root: String, hops: u32 },
    FlashRefused { why: String },
    /// #90: the show a flash asked for runs now
    Installed,
    /// #90: a deck's or child's report on a flash
    Reported { from: String, running: String, refused: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub ms: u64,
    pub what: Logged,
}

/// One conductor over one show.
#[derive(Debug, Clone)]
pub struct Conductor {
    show: Show,
    stage: BTreeMap<String, OnStage>,
    /// the last cell sent per outlet: a mirror only sends when it would show something else
    sent: BTreeMap<String, protocol::Cell>,
    current: Option<String>,
    standing: Option<Standing>,
    suggestion: Option<Suggestion>,
    log: VecDeque<Entry>,
}

fn full_cell(piece: &str, values: &Values) -> protocol::Cell {
    let src = LayerSrc::Piece(PieceSrc { piece: piece.to_string(), values: (!values.is_empty()).then(|| values.clone()) });
    let layer = Layer { id: PIECE_LAYER.into(), src, r#box: None, fit: None, opacity: None, r#loop: None, enter: None, exit: None };
    protocol::Cell { mode: CellMode::Own, layers: Some(vec![layer]), of: None, send: None }
}

fn black_cell() -> protocol::Cell {
    protocol::Cell { mode: CellMode::Black, layers: None, of: None, send: None }
}

fn envelope(message: Message, at: Option<i64>) -> Out {
    let mut e = Envelope::new("", message);
    e.at = at;
    Out::Send(Box::new(e))
}

/// when a moment falls (relay ms); None = at once
fn moment_at(moment: Moment, now: Now) -> Option<i64> {
    let clock = now.clock?;
    let per_bar = clock.per_bar.max(1);
    let next = match moment {
        Moment::Now => return None,
        Moment::Bar => Quantize::Bar.next(clock.beat, per_bar),
        Moment::Phrase => Quantize::Bar.next(clock.beat, per_bar * BARS_PER_PHRASE),
    };
    let ms = (next - clock.beat) * 60_000.0 / clock.bpm.max(1.0);
    Some(now.ms as i64 + ms.round() as i64)
}

impl Conductor {
    pub fn new(show: Show) -> Conductor {
        Conductor { show, stage: BTreeMap::new(), sent: BTreeMap::new(), current: None, standing: None, suggestion: None, log: VecDeque::new() }
    }

    pub fn show(&self) -> &Show {
        &self.show
    }

    /// the row that fired last
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn on_stage(&self, outlet: &str) -> Option<&OnStage> {
        self.stage.get(outlet)
    }

    pub fn suggestion(&self) -> Option<&Suggestion> {
        self.suggestion.as_ref()
    }

    pub fn log(&self) -> impl Iterator<Item = &Entry> {
        self.log.iter()
    }

    /// One line in the log, from the tree around this conductor (subtree.rs).
    pub fn note(&mut self, ms: u64, what: Logged) {
        self.record(ms, what);
    }

    /// A new version of the show (a flash, subtree.rs). What is on stage stays until a row changes it.
    pub fn replace(&mut self, show: Show) {
        self.show = show;
    }

    /// A parent's cell for an outlet of this show's face: it plays as it came, a full cell.
    pub fn take_cell(&mut self, outlet: &str, cell: protocol::Cell, at: Option<i64>, ms: u64) -> Vec<Out> {
        let playing = match cell.layers.as_deref().and_then(|l| l.first()).map(|l| &l.src) {
            Some(LayerSrc::Piece(p)) => Playing::Piece { piece: p.piece.clone(), values: p.values.clone().unwrap_or_default() },
            _ if cell.mode == CellMode::Black => Playing::Black,
            _ => {
                self.record(ms, Logged::ParentIgnored { row: None });
                return vec![];
            }
        };
        let row = self.current.clone().unwrap_or_default();
        self.stage.insert(outlet.to_string(), OnStage { playing, row, off_row: true });
        let mut out = vec![];
        self.send_cell(outlet, cell, at, &mut out);
        self.update_mirrors(at, &mut out);
        out
    }

    fn record(&mut self, ms: u64, what: Logged) {
        if self.log.len() == LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(Entry { ms, what });
    }

    /// A message from the relay. `story` is a reader's event: its `id` is the contract with the show, `left` = the reader
    /// scrolled back past it. Everything else is not the conductor's.
    pub fn on_message(&mut self, message: &Message, now: Now) -> Vec<Out> {
        let Message::Story(story) = message else { return vec![] };
        let Some(id) = story.get("id").and_then(|v| v.as_str()) else { return vec![] };
        if story.get("left").and_then(|v| v.as_bool()) == Some(true) {
            self.on_left(id, now);
            vec![]
        } else {
            self.on_event(id, now)
        }
    }

    /// A reader's event: it fires its row on auto, is held as a suggestion on suggest, does nothing on hand.
    pub fn on_event(&mut self, event: &str, now: Now) -> Vec<Out> {
        let Some(row) = self.show.rows.iter().find(|r| r.fire.event.as_deref() == Some(event)) else {
            self.record(now.ms, Logged::NoRow { event: event.to_string() });
            return vec![];
        };
        let (id, how, hold) = (row.id.clone(), row.fire.how, row.fire.hold_ms);
        if how == How::Hand {
            self.record(now.ms, Logged::Ignored { row: id, event: event.to_string() });
            return vec![];
        }
        // a newer event replaces the one still standing; one standing for the same event keeps its time
        let until_ms = match &self.standing {
            Some(s) if s.event == event => s.until_ms,
            _ => now.ms + hold as u64,
        };
        self.standing = Some(Standing { row: id, event: event.to_string(), how, until_ms });
        self.tick(now)
    }

    /// The reader scrolled back: the event no longer stands and its suggestion goes.
    pub fn on_left(&mut self, event: &str, now: Now) {
        if let Some(row) = self.standing.take_if(|s| s.event == event).map(|s| s.row) {
            self.record(now.ms, Logged::Withdrawn { row });
        }
        if let Some(row) = self.suggestion.take_if(|s| s.event.as_deref() == Some(event)).map(|s| s.row) {
            self.record(now.ms, Logged::Withdrawn { row });
        }
    }

    /// Time passes: an event that stood its hold time counts.
    pub fn tick(&mut self, now: Now) -> Vec<Out> {
        let Some(standing) = self.standing.take_if(|s| now.ms >= s.until_ms) else { return vec![] };
        match standing.how {
            How::Auto => self.fire_row(&standing.row, Origin::Auto, now),
            _ => {
                self.record(now.ms, Logged::Suggested { row: standing.row.clone() });
                self.suggestion = Some(Suggestion { row: standing.row, event: Some(standing.event) });
                vec![]
            }
        }
    }

    /// The operator fires a row. Always allowed.
    pub fn fire(&mut self, row: &str, now: Now) -> Vec<Out> {
        self.fire_row(row, Origin::Hand, now)
    }

    /// GO: the held suggestion, else the row after the current one. Always allowed.
    pub fn go(&mut self, now: Now) -> Vec<Out> {
        if let Some(s) = self.suggestion.take() {
            return self.fire_row(&s.row, Origin::Go, now);
        }
        let next = match &self.current {
            Some(c) => self.show.rows.iter().position(|r| &r.id == c).map(|i| i + 1).unwrap_or(0),
            None => 0,
        };
        match self.show.rows.get(next).map(|r| r.id.clone()) {
            Some(row) => self.fire_row(&row, Origin::Go, now),
            None => {
                self.record(now.ms, Logged::NoNextRow);
                vec![]
            }
        }
    }

    /// A parent's row, taken by the seat's word: a child treats its parent like a reader. The local GO always works.
    pub fn on_parent(&mut self, word: Word, says: &Says, now: Now) -> Vec<Out> {
        self.on_parent_at(word, says, None, now)
    }

    /// The same, with the parent's moment: a following child's row falls on the parent's beat, not on one of its own.
    pub fn on_parent_at(&mut self, word: Word, says: &Says, at: Option<i64>, now: Now) -> Vec<Out> {
        let row = match says {
            Says::Go(row) if self.show.row(row).is_some() => row.clone(),
            Says::Go(row) => {
                self.record(now.ms, Logged::ParentIgnored { row: Some(row.clone()) });
                return vec![];
            }
            Says::Free => {
                self.record(now.ms, Logged::ParentIgnored { row: None });
                return vec![];
            }
        };
        match word {
            Word::Follow => self.fire_row_at(&row, Origin::Parent, at, now),
            Word::Suggest => {
                self.record(now.ms, Logged::Suggested { row: row.clone() });
                self.suggestion = Some(Suggestion { row, event: None });
                vec![]
            }
            Word::Free => {
                self.record(now.ms, Logged::ParentIgnored { row: Some(row) });
                vec![]
            }
        }
    }

    /// One cell of a row, by hand; off its row it is marked. Row firing stays the main way (SHOW.md §7).
    pub fn launch_cell(&mut self, row: &str, column: &str, now: Now) -> Vec<Out> {
        let Some(r) = self.show.row(row) else { return vec![] };
        let cell = r.cells.get(column).cloned().unwrap_or(Cell::Keep);
        let at = moment_at(r.fire.moment, now);
        let off_row = self.current.as_deref() != Some(row);
        self.record(now.ms, Logged::CellLaunched { row: row.to_string(), column: column.to_string(), off_row });
        let mut out = vec![];
        self.apply(Launch { row, off_row, at, ms: now.ms }, column, &cell, &mut out);
        self.update_mirrors(at, &mut out);
        out
    }

    /// `kind:id > action`, to what plays that piece on this conductor's stage. Finding nobody stays silent and is logged;
    /// nothing goes up (owner 2026-11-09).
    pub fn address(&mut self, address: &str, action: &str, now: Now) -> Vec<Out> {
        let place = Place::parse(address).ok().map(|p| p.to_string());
        let reached = self.stage.values().filter(|s| matches!(&s.playing, Playing::Piece { piece, .. } if Some(piece) == place.as_ref())).count();
        self.record(now.ms, Logged::Addressed { address: address.to_string(), action: action.to_string(), reached });
        if reached == 0 {
            return vec![];
        }
        let story = json!({ "type": "address", "params": { "address": address, "action": action } });
        let Some(story) = story.as_object().cloned() else { return vec![] };
        let mut e = Envelope::new("", Message::Story(story));
        e.to = Some(To { role: Some(Role::Output), group: None, client: None });
        vec![Out::Send(Box::new(e))]
    }

    fn fire_row(&mut self, id: &str, origin: Origin, now: Now) -> Vec<Out> {
        self.fire_row_at(id, origin, None, now)
    }

    fn fire_row_at(&mut self, id: &str, origin: Origin, at: Option<i64>, now: Now) -> Vec<Out> {
        let Some(row) = self.show.row(id).cloned() else { return vec![] };
        let at = at.or_else(|| moment_at(row.fire.moment, now));
        self.record(now.ms, Logged::Fired { row: id.to_string(), origin });
        if self.suggestion.as_ref().is_some_and(|s| s.row == id) {
            self.suggestion = None;
        }
        self.current = Some(id.to_string());
        let columns: Vec<String> = self.show.outlets.iter().map(|o| o.id.clone()).chain(self.show.seats.iter().map(|s| s.id.clone())).collect();
        let mut out = vec![];
        for column in &columns {
            let cell = row.cells.get(column).unwrap_or(&Cell::Keep);
            self.apply(Launch { row: id, off_row: false, at, ms: now.ms }, column, cell, &mut out);
        }
        self.update_mirrors(at, &mut out);
        out
    }

    fn apply(&mut self, launch: Launch, column: &str, cell: &Cell, out: &mut Vec<Out>) {
        let Launch { row, off_row, at, ms } = launch;
        if let Some(seat) = self.show.seat(column) {
            let word = seat.word;
            match cell {
                Cell::Go { row } => out.push(Out::Seat { seat: column.to_string(), word, says: Says::Go(row.clone()), at }),
                Cell::Free => out.push(Out::Seat { seat: column.to_string(), word, says: Says::Free, at }),
                _ => {}
            }
            return;
        }
        let device = self.show.outlet(column).is_some_and(|o| o.kind == OutletKind::Device);
        let playing = match cell {
            Cell::Keep => {
                if let Some(on) = self.stage.get_mut(column) {
                    on.row = row.to_string();
                }
                return;
            }
            Cell::Play { send, .. } if device => {
                self.send_device(column, send, at, out);
                Playing::Sent
            }
            Cell::Play { piece, values, .. } => Playing::Piece { piece: piece.clone().unwrap_or_default(), values: values.clone() },
            Cell::Values { piece, values } => match self.stage.get(column).map(|s| &s.playing) {
                Some(Playing::Piece { piece: there, values: old }) if there == piece => {
                    let mut merged = old.clone();
                    merged.extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
                    Playing::Piece { piece: piece.clone(), values: merged }
                }
                found => {
                    let found = found.map(|p| match p {
                        Playing::Piece { piece, .. } => piece.clone(),
                        Playing::Mirror { of } => format!("mirror of {of}"),
                        Playing::Black => "black".to_string(),
                        Playing::Sent => "device".to_string(),
                    });
                    self.record(ms, Logged::ValuesSkipped { row: row.to_string(), outlet: column.to_string(), expected: piece.clone(), found });
                    return;
                }
            },
            Cell::Mirror { of } => Playing::Mirror { of: of.clone() },
            Cell::Black => Playing::Black,
            Cell::Go { .. } | Cell::Free => return,
        };
        match &playing {
            Playing::Piece { piece, values } => self.send_cell(column, full_cell(piece, values), at, out),
            Playing::Black => self.send_cell(column, black_cell(), at, out),
            Playing::Mirror { .. } | Playing::Sent => {}
        }
        self.stage.insert(column.to_string(), OnStage { playing, row: row.to_string(), off_row });
    }

    fn send_cell(&mut self, outlet: &str, cell: protocol::Cell, at: Option<i64>, out: &mut Vec<Out>) {
        self.sent.insert(outlet.to_string(), cell.clone());
        out.push(envelope(Message::Cell(CellChange { outlet: outlet.to_string(), cell }), at));
    }

    /// A device outlet: the bridge takes `device`, not a cell (`Cell.send` has no reader).
    fn send_device(&mut self, outlet: &str, msgs: &[DeviceMsg], at: Option<i64>, out: &mut Vec<Out>) {
        out.push(envelope(Message::Device(Device { outlet: outlet.to_string(), msgs: msgs.to_vec(), at }), at));
    }

    /// What a mirrored outlet shows now (the deck takes a mirror as black: the conductor sends the cell itself).
    fn resolved(&self, outlet: &str) -> protocol::Cell {
        let mut at = outlet;
        // a chain across rows can close; the walk is bounded by the number of outlets
        for _ in 0..=self.show.outlets.len() {
            match self.stage.get(at).map(|s| &s.playing) {
                Some(Playing::Mirror { of }) => at = of,
                Some(Playing::Piece { piece, values }) => return full_cell(piece, values),
                _ => return black_cell(),
            }
        }
        black_cell()
    }

    /// Every mirroring outlet shows its source's cell, also when only the source changed.
    fn update_mirrors(&mut self, at: Option<i64>, out: &mut Vec<Out>) {
        let mirrors: Vec<String> = self.stage.iter().filter(|(_, s)| matches!(s.playing, Playing::Mirror { .. })).map(|(k, _)| k.clone()).collect();
        for outlet in mirrors {
            let cell = self.resolved(&outlet);
            if self.sent.get(&outlet) != Some(&cell) {
                self.send_cell(&outlet, cell, at, out);
            }
        }
    }
}
