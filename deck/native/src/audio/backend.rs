//! Where the engine plays: PipeWire (Linux, feature `pipewire` – the deck), cpal (every OS; on Linux through ALSA), or
//! the null backend (a timer thread, no device – the chosen fallback when there is no audio device, CYBERDECKS.md §2).
//! `auto` tries them in that order and logs why one was skipped. Each one stamps its blocks with the moment they are
//! heard (`Engine::mark`, the time base of clock.rs): PipeWire from its cycle timing (pw_time: now + the delay to the
//! device + what the resampler holds), cpal from the stream's playback instant, the null backend from its schedule.
use super::Engine;
use crate::clock::now_ns;
use anyhow::{bail, Result};
use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Auto,
    PipeWire,
    Cpal,
    Null,
}

impl Choice {
    pub fn parse(s: &str) -> Option<Choice> {
        Some(match s {
            "auto" => Choice::Auto,
            "pipewire" => Choice::PipeWire,
            "cpal" => Choice::Cpal,
            "null" | "none" => Choice::Null,
            _ => return None,
        })
    }
}

/// A playing backend; dropping it stops the audio.
pub struct Running {
    pub name: &'static str,
    pub rate: u32,
    /// asks the stream to fade out ahead of the drop (PipeWire; #250 – the picture fades as long)
    closing: Option<Arc<AtomicBool>>,
    _keep: Box<dyn Any>,
}

impl Running {
    /// Starts the fade-out without waiting; dropping it later finds the stream already silent.
    pub fn fade_out(&self) {
        if let Some(c) = &self.closing {
            c.store(true, Ordering::Relaxed);
        }
    }
}

/// Builds an engine for a sample rate and the device's channel count (the backend reports it: the engine's deck
/// configuration, plan 2.6b); called once per backend tried.
pub type Factory<'a> = dyn FnMut(f32, usize) -> Result<Engine> + 'a;

/// A callback that renders the graph itself, in the device's thread, measured by the glitch monitor (#208).
pub struct Direct {
    engine: Engine,
    mon: super::glitch::Monitor,
    underruns: Arc<std::sync::atomic::AtomicU64>,
    /// a gap in the heard stamps counts as an underrun (false where the stamps jitter: WASAPI's)
    gaps: bool,
}

impl Direct {
    pub fn new(mut engine: Engine) -> Direct {
        let mon = engine.take_monitor();
        let underruns = engine.underruns();
        Direct { engine, mon, underruns, gaps: true }
    }

    /// a host whose callbacks top up one buffer (WASAPI shared): the deadline is the buffer, the stamps' gaps are
    /// jitter (glitch.rs `Deadline::Buffer`)
    fn buffered(mut self) -> Direct {
        self.mon.deadline = super::glitch::Deadline::Buffer;
        self.gaps = false;
        self
    }

    /// One device callback: `out` (interleaved, `channels` per frame) whose first frame is heard at `heard_ns`.
    /// Never allocates.
    pub fn play<T: Copy>(&mut self, out: &mut [T], channels: usize, heard_ns: u64, conv: impl Fn(f32) -> T) {
        let start = self.mon.begin();
        let before = self.underruns.load(Ordering::Relaxed);
        self.engine.mark(heard_ns);
        self.engine.render(out, channels, conv);
        let gap = self.gaps && self.underruns.load(Ordering::Relaxed) != before;
        self.mon.end(start, out.len() / channels.max(1), gap);
    }

    pub fn glitches(&self) -> &super::glitch::Glitches {
        &self.mon.shared
    }
}

/// How a device callback plays (#208): the graph rendered in the callback (`Direct`), or copied from the render
/// thread's ring (ring.rs – the default, `--audio-thread off` renders in the callback).
pub enum Feed {
    Direct(Box<Direct>),
    Ring(super::ring::Feed),
}

impl Feed {
    /// a ring with its render thread (`thread`), else the callback renders; `buffer` = the device's buffer if known
    /// (the ring's first fill). Err only when the OS has no thread to give.
    pub fn new(engine: Engine, thread: bool, buffer: Option<u32>) -> Result<Feed> {
        Ok(match thread {
            true => Feed::Ring(super::ring::spawn(engine, buffer)?),
            false => Feed::Direct(Box::new(Direct::new(engine))),
        })
    }

    /// a host whose callbacks top up one buffer (WASAPI shared – glitch.rs `Deadline::Buffer`)
    fn buffered(self) -> Feed {
        match self {
            Feed::Direct(d) => Feed::Direct(Box::new(d.buffered())),
            Feed::Ring(mut r) => {
                r.mon.deadline = super::glitch::Deadline::Buffer;
                Feed::Ring(r)
            }
        }
    }

    pub fn play<T: Copy>(&mut self, out: &mut [T], channels: usize, heard_ns: u64, conv: impl Fn(f32) -> T) {
        match self {
            Feed::Direct(d) => d.play(out, channels, heard_ns, conv),
            Feed::Ring(r) => r.play(out, channels, heard_ns, conv),
        }
    }

    pub fn glitches(&self) -> &super::glitch::Glitches {
        match self {
            Feed::Direct(d) => d.glitches(),
            Feed::Ring(r) => r.glitches(),
        }
    }
}

