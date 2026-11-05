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
use std::collections::BTreeMap;

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
fn event_capacity() -> u16 {
    EVENT_CAPACITY
}
fn is_event_capacity(n: &u16) -> bool {
    *n == EVENT_CAPACITY
}
fn is_zero(n: &u32) -> bool {
    *n == 0
}
fn empty_assets() -> Cow<'static, [AssetDesc]> {
    Cow::Borrowed(&[])
}

/// an event port's prop slots: deck-events' `Event` carries at most this many props (ABI-V2 §1.2, §1.4)
pub const MAX_EVENT_PROPS: usize = 6;
/// an event port's records per block when the descriptor does not say (`capacity`)
pub const EVENT_CAPACITY: u16 = 64;
/// the most records per block an event port may declare
pub const MAX_EVENT_CAPACITY: u16 = 512;

/// how a control output with a `path` acts on its target (ABI-V2 §1.2, §5): replace it, add to it, scale it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Set,
    Add,
    Mul,
}

fn is_set(m: &Mode) -> bool {
    *m == Mode::Set
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
    /// audio inputs: a second input fed by sends, not by the graph's sum (ENGINE-REFACTOR-PLAN §2.6)
    #[serde(default, skip_serializing_if = "is_false")]
    pub sidechain: bool,
    /// event ports: the deck-events names it emits / accepts (`music.note`, `story.act` …); interned once at load
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub names: Cow<'static, [Cow<'static, str>]>,
    /// event ports: the positional meaning of the prop slots (at most `MAX_EVENT_PROPS`)
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub props: Cow<'static, [Cow<'static, str>]>,
    /// event ports: records per block (1 … `MAX_EVENT_CAPACITY`)
    #[serde(default = "event_capacity", skip_serializing_if = "is_event_capacity")]
    pub capacity: u16,
    /// control outputs: the target it writes (a group child's parameter, preset, gain …, ABI-V2 §5); empty = a graph cable
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub path: Cow<'static, str>,
    /// control outputs with a `path`: how the value acts on the target
    #[serde(default, skip_serializing_if = "is_set")]
    pub mode: Mode,
    /// control outputs: the value is an index into these names (an enum)
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub choices: Cow<'static, [Cow<'static, str>]>,
}

/// what an asset slot holds once the host has filled it: planar f32 PCM, raw bytes, or a small RGBA8 image (ABI-V2 §1.5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Pcm,
    Bytes,
    Image,
}

/// a read-only data slot of a module; a node binds an asset id to it, the host fills it at instantiation (ABI-V2 §1.5)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssetDesc {
    pub name: Cow<'static, str>,
    pub kind: AssetKind,
    #[serde(default = "empty_str", skip_serializing_if = "is_empty_str")]
    pub doc: Cow<'static, str>,
    /// the largest asset the slot takes, in bytes (decoded)
    pub max_bytes: u32,
}

