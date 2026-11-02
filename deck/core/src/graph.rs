//! The control graph: a scene's nodes built through the registry, links checked once (types, one link per input, no
//! cycles), then one `step` per frame runs the control blocks in topological order and `render` fills an audio block
//! with the audio nodes' sum. The scene format is deck/native's (`[[node]]` id + kind + params, `[[link]]` from → to with
//! scale / offset); an unlinked input reads the node's param of the same name.
//!
//! Values live in one flat `Vec<f32>` (a slot per output port) and links are resolved to slot indices at build – a
//! step does no lookups, no allocation (the same rule as the stage's audio callback).
use serde::Deserialize;

use crate::module::{Block, KindInfo, Params, PortType, Registry};

#[derive(Debug, Clone, Deserialize)]
pub struct SceneDef {
    #[serde(default, rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link")]
    pub links: Vec<LinkDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub kind: String,
    #[serde(flatten)]
    pub params: Params,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkDef {
    pub from: String,
    pub to: String,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub offset: f32,
}

fn one() -> f32 {
    1.0
}

impl SceneDef {
    pub fn parse(toml_text: &str) -> Result<SceneDef, String> {
        toml::from_str(toml_text).map_err(|e| e.to_string())
    }

    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().map(|n| n.kind.as_str())
    }
}

/// `node.port`, or a bare node id = its port `out`
fn split_ref(r: &str) -> (&str, &str) {
    r.split_once('.').unwrap_or((r, "out"))
}

/// An input: a link (slot, scale, offset) or a constant (the param).
#[derive(Debug, Clone, Copy)]
enum Input {
    Link { slot: usize, scale: f32, offset: f32 },
    Const(f32),
}

struct Node {
    block: Box<dyn Block>,
    info: &'static KindInfo,
    inputs: Vec<Input>,
    /// the first slot of this node's outputs
    out: usize,
}

pub struct ControlGraph {
    nodes: Vec<Node>,
    order: Vec<usize>,
    values: Vec<f32>,
    scratch_in: Vec<f32>,
    scratch_out: Vec<f32>,
    audio: Vec<usize>,
    mix: Vec<f32>,
}

impl ControlGraph {
    pub fn build(scene: &SceneDef, registry: &Registry, rate: f32) -> Result<ControlGraph, String> {
        let mut nodes = Vec::with_capacity(scene.nodes.len());
        let mut slots = 0;
        for n in &scene.nodes {
            if scene.nodes.iter().filter(|m| m.id == n.id).count() > 1 {
                return Err(format!("node id '{}' is used twice", n.id));
            }
            let (module, info) = registry.find(&n.kind).map_err(|e| format!("node '{}': {e}", n.id))?;
            let block = module.build(&n.kind, &n.params, rate).ok_or_else(|| format!("node '{}': cannot build", n.id))?;
            let inputs = info.inputs.iter().map(|p| Input::Const(crate::module::param(&n.params, p.name, 0.0))).collect();
            nodes.push(Node { block, info, inputs, out: slots });
            slots += info.outputs.len();
        }
        let index = |id: &str| scene.nodes.iter().position(|n| n.id == id);
        let mut edges = vec![];
        for l in &scene.links {
            let ((fid, fport), (tid, tport)) = (split_ref(&l.from), split_ref(&l.to));
            let what = || format!("link {} → {}", l.from, l.to);
            let fi = index(fid).ok_or_else(|| format!("{}: no node '{fid}'", what()))?;
            let ti = index(tid).ok_or_else(|| format!("{}: no node '{tid}'", what()))?;
            let fp = nodes[fi].info.output(fport).ok_or_else(|| format!("{}: no output '{fport}'", what()))?;
            let tp = nodes[ti].info.input(tport).ok_or_else(|| format!("{}: no input '{tport}'", what()))?;
            let (fty, tty) = (nodes[fi].info.outputs[fp].ty, nodes[ti].info.inputs[tp].ty);
            if fty != PortType::Control || tty != PortType::Control {
                return Err(format!("{}: the control graph links control ports only ({fty:?} → {tty:?})", what()));
            }
            if matches!(nodes[ti].inputs[tp], Input::Link { .. }) {
                return Err(format!("{}: the input already has a link", what()));
            }
            nodes[ti].inputs[tp] = Input::Link { slot: nodes[fi].out + fp, scale: l.scale, offset: l.offset };
            edges.push((fi, ti));
        }
        let order = topo(nodes.len(), &edges)?;
        let audio = (0..nodes.len()).filter(|&i| nodes[i].info.outputs.iter().any(|p| p.ty == PortType::Audio)).collect();
        let max_in = nodes.iter().map(|n| n.inputs.len()).max().unwrap_or(0);
        let max_out = nodes.iter().map(|n| n.info.outputs.len()).max().unwrap_or(0);
        Ok(ControlGraph {
            nodes,
            order,
            values: vec![0.0; slots],
            scratch_in: vec![0.0; max_in],
            scratch_out: vec![0.0; max_out],
            audio,
            mix: Vec::new(),
        })
    }