/// cpal's host (#208): the platform's default (WASAPI shared on Windows), WASAPI by name, or ASIO (the `asio` feature)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Host {
    #[default]
    Default,
    Wasapi,
    Asio,
}

impl Host {
    pub fn parse(s: &str) -> Option<Host> {
        Some(match s.to_lowercase().as_str() {
            "" | "default" | "auto" => Host::Default,
            "wasapi" => Host::Wasapi,
            "asio" => Host::Asio,
            _ => return None,
        })
    }
}

/// How the audio plays (#208): the backend, cpal's host and device, the buffer asked of the device.
#[derive(Debug, Clone)]
pub struct Options {
    pub choice: Choice,
    pub host: Host,
    /// a part of the output device's name (any case); None = the host's default
    pub device: Option<String>,
    /// frames; None = the device's default
    pub buffer: Option<u32>,
    /// render on a thread of its own into a ring ahead of the device (ring.rs); false = in the device callback
    pub thread: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { choice: Choice::Auto, host: Host::Default, device: None, buffer: None, thread: true }
    }
}

/// deck.toml's audio keys (#208) – the machine's settings; the stage's flags win over them, they win over the scene's.
/// `audio_buffer = 256`, `audio_host = "asio"`, `audio_device = "Fireface"`; empty = unset.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct DeckAudio {
    #[serde(default)]
    pub audio_buffer: Option<u32>,
    #[serde(default)]
    pub audio_host: Option<String>,
    #[serde(default)]
    pub audio_device: Option<String>,
    #[serde(default)]
    pub audio_thread: Option<bool>,
}

impl DeckAudio {
    /// deck.toml's text (its other keys are the session's; a broken file = nothing set, said in the log)
    pub fn parse(text: &str) -> DeckAudio {
        let mut d: DeckAudio = toml::from_str(text).unwrap_or_else(|e| {
            tracing::warn!("deck.toml (audio): {e}");
            DeckAudio::default()
        });
        d.audio_host = d.audio_host.filter(|s| !s.trim().is_empty());
        d.audio_device = d.audio_device.filter(|s| !s.trim().is_empty());
        d.audio_buffer = d.audio_buffer.filter(|&n| n > 0);
        // audio_thread stays as written (true / false; unset = the default)
        d
    }

    /// the deck's file (`DECK_TOML` names another – a dev machine's), nothing set when there is none
    pub fn read() -> DeckAudio {
        let path = std::env::var_os("DECK_TOML").map(std::path::PathBuf::from).unwrap_or_else(|| "/var/lib/deck/deck.toml".into());
        std::fs::read_to_string(path).map(|t| DeckAudio::parse(&t)).unwrap_or_default()
    }
}

pub fn start(o: &Options, factory: &mut Factory) -> Result<Running> {
    let order: &[Choice] = match o.choice {
        Choice::Auto => &[Choice::PipeWire, Choice::Cpal, Choice::Null],
        Choice::PipeWire => &[Choice::PipeWire],
        Choice::Cpal => &[Choice::Cpal],
        Choice::Null => &[Choice::Null],
    };
    let mut last = None;
    for &c in order {
        let r = match c {
            Choice::PipeWire => pipewire(o, factory),
            Choice::Cpal => cpal(o, factory),
            _ => null(factory),
        };
        match r {
            Ok(run) => return Ok(run),
            Err(e) => {
                tracing::warn!("audio: {c:?} not available ({e:#}) – trying the next backend");
                last = Some(e);
            }
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("no audio backend")))
}

const NULL_RATE: u32 = 48_000;
const NULL_BLOCK: usize = 512;

/// no device: the engine runs on a timer thread so the clock, the music and the FFT still move
fn null(factory: &mut Factory) -> Result<Running> {
    let mut direct = Direct::new(factory(NULL_RATE as f32, 2)?);
    // a timer's sleep is no device deadline: only long renders count
    direct.mon.check_late = false;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new().name("audio-null".into()).spawn(move || {
        let mut buf = vec![0.0f32; NULL_BLOCK * 2];
        let period = Duration::from_secs_f64(NULL_BLOCK as f64 / NULL_RATE as f64);
        let mut next = std::time::Instant::now();
        // block k is "heard" on its schedule (no device, no latency)
        let (t0, mut k) = (now_ns(), 0u64);
        while !flag.load(Ordering::Relaxed) {
            let heard = t0 + k * NULL_BLOCK as u64 * 1_000_000_000 / NULL_RATE as u64;
            k += 1;
            direct.play(&mut buf, 2, heard, |x| x);
            next += period;
            if let Some(wait) = next.checked_duration_since(std::time::Instant::now()) {
                std::thread::sleep(wait);
            }
        }
    })?;
    struct Stop(Arc<AtomicBool>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    Ok(Running { name: "null", rate: NULL_RATE, closing: None, _keep: Box::new(Stop(stop)) })
}

/// The buffer a device gets for `want` frames (#208): `want` within the device's supported range, else the nearest end
/// (with the reason for the log); an unknown range – asked as is (a refused build falls back to the default).
pub fn pick_buffer(want: u32, range: Option<(u32, u32)>) -> (u32, Option<String>) {
    match range {
        Some((min, max)) if min <= max && (want < min || want > max) => {
            let got = want.clamp(min, max);
            (got, Some(format!("the device takes {min}–{max} frames: {want} → {got}")))
        }
        _ => (want, None),
    }
}

/// the cpal host `h` names
fn cpal_host(h: Host) -> Result<cpal::Host> {
    match h {
        Host::Default => Ok(cpal::default_host()),
        #[cfg(windows)]
        Host::Wasapi => Ok(cpal::host_from_id(cpal::HostId::Wasapi)?),
        #[cfg(not(windows))]
        Host::Wasapi => bail!("WASAPI is Windows only"),
        #[cfg(all(windows, feature = "asio"))]
        Host::Asio => Ok(cpal::host_from_id(cpal::HostId::Asio)?),
        #[cfg(not(all(windows, feature = "asio")))]
        Host::Asio => bail!("this build has no ASIO (Windows, `cargo build --features asio` with the ASIO SDK – deck/native/README.md \"Audio\")"),
    }
}

/// the output device: the first whose name contains `name` (any case), else the host's default
fn cpal_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let Some(want) = name.map(str::to_lowercase) else {
        return host.default_output_device().ok_or_else(|| anyhow::anyhow!("no default output device on {}", host.id().name()));
    };
    let mut names = Vec::new();
    for d in host.output_devices()? {
        let n = d.name().unwrap_or_default();
        if n.to_lowercase().contains(&want) {
            return Ok(d);
        }
        names.push(n);
    }
    bail!("no output device '{want}' on {} (there are: {})", host.id().name(), names.join(", "))
}

