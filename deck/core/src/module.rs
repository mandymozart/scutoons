//! Modules and the registry. A module is a set of node kinds (ports + a constructor); the registry holds the modules a
//! build carries. A scene that uses a kind whose module is not in the build gets a clear error ("needs module X") –
//! never a crash – and the full kind table (every module, compiled in or not) says which feature to turn on.
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    Video,
    Audio,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Port {
    pub name: &'static str,
    pub ty: PortType,
}

pub const fn port(name: &'static str, ty: PortType) -> Port {
    Port { name, ty }
}

/// One node kind: its name in the scene, the module (= cargo feature) that implements it, its ports.
#[derive(Debug, Clone, Copy)]
pub struct KindInfo {
    pub name: &'static str,
    pub module: &'static str,
    pub inputs: &'static [Port],
    pub outputs: &'static [Port],
}

impl KindInfo {
    pub fn input(&self, name: &str) -> Option<usize> {
        self.inputs.iter().position(|p| p.name == name)
    }
    pub fn output(&self, name: &str) -> Option<usize> {
        self.outputs.iter().position(|p| p.name == name)
    }
}

/// A node's params from the scene (`[[node]]` keys besides id / kind).
pub type Params = toml::Table;

pub fn param(params: &Params, key: &str, default: f32) -> f32 {
    match params.get(key) {
        Some(toml::Value::Float(f)) => *f as f32,
        Some(toml::Value::Integer(i)) => *i as f32,
        _ => default,
    }
}

/// A running node. Control blocks map control inputs to control outputs once per frame; audio blocks fill a mono
/// block with their control inputs as parameters. One trait for both keeps a module's table small.
pub trait Block: Send {
    fn control(&mut self, _inputs: &[f32], _outputs: &mut [f32], _dt: f32) {}
    fn audio(&mut self, _inputs: &[f32], _out: &mut [f32]) {}
}

/// A module: its kinds and how to build one. Implemented by the built-in modules (below), by deck/stage's GPU module,
/// and – in the lab – by a WASM module loaded at runtime (deck/lab/wasm-host).
pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;
    fn kinds(&self) -> &'static [KindInfo];
    fn build(&self, kind: &str, params: &Params, rate: f32) -> Option<Box<dyn Block>>;
}

/// Every kind the platform knows, compiled in or not (one table per module) – the scene check and `features_for` read it. A new kind = a line
/// here + its block in its module. The `stage` kinds are deck/stage's (GPU, native only); `3d`, `text`, `video` are
/// the planned modules of CYBERDECKS.md §8 (S4, S5) and the owner's list (#182).
pub const CONTROL_KINDS: &[KindInfo] = &[
    kind("lfo", "control", &[port("rate", PortType::Control), port("depth", PortType::Control)], &[port("out", PortType::Control)]),
    kind("add", "control", &[port("a", PortType::Control), port("b", PortType::Control)], &[port("out", PortType::Control)]),
    kind("mul", "control", &[port("a", PortType::Control), port("b", PortType::Control)], &[port("out", PortType::Control)]),
    kind("smooth", "control", &[port("in", PortType::Control), port("time", PortType::Control)], &[port("out", PortType::Control)]),
];
pub const SYNTH_KINDS: &[KindInfo] = &[
    kind("osc", "synth", &[port("freq", PortType::Control), port("gain", PortType::Control)], &[port("out", PortType::Audio)]),
    kind(
        "fm",
        "synth",
        &[port("freq", PortType::Control), port("ratio", PortType::Control), port("index", PortType::Control), port("gain", PortType::Control)],
        &[port("out", PortType::Audio)],
    ),
];
pub const ANALYSIS_KINDS: &[KindInfo] = &[
    kind(
        "fft",
        "analysis",
        &[port("in", PortType::Audio)],
        &[port("level", PortType::Control), port("bass", PortType::Control), port("mid", PortType::Control), port("high", PortType::Control)],
    ),
];
/// deck/stage's GPU kinds – listed so a scene check outside the stage can name them.
pub const STAGE_KINDS: &[KindInfo] = &[
    kind("screen", "stage", &[port("in", PortType::Video)], &[]),
    kind("shader", "stage", &[port("in", PortType::Video)], &[port("out", PortType::Video)]),
    kind("chapter", "stage", &[port("in", PortType::Video)], &[port("out", PortType::Video), port("text", PortType::Video)]),
    kind("test_pattern", "stage", &[port("speed", PortType::Control)], &[port("out", PortType::Video)]),
    kind("overlay", "stage", &[port("in", PortType::Video), port("top", PortType::Video)], &[port("out", PortType::Video)]),
    kind("noise", "stage", &[port("cutoff", PortType::Control)], &[port("out", PortType::Audio)]),
    kind("seq", "stage", &[port("rate", PortType::Control)], &[port("cv", PortType::Audio)]),
    kind("chapter_music", "stage", &[port("gain", PortType::Control)], &[port("out", PortType::Audio)]),
    kind("mixer", "stage", &[port("in1", PortType::Audio)], &[port("out", PortType::Audio)]),
    kind("audio_out", "stage", &[port("in", PortType::Audio)], &[]),
];
pub const PLANNED_KINDS: &[KindInfo] = &[
    kind("model3d", "3d", &[port("rot", PortType::Control)], &[port("out", PortType::Video)]),
    kind("text", "text", &[port("in", PortType::Video)], &[port("out", PortType::Video)]),
    kind("video", "video", &[port("speed", PortType::Control)], &[port("out", PortType::Video)]),
];

