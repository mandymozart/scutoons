//! Where a being's values come from (#188): this deck's running graph (`Local` – the control domain's slots and the
//! audio meters) or another deck's stream over the relay (`Mirror`, phase 4 – what its `deck.values` said last). The
//! being reads both the same way, so a deck's being on the conductor's screen is built and drawn like its own.
//!
//! A value's path on the wire is `node.port` – a control input (a button), a control output or an audio output (its
//! block peak; what a hose's bubbles follow) – or `node.shape` for an LFO's waveform (an index into LFO_SHAPES). No
//! kind has an input and an output of the same name, so a path is never ambiguous (graph.rs, tested below).
use crate::control::Controls;
use crate::graph::{Graph, Kind, PortType};
use crate::params;

pub trait Values {
    /// `node`'s control input `port` (linked or not), else `default`
    fn input(&self, node: usize, port: &str, default: f32) -> f32;
    /// `node`'s control output `port` this frame
    fn output(&self, node: usize, port: usize) -> f32;
    /// `node`'s audio output `port`: its newest block peak
    fn meter(&self, node: usize, port: usize) -> f32;
    /// an LFO's waveform (None for other nodes)
    fn lfo_shape(&self, node: usize) -> Option<usize>;
}

/// this deck's own graph: the control domain + the audio meters (node, port → peak)
pub struct Local<'a> {
    pub ctl: &'a Controls,
    pub meter: &'a dyn Fn(usize, usize) -> f32,
}

impl Values for Local<'_> {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        self.ctl.input(node, port, default)
    }
    fn output(&self, node: usize, port: usize) -> f32 {
        self.ctl.outputs(node).get(port).copied().unwrap_or(0.0)
    }
    fn meter(&self, node: usize, port: usize) -> f32 {
        (self.meter)(node, port)
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.ctl.lfo_shape(node)
    }
}

/// What a value's path names in a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    Input(usize, usize),
    Output(usize, usize),
    Shape(usize),
}

/// `node.port` → what it names in `graph` (None: no such node or port)
pub fn resolve(graph: &Graph, path: &str) -> Option<Path> {
    let (id, port) = path.split_once('.')?;
    let node = graph.nodes.iter().position(|n| n.id == id)?;
    let kind = graph.nodes[node].kind;
    if port == "shape" && kind == Kind::Lfo {
        return Some(Path::Shape(node));
    }
    kind.input(port).map(|p| Path::Input(node, p)).or_else(|| kind.output(port).map(|p| Path::Output(node, p)))
}

/// `node.port` with the node named by its id, else by its kind (the first node of that kind: `fft.bass`, `fm.index`)
/// – a being's feelings (feel.rs) read any deck's graph this way
pub fn resolve_loose(graph: &Graph, path: &str) -> Option<Path> {
    resolve(graph, path).or_else(|| {
        let (kind, port) = path.split_once('.')?;
        let node = graph.nodes.iter().find(|n| n.kind.name() == kind)?;
        resolve(graph, &format!("{}.{port}", node.id))
    })
}

/// the path of a resolved value (the wire's name)
pub fn path_name(graph: &Graph, p: Path) -> String {
    let n = |i: usize| &graph.nodes[i];
    match p {
        Path::Input(i, k) => format!("{}.{}", n(i).id, n(i).kind.inputs()[k].name),
        Path::Output(i, k) => format!("{}.{}", n(i).id, n(i).kind.outputs()[k].name),
        Path::Shape(i) => format!("{}.shape", n(i).id),
    }
}

/// Another deck's values as its stream said them last (phase 4): inputs, outputs (control values and audio peaks)
/// and LFO shapes per node; what never came reads as the param's default.
#[derive(Debug, Clone)]
pub struct Mirror {
    kinds: Vec<Kind>,
    inputs: Vec<Vec<f32>>,
    outputs: Vec<Vec<f32>>,
    shapes: Vec<Option<usize>>,
}