/// `--audio-list`: every host this build has, its output devices with their default config and buffer range
pub fn list() -> String {
    use cpal::traits::{DeviceTrait, HostTrait};
    let mut out = String::new();
    for id in cpal::available_hosts() {
        out += &format!("{}\n", id.name());
        let Ok(host) = cpal::host_from_id(id) else { continue };
        let default = host.default_output_device().and_then(|d| d.name().ok());
        for d in host.output_devices().into_iter().flatten() {
            let name = d.name().unwrap_or_default();
            let cfg = match d.default_output_config() {
                Ok(c) => {
                    let buf = match c.buffer_size() {
                        cpal::SupportedBufferSize::Range { min, max } => format!("buffer {min}–{max}"),
                        cpal::SupportedBufferSize::Unknown => "buffer ?".into(),
                    };
                    format!("{} Hz, {} ch, {:?}, {buf}", c.sample_rate().0, c.channels(), c.sample_format())
                }
                Err(e) => format!("({e})"),
            };
            let mark = if default.as_deref() == Some(name.as_str()) { "*" } else { " " };
            out += &format!(" {mark} {name}: {cfg}\n");
        }
    }
    out
}

/// One cpal callback's player: the heard stamp for the host, then the feed.
struct CpalPlayer {
    feed: Feed,
    channels: usize,
    rate: u32,
    /// WASAPI shared: cpal 0.15's playback instant there is callback + the frames asked for – but the new frames play
    /// after what is still queued (the buffer minus what this callback fills; #208: the stamps jittered by a period)
    wasapi: bool,
    /// raise the callback's thread to audio priority on its first call (priority.rs) – also when it only copies from
    /// the ring: measured 2026-10-31 under a full CPU, cpal's WASAPI thread at its own TIME_CRITICAL came 32–38 ms late
    /// three times in 8 min (the whole 22 ms buffer drained), raised to MMCSS never. ASIO's thread is the driver's – left
    /// alone.
    raise: bool,
}

impl CpalPlayer {
    fn fill<T: Copy>(&mut self, out: &mut [T], info: &cpal::OutputCallbackInfo, conv: impl Fn(f32) -> T) {
        if std::mem::take(&mut self.raise) {
            // once, in the settling half second (the log line allocates – never again in this thread)
            match super::priority::raise() {
                Ok(how) => tracing::info!("audio: the device callback runs at {how}"),
                Err(e) => tracing::warn!("audio: the device callback at its own priority ({e})"),
            }
        }
        let frames = out.len() / self.channels.max(1);
        let g = self.feed.glitches();
        let latency = if self.wasapi {
            let queued = (g.device_frames() as usize).saturating_sub(frames);
            (queued as f64 * 1e9 / self.rate as f64) as u64
        } else {
            // the block is heard at its playback instant: now + (playback − callback), the output latency cpal reports
            let ts = info.timestamp();
            ts.playback.duration_since(&ts.callback).map(|d| d.as_nanos() as u64).unwrap_or(0)
        };
        g.set_latency_us((latency / 1000) as u32);
        self.feed.play(out, self.channels, now_ns() + latency, conv);
    }
}

