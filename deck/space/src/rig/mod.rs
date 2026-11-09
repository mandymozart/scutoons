//! Instruments, presets and character rigs (#191, research/INSTRUMENTS.md): the data a being's sound is made of.
//! - instrument.rs – a patch of the node graph with its exposed parameters and a kind (drum, lead, pad, voice …)
//! - preset.rs – values + routing changes + mappings for one instrument, with a parent (derivatives, lineage)
//! - assign.rs – the assignment (#283): which characters have a default rig, a draft rig's bundle from the catalog
//! - bundle.rs – a rig with what it names (scene-content's answer) → the played scene + the page's knobs
//! - content.rs – scene-content's answer (its rigs, instruments, presets with their versions) → a bundle per holder
//! - engine.rs – a flattened rig split for live-core (deck-engine in the browser): the kinds it carries, and the rest
//! - horizon.rs – the mood horizon: the moods reachable next, played ahead (deck-events' horizon), swapped on the trigger
//! - flatten.rs – a rig as one playable scene (the tracks summed into audio_out, the routes as links)
//! - validate.rs – an instrument / preset / rig document checked with what it names (#313: the API's catalog writes via
//!   deck/build `POST /validate`, the creator's hints via space-web)
//! - wiring.rs – a played scene's nodes, ports and links as the rig designer's Nodes tab draws them (#282)
//! - policy.rs – a code rig's policy (#408): the module that drives the tracks through path targets, its checks and the
//!   paths it owns
//! - face.rs – `Rig::describe`: what a rig lists to the editor and the validator (tracks, addressable inputs, the policy)
//! - address.rs – the one address format every cable, value and stick is written in, and `Reach`: how far an address
//!   may reach from its document (#432, research/MODULE-WIRING.md §3)
//! - edit.rs – a graph gesture as one change of the one document that holds the cable or node (`Op`, #434), "used by N",
//!   and the text view: the same document as TOML
//! - here – a rig: a character's (or a scene object's, #210) tracks, each an instrument + its default preset, routes
//!   between tracks and to the being's look, and per-persona / per-mood variants; `Rig::play` resolves it.
//!
//! JSON (the platform's entity documents) and TOML (the core set in the deck image) read the same serde forms. A check
//! lists every problem it finds at its cable, node or track (problem.rs), not the first alone.
pub mod address;
pub mod assign;
pub mod edit;
pub mod bundle;
pub mod content;
pub mod engine;
pub mod face;
mod flatten;
pub mod horizon;
pub mod instrument;
pub mod policy;
pub mod preset;
pub mod template;
pub mod validate;
pub mod wiring;

pub use bundle::{Bundle, Diff, Knob, Played, PresetChoice, Stream};
pub use face::RigFace;
pub use policy::Policy;
pub use horizon::Prebuilt;
pub use instrument::{Expose, Instrument, InstrumentKind, Patch};
pub use preset::{Mapping, Preset, PresetBook, Resolved};

use crate::problem::Problems;
use crate::world::{Source, World};
use address::{Address, Leaf};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a locale map (`{ de, en }`), the platform's text form
pub type Text = BTreeMap<String, String>;

/// How a deck holds a definition (#191, the carrier bag #192): lent by a conductor for one scene, owned in the member's
/// carrier bag, or core – shipped in the deck image.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    Lent,
    Owned,
    Core,
}

/// What a holder may do (owner 2026-10-30: "the definition says who may save, copy or change it").
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    /// load and hear it
    Play,
    /// turn its knobs live (not saved)
    Tweak,
    /// save a derivative (a child preset) into one's carrier bag
    Derive,
    /// save a copy without the lineage
    Copy,
}

/// Rights per way of holding, carried by every definition. Default: lent = play + tweak, core = + derive, owned = all.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Rights {
    pub lent: Vec<Right>,
    pub owned: Vec<Right>,
    pub core: Vec<Right>,
}

impl Default for Rights {
    fn default() -> Self {
        use Right::*;
        Rights { lent: vec![Play, Tweak], owned: vec![Play, Tweak, Derive, Copy], core: vec![Play, Tweak, Derive] }
    }
}

impl Rights {
    pub fn allows(&self, held: Hold, right: Right) -> bool {
        match held {
            Hold::Lent => &self.lent,
            Hold::Owned => &self.owned,
            Hold::Core => &self.core,
        }
        .contains(&right)
    }
}

