//! Slots (research/MODULE-EXTENSIONS.md §3): where a module can attach. Every entity kind declares its slots – the domain a
//! module there runs in, what it reads and what it may write, always inside that entity. A module's descriptor names the
//! one slot it is built for (`camera.motion`); an entity's document attaches modules to its slots (`Attach`), checked
//! here against the table and the module's descriptor.
use crate::{Descriptor, Domain};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a slot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    /// a character, its persona or its look
    Character,
    Item,
    Location,
    Scene,
    Camera,
    /// a show's row or one of its cells
    Row,
    /// an outlet or a deck
    Outlet,
    Device,
}

impl Entity {
    pub const ALL: [Entity; 8] = [Entity::Character, Entity::Item, Entity::Location, Entity::Scene, Entity::Camera, Entity::Row, Entity::Outlet, Entity::Device];

    pub fn word(self) -> &'static str {
        match self {
            Entity::Character => "character",
            Entity::Item => "item",
            Entity::Location => "location",
            Entity::Scene => "scene",
            Entity::Camera => "camera",
            Entity::Row => "row",
            Entity::Outlet => "outlet",
            Entity::Device => "device",
        }
    }

    pub fn parse(s: &str) -> Option<Entity> {
        Entity::ALL.into_iter().find(|e| e.word() == s)
    }
}

/// What a module in a slot does and where it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotDomain {
    /// sound, on the audio thread
    Audio,
    /// values and events, in the control half
    Control,
    /// the entity's transform, in the control half (the transform table, deck-module `motion`)
    Motion,
    /// pixels, on the render thread
    Picture,
    /// geometry, on the render thread
    Object,
    /// a device's bytes to sources and back
    Codec,
}

impl SlotDomain {
    pub fn word(self) -> &'static str {
        match self {
            SlotDomain::Audio => "audio",
            SlotDomain::Control => "control",
            SlotDomain::Motion => "motion",
            SlotDomain::Picture => "picture",
            SlotDomain::Object => "object",
            SlotDomain::Codec => "codec",
        }
    }

    /// the descriptor's `domain` a module of this slot runs in
    pub fn runs_in(self) -> Option<Domain> {
        match self {
            SlotDomain::Audio => Some(Domain::Audio),
            SlotDomain::Control | SlotDomain::Motion | SlotDomain::Codec => Some(Domain::Control),
            SlotDomain::Picture | SlotDomain::Object => None,
        }
    }
}

/// One slot of an entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub entity: Entity,
    pub domain: SlotDomain,
    /// what the module reads besides the times and the tempo every module has
    pub reads: &'static [&'static str],
    /// what it may write – only the entity's own
    pub writes: &'static [&'static str],
    /// the deck runs a module attached here; else the slot is declared and an attach is refused
    pub built: bool,
}

impl Slot {
    /// `camera.motion`
    pub fn name(&self) -> String {
        format!("{}.{}", self.entity.word(), self.domain.word())
    }

    /// it writes `what` (`position`, `focal` …)
    pub fn writes(&self, what: &str) -> bool {
        self.writes.contains(&what)
    }
}

const fn slot(entity: Entity, domain: SlotDomain, reads: &'static [&'static str], writes: &'static [&'static str], built: bool) -> Slot {
    Slot { entity, domain, reads, writes, built }
}

use Entity as E;
use SlotDomain as D;

/// The transform an entity's motion module may write.
pub const ENTITY_TRANSFORM: &[&str] = &["position", "rotation", "scale"];
/// A camera's.
pub const CAMERA_TRANSFORM: &[&str] = &["position", "target", "focal", "projection"];

/// Every slot, entity by entity (MODULE-EXTENSIONS §3).
pub const SLOTS: &[Slot] = &[
    slot(E::Character, D::Audio, &["rig"], &["rig"], false),
    slot(E::Character, D::Control, &["rig", "variables"], &["rig", "moods"], false),
    slot(E::Character, D::Motion, &["row", "spot"], ENTITY_TRANSFORM, false),
    slot(E::Character, D::Picture, &[], &["look"], false),
    slot(E::Item, D::Motion, &["row", "spot"], ENTITY_TRANSFORM, true),
    slot(E::Item, D::Picture, &[], &["node"], false),
    slot(E::Item, D::Object, &[], &["node"], false),
    slot(E::Location, D::Picture, &[], &["set"], false),
    slot(E::Location, D::Object, &[], &["set"], false),
    slot(E::Scene, D::Motion, &["row", "spots"], &["holders", "cameras"], false),
    slot(E::Scene, D::Picture, &[], &["pass"], false),
    slot(E::Scene, D::Control, &["variables"], &["holders", "cameras", "variables"], false),
    slot(E::Camera, D::Motion, &["row", "shots"], CAMERA_TRANSFORM, true),
    slot(E::Camera, D::Control, &["shots"], &["shot"], false),
    slot(E::Row, D::Control, &["row"], &["cells", "events"], false),
    slot(E::Outlet, D::Codec, &["sources"], &["sources", "outputs"], false),
    slot(E::Outlet, D::Picture, &[], &["pass"], false),
    slot(E::Device, D::Codec, &["sources"], &["sources", "outputs"], false),
];

