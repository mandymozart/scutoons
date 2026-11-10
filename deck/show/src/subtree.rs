//! One conductor in the tree (#90, SHOW.md §4, §4b): the bare [`Conductor`] and what it knows of its neighbours. Its own rows,
//! operator and clock; its seats below and its parent above. The adapter moves envelopes and passes the time in, everything
//! that is decided is here (RULES A1).
//!
//! **A child treats its parent like a reader:** a parent's row arrives as a word and the child's rule applies, follow = auto,
//! suggest = its operator decides, free = ignored. The local GO always works. A lock from above makes the parent's word the
//! one in force; otherwise the nearest wins, so a child's own word stands.
//!
//! If the parent goes away nothing changes here: the rows that fired stay fired, the operator goes on.
use crate::conductor::{Conductor, Entry, Logged, Now, Out, Says};
use crate::document::{Pinned, Show, Word};
use crate::flash::{self, Assignment};
use crate::validate::Lock;
use protocol::{Address, AddressAnswer, AddressFilter, CellChange, Envelope, Flash, Message, Panic, Role, SeatState, SeatWord, Tempo, To, PARENT};
use std::collections::{BTreeMap, BTreeSet};

/// How long a conductor waits for answers to an address before it writes the log line.
pub const ANSWER_MS: u64 = 2000;

/// Whose time a subtree plays on (owner 2026-11-03 [clock]: a question per subtree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockChoice {
    /// the parent's anchor, passed on to this subtree's decks
    Parent,
    /// its own; the id goes out as `Tempo.domain`
    Own(String),
}

/// An address this conductor asked, waiting for its answers.
#[derive(Debug, Clone)]
struct Asked {
    address: String,
    action: String,
    since: u64,
    reached: u64,
    passed: u64,
    answered: u64,
}

/// To the parent, over the link's end in this conductor's own session.
fn up() -> To {
    To::role_group(Role::Seat, PARENT)
}

fn message(m: Message, at: Option<i64>) -> Envelope {
    let mut e = Envelope::new("", m);
    e.at = at;
    e
}

fn up_envelope(m: Message) -> Envelope {
    Envelope::new("", m).to(up())
}

#[derive(Debug, Clone)]
pub struct Subtree {
    conductor: Conductor,
    clock: ClockChoice,
    anchor: Option<Tempo>,
    /// the parent's word and whether it is locked
    parent_word: Option<Word>,
    locked: bool,
    /// the word this subtree's own operator set
    own_word: Option<Word>,
    /// the seats this conductor has locked
    locks: BTreeSet<String>,
    assignment: Option<Assignment>,
    /// why a flash cannot be taken here (no such hardware …); none = it can
    cannot: Option<String>,
    /// a version a flash wants and the adapter has yet to fetch
    wanted: Option<Pinned>,
    asked: BTreeMap<String, Asked>,
    next_address: u64,
    /// the link to the parent: `None` = no parent
    linked: Option<bool>,
    /// #91: what each deck and sub conductor answered to the last flash, by the path it came along (`from`)
    reports: BTreeMap<String, protocol::FlashReport>,
    /// #471: the last thing each deck and sub conductor said about its flash side (also before any flash), never cleared:
    /// the flash list (choose.rs) is made of it
    devices: BTreeMap<String, protocol::FlashReport>,
    /// #471: who is here now: this session's client ids (presence) and the seats whose relay is linked
    online: BTreeSet<String>,
    ready: u64,
    clients: u64,
    reported: Option<SeatState>,
}

fn word_of(w: &SeatWord) -> Says {
    match (w.says, &w.row) {
        (protocol::Says::Go, Some(row)) => Says::Go(row.clone()),
        _ => Says::Free,
    }
}

impl Subtree {
    pub fn new(show: Show, clock: ClockChoice) -> Subtree {
        Subtree {
            conductor: Conductor::new(show),
            clock,
            anchor: None,
            parent_word: None,
            locked: false,
            own_word: None,
            locks: BTreeSet::new(),
            assignment: None,
            cannot: None,
            wanted: None,
            asked: BTreeMap::new(),
            next_address: 0,
            linked: None,
            reports: BTreeMap::new(),
            devices: BTreeMap::new(),
            online: BTreeSet::new(),
            ready: 0,
            clients: 0,
            reported: None,
        }
    }

    pub fn conductor(&self) -> &Conductor {
        &self.conductor
    }

    pub fn log(&self) -> impl Iterator<Item = &Entry> {
        self.conductor.log()
    }