/// A character's (or object's) set of tracks.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rig {
    /// whose rig: `character.<id>` (`character.<id>.<persona>` for a persona's own) or `item.<id>` – KANON's subjects
    pub subject: String,
    #[serde(rename = "track")]
    pub tracks: Vec<Track>,
    /// sends and modulation between tracks, and to the being's look (`being:hue`) – everything is audiovisual
    #[serde(default, rename = "route", skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<Route>,
    /// persona → overrides (applied first)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub personas: BTreeMap<String, Variant>,
    /// mood (the `moods` registry's ids) → overrides (applied after the persona's)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub moods: BTreeMap<String, Variant>,
    #[serde(default)]
    pub rights: Rights,
    /// the two corner sticks of the phone deck, `[[x, y], [x, y]]` (left, right) as addresses of its knobs
    /// (`<track>:<node>.<port>`, a code rig's `policy:<param>`; "" = none); none = the first four knobs (#191, #432)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticks: Option<[[String; 2]; 2]>,
    /// a code rig (#408): the control module that drives the tracks (policy.rs); none = the tables alone
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<Policy>,
}

/// One track: an instrument, its default preset, its mixer strip.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: String,
    pub instrument: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default = "one")]
    pub gain: f32,
    /// −1 left … 1 right
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub mute: bool,
}

fn one() -> f32 {
    1.0
}

/// `from` / `to` are `<track>:<node>.<port>`; `to` may be `being:<input>` (the space node's hue, glow, light, pulse).
/// Every sounding track reaches the master through its mixer strip; an audio route is a send into another track's audio
/// input (an fx track; `scale` = its level), a control route a modulation, an event route an event cable (#430: many to
/// many, a note source's silent track to a synth's track).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub from: String,
    pub to: String,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub offset: f32,
}

/// A persona's or a mood's overrides: another preset per track, and single values (`<track>:<node>.<port>`).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Variant {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub presets: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f32>,
}

/// What a rig needs to play: the instruments and presets by id (scene-content delivers them, #195), and the descriptor
/// of a code rig's policy module by its name (#408).
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub instruments: BTreeMap<String, Instrument>,
    pub presets: PresetBook,
    pub modules: BTreeMap<String, deck_catalog::Descriptor>,
}

impl Library {
    pub fn new(instruments: impl IntoIterator<Item = Instrument>, presets: impl IntoIterator<Item = Preset>) -> Library {
        Library {
            instruments: instruments.into_iter().map(|i| (i.id.clone(), i)).collect(),
            presets: PresetBook::new(presets),
            modules: BTreeMap::new(),
        }
    }

    /// the library with these module descriptors (a policy's), by name
    pub fn with_modules(mut self, modules: impl IntoIterator<Item = deck_catalog::Descriptor>) -> Library {
        self.modules.extend(modules.into_iter().map(|d| (d.name.to_string(), d)));
        self
    }

    /// the library's modules registered as node kinds (`module:<name>`), so its rigs build as graphs anywhere – the
    /// browser's, the build service's and the headless host's validation (a loaded module's registration is the same
    /// kind). Done when a checked rig is played: a refused draft's descriptor must never become the newest of its name.
    pub(crate) fn register_modules(&self) {
        for d in self.modules.values() {
            crate::modreg::register(&d.name, &crate::modreg::ModuleDef::of(d));
        }
    }

    pub fn instrument(&self, id: &str) -> Result<&Instrument> {
        match self.instruments.get(id) {
            Some(i) => Ok(i),
            None => bail!("no instrument '{id}'"),
        }
    }
}

/// A track ready to build: which preset it plays (after the variants) and the patched instrument.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackPatch {
    pub track: String,
    pub preset: Option<String>,
    pub patch: Patch,
}

