//! The dispatcher (#236, research/EVENTS.md §16 "events first, playback second"): adapters (MIDI, the network, the
//! story …) push events into one bounded MPSC queue (`Inlet::send` never blocks – full = dropped and counted); a
//! high-priority thread (not the audio thread) owns the routing table (deck-events route.rs) and fans each event out
//! to the receivers that listen to its name or domain – today the hosts' `listen` inputs, one SPSC ring (rtrb) each,
//! drained by the sequencer (or by the block, in the in-block fallback) before its next window. Routing never waits
//! for a receiver: a full ring drops that copy and counts it.
//!
//! The table is immutable; a patch change builds a new one (with the new hosts' rings) and swaps it whole
//! (`Inlet::swap`): the swap travels through the same queue, so every event before it goes by the old table and every
//! one after it by the new – no lock anywhere on the hot path, no table mutated while in use. Names are interned at
//! load (the engine's `Names`); a name interned later has no receivers.
//!
//! The drop counters (`Counters`: capped, past the hop limit, refused – compound events, send a reference –, late)
//! are shared atomics; the HUD shows them next to the glitch counter.
use super::glitch::Glitches;
use super::sequencer::{self, Sequencer};
use super::Sequencing;
use crate::tempo::BeatReader;
use deck_core::tempo::TempoMap;
use deck_events::{Event, Names, Route, Stats, Table};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;

/// events waiting for the dispatcher at most (more are dropped and counted)
pub const QUEUE: usize = 1024;

/// What happened to events on their way (since the start), for the HUD.
#[derive(Debug, Default)]
pub struct Counters {
    /// copies delivered to a receiver
    pub routed: AtomicU64,
    /// over a queue's, a ring's or an edge's cap
    pub capped: AtomicU64,
    /// past the hop limit (deck-events MAX_HOPS)
    pub hops: AtomicU64,
    /// refused: compound events / payloads (send a reference)
    pub refused: AtomicU64,
    /// a window the render had already passed (sequencer.rs), or an event its block took late
    pub late: AtomicU64,
}

impl Counters {
    pub fn add(&self, s: Stats) {
        for (c, n) in [(&self.capped, s.capped), (&self.hops, s.hops), (&self.refused, s.refused)] {
            if n > 0 {
                c.fetch_add(n, Ordering::Relaxed);
            }
        }
    }

    /// every event lost on the way: capped + hops + refused + late
    pub fn dropped(&self) -> u64 {
        [&self.capped, &self.hops, &self.refused, &self.late].iter().map(|c| c.load(Ordering::Relaxed)).sum()
    }
}

/// The events side of an engine's build (`Handles::events`): the name dictionary every host interned into, where the
/// hosts' generators run, their `listen` inputs (routes + rings) and – on the thread – the sequencer holding them.
pub struct Events {
    pub names: Names,
    pub(super) mode: Sequencing,
    pub(super) routes: Vec<Route>,
    pub(super) outlets: Vec<rtrb::Producer<Event>>,
    sequencer: Option<Sequencer>,
    pub counters: Arc<Counters>,
    glitches: Arc<Glitches>,
}

impl Events {
    pub(super) fn new(mode: Sequencing, glitches: Arc<Glitches>) -> Events {
        Events {
            names: Names::new(),
            mode,
            routes: Vec::new(),
            outlets: Vec::new(),
            sequencer: None,
            counters: Arc::new(Counters::default()),
            glitches,
        }
    }

    pub fn mode(&self) -> Sequencing {
        self.mode
    }

    /// the sequencer the thread-mode hosts join (made with the first)
    pub(super) fn sequencer(&mut self, beats: &BeatReader, own: TempoMap) -> &mut Sequencer {
        let (glitches, counters) = (&self.glitches, &self.counters);
        self.sequencer.get_or_insert_with(|| {
            let beats = BeatReader { shared: beats.shared.clone(), frames: beats.frames.clone() };
            Sequencer::new(beats, own, glitches.clone(), counters.clone())
        })
    }

