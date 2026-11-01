//! The tempo clock (CYBERDECKS.md §8 "Two clocks", #181) on top of the time base: BPM, beat, bar and phase as a tempo
//! map over the audio frame position (map.rs – beat = f(audio position), never a second clock). One source is the
//! master at a time: internal (set, + / −, tap), MIDI clock in, Ableton Link (feature `link`), OSC, the relay's show
//! clock (a stub until the relay has one). The frame thread owns the map: each frame it takes the sources' events
//! (stamped on arrival, placed on the frame axis by the time base's line), follows the master, re-anchors the map at
//! the audio thread's frontier and publishes it – the click and the synced sequencer read it per block, MIDI clock out
//! per tick, the control domain per frame (the `tempo` node's outputs, quantised cuts).
//!
//! The map, quantize and tap tempo are deck-core's `tempo` module (#185); here: the sources' I/O (midir, UDP,
//! rusty_link), the followers and the hand-over across threads (map.rs).
pub mod map;
pub mod midi;
pub mod osc;
#[cfg(feature = "link")]
pub mod link;

pub use map::{position, BeatReader, Position, Quantize, Shared, TempoMap};

use crate::clock::{now_ns, Moment};
use crate::graph::{Graph, Kind};
use anyhow::{bail, Result};
use std::sync::mpsc::{channel, Receiver, Sender};
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

/// Something a source said, stamped (clock::now_ns) when it arrived.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub at_ns: u64,
    pub what: What,
}

#[derive(Debug, Clone, Copy)]
pub enum What {
    /// a MIDI clock message (status byte first)
    Midi([u8; 3]),
    Osc(osc::Cmd),
    Key(Key),
}

/// The scene's `tempo` node (one per scene; none = 120 BPM in 4, internal, the click off).
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
    pub fn from_graph(graph: &Graph) -> Result<Config> {
        let mut c = Config::default();
        let Some(i) = graph.first(Kind::Tempo) else { return Ok(c) };
        let n = &graph.nodes[i];
        c.bpm = map::clamp_bpm(n.f32("bpm", 120.0) as f64);
        c.per_bar = n.f32("beats", 4.0).clamp(1.0, 32.0) as u32;
        if let Some(s) = n.str("source") {
            let Some(src) = Source::parse(s) else {
                bail!("node '{}': unknown source '{s}' (internal, midi, link, osc, relay)", n.id)
            };
            c.source = src;
        }
        c.click = n.bool("click", c.click);
        c.midi_in = n.str("midi_in").unwrap_or_default().into();
        c.midi_out = n.str("midi_out").map(String::from);
        c.midi_out_lead = n.f32("midi_out_lead", 0.0) as f64 / 1000.0;
        if let Some(o) = n.params.get("osc_port").and_then(|v| v.as_integer()) {
            c.osc = format!("0.0.0.0:{o}");
        }
        Ok(c)
    }

    /// the map the audio starts with (before the first frame publishes)
    pub fn map(&self, rate: f64) -> TempoMap {
        TempoMap::new(self.bpm, self.per_bar, rate)
    }
}

/// The clock at this frame's heard moment (deck-space's: the space reads it too).
pub use deck_space::tempo::Now;

/// The sources' I/O, opened when a source is first chosen (MIDI out: at the start, it doesn't depend on the master).
#[derive(Default)]
struct Io {
    #[cfg(feature = "midi")]
    midi_in: Option<(midir::MidiInputConnection<()>, String)>,
    #[cfg(feature = "midi")]
    midi_out: Option<midi::ClockOutThread>,
    #[cfg(feature = "osc")]
    osc: Option<osc::Listener>,
    #[cfg(feature = "link")]
    link: Option<link::Link>,
    /// why a source isn't there (shown in the HUD)
    error: Option<String>,
}

