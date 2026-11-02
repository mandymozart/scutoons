//! The engine one worker runs (#184): a scene on deck-core's batch graph (#183), its audio rendered block by block,
//! the rendered audio analysed (deck-core's FFT `Analyzer`, looking back by the output lead), and sent values applied
//! from wire records. Safe Rust; lib.rs puts it behind the raw wasm exports.
use crate::signals::{Outputs, Slot};
use crate::walks::{self, Walk};
use crate::wire::{self, Mode, FRAME, MAX_OUTPUTS, OUTPUT_COUNT, PEAKS, REDUCTION};
use deck_core::analysis::{Analyzer, SPECTRUM};
use deck_core::batch::BatchGraph;
use deck_core::limiter::Limiter;
use deck_core::SceneDef;

/// the longest sub-block (frames) the render runs while a walk glides
const GLIDE_BLOCK: usize = 64;

/// The audio outputs the engine renders (#282): the master is mono today and plays on both channels of the speaker,
/// so two equal peaks. A multi-channel engine raises this (≤ `MAX_OUTPUTS`) and feeds `meter` per channel.
pub const OUTPUTS: usize = 2;

pub struct Core {
    rate: f32,
    graph: BatchGraph,
    /// the scene's walks (#236): their knobs are the load table's rows after the batch nodes
    walks: Vec<Walk>,
    /// this render's notes (frame offset, pitch, glide frames, the walk), reused
    notes: Vec<(usize, f32, u32, usize)>,
    scratch: Vec<(usize, f32, u32)>,
    analyzer: Analyzer,
    /// the output's soft cap (#188), last before the worklet
    limiter: Limiter,
    audio: Vec<f32>,
    frame: Vec<f32>,
    /// each output's largest sample since the last analysis frame
    peaks: [f32; MAX_OUTPUTS],
    /// the scene's `[outputs]` (#211, signals.rs), if it has any
    outputs: Option<Outputs>,
    /// the device messages since the last `take_signals` (signal records) and the ones just handed out
    signals: Vec<u8>,
    signals_out: Vec<u8>,
}

impl Core {
    /// A scene in deck/native's TOML format (the edge: text, once). Only the batch kinds (lfo, smooth, mul, add, osc, fm),
    /// plus `[[walk]]`s that play a batch input (walks.rs).
    pub fn load(scene_toml: &str, rate: f32) -> Result<Core, String> {
        let scene = SceneDef::parse(scene_toml)?;
        let graph = BatchGraph::build(&scene, rate)?;
        let walks = walks::parse(scene_toml, &graph, rate)?;
        let outputs = Self::outputs_of(scene_toml, &graph)?;
        Ok(Core {
            rate,
            graph,
            walks,
            notes: Vec::with_capacity(256),
            scratch: Vec::with_capacity(256),
            analyzer: Analyzer::new(rate, 0.25),
            limiter: Limiter::new(rate),
            audio: Vec::new(),
            frame: vec![0.0; FRAME],
            peaks: [0.0; MAX_OUTPUTS],
            outputs,
            signals: Vec::new(),
            signals_out: Vec::new(),
        })
    }

    /// The parameter paths a mapping may read: every node's `node.<input>` and its output (`node`, `node.out`).
    fn paths(graph: &BatchGraph) -> Vec<(String, Slot)> {
        let mut out = Vec::new();
        for (i, (id, _, ports)) in graph.nodes().enumerate() {
            for (p, name) in ports.iter().enumerate() {
                out.push((format!("{id}.{name}"), Slot::In(i, p)));
            }
            out.push((id.to_owned(), Slot::Out(i)));
            out.push((format!("{id}.out"), Slot::Out(i)));
        }
        out
    }

