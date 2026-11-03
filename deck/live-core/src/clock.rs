//! The browser deck's tempo clock (#321, plan §2.8 / §3.7): deck-core's `Controller` – sources, keys, the show clock's
//! filter, the leader's anchors – and its time base, in the worker next to the engine. The page and the worker only move
//! numbers in: the relay's pongs and `tempo` anchors, Web MIDI's clock bytes, `getOutputTimestamp`'s stamps, the
//! picker's choice; and out: the state block (`STATE` f64), which says when to ping and what anchor to send.
//!
//! Time: the page's clock is `performance.timeOrigin + performance.now()` (ms since the epoch, the same in the page and
//! the worker); the controller's ns count from the first such time seen (+ 1 s, so early differences never underflow).
//! Frames: the engine of each loaded scene counts from its own 0; the clock counts every frame rendered since `lc_init`
//! (the order the worklet plays them) and moves the map to each new engine's axis (`Controller::rebind`).
use deck_core::tempo::{Config, Controller, Event, Key, Line, Moment, NoHost, Shared, Source, TimeBase, What};
use std::sync::Arc;

/// the state block: [heard ms, beat, bpm, per_bar, running, source, flags, offset ms, rtt ms, error ms, the anchor to
/// send: bpm, per_bar, beat, at (relay ms), running, click]
pub const STATE: usize = 16;
/// flags: the filter is locked, an anchor came, this deck leads, the MIDI clock is lost, a ping is due, an anchor to send
pub const LOCKED: u32 = 1;
pub const ANCHOR: u32 = 2;
pub const LEADING: u32 = 4;
pub const MIDI_LOST: u32 = 8;
pub const PING: u32 = 16;
pub const SEND: u32 = 32;

pub struct Clock {
    ctl: Controller,
    time: TimeBase,
    rate: f64,
    /// the page's ms of ns 0 (− 1 s): set by the first time seen
    origin: Option<f64>,
    /// frames rendered since `lc_init`, and that count when the loaded scene's engine started
    rendered: u64,
    base: u64,
    state: [f64; STATE],
}

impl Clock {
    /// A deck without a scene: 120 BPM in 4, following the show clock (a scene without a `tempo` node keeps it; with
    /// none on the relay it runs on its own).
    pub fn new(rate: f32) -> Clock {
        let mut ctl = Controller::new(&Config::default(), Arc::new(Shared::default()), f64::from(rate));
        ctl.set_source(Source::Relay, &mut NoHost);
        Clock { ctl, time: TimeBase::new(rate), rate: f64::from(rate), origin: None, rendered: 0, base: 0, state: [0.0; STATE] }
    }

    /// the page's ms → the controller's ns
    fn ns(&mut self, ms: f64) -> u64 {
        let o = *self.origin.get_or_insert(ms - 1000.0);
        ((ms - o).max(0.0) * 1e6) as u64
    }

    /// the controller's ns → the page's ms
    fn ms(&self, ns: f64) -> f64 {
        self.origin.unwrap_or(0.0) + ns * 1e-6
    }

    /// `frames` were rendered (every render, in the order the worklet plays them)
    pub fn rendered(&mut self, frames: usize) {
        self.rendered += frames as u64;
    }

    /// A scene loaded: its engine's map in `shared`, its frames from 0 now; its `tempo` node (if it has one) sets the
    /// clock – tempo, bar, source, click, lead – a scene without one keeps the clock as it is.
    pub fn loaded(&mut self, shared: Arc<Shared>, tempo: Option<Config>) {
        let moved = self.rendered - self.base;
        self.base = self.rendered;
        self.ctl.rebind(shared, moved as f64);
        if let Some(c) = tempo {
            self.ctl.configure(&c, &mut NoHost);
        }
    }

    /// `getOutputTimestamp`: the audio context's `context` s were heard at the page's `heard_ms`; the worklet's `skew` =
    /// its context frame minus the frames it played (silence before the first, a dropout's)
    pub fn stamp(&mut self, context: f64, heard_ms: f64, skew: f64) {
        let frame = context * self.rate - skew;
        if frame >= 0.0 {
            let ns = self.ns(heard_ms);
            self.time.block(frame.round() as u64, ns);
        }
    }