/// All kind tables, in module order.
pub const KIND_TABLES: &[&[KindInfo]] = &[CONTROL_KINDS, SYNTH_KINDS, ANALYSIS_KINDS, STAGE_KINDS, PLANNED_KINDS];

pub fn all_kinds() -> impl Iterator<Item = &'static KindInfo> {
    KIND_TABLES.iter().flat_map(|t| t.iter())
}

const fn kind(name: &'static str, module: &'static str, inputs: &'static [Port], outputs: &'static [Port]) -> KindInfo {
    KindInfo { name, module, inputs, outputs }
}

pub fn kind_info(name: &str) -> Option<&'static KindInfo> {
    all_kinds().find(|k| k.name == name)
}

/// The modules (= cargo features) a scene's kinds need, sorted; unknown kinds are skipped (the graph check names them).
pub fn features_for<'a>(kinds: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let set: BTreeSet<&'static str> = kinds.into_iter().filter_map(kind_info).map(|k| k.module).collect();
    set.into_iter().collect()
}

/// The modules this build carries.
pub struct Registry {
    modules: Vec<Box<dyn Module>>,
}

impl Registry {
    pub fn empty() -> Self {
        Registry { modules: Vec::new() }
    }

    /// Every module compiled into this build (the cargo features).
    pub fn compiled() -> Self {
        #[allow(unused_mut)]
        let mut r = Registry::empty();
        #[cfg(feature = "control")]
        r.add(Box::new(crate::control::ControlModule));
        #[cfg(feature = "synth")]
        r.add(Box::new(crate::synth::SynthModule));
        #[cfg(feature = "analysis")]
        r.add(Box::new(crate::analysis::AnalysisModule));
        r
    }

    /// A module loaded at runtime (a plugin, a WASM module) joins like a compiled one.
    pub fn add(&mut self, m: Box<dyn Module>) {
        self.modules.push(m);
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.name()).collect()
    }

    /// The kind's info from the module that carries it, or why not: unknown kind / module not in this build.
    pub fn find(&self, kind: &str) -> Result<(&dyn Module, &'static KindInfo), String> {
        for m in &self.modules {
            if let Some(k) = m.kinds().iter().find(|k| k.name == kind) {
                return Ok((m.as_ref(), k));
            }
        }
        match kind_info(kind) {
            Some(k) => Err(format!("kind '{kind}' needs module '{}', which this build does not carry", k.module)),
            None => Err(format!("unknown kind '{kind}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_of_a_scene() {
        assert_eq!(features_for(["lfo", "fft", "osc", "lfo", "screen", "nope"]), ["analysis", "control", "stage", "synth"]);
        assert_eq!(features_for(["model3d"]), ["3d"]);
    }

    #[test]
    fn kinds_are_unique() {
        let names: BTreeSet<&str> = all_kinds().map(|k| k.name).collect();
        assert_eq!(names.len(), all_kinds().count());
    }

    #[test]
    fn missing_modules_are_named() {
        let r = Registry::empty();
        assert_eq!(r.find("model3d").err().unwrap(), "kind 'model3d' needs module '3d', which this build does not carry");
        assert_eq!(r.find("teleport").err().unwrap(), "unknown kind 'teleport'");
    }

    #[cfg(all(feature = "control", feature = "synth", feature = "analysis"))]
    #[test]
    fn the_default_build_carries_three_modules() {
        let r = Registry::compiled();
        assert_eq!(r.names(), ["control", "synth", "analysis"]);
        assert_eq!(r.find("fm").unwrap().1.module, "synth");
        // every compiled kind is in the platform table with the same module
        for name in r.names() {
            assert!(all_kinds().any(|k| k.module == name));
        }
    }
}
