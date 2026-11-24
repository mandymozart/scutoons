// GENERATED from server/schema/api/library.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
//! The extended Library (#864, agents/research/redesign/lap2/LIBRARY.md): things and files in one list, one server answer for the creator and the deck (GET /api/library, server/docs/API.md "The extended Library"). The server resolves – types, kinds, names, pictures, places, used in – the apps only read. Edit this file, run `npm run generate` (server/schema/generate.ts writes the TS and PHP types), commit the outputs; library.vectors.json holds the file cases every reader holds to.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// the format the API answers (`library`)
pub const LIBRARY_FORMAT: i64 = 1;

/// What a thing is (owner 2026-11-24, LIBRARY.md "Answered" 1): a toon, a character, a deck scene, a live show, a place of the world map, a catalog item (with its subtype), a rig module, an instrument, a rig, a generator (the storyboard's providers), and for now a Kanon text and a person (for credits). Its address is its entity key (`toon:catpope`, `character:catpope`, `scene:nave`, `show:duet`, `place:dom`, `item:camera`, `recipe:x`, `product:x`, `edition:x`, `album:x`, `home-scene:x`, `module:walk`, `instrument:organ`, `rig:character.catpope`, `kanon:character.catpope:de`), a generator's `generator:<id>`, a person's `person:<id>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThingType {
	#[serde(rename = "toon")]
	Toon,
	#[serde(rename = "character")]
	Character,
	#[serde(rename = "scene")]
	Scene,
	#[serde(rename = "show")]
	Show,
	#[serde(rename = "place")]
	Place,
	#[serde(rename = "item")]
	Item,
	#[serde(rename = "module")]
	Module,
	#[serde(rename = "instrument")]
	Instrument,
	#[serde(rename = "rig")]
	Rig,
	#[serde(rename = "generator")]
	Generator,
	#[serde(rename = "kanon")]
	Kanon,
	#[serde(rename = "person")]
	Person,
}

impl ThingType {
	/// every ThingType, in the schema's order
	pub const ALL: [ThingType; 12] = [ThingType::Toon, ThingType::Character, ThingType::Scene, ThingType::Show, ThingType::Place, ThingType::Item, ThingType::Module, ThingType::Instrument, ThingType::Rig, ThingType::Generator, ThingType::Kanon, ThingType::Person];

	pub const fn as_str(self) -> &'static str {
		match self {
			ThingType::Toon => "toon",
			ThingType::Character => "character",
			ThingType::Scene => "scene",
			ThingType::Show => "show",
			ThingType::Place => "place",
			ThingType::Item => "item",
			ThingType::Module => "module",
			ThingType::Instrument => "instrument",
			ThingType::Rig => "rig",
			ThingType::Generator => "generator",
			ThingType::Kanon => "kanon",
			ThingType::Person => "person",
		}
	}
}

/// An item's subtype: a catalog entry of its own table listed as an item – absent on a plain item (`item:<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemSubtype {
	#[serde(rename = "recipe")]
	Recipe,
	#[serde(rename = "product")]
	Product,
	#[serde(rename = "edition")]
	Edition,
	#[serde(rename = "album")]
	Album,
	#[serde(rename = "home-scene")]
	HomeScene,
}

impl ItemSubtype {
	/// every ItemSubtype, in the schema's order
	pub const ALL: [ItemSubtype; 5] = [ItemSubtype::Recipe, ItemSubtype::Product, ItemSubtype::Edition, ItemSubtype::Album, ItemSubtype::HomeScene];

	pub const fn as_str(self) -> &'static str {
		match self {
			ItemSubtype::Recipe => "recipe",
			ItemSubtype::Product => "product",
			ItemSubtype::Edition => "edition",
			ItemSubtype::Album => "album",
			ItemSubtype::HomeScene => "home-scene",
		}
	}
}

/// What a file is: the registry's kind, where `archive`, `document`, `font` and `other` are a `file`. Its address is `<kind>:<path>` – the kind and the current version's main file (CDN-root path), as the Library opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FileKind {
	#[serde(rename = "image")]
	Image,
	#[serde(rename = "audio")]
	Audio,
	#[serde(rename = "video")]
	Video,
	#[serde(rename = "model")]
	Model,
	#[serde(rename = "animation")]
	Animation,
	#[serde(rename = "file")]
	File,
}

impl FileKind {
	/// every FileKind, in the schema's order
	pub const ALL: [FileKind; 6] = [FileKind::Image, FileKind::Audio, FileKind::Video, FileKind::Model, FileKind::Animation, FileKind::File];

	pub const fn as_str(self) -> &'static str {
		match self {
			FileKind::Image => "image",
			FileKind::Audio => "audio",
			FileKind::Video => "video",
			FileKind::Model => "model",
			FileKind::Animation => "animation",
			FileKind::File => "file",
		}
	}
}

/// Where a file came from (the From place): its main file's origin in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FileOrigin {
	#[serde(rename = "uploaded")]
	Uploaded,
	#[serde(rename = "imported")]
	Imported,
	#[serde(rename = "generated")]
	Generated,
}

impl FileOrigin {
	/// every FileOrigin, in the schema's order
	pub const ALL: [FileOrigin; 3] = [FileOrigin::Uploaded, FileOrigin::Imported, FileOrigin::Generated];

	pub const fn as_str(self) -> &'static str {
		match self {
			FileOrigin::Uploaded => "uploaded",
			FileOrigin::Imported => "imported",
			FileOrigin::Generated => "generated",
		}
	}
}

