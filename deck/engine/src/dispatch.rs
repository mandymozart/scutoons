//! The events side of an engine's build (#236, research/EVENTS.md §16 "events first, playback second"): the name
//! dictionary every host interned into, the hosts' `listen` inputs (routes + one SPSC ring each) and – in thread mode –
//! the sequencer holding the generators. The routing itself (the dispatcher's queue and thread) is the platform's
//! scheduling (deck/native's audio/dispatch.rs): it takes `Routing` out of `Events::take_routing`.
//!
//! The drop counters (`Counters`: capped, past the hop limit, refused – compound events, send a reference –, late)
//! are shared atomics; the HUD shows them next to the glitch counter.
use super::sequencer::Sequencer;
use super::Sequencing;
use deck_core::tempo::BeatReader;
use deck_core::tempo::TempoMap;
use deck_events::{Event, Names, Route, Stats, Table};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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
    /// the module event outputs' deck-events events (#342, evhost.rs) for the platform to route
    pub(super) outbox: Vec<rtrb::Consumer<Event>>,
    /// the slot of each outbox ring, `<node>.<port>` (#442: what another deck may ask for)
    pub(super) outbox_slots: Vec<String>,
    /// #442: the far world nodes' cables (evhost.rs): (the address, its `listen`, the ring into the cabled input) –
    /// the platform's link fills them with the events another deck sends
    pub(super) far: Vec<(String, Vec<String>, rtrb::Producer<Event>)>,
    /// (world node, its cables) of the `generator` world nodes (#430): taken by the node's block
    #[cfg(feature = "modules")]
    pub(super) generators: Vec<(usize, Vec<super::evhost::Link>)>,
    sequencer: Option<Sequencer>,
    pub counters: Arc<Counters>,
}

impl Events {
    pub(super) fn new(mode: Sequencing) -> Events {
        Events {
            names: Names::new(),
            mode,
            routes: Vec::new(),
            outlets: Vec::new(),
            outbox: Vec::new(),
            outbox_slots: Vec::new(),
            far: Vec::new(),
            #[cfg(feature = "modules")]
            generators: Vec::new(),
            sequencer: None,
            counters: Arc::new(Counters::default()),
        }
    }

    pub fn mode(&self) -> Sequencing {
        self.mode
    }

    /// the sequencer the thread-mode hosts join (made with the first)
    pub(super) fn sequencer(&mut self, beats: &BeatReader, own: TempoMap) -> &mut Sequencer {
        let counters = &self.counters;
        self.sequencer.get_or_insert_with(|| {
            let beats = BeatReader { shared: beats.shared.clone(), frames: beats.frames.clone() };
            Sequencer::new(beats, own, counters.clone())
        })
    }

    /// the hosts' sequencer (thread mode, a scene with a host) – to `spawn`, or to `fill` by hand (tests)
    pub fn take_sequencer(&mut self) -> Option<Sequencer> {
        self.sequencer.take()
    }

    /// whether any host listens to events (a dispatcher is needed)
    pub fn has_routes(&self) -> bool {
        !self.routes.is_empty()
    }

    /// What the modules' event outputs emit (#342): one ring per output port, deck-events events with their true time
    /// (a note with its `dur`) – the platform drains them into its dispatcher. Taken once.
    pub fn take_outbox(&mut self) -> Vec<rtrb::Consumer<Event>> {
        self.outbox_slots.clear();
        std::mem::take(&mut self.outbox)
    }

    /// `take_outbox` with each ring's slot, `<node>.<port>` (#442: the link offers them to other decks). Taken once.
    pub fn take_outbox_slots(&mut self) -> Vec<(String, rtrb::Consumer<Event>)> {
        let slots = std::mem::take(&mut self.outbox_slots);
        slots.into_iter().zip(std::mem::take(&mut self.outbox)).collect()
    }

    /// #442: the far world nodes' cables – (the address, its `listen`, the ring into the cabled input), one per cable;
    /// the platform's link pushes what the named deck sends. Taken once.
    pub fn take_far(&mut self) -> Vec<(String, Vec<String>, rtrb::Producer<Event>)> {
        std::mem::take(&mut self.far)
    }

    /// the cables of the generator world node `node` (#430), taken once
    #[cfg(feature = "modules")]
    pub(super) fn take_generator(&mut self, node: usize) -> Vec<super::evhost::Link> {
        let at = self.generators.iter().position(|(n, _)| *n == node);
        at.map(|i| self.generators.swap_remove(i).1).unwrap_or_default()
    }

    /// the routing for a dispatcher: the table over every name interned so far, the hosts' input rings (taken)
    pub fn take_routing(&mut self) -> Routing {
        let outlets = std::mem::take(&mut self.outlets);
        Routing { table: Arc::new(Table::new(&self.names, &self.routes, outlets.len())), outlets }
    }
}

/// a routing table and the receivers' rings it names (outlet i = `outlets[i]`)
pub struct Routing {
    pub table: Arc<Table>,
    pub outlets: Vec<rtrb::Producer<Event>>,
}
