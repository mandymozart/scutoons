//! The bare conductor (#89, SHOW.md §5, §7): one show, in memory, driven by calls. The adapter passes the time in and moves
//! the bytes; every decision is here (RULES A1). No relay tree yet (#90): what a row says to a seat comes back as [`Out::Seat`].
//!
//! You write only what changes; the conductor resolves `keep` and `values` along the path really taken and sends a **full
//! cell**, so a deck that reboots mid-show is right at once from the snapshot. GO and the hand are always allowed.
use crate::document::{Cell, How, Moment, OutletKind, Show, Values, Word};
use deck_core::tempo::Quantize;
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
    /// stopped, the beat stands
    pub running: bool,
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
    /// #91: a fired row's address, down this subtree (the subtree asks it and logs the answers)
    Address(crate::document::Addressed),
    /// a cell's camera word (`<scene> > shot:<id>`) for its column's decks only (the address's `where.group`), on the beat
    /// of the row's moment (none: at once)
    Camera { column: String, address: String, action: String, beat: Option<f64> },
    /// #548: a fired row's question, to open as the session's vote (the subtree closes one still open first, then opens it:
    /// `Subtree::vote_open`)
    Ask(Ask),
}

impl Out {
    /// what goes over the relay, if this does
    pub fn envelope(&self) -> Option<&Envelope> {
        match self {
            Out::Send(e) => Some(e),
            Out::Seat { .. } | Out::Address(_) | Out::Camera { .. } | Out::Ask(_) => None,
        }
    }
}

/// #548 (SHOW.md §15 3): the question a row asks – the show's question, the columns whose `ask` cells name it (the show's
/// outlet order: the vote's `groups`) and their filter (one per question, validate's rule: the vote's `where`)
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub row: String,
    pub question: String,
    pub columns: Vec<String>,
    pub filter: Option<protocol::Filter>,
}

