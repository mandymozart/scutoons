//! The browser deck's tempo clock (#321, plan §2.8 / §3.7): deck-core's `Controller` – sources, keys, the show clock's
//! filter, the leader's anchors – and its time base, in the worker next to the engine. The page and the worker only move
//! numbers in: the relay's pongs and `tempo` anchors, Web MIDI's clock bytes, `getOutputTimestamp`'s stamps, the
//! picker's choice; and out: the state block (`STATE` f64), which says when to ping and what anchor to send.
//!
//! Time: the page's clock is `performance.timeOrigin + performance.now()` (ms since the epoch, the same in the page and
//! the worker); the controller's ns count from the first such time seen (+ 1 s, so early differences never underflow).
//! Frames: the engine of each loaded scene counts from its own 0; the clock counts every frame rendered since `lc_init`
//! (the order the worklet plays them) and moves the map to each new engine's axis (`Controller::rebind`).
//!
//! MIDI clock out (the page's MIDI OUT): deck-core's `ClockOut` – the native deck's schedule – on the map and line each
//! frame leaves, `OUT_AHEAD_S` ahead; each message with the page's ms it is due at. The page writes the bytes to Web
//! MIDI with that timestamp and decides nothing.
use deck_core::tempo::{ClockMode, ClockOut, Config, Controller, Event, Key, Line, Moment, NoHost, Shared, Source, TimeBase, What};
use std::sync::Arc;

/// the state block: [heard ms, beat, bpm, per_bar, running, source, flags, offset ms, rtt ms, error ms, the anchor to
/// send: bpm, per_bar, beat, at (relay ms), running, click, the picker's choice (`ClockMode::ALL`'s index, -1: none of
/// them), the MIDI clock out on, the strobe's flash: its beat edge (page ms) and length (ms; 0: no flash)] (#370)
pub const STATE: usize = 20;
/// flags: the filter is locked, an anchor came, this deck leads, the MIDI clock is lost, a ping is due, an anchor to send
pub const LOCKED: u32 = 1;
pub const ANCHOR: u32 = 2;
pub const LEADING: u32 = 4;
pub const MIDI_LOST: u32 = 8;
pub const PING: u32 = 16;
pub const SEND: u32 = 32;
/// how far ahead the clock out hands its messages to the page (s): more than the worker's longest pause between two
/// frames (the S22: ~75 ms), so no tick is late; a tempo nudge moves the ones already handed out by at most 2 % of it
pub const OUT_AHEAD_S: f64 = 0.15;

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
    /// the MIDI clock out while the page has it on, and its messages of the last frame: [page ms, status, d1, d2] each
    /// (NaN = no such data byte)
    out: Option<ClockOut>,
    out_msgs: Vec<f64>,
}