impl Rig {
    /// The rig checked against `lib`: unique tracks, known instruments and presets (of the track's instrument), variants
    /// and routes naming known tracks, every route end a relative address of a port (#432: a rig's cable stays relative,
    /// an absolute address sits on a world node), a far world node naming something outside the rig; a code rig's
    /// policy, its targets and who writes which input (policy.rs). Every problem found is listed (problem.rs), at its
    /// track, variant, route or node.
    pub fn check(&self, lib: &Library) -> Result<()> {
        let mut out = Problems::default();
        let rig = format!("rig {}", self.subject);
        let mut seen = std::collections::BTreeSet::new();
        for t in &self.tracks {
            if !seen.insert(t.id.as_str()) {
                out.push(&rig, format!("track '{}' twice", t.id));
            }
            if t.id.contains([':', '.', '/', '@', '#']) || t.id.chars().any(char::is_whitespace) {
                out.push(&rig, format!("a track id has no ':', '.', '/', '@', '#' or space ({})", t.id));
            }
            if t.id == policy::NODE {
                out.push(&rig, format!("the track id '{}' is reserved for a code rig's policy", t.id));
            }
            if t.id == address::BEING {
                out.push(&rig, format!("the track id '{}' is reserved for the being's look", t.id));
            }
            if !lib.instruments.contains_key(&t.instrument) {
                out.push(&rig, format!("unknown instrument \"{}\"", t.instrument));
            } else if let Some(p) = &t.preset {
                if let Err(e) = self.preset_fits(lib, t, p) {
                    out.note(e);
                }
            }
        }
        for (name, v) in self.personas.iter().map(|(n, v)| (format!("persona {n}"), v)).chain(
            self.moods.iter().map(|(n, v)| (format!("mood {n}"), v)),
        ) {
            for (track, p) in &v.presets {
                let fits = self.track(track).and_then(|t| self.preset_fits(lib, t, p));
                if let Err(e) = fits {
                    out.push(&name, format!("{e:#}"));
                }
            }
            for path in v.values.keys() {
                if let Err(e) = self.value_input(lib, path) {
                    out.push(&name, e);
                }
            }
        }
        for r in &self.routes {
            let at = format!("route {} → {}", r.from, r.to);
            for (end, is_to) in [(&r.from, false), (&r.to, true)] {
                if let Err(e) = self.route_end(end, is_to) {
                    out.push(&at, e);
                }
            }
        }
        if let Err(e) = self.check_policy(lib) {
            out.note(e);
        }
        // a far world node (#432) brings in what is outside the rig: naming the rig itself is a cable's job
        let context = address::Context { own: address::Place::of_subject(&self.subject) };
        for (track, node, a) in self.far_world_nodes(lib) {
            let at = format!("world node '{node}' (track {track})");
            match address::resolve(&a, &context, address::Reach::Wide) {
                Ok(r) if r.is_far() => {}
                Ok(_) => out.push(at, format!("'{a}' names this rig itself – inside the rig a cable joins the ports; a world node brings in what is outside")),
                Err(e) => out.push(at, e),
            }
        }
        out.into_result()
    }

    /// a persona's or mood's value path: `<track>:<node>.<port>`, a control input of the track's instrument, or a code
    /// rig's `policy:<param>`
    fn value_input(&self, lib: &Library, path: &str) -> Result<(), String> {
        let leaf = Address::leaf(path).map_err(|e| format!("'{path}': {e}"))?;
        match leaf {
            Leaf::Port { track: Some(track), node, port } => {
                let t = self.track(&track).map_err(|e| format!("{e:#}"))?;
                lib.instrument(&t.instrument).and_then(|i| i.control_input(&format!("{node}.{port}"))).map(|_| ()).map_err(|e| format!("{e:#}"))
            }
            Leaf::Policy(name) if self.policy.is_some() => self.policy_param(lib, &name).map_err(|e| format!("{e:#}")),
            Leaf::Policy(_) => Err(format!("'{path}': no track 'policy' and no policy in this rig")),
            _ => Err(format!("'{path}' is not <track>:<node>.<port>")),
        }
    }

    /// a route's end: a relative address of a port – a track's node's, the being's (`to` only), a code rig's policy's
    pub(crate) fn route_end(&self, end: &str, is_to: bool) -> Result<(), String> {
        let a = Address::parse(end).map_err(|e| format!("'{end}': {e}"))?;
        if !a.is_relative() {
            return Err(format!("'{end}' leaves the rig – a rig's cable stays relative, the absolute address sits on a world node"));
        }
        match a.leaf {
            Leaf::Port { track: Some(t), .. } => self.track(&t).map(|_| ()).map_err(|e| format!("{e:#}")),
            Leaf::Port { track: None, .. } => Err(format!("'{end}' has no track – a route end is <track>:<node>.<port>")),
            Leaf::Being(_) if is_to => Ok(()),
            Leaf::Being(_) => Err(format!("'{end}': the being's look is a target only")),
            Leaf::Policy(_) if self.policy.is_some() => Ok(()),
            Leaf::Policy(_) => Err(format!("'{end}': no track 'policy' and no policy in this rig")),
            Leaf::Member { .. } | Leaf::State(_) => Err(format!("'{end}' is not a port – a route joins ports (<track>:<node>.<port>)")),
        }
    }