/// the slots of `entity`, in the table's order
pub fn of(entity: Entity) -> impl Iterator<Item = &'static Slot> {
    SLOTS.iter().filter(move |s| s.entity == entity)
}

/// the slot named `camera.motion`
pub fn find(name: &str) -> Option<&'static Slot> {
    let (e, d) = name.split_once('.')?;
    let e = Entity::parse(e)?;
    SLOTS.iter().find(|s| s.entity == e && s.domain.word() == d)
}

/// A module's descriptor against the slot it names (none: a rig's module, nothing to check): the slot exists, the module
/// runs in its domain, and – the entity's transform being its only output – a motion module has no ports.
pub fn check_descriptor(d: &Descriptor) -> Result<Option<&'static Slot>, String> {
    let Some(name) = d.slot.as_deref() else { return Ok(None) };
    let me = &d.name;
    let s = find(name).ok_or_else(|| format!("module '{me}': no entity has the slot '{name}' (camera.motion, item.motion …)"))?;
    if s.domain.runs_in() != Some(d.domain) {
        return Err(format!("module '{me}': a {} module runs in the {} domain", s.name(), s.domain.runs_in().map_or("render", |x| domain_word(x))));
    }
    if s.domain == SlotDomain::Motion {
        if let Some(p) = d.ports.first() {
            return Err(format!("module '{me}': a motion module writes only its entity's transform – no ports ('{}')", p.name));
        }
    }
    Ok(Some(s))
}

fn domain_word(d: Domain) -> &'static str {
    match d {
        Domain::Audio => "audio",
        Domain::Control => "control",
        Domain::Video => "video",
    }
}

/// A module attached to a slot of an entity, on that entity's document.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attach {
    /// the slot's domain word on this entity (`motion`)
    pub slot: String,
    /// the module's id (`dolly`): the catalog's `module:<id>`
    pub module: String,
    /// its parameters by name; the rest at their defaults
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f32>,
}

impl Attach {
    pub fn new(slot: &str, module: &str) -> Attach {
        Attach { slot: slot.to_string(), module: module.to_string(), params: BTreeMap::new() }
    }
}