/// The frame thread's tempo clock.
pub struct Tempo {
    cfg: Config,
    pub source: Source,
    map: TempoMap,
    pub click: bool,
    shared: Arc<Shared>,
    tap: map::Tap,
    midi: midi::ClockIn,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    io: Io,
    now: Now,
    /// open the I/O of the chosen source (off in tests)
    live: bool,
}

impl Tempo {
    /// `shared` = the engine's (audio::Handles::tempo) or a fresh one without audio
    pub fn new(cfg: Config, shared: Arc<Shared>, rate: f64) -> Tempo {
        let (tx, rx) = channel();
        let map = cfg.map(rate);
        shared.publish(&map, cfg.click);
        Tempo {
            source: Source::Internal,
            click: cfg.click,
            map,
            shared,
            tap: map::Tap::default(),
            midi: midi::ClockIn::default(),
            tx,
            rx,
            io: Io::default(),
            now: Now { bpm: map.bpm, pos: map.position(0.0), per_bar: map.per_bar, running: true },
            live: false,
            cfg,
        }
    }

    /// open the sources' I/O: the MIDI clock out if configured, the scene's source
    pub fn start(&mut self) {
        self.live = true;
        #[cfg(feature = "midi")]
        if let Some(want) = self.cfg.midi_out.clone() {
            match midi::spawn_out(&want, self.shared.clone(), self.cfg.midi_out_lead) {
                Ok(t) => {
                    tracing::info!("tempo: MIDI clock out → {}", t.port);
                    self.io.midi_out = Some(t);
                }
                Err(e) => tracing::warn!("tempo: MIDI clock out: {e:#}"),
            }
        }
        #[cfg(not(feature = "midi"))]
        if self.cfg.midi_out.is_some() {
            tracing::warn!("tempo: MIDI clock out: {}", not_built("midi"));
        }
        self.set_source(self.cfg.source);
    }

