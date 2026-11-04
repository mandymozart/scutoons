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

/// A module: its kinds and how to build one. Implemented by the built-in modules (below), by deck/native's GPU module,
/// and – in the lab – by a WASM module loaded at runtime (deck/lab/wasm-host).
pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;
    fn kinds(&self) -> &'static [KindInfo];
    fn build(&self, kind: &str, params: &Params, rate: f32) -> Option<Box<dyn Block>>;
}

/// Every kind the platform knows, compiled in or not, grouped by module (control, synth, analysis, stage, the planned
/// 3d / text / video) – derived from the catalog (deck-catalog, #315), the one table. Inside the engine modules
/// (control, synth, analysis) a kind lists only the ports this control graph takes (`PortDesc::engine`); the stage's
/// and the planned kinds list all of them. The scene check and `features_for` read it.
fn table() -> &'static Vec<(&'static str, Vec<KindInfo>)> {
    use deck_catalog::{static_str, PortType as T};
    static TABLE: std::sync::OnceLock<Vec<(&'static str, Vec<KindInfo>)>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let ty = |t: T| match t {
            T::Audio | T::Cv => PortType::Audio,
            T::Control | T::Event => PortType::Control,
            T::Video => PortType::Video,
        };
        let ports = |d: &'static deck_catalog::Descriptor, dir: deck_catalog::Dir| -> &'static [Port] {
            let engine = matches!(d.module.as_ref(), "control" | "synth" | "analysis");
            let list: Vec<Port> = d.ports.iter().filter(|p| p.dir == dir && (p.engine || !engine)).map(|p| Port { name: static_str(&p.name), ty: ty(p.ty) }).collect();
            Box::leak(list.into_boxed_slice())
        };
        let mut groups: Vec<(&'static str, Vec<KindInfo>)> = Vec::new();
        for d in deck_catalog::builtin_kinds().iter().filter(|d| !d.module.is_empty()) {
            let module = static_str(&d.module);
            let info = KindInfo { name: static_str(&d.name), module, inputs: ports(d, deck_catalog::Dir::In), outputs: ports(d, deck_catalog::Dir::Out) };
            match groups.iter_mut().find(|(m, _)| *m == module) {
                Some((_, v)) => v.push(info),
                None => groups.push((module, vec![info])),
            }
        }
        groups
    })
}

/// the kinds of one module (control · synth · analysis · stage · 3d …), empty for an unknown one
pub fn kinds_of_module(module: &str) -> &'static [KindInfo] {
    table().iter().find(|(m, _)| *m == module).map_or(&[], |(_, v)| v.as_slice())
}

/// All kinds, in module order.
pub fn all_kinds() -> impl Iterator<Item = &'static KindInfo> {
    table().iter().flat_map(|(_, v)| v.iter())
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

    #[cfg(all(feature = "control", feature = "analysis"))]
    #[test]
    fn the_default_build_carries_two_modules() {
        let r = Registry::compiled();
        assert_eq!(r.names(), ["control", "analysis"]);
        assert_eq!(r.find("lfo").unwrap().1.module, "control");
        // every compiled kind is in the platform table with the same module
        for name in r.names() {
            assert!(all_kinds().any(|k| k.module == name));
        }
    }
}
