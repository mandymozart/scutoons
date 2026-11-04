//! The audio engine (CYBERDECKS.md §8), moved out of the native deck into the universal core (research/ENGINE-REFACTOR-PLAN.md
//! §2.1, #317). Run by a device callback, a render thread or a worker in blocks of at most MAX_BLOCK frames, on the batch
//! model (#183): at build the audio nodes are split into dependency levels (by their audio links); inside a level the
//! sine oscillators and the FM voices become one batch each (voices.rs: struct of arrays, the voice loop vectorised,
//! AVX2 where the CPU has it), every other node stays a block of its own (blocks.rs – the patcher model, for the
//! one-off nodes). Every output port has one buffer in a pool ordered like the steps, so a step reads only buffers
//! before its own. Everything is allocated at build time – a render only reads atomics (Param), pops lock-free
//! rings (rtrb) and writes preallocated buffers (tests/no_alloc.rs proves it).
//! The frame counter and the per-block stamps are the deck's time base (§8 "Two clocks"): the platform calls `mark` with
//! the moment the block it is about to render will be heard. The tempo map (deck-core's `tempo`, #181) reaches the click
//! and the synced sequencer through a seqlock, read once per block.
//! No platform in here: no device, no thread, no file, no clock. What a platform supplies is passed in – the stamps, the
//! decoded tracks of `chapter_music` (`MusicFeed`), the module runtime (`Seams`), the scheduling of the events' sequencer.
mod blocks;
mod click;
pub mod config;
pub mod control;
#[cfg(feature = "events")]
pub mod dispatch;
#[cfg(all(feature = "events", feature = "modules"))]
mod evhost;
pub mod fft;
pub mod layout;
#[cfg(feature = "modules")]
mod module;
pub mod pan;
mod route;
#[cfg(feature = "events")]
pub mod sequencer;
mod tempo;
mod voices;
#[cfg(feature = "events")]
mod walk;

pub use config::{DeckConfig, Fold, OutletMap, PassThrough, StageEnvironment};
pub use tempo::tempo_config;

use anyhow::Result;
use deck_core::batch::levels;
use deck_core::limiter::{self, LinkedLimiter};
use deck_core::tempo::{SeqLock, Shared};
use deck_space::graph::{Domain, Graph, Kind, PortType, MAX_CHANNELS};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use voices::{VoiceKind, Voices};

#[cfg(feature = "modules")]
pub use module::{ModuleInstance, ModuleSource, Opened, Restart, Restarts};

/// A block stamp: [frames rendered before the block, the monotonic moment (ns) its first frame is heard]
pub type Stamp = SeqLock<2>;

pub use blocks::default_for;

/// The engine's quantum (plan 3.3, the browser's AudioWorklet quantum): it renders whole blocks of this many frames, one
/// control read and one gain ramp per block, whatever size the platform asks for – the output is the same for a device
/// buffer of 64, 480 or 1056 frames. `render` keeps the rest of a quantum for the next call.
pub const QUANTUM: usize = 128;
/// the largest block a `Block` may be asked for (the tests drive blocks directly with more than a quantum)
pub const MAX_BLOCK: usize = 1024;
/// the most input ports any audio kind has (the mixer: 4 audio + 4 gains)
const MAX_PORTS: usize = 8;

/// A control value shared with the audio thread: an f32 in an atomic, written by the frame thread, read per block.
#[derive(Clone, Debug)]
pub struct Param(Arc<AtomicU32>);

impl Param {
    pub fn new(v: f32) -> Param {
        Param(Arc::new(AtomicU32::new(v.to_bits())))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}

/// Every output buffer's newest block peak, per channel – built only for a scene with a `space` node (the being's audio
/// hoses, #188), so the other scenes' blocks cost nothing extra. Written by the audio thread after each block, read per frame.
pub struct Meters {
    /// the peaks of all pool channels, buffer after buffer
    levels: Box<[AtomicU32]>,
    /// per graph node: its first output buffer
    buf_of: Vec<Option<usize>>,
    /// per buffer: where its channels start in `levels`, and how many it has
    at: Vec<(usize, usize)>,
}

impl Meters {
    fn channels(&self, node: usize, port: usize) -> &[AtomicU32] {
        match self.buf_of.get(node).copied().flatten().and_then(|b| self.at.get(b + port)) {
            Some(&(at, n)) => &self.levels[at..at + n],
            None => &[],
        }
    }

    /// `node`'s output `port`: the newest block's peak over its channels (0 for a node without audio outputs)
    pub fn level(&self, node: usize, port: usize) -> f32 {
        self.channels(node, port).iter().fold(0.0, |m, l| m.max(f32::from_bits(l.load(Ordering::Relaxed))))
    }

    /// the same for one channel of the port (counted from 0)
    pub fn channel_level(&self, node: usize, port: usize, channel: usize) -> f32 {
        self.channels(node, port).get(channel).map_or(0.0, |l| f32::from_bits(l.load(Ordering::Relaxed)))
    }

    /// how many channels `node`'s output `port` carries
    pub fn width(&self, node: usize, port: usize) -> usize {
        self.channels(node, port).len()
    }
}

/// The peak of every device channel in the newest quantum, after the limiter – what the speakers get (always built; the
/// HUD, the gain-reduction meters of a venue). Written by the audio thread, read from any.
pub struct DeviceLevels(Box<[AtomicU32]>);

impl DeviceLevels {
    pub fn channels(&self) -> usize {
        self.0.len()
    }

    pub fn peak(&self, channel: usize) -> f32 {
        self.0.get(channel).map_or(0.0, |l| f32::from_bits(l.load(Ordering::Relaxed)))
    }
}

/// A decoded track at its own sample rate (the music block resamples while it plays).
pub struct Track {
    pub rate: f32,
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Track {
    pub fn seconds(&self) -> f32 {
        self.l.len() as f32 / self.rate
    }
}

/// One block buffer, planar: `ch` channels of `frames` samples each, one after the other (a block uses the first `n` of
/// every channel). A port owns one; a stereo port is two channels – the engine's pool is these (plan 2.6, #318).
pub struct Buf {
    data: Box<[f32]>,
    ch: usize,
    frames: usize,
}

impl Buf {
    /// `ch` channels of `frames` frames, silent
    pub fn sized(ch: usize, frames: usize) -> Buf {
        Buf { data: vec![0.0; ch * frames].into(), ch, frames }
    }

    pub fn channels(&self) -> usize {
        self.ch
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    /// channel `c`
    #[inline(always)]
    pub fn ch(&self, c: usize) -> &[f32] {
        &self.data[c * self.frames..(c + 1) * self.frames]
    }

    #[inline(always)]
    pub fn ch_mut(&mut self, c: usize) -> &mut [f32] {
        &mut self.data[c * self.frames..(c + 1) * self.frames]
    }

    /// the first two channels, both writable (a stereo block)
    #[inline(always)]
    pub fn lr_mut(&mut self) -> (&mut [f32], &mut [f32]) {
        let (l, r) = self.data.split_at_mut(self.frames);
        (l, &mut r[..self.frames])
    }

    /// the whole planar block and the stride between channels (the group limiter works on it)
    #[inline(always)]
    pub fn planar_mut(&mut self) -> (&mut [f32], usize) {
        (&mut self.data, self.frames)
    }

    /// frame `i` of channels 0 and 1
    #[inline(always)]
    pub fn put2(&mut self, i: usize, l: f32, r: f32) {
        self.data[i] = l;
        self.data[self.frames + i] = r;
    }

    /// every channel to silence (the first `n` frames)
    pub fn clear(&mut self, n: usize) {
        for c in 0..self.ch {
            self.ch_mut(c)[..n].fill(0.0);
        }
    }
}

/// a stereo buffer of MAX_BLOCK frames (what the tests give a block they drive by hand)
impl Default for Buf {
    fn default() -> Buf {
        Buf::sized(2, MAX_BLOCK)
    }
}

/// An audio node's DSP: reads its inputs (by input port index; None = not linked or a control port), writes its outputs.
pub trait Block: Send {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize);
}

/// How an engine is built. A deck runs `Build::detect()`; the patcher build is the reference the tests and the bench
/// compare with (every node a block, the sine oscillators and FM voices scalar – the engine before #183).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build {
    /// sine oscillators and FM voices batched per level (else every node a block)
    pub batch: bool,
    /// the batches' loops in AVX2 (only where the CPU has it)
    pub avx2: bool,
}

impl Build {
    /// The batch runtime, AVX2 where the CPU has it (never on wasm32: its simd128 comes from the compile flags).
    pub fn detect() -> Build {
        Build { batch: true, avx2: voices::avx2_available() }
    }