    /// the hosts' sequencer (thread mode, a scene with a host) – to `spawn`, or to `fill` by hand (tests)
    pub fn take_sequencer(&mut self) -> Option<Sequencer> {
        self.sequencer.take()
    }

    /// the routing for a dispatcher: the table over every name interned so far, the hosts' input rings (taken)
    pub fn take_routing(&mut self) -> Routing {
        let outlets = std::mem::take(&mut self.outlets);
        Routing { table: Arc::new(Table::new(&self.names, &self.routes, outlets.len())), outlets }
    }

    /// Start the threads a scene with hosts needs: the sequencer (thread mode) and the dispatcher. None without hosts.
    pub fn start(&mut self) -> std::io::Result<Option<Running>> {
        let seq = self.take_sequencer();
        if seq.is_none() && self.routes.is_empty() {
            return Ok(None);
        }
        let seq = seq.map(Sequencer::spawn).transpose()?;
        let (d, inlet) = Dispatcher::new(self.take_routing(), self.counters.clone());
        Ok(Some(Running { inlet, counters: self.counters.clone(), _dispatch: d.spawn()?, _seq: seq }))
    }
}

/// a routing table and the receivers' rings it names (outlet i = `outlets[i]`)
pub struct Routing {
    pub table: Arc<Table>,
    outlets: Vec<rtrb::Producer<Event>>,
}

enum Msg {
    Event(Event),
    Swap(Routing),
}

/// An adapter's handle on the dispatcher (clone one per adapter).
#[derive(Clone)]
pub struct Inlet {
    tx: SyncSender<Msg>,
    counters: Arc<Counters>,
}

impl Inlet {
    /// queue `e` for routing; false = the queue is full (counted) or the dispatcher is gone. Never blocks.
    pub fn send(&self, e: Event) -> bool {
        match self.tx.try_send(Msg::Event(e)) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.counters.capped.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// a patch change: the new table replaces the old one after the events queued before it (waits for room – a
    /// patch change is no adapter's hot path)
    pub fn swap(&self, r: Routing) -> bool {
        self.tx.send(Msg::Swap(r)).is_ok()
    }
}

pub struct Dispatcher {
    rx: Receiver<Msg>,
    routing: Routing,
    counters: Arc<Counters>,
    stats: Stats,
}

impl Dispatcher {
    pub fn new(routing: Routing, counters: Arc<Counters>) -> (Dispatcher, Inlet) {
        let (tx, rx) = sync_channel(QUEUE);
        (Dispatcher { rx, routing, counters: counters.clone(), stats: Stats::default() }, Inlet { tx, counters })
    }

    fn handle(&mut self, m: Msg) {
        match m {
            Msg::Swap(r) => self.routing = r,
            Msg::Event(e) => {
                let outlets = &mut self.routing.outlets;
                let n = self.routing.table.route(e, &mut self.stats, |o, e| outlets.get_mut(o).is_some_and(|r| r.push(e).is_ok()));
                if n > 0 {
                    self.counters.routed.fetch_add(n as u64, Ordering::Relaxed);
                }
                if self.stats != Stats::default() {
                    self.counters.add(std::mem::take(&mut self.stats));
                }
            }
        }
    }

    /// route everything queued now; returns how many messages (the thread's loop, and the tests')
    pub fn drain(&mut self) -> usize {
        let mut n = 0;
        while let Ok(m) = self.rx.try_recv() {
            self.handle(m);
            n += 1;
        }
        n
    }

    /// The dispatcher on a thread of its own ("events-dispatch", high priority, not the audio thread's): it wakes on
    /// each event. Dropping the handle stops it.
    pub fn spawn(mut self) -> std::io::Result<Thread> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::Builder::new().name("events-dispatch".into()).spawn(move || {
            if let Err(e) = super::priority::raise_high() {
                tracing::warn!("events: the dispatcher at normal priority ({e})");
            }
            while !flag.load(Ordering::Relaxed) {
                match self.rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(m) => self.handle(m),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })?;
        Ok(Thread { stop, handle: Some(handle) })
    }
}

/// the running dispatcher: dropping it stops the thread
pub struct Thread {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Thread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// a scene's event threads; dropping it stops them
pub struct Running {
    pub inlet: Inlet,
    pub counters: Arc<Counters>,
    _dispatch: Thread,
    _seq: Option<sequencer::Thread>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_events::Match;