fn cpal(o: &Options, factory: &mut Factory) -> Result<Running> {
    use cpal::traits::{DeviceTrait, StreamTrait};
    use cpal::SampleFormat;
    let host = cpal_host(o.host)?;
    let device = cpal_device(&host, o.device.as_deref())?;
    let cfg = device.default_output_config()?;
    let (rate, channels) = (cfg.sample_rate().0, cfg.channels() as usize);
    let hostname = host.id().name();
    let wasapi = hostname == "WASAPI";
    let mut sc: cpal::StreamConfig = cfg.config();
    if let Some(want) = o.buffer {
        let range = match cfg.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => Some((*min, *max)),
            cpal::SupportedBufferSize::Unknown => None,
        };
        let (frames, note) = pick_buffer(want, range);
        if let Some(n) = note {
            tracing::warn!("audio: {n}");
        }
        sc.buffer_size = cpal::BufferSize::Fixed(frames);
    }
    let asio = hostname == "ASIO";
    let player = |factory: &mut Factory| -> Result<CpalPlayer> {
        let mut feed = Feed::new(factory(rate as f32, channels)?, o.thread, o.buffer)?;
        if wasapi {
            feed = feed.buffered();
        }
        Ok(CpalPlayer { feed, channels, rate, wasapi, raise: !asio })
    };
    let build = |sc: &cpal::StreamConfig, mut p: CpalPlayer| -> Result<cpal::Stream, cpal::BuildStreamError> {
        let err = |e| tracing::error!("audio stream: {e}");
        match cfg.sample_format() {
            SampleFormat::F32 => device.build_output_stream(sc, move |d: &mut [f32], i| p.fill(d, i, |x| x), err, None),
            SampleFormat::I16 => device.build_output_stream(sc, move |d: &mut [i16], i| p.fill(d, i, |x| (x * i16::MAX as f32) as i16), err, None),
            // ASIO interfaces (the RME) talk 32-bit integers
            SampleFormat::I32 => device.build_output_stream(sc, move |d: &mut [i32], i| p.fill(d, i, |x| (x as f64 * i32::MAX as f64) as i32), err, None),
            _ => Err(cpal::BuildStreamError::StreamConfigNotSupported),
        }
    };
    if !matches!(cfg.sample_format(), SampleFormat::F32 | SampleFormat::I16 | SampleFormat::I32) {
        bail!("output sample format {:?} (f32, i16 and i32 are supported)", cfg.sample_format());
    }
    let stream = match build(&sc, player(factory)?) {
        Ok(s) => s,
        Err(e) if o.buffer.is_some() => {
            // the host refused the fixed buffer: its default (the player went into the failed build – a fresh one)
            tracing::warn!("audio: {hostname} refused a {:?} buffer ({e}) – its default instead", sc.buffer_size);
            sc.buffer_size = cpal::BufferSize::Default;
            build(&sc, player(factory)?)?
        }
        Err(e) => return Err(e.into()),
    };
    stream.play()?;
    tracing::info!(
        "audio: cpal {hostname} on '{}', {rate} Hz, {channels} ch, {:?}, buffer {:?}, {}",
        device.name().unwrap_or_default(),
        cfg.sample_format(),
        sc.buffer_size,
        if o.thread { "a render thread + ring" } else { "rendered in the callback" }
    );
    let name = match hostname {
        "WASAPI" => "wasapi",
        "ASIO" => "asio",
        "ALSA" => "alsa",
        "CoreAudio" => "coreaudio",
        "JACK" => "jack",
        _ => "cpal",
    };
    Ok(Running { name, rate, closing: None, _keep: Box::new(stream) })
}


#[cfg(not(all(target_os = "linux", feature = "pipewire")))]
fn pipewire(_: &Options, _: &mut Factory) -> Result<Running> {
    bail!("built without the pipewire feature")
}

/// the length of a stream's fade in and out (#250: a scene switch no longer leaves the old scene's sound playing)
#[cfg(all(target_os = "linux", feature = "pipewire"))]
const FADE_SECS: f32 = 0.25;

/// A gain ramp on the device's cycles: up from silence while open, down to silence once closing.
pub struct Fade {
    gain: f32,
    step: f32,
}

impl Fade {
    pub fn new(rate: f32, secs: f32) -> Fade {
        Fade { gain: 0.0, step: 1.0 / (rate * secs).max(1.0) }
    }

    /// Scales `out` (interleaved, `channels` per frame) by the ramp; true once closing and fully silent.
    pub fn apply(&mut self, out: &mut [f32], channels: usize, closing: bool) -> bool {
        for frame in out.chunks_exact_mut(channels.max(1)) {
            self.gain = if closing { (self.gain - self.step).max(0.0) } else { (self.gain + self.step).min(1.0) };
            for x in frame {
                *x *= self.gain;
            }
        }
        closing && self.gain == 0.0
    }
}

/// Dropping the PipeWire backend fades the stream out (≈ 0.25 s), ends its loop and joins the thread – bounded: a loop
/// that doesn't answer is logged and left behind, the frame thread waits ~0.5 s at most.
#[cfg(all(target_os = "linux", feature = "pipewire"))]
struct PipewireStop {
    closing: Arc<AtomicBool>,
    silent: Arc<AtomicBool>,
    quit: pipewire::channel::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(all(target_os = "linux", feature = "pipewire"))]
impl Drop for PipewireStop {
    fn drop(&mut self) {
        let wait = |ok: &dyn Fn() -> bool, ms: u64| {
            let end = std::time::Instant::now() + Duration::from_millis(ms);
            while !ok() && std::time::Instant::now() < end {
                std::thread::sleep(Duration::from_millis(5));
            }
            ok()
        };
        self.closing.store(true, Ordering::Relaxed);
        if !wait(&|| self.silent.load(Ordering::Relaxed), 400) {
            tracing::warn!("audio: the PipeWire stream did not fade out in time");
        }
        let _ = self.quit.send(());
        if let Some(t) = self.thread.take() {
            if wait(&|| t.is_finished(), 150) {
                let _ = t.join();
            } else {
                tracing::warn!("audio: the PipeWire loop did not end – left behind");
            }
        }
    }
}

