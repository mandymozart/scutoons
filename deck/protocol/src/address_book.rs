// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One toon's address book (#477, decisions/deck.md [contract] [typeahead]): every addressable thing of the toon's catalog with its address, kind, labels and where it comes from. The server writes it (GET /api/toons/<toon>/addresses), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// A payload whose shape is not fixed yet: any JSON object.
pub type Loose = serde_json::Map<String, serde_json::Value>;

/// What an entry is – the one thing a typeahead filters by: an asker takes the kinds it can address (a VN character event: character, persona, look; unlockLocation: location; an item event: item; a jump: chapter, group; setVariable: variable). character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); location, item and chapter are a place `<kind>:<id>`. Proposals the owner has not confirmed (#477): a chapter's group `chapter:<number>.<group>` (its `name` the label as an event stores it, the address word a safe form of it) and a story variable `variable:<scope>.<name>` (scope = the owning toon or `global`; its `name` the variable as written in this toon).
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
}

impl AddressKind {
	pub fn as_str(self) -> &'static str {
		match self {
			AddressKind::Character => "character",
			AddressKind::Persona => "persona",
			AddressKind::Look => "look",
			AddressKind::Location => "location",
			AddressKind::Item => "item",
			AddressKind::Chapter => "chapter",
			AddressKind::Group => "group",
			AddressKind::Variable => "variable",
		}
	}
}

/// `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OriginState {
	#[serde(rename = "published")]
	Published,
	#[serde(rename = "draft")]
	Draft,
}

impl OriginState {
	pub fn as_str(self) -> &'static str {
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
	/// the document's entity key (`character:catpope`, `place:dom`, `chapter:catpope:3`) or catalog entry (`item:incense`)
	pub key: String,
	pub state: OriginState,
}

/// One addressable thing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressEntry {
	/// the address, in address.rs's grammar: `character:catpope.human.default`, `location:dom`, `item:incense`, `chapter:3`, `chapter:3.intro`, `variable:global.coins`
	pub address: String,
	pub kind: AddressKind,
	/// its own id at its level (`default` for the look above, `3` for a chapter, a group's label, a variable as written in this toon: `coins`, `global.coins`, `toptoy.coins`): what a document's field holds
	pub name: String,
	/// the address one level up (a look's persona, a persona's character); absent at the top
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<String>,
	pub labels: LocaleText,
	pub origin: Origin,
	/// a look only: the moods it has art for, in the document's order
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub moods: Option<Vec<String>>,
}

/// The document: one per toon, versioned by the catalog it was built from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressBook {
	/// the format's version
	pub address_book: i64,
	pub toon: String,
	/// the catalog's version it was built from (the response's ETag)
	pub version: String,
	/// the toon's source language: the second step of the locale order
	pub source: String,
	/// parents before their children; characters, locations, items, chapters (each followed by its groups), variables
	pub entries: Vec<AddressEntry>,
}