    pub const fn patcher() -> Build {
        Build { batch: false, avx2: false }
    }

    /// for the HUD and the log: "batch avx2" · "batch" · "patcher"
    pub fn label(self) -> &'static str {
        match (self.batch, self.avx2) {
            (true, true) => "batch avx2",
            (true, false) => "batch",
            (false, _) => "patcher",
        }
    }
}

/// Where the `walk` / `events` hosts run their generators (#236, walk.rs): on the platform's sequencer thread, ahead
/// of the render (a deck's default), or inside the audio block (the fallback – the tests' `build_with`, the browser).
/// Both play the same samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sequencing {
    InBlock,
    Thread,
}

/// What a platform supplies to a build besides the graph: services the engine asks for, none by default.
#[derive(Default, Clone)]
pub struct Seams {
    /// the device this engine plays on: its channels, how outlets map onto them, the layouts of the venue (stereo by default)
    pub deck: DeckConfig,
    /// the loaded rig modules (`module:<name>` nodes); without one such a node is a build error
    #[cfg(feature = "modules")]
    pub modules: Option<Arc<dyn ModuleSource>>,
    /// the deck's tuning: the module event host fills a note's Hz from its key and back (#342)
    #[cfg(feature = "events")]
    pub tuning: deck_events::Tuning,
}

impl Seams {
    /// the seams of a platform that only knows its device
    #[allow(clippy::needless_update)] // the module runtime is a field of a feature
    pub fn with_deck(deck: DeckConfig) -> Seams {
        Seams { deck, ..Seams::default() }
    }
}

/// One step of a block, in level order.
enum Step {
    /// one node (the patcher model): its block, the buffer each input port reads, its outputs from buffer `out` on
    Node { block: Box<dyn Block>, inputs: Vec<Option<usize>>, out: usize, outs: usize },
    /// a level's voices of one kind as one batch
    Voices(Box<Voices>),
    /// an input that sums several links or maps channels: `dst` is the sum of the taps, in the links' order (a link that
    /// is alone, unscaled, unmapped and of the port's width is read in place and has no step)
    Gather { dst: usize, taps: Vec<Tap> },
    /// the end of a `feedback` loop, last in the block: the loop's input becomes what its output plays in the next block
    Capture { src: usize, dst: usize },
}

/// One link into a gathered input: the source buffer, the send level and the channel map as 0-based (source, target) pairs
/// (empty = channel n to channel n, as far as both reach).
struct Tap {
    src: usize,
    gain: f32,
    pairs: Vec<(u8, u8)>,
}

/// One channel of an outlet's sound on one device channel: `first` = the first to write it (a copy, else added)
struct Route {
    src: usize,
    from: u8,
    to: u8,
    gain: f32,
    first: bool,
}

/// A group of device channels whose gain the limiter takes off together
struct LimitSet {
    chans: Vec<usize>,
    limiter: LinkedLimiter,
}

/// The stage's clock: frames rendered for the device (or by the null backend's timer) and the stamp of the newest
/// block – when its first frame is heard (clock.rs turns the stamps into the heard position per video frame).
#[derive(Clone)]
pub struct Clock {
    pub frames: Arc<AtomicU64>,
    pub rate: f32,
    pub stamp: Arc<Stamp>,
}

impl Clock {
    /// frames rendered so far (ahead of what is heard by the output latency + the block)
    pub fn rendered(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }
}

pub struct Engine {
    /// every output port's buffer (and the sinks' own), in step order
    bufs: Vec<Buf>,
    steps: Vec<Step>,
    /// the outlets' channels on the device's, in the outlets' order
    routes: Vec<Route>,
    /// device channels no outlet plays on (silent), and those some do (the master gain ramps them)
    unwritten: Vec<usize>,
    written: Vec<usize>,
    master: Param,
    /// the master gain the last block ended on – a change ramps over one block (no click on a mute)
    gain: f32,
    frames: Arc<AtomicU64>,
    stamp: Arc<Stamp>,
    meters: Option<Arc<Meters>>,
    /// the room between the outlets and the limiter (plan 2.6b); the identity unless one is set
    environment: Box<dyn StageEnvironment>,
    /// the output's soft cap (#188, deck-core's limiter.rs) – last before the device – one per group of device channels
    sets: Vec<LimitSet>,
    /// the newest quantum on the device channels after master, environment and limiter, and how many of its last frames
    /// the platform has not taken yet
    dev: Buf,
    device: Arc<DeviceLevels>,
    pending: usize,
    /// the deepest gain reduction (dB, f32 bits) since the frame thread last took it
    reduction: Arc<AtomicU32>,
    /// late blocks (`mark`): the count, and the last mark (frames rendered, heard at)
    underruns: Arc<AtomicU64>,
    last_mark: Option<(u64, u64)>,
    rate: f32,
    /// the control half (#319): stepped at the start of every quantum, before the audio steps read its Params
    control: control::Control,
}

/// What the frame thread keeps of a built engine.
pub struct Handles {
    /// the linked control inputs of audio nodes: (graph node, input port, param)
    pub params: Vec<(usize, usize, Param)>,
    /// the unlinked ones (their param or default) – set from outside like a MIDI CC (a 3D button, #188)
    pub knobs: Vec<(usize, usize, Param)>,
    /// the output buffers' peaks (a scene with a `space` node)
    pub meters: Option<Arc<Meters>>,
    /// chapter_music nodes: where their loader sends the decoded track
    pub music: Vec<MusicFeed>,
    /// fft nodes: their sample ring (the build hands them to the control half)
    pub(crate) taps: Vec<(usize, rtrb::Consumer<f32>)>,
    /// the control half's link for the frame thread's face (`control::Controls::new` takes it): the snapshot ring by
    /// heard frame, the command ring (#319)
    pub control: Option<control::Link>,
    pub clock: Clock,
    pub master: Param,
    /// the tempo map the frame thread publishes (deck-core's tempo controller) and the audio thread reads
    pub tempo: Arc<Shared>,
    /// how the engine was built (the HUD shows it)
    pub build: Build,
    /// the limiter's deepest gain reduction since the last `take_reduction` (#188: the HUD, the being's red face)
    pub reduction: Arc<AtomicU32>,
    /// blocks the device played late (an underrun: `Engine::mark` saw a gap) – one of the glitch causes
    pub underruns: Arc<AtomicU64>,
    /// the peaks of the device's channels (what the speakers get)
    pub device: Arc<DeviceLevels>,
    /// what the build did with the deck configuration, in words: an outlet with no device channels, a fold, channels the
    /// device lacks – for the log and the HUD
    pub notes: Vec<String>,
    /// the hosts' events (#236): names, the sequencer, the routing for the dispatcher – `events.start()` runs them
    #[cfg(feature = "events")]
    pub events: dispatch::Events,
    /// one lane per module node for a show's restarts (#342): the platform's loader thread calls `serve` on them
    #[cfg(feature = "modules")]
    pub restarts: Vec<module::Restarts>,
    /// the module nodes opened before the levels are built (taken by their block or stepper)
    #[cfg(feature = "modules")]
    pub(crate) prepared: Vec<Option<module::Prepared>>,
    /// values written outside the control step: (node, output port, its Param) – modules' control outputs (#342)
    pub(crate) feeds: Vec<(usize, usize, Param)>,
}

