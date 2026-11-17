//! The stored scene document (#243, research/SCENES.md §3, §9.3): `scene:<id>` in the API's `entity_versions`. The one Rust
//! form of it serves
//! - the core's check before a publish (rig/validate.rs kind `scenes`, the build service's `POST /validate`): it parses
//!   here, passes `check` and resolves into a graph (scene_resolve.rs), or the API refuses it with these words;
//! - the JSON Schema the API guards a save with (deck-schema → server/schema/content/scene.schema.json, `[contract]`)
//!   and, from that schema, the TS and PHP types (server/schema/generate.ts → shared/api, server/src/Generated).
//!
//! **Format 2** (#489, SCENES.md §9.3; owner 2026-11-12 [scenes]: placeholder scenes, spawn zones, every fact of a filter
//! holds): beside the header (`format`, `id`, `version` stamped by the server, `toons`) a scene names
//! - its `location`: a reference `location:<id>`, or a placeholder (`id`, `fill`, `label`, `preview`) a use fills; the set
//!   follows from the location (one set, §8.4; the lookup is scene_resolve.rs, #512). A scene without one names its set on its `space` node
//!   (`set`, a neutral staging or a set not yet a location's); both at once is refused;
//! - its `holder`s: places on its stage, each carrying an authored filler (`ref`) or a placeholder's filter (`fill`), one
//!   filler (`take: one`) or a spawn zone's several (`take: many`, arranged by code – `arrange` – up to `max`), at a spot
//!   (missing: the scene's spot), with the filler's rig or a pinned one, values and preset choices, `enter: start | join`;
//! - its own graph (`node`, `link`: the space, mixers – the holders' rigs flatten in at resolve), `stage`, `outputs`.
//!
//! The being's look left the document: every character holder brings its own (the played `Scene` keeps a `being` until the
//! stage draws several looks, 2·1). Words (§8): spot, location, spawn zone – no `point`, no `place:`.
use crate::rig::address::{at_or_below, word, Address, Leaf, Place};
use crate::scene::{LinkDef, NodeDef, Scene, StageSettings};
use anyhow::{bail, Context, Result};
use protocol::address_book::AddressKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// The format this core reads and writes. A deck refuses a format it does not know (RULES A9: no back-compat; format 1
/// rows were moved by the migration 2026-11-12-scene-format-2).
pub const FORMAT: u32 = 2;

/// What a holder's `fill` may name: the address book's kinds (#477) of what stands on a stage. A nested scene as a filler
/// is later (SCENES.md §9.2), so `scene` is not among them yet.
pub const HOLDER_KINDS: [AddressKind; 4] = [AddressKind::Character, AddressKind::Persona, AddressKind::Look, AddressKind::Item];
/// What a location placeholder's `fill` names: a location only
pub const LOCATION_KINDS: [AddressKind; 1] = [AddressKind::Location];

/// a person's text: locale → text (the one shape of authored text)
pub type Label = BTreeMap<String, String>;

/// The neutral staging's spots (the `scene:stage` template's), in the order a new cast member takes them.
pub const STAGING_SPOTS: [&str; 4] = ["spot_left", "spot_right", "spot_centre", "spot_back"];

/// One `scene:<id>` document as the API stores it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDocument {
    /// the document's format: 2 (location, holders, spawn zones; #489)
    #[cfg_attr(feature = "schema", schemars(extend("const" = 2)))]
    pub format: u32,
    /// the key's id: `nave` for `scene:nave` – platform-wide, no toon in it (a scene can serve several toons and shows)
    pub id: String,
    /// the number the server stamps on every change (as a show's); absent until the first save
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// the toons whose address books list it; empty = a template, listed in every toon's book (#504, `is_template`)
    #[serde(default)]
    pub toons: Vec<String>,
    /// where it plays: `location:<id>`, or a placeholder a use fills; none = the `space` node's own `set` (or no set)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationSlot>,
    /// the places on its stage, in the order a story's `joinScene` tries them (SCENES.md §9.4)
    #[serde(default, rename = "holder", skip_serializing_if = "Vec::is_empty")]
    pub holders: Vec<Holder>,
    /// #517: what it does when a story event comes while it plays (research/SCENE-EVENTS.md §3.2): a story event's type,
    /// optionally its id and a character → join | leave (scene_play.rs `Playing::story`)
    #[serde(default, rename = "listener", skip_serializing_if = "Vec::is_empty")]
    pub listeners: Vec<Listener>,
    #[serde(default)]
    pub stage: StageSettings,
    /// the scene's own nodes (the space, mixers); the holders' rigs are flattened in by the resolve step
    #[serde(default, rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link")]
    pub links: Vec<LinkDef>,
    /// the parameters as outputs (#211, scene.rs `Scene::outputs`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<std::collections::BTreeMap<String, serde_json::Value>>"))]
    pub outputs: Option<toml::Table>,
    /// its primary camera: shots, track, moves (scene_camera.rs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<crate::scene_camera::SceneCamera>,
}

/// #517: a scene's listener – "listen to this event when this character …, then do this" (owner 2026-11-13 [scenes]). A story
/// event of type `on` (the chapter's `VNEvent` types, protocol::story_events), with this `id` when given, naming a character
/// this `character` admits when given (itself, or a persona / look below it), makes each such character `do` its word.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listener {
    /// the story event's type (`joinScene`, `leaveScene`, `showMessage` …)
    pub on: String,
    /// only the story event with this id (the creator's stable event id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// only the characters this reference admits (`character:catpope`: her and every persona of hers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    /// what each admitted character of the event does on this stage
    #[serde(rename = "do")]
    pub act: Act,
}

/// #517: what a listener makes a character do – the cast words of a playing scene (E1: rig/address.rs `CastWord`)
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Act {
    /// appears at the first free spot that admits it
    Join,
    /// disappears, its spot is free again
    Leave,
}

impl Act {
    /// the cast word's verb (`join:<who>`)
    pub fn word(self) -> &'static str {
        match self {
            Act::Join => "join",
            Act::Leave => "leave",
        }
    }
}

