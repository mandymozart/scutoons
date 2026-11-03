//! The sequencer thread (#236, research/EVENTS.md §16 "Threads"): the hosts' patches (walk.rs `Core`) run here, one
//! window of BLOCK frames at a time, ahead of the render – never on the audio thread. Each host's notes go into its
//! own lock-free SPSC ring (rtrb) as `Timed` events at absolute frames (the one timebase: the engine's frame counter,
//! clock.rs); the host's audio block takes only the events of the frames it renders.
//!
//! How far ahead (`horizon`): the device's buffer + the render lead + a block – the render thread renders in bursts of
//! about a device buffer (ring.rs) and the lead grows after a glitch, so the window always covers the next burst. A
//! live event (dispatch.rs) reaches the sound after this horizon + the render's own lead.
//!
//! Late: a window that ends before the render's frame has its events dropped and counted (`Counters::late`) – the
//! patch still runs it, so its state stays continuous. Full: while a host's ring has no room for a window, the
//! sequencer waits (no event is lost to it). Nothing here allocates per window (graph, resolve and rings are reserved).
use super::dispatch::Counters;
use super::walk::Core;
use deck_core::tempo::BeatReader;
use deck_core::tempo::TempoMap;
use deck_events::{Props, RtEvent, Sym, EDGE_CAP};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// frames per window: what the sequencer runs a patch for at a time (the native render thread's block is the same)
pub const BLOCK: usize = 256;

/// a host's ring, in events: well over the horizon's worth at the usual few notes a window; full = the sequencer waits
pub const SEQ_RING: usize = 4096;

/// a note for the audio block: an `RtEvent` at an absolute frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    pub at: u64,
    pub len: u32,
    pub name: Sym,
    pub props: Props,
}

struct SeqHost {
    core: Core,
    ring: rtrb::Producer<Timed>,
}

pub struct Sequencer {
    hosts: Vec<SeqHost>,
    beats: BeatReader,
    /// without a tempo map: 120 bpm from frame 0
    own: TempoMap,
    /// the next window's first frame (None until the first fill: then the render's frame)
    cursor: Option<u64>,
    /// the cursor, for other threads (`progress`)
    done: Arc<AtomicU64>,
    rt: Vec<RtEvent>,
    counters: Arc<Counters>,
}

impl Sequencer {
    pub(super) fn new(beats: BeatReader, own: TempoMap, counters: Arc<Counters>) -> Sequencer {
        Sequencer { hosts: Vec::new(), beats, own, cursor: None, done: Arc::default(), rt: Vec::with_capacity(EDGE_CAP), counters }
    }

    pub(super) fn add(&mut self, core: Core, ring: rtrb::Producer<Timed>) {
        self.hosts.push(SeqHost { core, ring });
    }

    pub fn hosts(&self) -> usize {
        self.hosts.len()
    }

    /// the frame up to which every host's events are in its ring – readable from any thread
    pub fn progress(&self) -> Arc<AtomicU64> {
        self.done.clone()
    }

    /// Windows until the cursor is `ahead` frames past what the engine has rendered (or a ring is too full for a
    /// window). Returns the windows run. Never allocates.
    pub fn fill(&mut self, ahead: u64) -> usize {
        let rendered = self.beats.frames.load(Ordering::Relaxed);
        let mut at = *self.cursor.get_or_insert(rendered);
        let mut windows = 0;
        let (mut late, mut capped) = (0, 0);
        while at < rendered + ahead && self.hosts.iter().all(|h| h.ring.slots() >= EDGE_CAP) {
            let map = self.beats.shared.map().map_or(self.own, |(m, _)| m);
            let behind = at + BLOCK as u64 <= rendered;
            for h in &mut self.hosts {
                h.core.window(&map, at as f64, BLOCK, &mut self.rt);
                if behind {
                    late += self.rt.len() as u64;
                    continue;
                }
                for e in &self.rt {
                    // room checked above for EDGE_CAP (an output's cap); only a compound event's children go past it
                    if h.ring.push(Timed { at: at + u64::from(e.frame), len: e.len, name: e.name, props: e.props }).is_err() {
                        capped += 1;
                    }
                }
            }
            at += BLOCK as u64;
            windows += 1;
        }
        self.cursor = Some(at);
        self.done.store(at, Ordering::Release);
        if late > 0 {
            self.counters.late.fetch_add(late, Ordering::Relaxed);
        }
        if capped > 0 {
            self.counters.capped.fetch_add(capped, Ordering::Relaxed);
        }
        windows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Build, Engine, Sequencing};
    use deck_space::graph::Graph;
    use deck_space::scene::Scene;

    const WALK: &str = "[[node]]\nid=\"tempo\"\nkind=\"tempo\"\nbpm=120\n[[node]]\nid=\"w\"\nkind=\"walk\"\nspread=1.0\nrate=48.0\n\
                        [[node]]\nid=\"v\"\nkind=\"fm\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n\
                        [[link]]\nfrom=\"w.cv\"\nto=\"v.cv\"\n[[link]]\nfrom=\"v\"\nto=\"out.in\"\n";

    fn build() -> (Engine, crate::Handles, Sequencer) {
        let g = Graph::build(&Scene::parse(WALK).unwrap()).unwrap();
        let (e, mut h) = Engine::build_seq(&g, 48_000.0, 1.0, Build::detect(), Sequencing::Thread).unwrap();
        let s = h.events.take_sequencer().unwrap();
        (e, h, s)
    }

    #[test]
    fn windows_the_render_has_passed_are_dropped_and_counted() {
        let (mut e, h, mut s) = build();
        assert_eq!(s.fill(0), 0, "nothing ahead of the render asked");
        let mut buf = vec![0.0f32; 4096 * 2];
        e.render(&mut buf, 2, |x| x);
        // the render is 16 blocks on, the sequencer still at 0: it runs those windows (its state stays continuous) but
        // their notes are late
        assert_eq!(s.fill(BLOCK as u64), 17);
        assert!(h.events.counters.late.load(Ordering::Relaxed) > 0, "the passed windows' notes are counted late");
        assert_eq!(s.progress().load(Ordering::Relaxed), 4096 + BLOCK as u64);
    }

    #[test]
    fn a_full_ring_makes_the_sequencer_wait() {
        let (_e, h, mut s) = build();
        // the render never takes anything: the sequencer runs until a window might not fit, then stops
        let windows = s.fill(u64::MAX / 2);
        assert!(windows > 100, "{windows}");
        assert!(s.hosts[0].ring.slots() < EDGE_CAP);
        assert_eq!(s.fill(u64::MAX / 2), 0, "still full: it waits");
        assert_eq!(h.events.counters.dropped(), 0, "waiting loses nothing");
    }
}
