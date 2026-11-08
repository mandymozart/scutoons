//! deck-events inside the stage (#236) – encapsulated (research/EVENTS.md §10). `Host` plays a patch of generators
//! (deck-events' patch.rs) on the tempo clock: its notes become real-time events and the `gate_cv` bridge writes them
//! as seq's cv (pitch left, gate right, gliding – glide.rs), so an fm / osc voice with its `cv` linked plays them
//! unchanged. Its control inputs are the patch's knobs, read once a window; a non-finite value (an `events` node's
//! unset input) leaves the patch's own value. Two kinds use it:
//! - `walk` – a patch of one random walk, its inputs spread · rate · glide
//! - `events` – the patch in the node's params (`generator`, `edge`, `out`, `knobs`, `listen`), its inputs a–d
//! - `world` with `source = "generator"` (#430) – the patch in its `patch` table, no cv: its notes go down its event cables
//!
//! Where the generators run (`Sequencing`, EVENTS.md §16 "events first, playback second"):
//! - **thread** (the stage's default): the patch (`Core`) runs on the sequencer thread one window ahead of the render
//!   (sequencer.rs) and sends its notes through a lock-free SPSC ring; the audio block only takes the events of its
//!   frames and renders the cv – no generator code on the audio thread
//! - **in the block** (`DECK_EVENTS=block`, the tests' `build_with`): the patch runs inside the block, once per block
//!
//! Both play the same samples (tests/walk.rs). The patch's `listen` inputs get their events from the dispatcher
//! (dispatch.rs) through one SPSC ring each. Nothing here allocates in `process` (tests/no_alloc.rs, both paths).
//!
//! The transport (#359, research/TRANSPORT.md): both kinds follow it unless `follow_transport = false` – a stop holds the
//! walk's position, closes the gate on its frame and drops the notes computed past it (the ring's notes of an older
//! transport epoch); a start or continue plays on from the transport's beat. A `listen` to `clock.start`, `clock.stop`,
//! `clock.continue` (or the domain `clock`) gets the edge as an event (prop `beat`) from here, not from the dispatcher.
use super::dispatch::{Counters, Events};
#[cfg(feature = "modules")]
use super::evhost::WorldOut;
use super::sequencer::{Timed, SEQ_RING};
use super::{Block, Buf, Param, Sequencing};
use deck_space::graph::Node;
#[cfg(feature = "modules")]
use deck_space::world::{Source as WorldSource, World};
use deck_core::tempo::{BeatReader, Edge, Follow};
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::graph::INJECT_CAP;
use deck_events::patch::{NoteNames, Patch};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, EventGraph, Match, Route, Span, Sym, Time, EDGE_CAP};
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
    /// (generator, input, the edge, its event name) per `listen` to the transport's events
    clock: Vec<(usize, usize, Edge, Sym)>,
    beat_key: Sym,
    follow: Follow,
    /// a window's part after a mid-window edge, resolved apart (reserved)
    part: Vec<RtEvent>,
    counters: Arc<Counters>,
}

impl Core {
    /// One window [start, start + n) on the transport's `map` of `epoch`: the knobs, the routed-in events, the
    /// transport's edge, the graph; its notes into `rt` (cleared first) at frame offsets from `start`, sorted. Returns
    /// the edge and its frame offset.
    pub(super) fn window(&mut self, map: &TempoMap, epoch: u64, start: f64, n: usize, rt: &mut Vec<RtEvent>) -> Option<(Edge, usize)> {
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
        let played = self.follow.block(*map, epoch, start, n);
        if let Some(edge) = played.edge {
            let beat = played.map.beat_at(start + played.split as f64);
            for &(g, p, e, name) in &self.clock {
                if e == edge {
                    self.graph.inject_at(g, p, Event::new(name, Time::Beats(beat)).with(self.beat_key, beat as f32));
                }
            }
        }
        rt.clear();
        if played.split > 0 {
            self.run(&played.before, start, played.split, rt);
            let mut part = std::mem::take(&mut self.part);
            part.clear();
            self.run(&played.map, start + played.split as f64, n - played.split, &mut part);
            for e in &part {
                if rt.len() < rt.capacity() {
                    rt.push(RtEvent { frame: e.frame + played.split as u32, ..*e });
                }
            }
            self.part = part;
        } else {
            self.run(&played.map, start, n, rt);
        }
        let s = self.graph.take_stats();
        if s != deck_events::Stats::default() {
            self.counters.add(s);
        }
        played.edge.map(|e| (e, played.split))
    }