fn word(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// The attachments of one entity without their modules at hand (a document's check): each names a slot the entity has
/// and the deck runs, at most one module per slot, a module id, finite parameters.
pub fn problems(entity: Entity, list: &[Attach], who: &str, out: &mut Vec<String>) {
    let mut taken: Vec<&str> = Vec::new();
    for a in list {
        let at = format!("{who}: module '{}'", a.module);
        match of(entity).find(|s| s.domain.word() == a.slot) {
            None => out.push(format!("{at}: {} has no slot '{}' ({})", a_entity(entity), a.slot, of(entity).map(|s| s.domain.word()).collect::<Vec<_>>().join(", "))),
            Some(s) if !s.built => out.push(format!("{at}: the slot {} is not built yet", s.name())),
            Some(_) => {}
        }
        if taken.contains(&a.slot.as_str()) {
            out.push(format!("{at}: the slot '{}' holds one module", a.slot));
        }
        taken.push(&a.slot);
        if !word(&a.module) {
            out.push(format!("{at}: a module id is a lowercase word"));
        }
        if let Some((k, _)) = a.params.iter().find(|(_, v)| !v.is_finite()) {
            out.push(format!("{at}: parameter '{k}' is a number"));
        }
    }
}

fn a_entity(e: Entity) -> String {
    let w = e.word();
    let article = if matches!(w.as_bytes()[0], b'a' | b'e' | b'i' | b'o' | b'u') { "an" } else { "a" };
    format!("{article} {w}")
}

/// Whether module `d` fits slot `slot` (its domain word) of `entity`: the slot it names is that one, and its
/// parameters are named and inside their ranges.
pub fn fits(entity: Entity, slot: &str, d: &Descriptor, params: &BTreeMap<String, f32>) -> Result<(), String> {
    let me = &d.name;
    let want = format!("{}.{slot}", entity.word());
    match d.slot.as_deref() {
        None => return Err(format!("'{me}' is no slot module (it plays on a rig's track)")),
        Some(s) if s != want => return Err(format!("'{me}' is made for {s}, not {want}")),
        Some(_) => {}
    }
    check_descriptor(d)?;
    for (k, v) in params {
        let p = d.param(k).ok_or_else(|| format!("'{me}' has no parameter '{k}'"))?;
        if !(p.min..=p.max).contains(v) {
            return Err(format!("'{me}': '{k}' is {}–{}", p.min, p.max));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PortDesc;

    fn module(slot: Option<&str>, domain: Domain) -> Descriptor {
        let mut d: Descriptor = serde_json::from_str(r#"{"name":"dolly","abi":2,"params":[{"name":"depth","min":0,"max":4,"default":1}]}"#).unwrap();
        d.slot = slot.map(|s| s.to_string().into());
        d.domain = domain;
        d
    }

    #[test]
    fn every_entity_kind_declares_its_slots_and_motion_runs_for_cameras_and_items() {
        for e in Entity::ALL {
            assert!(of(e).count() > 0, "{e:?}");
            assert_eq!(Entity::parse(e.word()), Some(e));
        }
        let built: Vec<String> = SLOTS.iter().filter(|s| s.built).map(Slot::name).collect();
        assert_eq!(built, ["item.motion", "camera.motion"]);
        let cam = find("camera.motion").unwrap();
        assert!(cam.writes("focal") && cam.writes("target") && !cam.writes("scale"));
        assert!(find("item.motion").unwrap().writes("scale"));
        assert!(find("camera.picture").is_none() && find("nothing.motion").is_none() && find("camera").is_none());
    }

    #[test]
    fn a_descriptor_names_a_slot_its_domain_runs() {
        assert_eq!(check_descriptor(&module(None, Domain::Audio)), Ok(None));
        assert_eq!(check_descriptor(&module(Some("camera.motion"), Domain::Control)).unwrap().unwrap().name(), "camera.motion");
        assert!(check_descriptor(&module(Some("camera.motion"), Domain::Audio)).unwrap_err().contains("control domain"));
        assert!(check_descriptor(&module(Some("camera.wobble"), Domain::Control)).unwrap_err().contains("no entity has"));
        let mut d = module(Some("item.motion"), Domain::Control);
        d.ports = vec![serde_json::from_value::<PortDesc>(serde_json::json!({"name":"out","dir":"out","type":"control","path":"voice.gain"})).unwrap()].into();
        assert!(check_descriptor(&d).unwrap_err().contains("no ports"), "its writes stay inside the entity");
    }

    #[test]
    fn attaching_is_checked_against_the_entity_and_the_module() {
        let mut out = Vec::new();
        problems(Entity::Camera, &[Attach::new("motion", "dolly")], "camera", &mut out);
        assert!(out.is_empty(), "{out:?}");
        let mut p = Attach::new("motion", "Bad Id");
        p.params.insert("depth".into(), f32::NAN);
        problems(Entity::Camera, &[Attach::new("motion", "dolly"), p, Attach::new("picture", "x"), Attach::new("control", "pick")], "camera", &mut out);
        let all = out.join(" | ");
        for want in ["holds one module", "lowercase word", "is a number", "a camera has no slot 'picture'", "camera.control is not built yet"] {
            assert!(all.contains(want), "{want}: {all}");
        }

        let d = module(Some("camera.motion"), Domain::Control);
        assert_eq!(fits(Entity::Camera, "motion", &d, &BTreeMap::new()), Ok(()));
        assert!(fits(Entity::Item, "motion", &d, &BTreeMap::new()).unwrap_err().contains("made for camera.motion, not item.motion"));
        assert!(fits(Entity::Camera, "motion", &module(None, Domain::Audio), &BTreeMap::new()).unwrap_err().contains("no slot module"));
        let far = BTreeMap::from([("depth".to_string(), 9.0)]);
        assert!(fits(Entity::Camera, "motion", &d, &far).unwrap_err().contains("0–4"));
        let none = BTreeMap::from([("speed".to_string(), 1.0)]);
        assert!(fits(Entity::Camera, "motion", &d, &none).unwrap_err().contains("no parameter 'speed'"));
    }
}
