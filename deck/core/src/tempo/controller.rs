//! The tempo controller (CYBERDECKS.md §8 "Two clocks", #181; moved here from deck/stage with #308, plan §2.8): one
//! source is the master at a time – internal (set, + / −, tap), MIDI clock in, Ableton Link, OSC, the relay's show
//! clock (a stub). Each frame it takes the events pushed since the last (stamped by the caller with its `now_ns`),
//! follows the master, re-anchors the map at the audio thread's frontier and publishes it.
//!
//! Platform-free: no thread, channel, clock or port in here. Time comes in as `Moment` / `Event::at_ns`, events by
//! `push`, and what only a platform can do (open a source's port, ask Ableton Link) is the `Host` the caller passes
//! to `frame`. deck/stage's `Tempo` is the native adapter (channels, midir, UDP, rusty_link); a browser page is
//! another.
use super::midi_in::{ClockIn, WallLine};
use super::shared::Shared;
use super::time::Moment;
use super::{clamp_bpm, Position, Tap, TempoMap};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Internal,
    Midi,
    Link,
    Osc,
    Relay,
}

impl Source {
    pub const ALL: [Source; 5] = [Source::Internal, Source::Midi, Source::Link, Source::Osc, Source::Relay];

    pub fn parse(s: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|x| x.name().eq_ignore_ascii_case(s))
    }

    pub fn name(self) -> &'static str {
        match self {
            Source::Internal => "internal",
            Source::Midi => "midi",
            Source::Link => "link",
            Source::Osc => "osc",
            Source::Relay => "relay",
        }
    }

    fn next(self) -> Source {
        let i = Source::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Source::ALL[(i + 1) % Source::ALL.len()]
    }
}

/// The deck's keys for the clock (main.rs maps them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Tap,
    Faster,
    Slower,
    StartStop,
    NextSource,
    Click,
}

/// What the steerable clock is told: by OSC, by the deck's keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cmd {
    Bpm(f64),
    Beat(f64),
    Start,
    Stop,
    Tap,
}

/// Something a source said, stamped (the caller's monotonic ns) when it arrived.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub at_ns: u64,
    pub what: What,
}

#[derive(Debug, Clone, Copy)]
pub enum What {
    /// a MIDI clock message (status byte first)
    Midi([u8; 3]),
    Osc(Cmd),
    Key(Key),
    /// the show clock's anchor (the relay, §3.7)
    Anchor(Anchor),
}

/// The conductor's tempo anchor (plan §3.7): at `at_ns` – already on the deck's monotonic clock, the relay's ms mapped
/// by the offset filter (#321) – the show was at `beat`, running at `bpm` (or stopped). Sent only on change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub bpm: f64,
    pub per_bar: u32,
    pub beat: f64,
    pub at_ns: u64,
    pub running: bool,
}

impl Anchor {
    fn line(&self) -> WallLine {
        WallLine { t: self.at_ns as f64 * 1e-9, beat: self.beat, bps: clamp_bpm(self.bpm) / 60.0, running: self.running }
    }
}

/// The scene's `tempo` node (one per scene; none = 120 BPM in 4, internal, the click off). The scene side builds it
/// (deck/stage's `tempo::config_from_graph`).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub bpm: f64,
    pub per_bar: u32,
    pub source: Source,
    pub click: bool,
    /// the MIDI input port for the clock in (a part of its name; "" = the first port)
    pub midi_in: String,
    /// the MIDI output port for the clock out (a part of its name); None = no clock out
    pub midi_out: Option<String>,
    /// the clock out's lead (s): the receiver's latency
    pub midi_out_lead: f64,
    /// where OSC listens
    pub osc: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bpm: 120.0,
            per_bar: 4,
            source: Source::Internal,
            click: false,
            midi_in: String::new(),
            midi_out: None,
            midi_out_lead: 0.0,
            osc: "0.0.0.0:9001".into(),
        }
    }
}

impl Config {
    /// the map the audio starts with (before the first frame publishes)
    pub fn map(&self, rate: f64) -> TempoMap {
        TempoMap::new(self.bpm, self.per_bar, rate)
    }
}

