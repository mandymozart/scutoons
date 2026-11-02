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

/// #264: how a scene switch treats the outputs of the scene before: hand over (fade / cut) or go dark first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Switch {
    pub transition: deck_space::scene::SceneTransition,
    pub secs: f32,
}

pub struct Outputs {
    runner: Runner,
    /// the devices this runs on (DMX widget, MIDI port, the conductor tap): a following scene may take the runner
    /// over only when it wants the same
    devices: (Option<String>, Option<String>, bool),
    values: Values,
    slots: Vec<Slot>,
    conductor: Option<Receiver<[u8; 3]>>,
    /// one HUD line: what runs
    pub summary: String,
}

impl Drop for Outputs {
    /// A scene that ends (switched, or the stage ending – Ctrl+Q, a stop signal, #263) leaves its lamps dark: a USB-DMX
    /// widget repeats the last frame for ever (#261). Only a SIGKILL can't: the next scene with outputs starts from its look.
    fn drop(&mut self) {
        tracing::info!("outputs: stopping – the lamps go dark");
        self.runner.stop_dark();
        tracing::info!("outputs: dark");
    }
}

impl Outputs {
    /// The scene's `[outputs]` → the running outputs. `dir`: the scene file's folder (profile files are relative to it).
    /// A wrong table is an error that lists every problem; a missing device is a warning (the runner drops its
    /// messages, a widget plugged in later is found by the adapter itself).
    pub fn start(table: &toml::Table, dir: Option<&Path>, graph: &Graph, tempo: Arc<Shared>) -> Result<Option<Outputs>> {
        Self::start_after(table, dir, graph, tempo, &mut None, Switch { transition: deck_space::scene::SceneTransition::Blackout, secs: 0.0 })
    }

    /// `start` on a scene switch (#264): `before` = the outputs of the scene that was playing. A `fade` / `cut` onto the
    /// same devices keeps their runner (no close and reopen – the lamps hold the last look) and crosses over to the new
    /// plan; anything else ends `before` dark first (its port is free for the new one).
    pub fn start_after(table: &toml::Table, dir: Option<&Path>, graph: &Graph, tempo: Arc<Shared>, before: &mut Option<Outputs>, switch: Switch) -> Result<Option<Outputs>> {
        let known = paths(graph);
        let load = |name: &str| -> Result<String, String> {
            let p = dir.map_or_else(|| Path::new(name).to_path_buf(), |d| d.join(name));
            std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
        };
        let plan = deck_outputs::parse(table, Some(&known), &load).map_err(|errs| anyhow!("the scene's [outputs]:\n  {}", errs.join("\n  ")))?;
        if plan.is_empty() {
            return Ok(None);
        }
        let slots: Vec<Slot> = plan.paths.iter().map(|p| slot(graph, p).expect("checked against the graph's paths")).collect();
        let summary = summary(&plan);
        let devices = (plan.dmx.clone(), plan.midi.clone(), plan.uses_conductor());
        if let Some(mut old) = before.take() {
            if old.devices == devices && switch.transition != deck_space::scene::SceneTransition::Blackout {
                tracing::info!("outputs: handing over to the next scene ({:?}, {} s)", switch.transition, switch.secs);
                let secs = if switch.transition == deck_space::scene::SceneTransition::Cut { 0.0 } else { f64::from(switch.secs) };
                let values = Values::new(plan.paths.len());
                old.runner.hand_over(plan, values.clone(), secs);
                old.values = values;
                old.slots = slots;
                old.summary = summary;
                return Ok(Some(old));
            }
            drop(old); // dark, its devices free
        }
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
        tracing::info!("outputs: {summary}");
        Self::launch(plan, ports, slots, conductor, summary, tempo, devices).map(Some)
    }

