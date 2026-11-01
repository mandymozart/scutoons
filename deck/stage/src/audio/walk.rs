//! deck-events inside the stage (#236) – encapsulated (research/EVENTS.md §10). One block, `Host`, plays a patch of
//! generators (deck-events' patch.rs) once per block on the tempo clock's span: its notes become real-time events and
//! the `gate_cv` bridge writes them as seq's cv (pitch left, gate right, gliding – glide.rs), so an fm / osc voice with
//! its `cv` linked plays them unchanged. Its control inputs are the patch's knobs, read once a block; a non-finite
//! value (an `events` node's unset input) leaves the patch's own value. Two kinds use it:
//! - `walk` – a patch of one random walk, its inputs spread · rate · glide
//! - `events` – the patch in the node's params (`generator`, `edge`, `out`, `knobs`), its inputs a–d
//!
//! Nothing here allocates in `process` (deck-events' no_alloc test, and the stage's).
use super::{Block, Param, Stereo};
use crate::graph::Node;
use crate::tempo::BeatReader;
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::patch::{NoteNames, Patch};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{EventGraph, Names, Span, EDGE_CAP};

pub(super) struct Host {
    graph: EventGraph,
    out: (usize, String),
    /// (the control input, the generator, its param) per knob
    knobs: Vec<(Param, usize, usize)>,
    cv: GateCv,
    rt: Vec<RtEvent>,
    beats: BeatReader,
    /// without a tempo clock: 120 bpm from frame 0, counted here
    own: TempoMap,
    frame: f64,
}

impl Host {
    /// `patch` played with `inputs` as its knobs, in order
    fn new(node: &Node, patch: &Patch, inputs: Vec<Param>, rate: f32, beats: BeatReader) -> Result<Host> {
        let mut names = Names::new();
        let n = NoteNames::new(&mut names);
        let built = patch.build(n, &mut names).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        let knobs = built.knobs.iter().zip(inputs).map(|(&(g, p), input)| (input, g, p)).collect();
        Ok(Host {
            graph: built.graph,
            out: built.out,
            knobs,
            cv: GateCv::new(n.note, n.pitch, n.glide, node.f32("base", 147.0)),
            rt: Vec::with_capacity(EDGE_CAP),
            beats,
            own: TempoMap::new(120.0, 4, f64::from(rate)),
            frame: 0.0,
        })
    }

    /// `walk`: a patch of one random walk (seed, base, octaves, length from the node), knobs spread · rate · glide
    pub(super) fn walk(node: &Node, rate: f32, spread: Param, steps: Param, glide: Param, beats: BeatReader) -> Result<Host> {
        let mut g = toml::Table::new();
        for k in ["seed", "base", "octaves", "length"] {
            if let Some(v) = node.params.get(k) {
                g.insert(k.into(), v.clone());
            }
        }
        g.insert("spread".into(), toml::Value::Float(f64::from(spread.get())));
        g.insert("rate".into(), toml::Value::Float(f64::from(steps.get())));
        g.insert("glide".into(), toml::Value::Float(f64::from(glide.get())));
        let patch = Patch {
            generator: vec![deck_events::patch::GenDef { id: "walk".into(), kind: "random_walk".into(), params: g }],
            edge: Vec::new(),
            out: "walk.notes".into(),
            knobs: vec!["walk.spread".into(), "walk.rate".into(), "walk.glide".into()],
        };
        Host::new(node, &patch, vec![spread, steps, glide], rate, beats)
    }

    /// `events`: the patch in the node's params, inputs a–d
    pub(super) fn events(node: &Node, rate: f32, inputs: [Param; 4], beats: BeatReader) -> Result<Host> {
        let patch = Patch::from_table(&node.params).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        Host::new(node, &patch, inputs.into_iter().collect(), rate, beats)
    }
}

impl Block for Host {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        for (input, g, p) in &self.knobs {
            let v = input.get();
            if v.is_finite() {
                self.graph.set(*g, *p, v);
            }
        }
        let (map, start) = match self.beats.block() {
            Some((m, _, start)) => (m, start),
            None => (self.own, self.frame),
        };
        self.frame = start + n as f64;
        let span = Span::frames(&map, start, start + n as f64);
        self.rt.clear();
        if self.graph.run(&span).is_ok() {
            if let Ok(notes) = self.graph.outputs(self.out.0, &self.out.1) {
                resolve(notes, &span, &mut self.rt);
            }
        }
        let out = &mut outs[0];
        self.cv.render(&self.rt, &mut out.l[..n], &mut out.r[..n]);
    }
}
