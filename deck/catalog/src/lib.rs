//! The node-kind catalog (#315, ENGINE-REFACTOR-PLAN §2.4): every kind the engines know – and every module – as one
//! `Descriptor`: its ports (type, direction, channels, doc) and parameters (range, curve, unit, default, choices, doc).
//! The built-in table is `builtin.rs`; a module's `deck.module` JSON deserializes into the same type (ABI v1 fields
//! keep parsing: everything the old descriptor lacks has a serde default). Doc strings are data (`doc` fields), not
//! Rust comments, so they reach `kinds.json`, the editor and the agents.
//!
//! Versioning: `CATALOG_VERSION` goes up when a kind, port or parameter is renamed or removed (a stored-format change:
//! analyse, migrate the documents, then the code – RULES "Migrations and stored data"); adding is additive.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

mod builtin;
pub use builtin::builtin_kinds;

pub const CATALOG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    In,
    Out,
}

/// a port's type; a link joins ports of one type (`cv` = pitch / gate carried in an audio port, see `seq`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PortType {
    Audio,
    Control,
    Cv,
    Video,
    /// events out (deck-events' model): a module's notes and triggers, not a graph cable
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Audio,
    Control,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Curve {
    #[default]
    Lin,
    /// equal ratios per equal travel (frequencies, times, rates); min must be > 0
    Log,
}

/// where a kind stands: usable in a scene / instrument graph, a helper only the batch evaluator runs, or planned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    Graph,
    Internal,
    Planned,
}

/// an audio port's channel count; `any` = whatever the cable carries (§2.6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Channels {
    Count(u8),
    Any(AnyChannels),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AnyChannels {
    #[serde(rename = "any")]
    Any,
}

fn yes() -> bool {
    true
}
fn is_yes(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}
#[allow(clippy::ptr_arg)] // serde's skip_serializing_if passes &field
fn is_empty_str(s: &Cow<'static, str>) -> bool {
    s.is_empty()
}
#[allow(clippy::ptr_arg)]
fn is_empty_slice<T: Clone>(s: &Cow<'static, [T]>) -> bool {
    s.is_empty()
}
fn empty_strs() -> Cow<'static, [Cow<'static, str>]> {
    Cow::Borrowed(&[])
}
fn empty_ports() -> Cow<'static, [PortDesc]> {
    Cow::Borrowed(&[])
}
fn empty_params() -> Cow<'static, [ParamDesc]> {
    Cow::Borrowed(&[])
}
fn empty_str() -> Cow<'static, str> {
    Cow::Borrowed("")
}
fn one() -> Cow<'static, str> {
    Cow::Borrowed("1")
}
fn default_domain() -> Domain {
    Domain::Audio
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PortDesc {
    pub name: Cow<'static, str>,
    pub dir: Dir,
    /// ABI v1 module descriptors call it `kind`
    #[serde(rename = "type", alias = "kind")]
    pub ty: PortType,
    /// audio ports: how many channels the port carries
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<Channels>,
    /// false = deck-core's control graph (its `KindInfo`) does not take this input, e.g. the `cv` of `osc`
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub engine: bool,
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub doc: Cow<'static, str>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParamDesc {
    pub name: Cow<'static, str>,
    #[serde(default)]
    pub min: f32,
    #[serde(default)]
    pub max: f32,
    /// None = not driven until a param or a link sets it (an `events` node's knobs: the patch's own values play)
    #[serde(default)]
    pub default: Option<f32>,
    #[serde(default)]
    pub curve: Curve,
    /// Hz, s, dB, ratio, px …
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub unit: Cow<'static, str>,
    /// a string parameter's values (`osc.wave`); min / max / default are then unused
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub choices: Cow<'static, [Cow<'static, str>]>,
    /// the value is also an input port of this name (a control input is both)
    #[serde(default, skip_serializing_if = "is_false")]
    pub port: bool,
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub doc: Cow<'static, str>,
}

/// One node kind or module: what the engine, the validator, the editor and the agents all read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Descriptor {
    pub name: Cow<'static, str>,
    #[serde(default = "one")]
    pub version: Cow<'static, str>,
    #[serde(default = "default_domain")]
    pub domain: Domain,
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub doc: Cow<'static, str>,
    /// the stage's cargo feature that carries the kind (`core` = always); empty = none
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub feature: Cow<'static, str>,
    /// deck-core's module for the kind (control · synth · analysis · stage · 3d …); empty = none
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub module: Cow<'static, str>,
    #[serde(default)]
    pub scope: Scope,
    /// a scene owns it once: never inside an instrument
    #[serde(default, skip_serializing_if = "is_false")]
    pub scene_only: bool,
    /// the batch evaluator (deck-core batch.rs, live-core, the browser) can run it
    #[serde(default, skip_serializing_if = "is_false")]
    pub batch: bool,
    /// capabilities the kind needs (#312) – reserved
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub needs: Cow<'static, [Cow<'static, str>]>,
    /// modules only: the ABI version they speak
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<u32>,
    #[serde(default = "empty_ports")]
    pub ports: Cow<'static, [PortDesc]>,
    #[serde(default = "empty_params")]
    pub params: Cow<'static, [ParamDesc]>,
}

