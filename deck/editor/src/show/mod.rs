//! The Show and Perform workspaces (#91; SHOW.md §3, §7, §10; the owner's drafts m1-show, m2-perform, m3-tree): beside
//! Build (the rig editor), the editor shows a show's conductor. The conductor is deck-show's `Subtree` (RULES A1, A2): the
//! editor hands it the operator's presses and the relay's envelopes with the time, sends what it answers through its
//! relay link (link.rs), and draws what deck-show's view.rs says – the matrix, the log in words, the inspector. Nothing
//! here decides what plays.
//!
//! - **Perform** (perform.rs): the matrix of rows × outlets and seats, the big row ▶, a small ▶ per cell (a hand launch,
//!   marked off its row), GO, the suggestion with GO / Drop, the log, the conductors rail; the stage on top.
//! - **Show** (author.rs): the conductors tree with a breadcrumb into a sub conductor, the rows band, the inspector, the
//!   seats, each deck's answer to the last flash; the 2D node overlay over the dimmed stage (#446) with a 2D / 3D switch; the
//!   cue sheet and the problems list in its place (sheets.rs); save and load through /api/shows (api.rs).
//!
//! The editor runs one conductor, this deck's (the tree's top it was given). A sub conductor runs on its own deck with its
//! own operator; entering it here shows its document, it does not run it.
pub mod api;
pub mod author;
mod flash;
mod link;
pub mod perform;
mod roles;
mod sheets;

use deck_kit::controls::Rect;
use deck_show::{ClockChoice, Show, Subtree};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Build,
    Show,
    Perform,
}

pub const WORKSPACES: [(Workspace, &str); 3] = [(Workspace::Build, "Build"), (Workspace::Show, "Show"), (Workspace::Perform, "Perform")];

impl Workspace {
    pub fn parse(s: &str) -> Option<Workspace> {
        WORKSPACES.iter().find(|(_, n)| n.eq_ignore_ascii_case(s)).map(|(w, _)| *w)
    }

    pub fn index(self) -> usize {
        WORKSPACES.iter().position(|(w, _)| *w == self).unwrap_or(0)
    }
}

/// What the Show workspace shows in the stage's place instead of the overlay (#91 item 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    Cues,
    Problems,
    /// #471: the devices to flash, then the prompt (flash.rs)
    Flash,
    /// #478: who is here, `role: name`, with move to, rename, kick (roles.rs)
    Who,
}

impl Sheet {
    pub fn parse(s: &str) -> Option<Sheet> {
        match s {
            "cues" => Some(Sheet::Cues),
            "problems" => Some(Sheet::Problems),
            "flash" => Some(Sheet::Flash),
            "who" => Some(Sheet::Who),
            _ => None,
        }
    }
}

/// what a press in a show workspace means
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// a row's big ▶ (Perform)
    Fire(usize),
    /// a cell's small ▶: row, column (Perform)
    Launch(usize, usize),
    Go,
    Drop,
    /// a seat of the document shown: enter its sub conductor (Show)
    Enter(usize),
    /// a step of the breadcrumb, 0 = the top
    Crumb(usize),
    /// a row of the document shown: the inspector shows it (Show)
    Row(usize),
    /// #478: a role of the document shown (deck-show's `roles` order): the tree and its column mark it (Show)
    Role(usize),
    /// #478: a new role, the selected role's name field, one of its filter's toggles, its remove (asks first)
    RoleAdd,
    RoleName,
    RoleFact(usize),
    RoleRemove,
    /// #478: the prompt asked (a role's remove, a kick): confirmed, or back
    AskYes,
    AskNo,
    /// #478, who is here: a line's move menu, rename, kick (asks first); a line of the open move menu
    WhoMove(usize),
    WhoRename(usize),
    WhoKick(usize),
    MenuPick(usize),
    Save,
    Load,
    /// the tree resolved to exact versions: the list of the devices to flash opens (#471)
    Flash,
    /// #471, the flash list: a line's tick, a quick filter, the typed filter's field, the lock, on to the prompt, the
    /// prompt confirmed, back / cancel
    FlashTick(usize),
    FlashQuick(usize),
    FlashType,
    FlashLock,
    FlashAsk,
    FlashConfirm,
    FlashCancel,
    /// the overlay: 2D (true) or the stage alone (3D)
    View(bool),
    /// the cue sheet or the problems list in the stage's place
    Sheet(Sheet),
    /// a line of the problems list: select what it points at
    Problem(usize),
}

