//! The show document (SHOW.md §4b, §5, §7). JSON: shows are API content later (#90), like every document in `entity_versions`.
use crate::validate::{self, Problem};
use protocol::DeviceMsg;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A piece's named values (a number, a word, a video's id).
pub type Values = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Show {
    pub id: String,
    pub version: u32,
    /// the toons whose scenes, story events and address books the show uses, the first its home (where it is listed, whose
    /// source language labels it); at least one – a document without one opens with the problem, so a toon can be picked
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub toons: Vec<String>,
    pub outlets: Vec<Outlet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seats: Vec<Seat>,
    pub rows: Vec<Row>,
    /// what a parent may use of this show, when another show seats it
    #[serde(default)]
    pub face: Face,
    /// #91: the lamps on the device outlets and the looks a row sets on them (LIVE.md B.5.2)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixtures: Vec<Fixture>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub looks: Vec<Look>,
    /// #546: the show's questions, each stored once; an `ask` cell names one, the row's columns that carry it are who votes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<Question>,
    /// the stage background on every stage outlet, over what its pieces set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<deck_space::Background>,
    /// the master tempo the show starts with (the play pill's BPM and source); none: the clock as it is
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempo: Option<Tempo>,
    /// the show reader a deck needs to read it right ([`READER`]); none: the first, every deck reads it. Written by
    /// [`Show::stamp`] only when the show uses what an older reader would misread – an older deck refuses the unknown field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reads: Option<u32>,
}

/// The show reader of this build: 2 reads several events on a row and another row's fire as an event (`row:<id>`).
pub const READER: u32 = 2;

impl Show {
    /// the reader this show needs: 2 when a row fires on several events or on another row's fire, else 1
    pub fn reader_needed(&self) -> u32 {
        let new = self.rows.iter().any(|r| r.fire.event.len() > 1 || r.fire.event.iter().any(|e| e.starts_with(ROW_FIRED)));
        if new {
            2
        } else {
            1
        }
    }

    /// writes the reader it needs (none for the first): an older deck then refuses the show instead of misreading it
    pub fn stamp(&mut self) {
        self.reads = Some(self.reader_needed()).filter(|n| *n > 1);
    }
}

/// what starts the event another row's fire is: `row:<id>`
pub const ROW_FIRED: &str = "row:";

/// the event row `row`'s fire is to the show's other rows
pub fn row_fired(row: &str) -> String {
    format!("{ROW_FIRED}{row}")
}

/// The events a row fires on – any of them wakes it. Read as one id or a list; written as one id while there is one, a
/// list only with several, so a show with one event per row reads on an older deck as before.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Events(Vec<String>);