impl Descriptor {
    pub fn inputs(&self) -> impl Iterator<Item = &PortDesc> {
        self.ports.iter().filter(|p| p.dir == Dir::In)
    }
    pub fn outputs(&self) -> impl Iterator<Item = &PortDesc> {
        self.ports.iter().filter(|p| p.dir == Dir::Out)
    }
    pub fn param(&self, name: &str) -> Option<&ParamDesc> {
        self.params.iter().find(|p| p.name == name)
    }
}

impl ParamDesc {
    /// value → 0..1 (clamped)
    pub fn norm(&self, v: f32) -> f32 {
        let n = match self.curve {
            Curve::Lin => (v - self.min) / (self.max - self.min),
            Curve::Log => (v.max(self.min).ln() - self.min.ln()) / (self.max.ln() - self.min.ln()),
        };
        if n.is_finite() {
            n.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// 0..1 → value
    pub fn value(&self, n: f32) -> f32 {
        let n = n.clamp(0.0, 1.0);
        match self.curve {
            Curve::Lin => self.min + n * (self.max - self.min),
            Curve::Log => (self.min.ln() + n * (self.max.ln() - self.min.ln())).exp(),
        }
    }
}

/// a built-in kind by its scene name
pub fn builtin(name: &str) -> Option<&'static Descriptor> {
    builtin_kinds().iter().find(|d| d.name == name)
}

/// The catalog as a document: `kinds.json`. The API merges the published modules' descriptors into `kinds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Catalog {
    pub catalog_version: u32,
    pub kinds: Vec<Descriptor>,
}

pub fn catalog() -> Catalog {
    Catalog { catalog_version: CATALOG_VERSION, kinds: builtin_kinds().to_vec() }
}

/// a built-in's name or port name as a `'static str` (the table borrows them)
#[allow(clippy::ptr_arg)] // the Cow itself must be 'static to hand out a 'static str
pub fn static_str(s: &'static Cow<'static, str>) -> &'static str {
    match s {
        Cow::Borrowed(b) => b,
        Cow::Owned(o) => o.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_ports_and_params_are_sane() {
        let mut seen = std::collections::BTreeSet::new();
        for d in builtin_kinds() {
            assert!(seen.insert(d.name.as_ref()), "kind {} twice", d.name);
            let mut ports = std::collections::BTreeSet::new();
            for p in d.ports.iter() {
                assert!(ports.insert((p.dir as u8, p.name.as_ref())), "{}: port {} twice", d.name, p.name);
            }
            for p in d.params.iter() {
                if p.choices.is_empty() {
                    assert!(p.min < p.max, "{}.{}: min < max", d.name, p.name);
                    if let Some(v) = p.default {
                        assert!((p.min..=p.max).contains(&v), "{}.{}: default inside the range", d.name, p.name);
                    }
                    if p.curve == Curve::Log {
                        assert!(p.min > 0.0, "{}.{}: log needs min > 0", d.name, p.name);
                    }
                }
                if p.port {
                    assert!(d.inputs().any(|q| q.name == p.name && q.ty == PortType::Control), "{}.{}: no such control input", d.name, p.name);
                }
            }
            for p in d.inputs().filter(|p| p.ty == PortType::Control) {
                assert!(d.param(&p.name).is_some_and(|q| q.port), "{}: control input {} has no parameter", d.name, p.name);
            }
        }
    }

    #[test]
    fn the_catalog_round_trips_and_a_v1_module_descriptor_parses() {
        let c = catalog();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Catalog>(&json).unwrap(), c);
        let v1 = r#"{"abi":1,"name":"gain","version":"0.1.0","params":[{"name":"amount","min":0.0,"max":2.0,"default":1.0}],
            "ports":[{"name":"in","dir":"in","kind":"audio"},{"name":"out","dir":"out","kind":"audio"}]}"#;
        let d: Descriptor = serde_json::from_str(v1).unwrap();
        assert_eq!((d.abi, d.ports.len(), d.params[0].default), (Some(1), 2, Some(1.0)));
        assert_eq!(d.ports[0].ty, PortType::Audio);
    }
}