    pub fn pong(&mut self, t0_us: f64, t1_ms: f64, t2_ms: f64) {
        let (t0, t2) = (self.ns(t0_us * 1e-3), self.ns(t2_ms));
        self.ctl.push_pong(t0, t1_ms as i64, t2);
    }

    pub fn anchor(&mut self, a: deck_core::tempo::Anchor, arrived_ms: f64) {
        let at = self.ns(arrived_ms);
        self.ctl.push_anchor(a, at);
    }

    /// Web MIDI's clock bytes, stamped by the browser (`event.timeStamp` on the page's clock)
    pub fn midi(&mut self, bytes: [u8; 3], at_ms: f64) {
        let at_ns = self.ns(at_ms);
        self.ctl.push(Event { at_ns, what: What::Midi(bytes) });
    }

    pub fn joined(&mut self, now_ms: f64) {
        let at_ns = self.ns(now_ms);
        self.ctl.push(Event { at_ns, what: What::Joined });
    }

    /// the page's picker: `Source::ALL[source]`, leading or not
    pub fn choose(&mut self, source: usize, lead: bool) {
        if let Some(&s) = Source::ALL.get(source) {
            self.ctl.set_source(s, &mut NoHost);
        }
        self.ctl.set_lead(lead);
    }

    /// a key of the page (tap, faster, slower, start / stop, click), at the page's ms
    pub fn key(&mut self, k: usize, at_ms: f64) {
        const KEYS: [Key; 5] = [Key::Tap, Key::Faster, Key::Slower, Key::StartStop, Key::Click];
        if let Some(&k) = KEYS.get(k) {
            let at = self.ns(at_ms);
            self.ctl.key(k, at);
        }
    }

    /// the clock's line at the page's `now_ms` – deck-core's, the same text the native HUD shows (`Controller::status`)
    pub fn line(&mut self, now_ms: f64) -> String {
        let now = self.ns(now_ms);
        self.ctl.status(now)
    }

    /// One clock frame at the page's `now_ms`: the events in, the master followed, the map published to the engine; the
    /// state block out (see `STATE`). Before the first stamp the audio's frames run on the wall clock.
    pub fn frame(&mut self, now_ms: f64) -> &[f64; STATE] {
        let now = self.ns(now_ms);
        let rendered = self.rendered - self.base;
        let base = self.base as f64;
        let m = match self.time.moment(now, self.rendered) {
            // the engine's axis: its frames from the scene's load
            Some(m) => Moment { heard: m.heard - base, rendered: m.rendered - base, line: Line { f: m.line.f - base, ..m.line }, ..m },
            None => {
                let line = Line { t: now as f64 * 1e-9, f: rendered as f64, spf: 1.0 / self.rate };
                Moment { now_ns: now, rate: self.rate, heard: rendered as f64, rendered: rendered as f64, line }
            }
        };
        let n = self.ctl.frame(&m, &mut NoHost);
        let show = self.ctl.show(now);
        let ping = self.ctl.ping_due(now);
        let send = self.ctl.take_anchor();
        let flag = |on: bool, f: u32| if on { f } else { 0 };
        let flags = flag(show.locked, LOCKED) | flag(show.anchor, ANCHOR) | flag(show.leading, LEADING) | flag(show.midi_lost, MIDI_LOST) | flag(ping, PING) | flag(send.is_some(), SEND);
        let source = Source::ALL.iter().position(|&s| s == self.ctl.source).unwrap_or(0);
        let a = send.unwrap_or(deck_core::tempo::Anchor { bpm: 0.0, per_bar: 0, beat: 0.0, at: 0, running: false });
        self.state = [
            self.ms(m.line.time_of(m.heard) * 1e9),
            n.pos.beat,
            n.bpm,
            f64::from(n.per_bar),
            f64::from(u8::from(n.running)),
            source as f64,
            f64::from(flags),
            show.offset_ms,
            show.rtt_ms,
            show.error_ms,
            a.bpm,
            f64::from(a.per_bar),
            a.beat,
            a.at as f64,
            f64::from(u8::from(a.running)),
            f64::from(u8::from(self.ctl.click)),
        ];
        &self.state
    }

    pub fn state(&self) -> &[f64; STATE] {
        &self.state
    }

    pub fn controller(&self) -> &Controller {
        &self.ctl
    }
}