impl Mirror {
    pub fn new(graph: &Graph) -> Mirror {
        Mirror {
            kinds: graph.nodes.iter().map(|n| n.kind).collect(),
            inputs: graph.nodes.iter().map(|n| vec![f32::NAN; n.kind.inputs().len()]).collect(),
            outputs: graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(),
            shapes: graph.nodes.iter().map(|n| (n.kind == Kind::Lfo).then_some(0)).collect(),
        }
    }

    /// one value of the stream (a float vector – today its first component is the value); false = an unknown path
    pub fn apply(&mut self, graph: &Graph, path: &str, v: &[f32]) -> bool {
        let Some(&x) = v.first() else { return false };
        match resolve(graph, path) {
            Some(Path::Input(n, p)) => self.inputs[n][p] = x,
            Some(Path::Output(n, p)) => self.outputs[n][p] = x,
            Some(Path::Shape(n)) => self.shapes[n] = Some((x.max(0.0) as usize).min(crate::control::LFO_SHAPES.len() - 1)),
            None => return false,
        }
        true
    }
}

impl Values for Mirror {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        let Some(k) = self.kinds[node].input(port) else { return default };
        let v = self.inputs[node][k];
        if v.is_nan() {
            default
        } else {
            v
        }
    }
    fn output(&self, node: usize, port: usize) -> f32 {
        self.outputs[node].get(port).copied().unwrap_or(0.0)
    }
    fn meter(&self, node: usize, port: usize) -> f32 {
        self.output(node, port)
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.shapes[node]
    }
}

/// how much a value must move before the stream sends it again: a thousandth of its range (inputs), 0.002 for
/// outputs and peaks (0–1 mostly; the tempo's counts move by whole beats anyway)
pub fn epsilon(graph: &Graph, p: Path) -> f32 {
    match p {
        Path::Input(n, k) => {
            let kind = graph.nodes[n].kind;
            let s = params::spec(kind, kind.inputs()[k].name);
            ((s.max - s.min).abs() * 1e-3).max(1e-5)
        }
        Path::Output(n, k) if graph.nodes[n].kind.outputs()[k].ty == PortType::Audio => 0.01,
        Path::Output(..) => 2e-3,
        Path::Shape(_) => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};

    #[test]
    fn no_kind_names_an_input_and_an_output_alike() {
        for k in Kind::ALL {
            for i in k.inputs() {
                assert!(k.output(i.name).is_none(), "{k:?}.{}", i.name);
            }
        }
    }

    #[test]
    fn paths_resolve_and_a_mirror_reads_what_came() {
        let scene = crate::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        let g = Graph::build(&scene).unwrap();
        let fft = g.nodes.iter().position(|n| n.id == "fft").unwrap();
        assert_eq!(resolve(&g, "fft.level"), Some(Path::Output(fft, 0)));
        assert_eq!(resolve(&g, "voice.index").map(|p| path_name(&g, p)).as_deref(), Some("voice.index"));
        assert!(matches!(resolve(&g, "sway.shape"), Some(Path::Shape(_))));
        assert_eq!(resolve(&g, "voice.shape"), None, "only an LFO has a shape");
        assert_eq!(resolve(&g, "nobody.gain"), None);
        let mut m = Mirror::new(&g);
        let voice = g.nodes.iter().position(|n| n.id == "voice").unwrap();
        assert_eq!(m.input(voice, "index", 2.0), 2.0, "nothing came yet: the default");
        assert!(m.apply(&g, "voice.index", &[3.5]));
        assert!(m.apply(&g, "fft.level", &[0.7, 1.0]));
        assert!(!m.apply(&g, "voice.nothing", &[1.0]));
        assert!(!m.apply(&g, "voice.index", &[]));
        assert_eq!((m.input(voice, "index", 2.0), m.output(fft, 0)), (3.5, 0.7));
    }
}
