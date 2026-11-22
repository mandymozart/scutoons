//! Live MIDI in: the deck's MIDI input devices into the dispatcher, for the `world` nodes that listen to `midi.note`.
//! The platform part only: list the devices, open them, hand their bytes over. Which device is which port, the notes and
//! their ends are deck-events' (adapters/midi_in.rs, adapters/midi.rs). One thread ("midi-in", never the audio thread)
//! owns the ports: midir's callbacks queue the bytes (bounded, never blocking), the thread feeds them through the adapter
//! into the dispatcher's `Inlet` and lists the devices again every second – one plugged in later opens, one gone ends its
//! held notes.
use super::dispatch::Inlet;
use deck_events::adapters::midi_in::{Change, Inputs};
use deck_events::{Event, Names, Time};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// messages waiting for the thread at most (more are dropped)
const QUEUE: usize = 1024;
/// how often the devices are listed again
const RESCAN: Duration = Duration::from_secs(1);

/// one message of an input: its port and up to three bytes
#[derive(Debug, Clone, Copy)]
pub struct Msg {
    pub port: u8,
    pub len: u8,
    pub bytes: [u8; 3],
}

impl Msg {
    /// a message as a device sends it; None when it is longer than three bytes (sysex – nothing to play)
    pub fn of(port: u8, bytes: &[u8]) -> Option<Msg> {
        let mut m = Msg { port, len: bytes.len() as u8, bytes: [0; 3] };
        m.bytes.get_mut(..bytes.len())?.copy_from_slice(bytes);
        Some(m)
    }
}

/// The system's MIDI inputs: their names now, and opening one whose bytes go to `tx` as `port`'s. The open handle closes
/// the device when dropped.
pub trait Devices: Send + 'static {
    fn names(&mut self) -> Vec<String>;
    fn open(&mut self, index: usize, name: &str, port: u8, tx: SyncSender<Msg>) -> anyhow::Result<Box<dyn Send>>;
}

/// the devices, their ports and the queue their bytes arrive on
pub struct Hub<D: Devices> {
    devices: D,
    inputs: Inputs,
    open: Vec<(u8, Box<dyn Send>)>,
    tx: SyncSender<Msg>,
    rx: Receiver<Msg>,
    out: Vec<Event>,
}

impl<D: Devices> Hub<D> {
    /// `names`: the engine's dictionary
    pub fn new(devices: D, names: Names) -> Hub<D> {
        let (tx, rx) = sync_channel(QUEUE);
        Hub { devices, inputs: Inputs::new(names), open: Vec::new(), tx, rx, out: Vec::with_capacity(64) }
    }

    /// list the devices: open the new ones, close the gone ones (their held notes end, into `send`)
    pub fn scan(&mut self, send: &mut impl FnMut(Event)) {
        let listing = self.devices.names();
        for change in self.inputs.sync(&listing, Time::Now, &mut self.out) {
            match change {
                Change::Opened { port, label, index } => match self.devices.open(index, &listing[index], port, self.tx.clone()) {
                    Ok(h) => {
                        tracing::info!("MIDI in {port}: {label}");
                        self.open.push((port, h));
                    }
                    Err(e) => tracing::warn!("MIDI in {port}: {label} – not opened ({e:#})"),
                },
                Change::Closed { port, label } => {
                    tracing::info!("MIDI in {port}: {label} gone – its held notes end");
                    self.open.retain(|(p, _)| *p != port);
                }
            }
        }
        self.out.drain(..).for_each(&mut *send);
    }

    /// wait up to `wait` for bytes, then feed everything queued; how many messages
    pub fn pump(&mut self, wait: Duration, send: &mut impl FnMut(Event)) -> usize {
        let first = match self.rx.recv_timeout(wait) {
            Ok(m) => m,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return 0,
        };
        let mut n = 0;
        for m in std::iter::once(first).chain(std::iter::from_fn(|| self.rx.try_recv().ok())) {
            self.inputs.feed(m.port, &m.bytes[..usize::from(m.len)], Time::Now, &mut self.out);
            n += 1;
        }
        self.out.drain(..).for_each(&mut *send);
        n
    }

    /// the devices present: (port, label)
    pub fn present(&self) -> Vec<(u8, String)> {
        self.inputs.present().map(|(p, l)| (p, l.to_string())).collect()
    }
}

/// the MIDI in thread: dropping it closes the devices and stops it
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

/// The hub on its own thread, its events into `inlet`.
pub fn spawn<D: Devices>(devices: D, names: Names, inlet: Inlet) -> std::io::Result<Thread> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let handle = std::thread::Builder::new().name("midi-in".into()).spawn(move || {
        let mut hub = Hub::new(devices, names);
        let mut send = |e: Event| {
            inlet.send(e);
        };
        hub.scan(&mut send);
        if hub.present().is_empty() {
            tracing::info!("MIDI in: no input device yet (one plugged in opens within a second)");
        }
        let mut listed = Instant::now();
        while !flag.load(Ordering::Relaxed) {
            hub.pump(Duration::from_millis(100), &mut send);
            if listed.elapsed() >= RESCAN {
                hub.scan(&mut send);
                listed = Instant::now();
            }
        }
    })?;
    Ok(Thread { stop, handle: Some(handle) })
}

/// The system's MIDI inputs through midir (ALSA, CoreMIDI, WinMM). On Windows a device is one program's at a time: one
/// the tempo clock holds does not open here (the log says so).
#[cfg(feature = "midi")]
pub struct System;