impl Events {
    pub fn one(event: impl Into<String>) -> Events {
        Events(vec![event.into()])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    pub fn contains(&self, event: &str) -> bool {
        self.0.iter().any(|e| e == event)
    }

    pub fn first(&self) -> Option<&str> {
        self.0.first().map(String::as_str)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// `ch3.vigil, row:dialog`
    pub fn words(&self) -> String {
        self.0.join(", ")
    }

    /// on if it was off, off if it was on; whether it is on now
    pub fn toggle(&mut self, event: &str) -> bool {
        let had = self.contains(event);
        if had {
            self.0.retain(|e| e != event);
        } else {
            self.0.push(event.to_string());
        }
        !had
    }

    /// the event `from` is called `to` now
    pub fn rename(&mut self, from: &str, to: &str) {
        for e in self.0.iter_mut().filter(|e| *e == from) {
            *e = to.to_string();
        }
    }

    pub fn remove(&mut self, event: &str) {
        self.0.retain(|e| e != event);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl From<Vec<String>> for Events {
    fn from(v: Vec<String>) -> Events {
        Events(v)
    }
}

impl Serialize for Events {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0.as_slice() {
            [one] => s.serialize_str(one),
            all => all.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Events {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Events, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            One(String),
            Many(Vec<String>),
        }
        match Either::deserialize(d) {
            Ok(Either::One(e)) => Ok(Events(vec![e])),
            Ok(Either::Many(v)) => Ok(Events(v)),
            Err(_) => Err(serde::de::Error::custom("an event id or a list of event ids")),
        }
    }
}

/// A show's master tempo: the BPM on the deck's own clock and the source the clock follows (`SOURCES`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tempo {
    pub bpm: f64,
    #[serde(default = "own_clock")]
    pub source: deck_core::tempo::Source,
}

fn own_clock() -> deck_core::tempo::Source {
    deck_core::tempo::Source::Internal
}

impl Tempo {
    /// the sources a show keeps: the deck's own clock, MIDI clock in, Link (the show clock is the show's own; OSC is a scene's)
    pub const SOURCES: [deck_core::tempo::Source; 3] = [deck_core::tempo::Source::Internal, deck_core::tempo::Source::Midi, deck_core::tempo::Source::Link];

    /// the tempo a show keeps from the pill's: the BPM within the clock's range, a source it cannot keep as the own clock
    pub fn of(bpm: f64, source: deck_core::tempo::Source) -> Tempo {
        let source = if Tempo::SOURCES.contains(&source) { source } else { deck_core::tempo::Source::Internal };
        Tempo { bpm: deck_core::tempo::clamp_bpm(bpm), source }
    }

    /// what the clock is told when the show opens
    pub fn event(&self) -> deck_core::tempo::What {
        deck_core::tempo::What::Saved(self.bpm, self.source)
    }

    /// the problem with it, in words
    pub fn check(&self) -> Option<String> {
        use deck_core::tempo::{MAX_BPM, MIN_BPM};
        if !self.bpm.is_finite() || !(MIN_BPM..=MAX_BPM).contains(&self.bpm) {
            return Some(format!("tempo: {} BPM is out of {MIN_BPM:.0}-{MAX_BPM:.0}", self.bpm));
        }
        (!Tempo::SOURCES.contains(&self.source)).then(|| format!("tempo: the source '{}' is none a show keeps (internal, midi, link)", self.source.name()))
    }
}

/// what a document saved before #708 (one `toon`, no `toons`) is told: its stored versions were migrated, so a file that
/// still names `toon` is an old copy
pub const OLD_TOON: &str = "it names the old `toon`, no `toons` – an old copy: load the show from the API again";

/// serde's words, or [`OLD_TOON`] for a document that names `toon`
fn shape(e: &serde_json::Error) -> String {
    let words = e.to_string();
    if words.contains("unknown field `toon`") {
        OLD_TOON.to_string()
    } else {
        words
    }
}

/// The toons a show document as JSON names, home first; none when it names none or is no object
pub fn toons_of(doc: &serde_json::Value) -> Vec<String> {
    doc.get("toons").and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or_default()
}

impl Show {
    /// the home toon: the first (empty: none)
    pub fn home(&self) -> &str {
        self.toons.first().map_or("", String::as_str)
    }

    pub fn question(&self, id: &str) -> Option<&Question> {
        self.questions.iter().find(|q| q.id == id)
    }

    pub fn row(&self, id: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.id == id)
    }

    pub fn outlet(&self, id: &str) -> Option<&Outlet> {
        self.outlets.iter().find(|o| o.id == id)
    }

    pub fn seat(&self, id: &str) -> Option<&Seat> {
        self.seats.iter().find(|s| s.id == id)
    }

    pub fn fixture(&self, id: &str) -> Option<&Fixture> {
        self.fixtures.iter().find(|f| f.id == id)
    }

    pub fn look(&self, id: &str) -> Option<&Look> {
        self.looks.iter().find(|l| l.id == id)
    }

    /// #478: the roles this conductor gives out – every stage outlet, its name and who may take it – as its session hears
    /// them (`roles`); the relay counts who is in each
    pub fn roles(&self) -> Vec<protocol::ShowRole> {
        let stage = self.outlets.iter().filter(|o| o.kind == OutletKind::Stage);
        stage.map(|o| protocol::ShowRole { id: o.id.clone(), label: Some(o.name().to_string()), r#for: o.admits.clone(), here: None }).collect()
    }
}

/// A lamp on a DMX device outlet (LIVE.md B.5.2, #91): where its channels start in the outlet's universe and how they are
/// laid out. Which interface or Art-Net universe the outlet is in a room is the venue's (open, SHOW.md §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub id: String,
    /// a device outlet of this show
    pub outlet: String,
    /// the first channel, 1-based
    pub address: u16,
    pub profile: Profile,
}

/// How a fixture's channels are laid out, from its address on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    Dimmer,
    Rgb,
    Rgbw,
    RgbwStrobe,
}

impl Profile {
    /// the channels' names in order
    pub fn channels(self) -> &'static [&'static str] {
        match self {
            Profile::Dimmer => &["dim"],
            Profile::Rgb => &["r", "g", "b"],
            Profile::Rgbw => &["r", "g", "b", "w"],
            Profile::RgbwStrobe => &["r", "g", "b", "w", "strobe"],
        }
    }
}

/// A light look: values (0–255) by fixture and channel; a channel it does not name is 0. A device cell sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    pub id: String,
    pub values: BTreeMap<String, BTreeMap<String, u8>>,
}