    /// The scene's `[outputs]` table checked against this graph's paths (profile files are not available in the
    /// browser – the built-in ones are); a wrong table is an error that lists every problem, as on the stage.
    fn outputs_of(scene_toml: &str, graph: &BatchGraph) -> Result<Option<Outputs>, String> {
        let doc: toml::Table = toml::from_str(scene_toml).map_err(|e| format!("the scene: {e}"))?;
        let Some(table) = doc.get("outputs").and_then(toml::Value::as_table) else {
            return Ok(None);
        };
        let paths = Self::paths(graph);
        let known: Vec<String> = paths.iter().map(|p| p.0.clone()).collect();
        let load = |name: &str| -> Result<String, String> { Err(format!("{name}: the browser has no profile files")) };
        let plan = deck_outputs::parse(table, Some(&known), &load).map_err(|errs| {
            format!(
                "the scene's [outputs]:
  {}",
                errs.join(
                    "
  "
                )
            )
        })?;
        let slots = plan
            .paths
            .iter()
            .map(|p| paths.iter().find(|x| &x.0 == p).map(|x| x.1).ok_or_else(|| format!("outputs: no path '{p}'")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Outputs::new(plan, slots))
    }

    /// The device messages the scene's `[outputs]` sent since the last call, as signal records (signals.rs) – valid
    /// until the next call.
    pub fn take_signals(&mut self) -> &[u8] {
        self.signals_out.clear();
        std::mem::swap(&mut self.signals, &mut self.signals_out);
        &self.signals_out
    }

    /// Where the records `take_signals` just returned are.
    pub fn signals_ptr(&self) -> *const u8 {
        self.signals_out.as_ptr()
    }

    /// An output's peak so far: `block` is what channel `channel` played.
    pub fn meter(&mut self, channel: usize, block: &[f32]) {
        if let Some(p) = self.peaks.get_mut(channel) {
            *p = block.iter().fold(*p, |m, x| m.max(x.abs()));
        }
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
        let nodes: Vec<String> =
            self.graph.nodes().map(|(id, kind, ports)| row(id, kind, ports)).chain(self.walks.iter().map(|w| row(&w.id, w.kind, w.ports))).collect();
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
            for (k, v) in values.enumerate().take(w.ports.len().saturating_sub(port)) {
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
        // the walks' notes in these frames: the render is split at each, its target glided to the note's pitch – while
        // a glide runs also every GLIDE_BLOCK frames, the target set at the start of each sub-block (the batch graph
        // ramps within a block)
        self.notes.clear();
        for (wi, w) in self.walks.iter_mut().enumerate() {
            self.scratch.clear();
            w.notes(frames, &mut self.scratch);
            self.notes.extend(self.scratch.iter().map(|&(f, p, g)| (f, p, g, wi)));
        }
        self.notes.sort_unstable_by_key(|n| n.0);
        let (mut at, mut next) = (0, 0);
        loop {
            while next < self.notes.len() && self.notes[next].0.min(frames) <= at {
                let (_, pitch, glide, wi) = self.notes[next];
                self.walks[wi].begin(pitch, glide);
                next += 1;
            }
            for w in &mut self.walks {
                if let Some(freq) = w.take() {
                    let (node, port) = w.target();
                    self.graph.send(node, port, freq, 1.0);
                }
            }
            if at >= frames {
                break;
            }
            let mut end = self.notes.get(next).map_or(frames, |n| n.0.min(frames));
            if self.walks.iter().any(Walk::gliding) {
                end = end.min(at + GLIDE_BLOCK);
            }
            self.graph.render(&mut self.audio[at..end]);
            for w in &mut self.walks {
                w.advance(end - at);
            }
            at = end;
        }
        self.limiter.process_mono(&mut self.audio);
        for &x in &self.audio {
            self.analyzer.push(x);
        }
        // the mono master plays on every output
        let master = std::mem::take(&mut self.audio);
        for ch in 0..OUTPUTS {
            self.meter(ch, &master);
        }
        self.audio = master;
        // the scene's outputs, on the audio clock: the control values as the last step left them
        if let Some(o) = self.outputs.as_mut() {
            let graph = &self.graph;
            let read = |s: Slot| match s {
                Slot::In(n, p) => graph.input(n, p).unwrap_or(0.0),
                Slot::Out(n) => graph.node_value(n).unwrap_or(0.0),
            };
            o.advance(frames as f64 / f64::from(self.rate), read, &mut self.signals);
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
        self.frame[REDUCTION] = self.limiter.take_reduction();
        self.frame[OUTPUT_COUNT] = OUTPUTS as f32;
        self.frame[PEAKS..PEAKS + MAX_OUTPUTS].copy_from_slice(&self.peaks);
        self.peaks = [0.0; MAX_OUTPUTS];
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
        let peak = (4..4 + SPECTRUM).max_by(|&i, &j| f[i].total_cmp(&f[j])).unwrap() - 4;
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
        assert!(f[REDUCTION] > 3.0, "the gain reduction in dB: {}", f[FRAME - 1]);
        let mut b = vec![];
        encode_send(&mut b, Mode::Base, 3, 0, 0.0, &[0.5]);
        assert_eq!(c.apply(&b), Ok(1));
        c.step(0.0);
        assert!((c.graph.value("m").unwrap() - 2.5).abs() < 1e-5, "the LFO's 1 · 2 on top of the base 0.5");
    }

    /// #282: the frame carries the output count and one peak per output (since the last frame, then reset)
    #[test]
    fn the_frame_reports_a_peak_per_output() {
        let mut c = Core::load(SCENE, 48_000.0).unwrap();
        c.step(1.0 / 60.0);
        c.render(128);
        let f = c.analyse(1.0 / 60.0, 0).to_vec();
        assert_eq!(f[OUTPUT_COUNT], OUTPUTS as f32);
        assert_eq!(&f[PEAKS..PEAKS + MAX_OUTPUTS], &[0.0; MAX_OUTPUTS], "silence: nothing on any output");
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[1500.0, 1.0, 0.0, 0.5]);
        c.apply(&b).unwrap();
        for _ in 0..40 {
            c.render(128);
        }
        let f = c.analyse(1.0 / 60.0, 0).to_vec();
        let peaks = &f[PEAKS..PEAKS + MAX_OUTPUTS];
        assert!(peaks[0] > 0.3 && peaks[0] <= 1.0, "gain 0.5 on a sine: {peaks:?}");
        assert_eq!(peaks[0], peaks[1], "the mono master plays on both");
        assert!(peaks[OUTPUTS..].iter().all(|p| *p == 0.0), "no peaks past the count");
        // "since the last frame": a frame right after, with no render between, starts over
        let f = c.analyse(1.0 / 60.0, 0).to_vec();
        assert!(f[PEAKS..PEAKS + MAX_OUTPUTS].iter().all(|p| *p == 0.0));
        // a multi-channel engine meters its channels one by one
        c.meter(1, &[0.0, -0.75, 0.25]);
        let f = c.analyse(1.0 / 60.0, 0).to_vec();
        assert_eq!((f[PEAKS], f[PEAKS + 1]), (0.0, 0.75));
        assert_eq!(f.len(), FRAME);
    }

    /// #282: a scene's [outputs] run on the audio clock and their messages leave as signal records
    const OUTPUTS_SCENE: &str = "[[node]]
id = \"wob\"
kind = \"lfo\"
rate = 0.0
depth = 1.0
phase = 0.25
        [[node]]
id = \"knob\"
kind = \"add\"
a = 0.0
b = 0.0
        [outputs]
dmx = \"auto\"
midi = \"x\"
        [[outputs.fixture]]
id = \"strobe\"
profile = \"strobe\"
address = 1
        [[outputs.map]]
from = \"knob.a\"
to = [{ dmx = 20 }, { dmx = 21 }, { cc = 74, channel = 1 }, { cc = 20, port = \"conductor\" }]
";

    /// the records of `bytes`: (kind, flags, ch, at, payload)
    fn walk(bytes: &[u8]) -> Vec<(u8, u8, u16, u32, Vec<u8>)> {
        let (mut at, mut out) = (0, vec![]);
        while at < bytes.len() {
            let n = u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
            let ch = u16::from_le_bytes([bytes[at + 4], bytes[at + 5]]);
            let ms = u32::from_le_bytes(bytes[at + 8..at + 12].try_into().unwrap());
            out.push((bytes[at], bytes[at + 1], ch, ms, bytes[at + 12..at + 12 + n].to_vec()));
            at += 12 + n.div_ceil(4) * 4;
        }
        out
    }

    #[test]
    fn a_scenes_outputs_leave_as_signal_records() {
        let mut c = Core::load(OUTPUTS_SCENE, 48_000.0).unwrap();
        // a knob forced to 1: node 1 = "knob", port 0 = "a"
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[1.0]);
        c.apply(&b).unwrap();
        let mut got = vec![];
        for _ in 0..400 {
            c.step(128.0 / 48_000.0);
            c.render(128);
            got.extend_from_slice(c.take_signals());
        }
        assert!(c.take_signals().is_empty(), "taken once");
        let rec = walk(&got);
        // the rest of the strobe fixture first (it has the DMX at 1…), the knob's two channels as one range
        let range = rec.iter().rev().find(|r| r.0 == 2 && r.2 == 20).expect("DMX 20…21 as one range");
        assert_eq!(range.4, vec![255, 255]);
        let cc = |conductor: u8| rec.iter().rev().find(|r| r.0 == 1 && r.1 == conductor).map(|r| r.4.clone());
        assert_eq!(cc(0), Some(vec![0xB0, 74, 127]), "the deck's MIDI out");
        assert_eq!(cc(1), Some(vec![0xB0, 20, 127]), "the conductor");
        // 400 blocks = 1.07 s: the stamps run on the audio clock, none past it
        assert!(rec.iter().all(|r| r.3 <= 1070), "{:?}", rec.iter().map(|r| r.3).max());
        assert!(rec.windows(2).all(|w| w[0].3 <= w[1].3), "in time order");
        // it follows the knob: forced back to 0 → the values fall
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[0.0]);
        c.apply(&b).unwrap();
        let mut got = vec![];
        for _ in 0..200 {
            c.step(128.0 / 48_000.0);
            c.render(128);
            got.extend_from_slice(c.take_signals());
        }
        let rec = walk(&got);
        assert_eq!(rec.iter().rev().find(|r| r.0 == 2 && r.2 == 20).unwrap().4, vec![0, 0]);
        // a scene without [outputs] sends nothing
        let mut quiet = Core::load(SCENE, 48_000.0).unwrap();
        for _ in 0..50 {
            quiet.render(128);
        }
        assert!(quiet.take_signals().is_empty());
    }

    #[test]
    fn outputs_name_real_paths_or_the_load_fails_listing_them() {
        let bad = OUTPUTS_SCENE.replace("knob.a", "knob.nope");
        let e = Core::load(&bad, 48_000.0).err().unwrap();
        assert!(e.starts_with("the scene's [outputs]:") && e.contains("knob.nope"), "{e}");
        let profile = OUTPUTS_SCENE.replace("profile = \"strobe\"", "profile = \"fixtures/mine.toml\"");
        assert!(Core::load(&profile, 48_000.0).err().unwrap().contains("no profile files"));
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

    /// #236: a note's glide shapes the sound (glide 0 jumps, 1 glides the whole note) and the same settings, the same
    /// samples
    #[test]
    fn a_glide_changes_the_sound_and_the_same_settings_sound_the_same() {
        let scene = |glide: f32| format!("{WALK}glide = {glide}\n");
        let with = |glide: f32| play(&mut Core::load(&scene(glide), 48_000.0).unwrap(), 750);
        let (jump, slide) = (with(0.0), with(1.0));
        assert!(jump.iter().any(|x| x.abs() > 0.01) && slide.iter().any(|x| x.abs() > 0.01));
        assert_ne!(jump, slide, "a glide is another sound than a jump");
        assert_eq!(slide, with(1.0), "two cores, the same settings, the same samples");
        assert_eq!(jump, with(0.0));
    }

    /// #236: an `[[events]]` host – an arp on a walk – plays its voice; its knobs a–d are table rows
    const EVENTS: &str = "[[node]]\nid = \"voice\"\nkind = \"fm\"\nfreq = 147.0\nratio = 2.0\nindex = 1.0\ngain = 0.2\n\
        [[events]]\nid = \"arp\"\ntarget = \"voice.freq\"\nbpm = 120.0\nout = \"arp.notes\"\nknobs = [\"walk.spread\", \"arp.rate\"]\n\
        generator = [{ id = \"walk\", kind = \"random_walk\", seed = 5, rate = 0.5, spread = 0.6 }, \
        { id = \"arp\", kind = \"arp\", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = \"updown\", rate = 4.0 }]\n\
        edge = [{ from = \"walk.notes\", to = \"arp.notes\" }]\n";

    #[test]
    fn an_events_host_plays_its_patch_and_its_knobs_are_table_rows() {
        let mut a = Core::load(EVENTS, 48_000.0).unwrap();
        let v: serde_json::Value = serde_json::from_str(&a.table_json()).unwrap();
        assert_eq!((v["nodes"][1]["id"].as_str(), v["nodes"][1]["kind"].as_str()), (Some("arp"), Some("events")));
        assert_eq!(v["nodes"][1]["ports"], serde_json::json!(["a", "b", "c", "d"]));
        let one = play(&mut a, 750);
        assert!(one.iter().any(|x| x.abs() > 0.01), "it sounds");
        assert_eq!(one, play(&mut Core::load(EVENTS, 48_000.0).unwrap(), 750), "one seed, the same samples");
        // knob b (the arp's rate) through a record; NaN on a (unset) leaves the patch's spread
        let mut fast = Core::load(EVENTS, 48_000.0).unwrap();
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[f32::NAN, 8.0]);
        assert_eq!(fast.apply(&b), Ok(1));
        assert_ne!(play(&mut fast, 750), one, "knob b: eight steps a beat");
        // a starting value in the entry: the same as the record
        let started = EVENTS.replace("bpm = 120.0\n", "bpm = 120.0\nb = 8.0\n");
        let mut by_record = Core::load(EVENTS, 48_000.0).unwrap();
        by_record.apply(&b).unwrap();
        assert_eq!(play(&mut Core::load(&started, 48_000.0).unwrap(), 750), play(&mut by_record, 750));
        assert!(Core::load(&EVENTS.replace("updown", "sideways"), 48_000.0).err().is_some_and(|e| e.contains("unknown mode")));
    }
}
