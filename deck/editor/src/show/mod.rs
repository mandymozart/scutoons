//! A Show tab (#91; SHOW.md §3, §7, §10; the owner's drafts m1-show, m2-perform, m3-tree): beside the stage editor, the
//! editor shows a show's conductor. The conductor is deck-show's `Subtree` (RULES A1, A2): the editor hands it the
//! operator's presses and the relay's envelopes with the time, sends what it answers through its relay link (link.rs), and
//! draws what deck-show's view.rs says – the matrix, the log in words, the inspector. Nothing here decides what plays.
//!
//! A Show tab authors and runs at once (Ableton-style: a cell is edited while it plays). Running (live.rs): the rows band
//! is the cue matrix – a row's ▶ fires it, a cell's ▶ launches it by hand (marked off its row) –, and the optional panels
//! Next (the suggestion with GO / Drop, else GO), Stage now and Session (the conductors rail), the Session and Vote sheets
//! (session.rs, vote.rs). Go live joins the relay (the decks, the readers, the vote), End leaves it; the lock refuses
//! every edit of the document while it is on.
//! - **Authoring** (author.rs): the conductors tree with a breadcrumb into a sub conductor, the rows band, the inspector, the
//!   seats, each deck's answer to the last flash; the 2D node overlay over the dimmed stage (#446) with a 2D / 3D switch; the
//!   cue sheet and the problems list in its place (sheets.rs); save and load through /api/shows (api.rs). rows.rs: rows,
//!   columns and groups named in place in the band's heads, columns grouped, a cell's content picked in the cell;
//!   #539 (events.rs): the event a row fires on, picked from the toon's story events. #551 (ask.rs): several cells of a
//!   row selected (shift), the right click's typeahead on them – a new question, the story's, add to, leave –, the
//!   question in the cell inspector, the wide cell in the rows band. The scene list beside the
//!   rows band (scenes.rs): a scene dragged onto a cell is its piece, as a pick is; its "new scene" opens the scene sheet
//!   (new_scene.rs): a scene named, cast from the toon's rigs, played, saved as a draft. The Demos sheet (demos.rs) opens a
//!   demo show in place of the show.
//!
//! The editor runs one conductor, this deck's (the tree's top it was given). A sub conductor runs on its own deck with its
//! own operator; entering it here shows its document, it does not run it.
pub mod api;
mod ask;
pub mod author;
pub mod band;
pub mod build;
pub mod cue;
pub(crate) mod condition;
pub mod demos;
mod events;
pub mod nodes;
mod node_drop;
// #763: the node view's painting is the Spores widget's (deck-spores), shared with the creator's Spores tab
use deck_spores::paint as node_paint;
mod node_pick;
mod node_wire;
pub(crate) mod flash;
mod link;
pub mod live;
pub mod new_scene;
mod roles;
mod rows;
pub mod saving;
pub mod scenes;
mod session;
mod sheets;
mod sizing;
mod toons;
pub(crate) mod values;
mod vote;
pub mod wiring;

use deck_kit::editor::controls::Rect;
use deck_show::{ClockChoice, Show, Subtree};
use deck_ui::session::Place;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What the front tab draws with (tabs.rs): the stage editor of a Scene tab and of a character's Asset tab, a module's
/// Asset tab, a Show tab; Home when no tab is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Stage,
    Show,
    /// a module's code, build and audition (crate::module)
    Module,
    /// a video, image, sound or 3D set on its own (crate::media)
    Media,
    Home,
}

