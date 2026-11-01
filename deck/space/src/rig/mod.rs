//! Instruments, presets and character rigs (#191, research/INSTRUMENTS.md): the data a being's sound is made of.
//! - instrument.rs – a patch of the node graph with its exposed parameters and a kind (drum, lead, pad, voice …)
//! - preset.rs – values + routing changes + mappings for one instrument, with a parent (derivatives, lineage)
//! - bundle.rs – a rig with what it names (scene-content's answer) → the played scene + the page's knobs
//! - engine.rs – a flattened rig lowered for live-core (the browser's batch engine) + what the page plays itself
//! - flatten.rs – a rig as one playable scene (the tracks summed into audio_out, the routes as links)
//! - here – a rig: a character's (or a scene object's, #210) tracks, each an instrument + its default preset, routes
//!   between tracks and to the being's look, and per-persona / per-mood variants; `Rig::play` resolves it.
//!
//! JSON (the platform's entity documents) and TOML (the core set in the deck image) read the same serde forms.
pub mod bundle;
pub mod engine;
mod flatten;
pub mod instrument;
pub mod preset;

pub use bundle::{Bundle, Knob, Played};
pub use instrument::{Expose, Instrument, InstrumentKind, Patch};
pub use preset::{Mapping, Preset, PresetBook, Resolved};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a locale map (`{ de, en }`), the platform's text form
pub type Text = BTreeMap<String, String>;

/// How a deck holds a definition (#191, the carrier bag #192): lent by a conductor for one scene, owned in the member's
/// carrier bag, or core – shipped in the deck image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    Lent,
    Owned,
    Core,
}

/// What a holder may do (owner 2026-10-30: "the definition says who may save, copy or change it").
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
}

/// One track: an instrument, its default preset, its mixer strip.
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
/// Every track reaches the master through its mixer strip; an audio route is a send into another track's audio input
/// (an fx track; `scale` = its level), a control route a modulation.
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Variant {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub presets: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f32>,
}

/// What a rig needs to play: the instruments and presets by id (scene-content delivers them, #195).
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub instruments: BTreeMap<String, Instrument>,
    pub presets: PresetBook,
}

impl Library {
    pub fn new(instruments: impl IntoIterator<Item = Instrument>, presets: impl IntoIterator<Item = Preset>) -> Library {
        Library {
            instruments: instruments.into_iter().map(|i| (i.id.clone(), i)).collect(),
            presets: PresetBook::new(presets),
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

/// `<scope>:<rest>` → (scope, rest)
fn scoped(r: &str) -> Option<(&str, &str)> {
    r.split_once(':')
}

impl Rig {
    /// The rig checked against `lib`: unique tracks, known instruments and presets (of the track's instrument), variants
    /// and routes naming known tracks.
    pub fn check(&self, lib: &Library) -> Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        for t in &self.tracks {
            if !seen.insert(t.id.as_str()) {
                bail!("rig '{}': track '{}' twice", self.subject, t.id);
            }
            if t.id.contains([':', '.']) {
                bail!("rig '{}': a track id has no ':' or '.' ({})", self.subject, t.id);
            }
            lib.instrument(&t.instrument)?;
            if let Some(p) = &t.preset {
                self.preset_fits(lib, t, p)?;
            }
        }
        for (name, v) in self.personas.iter().map(|(n, v)| (format!("persona {n}"), v)).chain(
            self.moods.iter().map(|(n, v)| (format!("mood {n}"), v)),
        ) {
            for (track, p) in &v.presets {
                self.preset_fits(lib, self.track(track).with_context(|| name.clone())?, p)?;
            }
            for path in v.values.keys() {
                let (track, inner) = scoped(path).with_context(|| format!("{name}: '{path}' is not <track>:<node>.<port>"))?;
                lib.instrument(&self.track(track).with_context(|| name.clone())?.instrument)?.control_input(inner)?;
            }
        }
        for r in &self.routes {
            for end in [&r.from, &r.to] {
                let Some((scope, _)) = scoped(end) else { bail!("rig '{}': route end '{end}' has no scope", self.subject) };
                if !(scope == "being" && end == &r.to) {
                    self.track(scope)?;
                }
            }
        }
        Ok(())
    }

    fn track(&self, id: &str) -> Result<&Track> {
        match self.tracks.iter().find(|t| t.id == id) {
            Some(t) => Ok(t),
            None => bail!("rig '{}': no track '{id}'", self.subject),
        }
    }

    fn preset_fits(&self, lib: &Library, t: &Track, preset: &str) -> Result<()> {
        let p = lib.presets.get(preset)?;
        if p.instrument != t.instrument {
            bail!("rig '{}': track '{}' plays '{}', preset '{preset}' is for '{}'", self.subject, t.id, t.instrument, p.instrument);
        }
        Ok(())
    }

    /// Every unmuted track patched for `persona` and `mood`: the track's preset ← the persona's ← the mood's (the last
    /// one named wins), folded through its lineage, then the persona's and the mood's single values on top.
    pub fn play(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>) -> Result<Vec<TrackPatch>> {
        let variants: Vec<&Variant> = [persona.and_then(|p| self.personas.get(p)), mood.and_then(|m| self.moods.get(m))]
            .into_iter()
            .flatten()
            .collect();
        let mut out = Vec::new();
        for t in self.tracks.iter().filter(|t| !t.mute) {
            let preset = variants.iter().rev().find_map(|v| v.presets.get(&t.id)).or(t.preset.as_ref());
            let mut resolved = match preset {
                Some(p) => {
                    self.preset_fits(lib, t, p)?;
                    lib.presets.resolve(p)?
                }
                None => Resolved::bare(&t.instrument),
            };
            for v in &variants {
                for (path, value) in &v.values {
                    if let Some((track, inner)) = scoped(path) {
                        if track == t.id {
                            resolved.values.insert(instrument::canonical(inner), *value);
                        }
                    }
                }
            }
            let patch = lib.instrument(&t.instrument)?.patch(&resolved).with_context(|| format!("track '{}'", t.id))?;
            out.push(TrackPatch { track: t.id.clone(), preset: preset.cloned(), patch });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
