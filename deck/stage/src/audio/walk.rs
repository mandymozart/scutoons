//! The `walk` node (#236): deck-events inside today's graph – encapsulated (research/EVENTS.md §10). A random walk
//! (deck-events `RandomWalk`, seeded, continuous pitch) runs in an event graph once per block on the tempo clock's
//! span; its notes become real-time events and the `gate_cv` bridge writes them as seq's cv (pitch left, gate right),
//! so an fm / osc voice with its `cv` linked plays them unchanged. Its control inputs (spread, rate, glide) are the
//! generator's params, read once a block. Nothing here allocates in `process` (deck-events' no_alloc test, and the
//! stage's).
use super::{Block, Param, Stereo};
use crate::graph::Node;
use crate::tempo::BeatReader;
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::generators::RandomWalk;
use deck_events::rt::{resolve, RtEvent};
use deck_events::{EventGraph, Names, Span, EDGE_CAP};

pub(super) struct Walk {
    graph: EventGraph,
    walk: usize,
    /// (the generator's param index, the control input) for spread, rate, glide
    knobs: [(usize, Param); 3],
    cv: GateCv,
    rt: Vec<RtEvent>,
    beats: BeatReader,
    /// without a tempo clock: 120 bpm from frame 0, counted here
    own: TempoMap,
    frame: f64,
}

impl Walk {
    pub(super) fn new(node: &Node, rate: f32, spread: Param, steps: Param, glide: Param, beats: BeatReader) -> Result<Walk> {
        let mut names = Names::new();
        let note = names.event("music.note")?;
        let (pitch, glide_key) = (names.key("pitch")?, names.key("glide")?);
        let base = node.f32("base", 147.0);
        let seed = match node.params.get("seed") {
            Some(toml::Value::Integer(i)) => *i as u64,
            Some(toml::Value::Float(f)) => *f as u64,
            _ => 1,
        };
        let mut graph = EventGraph::new();
        let walk = graph.add(RandomWalk {
            note,
            pitch,
            glide_key,
            seed,
            base_hz: base,
            spread: spread.get(),
            octaves: node.f32("octaves", 2.0),
            per_beat: f64::from(steps.get()),
            // legato by default: the gate stays open, the voice glides from note to note (the page's melody)
            length: f64::from(node.f32("length", 1.0)),
            glide: glide.get(),
        });
        graph.build().map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        let index = |name: &str| graph.param(walk, name).map_err(|e| anyhow!("node '{}': {e}", node.id));
        let knobs = [(index("spread")?, spread), (index("rate")?, steps), (index("glide")?, glide)];
        Ok(Walk {
            graph,
            walk,
            knobs,
            cv: GateCv::new(note, pitch, base),
            rt: Vec::with_capacity(EDGE_CAP),
            beats,
            own: TempoMap::new(120.0, 4, f64::from(rate)),
            frame: 0.0,
        })
    }
}

impl Block for Walk {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        for (i, p) in &self.knobs {
            self.graph.set(self.walk, *i, p.get());
        }
        let (map, start) = match self.beats.block() {
            Some((m, _, start)) => (m, start),
            None => (self.own, self.frame),
        };
        self.frame = start + n as f64;
        let span = Span::frames(&map, start, start + n as f64);
        self.rt.clear();
        if self.graph.run(&span).is_ok() {
            if let Ok(notes) = self.graph.outputs(self.walk, "notes") {
                resolve(notes, &span, &mut self.rt);
            }
        }
        let out = &mut outs[0];
        self.cv.render(&self.rt, &mut out.l[..n], &mut out.r[..n]);
    }
}
