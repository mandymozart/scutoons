//! The control domain, evaluated once per frame: the FFT analyzers, the LFOs, then every control link mapped
//! (`value * scale + offset`) into its input – video nodes read theirs with `input()`, audio nodes get theirs as Params.
use crate::audio::{Handles, Param};
use crate::fft::Analyzer;
use crate::graph::{Graph, Kind, PortType};

pub struct Controls {
    /// per node, per output port: this frame's value
    pub out: Vec<Vec<f32>>,
    phase: Vec<f32>,
    analyzers: Vec<(usize, Analyzer)>,
    params: Vec<(usize, usize, Param)>,
}

impl Controls {
    pub fn new(graph: &Graph, audio: Option<&mut Handles>) -> Controls {
        let mut c = Controls {
            out: graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(),
            phase: vec![0.0; graph.nodes.len()],
            analyzers: Vec::new(),
            params: Vec::new(),
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

    pub fn update(&mut self, graph: &Graph, dt: f32) {
        for (node, a) in &mut self.analyzers {
            self.out[*node].copy_from_slice(&a.update(dt));
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
        c.update(&g, 0.1);
        assert_eq!(c.input(&g, 1, "speed", 0.0), 3.0); // square high = 1 → 1 * 2 + 1
        assert_eq!(c.input(&g, 1, "zoom", 1.0), 3.0); // unlinked: the node's param
        assert_eq!(c.input(&g, 1, "hue", 0.5), 0.5); // neither: the default
        assert_eq!(freq.get(), 100.0);
        c.update(&g, 2.0); // phase 0.025 + 0.5 → the low half
        assert_eq!(c.input(&g, 1, "speed", 0.0), 1.0);
    }
}
