//! The control domain, evaluated once per frame: the FFT analyzers, the tempo node (the clock at the heard moment), the
//! LFOs, then every control link mapped (`value * scale + offset`) into its input – video nodes read theirs with
//! `input()`, audio nodes get theirs as Params.
use crate::audio::{Handles, Param};
use crate::fft::Analyzer;
use crate::graph::{Graph, Kind, PortType};
use crate::tempo;

pub struct Controls {
    /// per node, per output port: this frame's value
    pub out: Vec<Vec<f32>>,
    phase: Vec<f32>,
    analyzers: Vec<(usize, Analyzer)>,
    params: Vec<(usize, usize, Param)>,
    /// this frame's tempo clock (quantised cuts read it)
    pub tempo: tempo::Now,
}

impl Controls {
    pub fn new(graph: &Graph, audio: Option<&mut Handles>) -> Controls {
        let mut c = Controls {
            out: graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(),
            phase: vec![0.0; graph.nodes.len()],
            analyzers: Vec::new(),
            params: Vec::new(),
            tempo: tempo::Now::default(),
        };
        if let Some(h) = audio {
            let rate = h.clock.rate;
            for (node, ring) in h.taps.drain(..) {
                c.analyzers.push((node, Analyzer::new(ring, rate, graph.nodes[node].f32("release", 0.25))));
            }
            c.params = std::mem::take(&mut h.params);
        }
        c
    }

    /// `node`'s control input `port`: the linked output mapped by the link, else the node's own param, else `default`
    pub fn input(&self, graph: &Graph, node: usize, port: &str, default: f32) -> f32 {
        let n = &graph.nodes[node];
        let Some(k) = n.kind.input(port) else { return default };
        match graph.link_into(node, k) {
            Some(l) if l.ty == PortType::Control => self.out[l.from.0][l.from.1] * l.scale + l.offset,
            _ => n.f32(port, default),
        }
    }

    /// one frame: `lead` = how many samples the audio callback has rendered past the heard moment (the FFT looks
    /// back), `tempo` = the clock at the heard moment
    pub fn update(&mut self, graph: &Graph, dt: f32, lead: usize, tempo: &tempo::Now) {
        for (node, a) in &mut self.analyzers {
            self.out[*node].copy_from_slice(&a.update(dt, lead));
        }
        self.tempo = *tempo;
        if let Some(i) = graph.first(Kind::Tempo) {
            let p = &tempo.pos;
            // pulse: 1 on the beat, gone by its middle (0 while stopped)
            let pulse = if tempo.running { (1.0 - p.phase).powi(6) } else { 0.0 };
            let v = [tempo.bpm, p.beat, p.bar as f64, p.phase, p.bar_phase, pulse];
            for (o, x) in self.out[i].iter_mut().zip(v) {
                *o = x as f32;
            }
        }
        for &i in &graph.order {
            if graph.nodes[i].kind == Kind::Lfo {
                let rate = self.input(graph, i, "rate", 0.25);
                let depth = self.input(graph, i, "depth", 1.0);
                self.phase[i] = (self.phase[i] + rate * dt).fract();
                let wave = match graph.nodes[i].str("shape").unwrap_or("sine") {
                    "saw" => self.phase[i] * 2.0 - 1.0,
                    "square" => if self.phase[i] < 0.5 { 1.0 } else { -1.0 },
                    _ => (self.phase[i] * std::f32::consts::TAU).sin(),
                };
                self.out[i][0] = (0.5 + 0.5 * wave) * depth;
            }
        }
        for (node, port, p) in &self.params {
            let name = graph.nodes[*node].kind.inputs()[*port].name;
            p.set(self.input(graph, *node, name, p.get()));
        }
    }

    /// the first FFT node's values (for the HUD)
    pub fn fft(&self) -> Option<[f32; 4]> {
        self.analyzers.first().map(|(_, a)| a.values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Engine;
    use crate::scene::Scene;

    #[test]
    fn links_map_lfos_into_video_and_audio_inputs() {
        let g = Graph::build(
            &Scene::parse(
                "[[node]]\nid=\"l\"\nkind=\"lfo\"\nrate=0.25\nshape=\"square\"\n\
                 [[node]]\nid=\"p\"\nkind=\"test_pattern\"\nzoom=3\n[[node]]\nid=\"o\"\nkind=\"osc\"\n\
                 [[link]]\nfrom=\"l\"\nto=\"p.speed\"\nscale=2\noffset=1\n[[link]]\nfrom=\"l\"\nto=\"o.freq\"\nscale=100\n",
            )
            .unwrap(),
        )
        .unwrap();
        let (_, mut h) = Engine::build(&g, 48_000.0, 1.0).unwrap();
        let freq = h.params[0].2.clone();
        let mut c = Controls::new(&g, Some(&mut h));
        c.update(&g, 0.1, 0, &tempo::Now::default());
        assert_eq!(c.input(&g, 1, "speed", 0.0), 3.0); // square high = 1 → 1 * 2 + 1
        assert_eq!(c.input(&g, 1, "zoom", 1.0), 3.0); // unlinked: the node's param
        assert_eq!(c.input(&g, 1, "hue", 0.5), 0.5); // neither: the default
        assert_eq!(freq.get(), 100.0);
        c.update(&g, 2.0, 0, &tempo::Now::default()); // phase 0.025 + 0.5 → the low half
        assert_eq!(c.input(&g, 1, "speed", 0.0), 1.0);
    }

    #[test]
    fn the_tempo_node_outputs_the_clock() {
        let g = Graph::build(
            &Scene::parse("[[node]]
id=\"t\"
kind=\"tempo\"
[[node]]
id=\"p\"
kind=\"test_pattern\"
[[link]]
from=\"t.pulse\"
to=\"p.pulse\"
")
                .unwrap(),
        )
        .unwrap();
        let mut c = Controls::new(&g, None);
        let now = tempo::Now { bpm: 96.0, pos: tempo::position(9.0, 4), per_bar: 4, running: true };
        c.update(&g, 0.016, 0, &now);
        assert_eq!(c.out[0], vec![96.0, 9.0, 2.0, 0.0, 0.25, 1.0]);
        assert_eq!(c.input(&g, 1, "pulse", 0.0), 1.0);
        c.update(&g, 0.016, 0, &tempo::Now { pos: tempo::position(9.5, 4), ..now });
        assert!(c.input(&g, 1, "pulse", 0.0) < 0.02);
    }
}