    /// where a source's events go (tests and other inputs feed the clock through it)
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    pub fn key(&self, k: Key) {
        let _ = self.tx.send(Event { at_ns: now_ns(), what: What::Key(k) });
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

    pub fn set_source(&mut self, s: Source) {
        self.source = s;
        self.io.error = None;
        if self.live {
            self.open(s);
        }
        tracing::info!("tempo: source {}", s.name());
    }

    fn open(&mut self, s: Source) {
        let r: Result<()> = match s {
            #[cfg(feature = "midi")]
            Source::Midi if self.io.midi_in.is_none() => midi::connect_in(&self.cfg.midi_in, self.tx.clone()).map(|c| {
                tracing::info!("tempo: MIDI clock in ← {}", c.1);
                self.io.midi_in = Some(c);
            }),
            #[cfg(feature = "osc")]
            Source::Osc if self.io.osc.is_none() => osc::listen(&self.cfg.osc, self.tx.clone()).map(|l| {
                tracing::info!("tempo: OSC on {}", l.addr);
                self.io.osc = Some(l);
            }),
            #[cfg(feature = "link")]
            Source::Link if self.io.link.is_none() => {
                self.io.link = Some(link::Link::new(self.map.bpm));
                Ok(())
            }
            // F3 reaches every source; one whose module isn't built says so (the HUD shows it)
            _ if crate::modules::built(s.name()) == Some(false) => Err(anyhow::anyhow!(not_built(s.name()))),
            _ => Ok(()),
        };
        if let Err(e) = r {
            tracing::warn!("tempo: {}: {e:#}", s.name());
            self.io.error = Some(format!("{e:#}"));
        }
    }

    /// One frame: the events since the last, the master followed, the map re-anchored at the audio thread's frontier
    /// and published; the clock at the heard moment.
    pub fn frame(&mut self, m: &Moment) -> Now {
        self.shared.publish_line(&m.line);
        while let Ok(e) = self.rx.try_recv() {
            self.event(e, m);
        }
        let per_bar = self.map.per_bar;
        match self.source {
            Source::Midi if !self.midi.lost(m.now_ns as f64 * 1e-9) => {
                if let Some(l) = self.midi.line() {
                    self.map = l.to_map(&m.line, m.rate, per_bar);
                }
            }
            // Link gives tempo and phase; start / stop stay the deck's own (stopped, the map stands where it stopped)
            #[cfg(feature = "link")]
            Source::Link => {
                if let (Some(l), true) = (self.io.link.as_mut(), self.map.running) {
                    self.map = l.line(m.now_ns, per_bar).to_map(&m.line, m.rate, per_bar);
                }
            }
            // internal, OSC (a remote for the internal clock), relay (no show clock yet), a lost MIDI clock: the map runs on
            _ => {}
        }
        self.map = self.map.at(m.rendered.max(m.heard));
        self.shared.publish(&self.map, self.click);
        self.now = Now { bpm: self.map.bpm, pos: self.map.position(m.heard), per_bar, running: self.map.running };
        self.now
    }

    fn event(&mut self, e: Event, m: &Moment) {
        let f = m.frames_at(e.at_ns);
        match e.what {
            What::Midi(b) => self.midi.message(e.at_ns as f64 * 1e-9, &b),
            What::Osc(c) if self.source == Source::Osc => self.command(c, f, m, e.at_ns),
            What::Osc(_) => {}
            What::Key(Key::NextSource) => self.set_source(self.source.next()),
            What::Key(Key::Click) => self.click = !self.click,
            What::Key(k) => {
                // a hand on the clock takes over from a follower that can't be steered (MIDI in, the relay)
                if matches!(self.source, Source::Midi | Source::Relay) {
                    self.set_source(Source::Internal);
                }
                let c = match k {
                    Key::Tap => osc::Cmd::Tap,
                    Key::Faster => osc::Cmd::Bpm(self.map.bpm.round() + 1.0),
                    Key::Slower => osc::Cmd::Bpm(self.map.bpm.round() - 1.0),
                    _ if self.map.running => osc::Cmd::Stop,
                    _ => osc::Cmd::Start,
                };
                self.command(c, f, m, e.at_ns);
            }
        }
    }

    /// a command on the steerable clock that came at frame `f`: a tempo, start / stop from the audio thread's frontier
    /// on (what is rendered stays – the audio never hears a jump), a beat or a tap's phase exactly where it happened
    fn command(&mut self, c: osc::Cmd, f: f64, m: &Moment, at_ns: u64) {
        let frontier = f.max(m.rendered);
        match c {
            osc::Cmd::Bpm(b) => self.set_bpm(frontier, b),
            osc::Cmd::Beat(b) => self.map = self.map.with_beat(f, b),
            osc::Cmd::Start => self.map = self.map.with_running(frontier, true),
            osc::Cmd::Stop => self.map = self.map.with_running(frontier, false),
            osc::Cmd::Tap => {
                if let Some(bpm) = self.tap.tap(at_ns) {
                    self.set_bpm(frontier, bpm);
                    // the tap is a beat
                    let b = self.map.beat_at(f).round();
                    self.map = self.map.with_beat(f, b);
                }
            }
        }
    }

    fn set_bpm(&mut self, at: f64, bpm: f64) {
        #[cfg(feature = "link")]
        if self.source == Source::Link {
            if let Some(l) = self.io.link.as_mut() {
                l.set_tempo(map::clamp_bpm(bpm));
                return;
            }
        }
        self.map = self.map.with_bpm(at, bpm);
    }

    /// the source's state for the HUD ("" = fine)
    pub fn status(&self) -> String {
        if let Some(e) = &self.io.error {
            return e.chars().take(24).collect::<String>().to_uppercase();
        }
        let now = now_ns() as f64 * 1e-9;
        match self.source {
            Source::Midi if self.midi.lost(now) => "NO CLOCK".into(),
            Source::Midi => String::new(),
            #[cfg(feature = "link")]
            Source::Link => self.io.link.as_ref().map(|l| format!("{} PEERS", l.peers())).unwrap_or_default(),
            #[cfg(feature = "osc")]
            Source::Osc => self.io.osc.as_ref().map(|l| format!("PORT {}", l.addr.port())).unwrap_or_default(),
            // a source whose module isn't built (its error above says so)
            #[allow(unreachable_patterns)]
            Source::Link | Source::Osc => String::new(),
            Source::Relay => RelayClock.status().into(),
            Source::Internal => String::new(),
        }
    }

    /// the MIDI clock out's port, if it runs
    #[cfg(feature = "midi")]
    pub fn clock_out(&self) -> Option<&str> {
        self.io.midi_out.as_ref().map(|t| t.port.as_str())
    }

    #[cfg(not(feature = "midi"))]
    pub fn clock_out(&self) -> Option<&str> {
        None
    }
}

/// why a source's module isn't there (the HUD upper-cases and cuts it)
fn not_built(feature: &str) -> String {
    format!("not in this build (--features {feature})")
}

/// The relay's show clock (LIVE.md B.6) – a stub: the relay has only the ping / pong offset so far (#172). When it
/// gets a show clock (BPM + the beat at a relay `at`), this turns it into a WallLine: relay ms → local ns by the
/// bridge's offset (the median of 5 pings) → the time base's frames, like MIDI clock in.
pub struct RelayClock;

impl RelayClock {
    pub fn line(&self) -> Option<midi::WallLine> {
        None
    }