/// The clock at this frame's heard moment (the space reads it too).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Now {
    pub bpm: f64,
    pub pos: Position,
    pub per_bar: u32,
    pub running: bool,
}

/// What only the platform can do for the clock, handed to `Controller::frame`. All defaults do nothing.
pub trait Host {
    /// `s` just became the master (a key, `Controller::set_source`): open its port / session
    fn source_chosen(&mut self, _s: Source) {}
    /// Ableton Link's session as a line, if Link is open: its tempo and phase at `now_ns`
    fn link_line(&mut self, _now_ns: u64, _per_bar: u32) -> Option<WallLine> {
        None
    }
    /// set Link's session tempo; true = Link took it (the map then follows it back), false = no Link open
    fn link_set_tempo(&mut self, _bpm: f64) -> bool {
        false
    }
}

/// A host that does nothing (tests, the browser's internal clock).
pub struct NoHost;
impl Host for NoHost {}

/// The frame thread's tempo clock.
pub struct Controller {
    pub source: Source,
    pub click: bool,
    map: TempoMap,
    shared: Arc<Shared>,
    tap: Tap,
    midi: ClockIn,
    queue: VecDeque<Event>,
    /// the newest anchor of the show clock (followed while the relay is the master)
    anchor: Option<Anchor>,
    now: Now,
}

impl Controller {
    /// `shared` = the engine's (audio::Handles::tempo) or a fresh one without audio
    pub fn new(cfg: &Config, shared: Arc<Shared>, rate: f64) -> Controller {
        let map = cfg.map(rate);
        shared.publish(&map, cfg.click);
        Controller {
            source: Source::Internal,
            click: cfg.click,
            map,
            shared,
            tap: Tap::default(),
            midi: ClockIn::default(),
            queue: VecDeque::new(),
            anchor: None,
            now: Now { bpm: map.bpm, pos: map.position(0.0), per_bar: map.per_bar, running: true },
        }
    }

    /// a source's event (the platform adapters feed the clock through it)
    pub fn push(&mut self, e: Event) {
        self.queue.push_back(e);
    }

    /// the show clock's anchor, arrived at `at_ns` (the relay's transport adapter calls this; it is followed while
    /// `Source::Relay` is the master)
    pub fn push_anchor(&mut self, a: Anchor, at_ns: u64) {
        self.push(Event { at_ns, what: What::Anchor(a) });
    }

    /// has the relay's show clock said anything yet
    pub fn has_anchor(&self) -> bool {
        self.anchor.is_some()
    }

    /// a key of the deck, at `now_ns`
    pub fn key(&mut self, k: Key, now_ns: u64) {
        self.push(Event { at_ns: now_ns, what: What::Key(k) });
    }

    /// the map and line the audio and MIDI clock out read (#211: the outputs' strobe too)
    pub fn shared(&self) -> Arc<Shared> {
        self.shared.clone()
    }

    pub fn now(&self) -> Now {
        self.now
    }

    pub fn map(&self) -> TempoMap {
        self.map
    }

    /// the MIDI clock in has lost its clock at `now_ns`
    pub fn midi_lost(&self, now_ns: u64) -> bool {
        self.midi.lost(now_ns as f64 * 1e-9)
    }

    pub fn set_source(&mut self, s: Source, host: &mut dyn Host) {
        self.source = s;
        if s != Source::Relay {
            self.anchor = None;
        }
        host.source_chosen(s);
    }

