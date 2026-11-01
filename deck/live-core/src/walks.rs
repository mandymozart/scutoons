//! The browser's `walk` (#236): deck-events in live-core, the same random walk the stage's `walk` node runs. The
//! batch graph has no cv inputs, so a walk drives its voice's `freq` instead: deck-space's lowering (rig/engine.rs)
//! writes each walk with the input it plays (`[[walk]] id, target = "voice.freq", seed, base, octaves, length, spread,
//! rate, glide, bpm`) and live-core sets that input at each note's sample – the render is split there. A walk's knobs
//! (spread, rate, glide) are rows of the load table after the batch nodes, so the page sets them like any input.
//! A note's `glide` (the fraction of its length) glides the freq into its pitch (deck-events' `Glide`, exponential –
//! the stage's `GateCv` does the same per sample); live-core sets the glide's value at the start of each sub-block.
//! Its clock is its own `bpm` (the page has no tempo clock; the stage's walk follows the stage's).
use deck_core::batch::BatchGraph;
use deck_core::tempo::TempoMap;
use deck_events::generators::RandomWalk;
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

/// the `[[walk]]` list of a scene's TOML (other keys belong to the batch graph)
#[derive(Debug, Default, Deserialize)]
struct Walks {
    #[serde(default)]
    walk: Vec<WalkDef>,
}

pub const PORTS: [&str; 3] = ["spread", "rate", "glide"];

pub struct Walk {
    pub id: String,
    graph: EventGraph,
    node: usize,
    params: [usize; 3],
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
    let walks: Walks = toml::from_str(scene_toml).map_err(|e| format!("the scene's walks: {e}"))?;
    walks.walk.into_iter().map(|d| Walk::new(d, graph, rate)).collect()
}

impl Walk {
    fn new(d: WalkDef, graph: &BatchGraph, rate: f32) -> Result<Walk, String> {
        let (node, port) = d.target.rsplit_once('.').ok_or_else(|| format!("walk '{}': target '{}' is not node.port", d.id, d.target))?;
        let ni = graph.nodes().position(|(id, _, _)| id == node).ok_or_else(|| format!("walk '{}': no node '{node}'", d.id))?;
        let pi = graph.ports(ni).and_then(|p| p.iter().position(|x| *x == port)).ok_or_else(|| format!("walk '{}': '{node}' has no input '{port}'", d.id))?;
        let mut names = Names::new();
        let note = names.event("music.note").map_err(|e| e.to_string())?;
        let (pitch, glide) = (names.key("pitch").map_err(|e| e.to_string())?, names.key("glide").map_err(|e| e.to_string())?);
        let mut g = EventGraph::new();
        let walk = g.add(RandomWalk {
            note,
            pitch,
            glide_key: glide,
            seed: d.seed,
            base_hz: d.base,
            spread: d.spread,
            octaves: d.octaves,
            per_beat: f64::from(d.rate),
            length: f64::from(d.length),
            glide: d.glide,
        });
        g.build().map_err(|e| e.to_string())?;
        let idx = |n: &str| g.param(walk, n).map_err(|e| e.to_string());
        let params = [idx(PORTS[0])?, idx(PORTS[1])?, idx(PORTS[2])?];
        Ok(Walk { id: d.id, graph: g, node: walk, params, target: (ni, pi), pitch, glide_key: glide, glide: Glide::hold(d.base), elapsed: 0, dirty: false, tempo: TempoMap::new(d.bpm, 4, f64::from(rate)), frame: 0.0, rt: Vec::with_capacity(EDGE_CAP) })
    }

    /// a knob: port `port` of PORTS
    pub fn set(&mut self, port: usize, value: f32) {
        if let Some(&p) = self.params.get(port) {
            self.graph.set(self.node, p, value);
        }
    }

    /// The next `frames`: the notes starting in them as (frame offset, pitch, glide frames), sorted – the caller splits
    /// its render there and calls `begin`. The glide is the note's `glide` prop × its length.
    pub fn notes(&mut self, frames: usize, out: &mut Vec<(usize, f32, u32)>) {
        let span = Span::frames(&self.tempo, self.frame, self.frame + frames as f64);
        self.frame += frames as f64;
        self.rt.clear();
        if self.graph.run(&span).is_ok() {
            if let Ok(n) = self.graph.outputs(self.node, "notes") {
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