/// PipeWire: an output stream at 48 kHz / f32 with the sink's channels (#318: the count and positions the sink reports – the
/// RME's Pro Audio sink has 20, AUX0…AUX19 – so PipeWire maps channel n to channel n, no remix; stereo when the sink says
/// nothing) and a latency request of the buffer setting (default 128 frames, the deck's quantum, rootfs/etc/pipewire),
/// processed on PipeWire's realtime data thread (RT_PROCESS). The engine is built for that channel count once the loop's
/// thread has asked the sink.
#[cfg(all(target_os = "linux", feature = "pipewire"))]
fn pipewire(o: &Options, factory: &mut Factory) -> Result<Running> {
    use pipewire as pw;
    use pw::spa;
    const RATE: u32 = 48_000;
    let quantum = o.buffer.unwrap_or(128);
    let latency = format!("{quantum}/{RATE}");
    let shown = latency.clone();
    // the loop's thread asks the sink for its channels, this thread builds the engine for them and hands the feed over
    let (sink_tx, sink_rx) = std::sync::mpsc::channel::<usize>();
    let (feed_tx, feed_rx) = std::sync::mpsc::channel::<Feed>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<pw::channel::Sender<()>, String>>();
    // the stream fades in on its first cycles and out when the Running is dropped (#250): `closing` asks, `silent` answers
    let closing = Arc::new(AtomicBool::new(false));
    let silent = Arc::new(AtomicBool::new(false));
    let (closing_cb, silent_cb) = (closing.clone(), silent.clone());
    let mut fade = Fade::new(RATE as f32, FADE_SECS);
    let want = o.device.clone();
    let thread = std::thread::Builder::new().name("pipewire".into()).spawn(move || {
        let run = || -> Result<(), pw::Error> {
            pw::init();
            let mainloop = pw::main_loop::MainLoopRc::new(None)?;
            let context = pw::context::ContextRc::new(&mainloop, None)?;
            let core = context.connect_rc(None)?;
            // #285: a named device (`--audio-device`, deck.toml `audio_device`) picks the sink; none = the default sink.
            // #318: the sink's channel positions size the stream and the engine
            let sink = find_sink(&mainloop, &core, want.as_deref())?;
            let (target, position) = (sink.target, sink.position);
            let chans = position.len();
            sink_tx.send(chans).map_err(|_| pw::Error::CreationFailed)?;
            // the ring + render thread (#208) also here when on: the X1's quantum of 128 leaves the callback 2.7 ms
            let mut direct = feed_rx.recv_timeout(Duration::from_secs(10)).map_err(|_| pw::Error::CreationFailed)?;
            let channels = chans.to_string();
            let mut props = pw::properties::properties! {
                *pw::keys::MEDIA_TYPE => "Audio",
                *pw::keys::MEDIA_ROLE => "Music",
                *pw::keys::MEDIA_CATEGORY => "Playback",
                *pw::keys::NODE_NAME => "deck-native",
                *pw::keys::APP_NAME => "SCUTOONS deck stage",
                *pw::keys::NODE_LATENCY => latency.as_str(),
                *pw::keys::AUDIO_CHANNELS => channels.as_str(),
                // the positions are the sink's own: channel n is channel n, never up- or down-mixed (the engine folds)
                *pw::keys::STREAM_DONT_REMIX => "true",
            };
            if let Some(t) = &target {
                props.insert(*pw::keys::TARGET_OBJECT, t.as_str());
            }
            let stream = pw::stream::StreamBox::new(&core, "deck-native", props)?;
            let _listener = stream
                .add_local_listener_with_user_data(())
                .process(move |stream, _| {
                    let Some(mut buffer) = stream.dequeue_buffer() else { return };
                    // the frames this cycle wants (#208: not the whole mapped buffer – rendering all of it queued
                    // latency); 0 = unknown, then the buffer
                    let requested = buffer.requested() as usize;
                    let datas = buffer.datas_mut();
                    let Some(data) = datas.first_mut() else { return };
                    // pw_time (RT-safe): `now` = this cycle on CLOCK_MONOTONIC, `delay` = until the device plays the
                    // next sample (ticks of `rate`), `buffered` = frames in the resampler; no timing yet → the quantum
                    let heard = match stream.time() {
                        Ok(t) if t.rate().denom > 0 && t.now() > 0 => {
                            let r = t.rate();
                            let delay = t.delay().max(0) as f64 * r.num as f64 / r.denom as f64;
                            let buffered = t.buffered() as f64 / RATE as f64;
                            t.now() as u64 + ((delay + buffered) * 1e9) as u64
                        }
                        _ => now_ns() + quantum as u64 * 1_000_000_000 / RATE as u64,
                    };
                    let stride = std::mem::size_of::<f32>() * chans;
                    let frames = match data.data() {
                        Some(bytes) => match bytemuck::try_cast_slice_mut::<u8, f32>(bytes) {
                            Ok(samples) => {
                                let all = samples.len() / chans;
                                let n = if requested > 0 { requested.min(all) } else { all };
                                direct.play(&mut samples[..n * chans], chans, heard, |x| x);
                                if fade.apply(&mut samples[..n * chans], chans, closing_cb.load(Ordering::Relaxed)) {
                                    silent_cb.store(true, Ordering::Relaxed);
                                }
                                n
                            }
                            Err(_) => 0,
                        },
                        None => 0,
                    };
                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = stride as _;
                    *chunk.size_mut() = (stride * frames) as _;
                })
                .register()?;
            let mut info = spa::param::audio::AudioInfoRaw::new();
            info.set_format(spa::param::audio::AudioFormat::F32LE);
            info.set_rate(RATE);
            info.set_channels(chans as u32);
            let mut slots = [0; spa::param::audio::MAX_CHANNELS];
            slots[..chans].copy_from_slice(&position);
            info.set_position(slots);
            let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
                std::io::Cursor::new(Vec::new()),
                &spa::pod::Value::Object(spa::pod::Object {
                    type_: spa::sys::SPA_TYPE_OBJECT_Format,
                    id: spa::sys::SPA_PARAM_EnumFormat,
                    properties: info.into(),
                }),
            )
            .map_err(|_| pw::Error::CreationFailed)?
            .0
            .into_inner();
            let mut params = [spa::pod::Pod::from_bytes(&values).ok_or(pw::Error::CreationFailed)?];
            stream.connect(
                spa::utils::Direction::Output,
                None,
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS,
                &mut params,
            )?;
            // the drop's quit: a message into the loop's thread ends `run`
            let (quit_tx, quit_rx) = pw::channel::channel::<()>();
            let weak = mainloop.downgrade();
            let _quit = quit_rx.attach(mainloop.loop_(), move |_| {
                if let Some(m) = weak.upgrade() {
                    m.quit();
                }
            });
            let _ = ready_tx.send(Ok(quit_tx));
            mainloop.run();
            Ok(())
        };
        if let Err(e) = run() {
            let _ = ready_tx.send(Err(e.to_string()));
        }
    })?;
    let chans = match sink_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(chans) => chans,
        Err(_) => match ready_rx.try_recv() {
            Ok(Err(e)) => bail!("{e}"),
            _ => bail!("no answer from PipeWire within 3 s"),
        },
    };
    // the engine for the sink's channels (an error here drops `feed_tx`: the loop's thread gives up too)
    feed_tx.send(Feed::new(factory(RATE as f32, chans)?, o.thread, Some(quantum))?).map_err(|_| anyhow::anyhow!("the PipeWire thread ended"))?;
    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(quit)) => {
            tracing::info!("audio: PipeWire stream 'deck-native', {RATE} Hz, {chans} channels, latency {shown}");
            let flag = closing.clone();
            Ok(Running { name: "pipewire", rate: RATE, closing: Some(flag), _keep: Box::new(PipewireStop { closing, silent, quit, thread: Some(thread) }) })
        }
        Ok(Err(e)) => bail!("{e}"),
        Err(_) => bail!("no answer from PipeWire within 3 s"),
    }
}