    pub fn status(&self) -> &'static str {
        "NO SHOW CLOCK YET"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Line;
    use crate::scene::Scene;

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

    fn tempo(scene: &str) -> Tempo {
        let g = Graph::build(&Scene::parse(scene).unwrap()).unwrap();
        Tempo::new(Config::from_graph(&g).unwrap(), Arc::new(Shared::default()), RATE)
    }

    #[cfg(feature = "osc")]
    #[test]
    fn the_scene_configures_the_clock() {
        let g = Graph::build(
            &Scene::parse("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=60\nbeats=3\nsource=\"osc\"\nclick=true\nosc_port=9100\n").unwrap(),
        )
        .unwrap();
        let c = Config::from_graph(&g).unwrap();
        assert_eq!((c.bpm, c.per_bar, c.source, c.click, c.osc.as_str()), (60.0, 3, Source::Osc, true, "0.0.0.0:9100"));
        // an unknown source: the scene is refused before its graph is built (the module check, #185)
        let bad = crate::modules::check(&Scene::parse("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"vinyl\"\n").unwrap());
        assert!(bad.unwrap_err().to_string().contains("unknown source 'vinyl'"));
        assert_eq!(Config::from_graph(&Graph::build(&Scene::parse("").unwrap()).unwrap()).unwrap(), Config::default());
    }

    #[test]
    fn an_internal_tempo_change_keeps_the_beat_and_reaches_the_audio() {
        let mut t = tempo("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=60\n");
        let n = t.frame(&moment(1.5, 1024.0));
        assert!((n.pos.beat - 1.5).abs() < 1e-9 && n.pos.beat_in_bar == 1);
        // + at 2.0 s: from the audio frontier on at 61 BPM
        t.sender().send(at(2.0, What::Key(Key::Faster))).unwrap();
        let m = moment(2.0, 1024.0);
        let n = t.frame(&m);
        assert_eq!(n.bpm, 61.0);
        assert!((n.pos.beat - 2.0).abs() < 1e-3, "no jump: {}", n.pos.beat);
        let (shared, _) = t.shared.map().unwrap();
        assert_eq!(shared.frame, m.rendered, "anchored at the frontier");
        // what the audio thread rendered at 60 BPM stays: the new tempo starts where it stands
        assert!((shared.beat_at(m.rendered) - m.rendered / RATE).abs() < 1e-9);
        let later = t.frame(&moment(3.0, 1024.0)).pos.beat;
        assert!((later - (2.0 + 61.0 / 60.0)).abs() < 1e-3, "{later}");
    }

