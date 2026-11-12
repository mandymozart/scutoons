//! The resolve step (#489, research/SCENES.md §9.2–§9.3, SCENE-PACKAGE P1 without embedded rigs): a scene document and a
//! use's fillers → the concrete scene a deck plays. Pure: the rigs come from the caller (the catalog's, a deck's cache;
//! loading on the decks is #246), nothing is fetched here.
//!
//! - **Fillers:** a use (`{ scene, with }`) names a filler per placeholder id – authored, or by context (the receiver's:
//!   a location's default scene fills `where` with the location); `previews` fills the rest with the document's previews
//!   (the editor, the browser). Each filler passes its placeholder's `fill` or the use is refused with the reason; a
//!   holder takes one, a spawn zone up to its `max`; the same filler twice on one stage is refused (`#2` is reserved).
//! - **Empty:** an empty placeholder plays nothing and has no address (no member in `cast`); an empty location is the
//!   neutral staging (scene.rs `STAGING_SCENE`'s room on the space node, no set). A filled location is handed on
//!   (`Resolved::location`): its set is the location's (2·3).
//! - **Spawn zones:** the members are arranged by the zone's rule around its spot (`arrange`: row, cluster).
//! - **Rigs:** every member with a rig flattens it (rig/flatten.rs) with the holder's preset choices and values; a rig the
//!   caller does not have leaves its member silent (said in `notes`). One rig keeps its slots as a rig played alone has
//!   them (`voice_voice.ratio`), so `scene:nave` in format 2 plays the scene it was in format 1; several are prefixed by
//!   their holder (`<holder>_…`, a zone's members `<holder>_<n>_…`). They share one `out` (the sums meet on its input),
//!   one `space` and one `tempo`; the first rig's links to the space move the being and its look is the scene's being –
//!   until the stage draws one look per character holder (2·1).
//! - **The story's casting** (`Casting::join`, owner 2026-11-12 §9.7 Q4): `joinScene` takes the holder that carries the
//!   character already, else the first placeholder or spawn zone with room, in the scene's order, whose `fill` admits it.
use crate::problem::sentences;
use crate::rig::address::{node_port, Address, Leaf};
use crate::rig::bundle::Bundle;
use crate::scene::{LinkDef, NodeDef, Scene, STAGING_SCENE};
use crate::scene_document::{Arrange, ArrangeKind, Enter, FillerRef, Fillers, HolderValue, LocationSlot, SceneDocument};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// A reference to a scene with its fillers (SCENES.md §9.2): what a group, a cell, a location or a character stores (2·2).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Use {
    /// `scene:<id>`
    pub scene: String,
    /// placeholder id → its filler (a spawn zone: several)
    #[serde(default)]
    pub with: BTreeMap<String, Fillers>,
}

/// One filler on the stage.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// the holder it fills
    pub holder: String,
    /// its reference as given (`character:catpope.pope`)
    pub filler: String,
    /// its address's place (`character:catpope`): what a word to it names below the scene
    pub place: String,
    /// the spot it stands at; none = the scene's spot
    pub spot: Option<String>,
    /// where it stands from the spot, in metres (x across, y up, z deep): a spawn zone's arrangement, else 0
    pub at: [f32; 3],
    pub enter: Enter,
    /// the prefix of its rig's compiled slots in the scene (`""` when its rig is the scene's only one); none = no rig
    pub slots: Option<String>,
}

/// The concrete scene and what a deck needs beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub scene: Scene,
    /// the location it plays in (`location:<id>`): its set comes from it; none = the space node's own set or the staging
    pub location: Option<String>,
    /// every filled holder's members, in the scene's order
    pub cast: Vec<Member>,
    /// what plays less than written (a rig not at hand) – said, not refused
    pub notes: Vec<String>,
}

/// where the rigs come from: subject (`character.catpope`) and the pinned version (none = current) → the bundle
pub type Rigs<'a> = &'a dyn Fn(&str, Option<&str>) -> Option<Bundle>;

