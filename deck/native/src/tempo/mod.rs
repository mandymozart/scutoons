//! The tempo clock (CYBERDECKS.md §8 "Two clocks", #181) on top of the time base: BPM, beat, bar and phase as a tempo
//! map over the audio frame position (map.rs – beat = f(audio position), never a second clock). One source is the
//! master at a time: internal (set, + / −, tap), MIDI clock in, Ableton Link (feature `link`), OSC, the relay's show
//! clock (a stub until the relay has one). The frame thread owns the map: each frame it takes the sources' events
//! (stamped on arrival, placed on the frame axis by the time base's line), follows the master, re-anchors the map at
//! the audio thread's frontier and publishes it – the click and the synced sequencer read it per block, MIDI clock out
//! per tick, the control domain per frame (the `tempo` node's outputs, quantised cuts).
//!
//! The controller itself (sources, keys, tap, the followers, the map, the seqlock `Shared`) is deck-core's `tempo`
//! module (#185, #308) and platform-free; here: the native adapter – channels the sources' threads send into, the
//! sources' I/O (midir, UDP, rusty_link) opened through core's `Host`, and the scene's `tempo` node.
pub mod map;
pub mod midi;
pub mod osc;
#[cfg(feature = "link")]
pub mod link;

pub use deck_core::tempo::{Anchor, Cmd, Config, Controller, Event, Host, Key, Now, Source, What};
pub use map::{position, BeatReader, Position, Quantize, Shared, TempoMap};

use crate::clock::{now_ns, Moment};
use crate::graph::{Graph, Kind};
use anyhow::{bail, Result};
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// The scene's `tempo` node (none = the defaults: 120 BPM in 4, internal, the click off).
pub fn config_from_graph(graph: &Graph) -> Result<Config> {
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

/// The frame thread's tempo clock: deck-core's `Controller` + the native sources around it.
pub struct Tempo {
    cfg: Config,
    ctl: Controller,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    io: Io,
    /// open the I/O of the chosen source (off in tests)
    live: bool,
}

/// `.source`, `.click`, `.now()`, `.map()`, `.shared()` are the controller's
impl Deref for Tempo {
    type Target = Controller;
    fn deref(&self) -> &Controller {
        &self.ctl
    }
}

impl DerefMut for Tempo {
    fn deref_mut(&mut self) -> &mut Controller {
        &mut self.ctl
    }
}

/// The platform side of the controller: opens a source's port when it becomes the master, asks Link.
struct Native<'a> {
    cfg: &'a Config,
    io: &'a mut Io,
    tx: &'a Sender<Event>,
    live: bool,
    /// the map's tempo, for a Link session opened now
    #[cfg_attr(not(feature = "link"), allow(dead_code))]
    bpm: f64,
}

impl Host for Native<'_> {
    fn source_chosen(&mut self, s: Source) {
        self.io.error = None;
        if self.live {
            self.open(s);
        }
        tracing::info!("tempo: source {}", s.name());
    }

    #[cfg(feature = "link")]
    fn link_line(&mut self, now_ns: u64, per_bar: u32) -> Option<midi::WallLine> {
        self.io.link.as_mut().map(|l| l.line(now_ns, per_bar))
    }

    #[cfg(feature = "link")]
    fn link_set_tempo(&mut self, bpm: f64) -> bool {
        match self.io.link.as_mut() {
            Some(l) => {
                l.set_tempo(bpm);
                true
            }
            None => false,
        }
    }
}

impl Native<'_> {
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
                self.io.link = Some(link::Link::new(self.bpm));
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
}

impl Tempo {
    /// `shared` = the engine's (audio::Handles::tempo) or a fresh one without audio
    pub fn new(cfg: Config, shared: Arc<Shared>, rate: f64) -> Tempo {
        let (tx, rx) = channel();
        Tempo { ctl: Controller::new(&cfg, shared, rate), tx, rx, io: Io::default(), live: false, cfg }
    }

    fn host(&mut self) -> (Native<'_>, &mut Controller) {
        let bpm = self.ctl.map().bpm;
        (Native { cfg: &self.cfg, io: &mut self.io, tx: &self.tx, live: self.live, bpm }, &mut self.ctl)
    }

