//! Module kinds (#280, research/RIG-EDITING.md §3): node kinds that come from a library of compiled modules instead of
//! graph.rs's table. A module registers its ports once (deck-native's wasm host reads them from the module's descriptor);
//! a scene then names it `module:<name>` and the graph checks its links like any kind's. The registry is process-wide
//! and append-only: a kind is an index, so a `Kind` stays valid; registering the same name with other ports (a rebuilt
//! module) adds a new kind and `lookup` finds the newest. Specs are leaked – a handful of tiny tables per module.
use crate::graph::{Domain, Port, PortType};
use deck_catalog::{Channels, Mode};
use std::sync::RwLock;

/// the prefix of a module's kind name in a scene
pub const PREFIX: &str = "module:";

/// A module's ports as the graph sees them: the inputs (audio ports, control ports, then the parameters), the outputs
/// (audio and control ports) – in the order the host's layout binds them (deck-module-host's `Layout`); its domain (an
/// ABI v2 module may be `control`: it runs in the control half); the control outputs that write a `path` target.
#[derive(Debug)]
pub struct ModuleSpec {
    /// `module:<name>`
    pub kind: &'static str,
    pub inputs: &'static [Port],
    pub outputs: &'static [Port],
    /// a control input's default (its name, the value) – the parameters' defaults
    pub defaults: &'static [(&'static str, f32)],
    pub domain: Domain,
    /// (output port, path, mode) per control output bound to a target instead of a cable (ABI-V2 §5; graph.rs binds them)
    pub targets: &'static [(usize, &'static str, Mode)],
}

/// One port of a module as the registry takes it: its name, type and – an audio port of ABI v2 – its channels (None: the
/// stereo pair of ABI v1).
#[derive(Debug, Clone, PartialEq)]
pub struct PortDef {
    pub name: String,
    pub ty: PortType,
    pub channels: Option<Channels>,
}

impl PortDef {
    pub fn new(name: &str, ty: PortType) -> PortDef {
        PortDef { name: name.to_owned(), ty, channels: None }
    }
}

/// What a module registers: its ports, the defaults of its control inputs, its domain, its path targets.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDef {
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub defaults: Vec<(String, f32)>,
    pub domain: Domain,
    /// (output port, path, mode)
    pub targets: Vec<(usize, String, Mode)>,
}

impl ModuleDef {
    /// an audio-domain module with these ports and no targets
    pub fn new(inputs: Vec<PortDef>, outputs: Vec<PortDef>, defaults: Vec<(String, f32)>) -> ModuleDef {
        ModuleDef { inputs, outputs, defaults, domain: Domain::Audio, targets: Vec::new() }
    }

    /// What a module's descriptor registers: its graph ports as the catalog numbers them (`Descriptor::graph`, the
    /// module host binds by the same), its parameters' defaults (else their minimum), its domain, its path targets – the
    /// one mapping from a module to the graph (the native library's loaded modules, a rig's policy in a validation).
    pub fn of(desc: &deck_catalog::Descriptor) -> ModuleDef {
        let g = desc.graph();
        let port = |p: &deck_catalog::GraphPort| PortDef { name: p.name.clone(), ty: if p.audio { PortType::Audio } else { PortType::Control }, channels: p.channels };
        ModuleDef {
            inputs: g.inputs.iter().map(port).collect(),
            outputs: g.outputs.iter().map(port).collect(),
            defaults: desc.params.iter().map(|p| (p.name.to_string(), p.default.unwrap_or(p.min))).collect(),
            domain: if desc.domain == deck_catalog::Domain::Control { Domain::Control } else { Domain::Audio },
            targets: g.targets,
        }
    }
}

/// A registered module kind (an index into the registry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleKind(u32);

static REGISTRY: RwLock<Vec<&'static ModuleSpec>> = RwLock::new(Vec::new());

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn ports(list: &[PortDef]) -> &'static [Port] {
    Box::leak(list.iter().map(|p| Port { name: leak(&p.name), ty: p.ty, channels: p.channels }).collect::<Vec<_>>().into_boxed_slice())
}

