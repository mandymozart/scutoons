//! The one extension point (research/EVENTS.md §3): a generator answers "which events start in this span?" – the
//! TidalCycles query model. Pure where it can be (its state + a seed), so asking for [0, 2) gives what [0, 1) and
//! [1, 2) give together, every deck with the same seed and tempo plays the same events, and a contract is a unit test.
//! Roles come from the ports, not from a class: a source has only outputs, a transformer both, a sink only inputs.
use crate::event::Event;
use deck_core::tempo::TempoMap;

/// The window being asked for: frames on the audio timebase [from_frame, to_frame) and the same span in beats on
/// `tempo` (`from == to` while the tempo stops – beat generators emit nothing, triggers still flow).
#[derive(Debug, Clone, Copy)]
pub struct Span<'a> {
    pub from_frame: f64,
    pub to_frame: f64,
    pub from: f64,
    pub to: f64,
    pub tempo: &'a TempoMap,
}

impl<'a> Span<'a> {
    pub fn frames(tempo: &'a TempoMap, from_frame: f64, to_frame: f64) -> Span<'a> {
        Span { from_frame, to_frame, from: tempo.beat_at(from_frame), to: tempo.beat_at(to_frame), tempo }
    }

    /// does a beat fall in [from, to)?
    pub fn has_beat(&self, beat: f64) -> bool {
        beat >= self.from && beat < self.to
    }
}

/// a generator's port names (an edge names them; graph.rs checks them)
#[derive(Debug, Clone, Copy)]
pub struct Ports {
    pub ins: &'static [&'static str],
    pub outs: &'static [&'static str],
}

pub trait Generator: Send {
    fn ports(&self) -> Ports;
    /// the events starting in `span`, given what arrived on the inputs for it (sorted by start), into `out`
    fn query(&mut self, span: &Span, ins: &Inbox, out: &mut Outbox);
}

/// what arrived for this span, per input port, sorted by start
pub struct Inbox<'a> {
    pub ports: &'a [Vec<Event>],
}

impl Inbox<'_> {
    pub fn port(&self, i: usize) -> &[Event] {
        self.ports.get(i).map_or(&[], Vec::as_slice)
    }
}

/// what a generator emits, per output port – at most `cap` an output a span (the rest is counted in `dropped`)
pub struct Outbox {
    ports: Vec<Vec<Event>>,
    cap: usize,
    pub dropped: u64,
}

impl Outbox {
    pub fn new(outs: usize, cap: usize) -> Outbox {
        Outbox { ports: (0..outs).map(|_| Vec::with_capacity(cap.min(64))).collect(), cap, dropped: 0 }
    }

    pub fn push(&mut self, port: usize, e: Event) {
        match self.ports.get_mut(port) {
            Some(p) if p.len() < self.cap => p.push(e),
            _ => self.dropped += 1,
        }
    }

    pub fn port(&self, i: usize) -> &[Event] {
        self.ports.get(i).map_or(&[], Vec::as_slice)
    }

    pub fn clear(&mut self) {
        for p in &mut self.ports {
            p.clear();
        }
    }
}