    /// The word in force: a lock from above wins, otherwise the nearest wins.
    pub fn word(&self) -> Option<Word> {
        if self.locked {
            self.parent_word
        } else {
            self.own_word.or(self.parent_word)
        }
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn assignment(&self) -> Option<&Assignment> {
        self.assignment.as_ref()
    }

    /// What each deck and sub conductor answered to the last flash (#91), by the path it came along.
    pub fn reports(&self) -> &BTreeMap<String, protocol::FlashReport> {
        &self.reports
    }

    /// #471: what each deck and sub conductor last said about its flash side, by its path.
    pub fn devices(&self) -> &BTreeMap<String, protocol::FlashReport> {
        &self.devices
    }

    /// #471: whether the device at `path` is here: its client (this session) or its seat's relay (a sub conductor's session).
    pub fn is_online(&self, path: &str) -> bool {
        self.online.contains(path.split('/').next().unwrap_or(path))
    }

    /// The show a flash wants that the adapter has yet to fetch.
    pub fn wanted(&self) -> Option<&Pinned> {
        self.wanted.as_ref()
    }

    fn pinned(&self) -> Pinned {
        let show = self.conductor.show();
        Pinned { id: show.id.clone(), version: show.version }
    }

    fn now(&self, ms: u64) -> Now {
        Now { ms, clock: self.clock_at(ms) }
    }

    /// Where the clock this subtree plays on stands at relay time `ms`.
    pub fn clock_at(&self, ms: u64) -> Option<crate::conductor::Clock> {
        let t = self.anchor.as_ref()?;
        let at = t.at.unwrap_or(ms as i64);
        let beat = if t.running { t.beat + (ms as i64 - at) as f64 * t.bpm / 60_000.0 } else { t.beat };
        Some(crate::conductor::Clock { bpm: t.bpm, beat, per_bar: t.per_bar.max(1) as u32 })
    }

    fn wire(&mut self, outs: Vec<Out>, ms: u64) -> Vec<Envelope> {
        outs.into_iter()
            .flat_map(|o| match o {
                Out::Send(e) => vec![*e],
                Out::Address(a) => self.address(&a.address, &a.action, a.filter, ms),
                Out::Seat { seat, word, says, at } => {
                    let (says, row) = match says {
                        Says::Go(r) => (protocol::Says::Go, Some(r)),
                        Says::Free => (protocol::Says::Free, None),
                    };
                    let lock = self.locks.contains(&seat).then_some(true);
                    vec![message(Message::Seat(SeatWord { seat, word, says, row, lock }), at)]
                }
            })
            .collect()
    }

    /// What this conductor looks like from above, once when it changed.
    fn report(&mut self, out: &mut Vec<Envelope>) {
        if self.linked != Some(true) {
            return;
        }
        let own = matches!(self.clock, ClockChoice::Own(_));
        let clock = own.then(|| match &self.clock {
            ClockChoice::Own(d) => d.clone(),
            ClockChoice::Parent => String::new(),
        });
        let state = SeatState {
            show: Some(flash::wire(&self.pinned())),
            row: self.conductor.current().map(str::to_string),
            ready: Some(self.ready),
            clients: Some(self.clients),
            word: self.word(),
            locked: self.locked.then_some(true),
            clock,
            online: None,
        };
        if self.reported.as_ref() != Some(&state) {
            self.reported = Some(state.clone());
            out.push(up_envelope(Message::SeatState(state)));
        }
    }

    fn done(&mut self, outs: Vec<Out>, ms: u64) -> Vec<Envelope> {
        let mut out = self.wire(outs, ms);
        self.report(&mut out);
        out
    }

    // ---------------------------------------------------------------- the operator: always allowed

    pub fn fire(&mut self, row: &str, ms: u64) -> Vec<Envelope> {
        let now = self.now(ms);
        let outs = self.conductor.fire(row, now);
        self.done(outs, ms)
    }

    pub fn go(&mut self, ms: u64) -> Vec<Envelope> {
        let now = self.now(ms);
        let outs = self.conductor.go(now);
        self.done(outs, ms)
    }

    /// Drop: the held suggestion goes, nothing fires (#91).
    pub fn drop_suggestion(&mut self, ms: u64) -> bool {
        let now = self.now(ms);
        self.conductor.drop_suggestion(now)
    }

    pub fn launch_cell(&mut self, row: &str, column: &str, ms: u64) -> Vec<Envelope> {
        let now = self.now(ms);
        let outs = self.conductor.launch_cell(row, column, now);
        self.done(outs, ms)
    }

    pub fn tick(&mut self, ms: u64) -> Vec<Envelope> {
        let now = self.now(ms);
        let outs = self.conductor.tick(now);
        let done: Vec<String> = self.asked.iter().filter(|(_, a)| ms >= a.since + ANSWER_MS).map(|(id, _)| id.clone()).collect();
        for id in done {
            if let Some(a) = self.asked.remove(&id) {
                self.conductor.note(ms, Logged::Reached { address: a.address, action: a.action, reached: a.reached, passed: a.passed, answered: a.answered });
            }
        }
        self.done(outs, ms)
    }

    /// This subtree's own word. Refused under a lock from above (and logged); the local GO is not touched by that.
    pub fn set_word(&mut self, word: Option<Word>, ms: u64) -> Result<Vec<Envelope>, Vec<Envelope>> {
        if self.locked {
            self.conductor.note(ms, Logged::WordRefused);
            return Err(vec![]);
        }
        self.own_word = word;
        Ok(self.done(vec![], ms))
    }

    /// Lock a seat below: its word from here on is the one in force there. Only a seat that may be locked (RULES: across members
    /// only if accepted, SHOW.md §4b; the document's `lockable` is that answer).
    pub fn lock_seat(&mut self, seat: &str, lock: bool, ms: u64) -> Vec<Envelope> {
        let Some(s) = self.conductor.show().seat(seat) else { return vec![] };
        if lock && !s.lockable {
            self.conductor.note(ms, Logged::LockRefused { seat: seat.to_string() });
            return vec![];
        }
        let word = s.word;
        if lock {
            self.locks.insert(seat.to_string());
        } else {
            self.locks.remove(seat);
        }
        vec![message(Message::Seat(SeatWord { seat: seat.to_string(), word, says: protocol::Says::None, row: None, lock: Some(lock) }), None)]
    }

    /// A cell for an outlet of the child in a seat, when the seat uses it.
    pub fn cell_down(&mut self, seat: &str, outlet: &str, cell: protocol::Cell, ms: u64) -> Vec<Envelope> {
        let used = self.conductor.show().seat(seat).is_some_and(|s| s.uses.outlets.iter().any(|o| o == outlet));
        if !used {
            self.conductor.note(ms, Logged::NotUsed { seat: seat.to_string(), what: outlet.to_string() });
            return vec![];
        }
        vec![message(Message::Cell(CellChange { outlet: format!("{seat}/{outlet}"), cell }), None)]
    }

    /// This subtree's clock, when it leads it (the top, or a sub conductor with its own). A sub conductor on its parent's time
    /// does not lead.
    pub fn lead_clock(&mut self, tempo: Tempo, ms: u64) -> Vec<Envelope> {
        let domain = match &self.clock {
            ClockChoice::Own(d) => Some(d.clone()),
            ClockChoice::Parent if self.linked.is_some() => {
                self.conductor.note(ms, Logged::ClockRefused);
                return vec![];
            }
            ClockChoice::Parent => None,
        };
        let t = Tempo { domain, ..tempo };
        self.anchor = Some(t.clone());
        vec![message(Message::Tempo(t), None)]
    }

    /// `kind:id > action`, filtered, down this subtree. The log line comes when the answers are in (or after [`ANSWER_MS`]).
    pub fn address(&mut self, address: &str, action: &str, filter: Option<AddressFilter>, ms: u64) -> Vec<Envelope> {
        self.next_address += 1;
        let id = format!("{}/{}", self.conductor.show().id, self.next_address);
        self.asked.insert(id.clone(), Asked { address: address.to_string(), action: action.to_string(), since: ms, reached: 0, passed: 0, answered: 0 });
        vec![message(Message::Address(Address { id, address: address.to_string(), action: action.to_string(), r#where: filter }), None)]
    }

    /// The operator's flash to every device, the tree resolved to exact versions (`validate::resolve`).
    pub fn flash(&mut self, lock: &Lock, locked: bool, ms: u64) -> Vec<Envelope> {
        self.flash_to(lock, &crate::choose::Pick { locked, ..Default::default() }, ms)
    }

    /// #471: the operator's flash as the confirmed prompt chose it (`choose::Choice::pick`): who is left out, who is flashed
    /// although the prompt warned, whether it is locked. A deck that is not here takes it when it joins, unless left out.
    pub fn flash_to(&mut self, lock: &Lock, pick: &crate::choose::Pick, ms: u64) -> Vec<Envelope> {
        let Some(root) = lock.shows.first() else { return vec![] };
        let list = |l: &[String]| (!l.is_empty()).then(|| l.to_vec());
        let flash = Flash {
            root: flash::wire(root),
            shows: lock.shows.iter().map(flash::wire).collect(),
            lock: pick.locked.then_some(true),
            hops: 0,
            here: None,
            id: Some(format!("{root}/{ms}")),
            by: Some(pick.by.clone().unwrap_or_else(|| format!("the conductor of {}", self.conductor.show().id))),
            skip: list(&pick.skip),
            force: list(&pick.force),
        };
        self.take_flash(&flash, ms)
    }

    pub fn set_cannot(&mut self, why: Option<String>) {
        self.cannot = why;
    }

    /// The show a flash asked for, fetched: this is what runs from now, and the parent is told.
    pub fn install(&mut self, show: Show, ms: u64) -> Vec<Envelope> {
        self.conductor.replace(show);
        self.wanted = None;
        self.conductor.note(ms, Logged::Installed);
        let report = flash::report(&self.pinned(), true, None);
        let mut out = vec![up_envelope(Message::FlashReport(report))];
        self.report(&mut out);
        out
    }

    fn take_flash(&mut self, flash: &Flash, ms: u64) -> Vec<Envelope> {
        let running = self.pinned();
        // #471: from above, this subtree may have been left out (named by its show); then nothing goes down either
        let report = if flash.hops > 0 && flash::left_out(flash, &running.id) {
            flash::report(&running, false, Some("left out by the conductor above".to_string()))
        } else {
            flash::take(&mut self.assignment, flash, &running, self.cannot.as_deref())
        };
        let mut out = vec![];
        if report.accepted {
            self.conductor.note(ms, Logged::Flashed { root: flash.root.id.clone(), hops: flash.hops as u32 });
            let wanted = self.assignment.as_ref().and_then(|a| a.version_of(&running.id));
            if let Some(version) = wanted.filter(|v| *v != running.version) {
                self.wanted = Some(Pinned { id: running.id.clone(), version });
            }
            // the answers to an older flash are stale
            self.reports.clear();
            // down: this subtree's decks and its seats, one hop further; `here` = this show at the version the flash pins,
            // the one this session's decks take their outlet's cells of (#91)
            // #471: the lists relative to this session (this subtree's part, its seats named by the shows they sit)
            let show = self.conductor.show();
            let me = (flash.hops > 0).then_some(running.id.as_str());
            let mut next = flash::down(flash, me, |seat| show.seat(seat).map(|s| s.show.id.clone()));
            next.hops += 1;
            next.here = Some(flash::wire(&Pinned { id: running.id.clone(), version: wanted.unwrap_or(running.version) }));
            out.push(message(Message::Flash(next), None));
        } else {
            self.conductor.note(ms, Logged::FlashRefused { why: report.why.clone().unwrap_or_default() });
        }
        out.push(up_envelope(Message::FlashReport(report)));
        out
    }

    // ---------------------------------------------------------------- what comes over the relay

    /// What the snapshot of a (re)connected conductor says of its parent.
    pub fn on_snapshot(&mut self, snapshot: &protocol::Snapshot, ms: u64) -> Vec<Envelope> {
        let mut out = vec![];
        if let Some(t) = &snapshot.tempo {
            if self.clock == ClockChoice::Parent {
                self.anchor = Some(t.clone());
            }
        }
        if let Some(w) = &snapshot.seat {
            out.extend(self.on_word(w, None, ms));
        }
        if let Some(f) = &snapshot.flash {
            out.extend(self.take_flash(f, ms));
        }
        out
    }

    /// One message from this conductor's relay.
    pub fn on_wire(&mut self, e: &Envelope, ms: u64) -> Vec<Envelope> {
        let from_parent = e.from == PARENT;
        match &e.message {
            Message::Story(story) => self.on_story(&e.from, story, &e.message, ms),
            Message::Seat(w) if from_parent => self.on_word(w, e.at, ms),
            Message::Tempo(t) if from_parent => self.on_parent_tempo(t, ms),
            Message::Flash(f) if from_parent => self.take_flash(f, ms),
            Message::Address(a) if from_parent => vec![message(Message::Address(a.clone()), None)],
            Message::Cell(c) if from_parent => self.on_parent_cell(c, e.at, ms),
            Message::Panic(p) if from_parent => vec![message(Message::Panic(Panic { black: p.black, stop: p.stop }), e.at)],
            Message::AddressAnswer(a) => self.on_answer(a),
            // #471: joining (or back), this conductor learns what its decks and sub conductors last said about their flash
            // side – the flash list starts complete. Nothing else of the welcome is taken here.
            Message::Welcome(w) => {
                self.devices.extend(w.snapshot.reports.iter().flatten().map(|(p, r)| (p.clone(), r.clone())));
                vec![]
            }
            Message::FlashReport(r) => {
                self.devices.insert(e.from.clone(), r.clone());
                self.reports.insert(e.from.clone(), r.clone());
                self.conductor.note(ms, Logged::Reported { from: e.from.clone(), running: format!("{}@{}", r.running.id, r.running.version), refused: r.why.clone() });
                vec![up_envelope(Message::FlashReport(r.clone()))]
            }
            Message::Presence(p) => {
                self.ready = p.ready.len() as u64;
                self.clients = p.groups.values().copied().sum::<i64>().max(0) as u64;
                self.online = p.links.iter().flat_map(|l| l.keys().cloned()).collect();
                self.online.extend(p.seats.iter().flatten().filter(|(_, s)| s.online != Some(false)).map(|(seat, _)| seat.clone()));
                let was = self.linked;
                self.linked = p.parent.or(was);
                if self.linked != was {
                    if self.linked == Some(false) {
                        self.conductor.note(ms, Logged::ParentLost);
                    } else if was == Some(false) {
                        self.conductor.note(ms, Logged::ParentBack);
                    }
                    self.reported = None;
                }
                self.done(vec![], ms)
            }
            _ => vec![],
        }
    }

    fn on_story(&mut self, from: &str, story: &protocol::Loose, whole: &Message, ms: u64) -> Vec<Envelope> {
        let now = self.now(ms);
        let id = story.get("id").and_then(|v| v.as_str()).map(str::to_string);
        if from == PARENT {
            // an event from above: only a child that is listening takes it
            if self.word() == Some(Word::Free) {
                self.conductor.note(ms, Logged::ParentIgnored { row: None });
                return vec![];
            }
            let outs = self.conductor.on_message(whole, now);
            return self.done(outs, ms);
        }
        if let Some((seat, _)) = from.split_once('/') {
            // an event from a child: only what this parent uses of its face
            let used = id.as_deref().is_some_and(|id| self.conductor.show().seat(seat).is_some_and(|s| s.uses.events.iter().any(|e| e == id)));
            if !used {
                self.conductor.note(ms, Logged::NotUsed { seat: seat.to_string(), what: id.unwrap_or_default() });
                return vec![];
            }
        }
        let outs = self.conductor.on_message(whole, now);
        let mut out = self.done(outs, ms);
        // on up, if this show shows it: nothing travels unless asked (owner 2026-11-02 [events])
        if self.linked == Some(true) && id.is_some_and(|id| self.conductor.show().face.events.contains(&id)) {
            out.push(up_envelope(whole.clone()));
        }
        out
    }

    fn on_word(&mut self, w: &SeatWord, at: Option<i64>, ms: u64) -> Vec<Envelope> {
        let was_locked = self.locked;
        self.parent_word = Some(w.word);
        if let Some(lock) = w.lock {
            self.locked = lock;
        }
        if was_locked != self.locked {
            self.conductor.note(ms, if self.locked { Logged::Locked } else { Logged::Unlocked });
        }
        if w.says == protocol::Says::None {
            return self.done(vec![], ms);
        }
        let now = self.now(ms);
        let word = self.word().unwrap_or(Word::Free);
        let outs = self.conductor.on_parent_at(word, &word_of(w), at, now);
        self.done(outs, ms)
    }

    fn on_parent_tempo(&mut self, t: &Tempo, ms: u64) -> Vec<Envelope> {
        match &self.clock {
            ClockChoice::Parent => {
                self.anchor = Some(t.clone());
                vec![message(Message::Tempo(t.clone()), t.at)]
            }
            ClockChoice::Own(_) => {
                self.conductor.note(ms, Logged::ClockKept);
                vec![]
            }
        }
    }

    fn on_parent_cell(&mut self, c: &CellChange, at: Option<i64>, ms: u64) -> Vec<Envelope> {
        let Some((_, outlet)) = c.outlet.split_once('/') else { return vec![] };
        let shown = self.conductor.show().face.outlets.iter().any(|o| o == outlet);
        if !shown || self.word() != Some(Word::Follow) {
            self.conductor.note(ms, Logged::ParentIgnored { row: None });
            return vec![];
        }
        let outs = self.conductor.take_cell(outlet, c.cell.clone(), at, ms);
        self.done(outs, ms)
    }

    fn on_answer(&mut self, a: &AddressAnswer) -> Vec<Envelope> {
        match self.asked.get_mut(&a.id) {
            Some(asked) => {
                asked.reached += a.reached;
                asked.passed += a.passed;
                asked.answered += a.answered;
                vec![]
            }
            // someone above asked; on up
            None => vec![up_envelope(Message::AddressAnswer(a.clone()))],
        }
    }
}