impl Look {
    /// The DMX this look sends on `outlet`: one range per fixture of that outlet it names, in address order.
    pub fn dmx(&self, show: &Show, outlet: &str, fade: Option<u32>) -> Vec<DeviceMsg> {
        let mut lit: Vec<&Fixture> = show.fixtures.iter().filter(|f| f.outlet == outlet && self.values.contains_key(&f.id)).collect();
        lit.sort_by_key(|f| f.address);
        lit.iter()
            .map(|f| {
                let set = &self.values[&f.id];
                let values = f.profile.channels().iter().map(|c| set.get(*c).copied().unwrap_or(0)).collect();
                DeviceMsg::Dmx(protocol::DmxMessage { ch: f.address as u64, values, fade: fade.map(u64::from) })
            })
            .collect()
    }
}

/// A column of the show. A stage outlet is a **role** (#478, owner 2026-11-10 [role]): the conductor defines it, decks take it
/// (a deck may change its role at any time), its cells are what they play.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outlet {
    pub id: String,
    #[serde(default)]
    pub kind: OutletKind,
    /// #478: the role's name as a deck's Live page and the conductor show it (none: its id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// #478: who may take the role, any one fact admits (the wire's type, one copy); none = a guest, anyone with the code
    #[serde(rename = "for", default, skip_serializing_if = "Option::is_none")]
    pub admits: Option<protocol::Admits>,
    /// the head over a run of adjacent columns (`light & sound`); none: no group
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

impl Outlet {
    /// its name for people: the label, else the id
    pub fn name(&self) -> &str {
        self.label.as_deref().filter(|l| !l.trim().is_empty()).unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutletKind {
    /// a screen, a speaker: plays pieces
    #[default]
    Stage,
    /// a bridge's device (MIDI, DMX, OSC): plays the messages a row sends it, nothing else
    Device,
}

/// The place a parent keeps for a child (owner 2026-11-10, SHOW.md §4b). It stands in the parent's document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub id: String,
    /// the child's show at a pinned version
    pub show: Pinned,
    /// who sits there: `deck:<id>` or `member:<id>`; none = nobody yet
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sits: Option<String>,
    pub word: Word,
    /// whether the parent may lock it (across members only if the seat accepted that)
    #[serde(default)]
    pub lockable: bool,
    /// what the parent uses of the child's face
    #[serde(default)]
    pub uses: Face,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pinned {
    pub id: String,
    pub version: u32,
}

impl fmt::Display for Pinned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

/// How a child takes its parent's rows: a child treats its parent like a reader (SHOW.md §4b). The wire's type, one copy.
pub use protocol::Word;

/// What a show shows of itself, and what a seat uses of it (like `rig/face.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outlets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub id: String,
    #[serde(default)]
    pub fire: Fire,
    /// by outlet or seat id. You write only what changes: no cell = `keep`
    #[serde(default)]
    pub cells: BTreeMap<String, Cell>,
    /// #91: what the row sends to addresses when it fires (SHOW.md §8, §9): down this subtree, whoever plays it picks up;
    /// finding nobody stays silent and is logged
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<Addressed>,
    /// #546 (SHOW.md §15 2): by outlet id, the same keys as `cells` – who that column's cell reaches (a vote: who votes,
    /// the relay tests it); none = everyone in the column. Every fact holds (protocol `filter::passes`)
    #[serde(rename = "where", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub r#where: BTreeMap<String, protocol::Filter>,
    /// cells of this row wired to each other by their ports (wiring.rs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<crate::wiring::CellWire>,
    /// the node view's groups, notes and nodes folded by hand (canvas.rs)
    #[serde(default, skip_serializing_if = "crate::canvas::Canvas::is_empty")]
    pub nodes: crate::canvas::Canvas,
}

/// One address a row sends to: `character:yellowmouse > dance:samba`, filtered (for the workshop by group and device).
/// The address and the action are read by the one parser (deck-space `rig/address.rs` `Target`, RULES A14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Addressed {
    pub address: String,
    pub action: String,
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<protocol::Filter>,
}