#[cfg(feature = "midi")]
impl Devices for System {
    fn names(&mut self) -> Vec<String> {
        match midir::MidiInput::new("deck-native midi in") {
            Ok(m) => m.ports().iter().map(|p| m.port_name(p).unwrap_or_default()).collect(),
            Err(e) => {
                tracing::warn!("MIDI in: {e}");
                Vec::new()
            }
        }
    }

    fn open(&mut self, index: usize, name: &str, port: u8, tx: SyncSender<Msg>) -> anyhow::Result<Box<dyn Send>> {
        let mut input = midir::MidiInput::new("deck-native midi in")?;
        input.ignore(midir::Ignore::All);
        let ports = input.ports();
        let named = |p: &&midir::MidiInputPort| input.port_name(p).is_ok_and(|n| n == name);
        let Some(p) = ports.get(index).filter(named).or_else(|| ports.iter().find(named)).cloned() else {
            anyhow::bail!("gone while opening");
        };
        let conn = input
            .connect(
                &p,
                "deck-native-midi-in",
                move |_, bytes, _| {
                    if let Some(m) = Msg::of(port, bytes) {
                        let _ = tx.try_send(m);
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Box::new(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::dispatch::Dispatcher;
    use deck_engine::dispatch::Routing;
    use deck_events::{Match, Route, Table};
    use std::sync::Mutex;

    /// devices a test plugs in and out, and plays through the senders they were opened with
    #[derive(Clone, Default)]
    struct Fake {
        names: Arc<Mutex<Vec<String>>>,
        taps: Arc<Mutex<Vec<(String, u8, SyncSender<Msg>)>>>,
    }

    impl Fake {
        fn plug(&self, name: &str) {
            self.names.lock().unwrap().push(name.into());
        }
        fn unplug(&self, name: &str) {
            self.names.lock().unwrap().retain(|n| n != name);
            self.taps.lock().unwrap().retain(|(n, ..)| n != name);
        }
        /// bytes as the device's callback sends them: stamped with the port it was opened as
        fn play(&self, name: &str, bytes: &[u8]) {
            let taps = self.taps.lock().unwrap();
            let (_, port, tx) = taps.iter().find(|(n, ..)| n == name).expect("opened");
            tx.try_send(Msg::of(*port, bytes).unwrap()).unwrap();
        }
    }

    impl Devices for Fake {
        fn names(&mut self) -> Vec<String> {
            self.names.lock().unwrap().clone()
        }
        fn open(&mut self, _: usize, name: &str, port: u8, tx: SyncSender<Msg>) -> anyhow::Result<Box<dyn Send>> {
            self.taps.lock().unwrap().push((name.into(), port, tx));
            Ok(Box::new(()))
        }
    }

    #[test]
    fn a_fake_keyboard_reaches_the_world_node_on_its_port_and_channel_and_a_gone_one_ends_its_notes() {
        let mut names = Names::new();
        let (note, off) = (names.event("midi.note").unwrap(), names.event(deck_events::adapters::midi::NOTE_OFF).unwrap());
        let (port, ch, key) = (names.key("port").unwrap(), names.key("ch").unwrap(), names.key("note").unwrap());
        let (tx, mut ring) = rtrb::RingBuffer::new(64);
        let routes = [Route { on: Match::Name(note), to: 0 }, Route { on: Match::Name(off), to: 0 }];
        let routing = Routing { table: Arc::new(Table::new(&names, &routes, 1)), outlets: vec![tx] };
        let (mut d, inlet) = Dispatcher::new(routing, Arc::default());
        let world: toml::Table = toml::from_str("source = \"midi\"\nlisten = [\"midi.note\"]\nport = 2\nchannel = 1\n").unwrap();
        let filter = deck_space::world::World::read("midi", &world).unwrap().filter;

        let fake = Fake::default();
        fake.plug("Keystep");
        fake.plug("Elektron Analog Four MKII");
        let mut hub = Hub::new(fake.clone(), names.clone());
        let mut send = |e: Event| assert!(inlet.send(e));
        hub.scan(&mut send);
        assert_eq!(hub.present(), [(1, "Elektron Analog Four MKII".to_string()), (2, "Keystep".to_string())]);
        let mut heard = |d: &mut Dispatcher| {
            d.drain();
            std::iter::from_fn(|| ring.pop().ok()).filter(|e| filter.passes(e.props.get(port), e.props.get(ch))).map(|e| (e.name, e.props.get(key).unwrap())).collect::<Vec<_>>()
        };
        fake.play("Keystep", &[0x90, 60, 100]); // port 2, channel 1: heard
        fake.play("Keystep", &[0x91, 61, 100]); // channel 2: not
        fake.play("Elektron Analog Four MKII", &[0x90, 62, 100]); // port 1: not
        assert_eq!(hub.pump(Duration::ZERO, &mut send), 3);
        assert_eq!(heard(&mut d), [(note, 60.0)], "port 2 channel 1 only");

        fake.unplug("Keystep");
        hub.scan(&mut send);
        assert_eq!(heard(&mut d), [(off, 60.0)], "the gone keyboard's held note ends (its channel 2 note too, filtered out)");
        fake.plug("Keystep");
        hub.scan(&mut send);
        assert_eq!(hub.present()[1], (2, "Keystep".to_string()), "back on its own port");
        fake.play("Keystep", &[0x90, 64, 100]);
        hub.pump(Duration::ZERO, &mut send);
        assert_eq!(heard(&mut d), [(note, 64.0)]);
    }
}