    /// the graph over [from, from + n) on `map`, its notes resolved into `out`
    fn run(&mut self, map: &TempoMap, from: f64, n: usize, out: &mut Vec<RtEvent>) {
        let span = Span::frames(map, from, from + n as f64);
        if self.graph.run(&span).is_ok() {
            if let Ok(notes) = self.graph.outputs(self.out.0, &self.out.1) {
                resolve(notes, &span, out);
            }
        }
    }

    /// does the patch follow the transport (the sequencer re-runs only these windows when it moves)
    pub(super) fn follows(&self) -> bool {
        self.follow.on()
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
    /// the ring's side of the transport (the patch's own is in its `Core`)
    follow: Follow,
    counters: Arc<Counters>,
    /// a generator world node (#430): its notes go down its event cables, not into the cv
    #[cfg(feature = "modules")]
    sink: Option<WorldOut>,
}

impl Host {
    /// `patch` played with `inputs` as its knobs, in order
    fn new(node: &Node, patch: &Patch, inputs: Vec<Param>, rate: f32, beats: BeatReader, follow: Follow, ev: &mut Events) -> Result<Host> {
        let n = NoteNames::new(&mut ev.names);
        let built = patch.build(n, &mut ev.names).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        let knobs = built.knobs.iter().zip(inputs).map(|(&(g, p), input)| (input, g, p)).collect();
        let mut name = |s: &str| ev.names.event(s).map_err(|e| anyhow!("{s}: {e:?}"));
        let edges = [(Edge::Start, name("clock.start")?), (Edge::Stop, name("clock.stop")?), (Edge::Continue, name("clock.continue")?)];
        let domain = ev.names.key("clock").map_err(|e| anyhow!("clock: {e:?}"))?;
        let beat_key = ev.names.key("beat").map_err(|e| anyhow!("beat: {e:?}"))?;
        let mut inbox = Vec::with_capacity(built.listen.len());
        let mut clock = Vec::new();
        for l in &built.listen {
            // the transport's events come from the engine's own transport, not through the dispatcher
            let hit = edges.iter().filter(|(_, s)| l.on == Match::Name(*s) || l.on == Match::Domain(domain));
            let before = clock.len();
            clock.extend(hit.map(|&(e, s)| (l.node, l.port, e, s)));
            if clock.len() > before {
                continue;
            }
            let (tx, rx) = rtrb::RingBuffer::new(INJECT_CAP);
            ev.routes.push(Route { on: l.on, to: ev.outlets.len() });
            ev.outlets.push(tx);
            inbox.push((l.node, l.port, rx));
        }
        let own = TempoMap::new(120.0, 4, f64::from(rate));
        let core = Core {
            graph: built.graph,
            out: built.out,
            knobs,
            inbox,
            clock,
            beat_key,
            follow: follow.clone(),
            part: Vec::with_capacity(EDGE_CAP),
            counters: ev.counters.clone(),
        };
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
            follow,
            counters: ev.counters.clone(),
            #[cfg(feature = "modules")]
            sink: None,
        })
    }

    /// `walk`: a patch of one random walk (seed, base, octaves, length from the node), knobs spread · rate · glide
    pub(super) fn walk(node: &Node, rate: f32, knobs: [Param; 3], beats: BeatReader, follow: Follow, ev: &mut Events) -> Result<Host> {
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
        Host::new(node, &patch, vec![spread, steps, glide], rate, beats, follow, ev)
    }

    /// `events`: the patch in the node's params, inputs a–d
    pub(super) fn events(node: &Node, rate: f32, inputs: [Param; 4], beats: BeatReader, follow: Follow, ev: &mut Events) -> Result<Host> {
        let patch = Patch::from_table(&node.params).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
        Host::new(node, &patch, inputs.into_iter().collect(), rate, beats, follow, ev)
    }
}