/// The editor's conductor and the documents it shows.
pub struct Host {
    pub sub: Subtree,
    /// the sub conductors' documents, by show id (from the API or the cache, when entered)
    library: BTreeMap<String, Show>,
    /// the seats from the top into the document shown
    pub path: Vec<String>,
    /// the row the Show workspace inspects
    pub row: usize,
    /// #478: the role selected (a stage outlet's id); none = the row is inspected
    pub role: Option<String>,
    /// #478: a typed field open (a role's name) and its text
    pub typing: Option<(roles::Typing, deck_kit::text::Field)>,
    /// #478: the prompt asked before it is done (a role's remove, a kick)
    pub asking: Option<roles::Ask>,
    /// #478: the move menu open for a client (by its id), and whether a move from it locks
    pub menu: Option<String>,
    pub lock: bool,
    /// the last thing done, or why not
    pub status: String,
    pub two_d: bool,
    /// the sheet shown in the stage's place (Show), none = the stage
    pub sheet: Option<Sheet>,
    /// the seat a problem selected (marked in the tree)
    pub seat: Option<usize>,
    /// this frame's places
    pub hits: Vec<(Rect, Hit)>,
    /// #471: the flash list while it is open
    pub flashing: Option<flash::Flashing>,
    link: Option<link::Link>,
    api: api::Api,
}

/// the relay's clock: ms since the epoch (a show's moments are relay ms)
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// the demo document when none is named: one outlet, one row by hand (the editor opens on something it can show)
const EMPTY: &str = r#"{ "id": "new-show", "version": 1, "outlets": [{ "id": "left" }], "rows": [{ "id": "start", "cells": { "left": { "mode": "black" } } }] }"#;

