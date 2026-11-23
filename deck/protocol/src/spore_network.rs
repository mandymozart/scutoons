// GENERATED from server/schema/api/spore-network.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One level of the spore network as a weave (#763, agents/research/redesign/NETWORK.md §4; owner 2026-11-21: the tab is "Spores", connecting is "weaving"): the spores and threads a scope's projector computes from its documents, drawn by every canvas – the creator's Spores tab, the native deck editor, the Reception tab, the MCP. The documents stay the truth; a weave is never stored (only its canvas). Each scope's projector lives where that scope's rules live (story: shared/content; XR: the OSCT logic package; show and rig: the Rust core). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust and TS types), commit the outputs; spore-network.vectors.json holds the weaves every language reads.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// A payload whose shape is not fixed yet: any JSON object.
pub type Loose = serde_json::Map<String, serde_json::Value>;

/// What travels on a port or thread – deck-kit's one signal table (deck/kit/src/base/signal.rs, owner 2026-11-17 [design]): its shapes and colours draw every port and thread. OSCT's `value` is `control`, its `node` is `address`, its `media` is `picture`. `job` is a run that starts, progresses and ends (a generation, a night run, a publish); `asset` is a reference to a Library asset of any kind (a picture that arrived, an approved text) – owner 2026-11-21: spores get job and asset signals.
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
	#[serde(rename = "job")]
	Job,
	#[serde(rename = "asset")]
	Asset,
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
			Signal::Job => "job",
			Signal::Asset => "asset",
		}
	}
}

/// Which documents a weave projects: `story` chapters and toons, `xr` experiences, `show` rows and cells, `rig` rigs and instruments, `scene` deck scenes, `creator` the creator's own events that can trigger spores (a night run, a publish, a draft saved, a job triggered, a picture arriving in another spore, a budget authorized, a Kanon text or script approved, a storyboard cut, a chapter's status changed – owner 2026-11-21), `network` the spore network's levels above them all.
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

/// The driver that runs a spore (the runtimes): the `reader` (the VN runner), `xr` (OSCT's engine in the reader), the `conductor`, a `deck` (the engine, its own audio context), the `server` (jobs, generations), the `editor` (authoring-time nodes only, never at reading time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Driver {
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

impl Driver {
	pub const fn as_str(self) -> &'static str {
		match self {
			Driver::Reader => "reader",
			Driver::Xr => "xr",
			Driver::Conductor => "conductor",
			Driver::Deck => "deck",
			Driver::Server => "server",
			Driver::Editor => "editor",
		}
	}
}

/// A spore's family – how a canvas draws it and what a gesture may do. `container` can be entered (a chapter, a group, a row, a cell, a track); `world` comes from outside the level with an image of its origin (MIDI, clock, carrier bag, VN state); `free` lives only in the canvas (a reference, a generation, a manual instruction – OSCT L11–L13); `far` is a stub's far end.
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

/// How a thread crosses runtimes: `same` (one runtime), `relay` (the Live relay), `navigation` (the reader moves to another place), `savegame` (a variable or item read elsewhere).
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

/// What a port connects to: `cyber` (devices, modules, the engine – called a port) or `organic` (beings, characters, living matter – called a pore). Only the word and the look differ; the signal decides what may connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Matter {
	#[serde(rename = "cyber")]
	Cyber,
	#[serde(rename = "organic")]
	Organic,
}

impl Matter {
	pub const fn as_str(self) -> &'static str {
		match self {
			Matter::Cyber => "cyber",
			Matter::Organic => "organic",
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
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub matter: Option<Matter>,
}

/// A spore: one thing of a level – a group, a rule, a cell, a module, a generator. Its `kind` is its spore type, registered by a scope's projector, a plugin, a component or a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spore {
	/// Unique within the view.
	pub id: String,
	/// The spore type.
	pub kind: String,
	pub family: Family,
	pub label: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub detail: Option<String>,
	/// What the spore is, in the one grammar.
	pub address: String,
	pub driver: Driver,
	pub ports: Vec<Port>,
	/// The address of the level a double-click opens; none: a leaf.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub enter: Option<String>,
	/// An asset reference for the spore's image (a world spore's origin, a preview).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub picture: Option<String>,
}

/// One end of a thread: a spore and one of its ports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct End {
	pub spore: String,
	pub port: String,
}

/// The one document field that holds a thread or a spore: an edit is an op on it. None on a thread: derived (a by-name join, a computed link).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Home {
	/// The document's address, e.g. `rig:crow`, `chapter:sandbox/1/2`, `show:demo`.
	pub doc: String,
	/// A JSON pointer into the document.
	pub path: String,
}

/// A thread woven between two spores' ports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
	pub id: String,
	pub from: End,
	pub to: End,
	pub signal: Signal,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub label: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub home: Option<Home>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub crosses: Option<Transport>,
	/// A bundle of this many threads of one kind between one pair.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub count: Option<u64>,
}

/// A thread that leaves the level, named by its far end; following it changes the weave's scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stub {
	/// The thread's id; its spore at the level's edge is a `far` spore.
	pub thread: String,
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
	/// A spore's or a thread's id.
	pub at: String,
	pub severity: Severity,
	pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
	pub id: String,
	pub label: String,
	/// `#rrggbb`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub colour: Option<String>,
	/// The spores it frames – the one place a spore's group is kept.
	pub spores: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub folded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
	pub id: String,
	pub text: String,
	/// Who wrote it (the member's name).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub author: Option<String>,
	/// The spore it sits beside; none: after the weave.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub near: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
	pub x: f64,
	pub y: f64,
}

/// Authoring data beside the documents, never played – the shape deck-show keeps per row (deck/show/src/canvas.rs): groups, notes, spores folded (true) or opened (false) by hand over how they come, positions placed by hand (rem) over the auto layout.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Canvas {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub groups: Option<Vec<Group>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub notes: Option<Vec<Note>>,
	/// By spore id: folded (true) or opened (false) by hand; none: as it comes.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub folded: Option<std::collections::HashMap<String, bool>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub placed: Option<std::collections::HashMap<String, Point>>,
}

/// One level of the spore network: its spores, the threads woven between them, the threads that leave it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Weave {
	pub scope: Scope,
	/// The level's address.
	pub context: String,
	pub levels: Vec<Level>,
	pub spores: Vec<Spore>,
	pub threads: Vec<Thread>,
	pub stubs: Vec<Stub>,
	pub problems: Vec<Problem>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub canvas: Option<Canvas>,
}
