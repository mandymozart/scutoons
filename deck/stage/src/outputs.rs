//! The deck's parameters as outputs (#211, feature `outputs`): the scene's `[outputs]` (deck-outputs config.rs) checked
//! against this graph's paths, the adapters opened in-process – the bridge's own crates: the USB-DMX widget
//! (adapters-dmx-usbpro) and a MIDI out port (adapters-midi) – and deck-outputs' runner on its own thread. The frame
//! thread hands it the values each frame (`update`, atomics) and forwards the conductor's MIDI to the relay link.
//! The strobe's clock is the tempo clock (#181): the tempo map through the time base's line, on the monotonic clock.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use deck_outputs::host::{self, ClockSource, Ports, Runner, Values};
use deck_outputs::{Clock, Plan};

use crate::clock::now_ns;
use crate::control::Controls;
use crate::graph::{Graph, PortType};
use crate::tempo::Shared;

/// Where a path's value comes from: a node's control input (by name) or output (by index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    In(usize, &'static str),
    Out(usize, usize),
}

/// Every control path of the graph (`node.port`) – what a mapping may name.
pub fn paths(graph: &Graph) -> Vec<String> {
    let mut out = Vec::new();
    for n in &graph.nodes {
        for p in n.kind.inputs().iter().chain(n.kind.outputs()).filter(|p| p.ty == PortType::Control) {
            let path = format!("{}.{}", n.id, p.name);
            if !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out
}

fn slot(graph: &Graph, path: &str) -> Option<Slot> {
    let (id, port) = path.split_once('.')?;
    let (i, n) = graph.nodes.iter().enumerate().find(|(_, n)| n.id == id)?;
    if let Some(k) = n.kind.input(port).filter(|&k| n.kind.inputs()[k].ty == PortType::Control) {
        return Some(Slot::In(i, n.kind.inputs()[k].name));
    }
    n.kind.output(port).filter(|&k| n.kind.outputs()[k].ty == PortType::Control).map(|k| Slot::Out(i, k))
}

/// The tempo clock for the runner: the shared map + the time base's line → a line in seconds on clock::now_ns.
struct Tempo(Arc<Shared>);

impl ClockSource for Tempo {
    fn now(&self) -> f64 {
        now_ns() as f64 * 1e-9
    }

    fn clock(&self) -> Option<Clock> {
        let (map, _) = self.0.map()?;
        let line = self.0.line()?;
        let t = self.now();
        let bps = if map.running { map.per_frame() / line.spf } else { 0.0 };
        Some(Clock { t, beat: map.beat_at(line.frames_at(t)), bps, running: map.running && bps > 0.0, per_bar: map.per_bar })
    }
}

pub struct Outputs {
    runner: Runner,
    values: Values,
    slots: Vec<Slot>,
    conductor: Option<Receiver<[u8; 3]>>,
    /// one HUD line: what runs
    pub summary: String,
}

impl Outputs {
    /// The scene's `[outputs]` → the running outputs. `dir`: the scene file's folder (profile files are relative to it).
    /// A wrong table is an error that lists every problem; a missing device is a warning (the runner drops its
    /// messages, a widget plugged in later is found by the adapter itself).
    pub fn start(table: &toml::Table, dir: Option<&Path>, graph: &Graph, tempo: Arc<Shared>) -> Result<Option<Outputs>> {
        let known = paths(graph);
        let load = |name: &str| -> Result<String, String> {
            let p = dir.map_or_else(|| Path::new(name).to_path_buf(), |d| d.join(name));
            std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
        };
        let plan = deck_outputs::parse(table, Some(&known), &load).map_err(|errs| anyhow!("the scene's [outputs]:\n  {}", errs.join("\n  ")))?;
        if plan.is_empty() {
            return Ok(None);
        }
        let slots = plan.paths.iter().map(|p| slot(graph, p).expect("checked against the graph's paths")).collect();
        let summary = summary(&plan);
        let mut ports = Ports::default();
        if let Some(port) = &plan.dmx {
            let port = (!port.is_empty()).then_some(port.as_str());
            ports.dmx = Some(Box::new(adapters_dmx_usbpro::DmxUsbProAdapter::new("outputs", port)));
        }
        if let Some(port) = &plan.midi {
            // a missing port: the outputs run without it (the HUD says so)
            let mut a = adapters_midi::MidiAdapter::new("deck-outputs", port.as_str(), false);
            let (tx, _rx) = tokio::sync::mpsc::channel(1);
            match adapters_core::Adapter::start(&mut a, tx) {
                Ok(()) => {
                    // started here already: the runner's start would connect again – hand it a started adapter
                    ports.midi = Some(Box::new(Started(a)));
                }
                Err(e) => tracing::warn!("outputs: {e:#} – MIDI out off"),
            }
        }
        let conductor = plan.uses_conductor().then(|| {
            let (tx, rx) = channel();
            ports.conductor = Some(Box::new(move |b| {
                let _ = tx.send(b);
            }));
            rx
        });
        let values = Values::new(plan.paths.len());
        tracing::info!("outputs: {summary}");
        let runner = host::spawn(plan, ports, values.clone(), Box::new(Tempo(tempo)))?;
        Ok(Some(Outputs { runner, values, slots, conductor, summary }))
    }

    /// Each frame: the deck's values to the runner.
    pub fn update(&self, controls: &Controls) {
        for (i, s) in self.slots.iter().enumerate() {
            let v = match *s {
                Slot::In(n, port) => controls.input(n, port, 0.0),
                Slot::Out(n, k) => controls.outputs(n).get(k).copied().unwrap_or(0.0),
            };
            self.values.set(i, v);
        }
    }

    /// The MIDI for the conductor since the last frame (the frame thread hands it to the relay link).
    pub fn conductor_midi(&self, out: &mut Vec<[u8; 3]>) {
        if let Some(rx) = &self.conductor {
            out.extend(rx.try_iter());
        }
    }

    pub fn panic(&self) {
        self.runner.panic();
    }
}

fn summary(plan: &Plan) -> String {
    let mut parts = Vec::new();
    if !plan.targets.is_empty() {
        parts.push(format!("{} targets", plan.targets.len()));
    }
    if !plan.strobes.is_empty() {
        parts.push(format!("{} strobe{}", plan.strobes.len(), if plan.strobes.len() == 1 { "" } else { "s" }));
    }
    if let Some(d) = &plan.dmx {
        parts.push(format!("DMX {}", if d.is_empty() { "auto" } else { d }));
    }
    if let Some(m) = &plan.midi {
        parts.push(format!("MIDI {m}"));
    }
    parts.push(format!("lead {} ms", (plan.lead * 1000.0).round()));
    parts.join(" · ")
}

/// An adapter `start`ed already (so a missing port is known before the runner runs): the runner's start is a no-op.
struct Started(adapters_midi::MidiAdapter);

impl adapters_core::Adapter for Started {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn start(&mut self, _: tokio::sync::mpsc::Sender<adapters_core::Inbound>) -> anyhow::Result<()> {
        Ok(())
    }
    fn send(&mut self, msg: &protocol::DeviceMsg) -> anyhow::Result<()> {
        self.0.send(msg)
    }
    fn panic(&mut self) -> anyhow::Result<()> {
        self.0.panic()
    }
    fn stop(&mut self) -> anyhow::Result<()> {
        self.0.stop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    fn graph(text: &str) -> Graph {
        Graph::build(&Scene::parse(text).unwrap()).unwrap()
    }

    const SCENE: &str = "[[node]]\nid = \"sway\"\nkind = \"lfo\"\nrate = 0.5\n[[node]]\nid = \"tempo\"\nkind = \"tempo\"\n";

    #[test]
    fn paths_are_the_control_ports() {
        let g = graph(SCENE);
        let p = paths(&g);
        assert!(p.contains(&"sway.rate".to_owned()) && p.contains(&"sway.out".to_owned()) && p.contains(&"tempo.beat".to_owned()), "{p:?}");
        assert_eq!(slot(&g, "sway.rate"), Some(Slot::In(0, "rate")));
        assert_eq!(slot(&g, "sway.out"), Some(Slot::Out(0, 0)));
        assert_eq!(slot(&g, "sway.nope"), None);
    }

    #[test]
    fn a_wrong_table_lists_its_problems() {
        let g = graph(SCENE);
        let doc: toml::Table = toml::from_str("[outputs]\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"sway.speed\"\nto = [{ cc = 1 }]\n").unwrap();
        let err = Outputs::start(doc["outputs"].as_table().unwrap(), None, &g, Arc::default()).err().unwrap().to_string();
        assert!(err.contains("the deck has no parameter 'sway.speed'"), "{err}");
    }

    /// the bundled strobe test scene (the owner's first test, #211) parses and its outputs start (no widget here: the
    /// adapter keeps looking for one)
    #[test]
    fn the_strobe_scene_starts() {
        let scene = Scene::parse(include_str!("../scenes/strobe.toml")).unwrap();
        let g = Graph::build(&scene).unwrap();
        let o = Outputs::start(scene.outputs.as_ref().unwrap(), None, &g, Arc::default()).unwrap().expect("a strobe");
        assert_eq!(o.summary, "1 strobe · DMX auto · lead 20 ms");
        assert_eq!(o.slots.len(), 2);
    }

    #[test]
    fn the_clock_follows_the_tempo_map() {
        use crate::clock::Line;
        use crate::tempo::TempoMap;
        let shared = Arc::new(Shared::default());
        let t = Tempo(shared.clone());
        assert!(t.clock().is_none(), "no map yet");
        let now = t.now();
        // 48 kHz, frame 0 heard at `now`; 120 BPM from beat 8 at frame 0
        shared.publish_line(&Line { t: now, f: 0.0, spf: 1.0 / 48_000.0 });
        shared.publish(&TempoMap { frame: 0.0, beat: 8.0, bpm: 120.0, rate: 48_000.0, running: true, per_bar: 4 }, false);
        let c = t.clock().unwrap();
        assert!((c.bpm() - 120.0).abs() < 1e-6);
        assert!((c.beat_at(now + 0.5) - 9.0).abs() < 1e-6);
        assert!((c.time_of(10.0).unwrap() - (now + 1.0)).abs() < 1e-6);
    }
}