    /// open the sources' I/O: the MIDI clock out if configured, the scene's source
    pub fn start(&mut self) {
        self.live = true;
        #[cfg(feature = "midi")]
        if let Some(want) = self.cfg.midi_out.clone() {
            match midi::spawn_out(&want, self.ctl.shared(), self.cfg.midi_out_lead) {
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

    pub fn set_source(&mut self, s: Source) {
        let (mut host, ctl) = self.host();
        ctl.set_source(s, &mut host);
    }

    /// One frame: the events since the last (from the sources' channel into the controller), then the controller's.
    pub fn frame(&mut self, m: &Moment) -> Now {
        while let Ok(e) = self.rx.try_recv() {
            self.ctl.push(e);
        }
        let (mut host, ctl) = self.host();
        ctl.frame(m, &mut host)
    }

    /// the source's state for the HUD ("" = fine)
    pub fn status(&self) -> String {
        if let Some(e) = &self.io.error {
            return e.chars().take(24).collect::<String>().to_uppercase();
        }
        match self.ctl.source {
            Source::Midi if self.ctl.midi_lost(now_ns()) => "NO CLOCK".into(),
            Source::Midi => String::new(),
            #[cfg(feature = "link")]
            Source::Link => self.io.link.as_ref().map(|l| format!("{} PEERS", l.peers())).unwrap_or_default(),
            #[cfg(feature = "osc")]
            Source::Osc => self.io.osc.as_ref().map(|l| format!("PORT {}", l.addr.port())).unwrap_or_default(),
            // a source whose module isn't built (its error above says so)
            #[allow(unreachable_patterns)]
            Source::Link | Source::Osc => String::new(),
            Source::Relay if !self.ctl.has_anchor() => "NO SHOW CLOCK YET".into(),
            Source::Relay => String::new(),
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
        Tempo::new(config_from_graph(&g).unwrap(), Arc::new(Shared::default()), RATE)
    }

    #[cfg(feature = "osc")]
    #[test]
    fn the_scene_configures_the_clock() {
        let g = Graph::build(
            &Scene::parse("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=60\nbeats=3\nsource=\"osc\"\nclick=true\nosc_port=9100\n").unwrap(),
        )
        .unwrap();
        let c = config_from_graph(&g).unwrap();
        assert_eq!((c.bpm, c.per_bar, c.source, c.click, c.osc.as_str()), (60.0, 3, Source::Osc, true, "0.0.0.0:9100"));
        // an unknown source: the scene is refused before its graph is built (the module check, #185)
        let bad = crate::modules::check(&Scene::parse("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"vinyl\"\n").unwrap());
        assert!(bad.unwrap_err().to_string().contains("unknown source 'vinyl'"));
        assert_eq!(config_from_graph(&Graph::build(&Scene::parse("").unwrap()).unwrap()).unwrap(), Config::default());
    }

    // the controller's own tests (tempo change, tap, MIDI clock in, OSC, keys, Link) are deck-core's; these go through
    // the adapter: the sources' channel, the scene's config, the HUD status
    #[test]
    fn events_from_the_sources_channel_reach_the_clock() {
        let mut t = tempo("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=60\n");
        let n = t.frame(&moment(1.5, 1024.0));
        assert!((n.pos.beat - 1.5).abs() < 1e-9 && n.pos.beat_in_bar == 1);
        t.sender().send(at(2.0, What::Key(Key::Faster))).unwrap();
        assert_eq!(t.frame(&moment(2.0, 1024.0)).bpm, 61.0);
        t.set_source(Source::Midi);
        let tx = t.sender();
        tx.send(at(4.99, What::Midi([0xFA, 0, 0]))).unwrap();
        for k in 0..96 {
            tx.send(at(5.0 + k as f64 / 56.0, What::Midi([0xF8, 0, 0]))).unwrap(); // 140 BPM
        }
        let n = t.frame(&moment(5.0 + 95.0 / 56.0, 256.0));
        assert!((n.bpm - 140.0).abs() < 1e-6, "{}", n.bpm);
        tx.send(at(9.0, What::Key(Key::Faster))).unwrap();
        let n = t.frame(&moment(9.0, 256.0));
        assert_eq!((t.source, n.bpm), (Source::Internal, 141.0), "a key takes the clock over");
    }

    #[test]
    fn the_hud_status_and_the_relay_stub() {
        let mut t = tempo("");
        assert_eq!(t.status(), "");
        t.set_source(Source::Midi);
        assert_eq!(t.status(), "NO CLOCK");
        t.set_source(Source::Relay);
        assert_eq!(t.status(), "NO SHOW CLOCK YET");
        assert_eq!(t.frame(&moment(4.0, 0.0)).bpm, 120.0, "the relay stub: the clock stays");
        // F3 cycles the sources through the adapter and the click toggles
        t.set_source(Source::Internal);
        let tx = t.sender();
        tx.send(at(1.0, What::Key(Key::NextSource))).unwrap();
        tx.send(at(1.0, What::Key(Key::Click))).unwrap();
        t.frame(&moment(1.0, 0.0));
        assert_eq!(t.source, Source::Midi);
        assert!(t.shared().map().unwrap().1, "the click is on");
    }
}