    /// a routing with `outlets` rings of 64 and their consumers
    fn routing(names: &Names, routes: &[Route], outlets: usize) -> (Routing, Vec<rtrb::Consumer<Event>>) {
        let (tx, rx): (Vec<_>, Vec<_>) = (0..outlets).map(|_| rtrb::RingBuffer::new(64)).unzip();
        (Routing { table: Arc::new(Table::new(names, routes, outlets)), outlets: tx }, rx)
    }

    #[test]
    fn a_swap_takes_effect_between_the_events_around_it() {
        let mut n = Names::new();
        let (note, pitch) = (n.event("music.note").unwrap(), n.key("pitch").unwrap());
        let (old, mut a) = routing(&n, &[Route { on: Match::Name(note), to: 0 }], 1);
        let (new, mut b) = routing(&n, &[Route { on: Match::Name(note), to: 0 }], 1);
        let (mut d, inlet) = Dispatcher::new(old, Arc::default());
        assert!(inlet.send(Event::now(note).with(pitch, 1.0)));
        assert!(inlet.swap(new));
        assert!(inlet.send(Event::now(note).with(pitch, 2.0)));
        assert_eq!(d.drain(), 3);
        assert_eq!(a[0].pop().unwrap().props.get(pitch), Some(1.0), "before the swap: the old table");
        assert!(a[0].pop().is_err());
        let e = b[0].pop().unwrap();
        assert_eq!((e.props.get(pitch), e.hops), (Some(2.0), 1), "after it: the new one; one hop");
        assert_eq!(d.counters.routed.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn full_queues_and_rings_drop_and_count() {
        let mut n = Names::new();
        let note = n.event("music.note").unwrap();
        let (r, _rx) = routing(&n, &[Route { on: Match::Name(note), to: 0 }], 1);
        let (mut d, inlet) = Dispatcher::new(r, Arc::default());
        let sent = (0..QUEUE + 5).filter(|_| inlet.send(Event::now(note))).count();
        assert_eq!(sent, QUEUE, "the queue holds QUEUE, the rest is refused at once");
        d.drain();
        let c = d.counters.clone();
        // 5 over the queue, then 64 fit the ring and the rest of QUEUE is over its cap
        assert_eq!(c.routed.load(Ordering::Relaxed), 64);
        assert_eq!(c.capped.load(Ordering::Relaxed), 5 + QUEUE as u64 - 64);
        inlet.send(Event::now(note).containing(vec![Event::now(note)]));
        d.drain();
        assert_eq!(c.refused.load(Ordering::Relaxed), 1, "a compound event: send a reference");
    }

    /// the dispatcher thread wakes on each event: the time from `send` until the event is in its receiver's ring
    #[test]
    fn the_dispatcher_thread_routes_within_a_fraction_of_a_block() {
        let mut n = Names::new();
        let note = n.event("music.note").unwrap();
        let (r, mut rx) = routing(&n, &[Route { on: Match::Name(note), to: 0 }], 1);
        let (d, inlet) = Dispatcher::new(r, Arc::default());
        let _thread = d.spawn().unwrap();
        let mut us: Vec<f64> = (0..500)
            .map(|_| {
                let t0 = std::time::Instant::now();
                inlet.send(Event::now(note));
                while rx[0].pop().is_err() {
                    assert!(t0.elapsed().as_secs() < 2, "routed");
                    std::hint::spin_loop();
                }
                t0.elapsed().as_secs_f64() * 1e6
            })
            .collect();
        us.sort_by(f64::total_cmp);
        let (median, p99) = (us[us.len() / 2], us[us.len() * 99 / 100]);
        println!("dispatcher: send → receiver's ring median {median:.1} µs, p99 {p99:.1} µs");
        // a block is 5333 µs at 48 kHz; a loaded CI machine may be slow, but not a block slow at the median
        assert!(median < 2_000.0, "median {median} µs");
    }
}