impl Ask {
    /// what row `row` asks, if it asks a question
    pub fn of(show: &Show, row: &crate::document::Row) -> Option<Ask> {
        let question = crate::rows::asked(row)?.to_string();
        let columns: Vec<String> = show.outlets.iter().filter(|o| matches!(row.cells.get(&o.id), Some(Cell::Ask { question: q }) if *q == question)).map(|o| o.id.clone()).collect();
        let filter = columns.first().and_then(|c| row.r#where.get(c)).cloned();
        Some(Ask { row: row.id.clone(), question, columns, filter })
    }

    /// The vote it opens with the id `vote` (vote.rs `open`), or why not (no such question, the form's check).
    pub fn vote(&self, show: &Show, vote: &str) -> Result<protocol::VoteOpen, String> {
        let q = show.question(&self.question).ok_or_else(|| format!("no question '{}'", self.question))?;
        crate::vote::open(q, vote, &self.columns, self.filter.as_ref())
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
    /// the show clock's beat at `at`
    beat: Option<f64>,
    ms: u64,
}

/// An event standing for its hold time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Standing {
    row: String,
    event: String,
    how: How,
    until_ms: u64,
    /// the rows its pass fired already: a row fired by a row fires once per pass (A fires B fires A stops at A)
    pass: Vec<String>,
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
    /// the relay ms it starts at (its row's moment); none: at once
    pub from: Option<i64>,
    /// the row whose cell plays there until then
    pub before: Option<String>,
}

impl OnStage {
    /// whether it still waits for its moment at relay time `ms`
    pub fn queued(&self, ms: u64) -> bool {
        self.from.is_some_and(|f| f > ms as i64)
    }
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
    /// a row fired by a row came round in the same pass: it fires once per pass, the loop stops here
    Looped { row: String },
    /// #539: a story event without an id (its type, empty: none): dropped, no row can name it
    NoId { kind: String },
    /// a single cell, by hand
    CellLaunched { row: String, column: String, off_row: bool },
    /// a column stopped by hand
    Stopped { column: String },
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
    /// #533: the session's vote opened (its question, empty: none), and its result – the option and how (`rule`,
    /// `fallback`, `override`)
    VoteOpened { vote: String, question: String },
    /// #548: and the show's event the result is (`vote:<question>/<option>`, none: a vote not asked by a row)
    VoteDecided { vote: String, option: String, how: String, event: Option<String> },
    /// #548: a row's question did not open (no such question, the form's check), and why
    VoteNotOpened { row: String, question: String, why: String },
    Panicked,
    Resumed,
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
    standing: Vec<Standing>,
    suggestion: Option<Suggestion>,
    log: VecDeque<Entry>,
}

fn full_cell(piece: &str, values: &Values, background: Option<&deck_space::Background>) -> protocol::Cell {
    let background = background.and_then(|b| match serde_json::to_value(b) {
        Ok(serde_json::Value::Object(m)) => Some(m),
        _ => None,
    });
    let src = LayerSrc::Piece(PieceSrc { piece: piece.to_string(), values: (!values.is_empty()).then(|| values.clone()), background });
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
    let next = moment_beat(moment, now)?;
    let ms = (next - clock.beat) * 60_000.0 / clock.bpm.max(1.0);
    Some(now.ms as i64 + ms.round() as i64)
}

/// the show clock's beat of a row's moment (none: now, or no clock)
fn moment_beat(moment: Moment, now: Now) -> Option<f64> {
    let clock = now.clock?;
    let per_bar = clock.per_bar.max(1);
    match moment {
        Moment::Now => None,
        Moment::Bar => Some(Quantize::Bar.next(clock.beat, per_bar)),
        Moment::Phrase => Some(Quantize::Bar.next(clock.beat, per_bar * BARS_PER_PHRASE)),
    }
}

/// the show clock's beat at relay time `at`
fn beat_at(at: Option<i64>, now: Now) -> Option<f64> {
    let (at, clock) = (at?, now.clock?);
    Some(clock.beat + (at - now.ms as i64) as f64 * clock.bpm / 60_000.0)
}

impl Conductor {
    pub fn new(show: Show) -> Conductor {
        Conductor { show, stage: BTreeMap::new(), sent: BTreeMap::new(), current: None, standing: Vec::new(), suggestion: None, log: VecDeque::new() }
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
        let before = self.playing_row(outlet, ms);
        self.stage.insert(outlet.to_string(), OnStage { playing, row, off_row: true, from: at, before });
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
    /// scrolled back past it. One without an id is dropped and logged (#539). Everything else is not the conductor's.
    pub fn on_message(&mut self, message: &Message, now: Now) -> Vec<Out> {
        let Message::Story(story) = message else { return vec![] };
        let Some(id) = story.get("id").and_then(|v| v.as_str()).filter(|id| !id.is_empty()) else {
            let kind = story.get("type").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            self.record(now.ms, Logged::NoId { kind });
            return vec![];
        };
        if story.get("left").and_then(|v| v.as_bool()) == Some(true) {
            self.on_left(id, now);
            vec![]
        } else {
            self.on_event(id, now)
        }
    }

    /// A reader's event: it fires its row on auto, is held as a suggestion on suggest, does nothing on hand.
    /// Every row that listens to it hears it, in the show's order.
    pub fn on_event(&mut self, event: &str, now: Now) -> Vec<Out> {
        self.on_event_in(event, now, &[])
    }

    /// An event inside a pass: `pass` holds the rows this pass fired already.
    fn on_event_in(&mut self, event: &str, now: Now, pass: &[String]) -> Vec<Out> {
        let rows: Vec<(String, How, u32)> = self.show.rows.iter().filter(|r| r.fire.event.contains(event)).map(|r| (r.id.clone(), r.fire.how, r.fire.hold_ms)).collect();
        if rows.is_empty() {
            self.record(now.ms, Logged::NoRow { event: event.to_string() });
            return vec![];
        }
        // a newer reader's event replaces those still standing; one standing for the same event keeps its time. A row's
        // fire replaces nothing: it is part of the pass that stands.
        if !event.starts_with(crate::document::ROW_FIRED) {
            self.standing.retain(|s| s.event == event || s.event.starts_with(crate::document::ROW_FIRED));
        }
        for (id, how, hold) in rows {
            if how == How::Manual {
                self.record(now.ms, Logged::Ignored { row: id, event: event.to_string() });
                continue;
            }
            if !self.standing.iter().any(|s| s.row == id && s.event == event) {
                self.standing.push(Standing { row: id, event: event.to_string(), how, until_ms: now.ms + hold as u64, pass: pass.to_vec() });
            }
        }
        self.tick(now)
    }

    /// The reader scrolled back: the event no longer stands and its suggestion goes.
    pub fn on_left(&mut self, event: &str, now: Now) {
        let (gone, stays): (Vec<Standing>, Vec<Standing>) = std::mem::take(&mut self.standing).into_iter().partition(|s| s.event == event);
        self.standing = stays;
        for s in gone {
            self.record(now.ms, Logged::Withdrawn { row: s.row });
        }
        if let Some(row) = self.suggestion.take_if(|s| s.event.as_deref() == Some(event)).map(|s| s.row) {
            self.record(now.ms, Logged::Withdrawn { row });
        }
    }

    /// Time passes: the events that stood their hold time count, in the order they came. Several suggestions at once: the
    /// first is held, the operator's GO takes it.
    pub fn tick(&mut self, now: Now) -> Vec<Out> {
        let (due, stays): (Vec<Standing>, Vec<Standing>) = std::mem::take(&mut self.standing).into_iter().partition(|s| now.ms >= s.until_ms);
        self.standing = stays;
        let mut out = vec![];
        let mut held = false;
        for s in due {
            match s.how {
                How::Auto => {
                    let mut pass = s.pass;
                    out.extend(self.fire_in_pass(&s.row, Origin::Auto, None, now, &mut pass));
                }
                _ if held => {}
                _ => {
                    held = true;
                    self.record(now.ms, Logged::Suggested { row: s.row.clone() });
                    self.suggestion = Some(Suggestion { row: s.row, event: Some(s.event) });
                }
            }
        }
        out
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

    /// Drop (#91, Perform): the operator lets the held suggestion go; nothing fires. False when none was held.
    pub fn drop_suggestion(&mut self, now: Now) -> bool {
        let Some(s) = self.suggestion.take() else { return false };
        self.record(now.ms, Logged::Withdrawn { row: s.row });
        true
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
        let beat = moment_beat(r.fire.moment, now);
        let off_row = self.current.as_deref() != Some(row);
        // #548: one `ask` cell by hand opens its whole question – every column of the row that asks it votes
        let ask = matches!(cell, Cell::Ask { .. }).then(|| Ask::of(&self.show, r)).flatten();
        self.record(now.ms, Logged::CellLaunched { row: row.to_string(), column: column.to_string(), off_row });
        let mut out = vec![];
        self.apply(Launch { row, off_row, at, beat, ms: now.ms }, column, &cell, &mut out);
        self.update_mirrors(at, &mut out);
        out.extend(ask.map(Out::Ask));
        out
    }

    /// `kind:id > action`, to what plays that piece on this conductor's stage. Finding nobody stays silent and is logged;
    /// nothing goes up (owner 2026-11-09).
    pub fn address(&mut self, address: &str, action: &str, now: Now) -> Vec<Out> {
        let reached = self.stage.values().filter(|s| matches!(&s.playing, Playing::Piece { piece, .. } if crate::address::picks_up(address, piece))).count();
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

    /// a row fired from outside – by hand, GO, a parent, a reader's event: a new pass
    fn fire_row_at(&mut self, id: &str, origin: Origin, at: Option<i64>, now: Now) -> Vec<Out> {
        self.fire_in_pass(id, origin, at, now, &mut Vec::new())
    }

    /// A row fires in a pass; its fire is an event the rows that listen to it hear in the same pass. A row the pass fired
    /// already does not fire again: the loop stops there.
    fn fire_in_pass(&mut self, id: &str, origin: Origin, at: Option<i64>, now: Now, pass: &mut Vec<String>) -> Vec<Out> {
        if pass.iter().any(|r| r == id) {
            self.record(now.ms, Logged::Looped { row: id.to_string() });
            return vec![];
        }
        pass.push(id.to_string());
        let mut out = self.fire_once(id, origin, at, now);
        let fired = crate::document::row_fired(id);
        if self.show.rows.iter().any(|r| r.fire.event.contains(&fired)) {
            out.extend(self.on_event_in(&fired, now, pass));
        }
        out
    }

    fn fire_once(&mut self, id: &str, origin: Origin, at: Option<i64>, now: Now) -> Vec<Out> {
        let Some(row) = self.show.row(id).cloned() else { return vec![] };
        let beat = if at.is_some() { beat_at(at, now) } else { moment_beat(row.fire.moment, now) };
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
            self.apply(Launch { row: id, off_row: false, at, beat, ms: now.ms }, column, cell, &mut out);
        }
        self.update_mirrors(at, &mut out);
        // #548: the row's question opens as one vote for its `ask` columns (no vote outside the show)
        out.extend(Ask::of(&self.show, &row).map(Out::Ask));
        out.extend(row.addresses.iter().cloned().map(Out::Address));
        out
    }

    fn apply(&mut self, launch: Launch, column: &str, cell: &Cell, out: &mut Vec<Out>) {
        let Launch { row, off_row, at, beat, ms } = launch;
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
            // #546: an `ask` cell keeps the piece; the vote it opens is the row's (#548: `Out::Ask`)
            Cell::Keep | Cell::Ask { .. } => {
                if let Some(on) = self.stage.get_mut(column) {
                    on.row = row.to_string();
                }
                return;
            }
            Cell::Play { send, .. } if device => {
                self.send_device(column, send, at, out);
                Playing::Sent
            }
            Cell::Look { look, fade } => {
                let msgs = self.show.look(look).map(|l| l.dmx(&self.show, column, *fade)).unwrap_or_default();
                if msgs.is_empty() {
                    return;
                }
                self.send_device(column, &msgs, at, out);
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
            Playing::Piece { piece, values } => self.send_cell(column, full_cell(piece, values, self.show.background.as_ref()), at, out),
            Playing::Black => self.send_cell(column, black_cell(), at, out),
            Playing::Mirror { .. } | Playing::Sent => {}
        }
        let before = self.playing_row(column, ms);
        self.stage.insert(column.to_string(), OnStage { playing, row: row.to_string(), off_row, from: at, before });
        // the cell's camera shot: the scene's camera word, after the cell (a deck that is still opening the scene keeps it),
        // only to this column's decks, on the row's moment
        if let Cell::Play { piece: Some(p), shot: Some(s), .. } = cell {
            out.push(Out::Camera { column: column.to_string(), address: p.clone(), action: format!("{}:{s}", deck_space::rig::address::SHOT), beat });
        }
    }

    /// the row whose cell plays on `column` at relay time `ms` (one still waiting for its moment does not yet)
    fn playing_row(&self, column: &str, ms: u64) -> Option<String> {
        let on = self.stage.get(column)?;
        if on.queued(ms) { on.before.clone() } else { Some(on.row.clone()).filter(|r| !r.is_empty()) }
    }

    /// The operator stops a column: a stage outlet goes black at the moment of the row playing there, a seat's child is
    /// left free, a device outlet is sent nothing more. Always allowed.
    pub fn stop_column(&mut self, column: &str, now: Now) -> Vec<Out> {
        let mut out = vec![];
        if let Some(seat) = self.show.seat(column) {
            let word = seat.word;
            self.record(now.ms, Logged::Stopped { column: column.to_string() });
            out.push(Out::Seat { seat: column.to_string(), word, says: Says::Free, at: None });
            return out;
        }
        let Some(outlet) = self.show.outlet(column) else { return out };
        let device = outlet.kind == OutletKind::Device;
        let playing = self.playing_row(column, now.ms);
        let moment = playing.as_deref().and_then(|r| self.show.row(r)).map_or(Moment::Now, |r| r.fire.moment);
        let at = moment_at(moment, now);
        self.record(now.ms, Logged::Stopped { column: column.to_string() });
        if !device {
            self.send_cell(column, black_cell(), at, &mut out);
        }
        self.stage.insert(column.to_string(), OnStage { playing: Playing::Black, row: String::new(), off_row: false, from: at, before: playing });
        self.update_mirrors(at, &mut out);
        out
    }

    fn send_cell(&mut self, outlet: &str, cell: protocol::Cell, at: Option<i64>, out: &mut Vec<Out>) {
        self.sent.insert(outlet.to_string(), cell.clone());
        out.push(envelope(Message::Cell(CellChange { outlet: outlet.to_string(), cell }), at));
    }

    /// Every outlet's last cell again, at once.
    pub fn resend(&self) -> Vec<Out> {
        self.sent.iter().map(|(outlet, cell)| envelope(Message::Cell(CellChange { outlet: outlet.clone(), cell: cell.clone() }), None)).collect()
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
                Some(Playing::Piece { piece, values }) => return full_cell(piece, values, self.show.background.as_ref()),
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
