//! The saved PipeWire sink for the device watch: the registry's sinks as PipeWire's loop thread hears them, the node
//! events that wake the watch (the sink appeared) or lose its stream (the sink it plays on went), and the `Port` that finds
//! the sink there and opens a stream on it. The PipeWire side (Linux, feature `pipewire`) is backend.rs's; this part runs
//! without PipeWire, so the tests drive it with node events of their own.
#![cfg_attr(not(all(target_os = "linux", feature = "pipewire")), allow(dead_code))]
use super::backend::{SinkProps, PIPEWIRE_RATE};
use super::watch::{Found, Held, Link, Lost, Opened, Port, Waker};
use anyhow::{bail, Result};
use std::any::Any;
use std::sync::{Arc, Mutex};

/// A sink to open: its global id, its node name (`target.object`) and its channels' position names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sink {
    pub id: u32,
    pub name: String,
    pub positions: Vec<String>,
}

/// What opens a stream on a sink (PipeWire's loop thread in a deck, a fake in the tests); the stream holds `held` and
/// says `lost` when its sink goes. Dropping the box closes it.
pub trait Streams: Send {
    fn open(&mut self, sink: &Sink, held: Held, lost: Lost) -> Result<Box<dyn Any>>;
}

struct Node {
    id: u32,
    props: SinkProps,
    /// its info arrived: the channels are known
    told: bool,
}

#[derive(Default)]
struct Shared {
    nodes: Vec<Node>,
    waker: Option<Waker>,
    /// the sink a stream plays on, and how it says it is gone
    playing: Option<(u32, Lost)>,
}

/// The registry's `Audio/Sink` nodes and the saved name; the loop thread reports to it, the watch reads it.
#[derive(Clone)]
pub struct Events {
    want: String,
    shared: Arc<Mutex<Shared>>,
}

impl Events {
    pub fn new(want: &str) -> Events {
        Events { want: want.to_string(), shared: Arc::default() }
    }