impl Host {
    /// The show `file` holds (a document's JSON), else show `id` from the API / cache, else an empty one; joined to `relay`
    /// as its conductor when one is named.
    pub fn new(file: Option<PathBuf>, id: Option<String>, relay: Option<String>, api: String) -> Host {
        let api = api::Api::new(api);
        let mut status = String::new();
        let text = match (&file, &id) {
            (Some(f), _) => std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())),
            (None, Some(id)) => api.load(id).map(|(t, why)| {
                if let Some(w) = why {
                    status = format!("from the cache: {w}");
                }
                t
            }),
            (None, None) => Ok(EMPTY.to_string()),
        };
        // a document with problems opens: the problems list says what (#91)
        let show = match text.and_then(|t| deck_show::document::open(&t)) {
            Ok(s) => {
                let n = deck_show::validate(&s).len();
                if n > 0 {
                    status = format!("{n} problems – see Problems");
                }
                s
            }
            Err(e) => {
                status = e;
                deck_show::parse(EMPTY).expect("the empty show parses")
            }
        };
        let link = relay.and_then(|r| {
            link::Link::start(&r, std::env::var("SCUTOONS_LIVE_TOKEN").ok())
                .map_err(|e| {
                    status = format!("relay: {e}");
                })
                .ok()
        });
        Host { sub: Subtree::new(show, ClockChoice::Parent), library: BTreeMap::new(), path: Vec::new(), row: 0, role: None, typing: None, asking: None, menu: None, lock: false, status, two_d: true, sheet: None, seat: None, hits: Vec::new(), flashing: None, link, api }
    }

    /// the document at the end of the path (the top's when a step is missing)
    pub fn shown(&self) -> &Show {
        let mut show = self.sub.conductor().show();
        for seat in &self.path {
            match show.seat(seat).and_then(|s| self.library.get(&s.show.id)) {
                Some(child) => show = child,
                None => break,
            }
        }
        show
    }

    /// the breadcrumb's words: `main`, then each seat
    pub fn crumbs(&self) -> Vec<String> {
        std::iter::once(self.sub.conductor().show().id.clone()).chain(self.path.iter().cloned()).collect()
    }

    /// the relay link's line
    pub fn link_line(&self) -> String {
        self.link.as_ref().map_or("no relay (--relay ws://host:port/live/CODE)".into(), |l| l.line.lock().unwrap().clone())
    }

    fn send(&mut self, out: Vec<protocol::Envelope>) {
        if let Some(l) = &self.link {
            l.send(out);
        }
    }

    /// Each frame: the relay's envelopes and the clock to the core, what it answers out.
    pub fn frame(&mut self) {
        let ms = now();
        let mut out = self.sub.tick(ms);
        if let Some(l) = &self.link {
            while let Ok(e) = l.inbox.try_recv() {
                out.extend(self.sub.on_wire(&e, ms));
            }
        }
        // #471: the open flash list follows what the devices say (not under the prompt: what it says is what goes)
        if let Some(f) = self.flashing.as_mut().filter(|f| !f.asking) {
            f.choice.refresh(&self.sub);
        }
        self.send(out);
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        self.hits.iter().rev().find(|(r, _)| r.has(x, y)).map(|(_, h)| *h)
    }

    /// The operator's press: to the core (always allowed, SHOW.md §7) or to what the editor shows.
    pub fn press(&mut self, hit: Hit) {
        let ms = now();
        // #478: the move menu goes with any press but its own
        if !matches!(hit, Hit::MenuPick(_)) {
            self.menu = None;
        }
        let rows: Vec<String> = self.sub.conductor().show().rows.iter().map(|r| r.id.clone()).collect();
        match hit {
            Hit::Fire(i) => {
                if let Some(row) = rows.get(i) {
                    let out = self.sub.fire(row, ms);
                    self.send(out);
                }
            }
            Hit::Launch(r, c) => {
                let m = deck_show::view::matrix(&self.sub);
                if let (Some(row), Some(col)) = (rows.get(r), m.columns.get(c)) {
                    let out = self.sub.launch_cell(row, &col.id.clone(), ms);
                    self.send(out);
                }
            }
            Hit::Go => {
                let out = self.sub.go(ms);
                self.send(out);
            }
            Hit::Drop => {
                self.sub.drop_suggestion(ms);
            }
            Hit::Enter(i) => self.enter(i),
            Hit::Crumb(d) => {
                self.path.truncate(d);
                self.row = 0;
                self.role = None;
            }
            Hit::Row(i) => {
                self.row = i;
                self.role = None;
            }
            Hit::Role(i) => {
                self.role = deck_show::roles::roles(self.shown(), None).get(i).map(|r| r.id.clone());
                self.typing = None;
            }
            Hit::RoleAdd | Hit::RoleName | Hit::RoleFact(_) | Hit::RoleRemove | Hit::AskYes | Hit::AskNo => self.roles_press(hit),
            Hit::WhoMove(_) | Hit::WhoRename(_) | Hit::WhoKick(_) | Hit::MenuPick(_) => self.who_press(hit),
            Hit::Save => self.save(),
            Hit::Load => self.load(),
            Hit::Flash => self.open_flash(),
            Hit::FlashTick(_) | Hit::FlashQuick(_) | Hit::FlashType | Hit::FlashLock | Hit::FlashAsk | Hit::FlashConfirm | Hit::FlashCancel => self.flash_press(hit),
            Hit::View(two_d) => {
                self.two_d = two_d;
                self.sheet = None;
                self.flashing = None;
            }
            Hit::Sheet(s) => {
                self.sheet = Some(s);
                self.flashing = None;
            }
            Hit::Problem(i) => self.select_problem(i),
        }
    }

    /// a problem's line pressed: its row, cell or seat is selected (deck-show's `Points`)
    fn select_problem(&mut self, i: usize) {
        let show = self.shown().clone();
        let list = deck_show::view::problems(&show, self.path.is_empty().then_some(&self.sub));
        let Some(p) = list.get(i) else { return };
        self.seat = None;
        match p.points {
            deck_show::view::Points::Row(r) | deck_show::view::Points::Cell(r, _) => self.row = r,
            deck_show::view::Points::Seat(s) => self.seat = Some(s),
            deck_show::view::Points::Show => {}
        }
        self.status = p.text.clone();
    }

    /// into the sub conductor of the shown document's seat `i`: its document from the library, the API or the cache
    fn enter(&mut self, i: usize) {
        let Some(seat) = self.shown().seats.get(i).cloned() else { return };
        if !self.library.contains_key(&seat.show.id) {
            match self.api.load(&seat.show.id).and_then(|(t, why)| deck_show::document::open(&t).map(|s| (s, why))) {
                Ok((s, why)) => {
                    self.status = why.map_or(format!("{} loaded", seat.show), |w| format!("{} from the cache: {w}", seat.show));
                    self.library.insert(seat.show.id.clone(), s);
                }
                Err(e) => {
                    self.status = format!("{}: {e}", seat.show);
                    return;
                }
            }
        }
        self.path.push(seat.id);
        self.row = 0;
        self.role = None;
    }

    /// Flash (#91 item 5, SHOW.md §4b): the tree from the top, every show its seats pin fetched once (the API, else the
    /// cache), resolved by the core to exact versions (`deck_show::resolve`, like a lock file); then the list to choose
    /// from and the prompt (#471, flash.rs). A broken reference is the core's problem, said here, and nothing goes out.
    fn resolve(&mut self) -> Result<deck_show::Lock, String> {
        let mut todo: Vec<deck_show::Pinned> = self.sub.conductor().show().seats.iter().map(|s| s.show.clone()).collect();
        while let Some(p) = todo.pop() {
            if self.library.contains_key(&p.id) {
                continue;
            }
            if let Some(s) = self.api.load(&p.id).ok().and_then(|(t, _)| deck_show::parse(&t).ok()) {
                todo.extend(s.seats.iter().map(|x| x.show.clone()));
                self.library.insert(p.id.clone(), s);
            }
        }
        let lib: BTreeMap<deck_show::Pinned, Show> = self.library.values().map(|s| (deck_show::Pinned { id: s.id.clone(), version: s.version }, s.clone())).collect();
        deck_show::resolve(self.sub.conductor().show(), &lib).map_err(|problems| format!("not flashed: {}", problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")))
    }

    fn save(&mut self) {
        let show = self.shown().clone();
        self.status = match serde_json::to_value(&show).map_err(|e| e.to_string()).and_then(|v| self.api.save(&show.id, &v)) {
            Ok(v) => format!("saved {} as version {v}", show.id),
            Err(e) => format!("not saved: {e}"),
        };
    }

    fn load(&mut self) {
        let id = self.shown().id.clone();
        match self.api.load(&id).and_then(|(t, why)| deck_show::document::open(&t).map(|s| (s, why))) {
            Ok((s, why)) => {
                self.status = why.map_or(format!("loaded {}@{}", s.id, s.version), |w| format!("{}@{} from the cache: {w}", s.id, s.version));
                if self.path.is_empty() {
                    // a new document at the top: the conductor starts over on it (what plays stays until a row changes it)
                    self.sub = Subtree::new(s, ClockChoice::Parent);
                } else {
                    self.library.insert(id, s);
                }
            }
            Err(e) => self.status = format!("not loaded: {e}"),
        }
    }
}

/// A check run's presses (`--event`, `--fire`, `--launch`, `--go`): what the operator and a reader would do, at fixed
/// frames, so a screenshot shows a fired row and a suggestion.
#[derive(Debug, Clone, Default)]
pub struct Script {
    pub event: Option<String>,
    pub fire: Option<String>,
    pub launch: Option<(String, String)>,
    pub go: bool,
    pub flash: bool,
    /// Show: enter this seat's sub conductor, inspect this row
    pub seat: Option<String>,
    pub row: Option<String>,
    /// Show: the cue sheet or the problems list in the stage's place; a problem's line pressed
    pub sheet: Option<Sheet>,
    pub problem: Option<usize>,
    /// #471: with `flash`, the typed filter applied to the list, the prompt opened, the prompt confirmed
    pub flash_filter: Option<String>,
    pub flash_ask: bool,
    pub flash_confirm: bool,
    /// #478: the role the inspector shows (an outlet id); with `--sheet who`, a client (its name or id) moved to a role
    /// (`NAME=ROLE`, an empty role = no role) by its move menu, and a client's move menu left open
    pub role: Option<String>,
    pub who_move: Option<(String, String)>,
    pub who_menu: Option<String>,
}

impl Host {
    pub fn script(&mut self, frame: u64, s: &Script) {
        let ms = now();
        let rows: Vec<String> = self.sub.conductor().show().rows.iter().map(|r| r.id.clone()).collect();
        let row = |id: &Option<String>| id.as_ref().and_then(|r| rows.iter().position(|x| x == r));
        match frame {
            20 => {
                if let Some(i) = s.seat.as_ref().and_then(|id| self.shown().seats.iter().position(|x| x.id == *id)) {
                    self.press(Hit::Enter(i));
                }
            }
            30 => {
                if let Some(i) = s.row.as_ref().and_then(|id| self.shown().rows.iter().position(|x| x.id == *id)) {
                    self.press(Hit::Row(i));
                }
            }
            40 => {
                if let Some(i) = row(&s.fire) {
                    self.press(Hit::Fire(i));
                }
            }
            70 => {
                let cols = deck_show::view::matrix(&self.sub).columns;
                if let Some((r, c)) = s.launch.as_ref().and_then(|(r, c)| Some((rows.iter().position(|x| x == r)?, cols.iter().position(|x| x.id == *c)?))) {
                    self.press(Hit::Launch(r, c));
                }
            }
            100 => {
                // a reader's event, as the relay hands it on
                if let Some(e) = &s.event {
                    let story = serde_json::json!({ "id": e }).as_object().cloned().unwrap_or_default();
                    let out = self.sub.on_wire(&protocol::Envelope::new("reader", protocol::Message::Story(story)), ms);
                    self.send(out);
                }
            }
            130 if s.go => self.press(Hit::Go),
            140 => {
                if let Some(sheet) = s.sheet {
                    self.press(Hit::Sheet(sheet));
                }
            }
            200 => {
                if let Some(i) = s.problem {
                    self.press(Hit::Problem(i));
                }
            }
            35 => {
                if let Some(i) = s.role.as_ref().and_then(|id| deck_show::roles::roles(self.shown(), None).iter().position(|r| r.id == *id)) {
                    self.press(Hit::Role(i));
                }
            }
            150 => {
                if let Some((who, role)) = &s.who_move {
                    self.script_move(who, role);
                }
            }
            170 => {
                if let Some(i) = s.who_menu.as_ref().and_then(|who| self.script_line(who)) {
                    self.press(Hit::WhoMove(i));
                }
            }
            50 if s.flash => self.press(Hit::Flash),
            60 => {
                if let Some(text) = &s.flash_filter {
                    self.script_filter(text);
                }
            }
            80 if s.flash_ask => self.press(Hit::FlashAsk),
            90 if s.flash_confirm => self.press(Hit::FlashConfirm),
            _ => {}
        }
    }
}

/// The places of a show workspace in a window of `w` × `h` (logical px) under the top bar: where the stage draws.
pub fn stage_rect(ws: Workspace, w: f32, h: f32, t: &deck_kit::tokens::Tokens) -> Rect {
    match ws {
        Workspace::Perform => perform::layout(w, h, t).stage,
        _ => author::layout(w, h, t).stage,
    }
}
