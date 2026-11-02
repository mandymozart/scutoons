//! Where a being's values come from (#188): this deck's running graph (`Local` – the control domain's slots and the
//! audio meters) or another deck's stream over the relay (`Mirror`, phase 4 – what its `deck.values` said last). The
//! being reads both the same way, so a deck's being on the conductor's screen is built and drawn like its own.
//!
//! A value's path on the wire is `node.port` – a control input (a button), a control output or an audio output (its
//! block peak; what a hose's bubbles follow) – or `node.shape` for an LFO's waveform (an index into LFO_SHAPES). No
//! kind has an input and an output of the same name, so a path is never ambiguous (graph.rs, tested below).
//!
//! `Knobs` (#198) is what the space reads and turns of a graph's control domain: deck-native's `Controls` on the deck, a
//! the page's `Mirror` in the browser (deck/space-web), a `Table` in tests.
use crate::graph::{Graph, Kind, PortType};
use crate::params;

/// A graph's control domain as the space sees it: what an input reads, a node's outputs this frame, an LFO's shape –
/// and the hand that turns an unlinked input (a button's drag, a feeling's nudge, a double click's ease back).
pub trait Knobs {
    /// `node`'s control input `port`: the linked output mapped by the link, else the node's own param, else `default`
    fn input(&self, node: usize, port: &str, default: f32) -> f32;
    /// `node`'s control outputs this frame (its port order)
    fn outputs(&self, node: usize) -> &[f32];
    /// an LFO's shape (an index into LFO_SHAPES); None for other nodes
    fn lfo_shape(&self, node: usize) -> Option<usize>;
    /// `node`'s control input `port` to `value` – a linked input's base (#188: the link's modulation flows on top);
    /// false = no such port (or a linked input whose base this view can't turn), nothing changes
    fn set(&mut self, node: usize, port: usize, value: f32) -> bool;
    /// a linked input's base (what its knob turns – #188); None for an unlinked one, or where the base is unknown
    fn base(&self, _node: usize, _port: usize) -> Option<f32> {
        None
    }
    /// an LFO's shape from outside; false for other nodes
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool;
}

pub trait Values {
    /// `node`'s control input `port` (linked or not), else `default`
    fn input(&self, node: usize, port: &str, default: f32) -> f32;
    /// `node`'s control output `port` this frame
    fn output(&self, node: usize, port: usize) -> f32;
    /// `node`'s audio output `port`: its newest block peak
    fn meter(&self, node: usize, port: usize) -> f32;
    /// an LFO's waveform (None for other nodes)
    fn lfo_shape(&self, node: usize) -> Option<usize>;
    /// a linked input's base (#188: its knob; the input is the base + the link's modulation); None = unlinked or unknown
    fn base(&self, _node: usize, _port: usize) -> Option<f32> {
        None
    }
}

/// this deck's own graph: the control domain + the audio meters (node, port → peak)
pub struct Local<'a> {
    pub ctl: &'a dyn Knobs,
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
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.ctl.base(node, port)
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
    /// per linked input: its base as the page wrote it (#188; NaN = unknown); None = unlinked
    bases: Vec<Vec<Option<f32>>>,
    outputs: Vec<Vec<f32>>,
    shapes: Vec<Option<usize>>,
}

impl Mirror {
    pub fn new(graph: &Graph) -> Mirror {
        Mirror {
            kinds: graph.nodes.iter().map(|n| n.kind).collect(),
            inputs: graph.nodes.iter().map(|n| vec![f32::NAN; n.kind.inputs().len()]).collect(),
            bases: (0..graph.nodes.len())
                .map(|i| (0..graph.nodes[i].kind.inputs().len()).map(|k| graph.link_into(i, k).map(|_| f32::NAN)).collect())
                .collect(),
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
            Some(Path::Shape(n)) => self.shapes[n] = Some((x.max(0.0) as usize).min(crate::params::LFO_SHAPES.len() - 1)),
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
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        Mirror::base(self, node, port)
    }
}

/// The page's hand in the browser (#198, deck/space-web): a button or a feeling turns the mirror's values, the page
/// reads them back – one state, not two.
impl Mirror {
    /// `node`'s input `port` as last written (NaN = never)
    pub fn raw_input(&self, node: usize, port: usize) -> f32 {
        self.inputs[node][port]
    }

    /// `node`'s input `port` to `value` (false: no such port)
    pub fn set_input(&mut self, node: usize, port: usize, value: f32) -> bool {
        match self.inputs.get_mut(node).and_then(|i| i.get_mut(port)) {
            Some(v) => {
                *v = value;
                true
            }
            None => false,
        }
    }

    /// a linked input's base as last written (None: unlinked, or never written)
    pub fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.bases.get(node).and_then(|b| b.get(port)).copied().flatten().filter(|v| !v.is_nan())
    }

    /// a linked input's base (#188: the page's value under the modulation); false: no such input or not linked
    pub fn set_base(&mut self, node: usize, port: usize, value: f32) -> bool {
        match self.bases.get_mut(node).and_then(|b| b.get_mut(port)) {
            Some(Some(b)) => {
                *b = value;
                true
            }
            _ => false,
        }
    }

