//! The data-oriented evaluator – the second architecture of #182 §4b, measured against `graph::ControlGraph`.
//!
//! `ControlGraph` is the patcher model (Max/MSP, Pd, ossia, the stage today): one object per node, one virtual call per
//! node per frame, inputs gathered through an enum. `BatchGraph` is the engine model (an ECS's systems, TouchDesigner's
//! CHOP channels, vvvv gamma / gen~ compiling the patch): at load the graph is split into dependency levels, inside a
//! level the nodes of one kind become one batch with its state and inputs as columns (struct of arrays), and a frame is
//! a few tight loops – no per-node dispatch, no branches on input kind (a constant is a link to slot 0 with scale 0),
//! loops the compiler can unroll and vectorise. Same scene format, same results (the test checks it against
//! `ControlGraph`); only the built-in kinds batch – a real stage would run the others as single-node batches.
use crate::graph::SceneDef;
use crate::math::{sin_turns, wrap01};
use crate::module::param;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum K {
    Lfo,
    Smooth,
    Mul,
    Add,
    Osc,
    Fm,
}

impl K {
    fn parse(s: &str) -> Option<(K, &'static [&'static str])> {
        Some(match s {
            "lfo" => (K::Lfo, &["rate", "depth"]),
            "smooth" => (K::Smooth, &["in", "time"]),
            "mul" => (K::Mul, &["a", "b"]),
            "add" => (K::Add, &["a", "b"]),
            "osc" => (K::Osc, &["freq", "gain"]),
            "fm" => (K::Fm, &["freq", "ratio", "index", "gain"]),
            _ => return None,
        })
    }
    fn audio(self) -> bool {
        matches!(self, K::Osc | K::Fm)
    }
}

/// One input port of a batch, a column per node: `values[src] * scale + offset`.
#[derive(Default)]
struct Col {
    src: Vec<u32>,
    scale: Vec<f32>,
    offset: Vec<f32>,
}

impl Col {
    #[inline(always)]
    fn get(&self, v: &[f32], i: usize) -> f32 {
        v[self.src[i] as usize] * self.scale[i] + self.offset[i]
    }
}

struct Batch {
    kind: K,
    cols: Vec<Col>,
    out: Vec<u32>,
    /// per-node state: the LFO's / oscillator's phase, the smoother's value, the FM carrier
    s1: Vec<f32>,
    /// the FM modulator's phase
    s2: Vec<f32>,
    /// per-block parameters of audio batches (filled once per block, read per sample)
    p: [Vec<f32>; 4],
    /// one sample of every voice – written element-wise (vectorises), then summed
    y: Vec<f32>,
}

impl Batch {
    fn new(kind: K, ports: usize) -> Self {
        Batch { kind, cols: (0..ports).map(|_| Col::default()).collect(), out: vec![], s1: vec![], s2: vec![], p: Default::default(), y: vec![] }
    }
    fn len(&self) -> usize {
        self.out.len()
    }
}

pub struct BatchGraph {
    levels: Vec<Vec<Batch>>,
    audio: Vec<Batch>,
    /// slot 0 is always 0 (the constant source); node n writes slot n + 1
    values: Vec<f32>,
    ids: Vec<String>,
    rate: f32,
}

