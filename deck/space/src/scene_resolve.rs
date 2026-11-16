//! The resolve step (#489, research/SCENES.md §9.2–§9.3, SCENE-PACKAGE P1 without embedded rigs): a scene document and a
//! use's fillers → the concrete scene a deck plays. Pure: the rigs come from the caller (the catalog's, a deck's cache;
//! loading on the decks is #246), nothing is fetched here.
//!
//! - **Fillers:** a use (`{ scene, with }`) names a filler per placeholder id – authored, or by context (the receiver's:
//!   a location's default scene fills `where` with the location); `previews` fills the rest with the document's previews
//!   (the editor, the browser). Each filler passes its placeholder's `fill` or the use is refused with the reason; a
//!   holder takes one, a spawn zone up to its `max`; the same filler twice on one stage is refused (`#2` is reserved).
//! - **Empty:** an empty placeholder plays nothing and has no address (no member in `cast`); an empty location is the
//!   neutral staging (scene.rs `STAGING_SCENE`'s room on the space node, no set).
//! - **The set (#512, 2·3):** a filled location is handed on (`Resolved::location`) and its set is the location's – the
//!   caller's `Sets` (a place id → the set it names, scene-content's `locations`) puts it on the space node in the
//!   staging's room (`scene::staging`); a location that names none plays the neutral staging, said in `notes`. A
//!   `space.set` beside a location stays refused (scene_document.rs `check`).
//! - **Spawn zones:** the members are arranged by the zone's rule around its spot (`arrange`: row, cluster).
//! - **Rigs:** every member with a rig flattens it (rig/flatten.rs) with the holder's preset choices and values; a rig the
//!   caller does not have leaves its member silent (said in `notes`). One rig keeps its slots as a rig played alone has
//!   them (`voice_voice.ratio`), so `scene:nave` in format 2 plays the scene it was in format 1; several are prefixed by
//!   their holder (`<holder>_…`, a zone's members `<holder>_<n>_…`). They share one `out` (the sums meet on its input),
//!   one `space` and one `tempo`; a code rig's policy node is re-prefixed with them (policy.rs `reprefix`).
//! - **Looks (#492, 2·1):** on every stage but a single rigged member at the scene's own spot, each member – a character or,
//!   since #495, an item – gets a `look` node (scene.rs `look_node`) at its holder's spot; the space draws it as cast.rs says:
//!   a 3D being or a 2D card. Only its own
//!   rig's links to the being (`being:<input>` → `<prefix>look.<input>`) move it; a rig with no such mapping, or none at
//!   all, moves nothing (owner 2026-11-12 [rigs]: "if a rig has no mapping to visual properties nothing moves"). A rig's
//!   link into the room (the space's light, its post stack) is the scene's on such a stage and is left out, said in `notes`.
//! - **The story's casting** (`Casting::join`, owner 2026-11-12 §9.7 Q4): `joinScene` takes the holder that carries the
//!   character already, else the first placeholder or spawn zone with room, in the scene's order, whose `fill` admits it;
//!   `Casting::leave` frees it again. A scene that plays takes both as words (scene_play.rs, #516).
use crate::problem::sentences;
use crate::rig::address::{node_port, Address, Leaf};
use crate::rig::bundle::Bundle;
use crate::scene::{LinkDef, NodeDef, Scene};
use crate::scene_document::{Arrange, ArrangeKind, Enter, FillerRef, Fillers, HolderValue, LocationSlot, SceneDocument};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// A use of a scene (SCENES.md §9.2, #504): a scene plus the fillers for its placeholders – what a group, a cell, a location or
/// a character stores instead of a new scene document when only the fillers change (owner 2026-11-12 [scenes], §9.7 Q3).
/// Whether it plays is the core's: `Use::problems` (how the server would get this verdict is the owner's review, #506).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "SceneUse"))]
#[serde(deny_unknown_fields)]
pub struct Use {
    /// the scene it plays: `scene:<id>`
    pub scene: String,
    /// its fillers by placeholder id (the location placeholder's or a holder's `id`): a reference, or several for a spawn
    /// zone; a placeholder left out stays empty, or is filled by context (the receiver's) or the story (`joinScene`)
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

/// where a location's set comes from (#512): a place id (`cathedral`) → the set it names – stored configuration a deck
/// reads from scene-content (`push::Content::set_of`); none = it names none, or the caller does not have it
pub type Sets<'a> = &'a dyn Fn(&str) -> Option<String>;

/// no location's set at hand (the build service's check; a use's validity does not depend on its set)
pub fn no_sets(_: &str) -> Option<String> {
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

/// The concrete scene of `doc` with `with`'s fillers (and the previews for the rest when `previews`), the rigs from `rigs`,
/// a location's set from `sets`. Every reason it cannot be played is in the error.
pub fn resolve(doc: &SceneDocument, with: &BTreeMap<String, Fillers>, previews: bool, rigs: Rigs, sets: Sets) -> Result<Resolved, Vec<String>> {
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
        let moved = h.at.unwrap_or_default();
        let at = h.arrange.map(|a| arrange(a, given.len())).unwrap_or_else(|| vec![[0.0; 3]; given.len()]);
        let at = at.into_iter().map(|a| std::array::from_fn::<f32, 3, _>(|k| a[k] + moved[k]));
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
    let (mut rig_bg, mut module_bg) = (None, None);
    for (i, m) in cast.iter().enumerate() {
        let h = doc.holders.iter().find(|h| h.id == m.holder).expect("a member's holder");
        let f = FillerRef::parse(&m.filler).expect("parsed above");
        let pinned = h.rig.as_ref();
        let Some(subject) = pinned.and_then(|r| r.subject().map(String::from)).or_else(|| f.rig_subject()) else { continue };
        let Some(bundle) = rigs(&subject, pinned.and_then(|r| r.version.as_deref())) else {
            notes.push(format!("holder {}: no rig {subject} at hand: '{}' stands silent", h.id, m.filler));
            continue;
        };
        // the cast's rigs in holder order: the first one that sets a background, and its modules' first
        rig_bg = rig_bg.or_else(|| bundle.rig.background.clone());
        module_bg = module_bg.or_else(|| bundle.modules.iter().find_map(|d| d.background.clone()));
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
    scene.stage.background = deck_catalog::background::resolve(None, doc.stage.background.as_ref(), rig_bg.as_ref(), module_bg.as_ref()).cloned();
    scene.outputs = doc.outputs.clone();
    // one member with its rig at the scene's own spot plays as its rig played alone: its slots as they are, its being the
    // space's (so `scene:nave` plays the scene of format 1). Every other stage gives each character member a look of its
    // own (#492): a `look` node at its holder's spot that only its own rig's links to the being move – no rig, or a rig
    // with no mapping to the being, moves nothing (owner 2026-11-12 [rigs])
    let alone = cast.len() == 1 && flat.len() == 1 && cast[0].spot.is_none() && cast[0].at == [0.0; 3];
    let prefixes: Vec<String> = (0..cast.len())
        .map(|i| {
            let m = &cast[i];
            let zone = doc.holders.iter().find(|h| h.id == m.holder).is_some_and(|h| h.take == crate::scene_document::Take::Many);
            if alone {
                String::new()
            } else if zone {
                let nth = cast[..=i].iter().filter(|x| x.holder == m.holder).count();
                format!("{}_{nth}_", m.holder)
            } else {
                format!("{}_", m.holder)
            }
        })
        .collect();
    // #495: every member a look – a character's or an item's – which cast.rs gives its form (3D look or 2D card)
    let who = |m: &Member| crate::cast::Who::parse(&m.place);
    let mut looks: BTreeMap<usize, crate::scene::BeingLook> = BTreeMap::new();
    for (i, rig) in flat {
        let m = &cast[i];
        let prefix = prefixes[i].clone();
        let rename = |id: &str| if SHARED.contains(&id) { id.to_string() } else { format!("{prefix}{id}") };
        for node in rig.nodes {
            if SHARED.contains(&node.id.as_str()) {
                if !scene.nodes.iter().any(|x| x.id == node.id) {
                    scene.nodes.push(node);
                }
            } else {
                let mut node = NodeDef { id: rename(&node.id), ..node };
                // a code rig's policy drives its own tracks only (#492, left open by #489)
                if !alone && node.id == format!("{prefix}{}", crate::rig::policy::NODE) {
                    crate::rig::policy::reprefix(&mut node, &prefix);
                }
                scene.nodes.push(node);
            }
        }
        for l in rig.links {
            let (to, port) = node_port(&l.to);
            if !alone && to == "space" {
                // the rig's mappings to its being land on its own look (a character's or an item's, #495); the room
                // (light, post) is the scene's
                match who(m) {
                    Some(_) if crate::graph::Kind::Look.input(port).is_some() => {
                        scene.links.push(LinkDef { from: renamed(&l.from, &rename), to: format!("{}.{port}", crate::scene::look_id(&prefix)), ..l });
                    }
                    Some(_) => notes.push(format!("holder {}: its rig's link into the room's '{port}' is left out – on a stage of several the room is the scene's", m.holder)),
                    None => notes.push(format!("holder {}: '{}' has no look on the stage – its rig's link to being:{port} moves nothing", m.holder, m.filler)),
                }
                continue;
            }
            scene.links.push(LinkDef { from: renamed(&l.from, &rename), to: renamed(&l.to, &rename), ..l });
        }
        #[cfg(feature = "being")]
        let being = {
            let mut being = rig.being;
            being.expose = being.expose.iter().map(|e| exposed(&prefix, e)).collect();
            being.labels = being.labels.into_iter().map(|(k, v)| (exposed(&prefix, &k), v)).collect();
            being
        };
        #[cfg(not(feature = "being"))]
        let being = rig.being;
        if alone {
            scene.being = being;
        } else {
            looks.insert(i, being);
        }
        cast[i].slots = Some(prefix);
    }
    if !alone {
        for (i, m) in cast.iter().enumerate() {
            let Some(w) = who(m) else { continue };
            scene.nodes.push(crate::scene::look_node(&prefixes[i], &m.holder, &w, m.spot.as_deref(), m.at, looks.remove(&i).unwrap_or_default()));
        }
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
    // a location's set comes from the location (#512: one set per location, owner 2026-11-10 [live]) – the staging's room
    // with that set on the space node, as a rig played at a home (`scene::staging`); an empty location, or one that names
    // no set, stands in the neutral staging (the latter said in `notes`). The document's own params win (its `set` beside a
    // location is refused by `check`).
    if doc.location.is_some() {
        let set = location.as_deref().and_then(|r| {
            let id = r.strip_prefix("location:").unwrap_or(r);
            let set = sets(id).filter(|s| !s.is_empty());
            if set.is_none() {
                notes.push(format!("{r} names no set here: the neutral staging plays"));
            }
            set
        });
        let staging = crate::scene::staging(set.as_deref());
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
    pub fn resolve(&self, doc: &SceneDocument, previews: bool, rigs: Rigs, sets: Sets) -> Result<Resolved, Vec<String>> {
        if self.scene != format!("scene:{}", doc.id) {
            return Err(vec![format!("the use names {}, the document is scene:{}", self.scene, doc.id)]);
        }
        resolve(doc, &self.with, previews, rigs, sets)
    }

    /// Why this use cannot be stored in a toon's document (none = it can): its scene is `doc`, and – with the toon's book
    /// (the references of its address book, #477) – the scene and every filler are entries of that book; then each filler
    /// passes its placeholder's `fill` and the scene resolves with them (`resolve`, no previews, no rigs: the rigs are a
    /// deck's, a missing one leaves its member silent). Every reason is listed.
    pub fn problems(&self, doc: &SceneDocument, book: Option<&[String]>) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(book) = book {
            let listed = |r: &str| book.iter().any(|b| b == r);
            if !listed(&self.scene) {
                out.push(format!("{} is not in the toon's book (a scene of the toon, or a template)", self.scene));
            }
            for (id, fillers) in &self.with {
                for f in fillers.list().into_iter().filter(|f| !listed(f)) {
                    out.push(format!("{id}: '{f}' is not in the toon's book"));
                }
            }
        }
        if let Err(e) = self.resolve(doc, false, &no_rigs, &no_sets) {
            out.extend(e);
        }
        out
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

    /// `leave` (#516): the placeholder that carries the filler's place (any persona or look of it) gives it up – its id;
    /// none when no placeholder carries it (an authored filler is written into the scene and stays)
    pub fn leave(&mut self, filler: &str) -> Option<String> {
        let place = FillerRef::parse(filler).ok()?.place;
        for (id, list) in self.with.iter_mut() {
            if let Some(i) = list.iter().position(|r| FillerRef::parse(r).is_ok_and(|f| f.place == place)) {
                list.remove(i);
                return Some(id.clone());
            }
        }
        None
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

    #[test]
    fn the_scenes_background_over_its_rigs_and_their_modules() {
        use crate::Background;
        let rigs = |rig: Option<&str>, module: Option<&str>| {
            let (rig, module) = (rig.map(Background::colour), module.map(Background::colour));
            move |subject: &str, v: Option<&str>| {
                let mut b = core_rigs(subject, v)?;
                b.rig.background = rig.clone();
                b.modules.push(deck_catalog::Descriptor { background: module.clone(), ..deck_catalog::builtin("osc").unwrap().clone() });
                Some(b)
            }
        };
        let played = |doc: &SceneDocument, r: &dyn Fn(&str, Option<&str>) -> Option<Bundle>| resolve(doc, &BTreeMap::new(), false, r, &no_sets).unwrap().scene.stage.background;
        let mut doc = nave();
        assert_eq!(played(&doc, &rigs(None, None)), None, "none set: black");
        assert_eq!(played(&doc, &rigs(None, Some("#00f"))), Some(Background::colour("#00f")), "a module's");
        assert_eq!(played(&doc, &rigs(Some("#0f0"), Some("#00f"))), Some(Background::colour("#0f0")), "the rig over its module");
        doc.stage.background = Some(Background::image("backgrounds/nave.png"));
        assert_eq!(played(&doc, &rigs(Some("#0f0"), Some("#00f"))), Some(Background::image("backgrounds/nave.png")), "the scene over its rigs");
        doc.stage.background = Some(Background::colour("blue"));
        assert_eq!(doc.check(), ["stage background: 'blue' is no colour: #rgb or #rrggbb"]);
    }

    /// #489 acceptance (SCENES.md §9.6 2·0): `scene:nave` in format 2 resolves to the scene it played as format 1 – the
    /// core rig `character.catpope` flattened, staged as the cathedral sandbox stages it
    #[test]
    fn nave_in_format_2_plays_the_scene_of_format_1() {
        let stored = fixture("nave", Some(&nave()));
        let resolved = resolve(&stored, &BTreeMap::new(), false, &core_rigs, &no_sets).unwrap();
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
        // without the rig at hand it still resolves: the space and her look (#492), Cat Pope silent and said
        let bare = resolve(&stored, &BTreeMap::new(), false, &no_rigs, &no_sets).unwrap();
        let kinds: Vec<&str> = bare.scene.nodes.iter().map(|n| n.kind.as_str()).collect();
        assert_eq!(kinds, ["look", "space"], "her look stands there, nothing moves it");
        assert!(bare.notes[0].contains("no rig character.catpope"), "{:?}", bare.notes);
    }

    /// a holder's values and preset choices reach its rig's nodes
    #[test]
    fn a_holders_values_and_presets_reach_its_rig() {
        let mut d = nave();
        d.holders[0].values = [("voice:voice.index".to_string(), HolderValue::Number(4.5)), ("voice:@preset".into(), HolderValue::Preset("cathedral_voice.bright".into()))].into();
        let r = resolve(&d, &BTreeMap::new(), false, &core_rigs, &no_sets).unwrap();
        let voice = r.scene.nodes.iter().find(|n| n.id == "voice_voice").unwrap();
        assert_eq!(voice.params["index"].as_float(), Some(4.5));
        d.holders[0].values = [("voice:nothing.index".to_string(), HolderValue::Number(1.0))].into();
        assert!(resolve(&d, &BTreeMap::new(), false, &core_rigs, &no_sets).unwrap_err()[0].contains("names no node"));
    }

    /// the four templates (SCENES.md §9.5) are well formed: they resolve empty, with their previews and with fillers
    #[test]
    fn the_templates_resolve() {
        for name in ["place", "solo", "stage", "avatar-home"] {
            let t = fixture(&format!("templates/{name}"), None);
            assert_eq!(t.id, name);
            assert!(t.toons.is_empty(), "{name}: a template names no toon");
            assert!(t.location_placeholder().is_some(), "{name}: its location is a placeholder");
            let empty = resolve(&t, &BTreeMap::new(), false, &no_rigs, &no_sets).unwrap();
            assert!(empty.cast.is_empty() && empty.location.is_none(), "{name}");
            resolve(&t, &BTreeMap::new(), true, &no_rigs, &no_sets).unwrap();
        }
        let stage = fixture("templates/stage", None);
        let r = resolve(&stage, &with(&[("where", one("location:dom")), ("left", one("character:catpope")), ("right", one("character:toptoy"))]), false, &core_rigs, &no_sets).unwrap();
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
        let refused = |w: &[(&str, Fillers)]| resolve(&stage, &with(w), false, &no_rigs, &no_sets).unwrap_err().join(" | ");
        assert!(refused(&[("left", one("item:incense"))]).contains("holder left: 'item:incense' is an item, the placeholder takes character"));
        assert!(refused(&[("where", one("character:catpope"))]).contains("location where: 'character:catpope' is a character"));
        assert!(refused(&[("nowhere", one("character:catpope"))]).contains("'nowhere' is no placeholder of scene:stage"));
        assert!(refused(&[("left", one("character:catpope")), ("right", one("character:catpope"))]).contains("holder right: 'character:catpope' stands on this stage already"));
        assert!(refused(&[("left", Fillers::Many(vec!["character:a".into(), "character:b".into()]))]).contains("one filler, 2 given"));
        assert!(resolve(&nave(), &with(&[("catpope", one("character:toptoy"))]), false, &no_rigs, &no_sets).unwrap_err()[0].contains("authored filler"));
        let u = Use { scene: "scene:other".into(), with: BTreeMap::new() };
        assert!(u.resolve(&stage, false, &no_rigs, &no_sets).unwrap_err()[0].contains("the use names scene:other"));
    }

    /// #504: a use is checked in the toon's book – the scene and every filler listed there – and against each `fill`;
    /// every reason at once, none for a good one
    #[test]
    fn a_use_is_checked_in_the_toons_book() {
        let stage = fixture("templates/stage", None);
        let book: Vec<String> = ["scene:stage", "location:dom", "character:catpope", "character:toptoy", "item:incense"].map(String::from).to_vec();
        let u: Use = serde_json::from_str(r#"{"scene":"scene:stage","with":{"where":"location:dom","left":"character:catpope","right":"character:toptoy"}}"#).unwrap();
        assert_eq!(u.problems(&stage, Some(&book)), Vec::<String>::new());
        assert_eq!(u.problems(&stage, None), Vec::<String>::new(), "no book: the fills alone");
        let wrong: Use = serde_json::from_str(r#"{"scene":"scene:stage","with":{"left":"item:incense","right":"character:nobody"}}"#).unwrap();
        let p = wrong.problems(&stage, Some(&book));
        assert_eq!(p, ["right: 'character:nobody' is not in the toon's book", "holder left: 'item:incense' is an item, the placeholder takes character"], "{p:?}");
        let elsewhere = Use { scene: "scene:stage".into(), with: BTreeMap::new() };
        assert_eq!(elsewhere.problems(&stage, Some(&["location:dom".to_string()])), ["scene:stage is not in the toon's book (a scene of the toon, or a template)"]);
        assert!(serde_json::from_str::<Use>(r#"{"scene":"scene:stage","fillers":{}}"#).is_err(), "unknown fields are refused");
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
        let r = resolve(&choir, &with(&[("crows", crows(3))]), false, &no_rigs, &no_sets).unwrap();
        let at: Vec<[f32; 3]> = r.cast.iter().map(|m| m.at).collect();
        assert_eq!(at, [[-0.5, 0.0, 0.0], [0.0, 0.0, 0.0], [0.5, 0.0, 0.0]]);
        assert!(r.cast.iter().all(|m| m.spot.as_deref() == Some("spot_beam")));
        assert_eq!(r.location.as_deref(), Some("location:cathedral"));
        assert!(resolve(&choir, &with(&[("crows", crows(4))]), false, &no_rigs, &no_sets).unwrap_err()[0].contains("at most 3, 4 given"));
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
        assert_eq!(resolve(&choir, &w, false, &no_rigs, &no_sets).unwrap().cast.len(), 5);
        // #516: leave frees the room – the next one takes it again
        assert_eq!(casting.leave("item:crow2"), Some("crows".into()));
        assert_eq!(casting.leave("item:crow2"), None, "gone already");
        assert_eq!(casting.leave("character:catpope.pope"), Some("pope".into()), "any persona of her place");
        assert_eq!(casting.join("item:crow5"), Ok("crows".into()));
        assert_eq!(casting.with()["crows"], Fillers::Many(vec!["item:crow1".into(), "item:crow3".into(), "item:crow5".into()]));
        assert!(!casting.with().contains_key("pope"), "an emptied placeholder is no filler");
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
        let empty = resolve(&place, &BTreeMap::new(), false, &no_rigs, &no_sets).unwrap();
        assert_eq!(empty.location, None);
        assert_eq!(space(&empty.scene)["join"].as_str(), Some("deck"), "the staging's room");
        assert!(space(&empty.scene).get("set").is_none());
        assert!(empty.notes.is_empty(), "an empty location is said nowhere");
        let at = resolve(&place, &with(&[("where", one("location:dom"))]), false, &no_rigs, &no_sets).unwrap();
        assert_eq!(at.location.as_deref(), Some("location:dom"));
        assert!(space(&at.scene).get("set").is_none(), "a location whose set is not at hand: the neutral staging");
        assert_eq!(at.notes, ["location:dom names no set here: the neutral staging plays"]);
    }

    /// #512: a filled location brings its set – the caller's lookup (scene-content's `locations`, as the server's
    /// SceneContentTest writes it) – onto the space node, in the staging's room; a location that names none stays neutral
    #[test]
    fn a_location_brings_its_set() {
        let content = crate::push::Content::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/scene-content-location.json"))).unwrap();
        let sets = |l: &str| content.set_of(l);
        let place = fixture("templates/place", None);
        let at = resolve(&place, &with(&[("where", one("location:cathedral"))]), false, &no_rigs, &sets).unwrap();
        assert_eq!(space(&at.scene)["set"].as_str(), Some("cathedral"));
        assert_eq!(space(&at.scene)["join"].as_str(), Some("deck"), "in the staging's room");
        assert!(at.notes.is_empty());
        let dom = resolve(&place, &with(&[("where", one("location:dom"))]), false, &no_rigs, &sets).unwrap();
        assert!(space(&dom.scene).get("set").is_none());
        assert_eq!(dom.notes.len(), 1);
        // a scene that names its location itself takes the same set; a `space.set` beside it stays refused
        let named = doc(r#"{"format":2,"id":"vespers","location":"location:cathedral","node":[{"id":"space","kind":"space","quality":"high"}]}"#);
        let r = resolve(&named, &BTreeMap::new(), false, &no_rigs, &sets).unwrap();
        assert_eq!((space(&r.scene)["set"].as_str(), space(&r.scene)["quality"].as_str()), (Some("cathedral"), Some("high")), "the document's own params win");
        let both = doc(r#"{"format":2,"id":"vespers","location":"location:cathedral","node":[{"id":"space","kind":"space","set":"cave"}]}"#);
        assert!(resolve(&both, &BTreeMap::new(), false, &no_rigs, &sets).unwrap_err()[0].contains("`set` is refused"));
    }

    /// a core rig plus the one wire a person draws to make its being breathe: its sum's level into the being's pulse
    /// (#492, owner 2026-11-12: no default breath, only what is wired moves a being)
    fn breathing(subject: &str, v: Option<&str>) -> Option<Bundle> {
        let mut b = core_rigs(subject, v)?;
        b.rig.routes.push(crate::rig::Route { from: "sound:level".into(), to: "being:pulse".into(), scale: 0.6, offset: 0.0 });
        Some(b)
    }

    /// two rigs on one stage: prefixed by their holders, summed into the one `out` – each rig moves its own look, an
    /// item's too (#495: its card); no rig moves another's being (#492)
    #[test]
    fn two_rigs_share_the_out_and_the_space() {
        let d = doc(r#"{"format":2,"id":"duo","holder":[{"id":"pope","ref":"character:catpope"},{"id":"burner","fill":{"kind":["item"]}}],
          "node":[{"id":"space","kind":"space","set":"cathedral"}]}"#);
        let r = resolve(&d, &with(&[("burner", one("item:incense_burner"))]), false, &breathing, &no_sets).unwrap();
        let ids: Vec<&str> = r.scene.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"pope_voice_voice") && ids.iter().any(|i| i.starts_with("burner_")), "{ids:?}");
        assert_eq!(ids.iter().filter(|i| **i == "out").count(), 1);
        assert_eq!(ids.iter().filter(|i| **i == "space").count(), 1);
        assert!(r.scene.links.iter().all(|l| node_port(&l.to).0 != "space"), "no rig drives the room's being");
        let into = |node: &str| r.scene.links.iter().filter(|l| node_port(&l.to).0 == node).map(|l| (l.from.clone(), l.to.clone())).collect::<Vec<_>>();
        assert_eq!(into("pope_look"), [("pope_fft.level".to_string(), "pope_look.pulse".to_string())], "her rig's mapping, onto her look");
        assert_eq!(into("burner_look"), [("burner_fft.level".to_string(), "burner_look.pulse".to_string())], "the item's rig moves its own look (#495)");
        let who = |id: &str| r.scene.nodes.iter().find(|n| n.id == id).and_then(|n| n.params["who"].as_str().map(String::from));
        assert_eq!((who("pope_look").as_deref(), who("burner_look").as_deref()), (Some("character:catpope"), Some("item:incense_burner")));
        assert!(r.notes.is_empty(), "{:?}", r.notes);
        assert_eq!(r.cast.iter().map(|m| m.slots.clone()).collect::<Vec<_>>(), [Some("pope_".to_string()), Some("burner_".to_string())]);
        assert_eq!(space(&r.scene)["set"].as_str(), Some("cathedral"));
        #[cfg(feature = "being")]
        {
            let look: crate::space::being::Look = r.scene.nodes.iter().find(|n| n.id == "pope_look").unwrap().params["being"].clone().try_into().unwrap();
            assert!(look.expose.iter().all(|e| e.starts_with("pope_")) && look.character == "catpope", "{look:?}");
            assert_eq!(r.scene.being.character, "", "the space's own being is no one's");
        }
    }

    /// a character whose rig only listens (a silent track) maps nothing to its being
    pub(crate) const MUTE_RIG: &str = r#"
[rig]
subject = "character.mute"
track = [{ id = "ear", instrument = "ear" }]

[[instrument]]
id = "ear"
kind = "pad"
name = { en = "Ear" }
node = [{ id = "midi", kind = "world", source = "midi", listen = ["midi.note"] }]
"#;

    /// #492 (SCENES.md §9.6 2·1): `scene:stage` with two fillers – a look per character at its holder's spot, each moved
    /// only by its own rig's mappings to the being; a rig with none moves nothing, no "first rig moves the being"
    #[test]
    fn each_holder_gets_a_look_moved_by_its_own_rig() {
        let stage = fixture("templates/stage", None);
        let mute: Bundle = toml::from_str(MUTE_RIG).unwrap();
        let rigs = |s: &str, v: Option<&str>| if s == "character.mute" { Some(mute.clone()) } else { breathing(s, v) };
        let r = resolve(&stage, &with(&[("left", one("character:catpope")), ("right", one("character:mute"))]), false, &rigs, &no_sets).unwrap();
        let looks: Vec<(&str, &str, &str, &str)> = r
            .scene
            .nodes
            .iter()
            .filter(|n| n.kind == "look")
            .map(|n| (n.id.as_str(), n.params["holder"].as_str().unwrap(), n.params["spot"].as_str().unwrap(), n.params["being"]["character"].as_str().unwrap_or_default()))
            .collect();
        assert_eq!(looks, [("left_look", "left", "spot_left", "catpope"), ("right_look", "right", "spot_right", "mute")]);
        let into = |node: &str| r.scene.links.iter().filter(|l| node_port(&l.to).0 == node).map(|l| l.from.clone()).collect::<Vec<_>>();
        assert_eq!(into("left_look"), ["left_fft.level"], "Cat Pope breathes with her own sum, as her rig wires it");
        assert!(into("right_look").is_empty(), "a rig with no mapping to the being moves nothing");
        assert!(into("space").is_empty());
        let g = crate::graph::Graph::build(&r.scene).unwrap();
        assert_eq!(g.nodes.iter().find(|n| n.id == "left_look").unwrap().kind, crate::graph::Kind::Look);
        // one filler alone at a holder's spot stands there too: a look, not the space's being
        let solo = resolve(&stage, &with(&[("left", one("character:catpope"))]), false, &core_rigs, &no_sets).unwrap();
        assert!(solo.scene.nodes.iter().any(|n| n.id == "left_look"));
        assert_eq!(solo.cast[0].slots.as_deref(), Some("left_"));
    }

    const STAGE_DUO_HEAD: &str = "\
# A sandbox (#492): the template scene:stage resolved by the core with two fillers – Cat Pope at `left` with her core rig, Top
# Toy at `right` with no rig at hand (silent). The space draws one being per `look` node at its holder's spot (no location
# yet: the neutral staging, side by side). Neither moves: her core rig wires nothing to her being and there is no default
# breath (owner 2026-11-12) – wire `sound:level → being:pulse` in her rig and only her light breathes. Written by
# deck-space's scene_resolve.rs test `the_stage_duo_sandbox_is_the_core_s_resolve` – never by hand:
#   UPDATE_FIXTURES=1 cargo test -p deck-space stage_duo
# From deck/:  cargo run -p deck-native --release -- --dev --scene stage-duo --windowed
";

    /// #492: the sandbox `stage-duo` is the core's resolve of scene:stage with two fillers, so a deck (native, the browser's
    /// dev build) shows the stage of several beings the scenario checks
    #[test]
    fn the_stage_duo_sandbox_is_the_core_s_resolve() {
        let stage = fixture("templates/stage", None);
        let r = resolve(&stage, &with(&[("left", one("character:catpope")), ("right", one("character:toptoy"))]), false, &core_rigs, &no_sets).unwrap();
        let mut scene = r.scene;
        scene.stage.name = "Stage duo".into();
        let text = format!("{STAGE_DUO_HEAD}\n{}", toml::to_string(&scene).unwrap());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../native/scenes/sandbox/stage-duo.toml");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
            return;
        }
        assert_eq!(std::fs::read_to_string(path).unwrap().replace('\r', ""), text, "stage-duo is stale: UPDATE_FIXTURES=1 cargo test -p deck-space stage_duo");
        assert_eq!(Scene::parse(crate::scene::sandbox("stage-duo").unwrap()).unwrap(), scene, "what the decks read is what the core resolved");
        assert_eq!(scene.nodes.iter().filter(|n| n.kind == "look").count(), 2);
    }

    const STAGE_CARDS_HEAD: &str = "\
# A sandbox (#495): a stage in the cathedral's 3D set with a character and an item – the document
# deck/headless/documents/nave-cast.json resolved by the core. Cat Pope has 3D here, so she stands as her look (her core
# rig wires nothing to her being: still); the incense burner is an item, which has no 3D, so it stands as its 2D card –
# moved only by its own rig's wire `sound:level → being:pulse` (added here to its core rig, as a person draws it). Take
# the set away (`set`) and Cat Pope is a card too (cast.rs). Written by deck-space's scene_resolve.rs test
# `the_stage_cards_sandbox_is_the_core_s_resolve` – never by hand:
#   UPDATE_FIXTURES=1 cargo test -p deck-space stage_cards
# From deck/:  cargo run -p deck-native --release -- --dev --scene stage-cards --windowed
";

    /// #495: the sandbox `stage-cards` is the core's resolve of a stage in a 3D set with a character (a look) and an item
    /// (a card moved by its own wire), so a deck shows both forms the headless scenario 32 checks
    #[test]
    fn the_stage_cards_sandbox_is_the_core_s_resolve() {
        let d = doc(include_str!("../../headless/documents/nave-cast.json"));
        let rigs = |s: &str, v: Option<&str>| if s.starts_with("item.") { breathing(s, v) } else { core_rigs(s, v) };
        let r = resolve(&d, &BTreeMap::new(), false, &rigs, &no_sets).unwrap();
        let mut scene = r.scene;
        scene.stage.name = "Stage cards".into();
        let text = format!("{STAGE_CARDS_HEAD}\n{}", toml::to_string(&scene).unwrap());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../native/scenes/sandbox/stage-cards.toml");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
            return;
        }
        assert_eq!(std::fs::read_to_string(path).unwrap().replace('\r', ""), text, "stage-cards is stale: UPDATE_FIXTURES=1 cargo test -p deck-space stage_cards");
        assert_eq!(Scene::parse(crate::scene::sandbox("stage-cards").unwrap()).unwrap(), scene, "what the decks read is what the core resolved");
        assert!(crate::cast::stage_3d(&scene), "the cathedral's set: a 3D stage");
        let who = |id: &str| crate::cast::Who::parse(scene.nodes.iter().find(|n| n.id == id).unwrap().params["who"].as_str().unwrap()).unwrap();
        assert_eq!(crate::cast::form(&who("pope_look"), true), crate::cast::Form::Look);
        assert_eq!(crate::cast::form(&who("burner_look"), true), crate::cast::Form::Card);
        assert!(scene.links.iter().any(|l| l.to == "burner_look.pulse"), "the burner's own wire moves its card");
    }

    /// #492 (left open by #489): a code rig's policy drives its own holder's tracks and strips, never another's
    #[test]
    fn a_code_rigs_policy_is_reprefixed_per_holder() {
        let code = crate::rig::policy::tests::with_policy(
            r#"{ "name": "regtest_stage_policy", "domain": "control", "abi": 2, "ports": [
                { "name": "index", "dir": "out", "type": "control", "path": "voice:voice.index" },
                { "name": "level", "dir": "out", "type": "control", "path": "drone:@gain", "mode": "mul" } ] }"#,
        );
        let rigs = |s: &str, _: Option<&str>| s.starts_with("character.").then(|| code.clone());
        let stage = fixture("templates/stage", None);
        let r = resolve(&stage, &with(&[("left", one("character:catpope")), ("right", one("character:toptoy"))]), false, &rigs, &no_sets).unwrap();
        for h in ["left", "right"] {
            let p = r.scene.nodes.iter().find(|n| n.id == format!("{h}_policy")).unwrap();
            assert_eq!(p.params["@slots"].as_str(), Some(format!("{h}_").as_str()));
            assert_eq!(p.params["@gain"]["drone"].as_str(), Some(format!("{h}_mix1.gain2").as_str()));
        }
        let g = crate::graph::Graph::build(&r.scene).unwrap();
        let bound: Vec<(String, String)> = g
            .links
            .iter()
            .filter(|l| g.nodes[l.from.0].id.ends_with("_policy"))
            .map(|l| (g.nodes[l.from.0].id.clone(), format!("{}.{}", g.nodes[l.to.0].id, g.nodes[l.to.0].kind.inputs()[l.to.1].name)))
            .collect();
        let want = [("left", "left_voice_voice.index"), ("left", "left_mix1.gain2"), ("right", "right_voice_voice.index"), ("right", "right_mix1.gain2")];
        assert_eq!(bound, want.map(|(h, to)| (format!("{h}_policy"), to.to_string())));
    }
}