impl Clock {
    /// A deck without a scene: 120 BPM in 4, following the show clock (a scene without a `tempo` node keeps it; with
    /// none on the relay it runs on its own).
    pub fn new(rate: f32) -> Clock {
        let mut ctl = Controller::new(&Config::default(), Arc::new(Shared::default()), f64::from(rate));
        ctl.set_source(Source::Relay, &mut NoHost);
        Clock { ctl, time: TimeBase::new(rate), rate: f64::from(rate), origin: None, rendered: 0, base: 0, state: [0.0; STATE], out: None, out_msgs: Vec::new() }
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

    /// a master and the ask to lead as they are: `Source::ALL[source]` (the headless host's, a test's)
    pub fn choose(&mut self, source: usize, lead: bool) {
        if let Some(&s) = Source::ALL.get(source) {
            self.ctl.set_source(s, &mut NoHost);
        }
        self.ctl.set_lead(lead);
    }

    /// the page's picker: `ClockMode::ALL[mode]` (#370)
    pub fn set_mode(&mut self, mode: usize) {
        if let Some(&m) = ClockMode::ALL.get(mode) {
            self.ctl.set_mode(m, &mut NoHost);
        }
    }

    /// a key of the page (tap, faster, slower, start / stop, click), at the page's ms
    pub fn key(&mut self, k: usize, at_ms: f64) {
        const KEYS: [Key; 5] = [Key::Tap, Key::Faster, Key::Slower, Key::StartStop, Key::Click];
        if let Some(&k) = KEYS.get(k) {
            let at = self.ns(at_ms);
            self.ctl.key(k, at);
        }
    }

    /// the page's MIDI OUT on / off (off: the next start begins with Start or the song position again)
    pub fn set_out(&mut self, on: bool) {
        if on != self.out.is_some() {
            self.out = on.then(ClockOut::default);
        }
    }

    /// the clock out's messages of the last frame: [page ms, status, d1, d2] each
    pub fn out(&self) -> &[f64] {
        &self.out_msgs
    }

    /// the clock's line at the page's `now_ms` – deck-core's, the same text the native HUD shows (`Controller::status`)
    pub fn line(&mut self, now_ms: f64) -> String {
        let now = self.ns(now_ms);
        self.ctl.status(now)
    }

    /// the clock in one line at the page's `now_ms` – the tempo line and the source's state, the panel's (deck-core's
    /// `Controller::line`: the native HUD's two clock lines' texts, #370)
    pub fn hud_line(&mut self, now_ms: f64) -> String {
        let now = self.ns(now_ms);
        self.ctl.line(now)
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
        self.out_msgs.clear();
        // on MIDI clock in no clock out: the browser listens and sends on every port, its own clock would come back
        // through a loopback port (loopMIDI, the IAC bus) as the clock it follows
        if self.ctl.source != Source::Midi {
            if let Some(co) = self.out.as_mut() {
                let (map, t) = (self.ctl.map(), now as f64 * 1e-9);
                while let Some((due, o)) = co.peek(&map, &m.line, t, 0.0).filter(|(due, _)| *due <= t + OUT_AHEAD_S) {
                    let b = o.bytes();
                    let byte = |i: usize| b.get(i).map_or(f64::NAN, |&x| f64::from(x));
                    let at = self.origin.unwrap_or(0.0) + due.max(t) * 1e3;
                    self.out_msgs.extend([at, byte(0), byte(1), byte(2)]);
                    co.sent(o);
                }
            }
        }
        let show = self.ctl.show(now);
        let ping = self.ctl.ping_due(now);
        let send = self.ctl.take_anchor();
        let flag = |on: bool, f: u32| if on { f } else { 0 };
        let flags = flag(show.locked, LOCKED) | flag(show.anchor, ANCHOR) | flag(show.leading, LEADING) | flag(show.midi_lost, MIDI_LOST) | flag(ping, PING) | flag(send.is_some(), SEND);
        let source = Source::ALL.iter().position(|&s| s == self.ctl.source).unwrap_or(0);
        let a = send.unwrap_or(deck_core::tempo::Anchor { bpm: 0.0, per_bar: 0, beat: 0.0, at: 0, running: false });
        let heard_ms = self.ms(m.line.time_of(m.heard) * 1e9);
        let mode = self.ctl.mode().and_then(|m| ClockMode::ALL.iter().position(|&x| x == m)).map_or(-1.0, |i| i as f64);
        let (edge, dur) = n.flash().map_or((0.0, 0.0), |f| (heard_ms + f.edge_s * 1e3, f.dur_s * 1e3));
        self.state = [
            heard_ms,
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
            mode,
            f64::from(u8::from(self.out.is_some())),
            edge,
            dur,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// the clock out's messages over `secs` of 25 ms frames (the worker's), the audio rendering along: (page ms, bytes)
    fn run(c: &mut Clock, from_ms: f64, secs: f64) -> Vec<(f64, Vec<u8>)> {
        let mut got = vec![];
        let mut t = from_ms;
        while t < from_ms + secs * 1e3 {
            c.rendered(1200);
            c.frame(t);
            for m in c.out().chunks(4) {
                got.push((m[0], m[1..].iter().filter(|b| !b.is_nan()).map(|&b| b as u8).collect()));
            }
            t += 25.0;
        }
        got
    }

    #[test]
    fn the_state_block_says_the_choice_the_clock_out_and_the_flash() {
        let mut c = Clock::new(48_000.0);
        let t0 = 1_762_000_000_000.0;
        assert_eq!(c.frame(t0)[16], 0.0, "a new deck follows the show clock");
        for i in [1, 2, 3, 0] {
            c.set_mode(i);
            assert_eq!(c.frame(t0)[16], i as f64, "{:?}", ClockMode::ALL[i]);
        }
        // a hand on the clock takes over from the show clock: the deck's own now, and the block says so
        c.key(1, t0 + 10.0);
        assert_eq!(c.frame(t0 + 25.0)[16], 1.0);
        c.set_out(true);
        run(&mut c, t0 + 50.0, 1.0);
        let s = *c.state();
        assert_eq!(s[17], 1.0, "the clock out is on");
        // the flash: 60 ms on a beat edge at most 0.1 s past the heard moment, at most a beat (0.5 s at 121 BPM) before it
        assert_eq!(s[19], 60.0);
        assert!(s[18] - s[0] <= 100.0 + 1e-6 && s[18] - s[0] > -500.0, "{} vs {}", s[18], s[0]);
        c.key(3, t0 + 1100.0);
        assert_eq!(c.frame(t0 + 1125.0)[19], 0.0, "stopped: no flash");
        c.set_out(false);
        assert_eq!(c.frame(t0 + 1150.0)[17], 0.0);
        let line = c.hud_line(t0 + 1150.0);
        assert!(line.starts_with("BPM 121.0  BAR ") && line.ends_with(" STOP"), "{line}");
        c.set_mode(2);
        c.frame(t0 + 1175.0);
        assert!(c.hud_line(t0 + 1175.0).ends_with(" STOP  LEADS"), "{}", c.hud_line(t0 + 1175.0));
    }

    #[test]
    fn the_browser_clock_out_is_the_native_schedule_ahead_of_time() {
        let mut c = Clock::new(48_000.0);
        c.choose(0, false); // the deck's own clock, 120 BPM
        let t0 = 1_762_000_000_000.0;
        c.frame(t0);
        c.set_out(true);
        let got = run(&mut c, t0 + 25.0, 2.0);
        // a start (or the song position + continue), then ticks 20.83 ms apart, each handed out once, in time order
        assert!(matches!(got[0].1[0], 0xFA | 0xF2), "{:02X?}", got[0].1);
        let ticks: Vec<f64> = got.iter().filter(|(_, b)| b == &[0xF8]).map(|(t, _)| *t).collect();
        assert!(ticks.len() >= 90, "2 s at 120 BPM: {}", ticks.len());
        for w in ticks.windows(2) {
            assert!((w[1] - w[0] - 1000.0 / 48.0).abs() < 0.05, "spacing {} ms", w[1] - w[0]);
        }
        // never late: each due at most `OUT_AHEAD_S` after the frame that handed it out, never before that frame
        assert!(got.windows(2).all(|w| w[1].0 >= w[0].0));
        // stop: a Stop goes out; on MIDI clock in nothing (the browser would hear itself through a loopback port)
        c.key(3, t0 + 2100.0);
        let stop = run(&mut c, t0 + 2100.0, 0.2);
        assert!(stop.iter().any(|(_, b)| b == &[0xFC]), "{stop:?}");
        c.choose(1, false);
        c.midi([0xFA, 0, 0], t0 + 2399.0);
        for k in 0..48 {
            c.midi([0xF8, 0, 0], t0 + 2400.0 + f64::from(k) * 1000.0 / 48.0);
        }
        assert!(run(&mut c, t0 + 2400.0, 1.0).is_empty());
        assert!(c.controller().now().running, "the MIDI clock runs – and goes out nowhere");
        c.choose(0, false);
        c.set_out(false);
        assert!(run(&mut c, t0 + 3500.0, 0.5).is_empty());
    }
}
