//! The control domain, evaluated once per frame on the batch model (#183): compiled at load into one value slot per
//! control output, every control input as a column (`deck_core::batch::Col`: the linked slot × scale + offset, or the
//! node's param as a constant) and the LFOs as batches – per dependency level, per shape, struct-of-arrays, one loop
//! each. A frame: the FFT analyzers, the tempo node (the clock at the heard moment), the LFO batches level by level, then
//! the audio nodes' linked inputs into their Params. Video nodes read theirs with `input()` (a table lookup).
//! `set` changes an unlinked input from outside, like a MIDI CC (the being's 3D buttons, #188): the input's constant, the
//! LFO batch's column entry or the audio node's Param – a linked input belongs to its link.
use crate::audio::{Handles, Param};
use crate::fft::Analyzer;
use crate::graph::{Graph, Kind, PortType};
use crate::tempo;
use deck_core::batch::{levels, Col};
use deck_core::math::{fract_fast, sin_turns};

/// Where a control input's value comes from (fixed at load: the graph and the params don't change while it runs).
#[derive(Debug, Clone, Copy)]
enum Src {
    /// a link: `values[slot] * scale + offset`
    Link(u32, f32, f32),
    /// the node's param of the same name
    Param(f32),
    /// neither: the caller's default
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Sine,
    Saw,
    Square,
}

/// The LFOs of one level and one shape: rate and depth as columns, the phase per LFO.
struct Lfos {
    shape: Shape,
    rate: Col,
    depth: Col,
    phase: Vec<f32>,
    out: Vec<u32>,
}