    /// whether `node`'s input `port` is linked
    pub fn linked(&self, node: usize, port: usize) -> bool {
        matches!(self.bases.get(node).and_then(|b| b.get(port)), Some(Some(_)))
    }

    /// `node`'s outputs as last written
    pub fn outputs_of(&self, node: usize) -> &[f32] {
        &self.outputs[node]
    }

    /// an LFO's shape (false for other nodes)
    pub fn set_shape(&mut self, node: usize, shape: usize) -> bool {
        match self.shapes.get_mut(node) {
            Some(Some(s)) => {
                *s = shape.min(params::LFO_SHAPES.len() - 1);
                true
            }
            _ => false,
        }
    }

    /// `node`'s output `port` (a control value, an audio output's peak)
    pub fn set_output(&mut self, node: usize, port: usize, value: f32) {
        if let Some(v) = self.outputs.get_mut(node).and_then(|o| o.get_mut(port)) {
            *v = value;
        }
    }
}

/// a linked input's source: (node, output port, scale, offset)
type Linked = (usize, usize, f32, f32);

/// A graph's control domain without the running stage (tests, tools): inputs from the node's params and its links (the
/// linked output × scale + offset), outputs written by hand or by `step`'s LFOs – deck-native's `Controls` without the
/// audio, the FFT and the tempo node.
#[derive(Debug, Clone)]
pub struct Table {
    kinds: Vec<Kind>,
    /// per node, per input: the linked output, else the value set (NaN = the default)
    inputs: Vec<Vec<(Option<Linked>, f32)>>,
    outputs: Vec<Vec<f32>>,
    shapes: Vec<Option<usize>>,
    phases: Vec<f32>,
    order: Vec<usize>,
}

impl Table {
    pub fn new(graph: &Graph) -> Table {
        let inputs = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                n.kind
                    .inputs()
                    .iter()
                    .enumerate()
                    .map(|(k, port)| match graph.link_into(i, k) {
                        Some(l) if l.ty == PortType::Control => (Some((l.from.0, l.from.1, l.scale, l.offset)), f32::NAN),
                        _ if n.params.contains_key(port.name) => (None, n.f32(port.name, 0.0)),
                        _ => (None, f32::NAN),
                    })
                    .collect()
            })
            .collect();
        let shape = |s: &str| params::LFO_SHAPES.iter().position(|x| *x == s).unwrap_or(0);
        Table {
            kinds: graph.nodes.iter().map(|n| n.kind).collect(),
            inputs,
            outputs: graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(),
            shapes: graph.nodes.iter().map(|n| (n.kind == Kind::Lfo).then(|| shape(n.str("shape").unwrap_or("sine")))).collect(),
            phases: vec![0.0; graph.nodes.len()],
            order: graph.order.clone(),
        }
    }

    /// `node`'s output `port` this frame
    pub fn set_output(&mut self, node: usize, port: usize, value: f32) {
        self.outputs[node][port] = value;
    }

    /// one frame of the LFOs in the graph's order, like the stage's control domain: (0.5 + 0.5 × wave) × depth
    pub fn step(&mut self, dt: f32) {
        for k in 0..self.order.len() {
            let n = self.order[k];
            let Some(shape) = self.shapes[n] else { continue };
            let rate = self.input(n, "rate", 0.25);
            let depth = self.input(n, "depth", 1.0);
            let ph = (self.phases[n] + rate * dt).rem_euclid(1.0);
            self.phases[n] = ph;
            let wave = match shape {
                0 => (std::f32::consts::TAU * ph).sin(),
                1 => ph * 2.0 - 1.0,
                _ if ph < 0.5 => 1.0,
                _ => -1.0,
            };
            self.outputs[n][0] = (0.5 + 0.5 * wave) * depth;
        }
    }
}

impl Knobs for Table {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        let Some(k) = self.kinds[node].input(port) else { return default };
        match self.inputs[node][k] {
            (Some((n, p, scale, offset)), _) => self.outputs[n][p] * scale + offset,
            (None, v) if v.is_nan() => default,
            (None, v) => v,
        }
    }
    fn outputs(&self, node: usize) -> &[f32] {
        &self.outputs[node]
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.shapes[node]
    }
    fn set(&mut self, node: usize, port: usize, value: f32) -> bool {
        match self.inputs.get_mut(node).and_then(|i| i.get_mut(port)) {
            Some((None, v)) => {
                *v = value;
                true
            }
            // a linked input: its base (the link's offset), the link on top (#188)
            Some((Some(link), _)) => {
                link.3 = value;
                true
            }
            None => false,
        }
    }
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        match self.inputs.get(node).and_then(|i| i.get(port)) {
            Some((Some(link), _)) => Some(link.3),
            _ => None,
        }
    }
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        match self.shapes.get_mut(node) {
            Some(Some(s)) => {
                *s = shape.min(params::LFO_SHAPES.len() - 1);
                true
            }
            _ => false,
        }
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
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
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
