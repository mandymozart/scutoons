//! deck-events inside the stage (#236) – encapsulated (research/EVENTS.md §10). `Host` plays a patch of generators
//! (deck-events' patch.rs) on the tempo clock: its notes become real-time events and the `gate_cv` bridge writes them
//! as seq's cv (pitch left, gate right, gliding – glide.rs), so an fm / osc voice with its `cv` linked plays them
//! unchanged. Its control inputs are the patch's knobs, read once a window; a non-finite value (an `events` node's
//! unset input) leaves the patch's own value. Two kinds use it:
//! - `walk` – a patch of one random walk, its inputs spread · rate · glide
//! - `events` – the patch in the node's params (`generator`, `edge`, `out`, `knobs`, `listen`), its inputs a–d
//!
//! Where the generators run (`Sequencing`, EVENTS.md §16 "events first, playback second"):
//! - **thread** (the stage's default): the patch (`Core`) runs on the sequencer thread one window ahead of the render
//!   (sequencer.rs) and sends its notes through a lock-free SPSC ring; the audio block only takes the events of its
//!   frames and renders the cv – no generator code on the audio thread
//! - **in the block** (`DECK_EVENTS=block`, the tests' `build_with`): the patch runs inside the block, once per block
//!
//! Both play the same samples (tests/walk.rs). The patch's `listen` inputs get their events from the dispatcher
//! (dispatch.rs) through one SPSC ring each. Nothing here allocates in `process` (tests/no_alloc.rs, both paths).
use super::dispatch::{Counters, Events};
use super::sequencer::{Timed, SEQ_RING};
use super::{Block, Param, Sequencing, Stereo};
use crate::graph::Node;
use crate::tempo::BeatReader;
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::graph::INJECT_CAP;
use deck_events::patch::{NoteNames, Patch};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, EventGraph, Route, Span, EDGE_CAP};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// A host's generator side: the patch's graph, the knobs it reads, the rings its `listen` inputs drain. Runs once per
/// window – on the sequencer thread, or inside the block. Never allocates in `window`.
pub(super) struct Core {
    graph: EventGraph,
    out: (usize, String),
    /// (the control input, the generator, its param) per knob
    knobs: Vec<(Param, usize, usize)>,
    /// (generator, input, the dispatcher's ring) per `listen`
    inbox: Vec<(usize, usize, rtrb::Consumer<Event>)>,
    counters: Arc<Counters>,
}

impl Core {
    /// One window [start, start + n) on `map`: the knobs, the routed-in events, the graph; its notes into `rt`
    /// (cleared first) at frame offsets from `start`, sorted.
    pub(super) fn window(&mut self, map: &TempoMap, start: f64, n: usize, rt: &mut Vec<RtEvent>) {
        for (input, g, p) in &self.knobs {
            let v = input.get();
            if v.is_finite() {
                self.graph.set(*g, *p, v);
            }
        }
        for (g, p, ring) in &mut self.inbox {
            while let Ok(e) = ring.pop() {
                self.graph.inject_at(*g, *p, e);
            }
        }
        let span = Span::frames(map, start, start + n as f64);
        rt.clear();
        if self.graph.run(&span).is_ok() {
            if let Ok(notes) = self.graph.outputs(self.out.0, &self.out.1) {
                resolve(notes, &span, rt);
            }
        }
        let s = self.graph.take_stats();
        if s != deck_events::Stats::default() {
            self.counters.add(s);
        }
    }
}

/// what the block plays from
enum Source {
    /// the patch itself, run per block
    Block(Box<Core>),
    /// the sequencer's events, absolute frames, in order
    Ring(rtrb::Consumer<Timed>),
}

pub(super) struct Host {
    src: Source,
    cv: GateCv,
    rt: Vec<RtEvent>,
    beats: BeatReader,
    /// without a tempo clock: 120 bpm from frame 0, counted here
    own: TempoMap,
    frame: f64,
    counters: Arc<Counters>,
}

impl Host {
    /// `patch` played with `inputs` as its knobs, in order
    fn new(node: &Node, patch: &Patch, inputs: Vec<Param>, rate: f32, beats: BeatReader, ev: &mut Events) -> Result<Host> {
        let n = NoteNames::new(&mut ev.names);
        let built = patch.build(n, &mut ev.names).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        let knobs = built.knobs.iter().zip(inputs).map(|(&(g, p), input)| (input, g, p)).collect();
        let mut inbox = Vec::with_capacity(built.listen.len());
        for l in &built.listen {
            let (tx, rx) = rtrb::RingBuffer::new(INJECT_CAP);
            ev.routes.push(Route { on: l.on, to: ev.outlets.len() });
            ev.outlets.push(tx);
            inbox.push((l.node, l.port, rx));
        }
        let own = TempoMap::new(120.0, 4, f64::from(rate));
        let core = Core { graph: built.graph, out: built.out, knobs, inbox, counters: ev.counters.clone() };
        let src = match ev.mode {
            Sequencing::InBlock => Source::Block(Box::new(core)),
            Sequencing::Thread => {
                let (tx, rx) = rtrb::RingBuffer::new(SEQ_RING);
                ev.sequencer(&beats, own).add(core, tx);
                Source::Ring(rx)
            }
        };
        Ok(Host {
            src,
            cv: GateCv::new(n.note, n.pitch, n.glide, node.f32("base", 147.0)),
            rt: Vec::with_capacity(EDGE_CAP),
            beats,
            own,
            frame: 0.0,
            counters: ev.counters.clone(),
        })
    }

    /// `walk`: a patch of one random walk (seed, base, octaves, length from the node), knobs spread · rate · glide
    pub(super) fn walk(node: &Node, rate: f32, knobs: [Param; 3], beats: BeatReader, ev: &mut Events) -> Result<Host> {
        let [spread, steps, glide] = knobs;
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
            listen: Vec::new(),
        };
        Host::new(node, &patch, vec![spread, steps, glide], rate, beats, ev)
    }

    /// `events`: the patch in the node's params, inputs a–d
    pub(super) fn events(node: &Node, rate: f32, inputs: [Param; 4], beats: BeatReader, ev: &mut Events) -> Result<Host> {
        let patch = Patch::from_table(&node.params).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        Host::new(node, &patch, inputs.into_iter().collect(), rate, beats, ev)
    }
}

impl Block for Host {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let (map, start) = match self.beats.block() {
            Some((m, _, start)) => (m, start),
            None => (self.own, self.frame),
        };
        self.frame = start + n as f64;
        match &mut self.src {
            Source::Block(core) => core.window(&map, start, n, &mut self.rt),
            Source::Ring(ring) => {
                // the events of this block's frames (an earlier one is late: it plays at once, counted); the next
                // block's stay in the ring
                self.rt.clear();
                let (from, to) = (start as u64, start as u64 + n as u64);
                let (mut late, mut capped) = (0, 0);
                while let Ok(t) = ring.peek() {
                    if t.at >= to {
                        break;
                    }
                    let t = *t;
                    let _ = ring.pop();
                    late += u64::from(t.at < from);
                    if self.rt.len() < self.rt.capacity() {
                        self.rt.push(RtEvent { frame: t.at.saturating_sub(from) as u32, name: t.name, props: t.props, len: t.len });
                    } else {
                        capped += 1;
                    }
                }
                if late > 0 {
                    self.counters.late.fetch_add(late, Ordering::Relaxed);
                }
                if capped > 0 {
                    self.counters.capped.fetch_add(capped, Ordering::Relaxed);
                }
            }
        }
        let out = &mut outs[0];
        self.cv.render(&self.rt, &mut out.l[..n], &mut out.r[..n]);
    }
}
