//! The engine one worker runs (#184, P6 #320): deck-engine – the one engine of the universal core, the native deck's –
//! built from the scene for the device's channels, its control half stepped inside every 128-frame quantum, sent values
//! applied as the native deck's frame thread applies them (the face `Controls`: commands into the engine, the heard
//! block's values back), the rendered audio analysed for the page (deck-core's `Analyzer`, looking back by the output
//! lead). Safe Rust; lib.rs puts it behind the raw wasm exports. No allocation after `load` (tests/no_alloc.rs).
use crate::signals::{Outputs, Slot};
use crate::wire::{self, Mode, FRAME, MAX_OUTPUTS, MAX_VALUES, OUTPUT_COUNT, PEAKS, REDUCTION, VALUES_AT, VALUE_COUNT};
use deck_core::analysis::{Analyzer, SPECTRUM};
use deck_core::tempo::{clamp_bpm, Now};
use deck_engine::control::Controls;
use deck_engine::{tempo_config, Build, DeckConfig, Engine, Handles, Seams, Sequencing};
use deck_space::graph::{Graph, Kind, PortType};
use deck_space::rig::engine::carried;
use deck_space::scene::Scene;

/// One table row's place in the frame's values: its node, its inputs (`ins` of them) then its outputs from `at`, each
/// output audio (a block peak) or control (the heard block's value).
struct Row {
    node: usize,
    ins: usize,
    outs: Vec<PortType>,
    at: usize,
}

pub struct Core {
    rate: f32,
    channels: usize,
    graph: Graph,
    engine: Engine,
    handles: Handles,
    /// the native frame thread's face of the control half: sends and bases go to the engine through it, the heard
    /// block's values come back from its snapshot ring
    controls: Controls,
    rows: Vec<Row>,
    analyzer: Analyzer,
    /// the rendered frames, interleaved (`channels` per frame)
    audio: Vec<f32>,
    frame: Vec<f32>,
    /// each device channel's largest sample since the last analysis frame
    peaks: [f32; MAX_OUTPUTS],
    /// the scene's `[outputs]` (#211, signals.rs), if it has any
    outputs: Option<Outputs>,
    /// the device messages since the last `take_signals` (signal records) and the ones just handed out
    signals: Vec<u8>,
    signals_out: Vec<u8>,
}

/// room for the device messages of one analysis frame before a push would allocate
const SIGNALS: usize = 64 * 1024;
/// frames rendered at most per call before `render` grows its buffer (the worker asks for one quantum)
const RENDER: usize = 1024;