impl Lfos {
    /// One loop over the batch – no dispatch, no branch on the input's kind or the shape (vectorisable).
    #[inline(always)]
    fn step(&mut self, v: &mut [f32], dt: f32) {
        for i in 0..self.out.len() {
            let ph = fract_fast(self.phase[i] + self.rate.get(v, i) * dt);
            self.phase[i] = ph;
            let wave = match self.shape {
                Shape::Sine => sin_turns(ph),
                Shape::Saw => ph * 2.0 - 1.0,
                Shape::Square => {
                    if ph < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            };
            v[self.out[i] as usize] = (0.5 + 0.5 * wave) * self.depth.get(v, i);
        }
    }
}

pub struct Controls {
    /// this frame's value of every control output: slot 0 stays 0 (the constant source), node n's outputs from `base[n]`
    values: Vec<f32>,
    base: Vec<u32>,
    kinds: Vec<Kind>,
    /// per node, per input port
    inputs: Vec<Vec<Src>>,
    /// the LFO batches, level by level
    lfos: Vec<Vec<Lfos>>,
    tempo_node: Option<usize>,
    analyzers: Vec<(usize, Analyzer)>,
    /// the audio nodes' linked control inputs: one column, one Param each
    param_in: Col,
    params: Vec<Param>,
    /// the audio nodes' unlinked control inputs (node, port, param) – `set` writes them
    knobs: Vec<(usize, usize, Param)>,
    /// per node: where an LFO sits in the batches (level, batch, index)
    lfo_at: Vec<Option<(usize, usize, usize)>>,
    /// this frame's tempo clock (quantised cuts read it)
    pub tempo: tempo::Now,
}

impl Controls {
    pub fn new(graph: &Graph, audio: Option<&mut Handles>) -> Controls {
        let mut base = Vec::with_capacity(graph.nodes.len());
        let mut slots = 1u32;
        for n in &graph.nodes {
            base.push(slots);
            slots += n.kind.outputs().len() as u32;
        }
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
                        Some(l) if l.ty == PortType::Control => Src::Link(base[l.from.0] + l.from.1 as u32, l.scale, l.offset),
                        _ if n.params.contains_key(port.name) => Src::Param(n.f32(port.name, 0.0)),
                        _ => Src::Default,
                    })
                    .collect()
            })
            .collect();
        let mut c = Controls {
            values: vec![0.0; slots as usize],
            base,
            kinds: graph.nodes.iter().map(|n| n.kind).collect(),
            inputs,
            lfos: Vec::new(),
            tempo_node: graph.first(Kind::Tempo),
            analyzers: Vec::new(),
            param_in: Col::default(),
            params: Vec::new(),
            knobs: Vec::new(),
            lfo_at: vec![None; graph.nodes.len()],
            tempo: tempo::Now::default(),
        };
        c.lfos = c.batch_lfos(graph);
        for (l, level) in c.lfos.iter().enumerate() {
            for (b, batch) in level.iter().enumerate() {
                for (k, &slot) in batch.out.iter().enumerate() {
                    if let Some(node) = c.base.iter().position(|&x| x == slot) {
                        c.lfo_at[node] = Some((l, b, k));
                    }
                }
            }
        }
        if let Some(h) = audio {
            let rate = h.clock.rate;
            for (node, ring) in h.taps.drain(..) {
                c.analyzers.push((node, Analyzer::new(ring, rate, graph.nodes[node].f32("release", 0.25))));
            }
            for (node, port, p) in std::mem::take(&mut h.params) {
                let mut col = std::mem::take(&mut c.param_in);
                c.column(&mut col, node, port, p.get());
                c.param_in = col;
                c.params.push(p);
            }
            c.knobs = std::mem::take(&mut h.knobs);
        }
        c
    }

    /// Set `node`'s unlinked control input `port` to `value` (a MIDI CC, a 3D button): what `input` reads, the LFO's
    /// column, the audio node's Param. A linked input is its link's – false, nothing changes.
    pub fn set(&mut self, node: usize, port: usize, value: f32) -> bool {
        match self.inputs.get(node).and_then(|i| i.get(port)) {
            None | Some(Src::Link(..)) => return false,
            Some(_) => self.inputs[node][port] = Src::Param(value),
        }
        if let Some((l, b, k)) = self.lfo_at[node] {
            let batch = &mut self.lfos[l][b];
            match port {
                0 => batch.rate.set_constant(k, value),
                _ => batch.depth.set_constant(k, value),
            }
        }
        for (_, _, p) in self.knobs.iter().filter(|(n, k, _)| (*n, *k) == (node, port)) {
            p.set(value);
        }
        true
    }

    /// node `node`'s input `port` appended to `col` (a link, else the param, else `default` – as a constant)
    fn column(&self, col: &mut Col, node: usize, port: usize, default: f32) {
        match self.inputs[node][port] {
            Src::Link(slot, scale, offset) => col.push(slot, scale, offset),
            Src::Param(v) => col.constant(v),
            Src::Default => col.constant(default),
        }
    }

    /// The LFOs by level (an LFO feeding another's rate / depth sits a level above it; the tempo and the FFT are written
    /// before any level) and by shape inside a level.
    fn batch_lfos(&self, graph: &Graph) -> Vec<Vec<Lfos>> {
        let is_lfo = |i: usize| graph.nodes[i].kind == Kind::Lfo;
        let deps: Vec<Vec<usize>> = (0..graph.nodes.len())
            .map(|t| graph.links.iter().filter(|l| l.to.0 == t && is_lfo(t) && is_lfo(l.from.0)).map(|l| l.from.0).collect())
            .collect();
        // Graph::build refused cycles already
        let level = levels(&deps).expect("the graph is acyclic");
        let mut out: Vec<Vec<Lfos>> = Vec::new();
        for &i in graph.order.iter().filter(|&&i| is_lfo(i)) {
            let shape = match graph.nodes[i].str("shape").unwrap_or("sine") {
                "saw" => Shape::Saw,
                "square" => Shape::Square,
                _ => Shape::Sine,
            };
            if out.len() <= level[i] {
                out.resize_with(level[i] + 1, Vec::new);
            }
            let list = &mut out[level[i]];
            let b = match list.iter().position(|b| b.shape == shape) {
                Some(p) => &mut list[p],
                None => {
                    list.push(Lfos { shape, rate: Col::default(), depth: Col::default(), phase: vec![], out: vec![] });
                    list.last_mut().expect("pushed")
                }
            };
            self.column(&mut b.rate, i, 0, 0.25);
            self.column(&mut b.depth, i, 1, 1.0);
            b.phase.push(0.0);
            b.out.push(self.base[i]);
        }
        out.retain(|l| !l.is_empty());
        out
    }

    /// `node`'s control input `port`: the linked output mapped by the link, else the node's own param, else `default`
    pub fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        let Some(k) = self.kinds[node].input(port) else { return default };
        match self.inputs[node][k] {
            Src::Link(slot, scale, offset) => self.values[slot as usize] * scale + offset,
            Src::Param(v) => v,
            Src::Default => default,
        }
    }

    /// `node`'s control outputs this frame (its port order)
    pub fn outputs(&self, node: usize) -> &[f32] {
        let b = self.base[node] as usize;
        &self.values[b..b + self.kinds[node].outputs().len()]
    }

    /// one frame: `lead` = how many samples the audio callback has rendered past the heard moment (the FFT looks
    /// back), `tempo` = the clock at the heard moment
    pub fn update(&mut self, dt: f32, lead: usize, tempo: &tempo::Now) {
        let v = &mut self.values;
        for (node, a) in &mut self.analyzers {
            let b = self.base[*node] as usize;
            v[b..b + 4].copy_from_slice(&a.update(dt, lead));
        }
        self.tempo = *tempo;
        if let Some(i) = self.tempo_node {
            let p = &tempo.pos;
            // pulse: 1 on the beat, gone by its middle (0 while stopped)
            let pulse = if tempo.running { (1.0 - p.phase).powi(6) } else { 0.0 };
            let b = self.base[i] as usize;
            for (o, x) in v[b..b + 6].iter_mut().zip([tempo.bpm, p.beat, p.bar as f64, p.phase, p.bar_phase, pulse]) {
                *o = x as f32;
            }
        }
        for level in &mut self.lfos {
            for b in level {
                b.step(v, dt);
            }
        }
        for (i, p) in self.params.iter().enumerate() {
            p.set(self.param_in.get(v, i));
        }
    }

    /// the first FFT node's values (for the HUD)
    pub fn fft(&self) -> Option<[f32; 4]> {
        self.analyzers.first().map(|(_, a)| a.values)
    }

    /// (levels, batches) of the LFOs – the shape the build found
    pub fn shape(&self) -> (usize, usize) {
        (self.lfos.len(), self.lfos.iter().map(Vec::len).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Engine;
    use crate::scene::Scene;

    fn graph(text: &str) -> Graph {
        Graph::build(&Scene::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn links_map_lfos_into_video_and_audio_inputs() {
        let g = graph(
            "[[node]]\nid=\"l\"\nkind=\"lfo\"\nrate=0.25\nshape=\"square\"\n\
             [[node]]\nid=\"p\"\nkind=\"test_pattern\"\nzoom=3\n[[node]]\nid=\"o\"\nkind=\"osc\"\n\
             [[link]]\nfrom=\"l\"\nto=\"p.speed\"\nscale=2\noffset=1\n[[link]]\nfrom=\"l\"\nto=\"o.freq\"\nscale=100\n",
        );
        let (_, mut h) = Engine::build(&g, 48_000.0, 1.0).unwrap();
        let freq = h.params[0].2.clone();
        let mut c = Controls::new(&g, Some(&mut h));
        c.update(0.1, 0, &tempo::Now::default());
        assert_eq!(c.input(1, "speed", 0.0), 3.0); // square high = 1 → 1 * 2 + 1
        assert_eq!(c.input(1, "zoom", 1.0), 3.0); // unlinked: the node's param
        assert_eq!(c.input(1, "hue", 0.5), 0.5); // neither: the default
        assert_eq!(freq.get(), 100.0);
        c.update(2.0, 0, &tempo::Now::default()); // phase 0.025 + 0.5 → the low half
        assert_eq!(c.input(1, "speed", 0.0), 1.0);
    }

    #[test]
    fn set_writes_unlinked_inputs_like_a_cc() {
        let g = graph(
            "[[node]]
id=\"l\"
kind=\"lfo\"
rate=0
shape=\"square\"
[[node]]
id=\"p\"
kind=\"test_pattern\"
             [[node]]
id=\"f\"
kind=\"fm\"
[[link]]
from=\"l\"
to=\"p.speed\"
",
        );
        let (_, mut h) = Engine::build(&g, 48_000.0, 1.0).unwrap();
        let index = h.knobs.iter().find(|k| (k.0, k.1) == (2, 3)).expect("fm.index is a knob").2.clone();
        let mut c = Controls::new(&g, Some(&mut h));
        assert!(c.set(1, 2, 0.7), "p.hue: unlinked");
        assert_eq!(c.input(1, "hue", 0.0), 0.7);
        assert!(!c.set(1, 0, 3.0), "p.speed belongs to its link");
        assert!(c.set(2, 3, 4.5));
        assert_eq!(index.get(), 4.5, "the audio thread's Param");
        // the LFO's depth through its batch column: a square at phase 0 = depth
        assert!(c.set(0, 1, 0.25));
        c.update(0.01, 0, &tempo::Now::default());
        assert_eq!(c.input(1, "speed", 0.0), 0.25);
        assert!(c.set(0, 0, 50.0), "the rate");
        c.update(0.011, 0, &tempo::Now::default()); // phase 0.55 → the low half
        assert_eq!(c.input(1, "speed", 1.0), 0.0);
    }

    #[test]
    fn the_tempo_node_outputs_the_clock() {
        let g = graph("[[node]]\nid=\"t\"\nkind=\"tempo\"\n[[node]]\nid=\"p\"\nkind=\"test_pattern\"\n[[link]]\nfrom=\"t.pulse\"\nto=\"p.pulse\"\n");
        let mut c = Controls::new(&g, None);
        let now = tempo::Now { bpm: 96.0, pos: tempo::position(9.0, 4), per_bar: 4, running: true };
        c.update(0.016, 0, &now);
        assert_eq!(c.outputs(0), [96.0, 9.0, 2.0, 0.0, 0.25, 1.0]);
        assert_eq!(c.input(1, "pulse", 0.0), 1.0);
        c.update(0.016, 0, &tempo::Now { pos: tempo::position(9.5, 4), ..now });
        assert!(c.input(1, "pulse", 0.0) < 0.02);
    }

    /// The control domain as it ran before #183 (the patcher model: per frame, per LFO in topological order, each input
    /// found by searching the links, the shape read from the params) – the reference the batches must match.
    struct Patcher {
        out: Vec<Vec<f32>>,
        phase: Vec<f32>,
    }

    impl Patcher {
        fn input(&self, g: &Graph, node: usize, port: &str, default: f32) -> f32 {
            let n = &g.nodes[node];
            let Some(k) = n.kind.input(port) else { return default };
            match g.link_into(node, k) {
                Some(l) if l.ty == PortType::Control => self.out[l.from.0][l.from.1] * l.scale + l.offset,
                _ => n.f32(port, default),
            }
        }

        fn update(&mut self, g: &Graph, dt: f32, tempo: &tempo::Now) {
            if let Some(i) = g.first(Kind::Tempo) {
                let p = &tempo.pos;
                let pulse = if tempo.running { (1.0 - p.phase).powi(6) } else { 0.0 };
                let v = [tempo.bpm, p.beat, p.bar as f64, p.phase, p.bar_phase, pulse];
                for (o, x) in self.out[i].iter_mut().zip(v) {
                    *o = x as f32;
                }
            }
            for &i in &g.order {
                if g.nodes[i].kind == Kind::Lfo {
                    let rate = self.input(g, i, "rate", 0.25);
                    let depth = self.input(g, i, "depth", 1.0);
                    self.phase[i] = (self.phase[i] + rate * dt).fract();
                    let wave = match g.nodes[i].str("shape").unwrap_or("sine") {
                        "saw" => self.phase[i] * 2.0 - 1.0,
                        "square" => if self.phase[i] < 0.5 { 1.0 } else { -1.0 },
                        _ => (self.phase[i] * std::f32::consts::TAU).sin(),
                    };
                    self.out[i][0] = (0.5 + 0.5 * wave) * depth;
                }
            }
        }
    }

    /// The same scene through the patcher and the batches: every control output and every video input agree.
    #[test]
    fn the_batches_match_the_patcher() {
        // three levels of LFOs (rate and depth fed from the level below, the tempo's phase into one), all shapes,
        // negative rates, a test pattern reading them
        let mut s = String::from("[[node]]\nid=\"t\"\nkind=\"tempo\"\n[[node]]\nid=\"p\"\nkind=\"test_pattern\"\n");
        for l in 0..3 {
            for w in 0..7 {
                let shape = ["sine", "saw", "square"][(l + w) % 3];
                s += &format!("[[node]]\nid=\"l{l}_{w}\"\nkind=\"lfo\"\nrate={}\nshape=\"{shape}\"\n", 0.3 * w as f32 - 0.8);
                if l > 0 {
                    s += &format!("[[link]]\nfrom=\"l{}_{w}\"\nto=\"l{l}_{w}.rate\"\nscale=1.7\noffset=-0.4\n", l - 1);
                    s += &format!("[[link]]\nfrom=\"l{}_{}\"\nto=\"l{l}_{w}.depth\"\n", l - 1, (w + 1) % 7);
                }
            }
        }
        s += "[[link]]\nfrom=\"t.phase\"\nto=\"l0_3.depth\"\n[[link]]\nfrom=\"l2_4\"\nto=\"p.hue\"\nscale=2\n[[link]]\nfrom=\"l1_1\"\nto=\"p.zoom\"\n";
        let g = graph(&s);
        let mut b = Controls::new(&g, None);
        assert_eq!(b.shape(), (3, 9), "3 levels × 3 shapes");
        let mut p = Patcher { out: g.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(), phase: vec![0.0; g.nodes.len()] };
        for f in 0..600 {
            let now = tempo::Now { bpm: 120.0, pos: tempo::position(f as f64 * 0.033, 4), per_bar: 4, running: true };
            let dt = 1.0 / 60.0 + (f % 5) as f32 * 0.003;
            b.update(dt, 0, &now);
            p.update(&g, dt, &now);
            for i in 0..g.nodes.len() {
                for (x, y) in b.outputs(i).iter().zip(&p.out[i]) {
                    assert!((x - y).abs() < 1e-4, "frame {f} node {}: {x} vs {y}", g.nodes[i].id);
                }
            }
            for port in ["speed", "zoom", "hue", "pulse"] {
                let (x, y) = (b.input(1, port, 0.5), p.input(&g, 1, port, 0.5));
                assert!((x - y).abs() < 1e-4, "frame {f} p.{port}: {x} vs {y}");
            }
        }
    }
}