impl Listener {
    pub fn admits(&self, who: &str) -> bool {
        self.character.as_deref().is_none_or(|c| at_or_below(c, who))
    }

    /// its problems in the core's words (`who`: `listener 2`)
    fn problems(&self, who: &str, out: &mut Vec<String>) {
        use deck_events::adapters::story::STAYS_IN_THE_STORY;
        use protocol::story_events::STORY_EVENT_TYPES;
        if !STORY_EVENT_TYPES.contains(&self.on.as_str()) {
            out.push(format!("{who}: '{}' is no story event ({})", self.on, STORY_EVENT_TYPES.join(", ")));
        } else if STAYS_IN_THE_STORY.contains(&self.on.as_str()) {
            out.push(format!("{who}: '{}' stays in the story's runner – a deck never hears it", self.on));
        }
        if self.id.as_deref().is_some_and(|id| id.trim().is_empty()) {
            out.push(format!("{who}: `id` is empty"));
        }
        match self.character.as_deref().map(FillerRef::parse) {
            None => {}
            Some(Ok(f)) if matches!(f.kind, AddressKind::Character | AddressKind::Persona | AddressKind::Look) => {}
            Some(Ok(f)) => out.push(format!("{who}: `character` names {}, not a character", a(f.kind))),
            Some(Err(e)) => out.push(format!("{who}: `character`: {e}")),
        }
    }
}

/// The scene's location: a reference, or a placeholder.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocationSlot {
    /// `location:<id>`
    Ref(String),
    /// filled by a use (`with.<id>`): authored, or by context (a location's default scene: the location)
    Placeholder(LocationPlaceholder),
}

/// A location no scene names yet: what may fill it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationPlaceholder {
    /// its name in a use's `with` (`where`, `home`); not an address
    pub id: String,
    /// the filter: `kind` is `["location"]`
    pub fill: Fill,
    /// its name for people
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
    /// a filler for the editor and the browser
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// The filter of a placeholder (SCENES.md §9.2): the address book's typeahead question, stored. Within a fact any value
/// admits; every fact given must hold (owner 2026-11-12, §9.7 Q5).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    /// the address book's kinds that may fill it (required): a holder's of `character`, `persona`, `look`, `item`; a
    /// location's `location`
    #[cfg_attr(feature = "schema", schemars(schema_with = "fill_kinds_schema"))]
    pub kind: Vec<AddressKind>,
    /// a hand-picked list of references (`character:catpope`, `item:incense`); none = any of the kinds
    #[serde(default, rename = "ref", skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    /// only entries below this reference (`character:catpope`: her personas and looks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// One place on the stage (SCENE-PACKAGE §2, SCENES.md §9.3).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    /// its name in the document and in a use's `with` (`left`, `who`); not an address – a filled holder's address is its
    /// filler's place (`scene:stage/character:catpope/…`), an empty one has none
    pub id: String,
    /// the spot of the set where it stands and enters (`spot_left`); none = the scene's spot
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot: Option<String>,
    /// where it stands from its spot, metres (x across, y up, z deep): a member moved on the stage
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[f32; 3]>,
    /// an authored filler (`character:catpope`, `item:incense`) – or `fill`, never both
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub filler: Option<String>,
    /// a placeholder's filter – or `ref`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    /// `one` filler (the default) or `many`: a spawn zone (owner 2026-11-12 [scenes]) – a placeholder only
    #[serde(default, skip_serializing_if = "Take::is_one")]
    pub take: Take,
    /// a spawn zone's arrangement of its members within the spot (required with `take: many`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrange: Option<Arrange>,
    /// a spawn zone's limit; none = any number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// the rig its filler plays; none = the filler's own (`character:catpope` → `rig:character.catpope`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rig: Option<RigPin>,
    /// values and preset choices only (SCENE-PACKAGE §2), by address: `"voice:voice.index": 4.0`,
    /// `"voice:@preset": "cathedral_voice.bright"`
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, HolderValue>,
    /// a placeholder's name for people
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Label>,
    /// fillers for the editor and the browser (a spawn zone may preview several)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<Fillers>,
    /// `start` (the default): on stage when the scene starts; `join`: waits for the story (`joinScene`, SCENES.md §9.4)
    #[serde(default, skip_serializing_if = "Enter::is_start")]
    pub enter: Enter,
}

/// One filler or several (a spawn zone's members).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fillers {
    One(String),
    Many(Vec<String>),
}

impl Fillers {
    pub fn list(&self) -> Vec<&str> {
        match self {
            Fillers::One(r) => vec![r.as_str()],
            Fillers::Many(list) => list.iter().map(String::as_str).collect(),
        }
    }
}

/// How many fillers a holder takes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Take {
    /// one slot, one filler
    #[default]
    One,
    /// a spawn zone: several fillers of what its `fill` allows, arranged by code
    Many,
}

impl Take {
    fn is_one(&self) -> bool {
        *self == Take::One
    }
}

/// A spawn zone's arrangement: its kind and the spacing between members.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrange {
    pub kind: ArrangeKind,
    /// metres between neighbours (a set's spot is a point: the spacing says how wide the zone grows)
    #[serde(default = "default_spacing")]
    pub spacing: f32,
}

/// the default spacing between a spawn zone's members, in metres
pub const DEFAULT_SPACING: f32 = 0.8;

fn default_spacing() -> f32 {
    DEFAULT_SPACING
}

/// The arrangements of a first version (scene_resolve.rs `arrange`): more (a stack, a crowd) when a use needs one.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArrangeKind {
    /// side by side across the spot, centred on it (a choir on the altar, items on a table)
    Row,
    /// around the spot, the first in its middle, the next on a widening ring (crows on a beam's end, a crowd's core)
    Cluster,
}