/// a module's one factory preset (ABI-V2 §2); every other preset is a DB document (deck-space's rig `Preset`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FactoryPreset {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    /// parameter name → value; a parameter not named keeps its default
    #[serde(default)]
    pub values: BTreeMap<String, f32>,
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
    /// the kind follows the transport by default (#359: stops, holds, resumes with it); a node's `follow_transport` overrides it
    #[serde(default, skip_serializing_if = "is_false")]
    pub follow_transport: bool,
    /// capabilities the kind needs (#312): without one it does not run
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub needs: Cow<'static, [Cow<'static, str>]>,
    /// capabilities the kind uses when they are there (#312): without one it runs on, the sensor reads "absent"
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub wants: Cow<'static, [Cow<'static, str>]>,
    /// the show's variable slots the kind may read (`story.act` …, ABI-V2 §3)
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub reads: Cow<'static, [Cow<'static, str>]>,
    /// modules only (A1, #409): the moods the module knows, in its own order – the deck hands a module the mood's place
    /// here + 1 as `mood_index` (0 = a mood not in the list; deck-space context.rs)
    #[serde(default = "empty_strs", skip_serializing_if = "is_empty_slice")]
    pub moods: Cow<'static, [Cow<'static, str>]>,
    /// modules only: the ABI version they speak
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<u32>,
    /// constant latency in frames, for the engine's compensation (ENGINE-REFACTOR-PLAN §2.6a)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub latency: u32,
    #[serde(default = "empty_ports")]
    pub ports: Cow<'static, [PortDesc]>,
    #[serde(default = "empty_params")]
    pub params: Cow<'static, [ParamDesc]>,
    /// read-only data slots (samples, sequences, images) the host fills at instantiation
    #[serde(default = "empty_assets", skip_serializing_if = "is_empty_slice")]
    pub assets: Cow<'static, [AssetDesc]>,
    /// the kind's one factory preset
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FactoryPreset>,
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
    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.params.iter().position(|p| p.name == name)
    }
    /// the ports of one direction and type, in declaration order – for a module the n-th is `block.audio_in(n)` and so on
    pub fn ports_of(&self, dir: Dir, ty: PortType) -> impl Iterator<Item = &PortDesc> {
        self.ports.iter().filter(move |p| p.dir == dir && p.ty == ty)
    }

    /// A module's ports as the node graph numbers them (#342) – the one rule the module host binds by
    /// (deck-module-host's `Layout`) and the graph registers by (deck-space's `ModuleDef::of`): inputs = the audio inputs,
    /// then the control inputs (each in declaration order), then the parameters (a parameter is also a control input);
    /// outputs = the audio and control outputs in declaration order; a control output with a `path` is a target too.
    /// An audio port carries its declared channels (a `cv` port and an undeclared one two). Event ports are no graph
    /// ports (the event host routes them by name); video ports are skipped (no module host plays one yet).
    pub fn graph(&self) -> GraphPorts {
        let channels = |p: &PortDesc| match (p.ty, p.channels) {
            (PortType::Cv, _) | (_, None) => Some(Channels::Count(2)),
            (_, c) => c,
        };
        let port = |p: &PortDesc, audio: bool| GraphPort { name: p.name.to_string(), audio, channels: if audio { channels(p) } else { None } };
        let mut g = GraphPorts { inputs: Vec::new(), outputs: Vec::new(), targets: Vec::new() };
        let mut control_in = Vec::new();
        for p in self.ports.iter() {
            match (p.dir, p.ty) {
                (Dir::In, PortType::Audio | PortType::Cv) => g.inputs.push(port(p, true)),
                (Dir::In, PortType::Control) => control_in.push(port(p, false)),
                (Dir::Out, PortType::Audio | PortType::Cv) => g.outputs.push(port(p, true)),
                (Dir::Out, PortType::Control) => {
                    if !p.path.is_empty() {
                        g.targets.push((g.outputs.len(), p.path.to_string(), p.mode));
                    }
                    g.outputs.push(port(p, false));
                }
                _ => {}
            }
        }
        g.inputs.extend(control_in);
        g.inputs.extend(self.params.iter().map(|p| GraphPort { name: p.name.to_string(), audio: false, channels: None }));
        g
    }
}

/// One graph port of a module: its name, whether it carries audio (else a control value) and – an audio port – its
/// channels (None: a control port).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPort {
    pub name: String,
    pub audio: bool,
    pub channels: Option<Channels>,
}

/// A module's graph ports (`Descriptor::graph`): inputs, outputs, and (graph output, path, mode) per control output
/// bound to a target (ABI-V2 §5).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPorts {
    pub inputs: Vec<GraphPort>,
    pub outputs: Vec<GraphPort>,
    pub targets: Vec<(usize, String, Mode)>,
}

impl Descriptor {

