//! The runner (#211, feature `host`): the engine on its own thread, its messages played through the bridge's adapters
//! (`adapters-core::Adapter` – the MIDI out port, the USB-DMX widget) and the conductor's callback. The host's frame
//! thread writes the deck's values into `Values` (atomics: no lock between the frame and the runner); the clock comes
//! from a `ClockSource` (the stage: its tempo map through the time base, on the monotonic clock).
//!
//! Timing like MIDI clock out (deck-native tempo/midi.rs): sleep towards the engine's next wake in steps of ≤ 5 ms (so
//! a turned rate is seen within 5 ms), wait out the last 2 ms exactly, play everything due, step again.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use adapters_core::Adapter;
use protocol::{DeviceMsg, DmxMessage, MidiBytes};

use crate::config::{MidiPort, Plan};
use crate::engine::{Engine, Out, Timed};
use crate::strobe::Clock;

/// The deck's values for the runner: one f32 per `Plan::paths`, written by the host's frame thread.
#[derive(Clone)]
pub struct Values(Arc<[AtomicU32]>);

impl Values {
    pub fn new(n: usize) -> Values {
        Values((0..n).map(|_| AtomicU32::new(0)).collect())
    }

    pub fn set(&self, i: usize, v: f32) {
        if let Some(a) = self.0.get(i) {
            a.store(v.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn read_into(&self, out: &mut [f32]) {
        for (o, a) in out.iter_mut().zip(self.0.iter()) {
            *o = f32::from_bits(a.load(Ordering::Relaxed));
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Where the runner reads the time and the tempo.
pub trait ClockSource: Send {
    /// the monotonic clock, s
    fn now(&self) -> f64;
    /// the tempo as a line on that clock (None = no tempo yet: strobes dark)
    fn clock(&self) -> Option<Clock>;
}

/// The ports the plan's targets go to; a missing one drops its messages (the host said why at the start).
#[derive(Default)]
pub struct Ports {
    pub dmx: Option<Box<dyn Adapter>>,
    pub midi: Option<Box<dyn Adapter>>,
    /// MIDI for the conductor (the stage's relay link: `deck.midi`)
    pub conductor: Option<Box<dyn FnMut([u8; 3]) + Send>>,
}

impl Ports {
    fn play(&mut self, m: &Timed) {
        let result = match m.out {
            Out::Midi { port: MidiPort::Out, bytes } => match self.midi.as_mut() {
                Some(a) => a.send(&DeviceMsg::Midi(MidiBytes { bytes: bytes.to_vec() })),
                None => Ok(()),
            },
            Out::Midi { port: MidiPort::Conductor, bytes } => {
                if let Some(f) = self.conductor.as_mut() {
                    f(bytes);
                }
                Ok(())
            }
            Out::Dmx { ch, value, now } => match self.dmx.as_mut() {
                Some(a) => {
                    let msg = DeviceMsg::Dmx(DmxMessage { ch: u64::from(ch), values: vec![value], fade: None });
                    if now {
                        a.send_now(&msg)
                    } else {
                        a.send(&msg)
                    }
                }
                None => Ok(()),
            },
        };
        if let Err(e) = result {
            tracing::warn!("outputs: {e:#}");
        }
    }

    fn panic(&mut self) {
        for a in [self.dmx.as_mut(), self.midi.as_mut()].into_iter().flatten() {
            if let Err(e) = a.panic() {
                tracing::warn!("outputs panic: {e:#}");
            }
        }
    }

    fn stop(&mut self) {
        for a in [self.dmx.as_mut(), self.midi.as_mut()].into_iter().flatten() {
            if let Err(e) = a.stop() {
                tracing::warn!("outputs stop: {e:#}");
            }
        }
    }
}

/// The running outputs; dropping it stops the thread and the adapters (the last DMX look stands – the widget repeats
/// it, like the bridge).
pub struct Runner {
    stop: Arc<AtomicBool>,
    panic: Arc<AtomicBool>,
    /// stop with a blackout (the stage's scene ends: the lamp must not stay lit on the widget's last frame)
    dark: Arc<AtomicBool>,
    /// #264: the next scene's plan, waiting for the thread to take it
    swap: Arc<Mutex<Option<Swap>>>,
    thread: Option<JoinHandle<()>>,
}

/// A scene switch on the same devices: the thread keeps the adapters (no close / reopen, no dark frame) and plays the
/// new plan from the look the old one left, fading over `secs`.
struct Swap {
    plan: Plan,
    values: Values,
    secs: f64,
}

impl Runner {
    /// B.6 `panic`: the flashes end, the devices go dark / silent.
    pub fn panic(&self) {
        self.panic.store(true, Ordering::Relaxed);
    }

    /// Stop, and black out the devices on the way (the widget would repeat the last look for ever).
    pub fn stop_dark(&mut self) {
        self.dark.store(true, Ordering::Relaxed);
        self.stop();
    }

    /// #264: the next scene's plan on the same devices (the same DMX widget and MIDI port – the caller checks), its
    /// values in `values`; the lamps fade from the last look to the new one over `secs`, channels it doesn't drive to 0.
    pub fn hand_over(&self, plan: Plan, values: Values, secs: f64) {
        *self.swap.lock().unwrap_or_else(|e| e.into_inner()) = Some(Swap { plan, values, secs });
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the adapters (already built – `start` is called here) and the runner's thread.
pub fn spawn(plan: Plan, mut ports: Ports, values: Values, clock: Box<dyn ClockSource>) -> anyhow::Result<Runner> {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    for a in [ports.dmx.as_mut(), ports.midi.as_mut()].into_iter().flatten() {
        a.start(tx.clone())?;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let panic = Arc::new(AtomicBool::new(false));
    let dark = Arc::new(AtomicBool::new(false));
    let swap = Arc::new(Mutex::new(None));
    let (s, p, d, w) = (stop.clone(), panic.clone(), dark.clone(), swap.clone());
    let thread = std::thread::Builder::new().name("deck-outputs".into()).spawn(move || run(plan, ports, values, clock, &s, &p, &d, &w))?;
    Ok(Runner { stop, panic, dark, swap, thread: Some(thread) })
}

#[allow(clippy::too_many_arguments)]
fn run(plan: Plan, mut ports: Ports, mut values: Values, clock: Box<dyn ClockSource>, stop: &AtomicBool, panic: &AtomicBool, dark: &AtomicBool, swap: &Mutex<Option<Swap>>) {
    let mut engine = Engine::new(plan);
    let mut vals = vec![0.0f32; values.len()];
    let mut out: Vec<Timed> = Vec::with_capacity(64);
    while !stop.load(Ordering::Relaxed) {
        if panic.swap(false, Ordering::Relaxed) {
            engine.panic();
            ports.panic();
        }
        if let Some(s) = swap.lock().unwrap_or_else(|e| e.into_inner()).take() {
            engine = std::mem::replace(&mut engine, Engine::new(Plan::default())).hand_over(s.plan, s.secs);
            values = s.values;
            vals = vec![0.0f32; values.len()];
        }
        values.read_into(&mut vals);
        let tempo = clock.clock();
        out.clear();
        let wake = engine.step(clock.now(), &vals, tempo.as_ref(), &mut out);
        for m in &out {
            ports.play(m);
        }
        // towards the next wake: ≤ 5 ms steps (a turned knob is seen soon), the last 2 ms exact
        loop {
            let left = wake - clock.now();
            if left <= 0.0 || stop.load(Ordering::Relaxed) {
                break;
            }
            if left > 0.003 {
                std::thread::sleep(Duration::from_secs_f64((left - 0.002).min(0.005)));
                if wake - clock.now() > 0.002 {
                    break; // step again: the values or the tempo may have changed the plan
                }
            } else {
                std::hint::spin_loop();
            }
        }
    }
    if dark.load(Ordering::Relaxed) {
        ports.panic();
    }
    ports.stop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use std::sync::Mutex;
    use std::time::Instant;

    fn plan(text: &str) -> Plan {
        let doc: toml::Table = toml::from_str(text).unwrap();
        config::parse(doc["outputs"].as_table().unwrap(), None, &|n| Err(n.to_owned())).unwrap()
    }

    /// the test clock: Instant since `epoch` (Linux CI: the monotonic clock is the uptime – only differences matter)
    struct Fixed {
        epoch: Instant,
        tempo: Clock,
    }

    impl ClockSource for Fixed {
        fn now(&self) -> f64 {
            self.epoch.elapsed().as_secs_f64()
        }
        fn clock(&self) -> Option<Clock> {
            Some(self.tempo)
        }
    }

    /// a MIDI port in memory: every message with the moment it arrived
    type Log = Arc<Mutex<Vec<(Instant, Vec<u8>)>>>;

    #[derive(Clone, Default)]
    struct MockMidi(Log);

    impl Adapter for MockMidi {
        fn name(&self) -> &'static str {
            "midi"
        }
        fn start(&mut self, _: tokio::sync::mpsc::Sender<adapters_core::Inbound>) -> anyhow::Result<()> {
            Ok(())
        }
        fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
            let DeviceMsg::Midi(m) = msg else { anyhow::bail!("not midi") };
            self.0.lock().unwrap().push((Instant::now(), m.bytes.clone()));
            Ok(())
        }
        fn stop(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn midi_bytes_through_a_mock_port_and_the_conductor() {
        let midi = MockMidi::default();
        let conductor: Arc<Mutex<Vec<[u8; 3]>>> = Arc::default();
        let c = conductor.clone();
        let p = plan("[outputs]\nmidi = \"mock\"\n[[outputs.map]]\nfrom = \"fx.amount\"\nto = [{ cc = 74, channel = 3 }, { cc = 20, port = \"conductor\", invert = true }]\n");
        let values = Values::new(p.paths.len());
        let ports = Ports { midi: Some(Box::new(midi.clone())), conductor: Some(Box::new(move |b| c.lock().unwrap().push(b))), ..Ports::default() };
        let clock = Box::new(Fixed { epoch: Instant::now(), tempo: Clock::new(0.0, 0.0, 120.0, 4) });
        let mut r = spawn(p, ports, values.clone(), clock).unwrap();
        // the frame thread turns the knob: 0 → 1 in 11 steps
        for i in 0..=10 {
            values.set(0, i as f32 / 10.0);
            std::thread::sleep(Duration::from_millis(40));
        }
        let start = Instant::now();
        while midi.0.lock().unwrap().last().map(|(_, b)| b[2]) != Some(127) && start.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_millis(5));
        }
        r.stop();
        let got = midi.0.lock().unwrap().clone();
        assert!(got.iter().all(|(_, b)| b[0] == 0xB2 && b[1] == 74), "CC 74 on channel 3: {got:?}");
        assert_eq!(got.first().unwrap().1, vec![0xB2, 74, 0]);
        assert_eq!(got.last().unwrap().1, vec![0xB2, 74, 127]);
        assert!(got.windows(2).all(|w| w[1].1[2] > w[0].1[2]), "only changes, rising");
        // ≤ one per 25 ms frame: 11 changes, each one frame
        assert!(got.len() <= 11, "{}", got.len());
        let c = conductor.lock().unwrap();
        assert_eq!((c.first().copied(), c.last().copied()), (Some([0xB0, 20, 127]), Some([0xB0, 20, 0])), "inverted, to the conductor");
    }

    /// The strobe plan of the two tests below, run until the mock widget has seen `want` flashes (a deadline of 60 s, paid
    /// only on failure: the schedule runs on the wall clock, so a loaded machine only delays the flashes, #382); returns
    /// the song's epoch and the widget.
    fn strobe_run(want: usize) -> (Instant, adapters_dmx_usbpro::mock::Widget) {
        use adapters_dmx_usbpro::{mock::Widget, DmxUsbProAdapter};
        let widget = Widget::default();
        let dmx = DmxUsbProAdapter::with_opener("strobe", None, widget.opener());
        // 150 BPM, the neutral 1/4 stepped up to 1/8 (p = 0.7): a flash every 200 ms, 30 ms long, a lead of 20 ms; a
        // dimmer on channel 10 from the same knob
        let p = plan(
            "[outputs]\ndmx = \"auto\"\nlead = 20\n[[outputs.fixture]]\nid = \"s\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.strobe]]\nfixture = \"s\"\nrate = \"r.x\"\nduration = 30\n[[outputs.map]]\nfrom = \"r.x\"\nto = [{ dmx = 10 }]\n",
        );
        let values = Values::new(p.paths.len());
        values.set(0, 0.7);
        let epoch = Instant::now();
        // the song started 0.1 s from now – the first flash heard at 0.1 s
        let tempo = Clock::new(0.1, 0.0, 150.0, 4);
        let mut r = spawn(p, Ports { dmx: Some(Box::new(dmx)), ..Ports::default() }, values, Box::new(Fixed { epoch, tempo })).unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        while flash_ups(epoch, &widget).len() < want && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        r.stop();
        (epoch, widget)
    }

    /// the moments (seconds after `epoch`) channel 1 went up in the widget's frames
    fn flash_ups(epoch: Instant, widget: &adapters_dmx_usbpro::mock::Widget) -> Vec<f64> {
        let mut ups = Vec::new();
        let mut prev = 0;
        for (t, f) in &widget.frames() {
            if f[0] == 255 && prev != 255 {
                ups.push((*t - epoch).as_secs_f64());
            }
            prev = f[0];
        }
        ups
    }

    /// the states: the dimmer is the knob's value, the strobe flashes and each flash ends (no bound on when - that is
    /// `the_strobe_flashes_on_the_beat`, a real-time check)
    #[test]
    fn strobe_and_dimmer_frames_through_the_mock_widget() {
        let (epoch, widget) = strobe_run(4);
        let frames = widget.frames();
        // the dimmer: 0.7 -> 179 on channel 10, the rests (2, 3) at 0
        let last = frames.last().unwrap().1;
        assert_eq!((last[9], last[1], last[2]), (179, 0, 0));
        let ups = flash_ups(epoch, &widget);
        assert!(ups.len() >= 4, "flashes {ups:?}");
        // and each one ends: dark again
        let downs = frames.windows(2).filter(|w| w[0].1[0] == 255 && w[1].1[0] == 0).count();
        assert!(downs + 1 >= ups.len(), "{downs} ends for {} flashes", ups.len());
    }

    /// the flash frames: the moments channel 1 went up, against each eighth's moment - the lead. Real time on purpose (the
    /// output thread's wake-ups against the wall clock): it needs a quiet machine, so it is not in deck:ci (#382) - the
    /// hardware rig runs it with `--ignored`
    #[test]
    #[ignore = "hardware rig / quiet machine: run with --ignored"]
    fn the_strobe_flashes_on_the_beat() {
        let (epoch, widget) = strobe_run(4);
        let ups = flash_ups(epoch, &widget);
        assert!(ups.len() >= 4, "flashes {ups:?}");
        for t in &ups {
            // the nearest eighth (0.2 s) heard at 0.1 + k x 0.2, sent 20 ms before
            let k = ((t + 0.02 - 0.1) / 0.2).round();
            let want = 0.1 + k * 0.2 - 0.02;
            // measured bounds (thread wake-ups on a quiet machine): never early by more than 1 ms, at most 15 ms late
            assert!(*t - want > -0.001 && *t - want < 0.015, "a flash at {t:.4} s, its moment {want:.4} s");
        }
    }

    /// a scene that ends leaves the lamp dark (the widget would repeat the last frame), a plain stop leaves the look
    #[test]
    fn stop_dark_blacks_the_widget_out() {
        use adapters_dmx_usbpro::{mock::Widget, DmxUsbProAdapter};
        for dark in [true, false] {
            let widget = Widget::default();
            let dmx = DmxUsbProAdapter::with_opener("lamp", None, widget.opener());
            let p = plan("[outputs]
dmx = \"auto\"
[[outputs.map]]
from = \"r.x\"
to = [{ dmx = 1 }]
");
            let values = Values::new(p.paths.len());
            values.set(0, 1.0);
            let mut r = spawn(p, Ports { dmx: Some(Box::new(dmx)), ..Ports::default() }, values, Box::new(Fixed { epoch: Instant::now(), tempo: Clock::new(0.1, 0.0, 120.0, 4) })).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            if dark {
                r.stop_dark();
            } else {
                r.stop();
            }
            let last = widget.frames().last().unwrap().1;
            assert_eq!(last[0], if dark { 0 } else { 255 }, "dark = {dark}");
        }
    }
}