/// no rigs at hand (the build service's check: it has the document alone)
pub fn no_rigs(_: &str, _: Option<&str>) -> Option<Bundle> {
    None
}

/// the core rigs a deck carries (rig/bundle.rs `CORE`), any version
pub fn core_rigs(subject: &str, _: Option<&str>) -> Option<Bundle> {
    Bundle::core(subject).and_then(Result::ok)
}

/// the nodes every rig's scene has once and the holders share (graph.rs: one sink of each kind; the clock)
const SHARED: [&str; 3] = ["out", "space", "tempo"];

/// A spawn zone's members around its spot: `n` offsets in metres (x across, z deep).
pub fn arrange(a: Arrange, n: usize) -> Vec<[f32; 3]> {
    match a.kind {
        ArrangeKind::Row => (0..n).map(|i| [(i as f32 - (n as f32 - 1.0) / 2.0) * a.spacing, 0.0, 0.0]).collect(),
        ArrangeKind::Cluster => {
            // the first in the middle, then rings of 6, 12, 18 … at one spacing more each
            let mut out = Vec::with_capacity(n);
            let mut ring = 0usize;
            while out.len() < n {
                if ring == 0 {
                    out.push([0.0, 0.0, 0.0]);
                } else {
                    let seats = 6 * ring;
                    let r = ring as f32 * a.spacing;
                    for s in 0..seats.min(n - out.len()) {
                        let angle = std::f32::consts::TAU * s as f32 / seats as f32 + ring as f32 * 0.5;
                        out.push([r * angle.cos(), 0.0, r * angle.sin()]);
                    }
                }
                ring += 1;
            }
            out
        }
    }
}

/// `node.port` (or a bare node) with its node renamed
fn renamed(end: &str, rename: &dyn Fn(&str) -> String) -> String {
    match end.split_once('.') {
        Some((n, p)) => format!("{}.{p}", rename(n)),
        None => rename(end),
    }
}

/// an exposed path (`voice_voice.ratio`, a counted group `seq.step1..16 <= seq.steps`) under `prefix`
#[cfg(feature = "being")]
fn exposed(prefix: &str, e: &str) -> String {
    match e.split_once(" <= ") {
        Some((a, b)) => format!("{prefix}{} <= {prefix}{}", a.trim(), b.trim()),
        None => format!("{prefix}{e}"),
    }
}