    /// The ABI v2 fields against their own rules (ABI-V2 §1.2): each port field only on the port it belongs to, the
    /// event limits, no capability both needed and wanted, unique slots, a preset that names real parameters inside
    /// their ranges. Built-ins and modules alike; a module host adds its own checks (`deck_module::host::validate`).
    pub fn check(&self) -> Result<(), String> {
        let me = &self.name;
        let unique = |what: &str, list: &mut dyn Iterator<Item = &str>| -> Result<(), String> {
            let mut seen = std::collections::BTreeSet::new();
            for s in list {
                if s.is_empty() {
                    return Err(format!("'{me}': an empty name in {what}"));
                }
                if !seen.insert(s) {
                    return Err(format!("'{me}': '{s}' is in {what} twice"));
                }
            }
            Ok(())
        };
        for p in self.ports.iter() {
            let at = format!("'{me}': port '{}'", p.name);
            let event = p.ty == PortType::Event;
            let control_out = p.ty == PortType::Control && p.dir == Dir::Out;
            if p.sidechain && !(p.ty == PortType::Audio && p.dir == Dir::In) {
                return Err(format!("{at}: `sidechain` is for audio inputs"));
            }
            if !event && (!p.names.is_empty() || !p.props.is_empty() || p.capacity != EVENT_CAPACITY) {
                return Err(format!("{at}: `names`, `props` and `capacity` are for event ports"));
            }
            if !control_out && (!p.path.is_empty() || p.mode != Mode::Set || !p.choices.is_empty()) {
                return Err(format!("{at}: `path`, `mode` and `choices` are for control outputs"));
            }
            if p.mode != Mode::Set && p.path.is_empty() {
                return Err(format!("{at}: `mode` acts on a target – it needs a `path`"));
            }
            if p.props.len() > MAX_EVENT_PROPS {
                return Err(format!("{at}: {} props, at most {MAX_EVENT_PROPS}", p.props.len()));
            }
            if !(1..=MAX_EVENT_CAPACITY).contains(&p.capacity) {
                return Err(format!("{at}: capacity {}, 1 … {MAX_EVENT_CAPACITY}", p.capacity));
            }
            unique(&format!("{at}'s names"), &mut p.names.iter().map(|s| s.as_ref()))?;
            unique(&format!("{at}'s props"), &mut p.props.iter().map(|s| s.as_ref()))?;
            unique(&format!("{at}'s choices"), &mut p.choices.iter().map(|s| s.as_ref()))?;
        }
        unique("`needs`", &mut self.needs.iter().map(|s| s.as_ref()))?;
        unique("`wants`", &mut self.wants.iter().map(|s| s.as_ref()))?;
        if let Some(w) = self.wants.iter().find(|w| self.needs.contains(w)) {
            return Err(format!("'{me}': '{w}' is in `needs` and in `wants` – a capability is one or the other"));
        }
        unique("`reads`", &mut self.reads.iter().map(|s| s.as_ref()))?;
        unique("`moods`", &mut self.moods.iter().map(|s| s.as_ref()))?;
        unique("`assets`", &mut self.assets.iter().map(|a| a.name.as_ref()))?;
        if let Some(a) = self.assets.iter().find(|a| a.max_bytes == 0) {
            return Err(format!("'{me}': asset '{}' needs a `max_bytes` above 0", a.name));
        }
        if let Some(preset) = &self.preset {
            if preset.name.is_empty() {
                return Err(format!("'{me}': the preset needs a name"));
            }
            for (k, &v) in &preset.values {
                let p = self.param(k).ok_or_else(|| format!("'{me}': preset '{}' sets '{k}', which is no parameter", preset.name))?;
                if !p.choices.is_empty() {
                    if !(v >= 0.0 && v.fract() == 0.0 && (v as usize) < p.choices.len()) {
                        return Err(format!("'{me}': preset '{}': '{k}' = {v} is no index into its choices", preset.name));
                    }
                } else if !(v.is_finite() && (p.min..=p.max).contains(&v)) {
                    return Err(format!("'{me}': preset '{}': '{k}' = {v} is outside {} … {}", preset.name, p.min, p.max));
                }
            }
        }
        Ok(())
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
            d.check().unwrap();
        }
    }

    /// every ABI v2 field (ABI-V2 §1.2) filled, through JSON and back; then each misplaced field refused by `check`
    #[test]
    fn the_v2_fields_round_trip_and_check() {
        let json = r#"{"name":"m","abi":1,"latency":64,"needs":["motion"],"wants":["position"],"reads":["story.act"],
            "moods":["thinking","sad"],"params":[{"name":"cutoff","min":20,"max":20000,"default":800},{"name":"wave","choices":["sine","saw"]}],
            "ports":[{"name":"side","dir":"in","type":"audio","channels":2,"sidechain":true},
                     {"name":"notes","dir":"in","type":"event","names":["music.note"],"props":["pitch","vel","glide"],"capacity":128},
                     {"name":"sel","dir":"out","type":"control","path":"child.preset","mode":"add","choices":["calm","wild"]}],
            "assets":[{"name":"kick","kind":"pcm","doc":"the kick","max_bytes":8388608}],
            "preset":{"name":"warm","doc":"a soft start","values":{"cutoff":1200,"wave":1}}}"#;
        let d: Descriptor = serde_json::from_str(json).unwrap();
        d.check().unwrap();
        assert_eq!(serde_json::from_str::<Descriptor>(&serde_json::to_string(&d).unwrap()).unwrap(), d);
        assert_eq!((d.latency, d.wants[0].as_ref(), d.reads[0].as_ref(), d.assets[0].kind), (64, "position", "story.act", AssetKind::Pcm));
        assert_eq!(d.moods.iter().map(|m| m.as_ref()).collect::<Vec<_>>(), ["thinking", "sad"]);
        assert_eq!((d.ports[0].sidechain, d.ports[1].capacity, d.ports[2].mode), (true, 128, Mode::Add));
        assert_eq!(d.preset.as_ref().unwrap().values["cutoff"], 1200.0);
        // a port that sets none of them serializes without them
        let plain = serde_json::to_string(&builtin_kinds()[0].ports[0]).unwrap();
        for k in ["sidechain", "names", "props", "capacity", "path", "mode", "choices"] {
            assert!(!plain.contains(k), "{plain}");
        }

        let bad = |f: &dyn Fn(&mut Descriptor)| {
            let mut x = d.clone();
            f(&mut x);
            x.check().unwrap_err()
        };
        fn port(x: &mut Descriptor, i: usize) -> &mut PortDesc {
            &mut x.ports.to_mut()[i]
        }
        assert!(bad(&|x| port(x, 2).sidechain = true).contains("sidechain"));
        assert!(bad(&|x| port(x, 0).capacity = 8).contains("event ports"));
        assert!(bad(&|x| port(x, 1).path = "a.b".into()).contains("control outputs"));
        assert!(bad(&|x| port(x, 1).capacity = 0).contains("capacity"));
        assert!(bad(&|x| port(x, 1).capacity = MAX_EVENT_CAPACITY + 1).contains("capacity"));
        assert!(bad(&|x| port(x, 1).props = (0..7).map(|i| Cow::Owned(format!("p{i}"))).collect()).contains("at most 6"));
        assert!(bad(&|x| port(x, 1).names.to_mut().push("music.note".into())).contains("twice"));
        assert!(bad(&|x| port(x, 2).path = "".into()).contains("needs a `path`"));
        assert!(bad(&|x| x.wants = Cow::Owned(vec!["motion".into()])).contains("needs` and in `wants"));
        assert!(bad(&|x| x.assets.to_mut()[0].max_bytes = 0).contains("max_bytes"));
        assert!(bad(&|x| x.moods.to_mut().push("sad".into())).contains("`moods` twice"));
        assert!(bad(&|x| x.moods.to_mut().push("".into())).contains("empty name in `moods`"));
        assert!(bad(&|x| _ = x.preset.as_mut().unwrap().values.insert("nope".into(), 1.0)).contains("no parameter"));
        assert!(bad(&|x| _ = x.preset.as_mut().unwrap().values.insert("cutoff".into(), 1.0)).contains("outside"));
        assert!(bad(&|x| _ = x.preset.as_mut().unwrap().values.insert("wave".into(), 2.0)).contains("choices"));
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
