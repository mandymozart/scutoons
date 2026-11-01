//! The browser's generators (#236): deck-events in live-core – the same patches the stage's `walk` and `events` nodes
//! play. The batch graph has no cv inputs, so a host drives its voice's `freq` instead: deck-space's lowering
//! (rig/engine.rs) writes each with the input it plays –
//! - `[[walk]] id, target = "voice.freq", seed, base, octaves, length, spread, rate, glide, bpm` (one random walk; its
//!   knobs spread · rate · glide)
//! - `[[events]] id, target, bpm, base, a…d, generator, edge, out, knobs` (a patch, deck-events' patch.rs; its knobs a–d)
//!
//! – and live-core sets that input at each note's sample, gliding (glide.rs) in sub-blocks. A host's knobs are rows of
//! the load table after the batch nodes, so the page sets them like any input. Its clock is its own `bpm` (the page has
//! no tempo clock; the stage's hosts follow the stage's).
use deck_core::batch::BatchGraph;
use deck_core::tempo::TempoMap;
use deck_events::patch::{GenDef, NoteNames, Patch};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{EventGraph, Glide, Names, Span, Sym, EDGE_CAP};
use serde::Deserialize;

/// a walk as the lowered scene writes it
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkDef {
    pub id: String,
    /// `node.port` of the batch graph it plays (a voice's `freq`)
    pub target: String,
    #[serde(default = "seed")]
    pub seed: u64,
    #[serde(default = "base")]
    pub base: f32,
    #[serde(default = "octaves")]
    pub octaves: f32,
    #[serde(default = "one")]
    pub length: f32,
    #[serde(default)]
    pub spread: f32,
    #[serde(default = "two")]
    pub rate: f32,
    #[serde(default = "half")]
    pub glide: f32,
    #[serde(default = "bpm")]
    pub bpm: f64,
}

fn seed() -> u64 {
    1
}
fn base() -> f32 {
    147.0
}
fn octaves() -> f32 {
    2.0
}
fn one() -> f32 {
    1.0
}
fn two() -> f32 {
    2.0
}
fn half() -> f32 {
    0.5
}
fn bpm() -> f64 {
    120.0
}

/// an `[[events]]` host: a patch and the input it plays
#[derive(Debug, Clone, Deserialize)]
pub struct EventsDef {
    pub id: String,
    pub target: String,
    #[serde(default = "bpm")]
    pub bpm: f64,
    #[serde(default = "base")]
    pub base: f32,
    /// the knobs' starting values (unset = the patch's own)
    pub a: Option<f32>,
    pub b: Option<f32>,
    pub c: Option<f32>,
    pub d: Option<f32>,
    #[serde(flatten)]
    pub patch: Patch,
}

/// the `[[walk]]` and `[[events]]` lists of a scene's TOML (other keys belong to the batch graph)
#[derive(Debug, Default, Deserialize)]
struct Hosts {
    #[serde(default)]
    walk: Vec<WalkDef>,
    #[serde(default)]
    events: Vec<EventsDef>,
}

/// a walk's knobs, and an events host's
pub const PORTS: [&str; 3] = ["spread", "rate", "glide"];
pub const EVENTS_PORTS: [&str; 4] = ["a", "b", "c", "d"];

pub struct Walk {
    pub id: String,
    /// `walk` or `events` (its kind in the load table)
    pub kind: &'static str,
    /// its knobs' names (the load table's ports)
    pub ports: &'static [&'static str],
    graph: EventGraph,
    /// the generator and port it plays
    out: (usize, String),
    /// (generator, param) per knob
    knobs: Vec<(usize, usize)>,
    /// the batch node and input it plays
    target: (usize, usize),
    pitch: Sym,
    glide_key: Sym,
    /// the freq now: a glide from the last pitch, `elapsed` frames in
    glide: Glide,
    elapsed: u32,
    /// the target has to be sent: a note began, or a glide moved on
    dirty: bool,
    tempo: TempoMap,
    frame: f64,
    rt: Vec<RtEvent>,
}