/// The sink a stream plays on: its node name when a device was named (`target.object`; None = the default sink) and its
/// channel positions (SPA ids, one per channel, at most the engine's 32).
#[cfg(all(target_os = "linux", feature = "pipewire"))]
struct Sink {
    target: Option<String>,
    position: Vec<u32>,
}

/// #285: the PipeWire sink a device name picks – the first `Audio/Sink` node (not an internal one) whose `node.name`
/// or `node.description` holds `want` (any case); no name = the default sink (the `default` metadata's
/// `default.audio.sink`). On the deck the RME in the Pro Audio profile is `alsa_output.usb-RME_Fireface_UCX_II…pro-output-0`,
/// so `audio_device = "Fireface"` finds it. No match: a WARN with the sinks there are, and the default sink plays.
/// #318: every sink is bound for its info – `audio.position`, else `audio.channels` as AUX – in two round trips on the loop
/// before the stream opens; a sink that says nothing (or none known) is stereo, FL FR.
#[cfg(all(target_os = "linux", feature = "pipewire"))]
fn find_sink(mainloop: &pipewire::main_loop::MainLoopRc, core: &pipewire::core::CoreRc, want: Option<&str>) -> Result<Sink, pipewire::Error> {
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    /// a sink as the registry and its info show it
    #[derive(Default)]
    struct Seen {
        name: String,
        description: String,
        position: Option<String>,
        channels: Option<usize>,
    }
    let registry = core.get_registry_rc()?;
    let sinks: Rc<RefCell<Vec<Seen>>> = Rc::new(RefCell::new(Vec::new()));
    let default: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    // the bound proxies and their listeners live until the second round trip is done
    let keep: Rc<RefCell<Vec<Box<dyn std::any::Any>>>> = Rc::new(RefCell::new(Vec::new()));
    let (seen, chosen, kept, weak_registry) = (sinks.clone(), default.clone(), keep.clone(), registry.downgrade());
    let _globals = registry
        .add_listener_local()
        .global(move |g| {
            let Some(p) = g.props.as_ref() else { return };
            let Some(registry) = weak_registry.upgrade() else { return };
            if g.type_ == pw::types::ObjectType::Metadata && p.get("metadata.name") == Some("default") {
                let Ok(meta) = registry.bind::<pw::metadata::Metadata, _>(g) else { return };
                let chosen = chosen.clone();
                let listener = meta
                    .add_listener_local()
                    .property(move |_, key, _, value| {
                        if key == Some("default.audio.sink") {
                            *chosen.borrow_mut() = value.and_then(default_sink_name);
                        }
                        0
                    })
                    .register();
                kept.borrow_mut().push(Box::new((meta, listener)));
                return;
            }
            if g.type_ != pw::types::ObjectType::Node || p.get("media.class") != Some("Audio/Sink") {
                return;
            }
            let at = seen.borrow().len();
            let (name, description) = (p.get("node.name").unwrap_or_default().to_string(), p.get("node.description").unwrap_or_default().to_string());
            seen.borrow_mut().push(Seen { name, description, ..Seen::default() });
            let Ok(node) = registry.bind::<pw::node::Node, _>(g) else { return };
            let seen = seen.clone();
            let listener = node
                .add_listener_local()
                .info(move |info| {
                    let Some(props) = info.props() else { return };
                    let mut list = seen.borrow_mut();
                    let s = &mut list[at];
                    if let Some(p) = props.get("audio.position") {
                        s.position = Some(p.to_string());
                    }
                    if let Some(n) = props.get("audio.channels").and_then(|c| c.trim().parse().ok()) {
                        s.channels = Some(n);
                    }
                })
                .register();
            kept.borrow_mut().push(Box::new((node, listener)));
        })
        .register();
    // the globals arrive before the first sync's done, the bound objects' info and properties before the second's
    roundtrip(mainloop, core)?;
    roundtrip(mainloop, core)?;
    let sinks = sinks.borrow();
    let named = want.and_then(|want| {
        let w = want.to_lowercase();
        let hit = sinks.iter().find(|s| s.name.to_lowercase().contains(&w) || s.description.to_lowercase().contains(&w));
        match hit {
            Some(s) => tracing::info!("audio: PipeWire sink '{}' ({}) for '{want}'", s.description, s.name),
            None => {
                let names: Vec<&str> = sinks.iter().map(|s| s.description.as_str()).collect();
                tracing::warn!("audio: no PipeWire sink matches '{want}' (there are: {}) – the default sink plays", names.join(", "));
            }
        }
        hit
    });
    let default = default.borrow().clone();
    let sink = named.or_else(|| sinks.iter().find(|s| Some(&s.name) == default.as_ref()));
    let names: Vec<String> = match sink.map(|s| (&s.position, s.channels)) {
        Some((Some(p), _)) => position_names(p),
        Some((None, Some(n))) => (0..n).map(|i| format!("AUX{i}")).collect(),
        _ => Vec::new(),
    };
    let mut position: Vec<u32> = names.iter().take(deck_engine::config::MAX_DEVICE_CHANNELS).enumerate().map(|(i, n)| spa_channel(n, i)).collect();
    if position.is_empty() {
        position = vec![pw::spa::sys::SPA_AUDIO_CHANNEL_FL, pw::spa::sys::SPA_AUDIO_CHANNEL_FR];
    }
    match sink {
        Some(s) => tracing::info!("audio: PipeWire sink '{}': {} channels ({})", s.description, position.len(), names.join(" ")),
        None => tracing::info!("audio: the default PipeWire sink is not known – stereo"),
    }
    Ok(Sink { target: named.map(|s| s.name.clone()), position })
}