/// Registers `module:<name>` as `def` says. The same name with the same ports, defaults, domain and targets is the kind
/// it already is.
pub fn register(name: &str, def: &ModuleDef) -> ModuleKind {
    let kind = format!("{PREFIX}{name}");
    let mut reg = REGISTRY.write().unwrap_or_else(|e| e.into_inner());
    let same_ports = |a: &[Port], b: &[PortDef]| a.len() == b.len() && a.iter().zip(b).all(|(p, d)| p.name == d.name && p.ty == d.ty && p.channels == d.channels);
    let same = |s: &ModuleSpec| {
        s.kind == kind
            && same_ports(s.inputs, &def.inputs)
            && same_ports(s.outputs, &def.outputs)
            && s.defaults.len() == def.defaults.len()
            && s.defaults.iter().zip(&def.defaults).all(|(a, b)| a.0 == b.0 && a.1.to_bits() == b.1.to_bits())
            && s.domain == def.domain
            && s.targets.len() == def.targets.len()
            && s.targets.iter().zip(&def.targets).all(|(a, b)| a.0 == b.0 && a.1 == b.1 && a.2 == b.2)
    };
    if let Some(i) = reg.iter().rposition(|s| same(s)) {
        return ModuleKind(i as u32);
    }
    let defaults: Vec<(&'static str, f32)> = def.defaults.iter().map(|(n, v)| (leak(n), *v)).collect();
    let targets: Vec<(usize, &'static str, Mode)> = def.targets.iter().map(|(o, p, m)| (*o, leak(p), *m)).collect();
    let spec: &'static ModuleSpec = Box::leak(Box::new(ModuleSpec {
        kind: leak(&kind),
        inputs: ports(&def.inputs),
        outputs: ports(&def.outputs),
        defaults: Box::leak(defaults.into_boxed_slice()),
        domain: def.domain,
        targets: Box::leak(targets.into_boxed_slice()),
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

    fn pt(list: &[(&str, PortType)]) -> Vec<PortDef> {
        list.iter().map(|&(n, t)| PortDef::new(n, t)).collect()
    }

    fn def(inputs: &[(&str, PortType)], outputs: &[(&str, PortType)], defaults: &[(&str, f32)]) -> ModuleDef {
        ModuleDef::new(pt(inputs), pt(outputs), defaults.iter().map(|&(n, v)| (n.to_owned(), v)).collect())
    }

    #[test]
    fn a_module_kind_is_a_graph_kind() {
        let m = register("regtest_voice", &def(&[("spread", PortType::Control)], &[("cv", PortType::Audio)], &[("spread", 0.25)]));
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
        let a = register("regtest_twice", &def(&[("a", PortType::Control)], &[], &[]));
        assert_eq!(register("regtest_twice", &def(&[("a", PortType::Control)], &[], &[])), a);
        let b = register("regtest_twice", &def(&[("a", PortType::Control), ("b", PortType::Control)], &[], &[]));
        assert_ne!(a, b);
        assert_eq!(lookup("module:regtest_twice"), Some(b), "lookup finds the newest");
        assert_eq!(a.spec().inputs.len(), 1, "an old Kind keeps its ports");
        // another domain or channel count is another kind too
        let c = register("regtest_twice", &ModuleDef { domain: Domain::Control, ..def(&[("a", PortType::Control), ("b", PortType::Control)], &[], &[]) });
        assert_ne!(b, c);
        assert_eq!(Kind::Module(c).domain(), Domain::Control);
        assert_eq!(Kind::Module(b).domain(), Domain::Audio);
    }

    #[test]
    fn a_path_output_drives_its_target_like_a_link() {
        let mut d = def(&[("push", PortType::Control)], &[("level", PortType::Control), ("cut", PortType::Control), ("far", PortType::Control)], &[]);
        d.domain = Domain::Control;
        d.targets = vec![(0, "lead:osc.gain".into(), Mode::Set), (1, "fx.cutoff".into(), Mode::Mul), (2, "lead:@preset".into(), Mode::Set)];
        register("regtest_policy", &d);
        let scene = |extra: &str| {
            Scene::parse(&format!(
                "[[node]]\nid=\"p\"\nkind=\"module:regtest_policy\"\n[[node]]\nid=\"lead_osc\"\nkind=\"osc\"\ngain=0.25\n\
                 [[node]]\nid=\"fx\"\nkind=\"noise\"\n{extra}"
            ))
            .unwrap()
        };
        let g = Graph::build(&scene("")).unwrap();
        let osc = g.nodes[1].kind.input("gain").unwrap();
        let cut = g.nodes[2].kind.input("cutoff").unwrap();
        // set: the output alone, holding the target's own value until written; mul: the target's own value × the output
        let set = g.links.iter().find(|l| l.to == (1, osc)).expect("the set target is a link");
        assert_eq!((set.from, set.scale, set.offset), ((0, 0), 1.0, 0.0));
        assert_eq!(g.holds.iter().find(|h| h.0 == (0, 0)).map(|h| h.1), Some(0.25));
        let mul = g.links.iter().find(|l| l.to == (2, cut)).expect("the mul target is a link");
        let own = crate::params::default_for(Kind::Noise, "cutoff");
        assert_eq!((mul.from, mul.scale, mul.offset), ((0, 1), own, 0.0));
        assert_eq!(g.holds.iter().find(|h| h.0 == (0, 1)).map(|h| h.1), Some(1.0));
        // a reserved member (`@preset`) is the rig's (#314): unbound here, as is a node the scene does not have
        assert_eq!(g.links.len(), 2);
        // a typo'd port, a target that already has a cable: the core's error
        let bad = |d: &ModuleDef, extra: &str| {
            register("regtest_policy_bad", d);
            Graph::build(&Scene::parse(&format!("[[node]]\nid=\"p\"\nkind=\"module:regtest_policy_bad\"\n[[node]]\nid=\"fx\"\nkind=\"noise\"\n[[node]]\nid=\"l\"\nkind=\"lfo\"\n{extra}")).unwrap())
                .unwrap_err()
                .to_string()
        };
        let mut typo = d.clone();
        typo.targets = vec![(1, "fx.cutof".into(), Mode::Set)];
        assert!(bad(&typo, "").contains("fx.cutof"));
        let mut taken = d.clone();
        taken.targets = vec![(1, "fx.cutoff".into(), Mode::Add)];
        assert!(bad(&taken, "[[link]]\nfrom=\"l\"\nto=\"fx.cutoff\"\n").contains("already has a link"));
    }
}