/// The concrete scene of `doc` with `with`'s fillers (and the previews for the rest when `previews`), the rigs from `rigs`.
/// Every reason it cannot be played is in the error.
pub fn resolve(doc: &SceneDocument, with: &BTreeMap<String, Fillers>, previews: bool, rigs: Rigs) -> Result<Resolved, Vec<String>> {
    let mut problems = doc.check();
    if !problems.is_empty() {
        return Err(problems);
    }
    let location_ph = doc.location_placeholder();
    for key in with.keys() {
        let holder = doc.holders.iter().find(|h| &h.id == key);
        if holder.is_some_and(|h| !h.is_placeholder()) {
            problems.push(format!("'{key}' has an authored filler: a use fills placeholders only"));
        } else if holder.is_none() && location_ph.is_none_or(|p| &p.id != key) {
            problems.push(format!("'{key}' is no placeholder of scene:{}", doc.id));
        }
    }

    let location = match &doc.location {
        None => None,
        Some(LocationSlot::Ref(r)) => Some(r.clone()),
        Some(LocationSlot::Placeholder(p)) => {
            let given = match with.get(&p.id) {
                Some(Fillers::One(r)) => Some(r.clone()),
                Some(Fillers::Many(_)) => {
                    problems.push(format!("location {}: one location, not a list", p.id));
                    None
                }
                None if previews => p.preview.clone(),
                None => None,
            };
            given.filter(|r| match p.fill.admits(r) {
                Ok(()) => true,
                Err(e) => {
                    problems.push(format!("location {}: {e}", p.id));
                    false
                }
            })
        }
    };

    // the cast: who stands where
    let mut cast = Vec::new();
    let mut places = HashSet::new();
    for h in &doc.holders {
        let given: Vec<String> = match (&h.filler, with.get(&h.id), &h.preview) {
            (Some(r), _, _) => vec![r.clone()],
            (None, Some(f), _) => f.list().into_iter().map(String::from).collect(),
            (None, None, Some(p)) if previews => p.list().into_iter().map(String::from).collect(),
            _ => Vec::new(),
        };
        if !h.has_room(given.len().saturating_sub(1)) {
            problems.push(match h.max {
                Some(m) => format!("holder {}: a spawn zone of at most {m}, {} given", h.id, given.len()),
                None => format!("holder {}: one filler, {} given", h.id, given.len()),
            });
            continue;
        }
        let at = h.arrange.map(|a| arrange(a, given.len())).unwrap_or_else(|| vec![[0.0; 3]; given.len()]);
        for (filler, at) in given.into_iter().zip(at) {
            if let Some(Err(e)) = h.fill.as_ref().map(|f| f.admits(&filler)) {
                problems.push(format!("holder {}: {e}", h.id));
                continue;
            }
            let f = match FillerRef::parse(&filler) {
                Ok(f) => f,
                Err(e) => {
                    problems.push(format!("holder {}: {e}", h.id));
                    continue;
                }
            };
            let place = format!("{}:{}", f.place.kind, f.place.id);
            if !places.insert(place.clone()) {
                problems.push(format!("holder {}: '{filler}' stands on this stage already (a second copy is reserved)", h.id));
                continue;
            }
            cast.push(Member { holder: h.id.clone(), filler, place, spot: h.spot.clone(), at, enter: h.enter, slots: None });
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    // the rigs, flattened with each holder's preset choices and values
    let mut notes = Vec::new();
    let mut flat: Vec<(usize, Scene)> = Vec::new();
    for (i, m) in cast.iter().enumerate() {
        let h = doc.holders.iter().find(|h| h.id == m.holder).expect("a member's holder");
        let f = FillerRef::parse(&m.filler).expect("parsed above");
        let pinned = h.rig.as_ref();
        let Some(subject) = pinned.and_then(|r| r.subject().map(String::from)).or_else(|| f.rig_subject()) else { continue };
        let Some(bundle) = rigs(&subject, pinned.and_then(|r| r.version.as_deref())) else {
            notes.push(format!("holder {}: no rig {subject} at hand: '{}' stands silent", h.id, m.filler));
            continue;
        };
        let mut chosen = BTreeMap::new();
        for (path, v) in &h.values {
            if let (Ok(Leaf::Member { track, .. }), HolderValue::Preset(p)) = (Address::leaf(path), v) {
                chosen.insert(track, p.clone());
            }
        }
        let mut scene = match bundle.rig.flatten_with(&bundle.library(), f.persona.as_deref(), None, &chosen) {
            Ok(s) => s,
            Err(e) => {
                problems.extend(sentences(&e).into_iter().map(|s| format!("holder {}: {s}", h.id)));
                continue;
            }
        };
        for (path, v) in &h.values {
            if let (Ok(Leaf::Port { track: Some(t), node, port }), HolderValue::Number(x)) = (Address::leaf(path), v) {
                match scene.nodes.iter_mut().find(|n| n.id == format!("{t}_{node}")) {
                    Some(n) => {
                        n.params.insert(port, toml::Value::Float(f64::from(*x)));
                    }
                    None => problems.push(format!("holder {}: value '{path}' names no node of the rig {subject}", h.id)),
                }
            }
        }
        flat.push((i, scene));
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    let mut scene = Scene::parse("").expect("the empty scene parses");
    scene.stage = doc.stage.clone();
    scene.outputs = doc.outputs.clone();
    let alone = flat.len() == 1;
    for (n, (i, rig)) in flat.into_iter().enumerate() {
        let m = &cast[i];
        let zone = doc.holders.iter().find(|h| h.id == m.holder).is_some_and(|h| h.take == crate::scene_document::Take::Many);
        let prefix = if alone {
            String::new()
        } else if zone {
            let nth = cast[..=i].iter().filter(|x| x.holder == m.holder).count();
            format!("{}_{nth}_", m.holder)
        } else {
            format!("{}_", m.holder)
        };
        let rename = |id: &str| if SHARED.contains(&id) { id.to_string() } else { format!("{prefix}{id}") };
        for node in rig.nodes {
            if SHARED.contains(&node.id.as_str()) {
                if !scene.nodes.iter().any(|x| x.id == node.id) {
                    scene.nodes.push(node);
                }
            } else {
                scene.nodes.push(NodeDef { id: rename(&node.id), ..node });
            }
        }
        for l in rig.links {
            if n > 0 && node_port(&l.to).0 == "space" {
                continue;
            }
            scene.links.push(LinkDef { from: renamed(&l.from, &rename), to: renamed(&l.to, &rename), ..l });
        }
        if n == 0 {
            #[cfg(feature = "being")]
            {
                let mut being = rig.being;
                being.expose = being.expose.iter().map(|e| exposed(&prefix, e)).collect();
                being.labels = being.labels.into_iter().map(|(k, v)| (exposed(&prefix, &k), v)).collect();
                scene.being = being;
            }
            #[cfg(not(feature = "being"))]
            {
                scene.being = rig.being;
            }
        } else if n == 1 {
            notes.push(format!("only holder {}'s rig moves the being until the stage draws one look per character (2·1)", cast[0].holder));
        }
        cast[i].slots = Some(prefix);
    }

    // the scene's own nodes: a shared one (its space) takes the document's params over the rig's bare one
    for node in &doc.nodes {
        match scene.nodes.iter_mut().find(|x| x.id == node.id) {
            Some(x) if x.kind == node.kind => x.params.extend(node.params.iter().map(|(k, v)| (k.clone(), v.clone()))),
            Some(x) => problems.push(format!("node '{}': the scene's is a {}, a rig's a {}", node.id, node.kind, x.kind)),
            None => scene.nodes.push(node.clone()),
        }
    }
    scene.links.extend(doc.links.iter().cloned());
    // a location's set comes from the location (2·3); until a location is filled, and while its set is not here, the
    // space node stands in the neutral staging's room – the document's own params win
    if doc.location.is_some() {
        let staging = Scene::parse(STAGING_SCENE).expect("the staging is embedded and checked by the tests");
        let room = staging.nodes.into_iter().find(|n| n.kind == "space").expect("the staging has a space node");
        if let Some(space) = scene.nodes.iter_mut().find(|n| n.kind == "space") {
            for (k, v) in room.params {
                space.params.entry(k).or_insert(v);
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    crate::graph::Graph::build(&scene).map_err(|e| sentences(&e))?;
    Ok(Resolved { scene, location, cast, notes })
}

impl Use {
    /// `resolve` for this use: its scene must be the document's
    pub fn resolve(&self, doc: &SceneDocument, previews: bool, rigs: Rigs) -> Result<Resolved, Vec<String>> {
        if self.scene != format!("scene:{}", doc.id) {
            return Err(vec![format!("the use names {}, the document is scene:{}", self.scene, doc.id)]);
        }
        resolve(doc, &self.with, previews, rigs)
    }
}

/// The story's fillers of one scene while a reader reads (SCENES.md §9.4): who joined which placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct Casting<'a> {
    doc: &'a SceneDocument,
    with: BTreeMap<String, Vec<String>>,
}

impl<'a> Casting<'a> {
    /// the fillers so far (a use's, by context)
    pub fn new(doc: &'a SceneDocument, with: &BTreeMap<String, Fillers>) -> Casting<'a> {
        let with = with.iter().map(|(k, v)| (k.clone(), v.list().into_iter().map(String::from).collect())).collect();
        Casting { doc, with }
    }

    /// `joinScene`: the holder that carries the filler already, else the first placeholder or spawn zone with room, in the
    /// scene's order, whose `fill` admits it – its id; the reason when none does
    pub fn join(&mut self, filler: &str) -> Result<String, String> {
        let place = FillerRef::parse(filler)?.place;
        let same = |r: &str| FillerRef::parse(r).is_ok_and(|f| f.place == place);
        for h in &self.doc.holders {
            if h.filler.as_deref().is_some_and(same) || self.with.get(&h.id).is_some_and(|l| l.iter().any(|r| same(r))) {
                return Ok(h.id.clone());
            }
        }
        for h in self.doc.holders.iter().filter(|h| h.is_placeholder()) {
            let taken = self.with.get(&h.id).map_or(0, Vec::len);
            if h.has_room(taken) && h.fill.as_ref().is_some_and(|f| f.admits(filler).is_ok()) {
                self.with.entry(h.id.clone()).or_default().push(filler.to_string());
                return Ok(h.id.clone());
            }
        }
        Err(format!("no placeholder of scene:{} with room admits '{filler}'", self.doc.id))
    }

    /// the fillers as a use's `with`
    pub fn with(&self) -> BTreeMap<String, Fillers> {
        let zone = |id: &str| self.doc.holders.iter().any(|h| h.id == id && h.take == crate::scene_document::Take::Many);
        self.with
            .iter()
            .filter(|(_, l)| !l.is_empty())
            .map(|(k, l)| (k.clone(), if zone(k) || l.len() > 1 { Fillers::Many(l.clone()) } else { Fillers::One(l[0].clone()) }))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_document::{Holder, RigPin};

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes");

    fn doc(json: &str) -> SceneDocument {
        SceneDocument::parse_json(json).unwrap()
    }

    fn with(pairs: &[(&str, Fillers)]) -> BTreeMap<String, Fillers> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn one(r: &str) -> Fillers {
        Fillers::One(r.into())
    }

    fn space(s: &Scene) -> &toml::Table {
        &s.nodes.iter().find(|n| n.kind == "space").unwrap().params
    }

    /// a fixture as the core writes it: read, parsed, checked, and its text the canonical one (`UPDATE_FIXTURES=1` writes it)
    fn fixture(name: &str, built: Option<&SceneDocument>) -> SceneDocument {
        let path = format!("{FIXTURES}/{name}.json");
        let stored = std::fs::read_to_string(&path).unwrap_or_default().replace('\r', "");
        let doc = match built {
            Some(d) => d.clone(),
            None => SceneDocument::parse_json(&stored).unwrap_or_else(|e| panic!("{name}: {e:#}")),
        };
        let canonical = doc.to_json().unwrap() + "\n";
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(&path, &canonical).unwrap();
        } else {
            assert_eq!(stored, canonical, "{name} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space scene_resolve");
        }
        assert_eq!(doc.check(), Vec::<String>::new(), "{name}");
        doc
    }

    /// The first real scene in format 2 (SCENES.md §9.3): the cathedral set (no location yet) and one holder, Cat Pope with
    /// her rig – the hand-flattened nodes of format 1 left. server/tests/fixtures/scenes/nave.json is this.
    fn nave() -> SceneDocument {
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let mut room = toml::Table::new();
        for key in ["set", "quality", "at", "yaw", "pitch", "distance"] {
            room.insert(key.into(), space(&cathedral)[key].clone());
        }
        let mut doc = SceneDocument::from_scene("nave", vec!["catpope".into()], &Scene::parse("").unwrap());
        doc.stage.name = "Nave".into();
        doc.nodes = vec![NodeDef { id: "space".into(), kind: "space".into(), params: room }];
        doc.holders = vec![Holder {
            id: "catpope".into(),
            filler: Some("character:catpope".into()),
            rig: Some(RigPin { rig: "rig:character.catpope".into(), version: None }),
            ..Holder::default()
        }];
        doc
    }

    /// #489 acceptance (SCENES.md §9.6 2·0): `scene:nave` in format 2 resolves to the scene it played as format 1 – the
    /// core rig `character.catpope` flattened, staged as the cathedral sandbox stages it
    #[test]
    fn nave_in_format_2_plays_the_scene_of_format_1() {
        let stored = fixture("nave", Some(&nave()));
        let resolved = resolve(&stored, &BTreeMap::new(), false, &core_rigs).unwrap();
        let bundle = Bundle::core("character.catpope").unwrap().unwrap();
        let mut format_1 = bundle.rig.flatten(&bundle.library(), None, None).unwrap();
        format_1.stage.name = "Nave".into();
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let s = format_1.nodes.iter_mut().find(|n| n.kind == "space").unwrap();
        for key in ["set", "quality", "at", "yaw", "pitch", "distance"] {
            s.params.insert(key.into(), space(&cathedral)[key].clone());
        }
        assert_eq!(resolved.scene, format_1, "the same scene");
        assert_eq!(resolved.location, None);
        assert_eq!(resolved.cast.len(), 1);
        let m = &resolved.cast[0];
        assert_eq!((m.holder.as_str(), m.place.as_str(), m.slots.as_deref(), m.spot.as_ref()), ("catpope", "character:catpope", Some(""), None));
        assert!(resolved.notes.is_empty());
        // without the rig at hand it still resolves: the space alone, Cat Pope silent and said
        let bare = resolve(&stored, &BTreeMap::new(), false, &no_rigs).unwrap();
        assert_eq!(bare.scene.nodes.len(), 1);
        assert!(bare.notes[0].contains("no rig character.catpope"), "{:?}", bare.notes);
    }

    /// a holder's values and preset choices reach its rig's nodes
    #[test]
    fn a_holders_values_and_presets_reach_its_rig() {
        let mut d = nave();
        d.holders[0].values = [("voice:voice.index".to_string(), HolderValue::Number(4.5)), ("voice:@preset".into(), HolderValue::Preset("cathedral_voice.bright".into()))].into();
        let r = resolve(&d, &BTreeMap::new(), false, &core_rigs).unwrap();
        let voice = r.scene.nodes.iter().find(|n| n.id == "voice_voice").unwrap();
        assert_eq!(voice.params["index"].as_float(), Some(4.5));
        d.holders[0].values = [("voice:nothing.index".to_string(), HolderValue::Number(1.0))].into();
        assert!(resolve(&d, &BTreeMap::new(), false, &core_rigs).unwrap_err()[0].contains("names no node"));
    }

    /// the four templates (SCENES.md §9.5) are well formed: they resolve empty, with their previews and with fillers
    #[test]
    fn the_templates_resolve() {
        for name in ["place", "solo", "stage", "avatar-home"] {
            let t = fixture(&format!("templates/{name}"), None);
            assert_eq!(t.id, name);
            assert!(t.toons.is_empty(), "{name}: a template names no toon");
            assert!(t.location_placeholder().is_some(), "{name}: its location is a placeholder");
            let empty = resolve(&t, &BTreeMap::new(), false, &no_rigs).unwrap();
            assert!(empty.cast.is_empty() && empty.location.is_none(), "{name}");
            resolve(&t, &BTreeMap::new(), true, &no_rigs).unwrap();
        }
        let stage = fixture("templates/stage", None);
        let r = resolve(&stage, &with(&[("where", one("location:dom")), ("left", one("character:catpope")), ("right", one("character:toptoy"))]), false, &core_rigs).unwrap();
        assert_eq!(r.location.as_deref(), Some("location:dom"));
        let at: Vec<(&str, Option<&str>)> = r.cast.iter().map(|m| (m.holder.as_str(), m.spot.as_deref())).collect();
        assert_eq!(at, [("left", Some("spot_left")), ("right", Some("spot_right"))], "the empty ones have no member");
        assert_eq!(r.cast[0].enter, Enter::Join);
        assert!(r.notes.iter().any(|n| n.contains("character.toptoy")), "toptoy has no core rig: silent, said");
        let home = fixture("templates/avatar-home", None);
        assert_eq!(home.location_placeholder().unwrap().id, "home");
    }

    /// a filler that does not pass the filter is refused with the reason; a use names placeholders only
    #[test]
    fn a_filler_is_checked_against_its_fill() {
        let stage = fixture("templates/stage", None);
        let refused = |w: &[(&str, Fillers)]| resolve(&stage, &with(w), false, &no_rigs).unwrap_err().join(" | ");
        assert!(refused(&[("left", one("item:incense"))]).contains("holder left: 'item:incense' is an item, the placeholder takes character"));
        assert!(refused(&[("where", one("character:catpope"))]).contains("location where: 'character:catpope' is a character"));
        assert!(refused(&[("nowhere", one("character:catpope"))]).contains("'nowhere' is no placeholder of scene:stage"));
        assert!(refused(&[("left", one("character:catpope")), ("right", one("character:catpope"))]).contains("holder right: 'character:catpope' stands on this stage already"));
        assert!(refused(&[("left", Fillers::Many(vec!["character:a".into(), "character:b".into()]))]).contains("one filler, 2 given"));
        assert!(resolve(&nave(), &with(&[("catpope", one("character:toptoy"))]), false, &no_rigs).unwrap_err()[0].contains("authored filler"));
        let u = Use { scene: "scene:other".into(), with: BTreeMap::new() };
        assert!(u.resolve(&stage, false, &no_rigs).unwrap_err()[0].contains("the use names scene:other"));
    }

    const CHOIR: &str = r#"{"format":2,"id":"choir","location":"location:cathedral",
      "holder":[{"id":"crows","spot":"spot_beam","fill":{"kind":["item"]},"take":"many","arrange":{"kind":"row","spacing":0.5},"max":3},
                {"id":"table","fill":{"kind":["item"]}},
                {"id":"pope","fill":{"kind":["character"]}}],
      "node":[{"id":"space","kind":"space"}]}"#;

    /// a spawn zone takes several of what its fill allows, arranged by its rule, and stops at `max`
    #[test]
    fn a_spawn_zone_takes_several_up_to_its_max() {
        let choir = doc(CHOIR);
        let crows = |n: usize| Fillers::Many((1..=n).map(|i| format!("item:crow{i}")).collect());
        let r = resolve(&choir, &with(&[("crows", crows(3))]), false, &no_rigs).unwrap();
        let at: Vec<[f32; 3]> = r.cast.iter().map(|m| m.at).collect();
        assert_eq!(at, [[-0.5, 0.0, 0.0], [0.0, 0.0, 0.0], [0.5, 0.0, 0.0]]);
        assert!(r.cast.iter().all(|m| m.spot.as_deref() == Some("spot_beam")));
        assert_eq!(r.location.as_deref(), Some("location:cathedral"));
        assert!(resolve(&choir, &with(&[("crows", crows(4))]), false, &no_rigs).unwrap_err()[0].contains("at most 3, 4 given"));
        // the story's casting: the zone takes until it is full, the next item goes to the next placeholder that admits it
        let mut casting = Casting::new(&choir, &BTreeMap::new());
        let joined: Vec<String> = (1..=5).map(|i| casting.join(&format!("item:crow{i}")).unwrap_or_else(|e| e)).collect();
        assert_eq!(joined[..4], ["crows", "crows", "crows", "table"]);
        assert!(joined[4].contains("no placeholder of scene:choir with room admits 'item:crow5'"));
        assert_eq!(casting.join("item:crow2"), Ok("crows".into()), "a filler on stage stays where it is");
        assert_eq!(casting.join("character:catpope.pope"), Err("no placeholder of scene:choir with room admits 'character:catpope.pope'".into()), "a persona is no character");
        assert_eq!(casting.join("character:catpope"), Ok("pope".into()));
        let w = casting.with();
        assert_eq!(w["crows"], crows(3));
        assert_eq!(w["table"], one("item:crow4"));
        assert_eq!(resolve(&choir, &w, false, &no_rigs).unwrap().cast.len(), 5);
    }

    #[test]
    fn the_arrangements() {
        let row = Arrange { kind: ArrangeKind::Row, spacing: 1.0 };
        assert_eq!(arrange(row, 1), [[0.0, 0.0, 0.0]]);
        assert_eq!(arrange(row, 2), [[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0]]);
        let cluster = arrange(Arrange { kind: ArrangeKind::Cluster, spacing: 1.0 }, 9);
        assert_eq!(cluster.len(), 9);
        assert_eq!(cluster[0], [0.0, 0.0, 0.0]);
        let radius = |p: [f32; 3]| (p[0] * p[0] + p[2] * p[2]).sqrt();
        assert!(cluster[1..7].iter().all(|p| (radius(*p) - 1.0).abs() < 1e-5), "the first ring: six at one spacing");
        assert!(cluster[7..].iter().all(|p| (radius(*p) - 2.0).abs() < 1e-5), "then the next ring");
    }

    /// an empty location is the neutral staging (no set, the staging's room); a filled one is handed on
    #[test]
    fn an_empty_location_is_the_neutral_staging() {
        let place = fixture("templates/place", None);
        let empty = resolve(&place, &BTreeMap::new(), false, &no_rigs).unwrap();
        assert_eq!(empty.location, None);
        assert_eq!(space(&empty.scene)["join"].as_str(), Some("deck"), "the staging's room");
        assert!(space(&empty.scene).get("set").is_none());
        let at = resolve(&place, &with(&[("where", one("location:dom"))]), false, &no_rigs).unwrap();
        assert_eq!(at.location.as_deref(), Some("location:dom"));
    }

    /// two rigs on one stage: prefixed by their holders, summed into the one `out`, the first moves the being
    #[test]
    fn two_rigs_share_the_out_and_the_space() {
        let d = doc(r#"{"format":2,"id":"duo","holder":[{"id":"pope","ref":"character:catpope"},{"id":"burner","fill":{"kind":["item"]}}],
          "node":[{"id":"space","kind":"space","set":"cathedral"}]}"#);
        let r = resolve(&d, &with(&[("burner", one("item:incense_burner"))]), false, &core_rigs).unwrap();
        let ids: Vec<&str> = r.scene.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"pope_voice_voice") && ids.iter().any(|i| i.starts_with("burner_")), "{ids:?}");
        assert_eq!(ids.iter().filter(|i| **i == "out").count(), 1);
        assert_eq!(ids.iter().filter(|i| **i == "space").count(), 1);
        assert_eq!(r.scene.links.iter().filter(|l| node_port(&l.to).0 == "space").count(), 1, "one rig moves the being");
        assert_eq!(r.cast.iter().map(|m| m.slots.clone()).collect::<Vec<_>>(), [Some("pope_".to_string()), Some("burner_".to_string())]);
        assert_eq!(space(&r.scene)["set"].as_str(), Some("cathedral"));
        assert!(r.notes[0].contains("only holder pope's rig moves the being"));
        #[cfg(feature = "being")]
        assert!(r.scene.being.expose.iter().all(|e| e.starts_with("pope_")) && r.scene.being.character == "catpope");
    }
}