impl Core {
    /// A scene in the deck's TOML (the edge: text, once) for a device of `channels` channels (the destination's –
    /// 2 on a phone). Every kind must be one the browser's engine carries (`deck_space::rig::engine::ENGINE_KINDS`):
    /// `lower()` hands the others to the page before a rig gets here.
    pub fn load(scene_toml: &str, rate: f32, channels: usize) -> Result<Core, String> {
        let scene = Scene::parse(scene_toml).map_err(|e| format!("{e:#}"))?;
        if let Some(n) = scene.nodes.iter().find(|n| !carried(&n.kind)) {
            return Err(format!("node '{}': the browser's engine does not carry '{}' yet", n.id, n.kind));
        }
        let graph = Graph::build(&scene).map_err(|e| format!("{e:#}"))?;
        let seams = Seams::with_deck(DeckConfig::device(channels));
        let channels = seams.deck.channels;
        let (engine, mut handles) =
            Engine::build_seams(&graph, rate, 1.0, Build::detect(), Sequencing::InBlock, &seams).map_err(|e| format!("{e:#}"))?;
        let controls = Controls::new(&graph, Some(&mut handles));
        let mut at = 0;
        let rows = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(node, n)| {
                let row = Row { node, ins: n.kind.inputs().len(), outs: n.kind.outputs().iter().map(|p| p.ty).collect(), at };
                at += row.ins + row.outs.len();
                row
            })
            .collect();
        let outputs = Self::outputs_of(&scene, &graph)?;
        Ok(Core {
            rate,
            channels,
            engine,
            handles,
            controls,
            rows,
            analyzer: Analyzer::new(rate, 0.25),
            audio: vec![0.0; RENDER * channels],
            frame: vec![0.0; FRAME + at.min(MAX_VALUES)],
            peaks: [0.0; MAX_OUTPUTS],
            outputs,
            signals: Vec::with_capacity(SIGNALS),
            signals_out: Vec::with_capacity(SIGNALS),
            graph,
        })
    }

    /// the device channels the engine renders
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// The deck's tempo for a scene without a `tempo` node of its own (the browser's clock until the show clock, P8):
    /// the map from the first frame at `bpm`. False: the scene has its own clock.
    pub fn set_tempo(&mut self, bpm: f32) -> bool {
        if self.graph.first(Kind::Tempo).is_some() {
            return false;
        }
        let Ok(mut c) = tempo_config(&self.graph) else { return false };
        c.bpm = clamp_bpm(f64::from(bpm));
        self.handles.tempo.publish(&c.map(f64::from(self.rate)), c.click);
        true
    }

    /// The parameter paths a mapping may read: every node's `node.<input>`, its control outputs `node.<output>`, and
    /// `node` / `node.out` for its first output.
    fn paths(graph: &Graph) -> Vec<(String, Slot)> {
        let mut out = Vec::new();
        for (i, n) in graph.nodes.iter().enumerate() {
            for (p, port) in n.kind.inputs().iter().enumerate() {
                out.push((format!("{}.{}", n.id, port.name), Slot::In(i, p)));
            }
            out.push((n.id.clone(), Slot::Out(i, 0)));
            for (k, port) in n.kind.outputs().iter().enumerate() {
                if port.name != "out" && port.ty == PortType::Control {
                    out.push((format!("{}.{}", n.id, port.name), Slot::Out(i, k)));
                }
            }
            out.push((format!("{}.out", n.id), Slot::Out(i, 0)));
        }
        out
    }

    /// The scene's `[outputs]` table checked against this graph's paths (profile files are not available in the
    /// browser – the built-in ones are); a wrong table is an error that lists every problem, as on the native deck.
    fn outputs_of(scene: &Scene, graph: &Graph) -> Result<Option<Outputs>, String> {
        let Some(table) = scene.outputs.as_ref() else {
            return Ok(None);
        };
        let paths = Self::paths(graph);
        let known: Vec<String> = paths.iter().map(|p| p.0.clone()).collect();
        let load = |name: &str| -> Result<String, String> { Err(format!("{name}: the browser has no profile files")) };
        let plan = deck_outputs::parse(table, Some(&known), &load).map_err(|errs| format!("the scene's [outputs]:\n  {}", errs.join("\n  ")))?;
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

    /// A device channel's peak so far: `block` is what channel `channel` played.
    pub fn meter(&mut self, channel: usize, block: impl Iterator<Item = f32>) {
        if let Some(p) = self.peaks.get_mut(channel) {
            *p = block.fold(*p, |m, x| m.max(x.abs()));
        }
    }

    /// The load table, the one JSON that leaves the core: `{"channels":n,"nodes":[{"id","kind","ports":[…],"outs":[…],
    /// "at":k}],"notes":[…]}` – the host resolves `node.port` names to the indices of the wire records with it, and reads
    /// a path's value in the analysis frame at `at` (the inputs first, then the outputs; -1 = past the frame's room).
    /// `notes`: what the build did with the device (a fold, an outlet without channels) in words.
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
        let list = |names: &mut dyn Iterator<Item = &str>| names.map(q).collect::<Vec<_>>().join(",");
        let nodes: Vec<String> = self
            .graph
            .nodes
            .iter()
            .zip(&self.rows)
            .map(|(n, r)| {
                let at = if r.at + r.ins + r.outs.len() <= MAX_VALUES { r.at as i64 } else { -1 };
                format!(
                    "{{\"id\":{},\"kind\":{},\"ports\":[{}],\"outs\":[{}],\"at\":{at}}}",
                    q(&n.id),
                    q(n.kind.name()),
                    list(&mut n.kind.inputs().iter().map(|p| p.name)),
                    list(&mut n.kind.outputs().iter().map(|p| p.name))
                )
            })
            .collect();
        let notes = list(&mut self.handles.notes.iter().map(String::as_str));
        format!("{{\"channels\":{},\"nodes\":[{}],\"notes\":[{notes}]}}", self.channels, nodes.join(","))
    }

    /// Applies the records of `bytes`; returns how many.
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
    /// the node's last input are dropped (the node maps what it has, #189). The engine applies it at its next quantum.
    pub fn send(&mut self, mode: Mode, node: usize, port: usize, amount: f32, values: impl Iterator<Item = f32>) -> Result<(), String> {
        let Some(row) = self.rows.get(node) else { return Err(format!("wire: no node {node}")) };
        let amount = mode.amount(amount);
        for (k, v) in values.enumerate().take(row.ins.saturating_sub(port)) {
            if mode == Mode::Base {
                self.controls.set_base(node, port + k, v);
            } else {
                self.controls.send(node, port + k, v, amount);
            }
        }
        Ok(())
    }

    /// `frames` frames of the device channels, interleaved, after the engine's master, limiter and fold (#188 – never
    /// over 1, no hard clip); the main pair's mean goes to the analyzer, every channel to its peak.
    pub fn render(&mut self, frames: usize) -> &[f32] {
        let ch = self.channels;
        let len = frames * ch;
        if self.audio.len() < len {
            self.audio.resize(len, 0.0);
        }
        self.engine.render(&mut self.audio[..len], ch, |x| x);
        let main = ch.min(2);
        let k = 1.0 / main as f32;
        for f in self.audio[..len].chunks_exact(ch) {
            self.analyzer.push(f[..main].iter().sum::<f32>() * k);
        }
        for c in 0..ch.min(MAX_OUTPUTS) {
            let p = &mut self.peaks[c];
            *p = self.audio[c..len].iter().step_by(ch).fold(*p, |m, x| m.max(x.abs()));
        }
        // the scene's outputs, on the audio clock: the control values as the last quantum left them
        if let Some(o) = self.outputs.as_mut() {
            let control = self.engine.control();
            let read = |s: Slot| match s {
                Slot::In(n, p) => control.input_at(n, p).unwrap_or(0.0),
                Slot::Out(n, k) => control.outputs(n).get(k).copied().unwrap_or(0.0),
            };
            o.advance(frames as f64 / f64::from(self.rate), read, &mut self.signals);
        }
        &self.audio[..len]
    }

    /// The analysis frame (wire.rs: level, bass, mid, high, the spectrum, the limiter's gain reduction in dB since the
    /// last frame, the device channels' peaks, then every table row's values) of what the speaker plays now when `lead`
    /// is the ring's fill: the window ends `lead` frames before the newest rendered one, and the control values are the
    /// heard block's (the snapshot ring, plan 3.3). An audio output's value is its newest block's peak (built for a scene
    /// with a `space` node – the being's hoses).
    pub fn analyse(&mut self, dt: f32, lead: usize) -> &[f32] {
        let bands = self.analyzer.update(dt, lead);
        self.frame[..4].copy_from_slice(&bands);
        self.analyzer.spectrum(&mut self.frame[4..4 + SPECTRUM]);
        self.frame[REDUCTION] = self.handles.take_reduction();
        self.frame[OUTPUT_COUNT] = self.channels as f32;
        self.frame[PEAKS..PEAKS + MAX_OUTPUTS].copy_from_slice(&self.peaks);
        self.peaks = [0.0; MAX_OUTPUTS];
        self.controls.update(dt, lead, &Now::default());
        let room = self.frame.len() - FRAME;
        self.frame[VALUE_COUNT] = room as f32;
        let values = &mut self.frame[VALUES_AT..];
        let meters = self.handles.meters.as_deref();
        for r in &self.rows {
            if r.at + r.ins + r.outs.len() > room {
                break;
            }
            for p in 0..r.ins {
                values[r.at + p] = self.controls.input_at(r.node, p).unwrap_or(f32::NAN);
            }
            let control = self.controls.outputs(r.node);
            for (k, ty) in r.outs.iter().enumerate() {
                values[r.at + r.ins + k] = match ty {
                    PortType::Control => control.get(k).copied().unwrap_or(0.0),
                    _ => meters.map_or(0.0, |m| m.level(r.node, k)),
                };
            }
        }
        &self.frame
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