impl Workspace {
    /// a check run's `--workspace`: build | module | show | home, the names before the tabs
    pub fn parse(s: &str) -> Option<Workspace> {
        match s.to_ascii_lowercase().as_str() {
            "build" | "stage" | "scene" | "asset" => Some(Workspace::Stage),
            "module" => Some(Workspace::Module),
            "show" => Some(Workspace::Show),
            "home" => Some(Workspace::Home),
            _ => None,
        }
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
    /// #531: the session offered to a linked reader – the link, its QR, the linked readers (session.rs)
    Session,
    /// #533: the vote's conductor side – open a vote, its tally per group, Hold / Close now / Override, the result (vote.rs)
    Vote,
    /// the slot's Find of the inspected row: what can fire it, under its ⚡ slot (events.rs)
    Event,
    /// a new scene, or one from the server recast (new_scene.rs)
    Scene,
    /// the demo shows to open (demos.rs)
    Demos,
    /// a new show: its name and its toon (toons.rs)
    NewShow,
}

impl Sheet {
    pub fn parse(s: &str) -> Option<Sheet> {
        match s {
            "cues" => Some(Sheet::Cues),
            "problems" => Some(Sheet::Problems),
            "flash" => Some(Sheet::Flash),
            "who" => Some(Sheet::Who),
            "session" => Some(Sheet::Session),
            "vote" => Some(Sheet::Vote),
            "event" => Some(Sheet::Event),
            "scene" => Some(Sheet::Scene),
            "demos" => Some(Sheet::Demos),
            "new-show" => Some(Sheet::NewShow),
            _ => None,
        }
    }
}

/// what a press in a show workspace means
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// a row's ▶
    Fire(usize),
    /// a cell's small ▶: row, column
    Launch(usize, usize),
    /// a column's stop (the matrix's order)
    Stop(usize),
    Go,
    Drop,
    /// a seat of the document shown: enter its sub conductor (Show)
    Enter(usize),
    /// a step of the breadcrumb, 0 = the top
    Crumb(usize),
    /// a row of the document shown: the inspector shows it (Show)
    Row(usize),
    /// a column's head (the matrix's order): it alone is selected; with shift or ctrl it joins the selection
    Column(usize),
    ColumnAlso(usize),
    /// a group's head (the matrix's groups): the group and its columns are selected
    Group(usize),
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
    /// #537: a new row after the selected one, the selected row's name field, its remove (asks first)
    RowAdd,
    RowName,
    RowRemove,
    /// a port on a cell's foot in the rows band: row, column (the matrix's), outlet or inlet, its index (deck-show
    /// wiring.rs `ports`): pressed, a wire is taken in the hand
    Port(usize, usize, bool, usize),
    /// a cell of the document shown (row, column as the matrix counts): the cell inspector shows it; one of its mode
    /// chips; its piece field (opens the cell's typeahead)
    Cell(usize, usize),
    /// #551: a cell pressed with shift: its column joins the selection of the inspected row (or leaves it); a field, chip
    /// or toggle of the selected cell's question (ask.rs)
    CellAlso(usize, usize),
    Ask(ask::AskHit),
    CellMode(usize),
    CellPiece,
    /// the cell's camera shot field: the shot picker opens in place
    CellShot,
    /// #553: one chip of the selected cell's filter (the one condition control); an address of the inspected row (selects
    /// it: its filter's chips show under it); one chip of the selected address's filter
    CellFact(usize),
    AddressPick(usize),
    AddressFact(usize),
    /// a row's ⚡ slot (opens Find under it); a line of Find; its typed field; a scope pill; its ground (takes the press)
    Slot(usize),
    EventPick(usize),
    EventType,
    EventScope(usize),
    EventGround,
    /// the show's stage background: a click on its value types it, on its swatch opens the colour picker (the editor's)
    Background,
    BackgroundColour,
    Load,
    /// the tree resolved to exact versions: the list of the devices to flash opens (#471)
    Flash,
    /// #471, the flash list: a line's tick, a quick filter, the typed filter's field, the lock, forced or a suggestion
    /// (#481), on to the prompt, the prompt confirmed, back / cancel
    FlashTick(usize),
    FlashQuick(usize),
    FlashType,
    FlashLock,
    FlashForce,
    FlashAsk,
    FlashConfirm,
    FlashCancel,
    /// the overlay: 2D (true) or the stage alone (3D)
    View(bool),
    /// the node view: its direction, framing everything
    /// a node, a group, a note or a button of the node view (nodes.rs)
    Nodes(nodes::NodeHit),
    /// the cue sheet or the problems list in the stage's place
    Sheet(Sheet),
    /// a line of the problems list: select what it points at
    Problem(usize),
    /// #531, the Session sheet: a toon, a chapter of it, a label of it, Copy link, Open here
    SessionToon(usize),
    SessionChapter(usize),
    SessionLabel(usize),
    SessionCopy,
    SessionOpen,
    /// a live camera of the list picked (again: the list) – the QR becomes its own
    SessionCamera(usize),
    /// #533, the Vote sheet: a field, chip or button of the form or the panel (vote.rs)
    Vote(vote::VoteHit),
    /// the selected cell's values: a knob, its ×, take current (values.rs)
    Knob(values::KnobHit),
    /// #630: the Session panel's cue mix (cue.rs): a press takes it in the hand
    CueMix,
    /// an edge of the rows band: dragged it resizes, a double click fits it (sizing.rs)
    Edge(deck_kit::editor::table::Edge),
    /// the scene sheet's fields, buttons and lines (new_scene.rs)
    Compose(new_scene::ComposeHit),
    /// a demo of the Demos sheet
    Demo(usize),
    /// the new show's name field; the toon picker's field and a toon it found (toons.rs)
    NewName,
    ToonFind,
    ToonPick(usize),
    /// #708: a toon of the show's toons row, taken in or out
    ShowToon(usize),
}

