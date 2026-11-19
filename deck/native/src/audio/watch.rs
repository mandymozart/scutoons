//! The saved audio device comes back by itself. While it is away – missing at the start, or lost while playing – the
//! engine plays on a timer (silent; the clock and the show go on) and a thread looks for the device every `EVERY`. Found,
//! the device is opened at the engine's rate and takes the engine as it is – the saved buffer and channel map, the frame
//! count – and the timer stops. The audio path only takes the engine once (a wait-free pop) and stamps when its next frame
//! is heard; everything else runs on the watch's thread. deck.toml is never written here.
use super::backend::Feed;
use crate::clock::now_ns;
use anyhow::Result;
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const EVERY: Duration = Duration::from_secs(2);

/// frames per timer block
const BLOCK: usize = 512;

/// how long an opened device may take to call back before it takes the engine anyway
const FIRST_CALL: Duration = Duration::from_millis(500);

/// What the watch says of the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    /// the device is away: the deck runs silent
    pub away: bool,
}

#[derive(Clone, Default)]
pub struct Notes(Arc<Mutex<Option<Note>>>);

impl Notes {
    pub fn get(&self) -> Option<Note> {
        self.0.lock().ok()?.clone()
    }

    fn set(&self, text: String, away: bool) {
        // TODO(#608): the status bar shows it beside the Audio panel
        if away {
            tracing::warn!("audio: {text}");
        } else {
            tracing::info!("audio: {text}");
        }
        if let Ok(mut n) = self.0.lock() {
            *n = Some(Note { text, away });
        }
    }
}

/// The feed a device callback plays: its own, or the one the watch hands over (silence until it arrives), handed back
/// when the stream ends.
pub struct Held {
    feed: Option<Feed>,
    hand: Option<Hand>,
}

/// A watched callback's side of the hand-over.
pub struct Hand {
    next: rtrb::Consumer<Feed>,
    back: Sender<Feed>,
    heard: Arc<AtomicU64>,
    resume: Arc<AtomicU64>,
}

impl Held {
    pub fn own(feed: Feed) -> Held {
        Held { feed: Some(feed), hand: None }
    }

    pub fn handed(hand: Hand) -> Held {
        Held { feed: None, hand: Some(hand) }
    }

    /// the feed once it is here; wait-free
    pub fn feed(&mut self) -> Option<&mut Feed> {
        if self.feed.is_none() {
            self.feed = self.hand.as_mut().and_then(|h| h.next.pop().ok());
        }
        self.feed.as_mut()
    }

    /// the frame after this callback's is heard at `ns`: the watch's heartbeat, and – once the feed plays here – where
    /// the timer goes on from
    pub fn heard(&self, ns: u64) {
        if let Some(h) = &self.hand {
            h.heard.store(ns, Ordering::Relaxed);
            if self.feed.is_some() {
                h.resume.store(ns, Ordering::Relaxed);
            }
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(h) = &mut self.hand {
            if let Some(f) = self.feed.take().or_else(|| h.next.pop().ok()) {
                let _ = h.back.send(f);
            }
        }
    }
}

enum Wake {
    /// the stream of this opening says its device is gone
    Lost(u64),
    /// the host says a device appeared: look now, not at the next tick
    Look,
    Stop,
}

/// A stream's error callback: its device is gone.
#[derive(Clone)]
pub struct Lost(Sender<Wake>, u64);

impl Lost {
    pub fn now(&self) {
        let _ = self.0.send(Wake::Lost(self.1));
    }
}

/// A host's device events: the saved device may be there now.
#[derive(Clone)]
pub struct Waker(Sender<Wake>);

impl Waker {
    pub fn look(&self) {
        let _ = self.0.send(Wake::Look);
    }
}

/// What an opening gets from the watch: a hand per callback built, the lost signal.
pub struct Link {
    give: Option<rtrb::Producer<Feed>>,
    back_tx: Sender<Feed>,
    back: Receiver<Feed>,
    heard: Arc<AtomicU64>,
    /// when the feed's next frame is heard: the timer's schedule at the hand-over, then the callbacks that play it
    /// (#638: not the heartbeat – a stream lost before its callback took the engine has stamped only silence)
    resume: Arc<AtomicU64>,
    wake: Sender<Wake>,
    opening: u64,
}

impl Link {
    pub fn hand(&mut self) -> Hand {
        let (tx, rx) = rtrb::RingBuffer::new(1);
        self.give = Some(tx);
        Hand { next: rx, back: self.back_tx.clone(), heard: self.heard.clone(), resume: self.resume.clone() }
    }