/// A rig by reference and the version it was wired against (SCENE-PACKAGE §2).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigPin {
    /// `rig:<subject>` (`rig:character.catpope`)
    #[serde(rename = "ref")]
    pub rig: String,
    /// the version it was wired against: the rig's `version` as scene-content serves it (16 hex of its data's SHA-256 –
    /// the catalog keeps no version number); none = the current one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl RigPin {
    /// the rig's subject (`character.catpope`)
    pub fn subject(&self) -> Option<&str> {
        self.rig.strip_prefix("rig:").filter(|s| Place::of_subject(s).is_some())
    }
}

/// A holder's value: a parameter's number, or a track's preset choice.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HolderValue {
    Number(f32),
    Preset(String),
}

/// `start` or `join`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Enter {
    #[default]
    Start,
    Join,
}

impl Enter {
    fn is_start(&self) -> bool {
        *self == Enter::Start
    }
}

/// a kind with its article, for the problems' sentences
fn a(kind: AddressKind) -> String {
    let w = kind.as_str();
    format!("{} {w}", if w.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" })
}

/// the schema of a fill's `kind`: a non-empty list of the kinds a holder or a location takes (the address book's words)
#[cfg(feature = "schema")]
fn fill_kinds_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let kinds: Vec<&str> = HOLDER_KINDS.iter().chain(&LOCATION_KINDS).map(|k| k.as_str()).collect();
    let schema = serde_json::json!({ "type": "array", "items": { "type": "string", "enum": kinds }, "minItems": 1 });
    schemars::Schema::try_from(schema).expect("an object schema")
}

/// A filler's reference read: its kind (the address book's level) and its place on a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillerRef {
    pub kind: AddressKind,
    /// `character:catpope` for the character, a persona and a look of hers alike (one place per filler, address.rs)
    pub place: Place,
    /// a persona's id (`character:catpope.pope`), none at the character level
    pub persona: Option<String>,
}

impl FillerRef {
    /// `character:<id>[.<persona>[.<look>]]`, `item:<id>`, `location:<id>` – the address book's references of what fills a
    /// placeholder
    pub fn parse(r: &str) -> Result<FillerRef, String> {
        let Some((kind, rest)) = r.split_once(':') else { return Err(format!("'{r}' is not a reference (<kind>:<id>)")) };
        let levels: Vec<&str> = rest.split('.').collect();
        for l in &levels {
            word(l, &format!("the reference '{r}'"))?;
        }
        let (kind, deepest) = match kind {
            "character" => (AddressKind::Character, 3),
            "item" => (AddressKind::Item, 1),
            "location" => (AddressKind::Location, 1),
            other => return Err(format!("'{r}': a placeholder takes a character, a persona, a look, an item or a location, not a {other}")),
        };
        if levels.len() > deepest {
            return Err(format!("'{r}' has more levels than a {} reference", kind.as_str()));
        }
        let kind = match levels.len() {
            2 => AddressKind::Persona,
            3 => AddressKind::Look,
            _ => kind,
        };
        let place_kind = if matches!(kind, AddressKind::Persona | AddressKind::Look) { "character" } else { kind.as_str() };
        Ok(FillerRef { kind, place: Place::new(place_kind, levels[0]), persona: levels.get(1).map(|p| p.to_string()) })
    }

    /// the filler whose own rig `subject` is (`character.catpope.pope` → `character:catpope.pope`); none: no rig of
    /// what stands on a stage
    pub fn of_rig(subject: &str) -> Option<String> {
        let (kind, rest) = subject.split_once('.')?;
        let filler = format!("{kind}:{rest}");
        FillerRef::parse(&filler).ok().filter(|f| HOLDER_KINDS.contains(&f.kind) && f.rig_subject().as_deref() == Some(subject)).map(|_| filler)
    }

    /// the subject of the filler's own rig (`character.catpope`, `character.catpope.pope`, `item.incense_burner`); a
    /// location has none
    pub fn rig_subject(&self) -> Option<String> {
        match self.place.kind.as_str() {
            "location" => None,
            k => Some(match &self.persona {
                Some(p) => format!("{k}.{}.{p}", self.place.id),
                None => format!("{k}.{}", self.place.id),
            }),
        }
    }
}

impl Fill {
    pub fn admits(&self, filler: &str) -> Result<(), String> {
        let f = FillerRef::parse(filler)?;
        if !self.kind.contains(&f.kind) {
            let kinds: Vec<&str> = self.kind.iter().map(|k| k.as_str()).collect();
            return Err(format!("'{filler}' is {}, the placeholder takes {}", a(f.kind), kinds.join(" or ")));
        }
        if !self.refs.is_empty() && !self.refs.iter().any(|r| r == filler) {
            return Err(format!("'{filler}' is not one of {}", self.refs.join(", ")));
        }
        if let Some(parent) = &self.parent {
            if filler == parent.as_str() || !at_or_below(parent, filler) {
                return Err(format!("'{filler}' is not below '{parent}'"));
            }
        }
        Ok(())
    }

    /// the filter's own problems: kinds named and of the ones `allowed`, references well formed and of a kind it names
    fn problems(&self, allowed: &[AddressKind], who: &str, out: &mut Vec<String>) {
        if self.kind.is_empty() {
            out.push(format!("{who}: `fill.kind` names no kind"));
        }
        let names: Vec<&str> = allowed.iter().map(|k| k.as_str()).collect();
        for k in self.kind.iter().filter(|k| !allowed.contains(k)) {
            out.push(format!("{who}: {} cannot fill it ({})", a(*k), names.join(", ")));
        }
        for r in &self.refs {
            match FillerRef::parse(r) {
                Ok(f) if !self.kind.contains(&f.kind) => out.push(format!("{who}: `fill.ref` '{r}' is {}, a kind the filter does not take", a(f.kind))),
                Ok(_) => {}
                Err(e) => out.push(format!("{who}: {e}")),
            }
        }
        if let Some(p) = &self.parent {
            if let Err(e) = FillerRef::parse(p) {
                out.push(format!("{who}: `fill.parent`: {e}"));
            }
        }
    }
}

impl Holder {
    /// a placeholder: filled by a use, not authored
    pub fn is_placeholder(&self) -> bool {
        self.fill.is_some()
    }

