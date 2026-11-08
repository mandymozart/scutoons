//! A rig's face (#408, research/RIG-MODULE-GROUPS.md §2 Q2): what a rig lists to the editor and the validator, computed
//! once here (the editor only shows it, RULES A2). Each track with its instrument, preset and every addressable input – a
//! control input of its instrument with its range, and the policy output that drives it, if one does; a code rig's
//! policy with its parameters and its targets resolved (a flattened input, a mixer strip, a preset table); the personas
//! and moods the tables name. The children are the rig document's, not the module's: the module's descriptor only lists
//! what it needs (its params and its target paths).
use super::flatten::prefixed;
use super::policy::{self, mode_name, Member, Target};
use super::{Bundle, Library, Rig};
use crate::graph::{Kind, PortType};
use crate::params::{self, Curve};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RigFace {
    pub subject: String,
    pub tracks: Vec<FaceTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<FacePolicy>,
    pub personas: Vec<String>,
    pub moods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaceTrack {
    pub id: String,
    pub instrument: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub mute: bool,
    /// the presets of its instrument the library knows, its own first – what a `@preset` target may choose from
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<String>,
    /// the policy output that chooses its preset (`<track>:@preset`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_driven: Option<Driven>,
    /// the policy output that writes its mixer strip (`<track>:@gain`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gain_driven: Option<Driven>,
    pub addressable: Vec<Addressable>,
}

/// A control input of a track's instrument: what a table value, a route or a policy target may write.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Addressable {
    /// `<track>:<node>.<port>` – the rig document's form
    pub path: String,
    /// `<track>_<node>.<port>` – the flattened scene's input
    pub input: String,
    pub min: f32,
    pub max: f32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub log: bool,
    /// the policy output that drives it (`set` / `mul`: its only writer, "driven by code"; `add`: over the tables)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driven: Option<Driven>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Driven {
    pub output: String,
    pub mode: &'static str,
    /// the policy is the only writer (`Target::owns`: `set` / `mul`): the editor's form does not write it
    pub owned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FacePolicy {
    pub module: String,
    pub params: Vec<FaceParam>,
    pub targets: Vec<FaceTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaceParam {
    pub name: String,
    pub min: f32,
    pub max: f32,
    /// the policy's value, else the default
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaceTarget {
    pub output: String,
    pub path: String,
    pub mode: &'static str,
    pub resolved: Resolved,
}

/// what a target writes in the flattened rig
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "to", rename_all = "lowercase")]
pub enum Resolved {
    /// a flattened control input, `<track>_<node>.<port>`
    Input { input: String },
    /// the track's mixer strip, `mixN.gainK`
    Strip { input: String },
    /// the track's preset, by index into `choices`
    Preset { track: String, choices: Vec<String> },
}

/// which policy output writes a track's `member` – one at most (`Rig::targets` refuses two)
fn driven_by(targets: &[Target], track: &str, member: impl Fn(&Member) -> bool) -> Option<Driven> {
    targets.iter().find(|g| g.track == track && member(&g.member)).map(|g| Driven { output: format!("{}.{}", policy::NODE, g.output), mode: mode_name(g.mode), owned: g.owns() })
}

/// a bundle as JSON (`rig_catalog_bundle`'s answer, a draft as the deck hears it) → its `RigFace` as JSON (the wasm boundary, #412)
pub fn face_json(bundle: &str) -> Result<String> {
    let b = Bundle::parse_json(bundle)?;
    Ok(serde_json::to_string(&b.rig.describe(&b.library())?)?)
}

impl Rig {
    /// The rig's face (see the module doc); the rig is checked first.
    pub fn describe(&self, lib: &Library) -> Result<RigFace> {
        self.check(lib)?;
        let targets = self.targets(lib)?;
        let mut tracks = Vec::new();
        for t in &self.tracks {
            let inst = lib.instrument(&t.instrument)?;
            let mut addressable = Vec::new();
            for n in &inst.nodes {
                let Some(kind) = Kind::parse(&n.kind) else { continue };
                for port in kind.inputs().iter().filter(|p| p.ty == PortType::Control) {
                    let inner = format!("{}.{}", n.id, port.name);
                    let spec = params::spec(kind, port.name);
                    let driven = driven_by(&targets, &t.id, |m| *m == Member::Input(inner.clone()));
                    addressable.push(Addressable { path: format!("{}:{inner}", t.id), input: prefixed(&t.id, &inner), min: spec.min, max: spec.max, log: spec.curve == Curve::Log, driven });
                }
            }
            let preset_driven = driven_by(&targets, &t.id, |m| matches!(m, Member::Preset(_)));
            let gain_driven = driven_by(&targets, &t.id, |m| *m == Member::Gain);
            // its own preset first: a `@preset` output that has not chosen yet reads 0, the first choice (#410)
            let mut presets: Vec<String> = lib.presets.presets.values().filter(|p| p.instrument == t.instrument).map(|p| p.id.clone()).collect();
            presets.sort_by_key(|p| Some(p) != t.preset.as_ref());
            tracks.push(FaceTrack { id: t.id.clone(), instrument: t.instrument.clone(), preset: t.preset.clone(), mute: t.mute, presets, preset_driven, gain_driven, addressable });
        }
        let policy = match self.policy_module(lib)? {
            None => None,
            Some((p, d)) => {
                let played: Vec<&str> = self.tracks.iter().filter(|t| !t.mute).map(|t| t.id.as_str()).collect();
                let params = d
                    .params
                    .iter()
                    .filter(|q| q.choices.is_empty())
                    .map(|q| FaceParam { name: q.name.to_string(), min: q.min, max: q.max, value: p.values.get(q.name.as_ref()).copied().unwrap_or(q.default.unwrap_or(q.min)) })
                    .collect();
                let targets = targets
                    .iter()
                    .map(|g| {
                        let resolved = match &g.member {
                            Member::Input(inner) => Resolved::Input { input: prefixed(&g.track, inner) },
                            Member::Gain => {
                                let (n, k) = policy::strip(played.iter().position(|t| *t == g.track).expect("checked: an unmuted track"));
                                Resolved::Strip { input: format!("mix{n}.gain{k}") }
                            }
                            Member::Preset(choices) => Resolved::Preset { track: g.track.clone(), choices: choices.clone() },
                        };
                        FaceTarget { output: g.output.clone(), path: g.path.clone(), mode: mode_name(g.mode), resolved }
                    })
                    .collect();
                Some(FacePolicy { module: p.module.clone(), params, targets })
            }
        };
        Ok(RigFace { subject: self.subject.clone(), tracks, policy, personas: self.personas.keys().cloned().collect(), moods: self.moods.keys().cloned().collect() })
    }
}