    pub fn lost(&self) -> Lost {
        Lost(self.wake.clone(), self.opening)
    }
}

/// The device as the watch sees it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub rate: u32,
    pub channels: usize,
}

pub struct Opened {
    /// dropped = closed; the callback's `Held` goes with it
    pub stream: Box<dyn Any>,
    pub wasapi: bool,
}

/// The saved device on its host (cpal's in a deck, a fake in the tests).
pub trait Port: Send {
    /// the saved name
    fn name(&self) -> String;
    /// the device, if it is there and says its config
    fn find(&mut self) -> Option<Found>;
    /// a playing stream on it at `rate`, its callbacks' hands from `link`
    fn open(&mut self, rate: u32, link: &mut Link) -> Result<Opened>;
    /// a host that hears its devices come and go wakes the watch with this (the tick looks anyway)
    fn watch(&mut self, _waker: Waker) {}
}

/// The engine on a timer thread, its blocks heard on their schedule from `from_ns` on. Every wake plays all the blocks
/// due by then, so a schedule in the past (a device that went quiet) or a late wake (a loaded machine) is caught up and
/// the frames keep their time; ending it plays what is due too, in the thread that takes the engine, so the device
/// takes it where the time is (#638: a block a wake fell behind by the wakes lost, the device came back with a jump).
pub struct Timer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<(Feed, u64)>>,
    schedule: Schedule,
}

/// A timer's blocks: block `k` is heard at `at(k)`.
#[derive(Clone, Copy)]
struct Schedule {
    from_ns: u64,
    rate: u32,
    channels: usize,
}

impl Schedule {
    fn at(&self, k: u64) -> u64 {
        self.from_ns + k * BLOCK as u64 * 1_000_000_000 / u64::from(self.rate)
    }

    /// plays the blocks due by now from block `k` on – the first one always, the ones behind it while the feed holds
    /// them (a ring's render thread keeps up between wakes, never run dry by a burst) and a second's at most (a long
    /// stall is caught up a second a wake); the next block
    fn due(&self, feed: &mut Feed, buf: &mut [f32], mut k: u64) -> u64 {
        let now = now_ns();
        for n in 0..(self.rate as usize).div_ceil(BLOCK).max(1) {
            if self.at(k) > now || (n > 0 && !feed.holds(BLOCK)) {
                break;
            }
            feed.play(buf, self.channels, self.at(k), |x| x);
            k += 1;
        }
        k
    }
}

impl Timer {
    pub fn start(mut feed: Feed, rate: u32, channels: usize, from_ns: u64) -> std::io::Result<Timer> {
        feed.on_timer();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let schedule = Schedule { from_ns, rate, channels };
        let thread = std::thread::Builder::new().name("audio-null".into()).spawn(move || {
            let mut buf = vec![0.0f32; BLOCK * channels];
            let mut k = 0;
            while !flag.load(Ordering::Relaxed) {
                k = schedule.due(&mut feed, &mut buf, k);
                let wait = schedule.at(k).saturating_sub(now_ns()).max(1_000_000);
                std::thread::sleep(Duration::from_nanos(wait));
            }
            (feed, k)
        })?;
        Ok(Timer { stop, thread: Some(thread), schedule })
    }