impl BatchGraph {
    pub fn build(scene: &SceneDef, rate: f32) -> Result<BatchGraph, String> {
        let n = scene.nodes.len();
        let mut kinds = Vec::with_capacity(n);
        for node in &scene.nodes {
            kinds.push(K::parse(&node.kind).ok_or_else(|| format!("node '{}': kind '{}' does not batch", node.id, node.kind))?);
        }
        let index = |id: &str| scene.nodes.iter().position(|x| x.id == id);
        // inputs: (src slot, scale, offset) per node and port; unlinked = the param as a constant
        let mut inputs: Vec<Vec<(u32, f32, f32)>> = scene
            .nodes
            .iter()
            .zip(&kinds)
            .map(|(node, (_, ports))| ports.iter().map(|p| (0, 0.0, param(&node.params, p, 0.0))).collect())
            .collect();
        let mut deps: Vec<Vec<usize>> = vec![vec![]; n];
        for l in &scene.links {
            let (fid, _) = l.from.split_once('.').unwrap_or((&l.from, "out"));
            let (tid, tport) = l.to.split_once('.').unwrap_or((&l.to, "out"));
            let f = index(fid).ok_or_else(|| format!("link {}: no node '{fid}'", l.from))?;
            let t = index(tid).ok_or_else(|| format!("link {}: no node '{tid}'", l.to))?;
            let p = kinds[t].1.iter().position(|x| *x == tport).ok_or_else(|| format!("link {}: no input '{tport}'", l.to))?;
            if kinds[f].0.audio() {
                return Err(format!("link {}: audio outputs do not link in the control graph", l.from));
            }
            inputs[t][p] = (f as u32 + 1, l.scale, l.offset);
            deps[t].push(f);
        }
        // level = 1 + the deepest dependency (Kahn's order first, so a cycle is an error)
        let mut level = vec![0usize; n];
        let mut indeg: Vec<usize> = deps.iter().map(Vec::len).collect();
        let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
        let mut seen = 0;
        while let Some(i) = ready.pop() {
            seen += 1;
            for t in 0..n {
                for _ in deps[t].iter().filter(|&&f| f == i) {
                    level[t] = level[t].max(level[i] + 1);
                    indeg[t] -= 1;
                    if indeg[t] == 0 {
                        ready.push(t);
                    }
                }
            }
        }
        if seen < n {
            return Err("the control links form a cycle".into());
        }
        let depth = (0..n).filter(|&i| !kinds[i].0.audio()).map(|i| level[i] + 1).max().unwrap_or(0);
        let mut levels: Vec<Vec<Batch>> = (0..depth).map(|_| vec![]).collect();
        let mut audio: Vec<Batch> = vec![];
        for i in 0..n {
            let (k, ports) = kinds[i];
            let list = if k.audio() { &mut audio } else { &mut levels[level[i]] };
            let b = match list.iter_mut().position(|b| b.kind == k) {
                Some(p) => &mut list[p],
                None => {
                    list.push(Batch::new(k, ports.len()));
                    list.last_mut().expect("pushed")
                }
            };
            for (c, &(src, scale, offset)) in b.cols.iter_mut().zip(&inputs[i]) {
                c.src.push(src);
                c.scale.push(scale);
                c.offset.push(offset);
            }
            b.out.push(i as u32 + 1);
            b.s1.push(if k == K::Lfo { param(&scene.nodes[i].params, "phase", 0.0) } else { 0.0 });
            b.s2.push(0.0);
        }
        for b in &mut audio {
            b.p = std::array::from_fn(|_| vec![0.0; b.out.len()]);
            b.y = vec![0.0; b.out.len()];
        }
        Ok(BatchGraph { levels, audio, values: vec![0.0; n + 1], ids: scene.nodes.iter().map(|x| x.id.clone()).collect(), rate })
    }

    /// One frame of the control domain: per level, per kind, one loop.
    pub fn step(&mut self, dt: f32) {
        let v = &mut self.values;
        for level in &mut self.levels {
            for b in level.iter_mut() {
                let (c0, c1) = (&b.cols[0], &b.cols[1]);
                match b.kind {
                    K::Lfo => {
                        for i in 0..b.out.len() {
                            let ph = wrap01(b.s1[i] + c0.get(v, i) * dt);
                            b.s1[i] = ph;
                            v[b.out[i] as usize] = c1.get(v, i) * sin_turns(ph);
                        }
                    }
                    K::Smooth => {
                        for i in 0..b.out.len() {
                            let t = c1.get(v, i);
                            let k = if t <= 0.0 { 1.0 } else { 1.0 - (-dt / t).exp() };
                            b.s1[i] += (c0.get(v, i) - b.s1[i]) * k;
                            v[b.out[i] as usize] = b.s1[i];
                        }
                    }
                    K::Mul => {
                        for i in 0..b.out.len() {
                            v[b.out[i] as usize] = c0.get(v, i) * c1.get(v, i);
                        }
                    }
                    K::Add => {
                        for i in 0..b.out.len() {
                            v[b.out[i] as usize] = c0.get(v, i) + c1.get(v, i);
                        }
                    }
                    K::Osc | K::Fm => {}
                }
            }
        }
    }

