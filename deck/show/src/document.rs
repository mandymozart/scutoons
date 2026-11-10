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
    pub outlets: Vec<Outlet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seats: Vec<Seat>,
    pub rows: Vec<Row>,
    /// what a parent may use of this show, when another show seats it
    #[serde(default)]
    pub face: Face,
}

impl Show {
    pub fn row(&self, id: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.id == id)
    }

    pub fn outlet(&self, id: &str) -> Option<&Outlet> {
        self.outlets.iter().find(|o| o.id == id)
    }

    pub fn seat(&self, id: &str) -> Option<&Seat> {
        self.seats.iter().find(|s| s.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outlet {
    pub id: String,
    #[serde(default)]
    pub kind: OutletKind,
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
}

/// One address a row sends to: `character:yellowmouse > dance:samba`, filtered (for the workshop by group and device).
/// The address and the action are read by the one parser (deck-space `rig/address.rs` `Target`, RULES A14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Addressed {
    pub address: String,
    pub action: String,
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<protocol::AddressFilter>,
}

/// A row's firing (SHOW.md §7). GO and the hand always work, whatever stands here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fire {
    /// the event id that fires it (the contract with the chapter editor); none = only by hand
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
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
    #[default]
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Moment {
    #[default]
    Now,
    Bar,
    Phrase,
}

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
    },
    /// keep the piece, change these. Names the piece it expects
    Values { piece: String, values: Values },
    /// nothing happens on this outlet
    Keep,
    /// the same as another outlet
    Mirror { of: String },
    /// black, idle, all off
    Black,
    /// a seat: "go to your row"
    Go { row: String },
    /// a seat: "you are free"
    Free,
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

/// One document, read and checked on its own. The tree: [`validate::resolve`].
pub fn parse(json: &str) -> Result<Show, ParseError> {
    let show: Show = serde_json::from_str(json).map_err(|e| ParseError::Shape(e.to_string()))?;
    let problems = validate::validate(&show);
    if problems.is_empty() {
        Ok(show)
    } else {
        Err(ParseError::Invalid(problems))
    }
}
