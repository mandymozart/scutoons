// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One toon's address book (#477, decisions/deck.md [contract] [typeahead] [address]): every thing of the toon's catalog a typeahead may pick, with its reference, its address where something can receive one, its kind, labels and where it comes from. The server writes it (GET /api/addresses/<toon>), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// A payload whose shape is not fixed yet: any JSON object.
pub type Loose = serde_json::Map<String, serde_json::Value>;

/// What an entry is – the one thing a typeahead filters by: an asker takes the kinds it may pick (a VN character event: character, persona, look; unlockLocation: location; migrate: region; an item event: item; a jump: chapter, group, season, toon, product; setVariable: variable; a show cell's piece: scene). A jump's kinds are its target types (`JumpToTargetType`, a group is a chapter target with its label): `season:<number>` (the toon's season), `toon:<id>` (a toon: its page, and in the creator's jump field the way into its own book), `product:<id>` (a shop product of the toon or a shared one); XR is not listed: there is no catalog of XR targets. `region:<id>`: a region of the world map, where migrate registers someone. `scene:<id>`: a deck scene (#487, research/SCENES.md §8: `scene` means the deck scene only – never a chapter's group, a screenplay heading or a sandbox), listed only as published and only in the books of the toons its `toons` names. character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); location, item and chapter are referenced `<kind>:<id>`. Proposals the owner has not confirmed (#477): a chapter's group is referenced `chapter:<number>.<group>` (its `name` the label as an event stores it, the reference word a safe form of it) and a story variable `variable:<scope>.<name>` (scope = the owning toon or `global`; its `name` the variable as written in this toon).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressKind {
	#[serde(rename = "character")]
	Character,
	#[serde(rename = "persona")]
	Persona,
	#[serde(rename = "look")]
	Look,
	#[serde(rename = "location")]
	Location,
	#[serde(rename = "item")]
	Item,
	#[serde(rename = "chapter")]
	Chapter,
	#[serde(rename = "group")]
	Group,
	#[serde(rename = "variable")]
	Variable,
	#[serde(rename = "season")]
	Season,
	#[serde(rename = "toon")]
	Toon,
	#[serde(rename = "product")]
	Product,
	#[serde(rename = "region")]
	Region,
	#[serde(rename = "scene")]
	Scene,
}

impl AddressKind {
	pub const fn as_str(self) -> &'static str {
		match self {
			AddressKind::Character => "character",
			AddressKind::Persona => "persona",
			AddressKind::Look => "look",
			AddressKind::Location => "location",
			AddressKind::Item => "item",
			AddressKind::Chapter => "chapter",
			AddressKind::Group => "group",
			AddressKind::Variable => "variable",
			AddressKind::Season => "season",
			AddressKind::Toon => "toon",
			AddressKind::Product => "product",
			AddressKind::Region => "region",
			AddressKind::Scene => "scene",
		}
	}
}

/// `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's. A scene is the exception: only its published version is listed (a draft never leaves the editor), so its entry is always `published`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OriginState {
	#[serde(rename = "published")]
	Published,
	#[serde(rename = "draft")]
	Draft,
}

impl OriginState {
	pub const fn as_str(self) -> &'static str {
		match self {
			OriginState::Published => "published",
			OriginState::Draft => "draft",
		}
	}
}

/// Content text: locale → text (the one shape of authored text; pick a locale with the one locale order, server/schema/tables/locale-order.json).
pub type LocaleText = std::collections::HashMap<String, String>;

/// The catalog document an entry comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Origin {
	/// the document's entity key (`character:catpope`, `place:dom`, `chapter:catpope:3`, `season:catpope:1`, `toon:catpope`, `scene:nave`) or catalog entry (`item:incense`, `product:dago_hoodie`)
	pub key: String,
	pub state: OriginState,
}

/// One thing of the catalog: its reference always, its address only where a receiver of its kind exists (decisions/deck.md [address]: there is no interpreter, sending a word somewhere needs a possible receiver of the same type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressEntry {
	/// the reference – which thing; every entry has one, stable; what a typeahead picks and a document's fields are filled from: `character:catpope.human.default`, `location:dom`, `item:incense`, `chapter:3`, `chapter:3.intro`, `season:1`, `toon:toptoy`, `product:dago_hoodie`, `variable:global.coins`, `scene:nave`
	#[serde(rename = "ref")]
	pub r#ref: String,
	/// the address – who hears a word, in address.rs's grammar, the same text as `ref`; only where a receiver of the kind exists: character and item (deck-space far.rs), persona and look (the show's addressing, `picks_up`). A scene has one too (a deck plays it: a show cell's piece). A location, chapter, group or variable has none: nothing receives a word sent there today.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub address: Option<String>,
	pub kind: AddressKind,
	/// its own id at its level (`default` for the look above, `3` for a chapter, a group's label, a variable as written in this toon: `coins`, `global.coins`, `toptoy.coins`, a scene's id `nave`; its stage's name is its label under the toon's source language): what a document's field holds
	pub name: String,
	/// the reference one level up (a look's persona, a persona's character); absent at the top
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<String>,
	pub labels: LocaleText,
	pub origin: Origin,
	/// a look only: the moods it has art for, in the document's order
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub moods: Option<Vec<String>>,
	/// a scene only: the set its `space` node stages it in (`cathedral`), where the document says one
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub set: Option<String>,
	/// a scene only: the spot of that set it is placed at (`spot_altar`), where the document says one
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spot: Option<String>,
	/// a scene only: the published version listed (the document's stamped `version`, what `scene:<id>@<version>` pins)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub version: Option<i64>,
}

/// The document: one per toon, versioned by the catalog it was built from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressBook {
	/// the format's version (2: `ref` on every entry, `address` only where a receiver exists)
	pub address_book: i64,
	pub toon: String,
	/// the catalog's version it was built from (the response's ETag)
	pub version: String,
	/// the toon's source language: the second step of the locale order
	pub source: String,
	/// parents before their children; characters, locations, regions, items, seasons, chapters (each followed by its groups), products, toons (this one first), scenes (published, by id), variables
	pub entries: Vec<AddressEntry>,
}
