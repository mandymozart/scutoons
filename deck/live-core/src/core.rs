//! The engine one worker runs (#184): a scene on deck-core's batch graph (#183), its audio rendered block by block,
//! the rendered audio analysed (deck-core's FFT `Analyzer`, looking back by the output lead), and sent values applied
//! from wire records. Safe Rust; lib.rs puts it behind the raw wasm exports.
use crate::wire::{self, Mode, FRAME};
use deck_core::analysis::{Analyzer, SPECTRUM};
use deck_core::batch::BatchGraph;
use deck_core::limiter::Limiter;
use deck_core::SceneDef;
use crate::walks::{self, Walk};

pub struct Core {
    graph: BatchGraph,
    /// the scene's walks (#236): their knobs are the load table's rows after the batch nodes
    walks: Vec<Walk>,
    /// this render's notes (frame offset, pitch, the walk), reused
    notes: Vec<(usize, f32, usize)>,
    scratch: Vec<(usize, f32)>,
    analyzer: Analyzer,
    /// the output's soft cap (#188), last before the worklet
    limiter: Limiter,
    audio: Vec<f32>,
    frame: Vec<f32>,
}

impl Core {
    /// A scene in deck/stage's TOML format (the edge: text, once). Only the batch kinds (lfo, smooth, mul, add, osc, fm),
    /// plus `[[walk]]`s that play a batch input (walks.rs).
    pub fn load(scene_toml: &str, rate: f32) -> Result<Core, String> {
        let scene = SceneDef::parse(scene_toml)?;
        let graph = BatchGraph::build(&scene, rate)?;
        let walks = walks::parse(scene_toml, &graph, rate)?;
        Ok(Core {
            graph,
            walks,
            notes: Vec::with_capacity(256),
            scratch: Vec::with_capacity(256),
            analyzer: Analyzer::new(rate, 0.25),
            limiter: Limiter::new(rate),
            audio: Vec::new(),
            frame: vec![0.0; FRAME],
        })
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
        let row = |id: &str, kind: &str, ports: &[&str]| {
            let ports: Vec<String> = ports.iter().map(|p| q(p)).collect();
            format!("{{\"id\":{},\"kind\":{},\"ports\":[{}]}}", q(id), q(kind), ports.join(","))
        };
        let nodes: Vec<String> = self
            .graph
            .nodes()
            .map(|(id, kind, ports)| row(id, kind, ports))
            .chain(self.walks.iter().map(|w| row(&w.id, "walk", &walks::PORTS)))
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
        let batch = self.graph.nodes().count();
        if node >= batch {
            // a walk's knob (its params are set, not blended)
            let w = self.walks.get_mut(node - batch).ok_or_else(|| format!("wire: no node {node}"))?;
            for (k, v) in values.enumerate().take(walks::PORTS.len().saturating_sub(port)) {
                w.set(port + k, v);
            }
            return Ok(());
        }
        let ports = self.graph.ports(node).map(<[_]>::len).ok_or_else(|| format!("wire: no node {node}"))?;
        let amount = mode.amount(amount);
        for (k, v) in values.enumerate().take(ports.saturating_sub(port)) {
            if mode == Mode::Base {
                self.graph.set_base(node, port + k, v);
            } else {
                self.graph.send(node, port + k, v, amount);
            }
        }
        Ok(())
    }

    /// One frame of the control domain.
    pub fn step(&mut self, dt: f32) {
        self.graph.step(dt);
    }

    /// `frames` samples of the scene's audio (mono) through the soft limiter (#188 – never over 1, no hard clip); every
    /// sample also goes to the analyzer.
    pub fn render(&mut self, frames: usize) -> &[f32] {
        self.audio.resize(frames, 0.0);
        // the walks' notes in these frames: the render is split at each, its target set to the note's pitch
        self.notes.clear();
        for (wi, w) in self.walks.iter_mut().enumerate() {
            self.scratch.clear();
            w.notes(frames, &mut self.scratch);
            self.notes.extend(self.scratch.iter().map(|&(f, p)| (f, p, wi)));
        }
        self.notes.sort_unstable_by_key(|n| n.0);
        let mut at = 0;
        for &(f, pitch, wi) in &self.notes {
            let f = f.min(frames);
            if f > at {
                self.graph.render(&mut self.audio[at..f]);
                at = f;
            }
            let (node, port) = self.walks[wi].target();
            self.graph.send(node, port, pitch, 1.0);
        }
        if at < frames {
            self.graph.render(&mut self.audio[at..]);
        }
        self.limiter.process_mono(&mut self.audio);
        for &x in &self.audio {
            self.analyzer.push(x);
        }
        &self.audio
    }