impl Hit {
    /// the same press with shift (or ctrl) held: a cell, a column joins the selection
    pub fn with_shift(self) -> Hit {
        match self {
            Hit::Cell(r, c) => Hit::CellAlso(r, c),
            Hit::Column(c) => Hit::ColumnAlso(c),
            Hit::Nodes(nodes::NodeHit::Node(i)) => Hit::Nodes(nodes::NodeHit::NodeAlso(i)),
            other => other,
        }
    }
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
    /// #537, #551: the columns whose cells of the inspected row are selected (a set within one row; shift adds one); the
    /// cell inspector shows the last (`cell()`)
    pub cells: Vec<String>,
    /// the columns whose heads are selected (shift or ctrl adds one), and the group whose head is
    pub columns: Vec<String>,
    pub group: Option<String>,
    /// #551: the right click's typeahead on the selected cells while it is open (ask.rs)
    pub context: Option<ask::Context>,
    /// the Show tab's floating panels; the toolbar's place for the view switch; what the info says
    pub show_panels: crate::panels::Panels,
    /// where the rows band is scrolled, and the band as drawn last for the wheel
    pub show_scroll: deck_kit::editor::table::Scroll,
    band: Option<deck_kit::editor::table::Laid>,
    /// the band's sizes per show, as this viewer left them, the cache file they are kept in, an edge in the hand, the
    /// last press on an edge (for a double click) and what the content asked for as drawn (sizing.rs)
    pub sizes: sizing::Views,
    pub views_file: Option<std::path::PathBuf>,
    pub resizing: Option<sizing::Resize>,
    last_edge: Option<(deck_kit::editor::table::Edge, std::time::Instant)>,
    band_fits: sizing::Fits,
    pub help: Option<deck_kit::editor::info::Help>,
    /// #553: the address of the inspected row (its index) whose filter's chips show
    pub address: Option<usize>,
    /// #537: the toon's address book (the editor's, main.rs) – its scenes are what a cell's piece is picked from – and why
    /// there is none
    pub book: Option<std::sync::Arc<deck_space::rig::book::Book>>,
    /// the camera shots the editor knows per scene (`scene:<id>` → id and its words): Build's open scene's
    pub cameras: BTreeMap<String, Vec<(String, String)>>,
    /// the live cameras each scene the editor knows offers to phones (`scene:<id>` → its cameras): Build's open scene's
    pub live_cameras: BTreeMap<String, Vec<protocol::LiveCamera>>,
    /// the phones' pose, Record and Recenter heard this frame (Build's DOP panel records a take from them)
    pub hands: Vec<protocol::Envelope>,
    pub book_why: Option<String>,
    /// the library card in the hand: the piece a cell would take (the cell or node under the pointer shows it), else
    /// the core's sentence why Show takes no such card; the core's refusal where one was dropped
    pub carried: Option<Result<String, String>>,
    pub refused: Option<(f32, f32)>,
    /// #539: the toon whose story events a row's event is picked from (`--toon`; else the book's), the events once asked
    /// (`GET /api/creator/story-events`, read by deck-show story.rs) or why there are none, the picker's typed query;
    /// #551: the same answer's questions are what the right click offers as the story's
    pub toon: Option<String>,
    pub story: Option<Result<deck_show::story::Story, String>>,
    pub event_query: String,
    /// how wide the slot's Find looks, and where the open slot was drawn (Find opens under it)
    pub event_scope: deck_show::inlet::Scope,
    pub slot_at: Option<Rect>,
    /// #478: a typed field open (a role's name) and its text
    pub typing: Option<(roles::Typing, deck_kit::base::text::Field)>,
    /// #478: the prompt asked before it is done (a role's remove, a kick)
    pub asking: Option<roles::Ask>,
    /// #478: the move menu open for a client (by its id), and whether a move from it locks
    pub menu: Option<String>,
    pub lock: bool,
    /// the live night's lock (live.rs): every edit of the document is refused while it is on – not the deck's editor lock
    pub locked: bool,
    /// Go live was confirmed over another live show: the editor ends that one and this one goes live (live.rs)
    pub live_asked: bool,
    /// the last thing done, or why not: the editor takes it into its messages each frame (main.rs)
    pub status: String,
    /// a named show that did not load: why (no tab opens for it; Home says so)
    pub unloaded: Option<String>,
    /// the status is a refusal (a warning in the messages)
    pub bad: bool,
    pub two_d: bool,
    /// the node view's zoom, pan and direction
    pub nodes: nodes::NodeView,
    /// a wire between two cells in the hand (wiring.rs)
    pub wiring: Option<wiring::Wiring>,
    /// the seconds the beads on the wires have run; none: they rest out of sight (Signal flow on wires off, Reduce motion)
    pub flow: Option<f32>,
    /// the sheet shown in the stage's place (Show), none = the stage
    pub sheet: Option<Sheet>,
    /// the seat a problem selected (marked in the tree)
    pub seat: Option<usize>,
    /// this frame's places
    pub hits: Vec<(Rect, Hit)>,
    /// #471: the flash list while it is open
    pub flashing: Option<flash::Flashing>,
    /// each show document edited saves itself (saving.rs), by its id
    pub saves: BTreeMap<String, crate::autosave::Autosave<String>>,
    /// scenes with something unsent that are open nowhere, by id: they go up from here (saving.rs)
    pub unsent: BTreeMap<String, (deck_space::scene_document::SceneDocument, crate::autosave::Autosave<deck_space::scene_document::SceneDocument>)>,
    /// #531: the Session sheet's place and lists
    pub session: session::Session,
    /// the library's toons for the toon picker, its query, the new show's name
    pub toons: toons::Toons,
    /// #533: what the Vote sheet keeps
    pub voting: vote::Voting,
    /// the knobs of the cells' pieces, the knob being dragged, where the pointer is (values.rs)
    pub knobs: values::Loader,
    pub knob_drag: Option<values::KnobDrag>,
    pub cursor: (f32, f32),
    /// #630, the cue (cue.rs): the mix, its drag and its bar's width; a pre-listen asked of the stage, what is on the cue
    /// (the editor says), why nothing is heard there
    pub cue_mix: f32,
    pub mix_drag: Option<cue::MixDrag>,
    pub mix_width: f32,
    pub to_prelisten: Option<cue::Ask>,
    pub prelistening: Option<String>,
    pub cue_note: Option<String>,
    /// #568: Build linked to a cell, and what Build is asked to play next (build.rs)
    pub build: Option<build::BuildLink>,
    pub to_build: Option<build::ToBuild>,
    /// the scene sheet's state, and a scene it asks the stage to play (new_scene.rs)
    pub compose: new_scene::Composer,
    pub to_stage: Option<deck_space::scene::Scene>,
    /// the Demos sheet's list; the book file of the demo opened, for the editor's book to take (main.rs)
    pub demos: Vec<demos::Demo>,
    pub demo_book: Option<PathBuf>,
    link: Option<link::Link>,
    api: api::Api,
}

