//! The engine one worker runs (#184): a scene on deck-core's batch graph (#183), its audio rendered block by block,
//! the rendered audio analysed (deck-core's FFT `Analyzer`, looking back by the output lead), and sent values applied
//! from wire records. Safe Rust; lib.rs puts it behind the raw wasm exports.
use crate::wire::{self, Mode, FRAME};
use deck_core::analysis::{Analyzer, SPECTRUM};
use deck_core::batch::BatchGraph;
use deck_core::SceneDef;

pub struct Core {
    graph: BatchGraph,
    analyzer: Analyzer,
    audio: Vec<f32>,
    frame: Vec<f32>,
}

impl Core {
    /// A scene in deck/stage's TOML format (the edge: text, once). Only the batch kinds (lfo, smooth, mul, add, osc, fm).
    pub fn load(scene_toml: &str, rate: f32) -> Result<Core, String> {
        let scene = SceneDef::parse(scene_toml)?;
        let graph = BatchGraph::build(&scene, rate)?;
        Ok(Core { graph, analyzer: Analyzer::new(rate, 0.25), audio: Vec::new(), frame: vec![0.0; FRAME] })
    }

    /// The load table, the one JSON that leaves the core: `{"nodes":[{"id","kind","ports":[…]}]}` – the host resolves
    /// `node.port` names to the indices of the wire records with it.
    pub fn table_json(&self) -> String {
        let q = |s: &str| {
            let mut o = String::with_capacity(s.len() + 2);
            o.push('"');
            for c in s.chars() {
                match c {
                    '"' => o.push_str("\\\""),
                    '\\' => o.push_str("\\\\"),
                    c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                    c => o.push(c),
                }
            }
            o.push('"');
            o
        };
        let nodes: Vec<String> = self
            .graph
            .nodes()
            .map(|(id, kind, ports)| {
                let ports: Vec<String> = ports.iter().map(|p| q(p)).collect();
                format!("{{\"id\":{},\"kind\":{},\"ports\":[{}]}}", q(id), q(kind), ports.join(","))
            })
            .collect();
        format!("{{\"nodes\":[{}]}}", nodes.join(","))
    }

    /// Applies the records of `bytes` (`send` each); returns how many.
    pub fn apply(&mut self, bytes: &[u8]) -> Result<usize, String> {
        let mut n = 0;
        for r in wire::records(bytes) {
            let r = r.map_err(|e| e.to_string())?;
            self.send(r.mode, r.node as usize, r.port as usize, r.amount, r.values())?;
            n += 1;
        }
        Ok(n)
    }

    /// A float vector onto node `node`'s inputs: the components land on consecutive inputs from `port`, the ones past
    /// the node's last input are dropped (the node maps what it has, #189).
    pub fn send(&mut self, mode: Mode, node: usize, port: usize, amount: f32, values: impl Iterator<Item = f32>) -> Result<(), String> {
        let ports = self.graph.ports(node).map(<[_]>::len).ok_or_else(|| format!("wire: no node {node}"))?;
        let amount = mode.amount(amount);
        for (k, v) in values.enumerate().take(ports.saturating_sub(port)) {
            self.graph.send(node, port + k, v, amount);
        }
        Ok(())
    }

    /// One frame of the control domain.
    pub fn step(&mut self, dt: f32) {
        self.graph.step(dt);
    }

    /// `frames` samples of the scene's audio (mono); every sample also goes to the analyzer.
    pub fn render(&mut self, frames: usize) -> &[f32] {
        self.audio.resize(frames, 0.0);
        self.graph.render(&mut self.audio);
        for &x in &self.audio {
            self.analyzer.push(x);
        }
        &self.audio
    }

    /// The analysis frame (wire.rs: level, bass, mid, high, the spectrum) of the window that ends `lead` samples before
    /// the newest rendered one – what the speaker plays now when `lead` is the ring's fill.
    pub fn analyse(&mut self, dt: f32, lead: usize) -> &[f32] {
        let bands = self.analyzer.update(dt, lead);
        self.frame[..4].copy_from_slice(&bands);
        self.analyzer.spectrum(&mut self.frame[4..4 + SPECTRUM]);
        &self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::encode_send;

    pub const SCENE: &str = "[[node]]\nid = \"wob\"\nkind = \"lfo\"\nrate = 0.0\ndepth = 1.0\nphase = 0.25\n\
        [[node]]\nid = \"voice\"\nkind = \"fm\"\nfreq = 110.0\nratio = 2.0\nindex = 0.0\ngain = 0.0\n";

    #[test]
    fn the_table_names_nodes_and_ports() {
        let c = Core::load(SCENE, 48_000.0).unwrap();
        assert_eq!(
            c.table_json(),
            r#"{"nodes":[{"id":"wob","kind":"lfo","ports":["rate","depth"]},{"id":"voice","kind":"fm","ports":["freq","ratio","index","gain"]}]}"#
        );
        let v: serde_json::Value = serde_json::from_str(&c.table_json()).unwrap();
        assert_eq!(v["nodes"][1]["ports"][3], "gain");
        let odd = Core::load("[[node]]\nid = \"a\\\"b\\\\c\"\nkind = \"add\"\n", 48_000.0).unwrap();
        let v: serde_json::Value = serde_json::from_str(&odd.table_json()).unwrap();
        assert_eq!(v["nodes"][0]["id"], "a\"b\\c");
    }

    #[test]
    fn sent_vectors_reach_the_sound_and_the_analysis() {
        let mut c = Core::load(SCENE, 48_000.0).unwrap();
        c.step(1.0 / 60.0);
        assert!(c.render(128).iter().all(|x| *x == 0.0), "gain 0: silence");
        // a 4-vector onto the voice: 1500 Hz, ratio 1, no FM, gain 0.5 – plus a 5th component it has no input for
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[1500.0, 1.0, 0.0, 0.5, 9.0]);
        assert_eq!(c.apply(&b), Ok(1));
        for _ in 0..8 {
            c.render(128);
        }
        for _ in 0..8 {
            c.analyse(1.0 / 60.0, 0); // the analyzer rises over a few frames
        }
        let f = c.analyse(1.0 / 60.0, 0).to_vec();
        assert_eq!(f.len(), FRAME);
        assert!(f[0] > 0.5, "level {}", f[0]);
        assert!(f[2] > 0.8 && f[2] > f[1] + 0.3, "1500 Hz is mid: {f:?}");
        let peak = (4..FRAME).max_by(|&i, &j| f[i].total_cmp(&f[j])).unwrap() - 4;
        assert_eq!(peak, 32, "1500 Hz / 46.875 Hz per bin");
        // release: gain back to 0 → silence
        b.clear();
        encode_send(&mut b, Mode::Release, 1, 3, 0.0, &[0.0]);
        c.apply(&b).unwrap();
        assert!(c.render(128).iter().all(|x| *x == 0.0));
    }

    #[test]
    fn bad_records_and_nodes_are_errors() {
        let mut c = Core::load(SCENE, 48_000.0).unwrap();
        let mut b = vec![];
        encode_send(&mut b, Mode::Blend, 7, 0, 0.5, &[1.0]);
        assert_eq!(c.apply(&b), Err("wire: no node 7".into()));
        assert_eq!(c.apply(&[1, 0, 0]), Err("wire: a truncated header at byte 0".into()));
        assert!(Core::load("[[node]]\nid = \"f\"\nkind = \"fft\"\n", 48_000.0).err().unwrap().contains("does not batch"));
    }
}
