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
//! patch still runs it, so its state stays continuous. The transport (#359): when its epoch moves (a start, a stop, a
//! position), the windows of the hosts that follow it are run again from the render on, on the new map – their notes
//! carry the epoch, the block drops the older ones (walk.rs); the hosts that don't follow keep theirs. Full: while a host's ring has no room for a window, the
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
    /// the transport epoch the note was computed on
    pub epoch: u64,
}

struct SeqHost {
    core: Core,
    ring: rtrb::Producer<Timed>,
    /// the frame up to which its windows are in its ring
    done: u64,
}

pub struct Sequencer {
    hosts: Vec<SeqHost>,
    beats: BeatReader,
    /// without a tempo map: 120 bpm from frame 0
    own: TempoMap,
    /// the next window's first frame (None until the first fill: then the render's frame)
    cursor: Option<u64>,
    /// the transport epoch of the last window
    epoch: Option<u64>,
    /// the cursor, for other threads (`progress`)
    done: Arc<AtomicU64>,
    rt: Vec<RtEvent>,
    counters: Arc<Counters>,
}

impl Sequencer {
    pub(super) fn new(beats: BeatReader, own: TempoMap, counters: Arc<Counters>) -> Sequencer {
        Sequencer { hosts: Vec::new(), beats, own, cursor: None, epoch: None, done: Arc::default(), rt: Vec::with_capacity(EDGE_CAP), counters }
    }

    pub(super) fn add(&mut self, core: Core, ring: rtrb::Producer<Timed>) {
        self.hosts.push(SeqHost { core, ring, done: 0 });
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
        // after a rewind: what lies before the render was played on the old epoch already – not again
        let mut floor = 0;
        while at < rendered + ahead && self.hosts.iter().all(|h| h.ring.slots() >= EDGE_CAP) {
            let (map, epoch) = self.beats.shared.transport().map_or((self.own, 0), |(m, _, e)| (m, e));
            if self.epoch.is_some_and(|was| was != epoch) && at > rendered {
                // the transport moved: back to the window the render is in (on the same grid), the followers again
                let back = (at - rendered).div_ceil(BLOCK as u64) * BLOCK as u64;
                at = at.saturating_sub(back);
                floor = rendered;
                for h in self.hosts.iter_mut().filter(|h| h.core.follows()) {
                    h.done = h.done.min(at);
                }
            }
            self.epoch = Some(epoch);
            let behind = at + BLOCK as u64 <= rendered;
            for h in &mut self.hosts {
                if at < h.done {
                    continue;
                }
                h.done = at + BLOCK as u64;
                h.core.window(&map, epoch, at as f64, BLOCK, &mut self.rt);
                if behind {
                    late += self.rt.len() as u64;
                    continue;
                }
                for e in self.rt.iter().filter(|e| at + u64::from(e.frame) >= floor) {
                    // room checked above for EDGE_CAP (an output's cap); only a compound event's children go past it
                    if h.ring.push(Timed { at: at + u64::from(e.frame), len: e.len, name: e.name, props: e.props, epoch }).is_err() {
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

    /// #359: a stop reaches the notes the sequencer computed ahead (they are dropped, the gate closes on the stop's frame)
    /// and a continue plays on from where it stood – on the thread exactly as in the block
    #[test]
    fn a_stop_silences_what_was_computed_ahead_and_a_continue_plays_on() {
        // a note every beat (120 BPM: 0.5 s), 0.9 of it long; a voice that releases in a few ms
        let scene = WALK.replace("rate=48.0", "rate=1.0").replace("kind=\"fm\"\n", "kind=\"fm\"\nrelease=0.002\n");
        let g = Graph::build(&Scene::parse(&scene).unwrap()).unwrap();
        let play = |mode: Sequencing| {
            let (mut e, mut h) = Engine::build_seq(&g, 48_000.0, 1.0, Build::detect(), mode).unwrap();
            let mut s = h.events.take_sequencer();
            let (mut all, mut buf) = (Vec::new(), vec![0.0f32; BLOCK * 2]);
            let (stop, go) = (28_800, 72_000); // 0.6 s (beat 1.2, inside the second note) · 1.5 s
            while all.len() < 120_000 {
                if let Some(s) = s.as_mut() {
                    s.fill(4096); // well ahead: the second note's whole length is in the ring before the stop
                }
                let at = h.clock.rendered();
                if at >= stop && at < stop + BLOCK as u64 || at >= go && at < go + BLOCK as u64 {
                    let (m, c) = h.tempo.map().unwrap();
                    h.tempo.publish(&m.with_running(at as f64, at >= go), c);
                }
                e.render(&mut buf, 2, |x| x);
                all.extend(buf.chunks_exact(2).map(|f| f[0]));
            }
            (all, h.events.counters.dropped())
        };
        let (thread, lost) = play(Sequencing::Thread);
        let (block, _) = play(Sequencing::InBlock);
        assert_eq!(lost, 0, "nothing late or capped");
        let peak = |v: &[f32]| v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak(&thread[24_000..28_000]) > 0.01, "the second note sounds");
        assert!(peak(&thread[31_000..72_000]) < 1e-4, "stopped: silent at once, not after the note's 0.95 s");
        // standing at beat 1.2 → the next step at beat 2, 0.4 s after the continue
        assert!(peak(&thread[72_000..90_000]) < 1e-4 && peak(&thread[92_000..110_000]) > 0.01, "continued on the beat where it stood");
        assert!(thread == block, "the thread plays what the block plays");
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