    /// stops it and takes the feed back, what is due played, with when its next frame is heard
    pub fn end(mut self) -> Option<(Feed, u64)> {
        self.stop.store(true, Ordering::Relaxed);
        let (mut feed, k) = self.thread.take()?.join().ok()?;
        let mut buf = vec![0.0f32; BLOCK * self.schedule.channels];
        let k = self.schedule.due(&mut feed, &mut buf, k);
        Some((feed, self.schedule.at(k)))
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The watch's thread; dropping it closes the device (or stops the timer).
pub struct Watch {
    wake: Sender<Wake>,
    notes: Notes,
    thread: Option<JoinHandle<()>>,
}

impl Watch {
    /// Watches `port` with this feed (built for `rate`, `channels`): `found` opens it at once, else the timer plays.
    pub fn start(mut port: Box<dyn Port>, feed: Feed, rate: u32, channels: usize, found: Option<Found>, every: Duration) -> std::io::Result<Watch> {
        let (wake_tx, wake) = mpsc::channel();
        port.watch(Waker(wake_tx.clone()));
        let (back_tx, back) = mpsc::channel();
        let notes = Notes::default();
        let link = Link { give: None, back_tx, back, heard: Arc::new(AtomicU64::new(0)), resume: Arc::new(AtomicU64::new(0)), wake: wake_tx.clone(), opening: 0 };
        let (ready_tx, ready) = mpsc::channel();
        let said = notes.clone();
        let thread = std::thread::Builder::new().name("audio-watch".into()).spawn(move || {
            let name = found.as_ref().map_or_else(|| port.name(), |f| f.name.clone());
            let mut run = Run { port, rate, channels, link, notes: said, name, state: State::Gone, tried: None };
            run.silent(feed, now_ns());
            match found {
                Some(_) => run.open(false),
                None => run.notes.set(format!("'{}' not found – running silent, waiting for it", run.name), true),
            }
            let _ = ready_tx.send(());
            run.watch(&wake, every);
        })?;
        let _ = ready.recv();
        Ok(Watch { wake: wake_tx, notes, thread: Some(thread) })
    }

    pub fn notes(&self) -> Notes {
        self.notes.clone()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.wake.send(Wake::Stop);
        let Some(t) = self.thread.take() else { return };
        // bounded: the next stage may open the same device right after
        let end = Instant::now() + Duration::from_secs(2);
        while !t.is_finished() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(5));
        }
        if t.is_finished() {
            let _ = t.join();
        } else {
            tracing::warn!("audio: the device watch did not end – left behind");
        }
    }
}

enum State {
    Playing { _stream: Box<dyn Any>, seen: u64 },
    Silent(Timer),
    Gone,
}

struct Run {
    port: Box<dyn Port>,
    rate: u32,
    channels: usize,
    link: Link,
    notes: Notes,
    /// the device's name as found, else the saved one
    name: String,
    /// the last failed opening's reason (said once)
    tried: Option<String>,
    state: State,
}