    /// One frame: the events since the last, the master followed, the map re-anchored at the audio thread's frontier
    /// and published; the clock at the heard moment.
    pub fn frame(&mut self, m: &Moment, host: &mut dyn Host) -> Now {
        self.shared.publish_line(&m.line);
        while let Some(e) = self.queue.pop_front() {
            self.event(e, m, host);
        }
        let per_bar = self.map.per_bar;
        match self.source {
            Source::Midi if !self.midi.lost(m.now_ns as f64 * 1e-9) => {
                if let Some(l) = self.midi.line() {
                    self.map = l.to_map(&m.line, m.rate, per_bar);
                }
            }
            // the show clock: the newest anchor is a line, followed like the MIDI clock's
            Source::Relay => {
                if let Some(a) = self.anchor {
                    let mut map = a.line().to_map(&m.line, m.rate, a.per_bar.clamp(1, 32));
                    map.bpm = clamp_bpm(map.bpm);
                    self.map = map;
                }
            }
            // Link gives tempo and phase; start / stop stay the deck's own (stopped, the map stands where it stopped)
            Source::Link if self.map.running => {
                if let Some(l) = host.link_line(m.now_ns, per_bar) {
                    self.map = l.to_map(&m.line, m.rate, per_bar);
                }
            }
            // internal, OSC (a remote for the internal clock), a lost MIDI clock, a relay without an anchor yet: the map runs on
            _ => {}
        }
        self.map = self.map.at(m.rendered.max(m.heard));
        self.shared.publish(&self.map, self.click);
        self.now = Now { bpm: self.map.bpm, pos: self.map.position(m.heard), per_bar: self.map.per_bar, running: self.map.running };
        self.now
    }

    fn event(&mut self, e: Event, m: &Moment, host: &mut dyn Host) {
        let f = m.frames_at(e.at_ns);
        match e.what {
            What::Midi(b) => self.midi.message(e.at_ns as f64 * 1e-9, &b),
            What::Anchor(a) => self.anchor = Some(a),
            What::Osc(c) if self.source == Source::Osc => self.command(c, f, m, e.at_ns, host),
            What::Osc(_) => {}
            What::Key(Key::NextSource) => self.set_source(self.source.next(), host),
            What::Key(Key::Click) => self.click = !self.click,
            What::Key(k) => {
                // a hand on the clock takes over from a follower that can't be steered (MIDI in, the relay)
                if matches!(self.source, Source::Midi | Source::Relay) {
                    self.set_source(Source::Internal, host);
                }
                let c = match k {
                    Key::Tap => Cmd::Tap,
                    Key::Faster => Cmd::Bpm(self.map.bpm.round() + 1.0),
                    Key::Slower => Cmd::Bpm(self.map.bpm.round() - 1.0),
                    _ if self.map.running => Cmd::Stop,
                    _ => Cmd::Start,
                };
                self.command(c, f, m, e.at_ns, host);
            }
        }
    }

    /// a command on the steerable clock that came at frame `f`: a tempo, start / stop from the audio thread's frontier
    /// on (what is rendered stays – the audio never hears a jump), a beat or a tap's phase exactly where it happened
    fn command(&mut self, c: Cmd, f: f64, m: &Moment, at_ns: u64, host: &mut dyn Host) {
        let frontier = f.max(m.rendered);
        match c {
            Cmd::Bpm(b) => self.set_bpm(frontier, b, host),
            Cmd::Beat(b) => self.map = self.map.with_beat(f, b),
            Cmd::Start => self.map = self.map.with_running(frontier, true),
            Cmd::Stop => self.map = self.map.with_running(frontier, false),
            Cmd::Tap => {
                if let Some(bpm) = self.tap.tap(at_ns) {
                    self.set_bpm(frontier, bpm, host);
                    // the tap is a beat
                    let b = self.map.beat_at(f).round();
                    self.map = self.map.with_beat(f, b);
                }
            }
        }
    }