pub fn parse(scene_toml: &str, graph: &BatchGraph, rate: f32) -> Result<Vec<Walk>, String> {
    let hosts: Hosts = toml::from_str(scene_toml).map_err(|e| format!("the scene's walks / events: {e}"))?;
    let walks = hosts.walk.into_iter().map(|d| {
        let mut g = toml::Table::new();
        for (k, v) in [("seed", d.seed as f64), ("base", d.base.into()), ("octaves", d.octaves.into()), ("length", d.length.into())] {
            g.insert(k.into(), if k == "seed" { toml::Value::Integer(v as i64) } else { toml::Value::Float(v) });
        }
        for (k, v) in [("spread", d.spread), ("rate", d.rate), ("glide", d.glide)] {
            g.insert(k.into(), toml::Value::Float(v.into()));
        }
        let patch = Patch {
            generator: vec![GenDef { id: "walk".into(), kind: "random_walk".into(), params: g }],
            edge: Vec::new(),
            out: "walk.notes".into(),
            knobs: PORTS.iter().map(|p| format!("walk.{p}")).collect(),
            listen: Vec::new(),
        };
        Walk::new(d.id, "walk", &PORTS, &d.target, &patch, d.bpm, d.base, &[], graph, rate)
    });
    let events = hosts.events.into_iter().map(|d| {
        let start = [d.a, d.b, d.c, d.d];
        Walk::new(d.id, "events", &EVENTS_PORTS, &d.target, &d.patch, d.bpm, d.base, &start, graph, rate)
    });
    walks.chain(events).collect()
}

impl Walk {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: String,
        kind: &'static str,
        ports: &'static [&'static str],
        target: &str,
        patch: &Patch,
        bpm: f64,
        base: f32,
        start: &[Option<f32>],
        graph: &BatchGraph,
        rate: f32,
    ) -> Result<Walk, String> {
        let (node, port) = target.rsplit_once('.').ok_or_else(|| format!("{kind} '{id}': target '{target}' is not node.port"))?;
        let ni = graph.nodes().position(|(n, _, _)| n == node).ok_or_else(|| format!("{kind} '{id}': no node '{node}'"))?;
        let pi = graph.ports(ni).and_then(|p| p.iter().position(|x| *x == port)).ok_or_else(|| format!("{kind} '{id}': '{node}' has no input '{port}'"))?;
        let mut names = Names::new();
        let n = NoteNames::new(&mut names);
        let mut built = patch.build(n, &mut names).map_err(|e| format!("{kind} '{id}': {e}"))?;
        for (&(g, p), v) in built.knobs.iter().zip(start) {
            if let Some(v) = v.filter(|v| v.is_finite()) {
                built.graph.set(g, p, v);
            }
        }
        Ok(Walk {
            id,
            kind,
            ports,
            graph: built.graph,
            out: built.out,
            knobs: built.knobs,
            target: (ni, pi),
            pitch: n.pitch,
            glide_key: n.glide,
            glide: Glide::hold(base),
            elapsed: 0,
            dirty: false,
            tempo: TempoMap::new(bpm, 4, f64::from(rate)),
            frame: 0.0,
            rt: Vec::with_capacity(EDGE_CAP),
        })
    }

    /// a knob: port `port` of its ports (a non-finite value leaves the patch's own)
    pub fn set(&mut self, port: usize, value: f32) {
        if let Some(&(g, p)) = self.knobs.get(port) {
            if value.is_finite() {
                self.graph.set(g, p, value);
            }
        }
    }

    /// The next `frames`: the notes starting in them as (frame offset, pitch, glide frames), sorted – the caller splits
    /// its render there and calls `begin`. The glide is the note's `glide` prop × its length.
    pub fn notes(&mut self, frames: usize, out: &mut Vec<(usize, f32, u32)>) {
        let span = Span::frames(&self.tempo, self.frame, self.frame + frames as f64);
        self.frame += frames as f64;
        self.rt.clear();
        if self.graph.run(&span).is_ok() {
            if let Ok(n) = self.graph.outputs(self.out.0, &self.out.1) {
                resolve(n, &span, &mut self.rt);
            }
        }
        let glide = self.glide_key;
        out.extend(self.rt.iter().filter_map(|e| {
            let g = e.props.get(glide).unwrap_or(0.0).clamp(0.0, 1.0);
            e.props.get(self.pitch).map(|p| (e.frame as usize, p, (g * e.len as f32).round() as u32))
        }));
    }

    /// A note starts: glide from the freq now to `pitch` over `frames` (0 = a jump).
    pub fn begin(&mut self, pitch: f32, frames: u32) {
        let now = self.glide.at(self.elapsed);
        self.glide.start(now, pitch, frames);
        self.elapsed = 0;
        self.dirty = true;
    }

    /// `frames` of the render went by.
    pub fn advance(&mut self, frames: usize) {
        if self.glide.running(self.elapsed) {
            self.elapsed = self.elapsed.saturating_add(frames as u32);
            self.dirty = true;
        }
    }

    /// Whether a glide runs (the render is cut into sub-blocks while one does).
    pub fn gliding(&self) -> bool {
        self.glide.running(self.elapsed)
    }

    /// The freq to send now, if it changed since the last call.
    pub fn take(&mut self) -> Option<f32> {
        std::mem::take(&mut self.dirty).then(|| self.glide.at(self.elapsed))
    }

    pub fn target(&self) -> (usize, usize) {
        self.target
    }
}