/// A row's firing (SHOW.md §7). GO and the hand always work, whatever stands here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fire {
    /// the events that fire it, any of them: a story event's id (the contract with the chapter editor), another row's fire
    /// (`row:<id>`), a vote's result (`vote:<question>/<n>`); none = only by hand
    #[serde(default, skip_serializing_if = "Events::is_empty")]
    pub event: Events,
    #[serde(default)]
    pub how: How,
    /// how long an event must stand before it counts (ms)
    #[serde(default)]
    pub hold_ms: u32,
    #[serde(default)]
    pub moment: Moment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    Auto,
    Suggest,
    /// not auto: only the operator fires it
    #[default]
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Moment {
    #[default]
    Now,
    Bar,
    Phrase,
}

/// What a cell's piece may be from the toon's address book (#487): a published deck scene – the entry's address,
/// `scene:<id>`, is what the piece holds. Rigs and modules are pieces too but no entries of the book; sandboxes never are.
/// The deck editor's piece picker (#537, rows.rs `pieces`) asks the book's `find` for these kinds.
pub const PIECE_KINDS: [protocol::address_book::AddressKind; 1] = [protocol::address_book::AddressKind::Scene];

/// One cell (SHOW.md §5). On a seat's column: `go`, `free` or `keep`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum Cell {
    /// this piece with these values. A device outlet has no piece, it plays `send`
    Play {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        piece: Option<String>,
        #[serde(default, skip_serializing_if = "Values::is_empty")]
        values: Values,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        send: Vec<DeviceMsg>,
        /// one of the piece's camera shots: playing, the cell sends `<piece> > shot:<id>` (a scene's camera word)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shot: Option<String>,
    },
    /// keep the piece, change these. Names the piece it expects
    Values { piece: String, values: Values },
    /// nothing happens on this outlet
    Keep,
    /// the same as another outlet
    Mirror { of: String },
    /// the empty cell, Ableton's stop slot: launched (or its row fired) the column stops – black, the show's background.
    /// A cell left out of a row is `keep`, not empty
    Black,
    /// a seat: "go to your row"
    Go { row: String },
    /// a seat: "you are free"
    Free,
    /// #91: a device outlet's light look (its fixtures' DMX), faded in over `fade` ms
    Look {
        look: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade: Option<u32>,
    },
    /// #546 (SHOW.md §15 1): a question of the show to this column's audience – the piece keeps playing, the deck draws the
    /// question over it. Every column of a row that names it votes together; who in a column: the row's `where`
    Ask { question: String },
}

/// A text by locale (`{ "en": "Which way?" }`), picked in the one locale order (protocol `content_text`).
pub type Loc = BTreeMap<String, String>;

/// One question of a show (#546, SHOW.md §15 1): typed here, or one of the story's choices (#550 fills it); how the vote
/// runs is #533's form without its groups – the columns that ask it are the groups. The rule is the relay's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    /// typed in the show
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed: Option<Typed>,
    /// a choice of the story, its labels as published at that version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<StoryChoice>,
    pub rule: protocol::VoteRule,
    /// rule first: the votes an option needs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<i64>,
    pub seconds: i64,
    /// the option when nobody voted (its place)
    #[serde(default)]
    pub fallback: usize,
    /// a voter sees the tally once they have voted (the form's default: yes)
    #[serde(default = "yes")]
    pub show: bool,
}