    /// The analysis frame (wire.rs: level, bass, mid, high, the spectrum, the limiter's gain reduction in dB since the
    /// last frame) of the window that ends `lead` samples before the newest rendered one – what the speaker plays now
    /// when `lead` is the ring's fill.
    pub fn analyse(&mut self, dt: f32, lead: usize) -> &[f32] {
        let bands = self.analyzer.update(dt, lead);
        self.frame[..4].copy_from_slice(&bands);
        self.analyzer.spectrum(&mut self.frame[4..4 + SPECTRUM]);
        self.frame[4 + SPECTRUM] = self.limiter.take_reduction();
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
        // the inputs glide in (#188): 40 blocks = 4 time constants
        for _ in 0..40 {
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
        assert!(c.render(128).iter().any(|x| x.abs() > 0.05), "it fades, no click");
        for _ in 0..200 {
            c.render(128);
        }
        assert!(c.render(128).iter().all(|x| x.abs() < 1e-4), "then silence");
    }

    /// #188: the output is limited (never over 1, the gain reduction in the analysis frame); a base record turns a
    /// linked input under its link
    #[test]
    fn overload_is_limited_and_bases_turn() {
        let scene = "[[node]]
id = \"a\"
kind = \"osc\"
freq = 110.0
gain = 1.0
[[node]]
id = \"b\"
kind = \"osc\"
freq = 165.0
gain = 1.0
\n            [[node]]
id = \"l\"
kind = \"lfo\"
rate = 0.0
depth = 1.0
phase = 0.25
[[node]]
id = \"m\"
kind = \"mul\"
b = 1.0
\n            [[link]]
from = \"l\"
to = \"m.a\"
scale = 2.0
";
        let mut c = Core::load(scene, 48_000.0).unwrap();
        let mut worst = 0.0f32;
        for _ in 0..100 {
            c.step(128.0 / 48_000.0);
            worst = c.render(128).iter().fold(worst, |w, x| w.max(x.abs()));
        }
        assert!(worst < 1.0 && worst > 0.8, "two sines at full gain, held under 1: {worst}");
        let f = c.analyse(1.0 / 60.0, 0);
        assert!(f[FRAME - 1] > 3.0, "the gain reduction in dB: {}", f[FRAME - 1]);
        let mut b = vec![];
        encode_send(&mut b, Mode::Base, 3, 0, 0.0, &[0.5]);
        assert_eq!(c.apply(&b), Ok(1));
        c.step(0.0);
        assert!((c.graph.value("m").unwrap() - 2.5).abs() < 1e-5, "the LFO's 1 · 2 on top of the base 0.5");
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

    /// #236: a walk plays the voice's freq – the same seed, the same samples; its knobs are table rows
    const WALK: &str = "[[node]]\nid = \"voice\"\nkind = \"fm\"\nfreq = 147.0\nratio = 1.5\nindex = 2.0\ngain = 0.2\n\
        [[walk]]\nid = \"melody\"\ntarget = \"voice.freq\"\nseed = 42\nspread = 1.0\nrate = 4.0\nbpm = 120.0\n";

    fn play(c: &mut Core, blocks: usize) -> Vec<f32> {
        let mut all = Vec::new();
        for _ in 0..blocks {
            c.step(128.0 / 48_000.0);
            all.extend_from_slice(c.render(128));
        }
        all
    }

    #[test]
    fn a_walk_plays_its_voice_and_its_knobs_are_table_rows() {
        let mut a = Core::load(WALK, 48_000.0).unwrap();
        let v: serde_json::Value = serde_json::from_str(&a.table_json()).unwrap();
        assert_eq!(v["nodes"][1]["id"], "melody");
        assert_eq!(v["nodes"][1]["kind"], "walk");
        assert_eq!(v["nodes"][1]["ports"], serde_json::json!(["spread", "rate", "glide"]));
        let one = play(&mut a, 750);
        assert!(one.iter().any(|x| x.abs() > 0.01), "it sounds");
        let two = play(&mut Core::load(WALK, 48_000.0).unwrap(), 750);
        assert_eq!(one, two, "one seed, the same samples");
        // the spread to 0 through a wire record (node 1 = the walk, port 0 = spread): the base held, another sound
        let mut held = Core::load(WALK, 48_000.0).unwrap();
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[0.0]);
        assert_eq!(held.apply(&b), Ok(1));
        assert_ne!(play(&mut held, 750), one, "spread 0 holds the base");
        assert!(Core::load(&WALK.replace("voice.freq", "voice.nope"), 48_000.0).is_err(), "a target must be an input");
    }

}