/// run the loop until the server has answered everything sent so far
#[cfg(all(target_os = "linux", feature = "pipewire"))]
fn roundtrip(mainloop: &pipewire::main_loop::MainLoopRc, core: &pipewire::core::CoreRc) -> Result<(), pipewire::Error> {
    let pending = core.sync(0)?;
    let weak = mainloop.downgrade();
    let _done = core
        .add_listener_local()
        .done(move |id, seq| {
            if id == pipewire::core::PW_ID_CORE && seq == pending {
                if let Some(m) = weak.upgrade() {
                    m.quit();
                }
            }
        })
        .register();
    mainloop.run();
    Ok(())
}

/// the SPA id of a channel position name (`FL`, `AUX3` …); a name it does not know is AUX by its place
#[cfg(all(target_os = "linux", feature = "pipewire"))]
fn spa_channel(name: &str, place: usize) -> u32 {
    use pipewire::spa::sys as s;
    if let Some(n) = name.strip_prefix("AUX").and_then(|n| n.parse::<u32>().ok()) {
        return s::SPA_AUDIO_CHANNEL_AUX0 + n;
    }
    match name {
        "MONO" => s::SPA_AUDIO_CHANNEL_MONO,
        "FL" => s::SPA_AUDIO_CHANNEL_FL,
        "FR" => s::SPA_AUDIO_CHANNEL_FR,
        "FC" => s::SPA_AUDIO_CHANNEL_FC,
        "LFE" => s::SPA_AUDIO_CHANNEL_LFE,
        "SL" => s::SPA_AUDIO_CHANNEL_SL,
        "SR" => s::SPA_AUDIO_CHANNEL_SR,
        "FLC" => s::SPA_AUDIO_CHANNEL_FLC,
        "FRC" => s::SPA_AUDIO_CHANNEL_FRC,
        "RC" => s::SPA_AUDIO_CHANNEL_RC,
        "RL" => s::SPA_AUDIO_CHANNEL_RL,
        "RR" => s::SPA_AUDIO_CHANNEL_RR,
        "TC" => s::SPA_AUDIO_CHANNEL_TC,
        "TFL" => s::SPA_AUDIO_CHANNEL_TFL,
        "TFC" => s::SPA_AUDIO_CHANNEL_TFC,
        "TFR" => s::SPA_AUDIO_CHANNEL_TFR,
        "TRL" => s::SPA_AUDIO_CHANNEL_TRL,
        "TRC" => s::SPA_AUDIO_CHANNEL_TRC,
        "TRR" => s::SPA_AUDIO_CHANNEL_TRR,
        _ => s::SPA_AUDIO_CHANNEL_AUX0 + place as u32,
    }
}