/// How a thing uses another: `cast` (a season's cast lists the character), `credits` (a toon's credits name the person), `rig` (a character or item plays this rig; a rig plays this instrument or module), `reference` (the document names it: an address in a scene or show, a character or item in a chapter's events, an item in a recipe or product, a file in a document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkRole {
	#[serde(rename = "cast")]
	Cast,
	#[serde(rename = "credits")]
	Credits,
	#[serde(rename = "rig")]
	Rig,
	#[serde(rename = "reference")]
	Reference,
}

impl LinkRole {
	/// every LinkRole, in the schema's order
	pub const ALL: [LinkRole; 4] = [LinkRole::Cast, LinkRole::Credits, LinkRole::Rig, LinkRole::Reference];

	pub const fn as_str(self) -> &'static str {
		match self {
			LinkRole::Cast => "cast",
			LinkRole::Credits => "credits",
			LinkRole::Rig => "rig",
			LinkRole::Reference => "reference",
		}
	}
}

/// A level inside a thing: a character's persona › look › mood, a toon's season › chapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LevelKind {
	#[serde(rename = "persona")]
	Persona,
	#[serde(rename = "look")]
	Look,
	#[serde(rename = "mood")]
	Mood,
	#[serde(rename = "season")]
	Season,
	#[serde(rename = "chapter")]
	Chapter,
}

impl LevelKind {
	/// every LevelKind, in the schema's order
	pub const ALL: [LevelKind; 5] = [LevelKind::Persona, LevelKind::Look, LevelKind::Mood, LevelKind::Season, LevelKind::Chapter];

	pub const fn as_str(self) -> &'static str {
		match self {
			LevelKind::Persona => "persona",
			LevelKind::Look => "look",
			LevelKind::Mood => "mood",
			LevelKind::Season => "season",
			LevelKind::Chapter => "chapter",
		}
	}
}

/// One entry: a thing (with `type`) or a file (with `kind`), never both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
	/// the identity: a thing's address (ThingType) or a file's `<kind>:<path>`
	pub address: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub r#type: Option<ThingType>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub subtype: Option<ItemSubtype>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub kind: Option<FileKind>,
	/// what a person reads: the label in the asked locale (the one locale order), else the id; a file's title, else its slug
	pub name: String,
	/// a thumbnail's URL (the CDN's): a file's smallest picture (an image's, a video's poster), a thing's picture (a character's default look, a toon's tile, an item's icon, a place's image) – null where there is no still yet (a model, a scene, a module)
	pub picture: Option<String>,
	/// the thing it belongs to: a rig's character or item, a Kanon text's subject
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<String>,
	/// the toons it belongs to (the place This toon); empty = shared by every toon (an instrument, a template scene, a file no toon uses)
	pub toons: Vec<String>,
	/// how many things use it (a file: how many documents; a character: its casts, scenes, shows, chapters …)
	#[serde(rename = "usedIn")]
	pub used_in: i64,
	/// when it last changed (ISO 8601, UTC): a thing's newest version, a file's registry entry – the place Recently added
	pub updated: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub origin: Option<FileOrigin>,
}

/// How many entries of each type and kind the place holds before the type / kind filter (the type select's numbers): `character`, `image`, …
pub type LibraryCounts = std::collections::BTreeMap<String, i64>;

/// GET /api/library[?toon=&place=&type=&subtype=&kind=&from=&locale=&cursor=&limit=]: a page of one place's entries – things first (in ThingType's order), then files (in FileKind's order), each by address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryList {
	/// the format's version
	pub library: i64,
	/// the answer's version (the ETag without quotes)
	pub version: String,
	/// the toon asked for; null = the universe
	pub toon: Option<String>,
	/// the place: `toon`, `all`, `recent`, `unused`, `item:<address>`, `cast:<season>`
	pub place: String,
	pub locale: String,
	pub entries: Vec<LibraryEntry>,
	/// the entries the place and the filters hold, every page together
	pub total: i64,
	/// the cursor of the next page; null = the last page
	pub next: Option<String>,
	pub counts: LibraryCounts,
}

/// One level inside a thing, parents before children. A mood has no address of its own (the address grammar ends at the look, space rig/address.rs): its `address` is its look's, `mood` names it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryLevel {
	pub address: String,
	pub level: LevelKind,
	pub name: String,
	pub picture: Option<String>,
	/// the level (or the thing) one up
	pub parent: String,
	/// a mood only: its id
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mood: Option<String>,
}

/// A thing that uses the stepped-into one ("used in"): a link to open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryLink {
	/// its address (a season's cast: `season:<toon>:<n>`, a chapter: `chapter:<toon>:<n>` – documents that are no entry of the list are linked too)
	pub address: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub r#type: Option<ThingType>,
	pub name: String,
	pub r#as: LinkRole,
}

/// GET /api/library?at=<address>[&locale=]: a thing stepped into – its levels, the files it uses, the things it uses, the things that use it; a file's `at` answers its entry and what uses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryStep {
	pub library: i64,
	pub version: String,
	pub locale: String,
	pub at: LibraryEntry,
	pub levels: Vec<LibraryLevel>,
	/// the files it uses (the place This item's files)
	pub files: Vec<LibraryEntry>,
	/// the things it uses
	pub uses: Vec<LibraryEntry>,
	#[serde(rename = "usedIn")]
	pub used_in: Vec<LibraryLink>,
}