    /// One audio block: every audio batch's parameters read once, then per sample one loop over the voices.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let v = &self.values;
        let rate = self.rate;
        for b in &mut self.audio {
            let nv = b.len();
            for (p, c) in b.p.iter_mut().zip(&b.cols) {
                for (i, x) in p.iter_mut().enumerate() {
                    *x = c.get(v, i);
                }
            }
            match b.kind {
                K::Fm => {
                    let [freq, ratio, index, gain] = &mut b.p;
                    // freq → carrier step, ratio → modulator step (in place: read once per block)
                    for i in 0..nv {
                        ratio[i] *= freq[i] / rate;
                        freq[i] /= rate;
                    }
                    let (c, m, y) = (&mut b.s1[..nv], &mut b.s2[..nv], &mut b.y[..nv]);
                    let (freq, ratio, index, gain) = (&freq[..nv], &ratio[..nv], &index[..nv], &gain[..nv]);
                    for o in out.iter_mut() {
                        // element-wise over the voices (no loop-carried value): the compiler vectorises this loop
                        for i in 0..nv {
                            let mo = sin_turns(m[i]) * index[i];
                            y[i] = gain[i] * sin_turns(c[i] + mo * (1.0 / TAU));
                            c[i] = wrap01(c[i] + freq[i]);
                            m[i] = wrap01(m[i] + ratio[i]);
                        }
                        *o += y.iter().sum::<f32>();
                    }
                }
                K::Osc => {
                    let [freq, gain, _, _] = &mut b.p;
                    for f in freq.iter_mut() {
                        *f /= rate;
                    }
                    let (ph, y) = (&mut b.s1[..nv], &mut b.y[..nv]);
                    for o in out.iter_mut() {
                        for i in 0..nv {
                            y[i] = gain[i] * sin_turns(ph[i]);
                            ph[i] = wrap01(ph[i] + freq[i]);
                        }
                        *o += y.iter().sum::<f32>();
                    }
                }
                _ => {}
            }
        }
    }

    /// The value of node `id` after the last step.
    pub fn value(&self, id: &str) -> Option<f32> {
        self.ids.iter().position(|x| x == id).map(|i| self.values[i + 1])
    }

    /// (levels, batches) – the shape the build found.
    pub fn shape(&self) -> (usize, usize) {
        (self.levels.len(), self.levels.iter().map(Vec::len).sum::<usize>() + self.audio.len())
    }
}

#[cfg(all(test, feature = "control", feature = "synth"))]
mod tests {
    use super::*;
    use crate::graph::{bench_scene, ControlGraph};
    use crate::module::Registry;

    /// The same scene through both architectures: every node's value and every audio sample agree.
    #[test]
    fn same_results_as_the_patcher_graph() {
        let scene = SceneDef::parse(&bench_scene(30, 6)).unwrap();
        let mut a = ControlGraph::build(&scene, &Registry::compiled(), 48_000.0).unwrap();
        let mut b = BatchGraph::build(&scene, 48_000.0).unwrap();
        let (mut x, mut y) = (vec![0.0; 128], vec![0.0; 128]);
        for frame in 0..120 {
            a.step(1.0 / 60.0);
            b.step(1.0 / 60.0);
            for node in &scene.nodes {
                if node.kind == "fm" {
                    continue;
                }
                let (va, vb) = (a.value(&scene, &node.id).unwrap(), b.value(&node.id).unwrap());
                assert!((va - vb).abs() < 1e-4, "frame {frame} {}: {va} vs {vb}", node.id);
            }
            a.render(&mut x);
            b.render(&mut y);
            for (s, (p, q)) in x.iter().zip(&y).enumerate() {
                assert!((p - q).abs() < 1e-4, "frame {frame} sample {s}: {p} vs {q}");
            }
        }
        // 30 chains: lfo → smooth → mul, each lfo fed by the previous chain's mul → 90 levels of one batch each
        assert_eq!(b.shape(), (90, 91));
    }

    #[test]
    fn wide_scenes_batch_wide() {
        // 50 independent lfo → mul pairs: 2 levels, 2 batches of 50 + the audio batch
        let mut s = String::new();
        for i in 0..50 {
            s += &format!("[[node]]\nid = \"l{i}\"\nkind = \"lfo\"\nrate = 1.0\ndepth = 1.0\n[[node]]\nid = \"m{i}\"\nkind = \"mul\"\nb = 2.0\n");
            s += &format!("[[link]]\nfrom = \"l{i}\"\nto = \"m{i}.a\"\n");
        }
        s += "[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 440.0\ngain = 0.1\n";
        let b = BatchGraph::build(&SceneDef::parse(&s).unwrap(), 48_000.0).unwrap();
        assert_eq!(b.shape(), (2, 3));
    }

    #[test]
    fn refusals() {
        let e = |s: &str| BatchGraph::build(&SceneDef::parse(s).unwrap(), 48_000.0).err().unwrap();
        assert!(e("[[node]]\nid = \"f\"\nkind = \"fft\"\n").contains("does not batch"));
        let cycle = "[[node]]\nid = \"a\"\nkind = \"add\"\n[[node]]\nid = \"b\"\nkind = \"add\"\n[[link]]\nfrom = \"a\"\nto = \"b.a\"\n[[link]]\nfrom = \"b\"\nto = \"a.a\"\n";
        assert!(e(cycle).contains("cycle"));
    }
}