/// a node's `audio.position` (`FL,FR` · `[ FL FR ]` · `AUX0,AUX1,…`) as its names
#[cfg_attr(not(all(target_os = "linux", feature = "pipewire")), allow(dead_code))]
fn position_names(s: &str) -> Vec<String> {
    s.split(|c: char| c == ',' || c == '[' || c == ']' || c.is_whitespace()).filter(|t| !t.is_empty()).map(str::to_string).collect()
}

/// the sink name in the `default` metadata's `default.audio.sink` value (`{ "name": "alsa_output…" }`)
#[cfg_attr(not(all(target_os = "linux", feature = "pipewire")), allow(dead_code))]
fn default_sink_name(json: &str) -> Option<String> {
    let rest = &json[json.find("\"name\"")? + 6..];
    let rest = &rest[rest.find('"')? + 1..];
    Some(rest[..rest.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #208: a buffer outside the device's range is clamped to its nearest end, with a reason for the log
    /// #318: what PipeWire says about a sink – its positions in either spelling, the default sink in the metadata's JSON
    #[test]
    fn a_sinks_channels_and_the_default_sink_are_read() {
        assert_eq!(position_names("FL,FR"), ["FL", "FR"]);
        assert_eq!(position_names("[ FL FR FC LFE RL RR ]").len(), 6);
        assert_eq!(position_names("AUX0,AUX1,AUX2")[2], "AUX2");
        assert!(position_names("").is_empty());
        assert_eq!(default_sink_name(r#"{ "name": "alsa_output.usb-RME_Fireface_UCX_II-00.pro-output-0" }"#).as_deref(), Some("alsa_output.usb-RME_Fireface_UCX_II-00.pro-output-0"));
        assert_eq!(default_sink_name(r#"{"name":"x"}"#).as_deref(), Some("x"));
        assert_eq!(default_sink_name("{}"), None);
    }

    #[test]
    fn a_fade_ramps_up_then_down_to_silence() {
        let mut f = Fade::new(1000.0, 0.1);
        let mut a = [1.0f32; 400];
        assert!(!f.apply(&mut a, 2, false));
        assert!(a[0] < 0.02 && a[1] < 0.02, "starts from silence");
        assert!((a[399] - 1.0).abs() < 1e-6, "full after the ramp: {}", a[399]);
        let mut b = [1.0f32; 400];
        assert!(f.apply(&mut b, 2, true));
        assert!(b[0] > 0.9 && b[399] == 0.0, "down to silence");
        // closing from the middle of the ramp-up never goes negative
        let mut g = Fade::new(1000.0, 0.1);
        g.apply(&mut [1.0f32; 40], 2, false);
        let mut c = [1.0f32; 400];
        assert!(g.apply(&mut c, 2, true) && c.iter().all(|x| *x >= 0.0));
    }

    #[test]
    fn the_buffer_is_clamped_to_the_supported_range() {
        assert_eq!(pick_buffer(256, Some((64, 2048))), (256, None));
        let (got, note) = pick_buffer(32, Some((64, 2048)));
        assert_eq!(got, 64);
        assert!(note.unwrap().contains("64–2048"));
        assert_eq!(pick_buffer(4096, Some((64, 2048))).0, 2048);
        assert_eq!(pick_buffer(128, None), (128, None), "an unknown range: asked as is");
        assert_eq!(pick_buffer(128, Some((0, u32::MAX))), (128, None), "WASAPI shared reports everything");
    }

    #[test]
    fn deck_toml_names_the_machines_audio() {
        let d = DeckAudio::parse("kiosk = \"stage\"\nrelay = \"\"\naudio_buffer = 256\naudio_host = \"asio\"\naudio_device = \"Fireface\"\n");
        assert_eq!(d, DeckAudio { audio_buffer: Some(256), audio_host: Some("asio".into()), audio_device: Some("Fireface".into()), audio_thread: None });
        assert_eq!(DeckAudio::parse("audio_thread = false\n").audio_thread, Some(false));
        // the file a deck starts with: the device's default buffer, the render thread on
        let shipped = DeckAudio::parse(include_str!("../../../os/rootfs/usr/share/deck/deck.default.toml"));
        assert_eq!((shipped.audio_buffer, shipped.audio_thread), (None, Some(true)));
        let empty = DeckAudio::parse("audio_host = \"\"\naudio_device = \" \"\naudio_buffer = 0\n");
        assert_eq!(empty, DeckAudio::default(), "empty = unset");
        assert_eq!(DeckAudio::parse("this is = not toml ["), DeckAudio::default());
    }

    #[test]
    fn hosts_parse() {
        assert_eq!(Host::parse("ASIO"), Some(Host::Asio));
        assert_eq!(Host::parse("wasapi"), Some(Host::Wasapi));
        assert_eq!(Host::parse("default"), Some(Host::Default));
        assert_eq!(Host::parse("jack"), None);
    }
}