fn yes() -> bool {
    true
}

/// A question typed in the show: its words and its options, by locale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Typed {
    #[serde(default, skip_serializing_if = "Loc::is_empty")]
    pub question: Loc,
    pub options: Vec<Loc>,
}

/// A choice of the story as a question: the chapter's choice event at a version, its options' labels as they were then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryChoice {
    pub toon: String,
    pub chapter: u32,
    pub version: u32,
    pub event: String,
    pub options: Vec<Loc>,
    /// #552: each option's `addChoice` id, same order ('' = it has none) – how the linked reader finds the voted option
    /// (`VoteOpen.story`); empty in a snapshot picked before #552 (story.rs `changed_text` says: pick it again)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
}

impl Question {
    /// its options' labels by locale (typed, else the story's)
    pub fn options(&self) -> &[Loc] {
        match (&self.typed, &self.story) {
            (Some(t), _) => &t.options,
            (None, Some(s)) => &s.options,
            (None, None) => &[],
        }
    }

    /// what a person reads for it: the typed question in `locale`'s order, else the story's event, else its id
    pub fn label(&self, locale: &str) -> String {
        let typed = self.typed.as_ref().and_then(|t| protocol::content_text::pick_text(&t.question, locale, None));
        match (typed, &self.story) {
            (Some(q), _) => q.to_string(),
            (None, Some(s)) => format!("story: {}", s.event),
            (None, None) => self.id.clone(),
        }
    }
}