/// the relay's clock: ms since the epoch (a show's moments are relay ms)
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// the new show when none is named: one role, no row yet (#537: rows are added in the Show workspace)
const EMPTY: &str = r#"{ "id": "new-show", "version": 1, "outlets": [{ "id": "left" }], "rows": [] }"#;

impl Host {
    /// The show `file` holds (a document's JSON), else show `id` from the API / cache, else an empty one; joined to `relay`
    /// as its conductor when one is named.
    pub fn new(file: Option<PathBuf>, id: Option<String>, relay: Option<String>, api: String) -> Host {
        let api = api::Api::new(api);
        let mut status = String::new();
        let mut version = None;
        let text = match (&file, &id) {
            (Some(f), _) => std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())),
            (None, Some(id)) => api.load(id).map(|l| {
                if let Some(w) = l.why {
                    status = format!("from the cache: {w}");
                }
                version = l.version;
                l.text
            }),
            (None, None) => Ok(EMPTY.to_string()),
        };
        // a document with problems opens: the problems list says what, the status bar counts them (#91)
        let mut unloaded = None;
        let show = match text.and_then(|t| deck_show::document::open(&t)) {
            Ok(s) => s,
            Err(e) => {
                unloaded = (file.is_some() || id.is_some()).then(|| e.clone());
                status = e;
                deck_show::document::open(EMPTY).expect("the empty show has its shape")
            }
        };
        let link = relay.and_then(|r| {
            link::Link::start(&r, std::env::var("SCUTOONS_LIVE_TOKEN").ok())
                .map_err(|e| {
                    status = format!("relay: {e}");
                })
                .ok()
        });
        let mut show_panels = crate::panels::Panels::new(&crate::panels::SHOW);
        show_panels.floats.scrolls_itself("rows");
        show_panels.floats.scrolls_itself("nodes");
        let scroll = deck_kit::editor::table::Scroll::default();
        let saves = version.map(|v| (show.id.clone(), crate::autosave::Autosave::new(Some(v)))).into_iter().collect();
        let mut host = Host { sub: Subtree::new(show, ClockChoice::Parent), library: BTreeMap::new(), path: Vec::new(), row: 0, role: None, cells: Vec::new(), columns: Vec::new(), group: None, context: None, show_panels, show_scroll: scroll, band: None, sizes: BTreeMap::new(), views_file: None, resizing: None, last_edge: None, band_fits: sizing::Fits::default(), help: None, address: None, book: None, cameras: BTreeMap::new(), live_cameras: BTreeMap::new(), hands: Vec::new(), book_why: None, carried: None, refused: None, toon: None, story: None, event_query: String::new(), event_scope: Default::default(), slot_at: None, typing: None, asking: None, menu: None, lock: false, locked: false, live_asked: false, status, unloaded, bad: false, wiring: None, flow: None, two_d: true, sheet: None, seat: None, hits: Vec::new(), flashing: None, saves, unsent: BTreeMap::new(), session: session::Session::default(), toons: toons::Toons::default(), voting: vote::Voting::default(), knobs: values::Loader::new(api.base.clone()), knob_drag: None, cursor: (0.0, 0.0), cue_mix: deck_engine::cue::MIX, mix_drag: None, mix_width: 1.0, to_prelisten: None, prelistening: None, cue_note: None, build: None, to_build: None, compose: new_scene::Composer::default(), to_stage: None, demos: Vec::new(), demo_book: None, link, api, nodes: nodes::NodeView::default() };
        host.adopt_kept();
        host
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
    /// why not: the status, as a refusal
    /// a library card a cell would take is in the hand
    pub(crate) fn carrying(&self) -> bool {
        matches!(self.carried, Some(Ok(_)))
    }

    pub(crate) fn refuse(&mut self, why: String) {
        self.status = why;
        self.bad = true;
    }

    /// the relay this show is joined to, if any: the show live
    pub fn relay(&self) -> Option<&str> {
        self.link.as_ref().map(|l| l.relay.as_str())
    }

    pub fn link_line(&self) -> String {
        self.link.as_ref().map_or("not live: Go live joins the relay".into(), |l| l.line.lock().unwrap().clone())
    }

    /// the live cameras scene `scene` offers to phones (its DOP panel's access rows): the session hears every scene's
    /// when they changed
    pub fn offer_cameras(&mut self, scene: String, cameras: Vec<protocol::LiveCamera>) {
        if self.live_cameras.get(&scene) == Some(&cameras) {
            return;
        }
        self.live_cameras.insert(scene, cameras);
        let all = self.live_cameras.values().flatten().cloned().collect();
        let out = self.sub.offer_cameras(all);
        self.send(out);
    }

    /// who holds live camera `camera` now (its client id), as the relay last listed it; none while not live
    pub fn holder_of(&self, camera: &str) -> Option<String> {
        let l = self.link.as_ref()?;
        let list = l.cameras.lock().unwrap();
        list.iter().find(|c| c.id == camera).and_then(|c| c.holder.clone())
    }

    /// the live camera client `client` holds now, if any
    pub fn held_by(&self, client: &str) -> Option<String> {
        let l = self.link.as_ref()?;
        let list = l.cameras.lock().unwrap();
        list.iter().find(|c| c.holder.as_deref() == Some(client)).map(|c| c.id.clone())
    }

    fn send(&mut self, out: Vec<protocol::Envelope>) {
        if let Some(l) = &self.link {
            l.send(out);
        }
    }

    /// Each frame: the relay's envelopes and the clock to the core, what it answers out.
    pub fn frame(&mut self) {
        let ms = now();
        self.knobs.poll();
        self.compose_poll();
        self.toons_poll();
        let mut out = self.sub.tick(ms);
        if let Some(l) = &self.link {
            self.hands.clear();
            while let Ok(e) = l.inbox.try_recv() {
                if matches!(e.message, protocol::Message::HandPose(_) | protocol::Message::HandRecord(_) | protocol::Message::HandRecenter(_)) {
                    self.hands.push(e.clone());
                }
                out.extend(self.sub.on_wire(&e, ms));
            }
        }
        // #471: the open flash list follows what the devices say (not under the prompt: what it says is what goes)
        if let Some(f) = self.flashing.as_mut().filter(|f| !f.asking) {
            f.choice.refresh(&self.sub);
        }
        self.send(out);
    }

    /// The wheel over the rows band drawn last (`steps` across, down): whether it scrolled.
    pub fn band_wheel(&mut self, x: f32, y: f32, steps: (f32, f32)) -> bool {
        let Some(laid) = self.band.as_mut() else { return false };
        if !laid.area.has(x, y) {
            return false;
        }
        let Some(s) = laid.wheel(steps) else { return false };
        self.show_scroll = s;
        laid.scroll = s;
        true
    }

    pub(super) fn drew_band(&mut self, laid: deck_kit::editor::table::Laid, fits: sizing::Fits) {
        self.band = Some(laid);
        self.band_fits = fits;
    }

    /// The panic pill: the core's panic down the tree, or its resume when one holds; joined or not, the core keeps the state.
    pub fn panic_pill(&mut self) {
        let ms = now();
        let out = if self.sub.panicked() { self.sub.resume(ms) } else { self.sub.panic(ms) };
        self.status = if self.sub.panicked() { "PANIC – resume on the red pill".into() } else { "resumed".into() };
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
        self.context = None;
        self.refused = None;
        // the slot's Find goes with any press but its own
        if self.sheet == Some(Sheet::Event) && !matches!(hit, Hit::Slot(_) | Hit::EventPick(_) | Hit::EventType | Hit::EventScope(_) | Hit::EventGround) {
            self.events_close();
        }
        if self.locked && live::edits(hit) {
            return self.refuse(live::LOCKED.into());
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
            Hit::Stop(c) => {
                let m = deck_show::view::matrix(&self.sub);
                if let Some(col) = m.columns.get(c) {
                    let out = self.sub.stop_column(&col.id.clone(), ms);
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
                self.address = None;
                self.columns.clear();
                self.group = None;
            }
            Hit::Role(i) => {
                self.role = deck_show::roles::roles(self.shown(), None).get(i).map(|r| r.id.clone());
                self.typing = None;
            }
            Hit::RoleAdd | Hit::RoleName | Hit::RoleFact(_) | Hit::RoleRemove | Hit::AskYes | Hit::AskNo => self.roles_press(hit),
            Hit::WhoMove(_) | Hit::WhoRename(_) | Hit::WhoKick(_) | Hit::MenuPick(_) => self.who_press(hit),
            Hit::Ask(h) => self.ask_press(h),
            Hit::RowAdd | Hit::RowName | Hit::RowRemove | Hit::Cell(..) | Hit::CellAlso(..) | Hit::CellMode(_) | Hit::CellPiece | Hit::CellShot | Hit::Column(_) | Hit::ColumnAlso(_) | Hit::Group(_) | Hit::CellFact(_) | Hit::AddressPick(_) | Hit::AddressFact(_) => {
                self.rows_press(hit);
            }
            Hit::Slot(_) | Hit::EventPick(_) | Hit::EventType | Hit::EventScope(_) | Hit::EventGround => self.events_press(hit),
            // the editor's colour picker takes it before the show (colour.rs)
            Hit::BackgroundColour => {}
            Hit::Background => {
                let now = self.shown().background.as_ref().map_or("", |b| b.text()).to_string();
                self.typing = Some((roles::Typing::Background, deck_kit::base::text::Field::with(&now)));
            }
            Hit::Load => self.load(),
            Hit::Flash => self.open_flash(),
            Hit::FlashTick(_) | Hit::FlashQuick(_) | Hit::FlashType | Hit::FlashLock | Hit::FlashForce | Hit::FlashAsk | Hit::FlashConfirm | Hit::FlashCancel => self.flash_press(hit),
            Hit::Nodes(_) => self.nodes_press(hit),
            Hit::View(two_d) => {
                self.two_d = two_d;
                self.sheet = None;
                self.flashing = None;
            }
            Hit::Sheet(s) => {
                self.sheet = Some(s);
                self.flashing = None;
                if s == Sheet::Session {
                    self.session_open();
                }
                if s == Sheet::Event {
                    self.events_open();
                }
                if s == Sheet::Demos {
                    self.demos_open();
                }
                if s == Sheet::NewShow {
                    self.new_show_open();
                }
            }
            Hit::Problem(i) => self.select_problem(i),
            Hit::SessionToon(_) | Hit::SessionChapter(_) | Hit::SessionLabel(_) | Hit::SessionCopy | Hit::SessionOpen | Hit::SessionCamera(_) => self.session_press(hit),
            Hit::Vote(v) => self.vote_press(v),
            Hit::Knob(k) => self.knob_press(k),
            Hit::CueMix => self.mix_press(),
            Hit::Edge(e) => self.edge_press(e),
            Hit::Port(r, c, out, k) => self.port_press(r, c, out, k),
            Hit::Compose(c) => self.compose_press(c),
            Hit::Demo(i) => self.demo_press(i),
            Hit::NewName | Hit::ToonFind | Hit::ToonPick(_) | Hit::ShowToon(_) => self.toons_press(hit),
        }
    }

    /// A key in a Show tab: the open typeahead or typed field first, then the tab's keys – ctrl+r, ctrl+g, Enter, Delete
    /// (rows.rs); none that edits while the lock is on. True = taken.
    pub fn key(&mut self, key: &winit::keyboard::Key, ctrl: bool) -> bool {
        if self.type_key(key) {
            return true;
        }
        if !ctrl && *key == winit::keyboard::Key::Character("f".into()) && self.nodes_frame() {
            return true;
        }
        if self.locked || self.asking.is_some() {
            return false;
        }
        self.nodes_key(key, ctrl) || self.show_key(key, ctrl)
    }

    /// a problem's line pressed: its row, cell or seat is selected (deck-show's `Points`)
    fn select_problem(&mut self, i: usize) {
        let list = self.problems();
        let Some(p) = list.get(i) else { return };
        self.seat = None;
        match p.points {
            deck_show::view::Points::Row(r) | deck_show::view::Points::Cell(r, _) => self.row = r,
            deck_show::view::Points::Seat(s) => self.seat = Some(s),
            deck_show::view::Points::Show => {}
        }
        self.status = p.text.clone();
    }

    /// The problems list of the document shown: deck-show's (the document's errors, the decks' flash reports), then – once
    /// the toon's story questions are here – a story question whose published choice changed (#550 `story::changed`).
    pub(crate) fn problems(&self) -> Vec<deck_show::view::ProblemView> {
        let show = self.shown();
        // the flash reports belong to the conductor this editor runs: the top
        let mut list = deck_show::view::problems(show, self.path.is_empty().then_some(&self.sub));
        if let (Some(toon), Some(Ok(story))) = (self.toon_now(), &self.story) {
            list.extend(deck_show::story::changed(show, &toon, &story.choices));
        }
        list
    }

    /// into the sub conductor of the shown document's seat `i`: its document from the library, the API or the cache
    fn enter(&mut self, i: usize) {
        let Some(seat) = self.shown().seats.get(i).cloned() else { return };
        if !self.library.contains_key(&seat.show.id) {
            match self.api.load(&seat.show.id).and_then(|l| deck_show::document::open(&l.text).map(|s| (s, l))) {
                Ok((s, l)) => {
                    self.status = l.why.map_or(format!("{} loaded", seat.show), |w| format!("{} from the cache: {w}", seat.show));
                    self.saves.insert(seat.show.id.clone(), crate::autosave::Autosave::new(l.version));
                    self.library.insert(seat.show.id.clone(), s);
                }
                Err(e) => {
                    self.refuse(format!("{}: {e}", seat.show));
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
            if let Some((s, version)) = self.api.load(&p.id).ok().and_then(|l| deck_show::parse(&l.text).ok().map(|s| (s, l.version))) {
                todo.extend(s.seats.iter().map(|x| x.show.clone()));
                self.saves.insert(p.id.clone(), crate::autosave::Autosave::new(version));
                self.library.insert(p.id.clone(), s);
            }
        }
        let lib: BTreeMap<deck_show::Pinned, Show> = self.library.values().map(|s| (deck_show::Pinned { id: s.id.clone(), version: s.version }, s.clone())).collect();
        deck_show::resolve(self.sub.conductor().show(), &lib).map_err(|problems| format!("not flashed: {}", problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")))
    }

    /// the shown document's newest again (a revert stored it)
    pub fn reload(&mut self) {
        self.load();
    }

    /// the server's newest of the shown document; a local copy not sent yet stays open, one in conflict gives way
    fn load(&mut self) {
        let id = self.shown().id.clone();
        if self.api.outbox.get("show", &id).is_some_and(|e| !e.conflict) {
            self.status = format!("{id}: unsent – kept locally, it goes up first");
            return;
        }
        match self.api.load(&id).and_then(|l| deck_show::document::open(&l.text).map(|s| (s, l))) {
            Ok((s, l)) => {
                if l.why.is_none() {
                    self.api.outbox.remove("show", &id);
                }
                self.saves.insert(id.clone(), crate::autosave::Autosave::new(l.version));
                self.status = l.why.map_or(format!("loaded {}@{}", s.id, s.version), |w| format!("{}@{} from the cache: {w}", s.id, s.version));
                if self.path.is_empty() {
                    // a new document at the top: the conductor starts over on it (what plays stays until a row changes it)
                    self.sub = Subtree::new(s, ClockChoice::Parent);
                } else {
                    self.library.insert(id, s);
                }
            }
            Err(e) => self.refuse(format!("not loaded: {e}")),
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
    /// a column stopped from its head, after the launch
    pub stop: Option<String>,
    pub go: bool,
    pub flash: bool,
    /// Show: enter this seat's sub conductor, inspect this row
    pub seat: Option<String>,
    pub row: Option<String>,
    /// Show: the cue sheet or the problems list in the stage's place; a problem's line pressed
    pub sheet: Option<Sheet>,
    pub problem: Option<usize>,
    /// #471: with `flash`, the typed filter applied to the list, the prompt opened, the prompt confirmed; #481: the
    /// flash switched to forced
    pub flash_filter: Option<String>,
    pub flash_ask: bool,
    pub flash_confirm: bool,
    pub flash_force: bool,
    /// #478: the role the inspector shows (an outlet id); with `--sheet who`, a client (its name or id) moved to a role
    /// (`NAME=ROLE`, an empty role = no role) by its move menu, and a client's move menu left open
    pub role: Option<String>,
    pub who_move: Option<(String, String)>,
    pub who_menu: Option<String>,
    /// #531: with `--sheet session`, the readers a check run names (the first drives; the rig's character joined) and
    /// the place picked (`TOON,CHAPTER[,LABEL]`)
    pub linked: Vec<String>,
    /// with `--sheet session`, the live cameras a check run names (`ID=HOLDER`, `~` away; none: free)
    pub cameras: Vec<(String, Option<String>)>,
    /// with `--sheet session`, the live camera picked in the list (its id): its own QR
    pub camera_pick: Option<String>,
    /// the character the first reader joined with (the editor's rig)
    pub character: String,
    pub reader_place: Option<Place>,
    /// #533, #551: with `launch`, the cell's ▶ opens its question's vote on the Vote sheet, and this tally the relay would
    /// send (`left=10/0/0,right=0/1/0` per group, or `10/2/0`) is fed to the conductor as if its relay said them
    pub vote_tally: Option<String>,
    /// #537: a row authored as the operator would (`ROW,COLUMN[,PIECE]`): added and named when the document lacks it, its
    /// cell on the column selected, the piece picked from the book's list (or typed); no piece: `play` opens the picker
    pub author: Option<(String, String, Option<String>)>,
    /// #553: a cell selected (`ROW,COLUMN`): the cell inspector shows it, its filter's chips too; #551: more columns of
    /// the row selected with it (`ROW,COLUMN+COLUMN`)
    pub cell: Option<(String, String)>,
    /// with `cell`, a wire taken from its outlet of this name and held over the first inlet that takes it (wiring.rs)
    pub wiring: Option<String>,
    /// #551: the right click on the selected cells – the typeahead opened, this typed, the first line found picked if asked
    pub question: Option<(String, bool)>,
    /// #568: with `cell`, "open in Build" pressed; then "values → cell" in Build
    pub open_build: bool,
    pub to_cell: bool,
    /// the scene sheet filled as typed and picked (`--new-scene`), then played if asked
    pub new_scene: Option<NewScene>,
    /// edited in place: `row` the selected row's name, `cell` the selected cell's content, `shot` its camera shot,
    /// `group:NAME` that group selected
    pub in_place: Option<String>,
    /// the shots a scene's camera has, as Build's open scene would tell (`scene:<id>=wide+close`)
    pub known_shots: Option<(String, Vec<String>)>,
    /// the node view: these nodes selected (the last focused), then F over the canvas frames them
    pub node_select: Vec<String>,
    pub node_frame: bool,
    /// a box dragged on the canvas, from and to as fractions of it, held (not let go) for the screenshot
    pub node_box: Option<[f32; 4]>,
    /// the value of the n-th wire that has one pinned by a click
    pub node_pin: Option<usize>,
    /// a wiring gesture in the node view (node_wire.rs `script_node_wire`)
    pub node_wire: Option<String>,
    /// an edge of the rows band (`column:ID`, `row:ID`, `heads`) taken and moved by this many pixels, held for the
    /// screenshot (sizing.rs)
    pub band_drag: Option<(String, f32)>,
}

/// `ID,NAME,FILLER+FILLER[,play]`
#[derive(Debug, Clone, Default)]
pub struct NewScene {
    pub id: String,
    pub name: String,
    pub cast: Vec<String>,
    pub play: bool,
    /// opened in Build instead
    pub build: bool,
}

impl NewScene {
    pub fn parse(v: &str) -> Option<NewScene> {
        let p: Vec<&str> = v.split(',').map(str::trim).collect();
        let cast = p.get(2)?.split('+').map(str::trim).filter(|f| !f.is_empty()).map(String::from).collect();
        Some(NewScene { id: p.first()?.to_string(), name: p.get(1)?.to_string(), cast, play: p.get(3) == Some(&"play"), build: p.get(3) == Some(&"build") })
    }
}

impl Host {
    pub fn script(&mut self, frame: u64, s: &Script) {
        let ms = now();
        let rows: Vec<String> = self.sub.conductor().show().rows.iter().map(|r| r.id.clone()).collect();
        let row = |id: &Option<String>| id.as_ref().and_then(|r| rows.iter().position(|x| x == r));
        match frame {
            25 => {
                if let Some((row, column, piece)) = &s.author {
                    self.script_author(row, column, piece.as_deref());
                }
            }
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
            95 => self.script_nodes(&s.node_select, s.node_frame),
            96 => self.script_pick(s.node_box, s.node_pin),
            98 | 104 => {
                if let Some(spec) = &s.node_wire {
                    self.script_node_wire(spec, frame == 104);
                }
            }
            97 => {
                if let Some((edge, by)) = &s.band_drag {
                    self.script_band_drag(edge, *by);
                }
            }
            70 => {
                let cols = deck_show::view::matrix(&self.sub).columns;
                if let Some((r, c)) = s.launch.as_ref().and_then(|(r, c)| Some((rows.iter().position(|x| x == r)?, cols.iter().position(|x| x.id == *c)?))) {
                    match (&s.vote_tally, &s.launch) {
                        (Some(tally), Some((row, column))) => self.script_vote(row, column, tally),
                        _ => self.press(Hit::Launch(r, c)),
                    }
                }
            }
            75 => {
                let cols = deck_show::view::matrix(&self.sub).columns;
                if let Some(c) = s.stop.as_ref().and_then(|id| cols.iter().position(|x| x.id == *id)) {
                    self.press(Hit::Stop(c));
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
            32 => {
                let plan = deck_show::view::plan(self.shown());
                let Some((r, columns)) = s.cell.as_ref().and_then(|(r, c)| Some((plan.rows.iter().position(|x| x.id == *r)?, c))) else { return };
                for (n, c) in columns.split('+').filter_map(|c| plan.columns.iter().position(|x| x.id == c.trim())).enumerate() {
                    self.press(if n == 0 { Hit::Cell(r, c) } else { Hit::CellAlso(r, c) });
                }
            }
            38 if s.open_build => {
                let (row, column) = self.selected();
                if let Some(p) = row.zip(column).and_then(|(r, c)| deck_show::rows::cell_inspector(self.shown(), &r, &c)).and_then(|i| i.knobs_of) {
                    self.knobs.now(&p);
                }
                self.press(Hit::Knob(values::KnobHit::Build));
            }
            41 => {
                let Some(out) = s.wiring.as_deref() else { return };
                let columns = deck_show::view::plan(self.shown()).columns;
                let at = self.hits.iter().find_map(|(_, h)| match *h {
                    Hit::Port(r, c, true, k) if r == self.row => {
                        let column = &columns.get(c)?.id;
                        let cell = self.shown().rows.get(r)?.cells.get(column)?;
                        (deck_show::wiring::ports(self.shown(), column, cell).1.get(k)?.label == out).then_some(*h)
                    }
                    _ => None,
                });
                if let Some(h) = at {
                    self.press(h);
                }
            }
            43 => {
                let Some(w) = self.wiring.clone() else { return };
                let from = deck_show::view::plan(self.shown()).columns.iter().position(|c| c.id == w.held.from);
                if let Some((r, _)) = self.hits.iter().rev().find(|(_, h)| matches!(*h, Hit::Port(_, c, false, _) if Some(c) != from)) {
                    self.cursor = (r.x + r.w * 0.5, r.y + r.h * 0.5);
                }
            }
            // #551: a frame later, when the rows band has drawn the selection's places
            34 => {
                if let Some((query, pick)) = &s.question {
                    self.script_question(query, *pick);
                }
            }
            36 => match s.in_place.as_deref() {
                Some("row") => self.press(Hit::RowName),
                Some("cell") => self.open_pieces(),
                Some("shot") => {
                    if let Some((scene, shots)) = &s.known_shots {
                        self.cameras.insert(scene.clone(), shots.iter().map(|id| (id.clone(), String::new())).collect());
                    }
                    self.open_shots();
                }
                Some(g) => {
                    let groups = deck_show::view::plan(self.shown()).groups;
                    if let Some(i) = g.strip_prefix("group:").and_then(|n| groups.iter().position(|x| x.name == n)) {
                        self.press(Hit::Group(i));
                    }
                }
                None => {}
            },
            35 => {
                if s.camera_pick.is_some() {
                    self.session.camera.clone_from(&s.camera_pick);
                }
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
            145 => {
                if !s.linked.is_empty() {
                    self.session_demo(&s.linked, &s.character);
                }
                if !s.cameras.is_empty() {
                    self.session_cameras_demo(&s.cameras);
                }
                if let Some(p) = &s.reader_place {
                    self.session_toon(&p.toon);
                    self.session_labels(p.chapter);
                    self.session.place.chapter = p.chapter;
                    self.session.place.label = p.label.clone();
                }
            }
            46 => {
                if let Some(n) = &s.new_scene {
                    self.press(Hit::Compose(new_scene::ComposeHit::Open));
                    (self.compose.id, self.compose.name, self.compose.cast) = (n.id.clone(), n.name.clone(), n.cast.clone());
                    self.typing = None;
                }
            }
            52 if s.new_scene.as_ref().is_some_and(|n| n.play) => self.press(Hit::Compose(new_scene::ComposeHit::Play)),
            52 if s.new_scene.as_ref().is_some_and(|n| n.build) => self.press(Hit::Compose(new_scene::ComposeHit::Build)),
            50 if s.flash => self.press(Hit::Flash),
            60 => {
                if let Some(text) = &s.flash_filter {
                    self.script_filter(text);
                }
            }
            65 if s.flash_force => self.press(Hit::FlashForce),
            80 if s.flash_ask => self.press(Hit::FlashAsk),
            90 if s.flash_confirm => self.press(Hit::FlashConfirm),
            _ => {}
        }
    }
}