    fn gather(values: &[f32], inputs: &[Input], into: &mut [f32]) {
        for (x, input) in into.iter_mut().zip(inputs) {
            *x = match *input {
                Input::Link { slot, scale, offset } => values[slot] * scale + offset,
                Input::Const(c) => c,
            };
        }
    }

    /// One frame of the control domain.
    pub fn step(&mut self, dt: f32) {
        for &i in &self.order {
            let n = &mut self.nodes[i];
            let (ni, no) = (n.inputs.len(), n.info.outputs.len());
            Self::gather(&self.values, &n.inputs, &mut self.scratch_in[..ni]);
            n.block.control(&self.scratch_in[..ni], &mut self.scratch_out[..no], dt);
            self.values[n.out..n.out + no].copy_from_slice(&self.scratch_out[..no]);
        }
    }

    /// One audio block: the sum of every audio node, played with its current control inputs. (A prototype mix: audio
    /// links between nodes are deck/native's audio graph, not modelled here.)
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        if self.mix.len() < out.len() {
            self.mix.resize(out.len(), 0.0);
        }
        for &i in &self.audio {
            let n = &mut self.nodes[i];
            let ni = n.inputs.len();
            Self::gather(&self.values, &n.inputs, &mut self.scratch_in[..ni]);
            let mix = &mut self.mix[..out.len()];
            n.block.audio(&self.scratch_in[..ni], mix);
            for (o, m) in out.iter_mut().zip(mix.iter()) {
                *o += *m;
            }
        }
    }

    /// The value of `node.port` after the last step.
    pub fn value(&self, scene: &SceneDef, r: &str) -> Option<f32> {
        let (id, port) = split_ref(r);
        let i = scene.nodes.iter().position(|n| n.id == id)?;
        let p = self.nodes[i].info.output(port)?;
        Some(self.values[self.nodes[i].out + p])
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Kahn's algorithm, stable by declaration order.
fn topo(n: usize, edges: &[(usize, usize)]) -> Result<Vec<usize>, String> {
    let mut indeg = vec![0usize; n];
    for &(_, t) in edges {
        indeg[t] += 1;
    }
    let mut order = Vec::with_capacity(n);
    let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).rev().collect();
    while let Some(i) = ready.pop() {
        order.push(i);
        let mut next: Vec<usize> = edges.iter().filter(|&&(f, _)| f == i).map(|&(_, t)| t).collect();
        next.sort_unstable_by(|a, b| b.cmp(a));
        for t in next {
            indeg[t] -= 1;
            if indeg[t] == 0 {
                ready.push(t);
            }
        }
    }
    if order.len() < n {
        return Err("the control links form a cycle".into());
    }
    Ok(order)
}

/// A synthetic scene for benchmarks: `n` LFOs in a chain through `smooth` and `mul` (each LFO's depth fed by the
/// previous chain – 3n levels deep), plus `voices` FM voices whose pitch the chain modulates (deck/lab). Kinds: control
/// + synth.
pub fn bench_scene(n: usize, voices: usize) -> String {
    scene_of(n, voices, true)
}

/// The same nodes without the links between the chains: 3 levels, each n wide – the shape of a real show scene (many
/// parallel parameters), where batching pays.
pub fn bench_scene_wide(n: usize, voices: usize) -> String {
    scene_of(n, voices, false)
}