/// Why a text is no show.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// not the document's shape, serde's words
    Shape(String),
    /// the shape is right, what it says is not
    Invalid(Vec<Problem>),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Shape(e) => write!(f, "not a show document: {e}"),
            ParseError::Invalid(problems) => {
                let lines: Vec<String> = problems.iter().map(Problem::to_string).collect();
                write!(f, "invalid show: {}", lines.join("; "))
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// One document for editing (#91): the shape must be right, what it says may be wrong – the editor opens it and its
/// problems list says what (`validate`). What runs or is flashed goes through [`parse`].
pub fn open(json: &str) -> Result<Show, String> {
    serde_json::from_str(json).map_err(|e| ParseError::Shape(shape(&e)).to_string())
}

/// One document, read and checked on its own. The tree: [`validate::resolve`].
pub fn parse(json: &str) -> Result<Show, ParseError> {
    let show: Show = serde_json::from_str(json).map_err(|e| ParseError::Shape(shape(&e)))?;
    let problems = validate::validate(&show);
    if problems.is_empty() {
        Ok(show)
    } else {
        Err(ParseError::Invalid(problems))
    }
}

/// What New > Show makes: the name as its id (lowercase words joined by `-`), the toon, one stage role, no row yet.
pub fn new_show(name: &str, toon: &str) -> Result<Show, String> {
    let id = name.trim().to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
    if id.is_empty() {
        return Err("a new show needs a name".into());
    }
    let doc = serde_json::json!({ "id": id, "version": 1, "toons": Some(toon.trim()).filter(|t| !t.is_empty()).into_iter().collect::<Vec<_>>(), "outlets": [{ "id": "left" }], "rows": [] });
    parse(&doc.to_string()).map_err(|e| e.to_string())
}

/// what stands where the show names no toon, over the picker
pub const PICK_TOON: &str = "No toon yet – pick the show's:";
/// the toon picker's query found none
pub const NO_TOON_FOUND: &str = "No toon matches.";

/// The toons whose id holds `query` (any case), in the list's order.
pub fn toons_found<'a>(toons: &'a [String], query: &str) -> Vec<&'a String> {
    let q = query.trim().to_lowercase();
    toons.iter().filter(|t| t.to_lowercase().contains(&q)).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_show_is_named_and_has_its_toon() {
        let s = super::new_show(" Top Toy: Opening ", "catpope").unwrap();
        assert_eq!((s.id.as_str(), s.home(), s.version, s.outlets.len(), s.rows.len()), ("top-toy-opening", "catpope", 1, 1, 0));
        assert_eq!(super::new_show(" – ", "catpope").unwrap_err(), "a new show needs a name");
        assert_eq!(super::new_show("Opening", " ").unwrap_err(), "invalid show: show: it names no toon – pick the show's toon");
        let toons = ["catpope".to_string(), "feilchen".to_string()];
        assert_eq!(super::toons_found(&toons, "FEIL"), [&toons[1]]);
        assert_eq!(super::toons_found(&toons, "").len(), 2);
    }

    /// #708: a show names its toons, the first its home, and is written with them
    #[test]
    fn a_show_names_its_toons_the_first_its_home() {
        let new = super::parse(r#"{ "id": "x", "version": 1, "toons": ["catpope", "yellowmouse"], "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap();
        assert_eq!((new.toons.len(), new.home()), (2, "catpope"));
        assert_eq!(super::parse(&serde_json::to_string(&new).unwrap()).unwrap(), new, "round trip");
        assert_eq!(super::toons_of(&serde_json::to_value(&new).unwrap()), ["catpope", "yellowmouse"]);
        let none = super::open(r#"{ "id": "x", "version": 1, "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap();
        assert_eq!((none.toons.len(), none.home()), (0, ""));
        let twice = super::open(r#"{ "id": "x", "version": 1, "toons": ["catpope", "catpope"], "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap();
        assert!(crate::validate(&twice).iter().any(|p| p.to_string().contains("twice")));
    }

    /// #708, RULES E2: the stored shows were migrated; an old file that still names `toon` is no show, and says why
    #[test]
    fn an_old_file_with_toon_is_refused_in_clear_words() {
        let old = r#"{ "id": "x", "version": 1, "toon": "catpope", "outlets": [{ "id": "left" }], "rows": [] }"#;
        assert_eq!(super::open(old).unwrap_err(), format!("not a show document: {}", super::OLD_TOON));
        assert_eq!(super::parse(old).unwrap_err().to_string(), "not a show document: it names the old `toon`, no `toons` – an old copy: load the show from the API again");
        assert!(super::toons_of(&serde_json::from_str(old).unwrap()).is_empty(), "no toons");
    }

    #[test]
    fn a_document_with_problems_opens_for_editing_and_does_not_parse() {
        let doc = r#"{ "id": "x", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [{ "id": "a", "cells": { "nowhere": { "mode": "black" } } }] }"#;
        assert!(super::open(doc).is_ok());
        assert!(matches!(super::parse(doc), Err(super::ParseError::Invalid(_))));
        assert!(super::open("{}").unwrap_err().starts_with("not a show document"));
    }

    /// #487: a cell's piece from the book is a published scene – what the book's typeahead offers for it and a pick writes
    #[test]
    fn a_cells_piece_from_the_book_is_a_scene() {
        let book = r#"{ "address_book": 2, "toon": "catpope", "version": "v1", "source": "en", "entries": [
            { "ref": "character:catpope", "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
            { "ref": "item:incense", "address": "item:incense", "kind": "item", "name": "incense", "labels": {}, "origin": { "key": "item:incense", "state": "published" } },
            { "ref": "chapter:3.nave", "kind": "group", "name": "nave", "parent": "chapter:3", "labels": {}, "origin": { "key": "chapter:catpope:3", "state": "published" } },
            { "ref": "scene:nave", "address": "scene:nave", "kind": "scene", "name": "nave", "labels": { "en": "Nave" }, "origin": { "key": "scene:nave", "state": "published" }, "version": 2 }
        ] }"#;
        let b = deck_space::rig::book::Book::parse(book).unwrap();
        let offered: Vec<_> = b.find("nav", &super::PIECE_KINDS, "en").into_iter().filter_map(|i| b.entry(i)?.address.clone()).collect();
        assert_eq!(offered, ["scene:nave"], "not the chapter's group of the same name");
        assert!(b.find("", &super::PIECE_KINDS, "en").iter().all(|&i| b.entry(i).is_some_and(|e| e.r#ref.starts_with("scene:"))));
    }
}