    fn set_bpm(&mut self, at: f64, bpm: f64, host: &mut dyn Host) {
        if self.source == Source::Link && host.link_set_tempo(clamp_bpm(bpm)) {
            return;
        }
        self.map = self.map.with_bpm(at, bpm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tempo::time::Line;

    const RATE: f64 = 48_000.0;

    /// a moment at `t` s on an exact 48 kHz line from 0, the audio `lead` frames ahead
    fn moment(t: f64, lead: f64) -> Moment {
        let line = Line { t: 0.0, f: 0.0, spf: 1.0 / RATE };
        let heard = t * RATE;
        Moment { now_ns: (t * 1e9) as u64, rate: RATE, heard, rendered: heard + lead, line }
    }

    fn at(t: f64, what: What) -> Event {
        Event { at_ns: (t * 1e9) as u64, what }
    }

    fn tempo(bpm: f64) -> Controller {
        Controller::new(&Config { bpm, ..Config::default() }, Arc::new(Shared::default()), RATE)
    }

    #[test]
    fn an_internal_tempo_change_keeps_the_beat_and_reaches_the_audio() {
        let mut t = tempo(60.0);
        let n = t.frame(&moment(1.5, 1024.0), &mut NoHost);
        assert!((n.pos.beat - 1.5).abs() < 1e-9 && n.pos.beat_in_bar == 1);
        // + at 2.0 s: from the audio frontier on at 61 BPM
        t.push(at(2.0, What::Key(Key::Faster)));
        let m = moment(2.0, 1024.0);
        let n = t.frame(&m, &mut NoHost);
        assert_eq!(n.bpm, 61.0);
        assert!((n.pos.beat - 2.0).abs() < 1e-3, "no jump: {}", n.pos.beat);
        let (shared, _) = t.shared.map().unwrap();
        assert_eq!(shared.frame, m.rendered, "anchored at the frontier");
        // what the audio thread rendered at 60 BPM stays: the new tempo starts where it stands
        assert!((shared.beat_at(m.rendered) - m.rendered / RATE).abs() < 1e-9);
        let later = t.frame(&moment(3.0, 1024.0), &mut NoHost).pos.beat;
        assert!((later - (2.0 + 61.0 / 60.0)).abs() < 1e-3, "{later}");
    }

    #[test]
    fn taps_set_tempo_and_phase() {
        let mut t = tempo(120.0);
        for i in 0..4 {
            t.push(at(10.0 + i as f64 * 0.6 + 0.013, What::Key(Key::Tap))); // 100 BPM, 13 ms after the grid
        }
        let n = t.frame(&moment(11.9, 512.0), &mut NoHost);
        assert!((n.bpm - 100.0).abs() < 1e-6, "{}", n.bpm);
        // the last tap (11.813 s) is a whole beat
        let b = t.map().beat_at(11.813 * RATE);
        assert!((b - b.round()).abs() < 1e-6, "{b}");
    }

    #[test]
    fn midi_clock_in_leads_and_a_key_takes_over() {
        let mut t = tempo(90.0);
        t.set_source(Source::Midi, &mut NoHost);
        t.push(at(4.99, What::Midi([0xFA, 0, 0])));
        for k in 0..96 {
            t.push(at(5.0 + k as f64 / 56.0, What::Midi([0xF8, 0, 0]))); // 140 BPM
        }
        let n = t.frame(&moment(5.0 + 95.0 / 56.0, 256.0), &mut NoHost);
        assert!((n.bpm - 140.0).abs() < 1e-6, "{}", n.bpm);
        assert!((n.pos.beat - 95.0 / 24.0).abs() < 1e-6, "{}", n.pos.beat);
        // the clock goes away: the stage runs on at its tempo
        let n = t.frame(&moment(9.0, 256.0), &mut NoHost);
        assert!((n.bpm - 140.0).abs() < 1e-6);
        // a key takes over as the internal clock, from where it is
        t.push(at(9.0, What::Key(Key::Faster)));
        let n = t.frame(&moment(9.0, 256.0), &mut NoHost);
        assert_eq!((t.source, n.bpm), (Source::Internal, 141.0));
    }

    #[test]
    fn osc_steers_only_as_the_master() {
        let mut t = tempo(120.0);
        t.push(at(1.0, What::Osc(Cmd::Bpm(80.0))));
        assert_eq!(t.frame(&moment(1.0, 0.0), &mut NoHost).bpm, 120.0, "not the master: ignored");
        t.set_source(Source::Osc, &mut NoHost);
        t.push(at(2.0, What::Osc(Cmd::Bpm(80.0))));
        t.push(at(2.0, What::Osc(Cmd::Beat(32.0))));
        t.frame(&moment(2.0, 0.0), &mut NoHost);
        t.push(at(2.5, What::Osc(Cmd::Stop)));
        t.frame(&moment(2.5, 0.0), &mut NoHost);
        let n = t.frame(&moment(3.0, 0.0), &mut NoHost);
        assert_eq!(n.bpm, 80.0);
        assert!(!n.running);
        assert!((n.pos.beat - (32.0 + 0.5 * 80.0 / 60.0)).abs() < 1e-6, "stopped at 2.5 s: {}", n.pos.beat);
        assert_eq!((n.pos.bar, n.pos.beat_in_bar), (8, 0));
        t.set_source(Source::Relay, &mut NoHost);
        assert_eq!(t.frame(&moment(4.0, 0.0), &mut NoHost).bpm, 80.0, "the relay stub: the clock stays");
    }

    #[test]
    fn keys_cycle_sources_and_toggle_the_click() {
        let mut t = tempo(120.0);
        t.push(at(1.0, What::Key(Key::NextSource)));
        t.push(at(1.0, What::Key(Key::Click)));
        t.frame(&moment(1.0, 0.0), &mut NoHost);
        assert_eq!(t.source, Source::Midi);
        assert!(t.shared.map().unwrap().1, "the click is on");
        t.push(at(1.0, What::Key(Key::StartStop)));
        assert!(!t.frame(&moment(1.1, 0.0), &mut NoHost).running);
    }

    /// a Link session stand-in: a fixed phase, takes tempo changes
    struct FakeLink {
        bpm: f64,
        opened: Vec<Source>,
    }

    impl Host for FakeLink {
        fn source_chosen(&mut self, s: Source) {
            self.opened.push(s);
        }
        fn link_line(&mut self, now_ns: u64, _per_bar: u32) -> Option<WallLine> {
            Some(WallLine { t: now_ns as f64 * 1e-9, beat: 8.0, bps: self.bpm / 60.0, running: true })
        }
        fn link_set_tempo(&mut self, bpm: f64) -> bool {
            self.bpm = bpm;
            true
        }
    }

    #[test]
    fn link_is_followed_and_steered_through_the_host() {
        let mut t = tempo(120.0);
        let mut link = FakeLink { bpm: 100.0, opened: vec![] };
        t.set_source(Source::Link, &mut link);
        assert_eq!(link.opened, [Source::Link]);
        let n = t.frame(&moment(1.0, 0.0), &mut link);
        assert!((n.bpm - 100.0).abs() < 1e-6 && (n.pos.beat - 8.0).abs() < 1e-6, "{} {}", n.bpm, n.pos.beat);
        // + goes to the session, the map follows it back on the next frame
        t.push(at(1.1, What::Key(Key::Faster)));
        t.frame(&moment(1.1, 0.0), &mut link);
        assert_eq!(link.bpm, 101.0);
        assert!((t.frame(&moment(1.2, 0.0), &mut link).bpm - 101.0).abs() < 1e-6);
    }

    #[test]
    fn the_relays_anchor_is_followed_and_a_key_takes_over() {
        let mut t = tempo(120.0);
        t.set_source(Source::Relay, &mut NoHost);
        assert!(!t.has_anchor());
        assert_eq!(t.frame(&moment(1.0, 0.0), &mut NoHost).bpm, 120.0, "no anchor yet: the clock stays");
        // at 2.0 s the show was at beat 16, 90 BPM, in 3
        let a = Anchor { bpm: 90.0, per_bar: 3, beat: 16.0, at_ns: 2_000_000_000, running: true };
        t.push_anchor(a, 2_100_000_000);
        let n = t.frame(&moment(3.0, 0.0), &mut NoHost);
        assert!(t.has_anchor());
        assert!((n.bpm - 90.0).abs() < 1e-6 && n.per_bar == 3, "{} {}", n.bpm, n.per_bar);
        assert!((n.pos.beat - (16.0 + 1.5)).abs() < 1e-6, "one second at 1.5 beats/s: {}", n.pos.beat);
        // a stopped anchor stands
        t.push_anchor(Anchor { running: false, beat: 20.0, ..a }, 3_100_000_000);
        let n = t.frame(&moment(4.0, 0.0), &mut NoHost);
        assert!(!n.running && (n.pos.beat - 20.0).abs() < 1e-6);
        // a hand on the clock takes over from the relay
        t.push(at(4.5, What::Key(Key::Faster)));
        t.frame(&moment(4.5, 0.0), &mut NoHost);
        assert_eq!(t.source, Source::Internal);
        assert!(!t.has_anchor());
    }
}