fn scene_of(n: usize, voices: usize, chained: bool) -> String {
    let mut s = String::new();
    for i in 0..n {
        s += &format!("[[node]]\nid = \"l{i}\"\nkind = \"lfo\"\nrate = {}\ndepth = 1.0\n", 0.1 + i as f32 * 0.01);
        s += &format!("[[node]]\nid = \"s{i}\"\nkind = \"smooth\"\ntime = 0.05\n");
        s += &format!("[[node]]\nid = \"m{i}\"\nkind = \"mul\"\nb = 0.5\n");
    }
    for i in 0..n {
        s += &format!("[[link]]\nfrom = \"l{i}\"\nto = \"s{i}.in\"\n[[link]]\nfrom = \"s{i}\"\nto = \"m{i}.a\"\n");
        if chained && i > 0 {
            s += &format!("[[link]]\nfrom = \"m{}\"\nto = \"l{i}.depth\"\noffset = 1.0\n", i - 1);
        }
    }
    for v in 0..voices {
        s += &format!("[[node]]\nid = \"v{v}\"\nkind = \"fm\"\nratio = 2.0\nindex = 1.5\ngain = 0.05\n");
        if n > 0 {
            s += &format!("[[link]]\nfrom = \"m{}\"\nto = \"v{v}.freq\"\nscale = 20.0\noffset = {}\n", v % n, 110.0 * (v + 1) as f32);
        }
    }
    s
}

#[cfg(all(test, feature = "control", feature = "synth"))]
mod tests {
    use super::*;

    const SCENE: &str = r#"
[[node]]
id = "slow"
kind = "lfo"
rate = 0.25
depth = 1.0
[[node]]
id = "twice"
kind = "mul"
b = 2.0
[[node]]
id = "voice"
kind = "fm"
ratio = 2.0
index = 1.0
gain = 0.5
[[link]]
from = "slow"
to = "twice.a"
[[link]]
from = "twice"
to = "voice.freq"
scale = 100.0
offset = 220.0
"#;

    #[test]
    fn runs_in_order_and_renders() {
        let scene = SceneDef::parse(SCENE).unwrap();
        let mut g = ControlGraph::build(&scene, &Registry::compiled(), 48_000.0).unwrap();
        g.step(1.0); // a quarter turn of a 0.25 Hz LFO
        assert!((g.value(&scene, "slow").unwrap() - 1.0).abs() < 1e-5);
        assert!((g.value(&scene, "twice").unwrap() - 2.0).abs() < 1e-5, "the mul ran after the lfo in the same step");
        let mut block = [0.0; 128];
        g.render(&mut block);
        assert!(block.iter().any(|x| x.abs() > 0.1));
    }

    #[test]
    fn refusals() {
        let r = Registry::compiled();
        let build = |s: &str| ControlGraph::build(&SceneDef::parse(s).unwrap(), &r, 48_000.0).err().unwrap();
        assert!(build("[[node]]\nid = \"m\"\nkind = \"model3d\"\n").contains("needs module '3d'"));
        let cycle = "[[node]]\nid = \"a\"\nkind = \"add\"\n[[node]]\nid = \"b\"\nkind = \"add\"\n[[link]]\nfrom = \"a\"\nto = \"b.a\"\n[[link]]\nfrom = \"b\"\nto = \"a.a\"\n";
        assert!(build(cycle).contains("cycle"));
        let twice = "[[node]]\nid = \"a\"\nkind = \"lfo\"\n[[node]]\nid = \"b\"\nkind = \"add\"\n[[link]]\nfrom = \"a\"\nto = \"b.a\"\n[[link]]\nfrom = \"a\"\nto = \"b.a\"\n";
        assert!(build(twice).contains("already has a link"));
    }

    #[test]
    fn the_bench_scene_builds() {
        let scene = SceneDef::parse(&bench_scene(100, 8)).unwrap();
        let mut g = ControlGraph::build(&scene, &Registry::compiled(), 48_000.0).unwrap();
        assert_eq!(g.len(), 308);
        g.step(1.0 / 60.0);
        let mut block = [0.0; 128];
        g.render(&mut block);
        assert!(block.iter().all(|x| x.is_finite()));
    }
}