    /// room for one more filler beside `taken`
    pub fn has_room(&self, taken: usize) -> bool {
        match self.take {
            Take::One => taken == 0,
            Take::Many => self.max.is_none_or(|m| taken < m as usize),
        }
    }
}

impl SceneDocument {
    /// A played scene (TOML, a sandbox) as a document of this format: its graph, stage and outputs; its being is no
    /// document field (a character holder brings its look). No version yet: the server stamps it.
    pub fn from_scene(id: &str, toons: Vec<String>, scene: &Scene) -> SceneDocument {
        SceneDocument {
            format: FORMAT,
            id: id.to_string(),
            version: None,
            toons,
            location: None,
            holders: Vec::new(),
            listeners: Vec::new(),
            stage: scene.stage.clone(),
            nodes: scene.nodes.clone(),
            links: scene.links.clone(),
            outputs: scene.outputs.clone(),
            camera: scene.camera.clone(),
        }
    }

    /// A new scene as an editor makes it: its id, its stage's name, the toons whose books list it, a `space` node at the
    /// staging's centre and the stage's defaults; no cast yet (`recast`). `check_new` says what is wrong with it.
    pub fn new(id: &str, name: &str, toons: Vec<String>) -> SceneDocument {
        let mut space = toml::Table::new();
        space.insert("spot".into(), toml::Value::String(STAGING_SPOTS[2].into()));
        SceneDocument {
            format: FORMAT,
            id: id.to_string(),
            version: None,
            toons,
            location: None,
            holders: Vec::new(),
            listeners: Vec::new(),
            stage: StageSettings { name: name.trim().to_string(), ..StageSettings::default() },
            nodes: vec![NodeDef { id: "space".into(), kind: "space".into(), params: space }],
            links: Vec::new(),
            outputs: None,
            camera: None,
        }
    }

    /// the authored fillers on its stage, in the holders' order (`character:catpope`)
    pub fn cast(&self) -> Vec<&str> {
        self.holders.iter().filter_map(|h| h.filler.as_deref()).collect()
    }

    /// The cast set to `cast` (fillers, `character:catpope`): a holder whose filler stays keeps its spot, rig and values; one
    /// whose filler left goes; a new one stands at the first staging spot no holder takes, named by its filler's place.
    /// Placeholders stay as they are.
    pub fn recast(&mut self, cast: &[String]) {
        self.holders.retain(|h| h.filler.as_ref().is_none_or(|f| cast.contains(f)));
        for filler in cast {
            if self.holders.iter().any(|h| h.filler.as_ref() == Some(filler)) {
                continue;
            }
            let base = FillerRef::parse(filler).map(|f| f.persona.map_or(f.place.id.clone(), |p| format!("{}_{p}", f.place.id))).unwrap_or_else(|_| "member".into());
            let mut id = base.clone();
            for n in 2.. {
                if !self.holders.iter().any(|h| h.id == id) && self.location_placeholder().is_none_or(|p| p.id != id) {
                    break;
                }
                id = format!("{base}_{n}");
            }
            let spot = STAGING_SPOTS.iter().find(|s| !self.holders.iter().any(|h| h.spot.as_deref() == Some(**s))).map(|s| s.to_string());
            self.holders.push(Holder { id, spot, filler: Some(filler.clone()), ..Holder::default() });
        }
    }

    /// `check`, and what a new scene needs beside it: an id a `scene:` key takes, a stage name, a toon, a cast.
    pub fn check_new(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !crate::rig::content::safe_id(&self.id) || self.id.contains('.') {
            out.push(format!("the id '{}' is no scene id: letters, digits, '-' and '_'", self.id));
        }
        if self.stage.name.trim().is_empty() {
            out.push("the scene needs a name".into());
        }
        if self.toons.is_empty() {
            out.push("open a toon first: a scene without one is a template, in every toon's book".into());
        }
        if self.holders.is_empty() {
            out.push("the cast is empty: pick who stands on its stage".into());
        }
        out.extend(self.check());
        out
    }

    /// The document from its JSON: the format first (a format this core does not read is said as such), then the fields,
    /// unknown ones refused.
    pub fn from_value(value: Value) -> Result<SceneDocument> {
        let Value::Object(fields) = &value else { bail!("a scene document is a JSON object") };
        match fields.get("format") {
            Some(v) if v.as_u64() == Some(u64::from(FORMAT)) => {}
            Some(v) => bail!("format {v} is not one this core reads (it reads {FORMAT})"),
            None => bail!("`format` is missing ({FORMAT})"),
        }
        if !fields.get("id").and_then(Value::as_str).is_some_and(|id| !id.is_empty()) {
            bail!("`id` is missing: the id of the key `scene:<id>`");
        }
        serde_json::from_value(value).context("the scene document")
    }

    pub fn parse_json(text: &str) -> Result<SceneDocument> {
        SceneDocument::from_value(serde_json::from_str(text).context("a scene document is JSON")?)
    }

    /// The document as JSON text (f32 values written as f32, so `0.15` stays `0.15`; a `serde_json::Value` would widen
    /// them first)
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// the `space` node, if the scene has one
    pub fn space(&self) -> Option<&NodeDef> {
        self.nodes.iter().find(|n| n.kind == "space")
    }

    /// A template (SCENES.md §9.5, owner 2026-11-12 [scenes] §9.7 Q2): a scene with no toon – every toon's book lists it
    /// (#504). The one definition (#509): the address book reads it from the stored `toons`, the shared vectors
    /// (server/tests/fixtures/scenes/book-entries.json) hold both to it.
    pub fn is_template(&self) -> bool {
        self.toons.is_empty()
    }