/// a world node with nothing to run: MIDI comes through the dispatcher and the event host
struct Quiet;

impl Block for Quiet {
    fn process(&mut self, _: &[Option<&Buf>], _: &mut [Buf], _: usize) {}
}

/// A `world` node's block (#430): a generator's patch with its event cables as its sink; a MIDI node, or a generator no
/// cable reaches (the event host joins only modules – a build without any), has nothing to run.
pub(super) fn world_block(node: &Node, index: usize, rate: f32, beats: BeatReader, follow: Follow, tuning: deck_events::Tuning, ev: &mut Events) -> Result<Box<dyn Block>> {
    #[cfg(feature = "modules")]
    {
        let links = ev.take_generator(index);
        if World::of(node).map_err(|e| anyhow!("{e}"))?.source == WorldSource::Generator && !links.is_empty() {
            let table = node.params.get("patch").and_then(|p| p.as_table()).expect("checked by World::of");
            let patch = Patch::from_table(table).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
            let sink = WorldOut::new(links, &mut ev.names, tuning)?;
            let mut host = Host::new(node, &patch, Vec::new(), rate, beats, follow, ev)?;
            host.sink = sink;
            return Ok(Box::new(host));
        }
    }
    #[cfg(not(feature = "modules"))]
    let _ = (node, index, rate, beats, follow, tuning, ev);
    Ok(Box::new(Quiet))
}

impl Block for Host {
    fn process(&mut self, _: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (map, epoch, start) = self.beats.transport().unwrap_or((self.own, 0, self.frame));
        self.frame = start + n as f64;
        let edge = match &mut self.src {
            Source::Block(core) => core.window(&map, epoch, start, n, &mut self.rt),
            Source::Ring(ring) => {
                // the events of this block's frames (an earlier one is late: it plays at once, counted); the next
                // block's stay in the ring. Following the transport: a note of an older epoch was computed before its
                // last edge (superseded: dropped), a newer one waits until this side sees that edge too; while the
                // transport stands nothing plays.
                let played = self.follow.block(map, epoch, start, n);
                let on = self.follow.on();
                self.rt.clear();
                let (from, to) = (start as u64, start as u64 + n as u64);
                let (mut late, mut capped) = (0, 0);
                while let Ok(t) = ring.peek() {
                    if on && t.epoch < epoch {
                        let _ = ring.pop();
                        continue;
                    }
                    if t.at >= to || (on && t.epoch > epoch) {
                        break;
                    }
                    let t = *t;
                    let _ = ring.pop();
                    let frame = t.at.saturating_sub(from) as u32;
                    if on && !played.at(frame as usize).running {
                        continue;
                    }
                    late += u64::from(t.at < from);
                    if self.rt.len() < self.rt.capacity() {
                        self.rt.push(RtEvent { frame, name: t.name, props: t.props, len: t.len });
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
                played.edge.map(|e| (e, played.split))
            }
        };
        #[cfg(feature = "modules")]
        if let Some(sink) = &mut self.sink {
            sink.send(&self.rt, start as u64);
            return;
        }
        let out = &mut outs[0];
        let (l, r) = out.lr_mut();
        match edge {
            // stopped on frame k: what began before it plays up to it, then every gate closes (all notes off)
            Some((Edge::Stop, k)) => {
                let keep = self.rt.partition_point(|e| (e.frame as usize) < k);
                self.cv.render(&self.rt[..keep], &mut l[..k], &mut r[..k]);
                self.cv.release();
                self.cv.render(&[], &mut l[k..n], &mut r[k..n]);
            }
            _ => self.cv.render(&self.rt, &mut l[..n], &mut r[..n]),
        }
    }
}