impl Run {
    fn watch(&mut self, wake: &Receiver<Wake>, every: Duration) {
        let mut tick = Instant::now() + every;
        loop {
            match wake.recv_timeout(tick.saturating_duration_since(Instant::now())) {
                Ok(Wake::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(Wake::Lost(opening)) if opening == self.link.opening => self.lost(),
                Ok(Wake::Lost(_)) => {}
                Ok(Wake::Look) => self.look(),
                Err(RecvTimeoutError::Timeout) => {
                    tick = Instant::now() + every;
                    self.tick();
                }
            }
        }
    }

    fn tick(&mut self) {
        match &mut self.state {
            State::Playing { seen, .. } => {
                let heard = self.link.heard.load(Ordering::Relaxed);
                // no callback since the last tick: gone without a word
                if heard == *seen {
                    self.lost();
                } else {
                    *seen = heard;
                }
            }
            State::Silent(_) => self.look(),
            State::Gone => {}
        }
    }

    /// silent: the device opened if it is there
    fn look(&mut self) {
        if !matches!(self.state, State::Silent(_)) {
            return;
        }
        if let Some(f) = self.port.find() {
            self.name = f.name;
            self.open(true);
        }
    }

    /// the timer plays the feed from `from_ns` on
    fn silent(&mut self, feed: Feed, from_ns: u64) {
        self.state = match Timer::start(feed, self.rate, self.channels, from_ns) {
            Ok(t) => State::Silent(t),
            Err(e) => {
                tracing::error!("audio: no timer thread ({e}) – the engine stops");
                State::Gone
            }
        };
    }

    /// The device opens first (silent until the engine arrives) and calls back, then the timer hands the engine over –
    /// not before: the frames due while a device starts up are the timer's (#638: they were nobody's, the clock jumped).
    fn open(&mut self, back: bool) {
        self.link.opening += 1;
        self.link.heard.store(0, Ordering::Relaxed);
        let opened = match self.port.open(self.rate, &mut self.link) {
            Ok(o) => o,
            Err(e) => {
                let why = format!("{e:#}");
                if self.tried.as_deref() != Some(why.as_str()) {
                    self.notes.set(format!("'{}' does not open ({why}) – running silent, trying again", self.name), true);
                    self.tried = Some(why);
                }
                return;
            }
        };
        self.tried = None;
        let end = Instant::now() + FIRST_CALL;
        while self.link.heard.load(Ordering::Relaxed) == 0 && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(1));
        }
        let State::Silent(timer) = std::mem::replace(&mut self.state, State::Gone) else { return };
        let Some((mut feed, next)) = timer.end() else {
            tracing::error!("audio: the timer ended without the engine");
            return;
        };
        feed.on_device(opened.wasapi);
        let Some(give) = self.link.give.as_mut() else { return };
        self.link.resume.store(next, Ordering::Relaxed);
        if let Err(rtrb::PushError::Full(feed)) = give.push(feed) {
            self.silent(feed, next);
            return;
        }
        self.state = State::Playing { _stream: opened.stream, seen: self.link.heard.load(Ordering::Relaxed) };
        if back {
            self.notes.set(format!("audio back on '{}'", self.name), false);
        }
    }

    /// The stream is closed, its callback hands the engine back, the timer goes on where the device stopped.
    fn lost(&mut self) {
        if !matches!(self.state, State::Playing { .. }) {
            return;
        }
        self.state = State::Gone;
        let from = self.link.resume.load(Ordering::Relaxed);
        match self.link.back.recv_timeout(Duration::from_secs(1)) {
            Ok(feed) => {
                self.notes.set(format!("'{}' lost – running silent, waiting for it", self.name), true);
                self.silent(feed, if from > 0 { from } else { now_ns() });
            }
            Err(_) => self.notes.set(format!("'{}' lost, and the engine with it – Apply restarts the sound", self.name), true),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::Scene;
    use std::sync::atomic::AtomicUsize;

    pub(in crate::audio) const RATE: u32 = 48_000;
    const FAKE_BLOCK: usize = 256;

    /// A device that can be plugged in and pulled out; what it plays lands in `peaks`.
    #[derive(Clone)]
    pub(in crate::audio) struct Fake {
        there: Arc<AtomicBool>,
        /// pulled while playing: its callbacks stop
        pub(in crate::audio) quiet: Arc<AtomicBool>,
        pub(in crate::audio) opens: Arc<AtomicUsize>,
        peaks: Arc<Mutex<Vec<f32>>>,
        lost: Arc<Mutex<Option<Lost>>>,
        channels: usize,
    }

    impl Fake {
        pub(in crate::audio) fn new(there: bool, channels: usize) -> Fake {
            Fake {
                there: Arc::new(AtomicBool::new(there)),
                quiet: Arc::new(AtomicBool::new(false)),
                opens: Arc::new(AtomicUsize::new(0)),
                peaks: Arc::new(Mutex::new(vec![0.0; channels])),
                lost: Arc::new(Mutex::new(None)),
                channels,
            }
        }

        fn plug(&self, there: bool) {
            self.there.store(there, Ordering::Relaxed);
            self.quiet.store(!there, Ordering::Relaxed);
        }

        fn take_peaks(&self) -> Vec<f32> {
            std::mem::replace(&mut *self.peaks.lock().unwrap(), vec![0.0; self.channels])
        }

        /// a playing stream: its callbacks on the device's own clock, quiet while it is pulled
        pub(in crate::audio) fn stream(&self, mut held: Held) -> FakeStream {
            self.opens.fetch_add(1, Ordering::Relaxed);
            let stop = Arc::new(AtomicBool::new(false));
            let me = self.clone();
            let halt = stop.clone();
            let thread = std::thread::spawn(move || {
                let ch = me.channels;
                let mut buf = vec![0.0f32; FAKE_BLOCK * ch];
                let t0 = now_ns();
                let at = |k: u64| t0 + k * FAKE_BLOCK as u64 * 1_000_000_000 / u64::from(RATE);
                let mut k = 0;
                while !halt.load(Ordering::Relaxed) {
                    if me.quiet.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    // the device's own clock: a late callback (a loaded machine) does not move when its frames are heard
                    let heard = at(k) + 1_000_000;
                    match held.feed() {
                        Some(f) => f.play(&mut buf, ch, heard, |x| x),
                        None => buf.fill(0.0),
                    }
                    held.heard(heard + FAKE_BLOCK as u64 * 1_000_000_000 / u64::from(RATE));
                    let mut peaks = me.peaks.lock().unwrap();
                    for frame in buf.chunks_exact(ch) {
                        for (p, x) in peaks.iter_mut().zip(frame) {
                            *p = p.max(x.abs());
                        }
                    }
                    drop(peaks);
                    k += 1;
                    std::thread::sleep(Duration::from_nanos(at(k).saturating_sub(now_ns())));
                }
            });
            FakeStream { stop, thread: Some(thread) }
        }
    }

    pub(in crate::audio) struct FakeStream {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Drop for FakeStream {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }

    impl Port for Fake {
        fn name(&self) -> String {
            "UCX".into()
        }

        fn find(&mut self) -> Option<Found> {
            self.there.load(Ordering::Relaxed).then(|| Found { name: "Fake UCX II".into(), rate: RATE, channels: self.channels })
        }

        fn open(&mut self, rate: u32, link: &mut Link) -> Result<Opened> {
            anyhow::ensure!(self.there.load(Ordering::Relaxed), "not there");
            anyhow::ensure!(rate == RATE, "{rate} Hz");
            *self.lost.lock().unwrap() = Some(link.lost());
            Ok(Opened { stream: Box::new(self.stream(Held::handed(link.hand()))), wasapi: false })
        }
    }

    /// a test tone on the group `cue`, which the saved map puts on channels 3 and 4
    pub(in crate::audio) fn engine(channels: usize) -> (super::super::Engine, super::super::Handles) {
        let map = [deck_engine::OutletMap { group: "cue".into(), channels: vec![3, 4] }];
        assert_eq!(super::super::backend::map_channels(&map), 4);
        let tone = deck_engine::Tone::Group("cue".into());
        let graph = Graph::build(&Scene::parse(&tone.scene()).unwrap()).unwrap();
        super::super::Engine::build_mapped(&graph, RATE as f32, 1.0, super::super::detect_build(), deck_engine::Sequencing::InBlock, channels, &map).unwrap()
    }

    pub(in crate::audio) fn wait(what: &str, secs: u64, mut ok: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(secs);
        while !ok() {
            assert!(Instant::now() < end, "waited {secs} s for {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub(in crate::audio) fn plays_the_map(fake: &Fake) {
        fake.take_peaks();
        let mut p = vec![0.0f32; fake.channels];
        wait("the tone", 10, || {
            for (a, b) in p.iter_mut().zip(fake.take_peaks()) {
                *a = a.max(b);
            }
            p[2] > 0.01 && p[3] > 0.01
        });
        assert!(p[0] == 0.0 && p[1] == 0.0, "the tone on 3 and 4 only: {p:?}");
    }

    pub(in crate::audio) fn away(notes: &Notes) -> bool {
        notes.get().is_some_and(|n| n.away)
    }

    fn back(notes: &Notes) -> bool {
        notes.get().is_some_and(|n| !n.away && n.text.contains("back on 'Fake UCX II'"))
    }

    #[test]
    fn a_device_missing_at_the_start_plays_the_saved_map_once_it_appears() {
        let fake = Fake::new(false, 8);
        let (engine, handles) = engine(4);
        let feed = Feed::new(engine, true, Some(128)).unwrap();
        let watch = Watch::start(Box::new(fake.clone()), feed, RATE, 4, None, Duration::from_millis(50)).unwrap();
        let notes = watch.notes();
        assert_eq!(notes.get().map(|n| (n.text, n.away)), Some(("'UCX' not found – running silent, waiting for it".into(), true)));
        let rendered = handles.clock.rendered();
        wait("the timer to play", 5, || handles.clock.rendered() > rendered + 4096);
        assert_eq!(fake.opens.load(Ordering::Relaxed), 0);

        fake.plug(true);
        wait("the device back", 5, || back(&notes));
        assert_eq!(fake.opens.load(Ordering::Relaxed), 1);
        plays_the_map(&fake);
        drop(watch);
    }

    #[test]
    fn a_device_lost_while_playing_runs_silent_and_comes_back_without_a_jump() {
        let fake = Fake::new(true, 4);
        let (engine, handles) = engine(4);
        let feed = Feed::new(engine, false, None).unwrap();
        let found = fake.clone().find();
        // the loss is said by the error callback here: a tick long enough that a starved fake (a loaded test run) is not
        // lost besides
        let watch = Watch::start(Box::new(fake.clone()), feed, RATE, 4, found, Duration::from_millis(250)).unwrap();
        let notes = watch.notes();
        assert_eq!(notes.get(), None, "there at the start: nothing to say");
        plays_the_map(&fake);

        // the clock: when a frame is heard, against the rate – a jump moves it
        let offset = || {
            let (_, [frames, heard]) = handles.clock.stamp.read().unwrap();
            heard as f64 * 1e-6 - frames as f64 * 1e3 / f64::from(RATE)
        };
        let o0 = offset();
        let mut worst = 0.0f64;
        // a fifth of a second of the engine's own frames (the timer's or the device's, not the wall's), the clock checked
        // all along
        let mut track = |label: &str| {
            let to = handles.clock.rendered() + u64::from(RATE / 5);
            wait(label, 10, || {
                let d = offset() - o0;
                worst = worst.max(d.abs());
                assert!(d.abs() < 40.0, "{label}: the clock moved {d:+.1} ms");
                handles.clock.rendered() >= to
            });
        };

        // pulled: its callbacks stop, its error callback says so
        fake.plug(false);
        fake.lost.lock().unwrap().as_ref().unwrap().now();
        wait("the loss said", 5, || away(&notes));
        assert!(notes.get().unwrap().text.contains("'Fake UCX II' lost – running silent"));
        // the stream is closed once the loss is said: what it played before is not the silent phase's
        fake.take_peaks();
        track("silent");
        assert!(fake.take_peaks().iter().all(|&p| p == 0.0), "nothing plays while it is away");

        fake.plug(true);
        wait("the device back", 5, || back(&notes));
        plays_the_map(&fake);
        track("back");
        assert_eq!(fake.opens.load(Ordering::Relaxed), 2, "opened once more only: {:?}", notes.get());
        drop(watch);
        eprintln!("the clock moved {worst:.1} ms at most");
    }

    /// #638: a schedule half a second in the past is caught up by the time the timer ends, however few its wakes were –
    /// the device that takes the engine then goes on where the time is, without a jump
    #[test]
    fn a_timer_ends_with_its_schedule_caught_up() {
        let (engine, handles) = engine(4);
        let feed = Feed::new(engine, false, None).unwrap();
        let from = now_ns() - 500_000_000;
        let timer = Timer::start(feed, RATE, 4, from).unwrap();
        let before = now_ns();
        assert!(timer.end().is_some(), "the engine back");
        let due = (before - from) * u64::from(RATE) / 1_000_000_000;
        let rendered = handles.clock.rendered();
        assert!(rendered + 2 * BLOCK as u64 >= due, "{rendered} of the {due} frames due were played");
    }

    #[test]
    fn a_device_that_stops_calling_back_is_lost_too() {
        let fake = Fake::new(true, 4);
        let (engine, _handles) = engine(4);
        let feed = Feed::new(engine, true, None).unwrap();
        let found = fake.clone().find();
        let watch = Watch::start(Box::new(fake.clone()), feed, RATE, 4, found, Duration::from_millis(50)).unwrap();
        let notes = watch.notes();
        fake.quiet.store(true, Ordering::Relaxed);
        fake.there.store(false, Ordering::Relaxed);
        wait("the silence noticed", 5, || away(&notes));
        fake.plug(true);
        wait("the device back", 5, || back(&notes));
        plays_the_map(&fake);
    }
}