    #[test]
    fn taps_set_tempo_and_phase() {
        let mut t = tempo("");
        let tx = t.sender();
        for i in 0..4 {
            tx.send(at(10.0 + i as f64 * 0.6 + 0.013, What::Key(Key::Tap))).unwrap(); // 100 BPM, 13 ms after the grid
        }
        let n = t.frame(&moment(11.9, 512.0));
        assert!((n.bpm - 100.0).abs() < 1e-6, "{}", n.bpm);
        // the last tap (11.813 s) is a whole beat
        let b = t.map().beat_at(11.813 * RATE);
        assert!((b - b.round()).abs() < 1e-6, "{b}");
    }

    #[test]
    fn midi_clock_in_leads_and_a_key_takes_over() {
        // the follower is built without the `midi` module too (only its ports need it)
        let mut t = tempo("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=90\n");
        t.set_source(Source::Midi);
        let tx = t.sender();
        tx.send(at(4.99, What::Midi([0xFA, 0, 0]))).unwrap();
        for k in 0..96 {
            tx.send(at(5.0 + k as f64 / 56.0, What::Midi([0xF8, 0, 0]))).unwrap(); // 140 BPM
        }
        let n = t.frame(&moment(5.0 + 95.0 / 56.0, 256.0));
        assert!((n.bpm - 140.0).abs() < 1e-6, "{}", n.bpm);
        assert!((n.pos.beat - 95.0 / 24.0).abs() < 1e-6, "{}", n.pos.beat);
        // the clock goes away: the stage runs on at its tempo
        let n = t.frame(&moment(9.0, 256.0));
        assert!((n.bpm - 140.0).abs() < 1e-6);
        // a key takes over as the internal clock, from where it is
        tx.send(at(9.0, What::Key(Key::Faster))).unwrap();
        let n = t.frame(&moment(9.0, 256.0));
        assert_eq!((t.source, n.bpm), (Source::Internal, 141.0));
    }

    #[test]
    fn osc_steers_only_as_the_master() {
        let mut t = tempo("");
        let tx = t.sender();
        tx.send(at(1.0, What::Osc(osc::Cmd::Bpm(80.0)))).unwrap();
        assert_eq!(t.frame(&moment(1.0, 0.0)).bpm, 120.0, "not the master: ignored");
        t.set_source(Source::Osc);
        tx.send(at(2.0, What::Osc(osc::Cmd::Bpm(80.0)))).unwrap();
        tx.send(at(2.0, What::Osc(osc::Cmd::Beat(32.0)))).unwrap();
        t.frame(&moment(2.0, 0.0));
        tx.send(at(2.5, What::Osc(osc::Cmd::Stop))).unwrap();
        t.frame(&moment(2.5, 0.0));
        let n = t.frame(&moment(3.0, 0.0));
        assert_eq!(n.bpm, 80.0);
        assert!(!n.running);
        assert!((n.pos.beat - (32.0 + 0.5 * 80.0 / 60.0)).abs() < 1e-6, "stopped at 2.5 s: {}", n.pos.beat);
        assert_eq!((n.pos.bar, n.pos.beat_in_bar), (8, 0));
        t.set_source(Source::Relay);
        assert_eq!(t.status(), "NO SHOW CLOCK YET");
        assert_eq!(t.frame(&moment(4.0, 0.0)).bpm, 80.0, "the relay stub: the clock stays");
    }

    #[test]
    fn keys_cycle_sources_and_toggle_the_click() {
        let mut t = tempo("");
        let tx = t.sender();
        tx.send(at(1.0, What::Key(Key::NextSource))).unwrap();
        tx.send(at(1.0, What::Key(Key::Click))).unwrap();
        t.frame(&moment(1.0, 0.0));
        assert_eq!(t.source, Source::Midi);
        assert!(t.shared.map().unwrap().1, "the click is on");
        tx.send(at(1.0, What::Key(Key::StartStop))).unwrap();
        assert!(!t.frame(&moment(1.1, 0.0)).running);
    }
}