    fn with<T>(&self, f: impl FnOnce(&mut Shared) -> T) -> T {
        f(&mut self.shared.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// a sink node appeared (its info follows)
    pub fn added(&self, id: u32, name: &str, description: &str) {
        self.with(|s| {
            s.nodes.retain(|n| n.id != id);
            s.nodes.push(Node { id, props: SinkProps { name: name.into(), description: description.into(), ..SinkProps::default() }, told: false });
        });
    }

    /// a sink's info: its channels; the saved sink complete now wakes the watch
    pub fn info(&self, id: u32, position: Option<&str>, channels: Option<usize>) {
        let waker = self.with(|s| {
            let n = s.nodes.iter_mut().find(|n| n.id == id)?;
            if let Some(p) = position {
                n.props.position = Some(p.to_string());
            }
            if channels.is_some() {
                n.props.channels = channels;
            }
            n.told = true;
            n.props.matches(&self.want).then(|| s.waker.clone()).flatten()
        });
        if let Some(w) = waker {
            w.look();
        }
    }

    /// a global went: the sink a stream plays on loses it
    pub fn removed(&self, id: u32) {
        let lost = self.with(|s| {
            s.nodes.retain(|n| n.id != id);
            match &s.playing {
                Some((at, _)) if *at == id => s.playing.take().map(|(_, l)| l),
                _ => None,
            }
        });
        if let Some(l) = lost {
            l.now();
        }
    }

    /// the saved sink, once its info is here
    pub fn find(&self) -> Option<(Sink, String)> {
        self.with(|s| s.nodes.iter().find(|n| n.told && n.props.matches(&self.want)).map(|n| (sink(n), n.props.description.clone())))
    }

    fn watch(&self, waker: Waker) {
        self.with(|s| s.waker = Some(waker));
    }

    /// the saved sink, marked as the one a stream plays on (one look: a sink that went in between is not opened)
    fn play(&self, lost: Lost) -> Option<Sink> {
        self.with(|s| {
            let n = s.nodes.iter().find(|n| n.told && n.props.matches(&self.want))?;
            let at = sink(n);
            s.playing = Some((at.id, lost));
            Some(at)
        })
    }
}

fn sink(n: &Node) -> Sink {
    let mut positions: Vec<String> = n.props.channel_names().into_iter().take(deck_engine::config::MAX_DEVICE_CHANNELS).collect();
    if positions.is_empty() {
        positions = vec!["FL".into(), "FR".into()];
    }
    Sink { id: n.id, name: n.props.name.clone(), positions }
}

/// The saved sink (deck.toml's `audio_device`, a part of its node name or description) for the watch.
pub struct PwPort {
    events: Events,
    streams: Box<dyn Streams>,
}

impl PwPort {
    pub fn new(events: Events, streams: Box<dyn Streams>) -> PwPort {
        PwPort { events, streams }
    }
}

impl Port for PwPort {
    fn name(&self) -> String {
        self.events.want.clone()
    }

    fn find(&mut self) -> Option<Found> {
        let (sink, description) = self.events.find()?;
        let name = if description.is_empty() { sink.name } else { description };
        Some(Found { name, rate: PIPEWIRE_RATE, channels: sink.positions.len() })
    }

    fn open(&mut self, rate: u32, link: &mut Link) -> Result<Opened> {
        if rate != PIPEWIRE_RATE {
            bail!("the stream plays {PIPEWIRE_RATE} Hz, the show runs at {rate} Hz");
        }
        let lost = link.lost();
        let Some(sink) = self.events.play(lost.clone()) else { bail!("not in PipeWire's registry") };
        let stream = self.streams.open(&sink, Held::handed(link.hand()), lost)?;
        Ok(Opened { stream, wasapi: false })
    }

    fn watch(&mut self, waker: Waker) {
        self.events.watch(waker);
    }
}

#[cfg(test)]
mod tests {
    use super::super::backend::Feed;
    use super::super::watch::tests::{away, engine, plays_the_map, wait, Fake, RATE};
    use super::super::watch::{Notes, Watch};
    use super::*;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    const RME: &str = "alsa_output.usb-RME_Fireface_UCX_II-00.pro-output-0";

    /// PipeWire's loop thread as a fake: a stream is the fake device's, its sinks what the test says
    struct FakeStreams {
        fake: Fake,
        opened: Arc<Mutex<Vec<Sink>>>,
    }

    impl Streams for FakeStreams {
        fn open(&mut self, sink: &Sink, held: Held, _lost: Lost) -> Result<Box<dyn Any>> {
            self.opened.lock().unwrap().push(sink.clone());
            self.fake.quiet.store(false, Ordering::Relaxed);
            Ok(Box::new(self.fake.stream(held)))
        }
    }

    fn aux(n: usize) -> String {
        (0..n).map(|i| format!("AUX{i}")).collect::<Vec<_>>().join(",")
    }

    fn back(notes: &Notes) -> bool {
        notes.get().is_some_and(|n| !n.away && n.text == "audio back on 'Fireface UCX II Pro'")
    }

    #[test]
    fn the_saved_sink_is_found_once_the_registry_has_its_info() {
        let ev = Events::new("fireface");
        ev.added(40, "alsa_output.pci-0000_00_1f.3.analog-stereo", "Built-in Audio Analog Stereo");
        ev.info(40, Some("[ FL FR ]"), Some(2));
        assert_eq!(ev.find(), None, "another sink");
        ev.added(57, RME, "Fireface UCX II Pro");
        assert_eq!(ev.find(), None, "its channels are not known yet");
        ev.info(57, Some(&aux(20)), Some(20));
        let (sink, label) = ev.find().unwrap();
        assert_eq!((sink.id, sink.name.as_str(), sink.positions.len(), label.as_str()), (57, RME, 20, "Fireface UCX II Pro"));
        assert_eq!(sink.positions[19], "AUX19");
        ev.removed(57);
        assert_eq!(ev.find(), None, "gone");

        let quiet = Events::new("quiet");
        quiet.added(3, "quiet", "Says nothing");
        quiet.info(3, None, None);
        assert_eq!(quiet.find().unwrap().0.positions, ["FL", "FR"], "a sink that says nothing is stereo");
    }

    /// missing at the start, then node events alone (the tick a minute away) bring it in, take it out and bring it back
    #[test]
    fn node_events_bring_the_sink_back_and_take_it_out() {
        let fake = Fake::new(true, 20);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let events = Events::new("Fireface");
        let mut port = PwPort::new(events.clone(), Box::new(FakeStreams { fake: fake.clone(), opened: opened.clone() }));
        assert_eq!(port.find(), None);
        let (engine, handles) = engine(4);
        let feed = Feed::new(engine, true, Some(128)).unwrap();
        let watch = Watch::start(fake.time(), Box::new(port), feed, RATE, 4, None, Duration::from_secs(60)).unwrap();
        let notes = watch.notes();
        assert_eq!(notes.get().map(|n| (n.text, n.away)), Some(("'Fireface' not found – running silent, waiting for it".into(), true)));
        let rendered = handles.clock.rendered();
        wait(&fake.clock, "the timer to play", 5, || handles.clock.rendered() > rendered + 4096);

        // another sink comes and goes: nothing opens
        events.added(40, "alsa_output.pci-0000_00_1f.3.analog-stereo", "Built-in Audio Analog Stereo");
        events.info(40, Some("FL,FR"), Some(2));
        events.removed(40);
        events.added(57, RME, "Fireface UCX II Pro");
        fake.clock.advance(Duration::from_millis(100));
        assert_eq!(fake.opens.load(Ordering::Relaxed), 0, "no info yet: not opened");
        events.info(57, Some(&aux(20)), Some(20));
        wait(&fake.clock, "the sink back", 5, || back(&notes));
        assert_eq!(opened.lock().unwrap().last().map(|s| (s.name.clone(), s.positions.len())), Some((RME.to_string(), 20)));
        plays_the_map(&fake);

        // unplugged: the node goes, the stream is lost, the timer plays
        fake.quiet.store(true, Ordering::Relaxed);
        events.removed(57);
        wait(&fake.clock, "the loss said", 5, || away(&notes));
        assert_eq!(notes.get().unwrap().text, "'Fireface UCX II Pro' lost – running silent, waiting for it");
        let rendered = handles.clock.rendered();
        wait(&fake.clock, "the timer to play again", 5, || handles.clock.rendered() > rendered + 4096);

        // replugged: a new global id
        events.added(61, RME, "Fireface UCX II Pro");
        events.info(61, Some(&aux(20)), Some(20));
        wait(&fake.clock, "the sink back again", 5, || back(&notes));
        plays_the_map(&fake);
        assert_eq!(fake.opens.load(Ordering::Relaxed), 2);
        assert_eq!(opened.lock().unwrap().iter().map(|s| s.id).collect::<Vec<_>>(), [57, 61]);
        drop(watch);
    }
}