    /// The runner on its thread with the ports as they are.
    fn launch(plan: Plan, ports: Ports, slots: Vec<Slot>, conductor: Option<Receiver<[u8; 3]>>, summary: String, tempo: Arc<Shared>, devices: (Option<String>, Option<String>, bool)) -> Result<Outputs> {
        let values = Values::new(plan.paths.len());
        let runner = host::spawn(plan, ports, values.clone(), Box::new(Tempo(tempo)))?;
        Ok(Outputs { runner, devices, values, slots, conductor, summary })
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

    /// #264: a scene without outputs follows one with: the lamps fade to 0 over `secs` and the caller drops this (dark)
    /// once that time is over. The widget and port stay open meanwhile.
    pub fn fade_out(&mut self, secs: f32) {
        self.values = Values::new(0);
        self.slots.clear();
        self.runner.hand_over(Plan::default(), self.values.clone(), f64::from(secs));
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

    /// the Thunder Wash scene (#261): its hue colours the three channels, the beat's pulse is a gain on them
    #[test]
    fn the_pulse_thunderwash_scene_starts() {
        let scene = Scene::parse(include_str!("../scenes/pulse-thunderwash.toml")).unwrap();
        let g = Graph::build(&scene).unwrap();
        let o = Outputs::start(scene.outputs.as_ref().unwrap(), None, &g, Arc::default()).unwrap().expect("targets");
        assert_eq!(o.summary, "3 targets · DMX auto · lead 20 ms");
        assert_eq!(o.slots.len(), 3, "hue, the beat's pulse, the breathing");
    }

    /// the lamps-off scene (#261) holds the four channels at 0
    #[test]
    fn the_lamps_off_scene_starts() {
        let scene = Scene::parse(include_str!("../scenes/lamps-off.toml")).unwrap();
        let g = Graph::build(&scene).unwrap();
        let o = Outputs::start(scene.outputs.as_ref().unwrap(), None, &g, Arc::default()).unwrap().expect("targets");
        assert_eq!(o.summary, "4 targets · DMX auto · lead 20 ms");
    }

    /// #263: the outputs ending (the stage's end on a stop signal, Ctrl+Q, a scene switch) = the widget's last frame is
    /// all zeros, whatever the lamp showed before
    #[test]
    fn the_outputs_ending_black_the_widget_out() {
        use adapters_dmx_usbpro::{mock::Widget, DmxUsbProAdapter};
        let g = graph(SCENE);
        let table: toml::Table = toml::from_str("[outputs]
dmx = \"auto\"
[[outputs.map]]
from = \"sway.out\"
to = [{ dmx = 1 }, { dmx = 2 }]
").unwrap();
        let known = paths(&g);
        let plan = deck_outputs::parse(table["outputs"].as_table().unwrap(), Some(&known), &|_| Err("no files".into())).unwrap();
        let slots = plan.paths.iter().map(|p| slot(&g, p).unwrap()).collect();
        let widget = Widget::default();
        let ports = Ports { dmx: Some(Box::new(DmxUsbProAdapter::with_opener("lamp", None, widget.opener()))), ..Ports::default() };
        let o = Outputs::launch(plan, ports, slots, None, String::new(), Arc::default(), (None, None, false)).unwrap();
        o.values.set(0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let lit = widget.frames().last().expect("a frame while it plays").1;
        assert_eq!((lit[0], lit[1]), (255, 255), "the lamp is lit before the end");
        drop(o);
        let last = widget.frames().last().unwrap().1;
        assert_eq!((last[0], last[1]), (0, 0), "dark after the end");
    }

    /// #264: a switch onto the same widget keeps the runner: the lamp holds its look, then crosses over (ch 1 only the
    /// old scene drives → fades to 0, ch 2 only the new one → up), no dark frame in between
    #[test]
    fn a_switch_hands_the_lamp_over_without_a_dark_frame() {
        use adapters_dmx_usbpro::{mock::Widget, DmxUsbProAdapter};
        use deck_space::scene::SceneTransition;
        let g = graph(SCENE);
        let map = |ch: u8| format!("[outputs]
dmx = \"auto\"
[[outputs.map]]
from = \"sway.out\"
to = [{{ dmx = {ch} }}]
");
        let parse = |text: &str| -> toml::Table { toml::from_str(text).unwrap() };
        let (a, b) = (parse(&map(1)), parse(&map(2)));
        let known = paths(&g);
        let plan = deck_outputs::parse(a["outputs"].as_table().unwrap(), Some(&known), &|_| Err("no files".into())).unwrap();
        let slots = plan.paths.iter().map(|p| slot(&g, p).unwrap()).collect();
        let widget = Widget::default();
        let ports = Ports { dmx: Some(Box::new(DmxUsbProAdapter::with_opener("lamp", None, widget.opener()))), ..Ports::default() };
        let devices = (plan.dmx.clone(), plan.midi.clone(), false);
        let o = Outputs::launch(plan, ports, slots, None, String::new(), Arc::default(), devices).unwrap();
        o.values.set(0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(widget.frames().last().unwrap().1[0], 255);
        let at_switch = widget.frames().len();
        let mut before = Some(o);
        let switch = Switch { transition: SceneTransition::Fade, secs: 0.8 };
        let o = Outputs::start_after(b["outputs"].as_table().unwrap(), None, &g, Arc::default(), &mut before, switch).unwrap().expect("handed over");
        assert!(before.is_none());
        o.values.set(0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mid = widget.frames().last().unwrap().1;
        assert!(mid[0] > 0 && mid[0] < 255 && mid[1] > 0 && mid[1] < 255, "half way: {:?}", &mid[..2]);
        std::thread::sleep(std::time::Duration::from_millis(800));
        let end = widget.frames().last().unwrap().1;
        assert_eq!((end[0], end[1]), (0, 255), "crossed over");
        // never dark in between: every frame since the switch has one of the two channels well up
        assert!(widget.frames()[at_switch..].iter().all(|(_, f)| f[0] >= 60 || f[1] >= 60), "a dark frame in between");
        // a stop is still a stop
        drop(o);
        let last = widget.frames().last().unwrap().1;
        assert_eq!((last[0], last[1]), (0, 0));
    }

    /// #264: `blackout` ends the scene before dark and starts the next one anew (its own widget)
    #[test]
    fn a_blackout_switch_ends_the_old_outputs_dark() {
        use adapters_dmx_usbpro::{mock::Widget, DmxUsbProAdapter};
        use deck_space::scene::SceneTransition;
        let g = graph(SCENE);
        let table: toml::Table = toml::from_str("[outputs]
dmx = \"auto\"
[[outputs.map]]
from = \"sway.out\"
to = [{ dmx = 1 }]
").unwrap();
        let known = paths(&g);
        let plan = deck_outputs::parse(table["outputs"].as_table().unwrap(), Some(&known), &|_| Err("no files".into())).unwrap();
        let slots = plan.paths.iter().map(|p| slot(&g, p).unwrap()).collect();
        let widget = Widget::default();
        let ports = Ports { dmx: Some(Box::new(DmxUsbProAdapter::with_opener("lamp", None, widget.opener()))), ..Ports::default() };
        let devices = (plan.dmx.clone(), plan.midi.clone(), false);
        let o = Outputs::launch(plan, ports, slots, None, String::new(), Arc::default(), devices).unwrap();
        o.values.set(0, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let mut before = Some(o);
        let switch = Switch { transition: SceneTransition::Blackout, secs: 0.4 };
        let _next = Outputs::start_after(table["outputs"].as_table().unwrap(), None, &g, Arc::default(), &mut before, switch).unwrap();
        assert!(before.is_none());
        assert_eq!(widget.frames().last().unwrap().1[0], 0, "dark");
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