    /// the world nodes of the rig's instruments whose source is far (#432): (track, node id, its address)
    pub(crate) fn far_world_nodes(&self, lib: &Library) -> Vec<(String, String, Address)> {
        let mut out = Vec::new();
        for t in &self.tracks {
            let Ok(i) = lib.instrument(&t.instrument) else { continue };
            for n in i.nodes.iter().filter(|n| n.kind == crate::graph::Kind::World.name()) {
                if let Ok(World { source: Source::Far(a), .. }) = World::read(&n.id, &n.params) {
                    out.push((t.id.clone(), n.id.clone(), a));
                }
            }
        }
        out
    }

    /// A stick's address → the knob slot it turns (`<track>:<node>.<port>` → `<track>_<node>.<port>`, `policy:<param>` →
    /// `policy.<param>`); whether the slot is one of the rig's knobs is `Bundle::play`'s check.
    pub fn stick_slot(&self, stick: &str) -> Result<String> {
        let leaf = Address::leaf(stick).map_err(|e| anyhow::anyhow!("rig {}: stick '{stick}': {e}", self.subject))?;
        match &leaf {
            Leaf::Port { track: Some(_), .. } | Leaf::Policy(_) => Ok(leaf.slot().expect("a port has a slot")),
            _ => bail!("rig {}: stick '{stick}' names no track – a stick is <track>:<node>.<port> or policy:<param>", self.subject),
        }
    }

    fn track(&self, id: &str) -> Result<&Track> {
        match self.tracks.iter().find(|t| t.id == id) {
            Some(t) => Ok(t),
            None => bail!("rig {}: no track '{id}'", self.subject),
        }
    }

    fn preset_fits(&self, lib: &Library, t: &Track, preset: &str) -> Result<()> {
        let Some(p) = lib.presets.presets.get(preset) else { bail!("rig {}: unknown preset \"{preset}\"", self.subject) };
        if p.instrument != t.instrument {
            bail!("rig {}: track '{}' plays '{}', preset '{preset}' is for '{}'", self.subject, t.id, t.instrument, p.instrument);
        }
        Ok(())
    }

    /// Every unmuted track patched for `persona` and `mood`: the track's preset ← the persona's ← the mood's (the last
    /// one named wins), folded through its lineage, then the persona's and the mood's single values on top.
    pub fn play(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>) -> Result<Vec<TrackPatch>> {
        self.play_with(lib, persona, mood, &BTreeMap::new())
    }

    /// `play` with presets chosen per track (`chosen`: track → preset – a code rig's `@preset` outputs, #410): a choice
    /// wins over the tables. A code rig's owned inputs (a `set` / `mul` target's) lose their preset values (policy.rs).
    pub fn play_with(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>, chosen: &BTreeMap<String, String>) -> Result<Vec<TrackPatch>> {
        lib.register_modules();
        let variants = self.variants(persona, mood);
        let owned = self.owned(lib)?;
        let mut out = Vec::new();
        for t in self.tracks.iter().filter(|t| !t.mute) {
            let preset = chosen.get(&t.id).or_else(|| variants.iter().rev().find_map(|v| v.presets.get(&t.id))).or(t.preset.as_ref());
            let mut resolved = match preset {
                Some(p) => {
                    self.preset_fits(lib, t, p)?;
                    lib.presets.resolve(p)?
                }
                None => Resolved::bare(&t.instrument),
            };
            // decision 2 (#314): a set / mul target is the only writer of its input – the preset's value there is dropped
            // (Rig::warnings says so)
            for (track, path) in &owned {
                if *track == t.id {
                    resolved.values.remove(path);
                }
            }
            for v in &variants {
                for (path, value) in &v.values {
                    if let Ok(Leaf::Port { track: Some(track), node, port }) = Address::leaf(path) {
                        if track == t.id {
                            resolved.values.insert(format!("{node}.{port}"), *value);
                        }
                    }
                }
            }
            let patch = lib.instrument(&t.instrument)?.patch(&resolved).with_context(|| format!("track '{}'", t.id))?;
            out.push(TrackPatch { track: t.id.clone(), preset: preset.cloned(), patch });
        }
        Ok(out)
    }

    /// the persona's and the mood's variants, in the order they apply
    fn variants(&self, persona: Option<&str>, mood: Option<&str>) -> Vec<&Variant> {
        [persona.and_then(|p| self.personas.get(p)), mood.and_then(|m| self.moods.get(m))].into_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod event_tests;