impl Handles {
    /// the limiter's deepest gain reduction (dB) since the last call
    pub fn take_reduction(&self) -> f32 {
        f32::from_bits(self.reduction.swap(0, Ordering::Relaxed))
    }
}

pub struct MusicFeed {
    pub node: usize,
    pub tracks: rtrb::Producer<Arc<Track>>,
    /// what the HUD shows ("loading", "playing <track>", the error)
    pub status: Arc<Mutex<String>>,
}

impl Engine {
    /// Build the graph's audio nodes for `rate` on the batch runtime (`Build::detect`), the hosts' generators on the
    /// platform's sequencer thread (`handles.events.take_sequencer()`). `volume` is the master gain on top of the
    /// audio_out node's own.
    pub fn build(graph: &Graph, rate: f32, volume: f32) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, Build::detect(), Sequencing::Thread)
    }

    /// `build` with this `build`, the hosts' generators inside the block (no thread to start – the tests)
    pub fn build_with(graph: &Graph, rate: f32, volume: f32, build: Build) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, build, Sequencing::InBlock)
    }

    pub fn build_seq(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing) -> Result<(Engine, Handles)> {
        Engine::build_seams(graph, rate, volume, build, seq, &Seams::default())
    }

    /// `build_seq` with the platform's services (`Seams`: the device's configuration, the module runtime)
    pub fn build_seams(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing, seams: &Seams) -> Result<(Engine, Handles)> {
        #[cfg(not(feature = "events"))]
        let _ = seq;
        let deck = &seams.deck;
        if !(1..=config::MAX_DEVICE_CHANNELS).contains(&deck.channels) {
            anyhow::bail!("the deck configuration '{}' has {} device channels (1..={})", deck.name, deck.channels, config::MAX_DEVICE_CHANNELS);
        }
        let device = Arc::new(DeviceLevels((0..deck.channels).map(|_| AtomicU32::new(0)).collect()));
        let mut handles = Handles {
            params: Vec::new(),
            knobs: Vec::new(),
            meters: None,
            music: Vec::new(),
            taps: Vec::new(),
            control: None,
            clock: Clock { frames: Arc::new(AtomicU64::new(0)), rate, stamp: Arc::new(Stamp::default()) },
            master: Param::new(volume),
            tempo: Arc::new(Shared::default()),
            build,
            reduction: Arc::new(AtomicU32::new(0)),
            underruns: Arc::new(AtomicU64::new(0)),
            device: device.clone(),
            notes: Vec::new(),
            #[cfg(feature = "events")]
            events: dispatch::Events::new(seq),
            #[cfg(feature = "modules")]
            restarts: Vec::new(),
            #[cfg(feature = "modules")]
            prepared: Vec::new(),
            feeds: Vec::new(),
        };
        // the scene's tempo until the frame thread takes over
        let tempo = tempo_config(graph)?;
        handles.tempo.publish(&tempo.map(rate as f64), tempo.click);
        // every module node opened first (audio and control domain), its event ports wired across the graph
        #[cfg(feature = "modules")]
        {
            let prepared = module::prepare(graph, rate, seams.modules.as_ref(), &mut handles, seams)?;
            handles.prepared = prepared;
        }
        let audio = |i: usize| graph.nodes[i].kind.domain() == Domain::Audio;
        // levels by the audio links (control links into or out of audio nodes cross threads and never order them; a link
        // into a `feedback` node is read a block late and orders nothing)
        let deps: Vec<Vec<usize>> = (0..graph.nodes.len())
            .map(|t| {
                if graph.nodes[t].kind == Kind::Feedback {
                    return Vec::new();
                }
                graph.links.iter().filter(|l| l.to.0 == t && l.ty == PortType::Audio).map(|l| l.from.0).collect()
            })
            .collect();
        let level = levels(&deps).map_err(|e| anyhow::anyhow!("the audio graph: {e}"))?;
        let depth = (0..graph.nodes.len()).filter(|&i| audio(i)).map(|i| level[i] + 1).max().unwrap_or(0);
        let mut bufs: Vec<Buf> = Vec::new();
        let mut steps = Vec::new();
        let mut tables = blocks::Tables::new(rate);
        let mut buf_of: Vec<Option<usize>> = vec![None; graph.nodes.len()];
        let mut inputs_of: Vec<Vec<Option<usize>>> = vec![Vec::new(); graph.nodes.len()];
        let mut feeds: Vec<Feed> = Vec::new();
        // every bus's solo param, before any bus is built: one soloed bus silences the out of the others
        let mut early: Vec<Option<Vec<Param>>> = vec![None; graph.nodes.len()];
        for (i, node) in graph.nodes.iter().enumerate() {
            if node.kind == Kind::Bus {
                early[i] = Some(node_params(graph, i, &mut handles));
            }
        }
        let solo = Kind::Bus.input("solo").expect("a bus has a solo");
        let solo_of: Vec<Option<Param>> = early.iter().map(|p| p.as_ref().map(|p| p[solo].clone())).collect();
        let all: Arc<[Param]> = solo_of.iter().flatten().cloned().collect();
        for l in 0..depth {
            let nodes: Vec<usize> = graph.order.iter().copied().filter(|&i| audio(i) && level[i] == l).collect();
            // a feedback loop's output is there from the start of its level; its input is wired at the end
            for &i in &nodes {
                if graph.nodes[i].kind == Kind::Feedback {
                    buf_of[i] = Some(bufs.len());
                    bufs.push(Buf::sized(route::out_widths(&graph.nodes[i], &[], deck)?[0], QUANTUM));
                }
            }
            // what each node reads (a gather buffer and its step come before any output buffer of the level)
            for &i in &nodes {
                if buf_of[i].is_none() {
                    inputs_of[i] = resolve_inputs(graph, i, deck, &buf_of, &mut bufs, &mut steps)?;
                }
            }
            // the level's batches first: one per kind, its voices' outputs side by side
            for kind in [VoiceKind::Sine, VoiceKind::Fm] {
                let voices: Vec<usize> =
                    nodes.iter().copied().filter(|&i| build.batch && VoiceKind::of(&graph.nodes[i]) == Some(kind)).collect();
                if voices.is_empty() {
                    continue;
                }
                let mut batch = Voices::new(kind, bufs.len(), rate, build.avx2);
                for &i in &voices {
                    let node = &graph.nodes[i];
                    let params = node_params(graph, i, &mut handles);
                    let p = |name: &str| params[node.kind.input(name).expect("a port of this kind")].clone();
                    let (ratio, index) = match kind {
                        VoiceKind::Fm => (p("ratio"), p("index")),
                        VoiceKind::Sine => (Param::new(1.0), Param::new(1.0)),
                    };
                    let cv = node.kind.input("cv").and_then(|k| inputs_of[i][k]);
                    batch.push([p("freq"), p("gain"), ratio, index], cv, blocks::env_coefs(rate, node));
                    buf_of[i] = Some(bufs.len());
                    bufs.push(Buf::sized(2, QUANTUM));
                }
                batch.finish();
                steps.push(Step::Voices(Box::new(batch)));
            }
            // the one-off nodes
            for &i in &nodes {
                if buf_of[i].is_some() {
                    continue; // batched above (or a feedback loop's output)
                }
                let node = &graph.nodes[i];
                let params = early[i].take().unwrap_or_else(|| node_params(graph, i, &mut handles));
                let inputs = std::mem::take(&mut inputs_of[i]);
                let in_widths: Vec<Option<usize>> = inputs.iter().map(|b| b.map(|b| bufs[b].channels())).collect();
                let mut widths = route::out_widths(node, &in_widths, deck)?;
                // the sinks have no output port but keep their block's result in one buffer – the outlets read it
                let sink = matches!(node.kind, Kind::AudioOut | Kind::Out);
                if sink {
                    widths = vec![if node.kind == Kind::AudioOut { 2 } else { in_widths[0].unwrap_or(2) }];
                }
                // control outputs keep a token buffer (the port numbers count them)
                for (w, port) in widths.iter_mut().zip(node.kind.outputs()) {
                    if port.ty != PortType::Audio {
                        *w = 2;
                    }
                }
                let solos = route::Solos { all: all.clone(), upstream: upstream_solos(graph, i, &solo_of), downstream: downstream_solos(graph, i, &solo_of) };
                let block = match route::make(node, rate, &params, &widths, deck, &solos)? {
                    Some(b) => b,
                    None => blocks::make(node, i, rate, &params, &mut handles, seams, &mut tables)?,
                };
                buf_of[i] = Some(bufs.len());
                if sink {
                    feeds.push(Feed::of(node, i, bufs.len(), widths[0]));
                }
                // a module has many control inputs (its parameters) after its audio ones: the block reads only the latter
                let mut inputs = inputs;
                if matches!(node.kind, Kind::Module(_)) {
                    let used = inputs.iter().rposition(Option::is_some).map_or(0, |k| k + 1);
                    inputs.truncate(used);
                }
                steps.push(Step::Node { block, inputs, out: bufs.len(), outs: widths.len() });
                bufs.extend(widths.iter().map(|&w| Buf::sized(w, QUANTUM)));
            }
        }
        // the loops close: each feedback node's input, wired now that its source exists, is copied into its output last. All
        // inputs are read before any output is overwritten: an input that is a feedback output itself (two in a row, a node
        // fed by its own output) is copied aside first, so every feedback node is exactly one block late
        let loops: Vec<usize> = (0..graph.nodes.len()).filter(|&i| graph.nodes[i].kind == Kind::Feedback).collect();
        let outputs: Vec<usize> = loops.iter().map(|&i| buf_of[i].expect("allocated with its level")).collect();
        let mut captures = Vec::new();
        for (&i, &dst) in loops.iter().zip(&outputs) {
            if let Some(mut src) = resolve_inputs(graph, i, deck, &buf_of, &mut bufs, &mut steps)?[0] {
                if outputs.contains(&src) {
                    steps.push(Step::Gather { dst: bufs.len(), taps: vec![Tap { src, gain: 1.0, pairs: Vec::new() }] });
                    bufs.push(Buf::sized(bufs[src].channels(), QUANTUM));
                    src = bufs.len() - 1;
                }
                captures.push(Step::Capture { src, dst });
            }
        }
        steps.extend(captures);
        if graph.first(Kind::Space).is_some() {
            let mut at = Vec::with_capacity(bufs.len());
            let mut total = 0;
            for b in &bufs {
                at.push((total, b.channels()));
                total += b.channels();
            }
            let levels = (0..total).map(|_| AtomicU32::new(0)).collect();
            handles.meters = Some(Arc::new(Meters { levels, buf_of: buf_of.clone(), at }));
        }
        feeds.sort_by_key(|f| f.node);
        let plan = plan_outlets(&feeds, deck, rate);
        handles.notes = plan.notes;
        // the control half: every control input of an audio node is a column of it (linked or not: a knob's base turns it)
        let inputs: Vec<(usize, usize, Param)> = handles.params.iter().chain(&handles.knobs).cloned().collect();
        let mut control = control::Control::new(graph, &inputs, std::mem::take(&mut handles.taps), rate);
        // the `domain: control` modules: steppers of the control half (#342)
        #[cfg(feature = "modules")]
        for i in graph.order.iter().copied().filter(|&i| matches!(graph.nodes[i].kind, Kind::Module(_)) && !audio(i)) {
            let node = &graph.nodes[i];
            let source = seams.modules.as_ref().ok_or_else(|| anyhow::anyhow!("node '{}': this engine has no module runtime", node.id))?;
            let p = handles.prepared[i].take().ok_or_else(|| anyhow::anyhow!("node '{}': its module was not opened", node.id))?;
            let beats = deck_core::tempo::BeatReader { shared: handles.tempo.clone(), frames: handles.clock.frames.clone() };
            let m = module::ControlModule::new(i, node, rate, p, beats, source, &mut handles);
            control.add_stepper(i, Box::new(m));
        }
        for (node, port, p) in std::mem::take(&mut handles.feeds) {
            control.feed(node, port, p);
        }
        let (feed, link) = control::link(control.slots(), handles.clock.frames.clone());
        control.attach(deck_core::tempo::BeatReader { shared: handles.tempo.clone(), frames: handles.clock.frames.clone() }, feed);
        handles.control = Some(link);
        let engine = Engine {
            meters: handles.meters.clone(),
            bufs,
            steps,
            routes: plan.routes,
            unwritten: plan.unwritten,
            written: plan.written,
            master: handles.master.clone(),
            gain: volume,
            frames: handles.clock.frames.clone(),
            stamp: handles.clock.stamp.clone(),
            environment: Box::new(PassThrough),
            sets: plan.sets,
            dev: Buf::sized(deck.channels, QUANTUM),
            device,
            pending: 0,
            reduction: handles.reduction.clone(),
            underruns: handles.underruns.clone(),
            last_mark: None,
            rate,
            control,
        };
        Ok((engine, handles))
    }

    /// the count `mark` raises on a gap in the heard stamps (a backend compares it around a callback)
    pub fn underruns(&self) -> Arc<AtomicU64> {
        self.underruns.clone()
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// how many device channels this engine renders (the deck configuration's)
    pub fn device_channels(&self) -> usize {
        self.dev.channels()
    }

    /// The room the show plays in (plan 2.6b): from the next quantum on it processes the outlets' sum on the device channels
    /// before the limiter. The engine starts with none (the outlets go straight on). Allocate it before, not in the callback.
    pub fn set_stage_environment(&mut self, environment: Box<dyn StageEnvironment>) {
        self.environment = environment;
    }

    /// The control half (#319): a worker that owns the engine drives it directly (`send`, `set_base`, `input_at` – no
    /// face, no ring); the tests step it.
    pub fn control(&self) -> &control::Control {
        &self.control
    }

    pub fn control_mut(&mut self) -> &mut control::Control {
        &mut self.control
    }

    /// The P5 golden diff's reference, never a deck's: the control half steps at `hz` (the native deck's video frame
    /// rate before #319) instead of once per quantum.
    pub fn control_at(&mut self, hz: f32) {
        self.control.every(hz, self.rate);
    }

    /// (steps, batches, the batches' loops: "avx2" / "baseline" / "-") – the shape the build found
    pub fn shape(&self) -> (usize, usize, &'static str) {
        let batches: Vec<&Voices> = self.steps.iter().filter_map(|s| if let Step::Voices(v) = s { Some(v.as_ref()) } else { None }).collect();
        (self.steps.len(), batches.len(), batches.first().map_or("-", |v| v.label()))
    }

    /// The block about to be rendered is heard at `heard_ns` (clock::now_ns's clock) – the backends call this right
    /// before `render`. Never allocates, never waits.
    /// The limiter's look-ahead delays the sound by `limiter::LOOK` frames: the stamp says when the block's first frame
    /// really comes out. A block heard later than the last one's end (by more than half a block + 2 ms of jitter) means
    /// the device ran dry in between – an underrun, counted for the HUD (#188).
    pub fn mark(&mut self, heard_ns: u64) {
        // the position of the next frame the platform takes: the rendered frontier less what is kept of a quantum
        let frames = self.frames.load(Ordering::Relaxed) - self.pending as u64;
        let ns = |n: u64| (n as f64 * 1e9 / self.rate as f64) as u64;
        if let Some((f0, h0)) = self.last_mark {
            let n = frames.saturating_sub(f0);
            if n > 0 && heard_ns > h0 + ns(n) + ns(n) / 2 + 2_000_000 {
                self.underruns.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.last_mark = Some((frames, heard_ns));
        self.stamp.write([frames, heard_ns + ns(limiter::LOOK as u64)]);
    }

    /// Fill an interleaved device buffer (`channels` per frame) of any length with the device channels (the deck
    /// configuration's count; a callback with more channels gets silence on the rest, one with fewer the first ones – a
    /// mono callback on a stereo engine the mean of both). The engine renders whole `QUANTUM`s (one control read and one
    /// master ramp each) and hands out their frames: what is left of the last one waits for the next call. Never allocates.
    pub fn render<T: Copy>(&mut self, out: &mut [T], channels: usize, conv: impl Fn(f32) -> T) {
        let channels = channels.max(1);
        let total = out.len() / channels;
        let mut done = 0;
        while done < total {
            if self.pending == 0 {
                self.quantum();
            }
            let from = QUANTUM - self.pending;
            let take = (total - done).min(self.pending);
            let frames = &mut out[done * channels..(done + take) * channels];
            if channels == 1 && self.dev.channels() == 2 {
                let (l, r) = (self.dev.ch(0), self.dev.ch(1));
                for (f, x) in frames.iter_mut().enumerate() {
                    *x = conv(0.5 * (l[from + f] + r[from + f]));
                }
            } else {
                for c in 0..channels {
                    if c < self.dev.channels() {
                        let src = &self.dev.ch(c)[from..from + take];
                        for (f, &v) in src.iter().enumerate() {
                            frames[f * channels + c] = conv(v);
                        }
                    } else {
                        for f in 0..take {
                            frames[f * channels + c] = conv(0.0);
                        }
                    }
                }
            }
            self.pending -= take;
            done += take;
        }
    }

    /// One quantum: the control half, the steps, the meters, the outlets on the device channels, the master's ramp, the stage environment and
    /// the soft limiters (silence without an outlet), the frame counter on.
    fn quantum(&mut self) {
        let n = QUANTUM;
        self.control.block(self.frames.load(Ordering::Relaxed), n, self.rate);
        self.run(n);
        if let Some(m) = &self.meters {
            for (b, &(at, _)) in self.bufs.iter().zip(&m.at) {
                for c in 0..b.channels() {
                    let peak = b.ch(c)[..n].iter().fold(0.0f32, |p, x| p.max(x.abs()));
                    m.levels[at + c].store(peak.to_bits(), Ordering::Relaxed);
                }
            }
        }
        let (from, to) = (self.gain, self.master.get());
        self.gain = to;
        // the outlets' sound on the device channels: the first writer of a channel copies, the rest add
        for r in &self.routes {
            let src = &self.bufs[r.src].ch(usize::from(r.from))[..n];
            let dst = &mut self.dev.ch_mut(usize::from(r.to))[..n];
            if r.first {
                for (d, s) in dst.iter_mut().zip(src) {
                    *d = s * r.gain;
                }
            } else {
                for (d, s) in dst.iter_mut().zip(src) {
                    *d += s * r.gain;
                }
            }
        }
        for &c in &self.unwritten {
            self.dev.ch_mut(c)[..n].fill(0.0);
        }
        // the master's ramp, the room, then the soft limiters (never over 1, NaN = silence – no hard clamp, #188)
        for &c in &self.written {
            for (f, d) in self.dev.ch_mut(c)[..n].iter_mut().enumerate() {
                *d *= from + (to - from) * (f + 1) as f32 / n as f32;
            }
        }
        if !self.routes.is_empty() {
            self.environment.process(&mut self.dev, n);
            let (data, stride) = self.dev.planar_mut();
            for set in &mut self.sets {
                set.limiter.process_planar(data, stride, &set.chans, n);
                self.reduction.fetch_max(set.limiter.take_reduction().to_bits(), Ordering::Relaxed);
            }
        }
        for c in 0..self.dev.channels() {
            let peak = self.dev.ch(c)[..n].iter().fold(0.0f32, |p, x| p.max(x.abs()));
            self.device.0[c].store(peak.to_bits(), Ordering::Relaxed);
        }
        self.pending = n;
        self.frames.fetch_add(n as u64, Ordering::Relaxed);
    }

    fn run(&mut self, n: usize) {
        for step in &mut self.steps {
            match step {
                Step::Node { block, inputs, out, outs } => {
                    let (before, after) = self.bufs.split_at_mut(*out);
                    let mut ins: [Option<&Buf>; MAX_PORTS] = [None; MAX_PORTS];
                    for (k, src) in inputs.iter().enumerate() {
                        ins[k] = src.map(|b| &before[b]);
                    }
                    block.process(&ins[..inputs.len()], &mut after[..*outs], n);
                }
                Step::Voices(v) => {
                    let (before, after) = self.bufs.split_at_mut(v.out);
                    v.process(before, &mut after[..v.len()], n);
                }
                Step::Gather { dst, taps } => {
                    let (before, after) = self.bufs.split_at_mut(*dst);
                    let d = &mut after[0];
                    d.clear(n);
                    for t in taps.iter() {
                        let s = &before[t.src];
                        if t.pairs.is_empty() {
                            for c in 0..d.channels().min(s.channels()) {
                                for (o, x) in d.ch_mut(c)[..n].iter_mut().zip(&s.ch(c)[..n]) {
                                    *o += x * t.gain;
                                }
                            }
                        } else {
                            for &(from, to) in &t.pairs {
                                for (o, x) in d.ch_mut(usize::from(to))[..n].iter_mut().zip(&s.ch(usize::from(from))[..n]) {
                                    *o += x * t.gain;
                                }
                            }
                        }
                    }
                }
                Step::Capture { src, dst } => {
                    let (src, dst) = (*src, *dst);
                    let (s, d) = if src < dst {
                        let (a, b) = self.bufs.split_at_mut(dst);
                        (&a[src], &mut b[0])
                    } else {
                        let (a, b) = self.bufs.split_at_mut(src);
                        (&b[0], &mut a[dst])
                    };
                    for c in 0..d.channels() {
                        if c < s.channels() {
                            d.ch_mut(c)[..n].copy_from_slice(&s.ch(c)[..n]);
                        } else {
                            d.ch_mut(c)[..n].fill(0.0);
                        }
                    }
                }
            }
        }
    }
}

/// the solo params of the buses with an audio path into `node`
fn upstream_solos(graph: &Graph, node: usize, solo_of: &[Option<Param>]) -> Arc<[Param]> {
    let mut seen = vec![false; graph.nodes.len()];
    let mut stack = vec![node];
    let mut found = Vec::new();
    while let Some(t) = stack.pop() {
        for l in graph.links.iter().filter(|l| l.ty == PortType::Audio && l.to.0 == t) {
            if !seen[l.from.0] {
                seen[l.from.0] = true;
                found.extend(solo_of[l.from.0].clone());
                stack.push(l.from.0);
            }
        }
    }
    found.into()
}

/// the solo params of the buses `node`'s post-fader `out` feeds, directly or further down (a folder it plays into, the
/// folder's folder): soloing a folder keeps its children audible (Reaper's folder solo)
fn downstream_solos(graph: &Graph, node: usize, solo_of: &[Option<Param>]) -> Arc<[Param]> {
    let Some(out) = graph.nodes[node].kind.output("out") else { return Arc::new([]) };
    let mut seen = vec![false; graph.nodes.len()];
    let mut stack: Vec<usize> = graph.links.iter().filter(|l| l.ty == PortType::Audio && l.from == (node, out)).map(|l| l.to.0).collect();
    let mut found = Vec::new();
    while let Some(t) = stack.pop() {
        if t == node || std::mem::replace(&mut seen[t], true) {
            continue;
        }
        found.extend(solo_of[t].clone());
        stack.extend(graph.links.iter().filter(|l| l.ty == PortType::Audio && l.from.0 == t).map(|l| l.to.0));
    }
    found.into()
}

/// An `out` node or the `audio_out` sink, as the outlet planning sees it.
struct Feed {
    node: usize,
    id: String,
    /// the sink's own buffer and its width
    buf: usize,
    width: usize,
    group: String,
    /// the device channels the node names itself, counted from 1: a `channels` list, else `first` (0 = none)
    list: Vec<usize>,
    first: usize,
    /// what the channels are (`layout`), for the fold; by count when absent
    layout: Option<String>,
}

impl Feed {
    fn of(node: &deck_space::graph::Node, index: usize, buf: usize, width: usize) -> Feed {
        Feed {
            node: index,
            id: node.id.clone(),
            buf,
            width,
            group: node.str("group").unwrap_or("main").to_string(),
            list: node.f32s("channels").iter().map(|&c| c.max(0.0) as usize).collect(),
            first: node.f32("first", 0.0).max(0.0) as usize,
            layout: node.str("layout").map(str::to_string),
        }
    }
}

/// what the outlets come to on a device
struct Plan {
    routes: Vec<Route>,
    written: Vec<usize>,
    unwritten: Vec<usize>,
    sets: Vec<LimitSet>,
    notes: Vec<String>,
}

/// The deck configuration applied: which device channels each outlet's channels play on, how an outlet with more channels
/// than the device channels it has is folded, which device channels the limiter links (the channels of one outlet group, and
/// groups that share a channel). Allocates; the render only runs the result.
fn plan_outlets(feeds: &[Feed], deck: &DeckConfig, rate: f32) -> Plan {
    let d = deck.channels;
    let (mut notes, mut routes) = (Vec::new(), Vec::new());
    let mut claimed = vec![false; d];
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for f in feeds {
        // the device channel of each of the outlet's channels (None = the device lacks it), then the ones there are
        let mut slots: Vec<Option<usize>> = if !f.list.is_empty() {
            f.list.iter().map(|&c| (c >= 1 && c <= d).then(|| c - 1)).collect()
        } else if f.first > 0 {
            (f.first..f.first + f.width).map(|c| (c <= d).then(|| c - 1)).collect()
        } else {
            deck.channels_of(&f.group, f.width)
        };
        slots.resize(f.width, None);
        let targets: Vec<usize> = slots.iter().flatten().copied().collect();
        let m = targets.len();
        if m == 0 {
            notes.push(format!("outlet '{}' ({}): no device channel to play on ({} has {d}) – silent", f.group, f.id, deck.name));
            continue;
        }
        let mut pairs: Vec<(usize, usize, f32)> = Vec::new();
        // a mono outlet the scene does not pin plays on both channels of its group's pair – the spec's mono → stereo
        // up-mix, at the outlet only (owner 2026-11-03: "copy to both"); pinned (`first`, `channels`) it keeps its one channel
        let pair = deck.channels_of(&f.group, 2);
        if f.width == 1 && f.list.is_empty() && f.first == 0 && pair.iter().all(Option::is_some) {
            pairs.extend(pair.iter().flatten().map(|&t| (0, t, 1.0)));
        } else if m == f.width {
            pairs.extend((0..f.width).map(|c| (c, targets[c], 1.0)));
        } else {
            let from = f.layout.clone().or_else(|| layout::name_for(f.width).map(str::to_string));
            let table = match (&from, layout::name_for(m)) {
                (Some(from), Some(to)) if deck.fold == Fold::Speakers => layout::matrix(from, to),
                _ => None,
            };
            match table {
                Some(mx) if mx.len() == m && mx.iter().all(|r| r.len() == f.width) => {
                    notes.push(format!("outlet '{}' ({}): {} channels folded onto {m} by the {} → {} down-mix table", f.group, f.id, f.width, from.unwrap_or_default(), layout::name_for(m).unwrap_or("?")));
                    for (r, row) in mx.iter().enumerate() {
                        pairs.extend(row.iter().enumerate().filter(|(_, &g)| g != 0.0).map(|(c, &g)| (c, targets[r], g)));
                    }
                }
                _ => {
                    notes.push(format!("outlet '{}' ({}): {} channels, {m} of them have a device channel – the others are dropped", f.group, f.id, f.width));
                    pairs.extend(slots.iter().enumerate().filter_map(|(c, &t)| t.map(|t| (c, t, 1.0))));
                }
            }
        }
        let g = match groups.iter().position(|(name, _)| *name == f.group) {
            Some(g) => g,
            None => {
                groups.push((f.group.clone(), Vec::new()));
                groups.len() - 1
            }
        };
        for (c, to, gain) in pairs {
            routes.push(Route { src: f.buf, from: c as u8, to: to as u8, gain, first: !claimed[to] });
            claimed[to] = true;
            if !groups[g].1.contains(&to) {
                groups[g].1.push(to);
            }
        }
    }
    // the limiter links the channels of an outlet group; groups that share a device channel are one
    let mut comp: Vec<usize> = (0..d).collect();
    for (_, chans) in &groups {
        for w in chans.windows(2) {
            let (a, b) = (comp[w[0]], comp[w[1]]);
            if a != b {
                comp.iter_mut().filter(|c| **c == b).for_each(|c| *c = a);
            }
        }
    }
    let mut sets: Vec<LimitSet> = Vec::new();
    let mut keys: Vec<usize> = Vec::new();
    for c in (0..d).filter(|&c| claimed[c]) {
        match keys.iter().position(|&k| k == comp[c]) {
            Some(s) => sets[s].chans.push(c),
            None => {
                keys.push(comp[c]);
                sets.push(LimitSet { chans: vec![c], limiter: LinkedLimiter::new(rate, 1) });
            }
        }
    }
    for s in &mut sets {
        s.limiter = LinkedLimiter::new(rate, s.chans.len());
    }
    Plan { routes, written: (0..d).filter(|&c| claimed[c]).collect(), unwritten: (0..d).filter(|&c| !claimed[c]).collect(), sets, notes }
}

/// `node`'s input params, one per input port: its param (or the kind's default); a linked control port's goes to the
/// frame thread (`Handles::params`)
fn node_params(graph: &Graph, i: usize, h: &mut Handles) -> Vec<Param> {
    let node = &graph.nodes[i];
    let ports = node.kind.inputs();
    // the block's input list holds the audio ports (a module's parameters come after them and are not in it)
    debug_assert!(ports.iter().filter(|p| p.ty == PortType::Audio).count() <= MAX_PORTS);
    ports
        .iter()
        .enumerate()
        .map(|(k, port)| {
            let p = Param::new(node.f32(port.name, blocks::default_for(node.kind, port.name)));
            if port.ty == PortType::Control {
                let list = if graph.link_into(i, k).is_some() { &mut h.params } else { &mut h.knobs };
                list.push((i, k, p.clone()));
            }
            p
        })
        .collect()
}

/// The buffer each audio input port of `node` reads (None = nothing linked). A port is as wide as its kind says, an `any`
/// port as the widest of its links; a link that is alone, unscaled, unmapped and of that width is read in place, else a
/// gather buffer holds the sum of the links (their scale as the send level, their map as the channel map: channels go n →
/// n by default, and a map names source → target pairs), in the scene's link order – float addition is not associative,
/// so the order is the contract (plan 2.6a).
fn resolve_inputs(
    graph: &Graph,
    node: usize,
    deck: &DeckConfig,
    buf_of: &[Option<usize>],
    bufs: &mut Vec<Buf>,
    steps: &mut Vec<Step>,
) -> Result<Vec<Option<usize>>> {
    let n = &graph.nodes[node];
    let mut inputs = vec![None; n.kind.inputs().len()];
    for (k, port) in n.kind.inputs().iter().enumerate() {
        if port.ty != PortType::Audio {
            continue;
        }
        let links: Vec<&deck_space::graph::Link> = graph.links.iter().filter(|l| l.to == (node, k) && l.ty == PortType::Audio).collect();
        if links.is_empty() {
            continue;
        }
        let mut sources = Vec::with_capacity(links.len());
        for l in &links {
            let Some(b) = buf_of[l.from.0] else { anyhow::bail!("node '{}': its link from '{}' has no sound to read", n.id, graph.nodes[l.from.0].id) };
            sources.push((b + l.from.1, bufs[b + l.from.1].channels()));
        }
        let width = match route::want_in(n, k, deck)? {
            Some(w) => w,
            None => links
                .iter()
                .zip(&sources)
                .map(|(l, &(_, w))| if l.map.is_empty() { w } else { l.map.iter().map(|&(_, t)| usize::from(t) + 1).max().unwrap_or(1) })
                .max()
                .unwrap_or(1),
        }
        .clamp(1, MAX_CHANNELS);
        if links.len() == 1 && links[0].scale == 1.0 && links[0].map.is_empty() && sources[0].1 == width {
            inputs[k] = Some(sources[0].0);
            continue;
        }
        let taps = links
            .iter()
            .zip(&sources)
            .filter_map(|(l, &(src, w))| {
                let pairs: Vec<(u8, u8)> = l.map.iter().copied().filter(|&(s, t)| usize::from(s) < w && usize::from(t) < width).collect();
                (l.map.is_empty() || !pairs.is_empty()).then_some(Tap { src, gain: l.scale, pairs })
            })
            .collect();
        inputs[k] = Some(bufs.len());
        steps.push(Step::Gather { dst: bufs.len(), taps });
        bufs.push(Buf::sized(width, QUANTUM));
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_space::scene::Scene;

    fn engine(text: &str) -> (Engine, Handles) {
        let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
        Engine::build(&g, 48_000.0, 1.0).unwrap()
    }

    const OSC_OUT: &str = "[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
                           [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n";

    #[test]
    fn an_oscillator_reaches_the_output() {
        let (mut e, h) = engine(OSC_OUT);
        let mut buf = vec![0.0f32; 4800 * 2];
        e.render(&mut buf, 2, |x| x);
        let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.3 && peak <= 0.5 + 1e-3, "peak {peak}");
        assert_eq!(h.clock.frames.load(Ordering::Relaxed), 4864, "4800 frames = 37.5 quanta: the engine has rendered 38");
    }

    #[test]
    fn the_master_param_scales_and_extra_channels_are_silent() {
        let (mut e, h) = engine(OSC_OUT);
        h.master.set(0.0);
        let mut buf = vec![1.0f32; 512 * 4];
        e.render(&mut buf, 4, |x| x); // the ramp down
        assert!(buf.chunks(4).all(|f| f[2] == 0.0 && f[3] == 0.0), "the extra channels are silent");
        e.render(&mut buf, 4, |x| x);
        // the ramp's end comes out after the limiter's look-ahead (#188): silence from there
        assert!(buf[deck_core::limiter::LOOK * 4..].iter().all(|x| *x == 0.0), "the ramp ends at the new gain");
        e.render(&mut buf, 4, |x| x);
        assert!(buf.iter().all(|x| *x == 0.0));
    }

    /// #188: four loud voices summed far over 1 never reach the device's 1 (the soft limiter, no hard clip), and the
    /// gain reduction reaches the frame thread
    #[test]
    fn overload_is_limited_and_its_reduction_reported() {
        let mut s = String::new();
        for i in 0..4 {
            s += &format!("[[node]]\nid=\"o{i}\"\nkind=\"osc\"\nfreq={}\ngain=0.9\n[[link]]\nfrom=\"o{i}\"\nto=\"m.in{}\"\n", 110 + 55 * i, i + 1);
        }
        s += "[[node]]\nid=\"m\"\nkind=\"mixer\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"m\"\nto=\"out.in\"\n";
        for build in [Build::detect(), Build::patcher()] {
            let g = Graph::build(&Scene::parse(&s).unwrap()).unwrap();
            let (mut e, h) = Engine::build_with(&g, 48_000.0, 1.0, build).unwrap();
            let mut buf = vec![0.0f32; 48_000 * 2];
            e.render(&mut buf, 2, |x| x);
            let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak < 1.0 && peak > 0.85, "{}: peak {peak}", build.label());
            assert!(h.take_reduction() > 6.0, "3.6 summed down to the ceiling");
            assert_eq!(h.take_reduction(), 0.0, "taken");
        }
    }

    /// #188: a block the device plays later than the last one's end is an underrun; on time (a little jitter) is not
    #[test]
    fn late_blocks_count_as_underruns() {
        let (mut e, h) = engine(OSC_OUT);
        let mut buf = vec![0.0f32; 128 * 2];
        let block = 128 * 1_000_000_000 / 48_000; // 2.67 ms
        let mut at = 1_000_000_000u64;
        for k in 0..20u64 {
            // a millisecond of jitter either way, and one gap of 30 ms after block 10
            at += block + if k % 2 == 0 { 1_000_000 } else { 0 } - if k % 2 == 1 { 1_000_000 } else { 0 };
            if k == 10 {
                at += 30_000_000;
            }
            e.mark(at);
            e.render(&mut buf, 2, |x| x);
        }
        assert_eq!(h.underruns.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn no_output_node_is_silence() {
        let (mut e, _) = engine("[[node]]\nid=\"o\"\nkind=\"osc\"\n");
        let mut buf = vec![1.0f32; 256];
        e.render(&mut buf, 2, |x| x);
        assert!(buf.iter().all(|x| *x == 0.0));
    }

    /// `n` voices of `kind`, a param set each (pitches, gains, envelopes apart), half of them on a sequencer's cv,
    /// mixed four by four into the output – plus `extra` nodes / links
    fn voices(kind: &str, n: usize, extra: &str) -> String {
        let mut s = String::from("[[node]]
id=\"seq\"
kind=\"seq\"
bpm=133
division=4
gate=0.45
steps=[45, 0, 52, 57, 55, 0, 60, 64]
");
        for i in 0..n {
            let (f, a, r) = (97.0 + 41.3 * i as f32, 0.001 + 0.002 * i as f32, 0.05 + 0.03 * i as f32);
            s += &format!("[[node]]
id=\"v{i}\"
kind=\"{kind}\"
freq={f}
gain=0.06
ratio={}
index={}
attack={a}
release={r}
", 1.0 + 0.5 * (i % 4) as f32, 0.3 * i as f32);
            s += &format!("[[link]]
from=\"v{i}\"
to=\"m{}.in{}\"
", i / 4, i % 4 + 1);
            if i % 2 == 1 {
                s += &format!("[[link]]
from=\"seq.cv\"
to=\"v{i}.cv\"
");
            }
        }
        for m in 0..n.div_ceil(4) {
            s += &format!("[[node]]
id=\"m{m}\"
kind=\"mixer\"
[[link]]
from=\"m{m}\"
to=\"sum.in{}\"
", m + 1);
        }
        s + "[[node]]
id=\"sum\"
kind=\"mixer\"
[[node]]
id=\"out\"
kind=\"audio_out\"
volume=1
[[link]]
from=\"sum\"
to=\"out.in\"
" + extra
    }

    /// two seconds of `text` through two builds, in blocks of 128, 37 and 1024 frames (a track for its chapter_music);
    /// the largest sample difference
    fn compare(text: &str, a: Build, b: Build) -> f32 {
        let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
        let (mut x, mut hx) = Engine::build_with(&g, 48_000.0, 1.0, a).unwrap();
        let (mut y, mut hy) = Engine::build_with(&g, 48_000.0, 1.0, b).unwrap();
        for h in [&mut hx, &mut hy] {
            if let Some(m) = h.music.first_mut() {
                let t = Track { rate: 44_100.0, l: (0..44_100).map(|i| (i as f32 * 0.01).sin() * 0.3).collect(), r: vec![0.1; 44_100] };
                m.tracks.push(Arc::new(t)).ok().unwrap();
            }
        }
        let mut worst = 0.0f32;
        let mut done = 0;
        for i in 0.. {
            let n = [128, 37, 1024][i % 3];
            let (mut p, mut q) = (vec![0.0f32; n * 2], vec![0.0f32; n * 2]);
            x.render(&mut p, 2, |v| v);
            y.render(&mut q, 2, |v| v);
            worst = p.iter().zip(&q).fold(worst, |w, (u, v)| w.max((u - v).abs()));
            done += n;
            if done >= 96_000 {
                break;
            }
        }
        assert!(hx.clock.rendered() == 96_000 || done > 96_000);
        worst
    }

    const BASELINE: Build = Build { batch: true, avx2: false };

    /// The same scenes through the patcher (every node a block, scalar sines and FM) and the batch runtime, two seconds
    /// each, blocks of 128, 37 and 1024: the same samples, bit for bit. The patcher blocks are the reference the batches
    /// are held to – two implementations of one arithmetic (deck-core's `sin_turns`, `START_PHASE`, the envelope, the
    /// glide), no outside library (fundsp, the old oracle, left with #317). Before it the batches were compared with
    /// fundsp's sines within 2e-7…1.7e-4; now the difference is exactly 0.
    #[test]
    fn the_batches_sound_like_the_patcher() {
        let extra = "[[node]]
id=\"saw\"
kind=\"osc\"
wave=\"saw\"
freq=55
gain=0.1
[[link]]
from=\"saw\"
to=\"sum.in4\"
";
        for (name, text, batches) in [
            ("12 sines + a saw", voices("osc", 12, extra), 2),
            ("the default scene", deck_space::scene::MADNESS_SCENE.to_string(), 1),
            ("16 FM voices, index up to 4.5", voices("fm", 16, ""), 2),
        ] {
            let g = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
            let (e, _) = Engine::build_with(&g, 48_000.0, 1.0, BASELINE).unwrap();
            // the sequenced voices sit a level above the free ones: two batches
            assert_eq!(e.shape().1, batches, "{name}: {:?}", e.shape());
            for b in [BASELINE, Build::detect()] {
                let worst = compare(&text, Build::patcher(), b);
                assert_eq!(worst, 0.0, "{name} ({}): the largest difference", b.label());
            }
        }
    }

    /// AVX2 changes the lanes, not the maths: the same samples, bit for bit.
    #[test]
    fn avx2_and_baseline_are_bit_identical() {
        if !voices::avx2_available() {
            return;
        }
        let avx2 = Build { batch: true, avx2: true };
        assert_eq!(compare(&voices("fm", 13, ""), BASELINE, avx2), 0.0);
        assert_eq!(compare(&voices("osc", 16, ""), BASELINE, avx2), 0.0);
    }

    /// The quantum makes the output independent of the platform's buffer size: the same scene (voices on a sequencer's
    /// cv, a mixer, the limiter, a master ramp at the start) through callbacks of 37, 128, 480 and 1056 frames gives the
    /// same bits, and `mark` reports the position of the next frame the callback takes (not the rendered frontier).
    #[test]
    fn the_output_does_not_depend_on_the_buffer_size() {
        let g = Graph::build(&Scene::parse(&voices("fm", 8, "")).unwrap()).unwrap();
        let render = |size: usize| {
            let (mut e, h) = Engine::build_with(&g, 48_000.0, 1.0, Build::detect()).unwrap();
            let mut all = Vec::new();
            let mut buf = vec![0.0f32; size * 2];
            h.master.set(0.5); // a ramp from 1 in the first quantum
            while all.len() < 48_000 * 2 {
                e.mark(0);
                e.render(&mut buf, 2, |x| x);
                all.extend_from_slice(&buf);
            }
            all.truncate(48_000 * 2);
            all
        };
        let reference = render(128);
        assert!(reference.iter().any(|x| x.abs() > 0.01), "the scene sounds");
        for size in [37, 480, 1056] {
            assert!(render(size) == reference, "callbacks of {size} frames change the sound");
        }
    }

    #[test]
    fn marks_say_where_the_next_frame_is() {
        let (mut e, h) = engine(OSC_OUT);
        let mut buf = vec![0.0f32; 100 * 2];
        e.mark(1_000_000_000);
        e.render(&mut buf, 2, |x| x);
        assert_eq!(h.clock.rendered(), QUANTUM as u64, "a whole quantum is rendered");
        e.mark(1_000_000_000 + 2_083_333);
        let (_, [frames, _]) = h.clock.stamp.read().unwrap();
        assert_eq!(frames, 100, "the next frame the callback takes is the hundredth");
    }

    #[test]
    fn levels_and_batches() {
        let g = Graph::build(&Scene::parse(&voices("fm", 16, "")).unwrap()).unwrap();
        let (b, _) = Engine::build_with(&g, 48_000.0, 1.0, BASELINE).unwrap();
        // seq + the free voices' batch | the sequenced voices' batch | 4 mixers | sum | out
        assert_eq!(b.shape(), (2 + 1 + 4 + 1 + 1, 2, "baseline"));
        let (p, _) = Engine::build_with(&g, 48_000.0, 1.0, Build::patcher()).unwrap();
        assert_eq!(p.shape(), (1 + 16 + 4 + 1 + 1, 0, "-"));
    }

    #[test]
    fn linked_control_inputs_become_params() {
        let (_, h) = engine(
            "[[node]]\nid=\"l\"\nkind=\"lfo\"\n[[node]]\nid=\"o\"\nkind=\"osc\"\n[[link]]\nfrom=\"l\"\nto=\"o.freq\"\n",
        );
        assert_eq!(h.params.len(), 1);
        assert_eq!(h.params[0].1, Kind::Osc.input("freq").unwrap());
    }
}
