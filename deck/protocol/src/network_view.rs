// GENERATED from server/schema/api/network-view.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One level of the one network (#763, agents/research/redesign/NETWORK.md §4): what a projector computes from a scope's documents and every canvas draws – the creator's Flow tab, the native deck editor, the Reception tab, the MCP. The documents stay the truth; a view is never stored (only its canvas). Each scope's projector lives where that scope's rules live (story: shared/content; XR: the OSCT logic package; show and rig: the Rust core). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust and TS types), commit the outputs; network-view.vectors.json holds the views every language reads.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// A payload whose shape is not fixed yet: any JSON object.
pub type Loose = serde_json::Map<String, serde_json::Value>;

/// What travels on a port or edge – deck-kit's one signal table (deck/kit/src/base/signal.rs, owner 2026-11-17 [design]): its shapes and colours draw every port and wire. OSCT's `value` is `control`, its `node` is `address`, its `media` is `picture`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Signal {
	#[serde(rename = "event")]
	Event,
	#[serde(rename = "control")]
	Control,
	#[serde(rename = "clock")]
	Clock,
	#[serde(rename = "audio")]
	Audio,
	#[serde(rename = "picture")]
	Picture,
	#[serde(rename = "address")]
	Address,
	#[serde(rename = "vote")]
	Vote,
	#[serde(rename = "scene")]
	Scene,
	#[serde(rename = "far")]
	Far,
}

impl Signal {
	pub const fn as_str(self) -> &'static str {
		match self {
			Signal::Event => "event",
			Signal::Control => "control",
			Signal::Clock => "clock",
			Signal::Audio => "audio",
			Signal::Picture => "picture",
			Signal::Address => "address",
			Signal::Vote => "vote",
			Signal::Scene => "scene",
			Signal::Far => "far",
		}
	}
}

/// Which documents a view projects: `story` chapters and toons, `xr` experiences, `show` rows and cells, `rig` rigs and instruments, `scene` deck scenes, `creator` the creator's own events (generation done, approved, published …), `network` the levels above them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
	#[serde(rename = "story")]
	Story,
	#[serde(rename = "xr")]
	Xr,
	#[serde(rename = "show")]
	Show,
	#[serde(rename = "rig")]
	Rig,
	#[serde(rename = "scene")]
	Scene,
	#[serde(rename = "creator")]
	Creator,
	#[serde(rename = "network")]
	Network,
}

impl Scope {
	pub const fn as_str(self) -> &'static str {
		match self {
			Scope::Story => "story",
			Scope::Xr => "xr",
			Scope::Show => "show",
			Scope::Rig => "rig",
			Scope::Scene => "scene",
			Scope::Creator => "creator",
			Scope::Network => "network",
		}
	}
}

/// Where a node runs: the `reader` (the VN runner), `xr` (OSCT's engine in the reader), the `conductor`, a `deck` (the engine, its own audio context), the `server` (jobs, generations), the `editor` (authoring-time nodes only, never at reading time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Runtime {
	#[serde(rename = "reader")]
	Reader,
	#[serde(rename = "xr")]
	Xr,
	#[serde(rename = "conductor")]
	Conductor,
	#[serde(rename = "deck")]
	Deck,
	#[serde(rename = "server")]
	Server,
	#[serde(rename = "editor")]
	Editor,
}

impl Runtime {
	pub const fn as_str(self) -> &'static str {
		match self {
			Runtime::Reader => "reader",
			Runtime::Xr => "xr",
			Runtime::Conductor => "conductor",
			Runtime::Deck => "deck",
			Runtime::Server => "server",
			Runtime::Editor => "editor",
		}
	}
}

/// A node's family – how a canvas draws it and what a gesture may do. `container` can be entered (a chapter, a group, a row, a cell, a track); `world` comes from outside the level with an image of its origin (MIDI, clock, carrier bag, VN state); `free` lives only in the canvas (a reference, a generation, a manual instruction – OSCT L11–L13); `far` is a stub's far end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Family {
	#[serde(rename = "event")]
	Event,
	#[serde(rename = "rule")]
	Rule,
	#[serde(rename = "action")]
	Action,
	#[serde(rename = "variable")]
	Variable,
	#[serde(rename = "entity")]
	Entity,
	#[serde(rename = "scene")]
	Scene,
	#[serde(rename = "plugin")]
	Plugin,
	#[serde(rename = "container")]
	Container,
	#[serde(rename = "sound")]
	Sound,
	#[serde(rename = "world")]
	World,
	#[serde(rename = "free")]
	Free,
	#[serde(rename = "far")]
	Far,
}