    /// Its placeholders as the toon's book lists them (#504, #509; the address book's generated `ScenePlaceholder`): the
    /// location's first when it is one, then each holder with a `fill`, in the document's order – what a use may fill by
    /// `id`. A preview is a list (a spawn zone may preview several).
    pub fn placeholders(&self) -> Vec<protocol::address_book::ScenePlaceholder> {
        use protocol::address_book::{PlaceholderFill, ScenePlaceholder};
        let one = |id: &str, fill: &Fill, label: &Option<Label>, preview: Vec<String>| ScenePlaceholder {
            id: id.to_string(),
            fill: PlaceholderFill {
                kind: fill.kind.clone(),
                r#ref: (!fill.refs.is_empty()).then(|| fill.refs.clone()),
                parent: fill.parent.clone(),
            },
            label: label.as_ref().filter(|l| !l.is_empty()).map(|l| l.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            preview: (!preview.is_empty()).then_some(preview),
        };
        let location = self.location_placeholder().map(|p| one(&p.id, &p.fill, &p.label, p.preview.iter().cloned().collect()));
        let holders = self.holders.iter().filter_map(|h| {
            let preview = h.preview.as_ref().map(|p| p.list().into_iter().map(String::from).collect()).unwrap_or_default();
            h.fill.as_ref().map(|f| one(&h.id, f, &h.label, preview))
        });
        location.into_iter().chain(holders).collect()
    }

    /// the location placeholder's id, if the location is one
    pub fn location_placeholder(&self) -> Option<&LocationPlaceholder> {
        match &self.location {
            Some(LocationSlot::Placeholder(p)) => Some(p),
            _ => None,
        }
    }

    /// The core's checks of the document beyond its shape (SCENES.md §9.3, §9.6 2·0): the location, the holders' ids,
    /// fillers and filters, spawn zones, rigs and values. Empty = well formed; the graph is checked when it resolves.
    pub fn check(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(e) = self.stage.background.as_ref().and_then(deck_catalog::Background::check) {
            out.push(format!("stage background: {e}"));
        }
        if self.stage.layers.len() > deck_catalog::layer::MOST {
            out.push(format!("stage layers: at most {}, not {}", deck_catalog::layer::MOST, self.stage.layers.len()));
        }
        for (i, e) in self.stage.layers.iter().enumerate().filter_map(|(i, l)| Some((i, l.check()?))) {
            out.push(format!("stage layer {}: {e}", i + 1));
        }
        let set = self.space().and_then(|s| s.params.get("set")).is_some();
        match &self.location {
            None => {}
            Some(LocationSlot::Ref(r)) => match FillerRef::parse(r) {
                Ok(f) if f.kind == AddressKind::Location => {}
                Ok(f) => out.push(format!("location: '{r}' is {}, not a location", a(f.kind))),
                Err(e) => out.push(format!("location: {e}")),
            },
            Some(LocationSlot::Placeholder(p)) => {
                let who = format!("location {}", p.id);
                if let Err(e) = word(&p.id, "the location placeholder's id") {
                    out.push(format!("location: {e}"));
                }
                p.fill.problems(&LOCATION_KINDS, &who, &mut out);
                if let Some(e) = p.preview.as_deref().and_then(|r| p.fill.admits(r).err()) {
                    out.push(format!("{who}: its preview: {e}"));
                }
            }
        }
        if self.location.is_some() {
            if set {
                out.push("a scene with a location takes its set from it: the `space` node's `set` is refused beside it".into());
            }
            if self.space().is_none() {
                out.push("a scene with a location needs a `space` node to stage it".into());
            }
        }
        let mut ids = BTreeSet::new();
        if let Some(p) = self.location_placeholder() {
            ids.insert(p.id.as_str());
        }
        let mut places = HashSet::new();
        for h in &self.holders {
            let who = format!("holder {}", h.id);
            if let Err(e) = word(&h.id, "a holder's id") {
                out.push(format!("holder: {e}"));
            }
            if !ids.insert(h.id.as_str()) {
                out.push(format!("{who}: the id is taken twice"));
            }
            if let Some(Err(e)) = h.spot.as_deref().map(|s| word(s, "a spot")) {
                out.push(format!("{who}: {e}"));
            }
            if h.at.is_some_and(|a| a.iter().any(|v| !v.is_finite())) {
                out.push(format!("{who}: `at` is three numbers (metres from its spot)"));
            }
            match (&h.filler, &h.fill) {
                (Some(_), Some(_)) => out.push(format!("{who}: an authored filler (`ref`) or a placeholder (`fill`), not both")),
                (None, None) => out.push(format!("{who}: neither a filler (`ref`) nor a placeholder (`fill`)")),
                (Some(r), None) => match FillerRef::parse(r) {
                    Ok(f) if !HOLDER_KINDS.contains(&f.kind) => out.push(format!("{who}: {} cannot stand on a stage", a(f.kind))),
                    // the same filler twice on one stage: `#2` is reserved (address.rs)
                    Ok(f) if !places.insert(f.place.clone()) => out.push(format!("{who}: '{r}' stands on this stage already (a second copy is reserved)")),
                    Ok(_) => {}
                    Err(e) => out.push(format!("{who}: {e}")),
                },
                (None, Some(fill)) => fill.problems(&HOLDER_KINDS, &who, &mut out),
            }
            match h.take {
                Take::One => {
                    if h.arrange.is_some() || h.max.is_some() {
                        out.push(format!("{who}: `arrange` and `max` belong to a spawn zone (`take: many`)"));
                    }
                }
                Take::Many => {
                    if h.fill.is_none() {
                        out.push(format!("{who}: a spawn zone is a placeholder: it needs `fill`"));
                    }
                    if h.arrange.is_none() {
                        out.push(format!("{who}: a spawn zone needs `arrange` (row, cluster)"));
                    }
                    if h.max == Some(0) {
                        out.push(format!("{who}: `max` is at least 1"));
                    }
                    if h.arrange.is_some_and(|a| a.spacing.is_nan() || a.spacing <= 0.0) {
                        out.push(format!("{who}: `arrange.spacing` is more than 0"));
                    }
                }
            }
            if let (Some(preview), Some(fill)) = (&h.preview, &h.fill) {
                let list = preview.list();
                if (h.take == Take::One && !matches!(preview, Fillers::One(_))) || !h.has_room(list.len().saturating_sub(1)) {
                    out.push(format!("{who}: more previews than it takes"));
                }
                for e in list.iter().filter_map(|r| fill.admits(r).err()) {
                    out.push(format!("{who}: its preview: {e}"));
                }
            } else if h.preview.is_some() {
                out.push(format!("{who}: a preview belongs to a placeholder"));
            }
            if let Some(rig) = &h.rig {
                if rig.subject().is_none() {
                    out.push(format!("{who}: the rig '{}' is no `rig:<subject>` (rig:character.catpope)", rig.rig));
                }
            }
            for (path, value) in &h.values {
                let ok = match (Address::leaf(path), value) {
                    (Ok(Leaf::Port { track: Some(_), .. }), HolderValue::Number(_)) => true,
                    (Ok(Leaf::Member { member, .. }), HolderValue::Preset(_)) => member == "preset",
                    _ => false,
                };
                if !ok {
                    out.push(format!("{who}: value '{path}' is neither `<track>:<node>.<port>` with a number nor `<track>:@preset` with a preset"));
                }
            }
        }
        for (i, l) in self.listeners.iter().enumerate() {
            l.problems(&format!("listener {}", i + 1), &mut out);
        }
        if let Some(c) = &self.camera {
            let holders: Vec<&str> = self.holders.iter().map(|h| h.id.as_str()).collect();
            c.problems(&holders, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{builtin, BUILTIN};

    /// #504 / #509: the shared vectors the address book is checked against (AddressBookApiTest) – the core writes whether a
    /// scene is a template and its placeholders as the book lists them; `UPDATE_FIXTURES=1` rewrites them
    #[test]
    fn book_entries() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes");
        let path = format!("{dir}/book-entries.json");
        let stored = std::fs::read_to_string(&path).unwrap().replace('\r', "");
        let mut set: Value = serde_json::from_str(&stored).unwrap();
        for case in set["cases"].as_array_mut().unwrap() {
            let doc = match case.get("file").and_then(Value::as_str) {
                Some(f) => SceneDocument::parse_json(&std::fs::read_to_string(format!("{dir}/{f}")).unwrap()).unwrap(),
                None => SceneDocument::from_value(case["scene"].clone()).unwrap(),
            };
            assert_eq!(doc.check(), Vec::<String>::new(), "{}", case["name"]);
            case["template"] = Value::Bool(doc.is_template());
            case["placeholders"] = serde_json::to_value(doc.placeholders()).unwrap();
        }
        let written = format!("{}\n", serde_json::to_string_pretty(&set).unwrap());
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(&path, &written).unwrap();
        } else {
            assert_eq!(stored, written, "book-entries.json is stale: UPDATE_FIXTURES=1 cargo test -p deck-space book_entries");
        }
    }

    /// every scene the repo carries survives TOML → document JSON → document unchanged (its graph, stage and outputs: the
    /// being is no document field)
    #[test]
    fn every_bundled_scene_round_trips_through_json() {
        let sandboxes = crate::scene::SANDBOX.iter().map(|n| (*n, crate::scene::sandbox(n).unwrap()));
        let scenes = BUILTIN.iter().map(|n| (*n, builtin(n).unwrap())).chain([("staging", crate::scene::STAGING_SCENE)]).chain(sandboxes);
        for (name, text) in scenes {
            let scene = Scene::parse(text).unwrap();
            let doc = SceneDocument::from_scene(name, vec!["catpope".into()], &scene);
            let back = SceneDocument::parse_json(&doc.to_json().unwrap()).unwrap();
            assert_eq!(back, doc, "{name}");
        }
    }

    #[test]
    fn the_header_is_checked_and_unknown_fields_are_refused() {
        let ok = r#"{"format":2,"id":"x","toons":[],"node":[{"id":"t","kind":"tempo","bpm":120}]}"#;
        let doc = SceneDocument::parse_json(ok).unwrap();
        assert_eq!((doc.format, doc.id.as_str(), doc.version, doc.nodes.len()), (2, "x", None, 1));
        assert!(SceneDocument::parse_json(r#"{"format":1,"id":"x"}"#).unwrap_err().to_string().contains("format 1"), "format 1 is migrated, not read");
        assert!(SceneDocument::parse_json(r#"{"id":"x"}"#).is_err(), "no format");
        assert!(SceneDocument::parse_json(r#"{"format":2}"#).is_err(), "no id");
        assert!(SceneDocument::parse_json(r#"{"format":2,"id":"x","nodes":[]}"#).is_err(), "a typo is refused, not dropped");
        assert!(SceneDocument::parse_json(r#"{"format":2,"id":"x","version":-1}"#).is_err());
        assert_eq!(SceneDocument::parse_json(r#"{"format":2,"id":"x","version":3}"#).unwrap().version, Some(3));
        assert!(SceneDocument::parse_json(r#"{"format":2,"id":"x","being":{"character":"catpope"}}"#).is_err(), "the being left the document");
    }

    /// #489: every field of format 2 survives JSON → document → JSON
    #[test]
    fn format_2_round_trips() {
        let text = r#"{"format":2,"id":"choir","toons":["catpope"],
          "location":{"id":"where","fill":{"kind":["location"]},"label":{"en":"where"},"preview":"location:cathedral"},
          "holder":[
            {"id":"pope","spot":"spot_altar","ref":"character:catpope","rig":{"ref":"rig:character.catpope","version":"1f0e9a2b3c4d5e6f"},
             "values":{"voice:voice.index":4.0,"voice:@preset":"cathedral_voice.bright"}},
            {"id":"crows","spot":"spot_beam","fill":{"kind":["character","item"],"ref":["item:crow","character:toptoy"]},
             "take":"many","arrange":{"kind":"row","spacing":0.5},"max":3,"preview":["item:crow"],"enter":"join","label":{"en":"crows"}},
            {"id":"guest","fill":{"kind":["persona","look"],"parent":"character:catpope"},"preview":"character:catpope.pope"}],
          "stage":{"name":"Choir"},"node":[{"id":"space","kind":"space","spot":"spot_altar"}],"link":[]}"#;
        let doc = SceneDocument::parse_json(text).unwrap();
        assert_eq!(doc.check(), Vec::<String>::new());
        assert_eq!(doc.holders[1].arrange, Some(Arrange { kind: ArrangeKind::Row, spacing: 0.5 }));
        assert_eq!(doc.holders[0].values["voice:voice.index"], HolderValue::Number(4.0));
        assert!(matches!(doc.location, Some(LocationSlot::Placeholder(ref p)) if p.id == "where"));
        let back = SceneDocument::parse_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(back, doc);
        let r: Value = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
        assert!(r["holder"][0].get("take").is_none() && r["holder"][0].get("enter").is_none(), "defaults are not written");
        let at: SceneDocument = SceneDocument::parse_json(r#"{"format":2,"id":"x","location":"location:dom","node":[{"id":"space","kind":"space"}]}"#).unwrap();
        assert_eq!(at.location, Some(LocationSlot::Ref("location:dom".into())));
        assert_eq!(at.check(), Vec::<String>::new());
    }

    /// the filter: every fact given holds, within a fact any value admits
    #[test]
    fn a_fill_admits_only_what_every_fact_allows() {
        let fill = |json: &str| -> Fill { serde_json::from_str(json).unwrap() };
        let characters = fill(r#"{"kind":["character"]}"#);
        assert_eq!(characters.admits("character:catpope"), Ok(()));
        assert!(characters.admits("item:incense").unwrap_err().contains("is an item, the placeholder takes character"));
        assert!(characters.admits("character:catpope.pope").unwrap_err().contains("is a persona"));
        let picked = fill(r#"{"kind":["character","item"],"ref":["character:catpope","item:incense"]}"#);
        assert_eq!(picked.admits("item:incense"), Ok(()));
        assert!(picked.admits("character:toptoy").unwrap_err().contains("not one of"));
        let hers = fill(r#"{"kind":["persona","look"],"parent":"character:catpope"}"#);
        assert_eq!(hers.admits("character:catpope.pope.default"), Ok(()));
        assert!(hers.admits("character:catpopes.pope").unwrap_err().contains("not below"), "a prefix of the id is not below it");
        assert!(hers.admits("character:catpope").is_err(), "the parent itself is not below itself (and a character)");
        assert!(characters.admits("scene:nave").unwrap_err().contains("not a scene"));
        assert!(characters.admits("character:a.b.c.d").is_err());
        let f = FillerRef::parse("character:catpope.pope.default").unwrap();
        assert_eq!((f.kind, f.place.kind.as_str(), f.place.id.as_str(), f.rig_subject().as_deref()), (AddressKind::Look, "character", "catpope", Some("character.catpope.pope")));
        assert_eq!(FillerRef::parse("item:incense_burner").unwrap().rig_subject().as_deref(), Some("item.incense_burner"));
    }

    /// the core's checks: what the shape allows but a scene cannot mean
    #[test]
    fn the_checks_say_what_is_wrong() {
        let problems = |holders: &str, extra: &str| -> Vec<String> {
            let text = format!(r#"{{"format":2,"id":"x","holder":{holders}{extra},"node":[{{"id":"space","kind":"space"}}]}}"#);
            SceneDocument::parse_json(&text).unwrap().check()
        };
        let one = |p: Vec<String>, has: &str| assert!(p.len() == 1 && p[0].contains(has), "{has}: {p:?}");
        one(problems(r#"[{"id":"a"}]"#, ""), "neither a filler");
        one(problems(r#"[{"id":"a","ref":"character:catpope","fill":{"kind":["character"]}}]"#, ""), "not both");
        one(problems(r#"[{"id":"a","ref":"character:catpope"},{"id":"a","ref":"item:x"}]"#, ""), "taken twice");
        one(problems(r#"[{"id":"a","ref":"character:catpope"},{"id":"b","ref":"character:catpope.pope"}]"#, ""), "a second copy is reserved");
        one(problems(r#"[{"id":"a b","ref":"item:x"}]"#, ""), "holder's id");
        one(problems(r#"[{"id":"a","ref":"location:dom"}]"#, ""), "cannot stand on a stage");
        one(problems(r#"[{"id":"a","fill":{"kind":["location"]}}]"#, ""), "a location cannot fill it");
        one(problems(r#"[{"id":"a","fill":{"kind":[]}}]"#, ""), "names no kind");
        one(problems(r#"[{"id":"a","fill":{"kind":["character"],"ref":["item:x"]}}]"#, ""), "a kind the filter does not take");
        one(problems(r#"[{"id":"a","fill":{"kind":["character"]},"take":"many"}]"#, ""), "needs `arrange`");
        one(problems(r#"[{"id":"a","ref":"item:x","take":"many","arrange":{"kind":"row"}}]"#, ""), "needs `fill`");
        one(problems(r#"[{"id":"a","fill":{"kind":["item"]},"max":2}]"#, ""), "belong to a spawn zone");
        one(problems(r#"[{"id":"a","fill":{"kind":["item"]},"take":"many","arrange":{"kind":"cluster"},"max":1,"preview":["item:x","item:y"]}]"#, ""), "more previews");
        one(problems(r#"[{"id":"a","fill":{"kind":["item"]},"preview":"character:catpope"}]"#, ""), "its preview");
        one(problems(r#"[{"id":"a","ref":"item:x","rig":{"ref":"character.catpope"}}]"#, ""), "no `rig:<subject>`");
        one(problems(r#"[{"id":"a","ref":"item:x","values":{"voice_voice.index":1.0}}]"#, ""), "value 'voice_voice.index'");
        one(problems(r#"[{"id":"a","ref":"item:x","values":{"voice:@preset":1.0}}]"#, ""), "value 'voice:@preset'");
        one(problems(r#"[{"id":"where","ref":"item:x"}]"#, r#","location":{"id":"where","fill":{"kind":["location"]}}"#), "taken twice");
        one(problems("[]", r#","location":"character:catpope""#), "not a location");
        let both = r#"{"format":2,"id":"x","location":"location:dom","node":[{"id":"space","kind":"space","set":"cathedral"}]}"#;
        assert!(SceneDocument::parse_json(both).unwrap().check()[0].contains("`set` is refused"));
        let bare = r#"{"format":2,"id":"x","location":"location:dom"}"#;
        assert!(SceneDocument::parse_json(bare).unwrap().check()[0].contains("needs a `space` node"));
    }

    /// #517: a listener names a story event a deck hears, a character reference and join / leave; anything else in the core's words
    #[test]
    fn listeners_are_checked() {
        let doc = |l: &str| SceneDocument::parse_json(&format!(r#"{{"format":2,"id":"x","listener":{l}}}"#));
        let ok = doc(r#"[{"on":"joinScene","do":"join"},{"on":"leaveScene","id":"e7","character":"character:catpope.pope","do":"leave"}]"#).unwrap();
        assert_eq!(ok.check(), Vec::<String>::new());
        assert_eq!(ok.listeners[1].act, Act::Leave);
        assert!(ok.to_json().unwrap().contains("\"listener\""), "written back under its name");
        let said = |l: &str| doc(l).unwrap().check();
        assert_eq!(said(r#"[{"on":"dance","do":"join"}]"#)[0], format!("listener 1: 'dance' is no story event ({})", protocol::story_events::STORY_EVENT_TYPES.join(", ")));
        assert_eq!(said(r#"[{"on":"jumpTo","do":"join"}]"#), ["listener 1: 'jumpTo' stays in the story's runner – a deck never hears it"]);
        assert_eq!(said(r#"[{"on":"joinScene","character":"item:incense","do":"join"}]"#), ["listener 1: `character` names an item, not a character"]);
        assert!(said(r#"[{"on":"joinScene","character":"catpope","do":"join"}]"#)[0].starts_with("listener 1: `character`: 'catpope' is not a reference"));
        assert_eq!(said(r#"[{"on":"joinScene","id":" ","do":"join"}]"#), ["listener 1: `id` is empty"]);
        assert!(doc(r#"[{"on":"joinScene","do":"dance"}]"#).is_err(), "join or leave only");
        // what a listener admits: any character without one, else it and what is below it
        let l = |c: Option<&str>| Listener { on: "joinScene".into(), id: None, character: c.map(String::from), act: Act::Join };
        assert!(l(None).admits("character:toptoy"));
        assert!(l(Some("character:catpope")).admits("character:catpope.pope") && l(Some("character:catpope")).admits("character:catpope"));
        assert!(!l(Some("character:catpope")).admits("character:catpopet") && !l(Some("character:catpope.pope")).admits("character:catpope"));
    }

    /// a scene made in the editor: two members at the staging's spots, checked by the core and resolved with the deck's
    /// core rigs; a recast keeps who stays where they stand; what a new scene lacks is said
    #[test]
    fn a_new_scene_with_two_members() {
        use crate::scene_resolve::{core_rigs, no_sets, resolve};
        assert_eq!(FillerRef::of_rig("character.catpope.pope").as_deref(), Some("character:catpope.pope"));
        assert_eq!(FillerRef::of_rig("item.incense_burner").as_deref(), Some("item:incense_burner"));
        assert_eq!(FillerRef::of_rig("location.dom"), None);
        let mut doc = SceneDocument::new("duet", " Duet ", vec!["catpope".into()]);
        assert_eq!(doc.check_new(), ["the cast is empty: pick who stands on its stage"]);
        doc.recast(&["character:catpope".into(), "item:incense_burner".into()]);
        assert_eq!(doc.check_new(), Vec::<String>::new());
        let at: Vec<(&str, Option<&str>, &str)> = doc.holders.iter().map(|h| (h.id.as_str(), h.spot.as_deref(), h.filler.as_deref().unwrap())).collect();
        assert_eq!(at, [("catpope", Some("spot_left"), "character:catpope"), ("incense_burner", Some("spot_right"), "item:incense_burner")]);
        assert_eq!(doc.stage.name, "Duet");
        let r = resolve(&doc, &BTreeMap::new(), false, &core_rigs, &no_sets).unwrap();
        assert_eq!(r.cast.iter().map(|m| m.place.as_str()).collect::<Vec<_>>(), ["character:catpope", "item:incense_burner"]);
        crate::graph::Graph::build(&r.scene).expect("the stage builds it");
        // written and read back as the server stores it
        assert_eq!(SceneDocument::parse_json(&doc.to_json().unwrap()).unwrap(), doc);
        // the incense burner leaves, the nun's persona joins at the free spot; catpope keeps hers
        doc.holders[0].values.insert("drone:drone.gain".into(), HolderValue::Number(0.3));
        doc.recast(&["character:catpope".into(), "character:nun.sister".into()]);
        let at: Vec<(&str, Option<&str>)> = doc.holders.iter().map(|h| (h.id.as_str(), h.spot.as_deref())).collect();
        assert_eq!(at, [("catpope", Some("spot_left")), ("nun_sister", Some("spot_right"))]);
        assert_eq!(doc.holders[0].values.len(), 1);
        assert_eq!(doc.cast(), ["character:catpope", "character:nun.sister"]);
        assert_eq!(doc.check(), Vec::<String>::new());
        let bad = SceneDocument::new("a.b", "", Vec::new());
        assert_eq!(bad.check_new()[..3], ["the id 'a.b' is no scene id: letters, digits, '-' and '_'", "the scene needs a name", "open a toon first: a scene without one is a template, in every toon's book"]);
    }

    /// #243 / SCENES.md §8.2: a nudge names its control input as `input` in the TOML too (no alias)
    #[cfg(feature = "being")]
    #[test]
    fn a_nudge_says_input() {
        assert!(Scene::parse("[being]\nreact = [{ mood = \"happy\", set = \"space.hue\", by = 0.1 }]\n").is_err(), "no alias in TOML");
        assert_eq!(Scene::parse("[being]\nreact = [{ mood = \"happy\", input = \"space.hue\", by = 0.1 }]\n").unwrap().being.react[0].input, "space.hue");
    }
}
