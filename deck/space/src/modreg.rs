//! Module kinds (#280, research/RIG-EDITING.md §3): node kinds that come from a library of compiled modules instead of
//! graph.rs's table. A module registers its ports once (deck-native's wasm host reads them from the module's descriptor);
//! a scene then names it `module:<name>` and the graph checks its links like any kind's. The registry is process-wide
//! and append-only: a kind is an index, so a `Kind` stays valid; registering the same name with other ports (a rebuilt
//! module) adds a new kind and `lookup` finds the newest. Specs are leaked – a handful of tiny tables per module.
use crate::graph::{Port, PortType};
use std::sync::RwLock;

/// the prefix of a module's kind name in a scene
pub const PREFIX: &str = "module:";

/// A module's ports as the graph sees them: the control inputs (parameters, control ports), the audio inputs, the
/// outputs – in the order the host's layout binds them.
#[derive(Debug)]
pub struct ModuleSpec {
    /// `module:<name>`
    pub kind: &'static str,
    pub inputs: &'static [Port],
    pub outputs: &'static [Port],
    /// a control input's default (its name, the value) – the parameters' defaults
    pub defaults: &'static [(&'static str, f32)],
}

/// A registered module kind (an index into the registry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleKind(u32);

static REGISTRY: RwLock<Vec<&'static ModuleSpec>> = RwLock::new(Vec::new());

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn ports(list: &[(String, PortType)]) -> &'static [Port] {
    Box::leak(list.iter().map(|(name, ty)| Port { name: leak(name), ty: *ty, channels: None }).collect::<Vec<_>>().into_boxed_slice())
}

/// Registers `module:<name>` with these ports (and the defaults of its control inputs). The same name with the same
/// ports is the kind it already is.
pub fn register(name: &str, inputs: &[(String, PortType)], outputs: &[(String, PortType)], defaults: &[(String, f32)]) -> ModuleKind {
    let kind = format!("{PREFIX}{name}");
    let mut reg = REGISTRY.write().unwrap_or_else(|e| e.into_inner());
    let same = |s: &ModuleSpec| {
        s.kind == kind
            && s.inputs.len() == inputs.len()
            && s.outputs.len() == outputs.len()
            && s.inputs.iter().zip(inputs).all(|(p, (n, t))| p.name == n && p.ty == *t)
            && s.outputs.iter().zip(outputs).all(|(p, (n, t))| p.name == n && p.ty == *t)
            && s.defaults.len() == defaults.len()
            && s.defaults.iter().zip(defaults).all(|(a, b)| a.0 == b.0 && a.1.to_bits() == b.1.to_bits())
    };
    if let Some(i) = reg.iter().rposition(|s| same(s)) {
        return ModuleKind(i as u32);
    }
    let defaults: Vec<(&'static str, f32)> = defaults.iter().map(|(n, v)| (leak(n), *v)).collect();
    let spec: &'static ModuleSpec = Box::leak(Box::new(ModuleSpec {
        kind: leak(&kind),
        inputs: ports(inputs),
        outputs: ports(outputs),
        defaults: Box::leak(defaults.into_boxed_slice()),
    }));
    reg.push(spec);
    ModuleKind(reg.len() as u32 - 1)
}

/// the newest registered kind of this scene name (`module:walk`)
pub fn lookup(kind: &str) -> Option<ModuleKind> {
    let reg = REGISTRY.read().unwrap_or_else(|e| e.into_inner());
    reg.iter().rposition(|s| s.kind == kind).map(|i| ModuleKind(i as u32))
}

impl ModuleKind {
    pub fn spec(self) -> &'static ModuleSpec {
        REGISTRY.read().unwrap_or_else(|e| e.into_inner())[self.0 as usize]
    }

    /// the default of control input `port` (0 when the spec names none)
    pub fn default_for(self, port: &str) -> f32 {
        self.spec().defaults.iter().find(|(n, _)| *n == port).map_or(0.0, |&(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Kind};
    use crate::scene::Scene;

    fn pt(list: &[(&str, PortType)]) -> Vec<(String, PortType)> {
        list.iter().map(|&(n, t)| (n.to_owned(), t)).collect()
    }

    #[test]
    fn a_module_kind_is_a_graph_kind() {
        let m = register("regtest_voice", &pt(&[("spread", PortType::Control)]), &pt(&[("cv", PortType::Audio)]), &[("spread".into(), 0.25)]);
        assert_eq!(lookup("module:regtest_voice"), Some(m));
        assert_eq!(Kind::parse("module:regtest_voice"), Some(Kind::Module(m)));
        assert_eq!(Kind::Module(m).name(), "module:regtest_voice");
        assert_eq!(m.default_for("spread"), 0.25);
        let g = Graph::build(
            &Scene::parse(
                "[[node]]\nid=\"w\"\nkind=\"module:regtest_voice\"\n[[node]]\nid=\"l\"\nkind=\"lfo\"\n\
                 [[node]]\nid=\"o\"\nkind=\"osc\"\n[[link]]\nfrom=\"l\"\nto=\"w.spread\"\n[[link]]\nfrom=\"w.cv\"\nto=\"o.cv\"\n",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(g.nodes[0].kind, Kind::Module(m));
        // a type mismatch against a module's ports is the graph's ordinary error
        let e = Graph::build(&Scene::parse("[[node]]\nid=\"w\"\nkind=\"module:regtest_voice\"\n[[node]]\nid=\"s\"\nkind=\"screen\"\n[[link]]\nfrom=\"w.cv\"\nto=\"s.in\"\n").unwrap()).unwrap_err();
        assert!(e.to_string().contains("cannot feed"), "{e}");
        assert!(Graph::build(&Scene::parse("[[node]]\nid=\"w\"\nkind=\"module:nobody\"\n").unwrap()).unwrap_err().to_string().contains("unknown kind"));
    }

    #[test]
    fn the_same_ports_are_the_same_kind_other_ports_a_newer_one() {
        let a = register("regtest_twice", &pt(&[("a", PortType::Control)]), &[], &[]);
        assert_eq!(register("regtest_twice", &pt(&[("a", PortType::Control)]), &[], &[]), a);
        let b = register("regtest_twice", &pt(&[("a", PortType::Control), ("b", PortType::Control)]), &[], &[]);
        assert_ne!(a, b);
        assert_eq!(lookup("module:regtest_twice"), Some(b), "lookup finds the newest");
        assert_eq!(a.spec().inputs.len(), 1, "an old Kind keeps its ports");
    }
}