impl Family {
	pub const fn as_str(self) -> &'static str {
		match self {
			Family::Event => "event",
			Family::Rule => "rule",
			Family::Action => "action",
			Family::Variable => "variable",
			Family::Entity => "entity",
			Family::Scene => "scene",
			Family::Plugin => "plugin",
			Family::Container => "container",
			Family::Sound => "sound",
			Family::World => "world",
			Family::Free => "free",
			Family::Far => "far",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
	#[serde(rename = "in")]
	In,
	#[serde(rename = "out")]
	Out,
}

impl Direction {
	pub const fn as_str(self) -> &'static str {
		match self {
			Direction::In => "in",
			Direction::Out => "out",
		}
	}
}

/// How an edge crosses runtimes: `same` (one runtime), `relay` (the Live relay), `navigation` (the reader moves to another place), `savegame` (a variable or item read elsewhere).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Transport {
	#[serde(rename = "same")]
	Same,
	#[serde(rename = "relay")]
	Relay,
	#[serde(rename = "navigation")]
	Navigation,
	#[serde(rename = "savegame")]
	Savegame,
}

impl Transport {
	pub const fn as_str(self) -> &'static str {
		match self {
			Transport::Same => "same",
			Transport::Relay => "relay",
			Transport::Navigation => "navigation",
			Transport::Savegame => "savegame",
		}
	}
}

/// One step of the breadcrumb, root first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
	/// The level's address in the one grammar (rig/address.rs), e.g. `chapter:sandbox/1/2`.
	pub address: String,
	pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
	pub id: String,
	pub label: String,
	pub signal: Signal,
	pub dir: Direction,
	/// Several edges may meet here (a multi-in socket).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub many: Option<bool>,
	/// Unused and folded behind the node's `+`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hidden: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
	/// Unique within the view.
	pub id: String,
	/// The registered node type (a scope's projector or a module names it).
	pub kind: String,
	pub family: Family,
	pub label: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub detail: Option<String>,
	/// What the node is, in the one grammar.
	pub address: String,
	pub runtime: Runtime,
	pub ports: Vec<Port>,
	/// The address of the level a double-click opens; none: a leaf.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub enter: Option<String>,
	/// An asset reference for the node's image (a world node's origin, a preview).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub picture: Option<String>,
	/// The canvas group it sits in.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
	pub node: String,
	pub port: String,
}

/// The one document field that holds an edge or a node: an edit is an op on it. None on an edge: derived (a by-name join, a computed link).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Home {
	/// The document's address, e.g. `rig:crow`, `chapter:sandbox/1/2`, `show:demo`.
	pub doc: String,
	/// A JSON pointer into the document.
	pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
	pub id: String,
	pub from: Pin,
	pub to: Pin,
	pub signal: Signal,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub label: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub home: Option<Home>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub crosses: Option<Transport>,
	/// A bundle of this many edges of one kind between one pair.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub count: Option<u64>,
}

/// An edge that leaves the level, named by its far end; following it changes the view's scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stub {
	/// The edge's id; its node at the level's edge is a `far` node.
	pub edge: String,
	pub side: Direction,
	/// The far end's address.
	pub far: String,
	pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
	#[serde(rename = "error")]
	Error,
	#[serde(rename = "warning")]
	Warning,
}

impl Severity {
	pub const fn as_str(self) -> &'static str {
		match self {
			Severity::Error => "error",
			Severity::Warning => "warning",
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
	/// A node's or an edge's id.
	pub at: String,
	pub severity: Severity,
	pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
	pub id: String,
	pub label: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub colour: Option<String>,
	pub nodes: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub folded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
	pub id: String,
	pub text: String,
	/// The node it sits beside.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub near: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
	pub x: f64,
	pub y: f64,
}

/// Authoring data beside the documents, never played: groups, notes, folds, positions pinned by hand (rem) over the auto layout.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Canvas {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub groups: Option<Vec<Group>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub notes: Option<Vec<Note>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub folded: Option<Vec<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pins: Option<std::collections::HashMap<String, Point>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
	pub scope: Scope,
	/// The level's address.
	pub context: String,
	pub levels: Vec<Level>,
	pub nodes: Vec<Node>,
	pub edges: Vec<Edge>,
	pub stubs: Vec<Stub>,
	pub problems: Vec<Problem>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub canvas: Option<Canvas>,
}
