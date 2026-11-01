//! The audio graph (CYBERDECKS.md §8), run by the device callback in blocks of at most MAX_BLOCK frames, on the batch
//! model (#183): at build the audio nodes are split into dependency levels (by their audio links); inside a level the
//! sine oscillators and the FM voices become one batch each (voices.rs: struct of arrays, the voice loop vectorised,
//! AVX2 where the CPU has it), every other node stays a block of its own (blocks.rs – the patcher model, for the
//! one-off nodes). Every output port has one buffer in a pool ordered like the steps, so a step reads only buffers
//! before its own. Everything is allocated at build time – the callback only reads atomics (Param), pops lock-free
//! rings (rtrb) and writes preallocated buffers (tests/no_alloc.rs proves it).
//! The frame counter and the per-block stamps are the stage's time base (§8 "Two clocks", clock.rs): each backend calls
//! `mark` with the moment the block it is about to render will be heard. The tempo map (tempo/, #181) reaches the
//! click and the synced sequencer through a seqlock, read once per block.
pub mod backend;
mod blocks;
mod click;
#[cfg(feature = "events")]
pub mod dispatch;
pub mod glitch;
pub mod lead;
pub mod priority;
pub mod ring;
#[cfg(feature = "events")]
pub mod sequencer;
mod voices;
#[cfg(feature = "events")]
mod walk;

use crate::clock::Stamp;
use crate::tempo;
use crate::graph::{Domain, Graph, Kind, PortType};
use anyhow::Result;
use deck_core::batch::levels;
use deck_core::limiter::{self, Limiter};
use voices::{VoiceKind, Voices};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub use blocks::default_for;

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

/// Every output buffer's newest block peak – built only for a scene with a `space` node (the being's audio hoses,
/// #188), so the other scenes' blocks cost nothing extra. Written by the audio thread after each block, read per frame.
pub struct Meters {
    levels: Box<[AtomicU32]>,
    /// per graph node: its first output buffer
    buf_of: Vec<Option<usize>>,
}

impl Meters {
    /// `node`'s output `port`: the newest block's peak (0 for a node without audio outputs)
    pub fn level(&self, node: usize, port: usize) -> f32 {
        match self.buf_of.get(node).copied().flatten().and_then(|b| self.levels.get(b + port)) {
            Some(l) => f32::from_bits(l.load(Ordering::Relaxed)),
            None => 0.0,
        }
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

/// One stereo block buffer (MAX_BLOCK frames, a block uses the first `n`).
pub struct Stereo {
    pub l: Box<[f32]>,
    pub r: Box<[f32]>,
}

impl Stereo {
    fn new() -> Stereo {
        Stereo { l: vec![0.0; MAX_BLOCK].into(), r: vec![0.0; MAX_BLOCK].into() }
    }
}

/// An audio node's DSP: reads its inputs (by input port index; None = not linked or a control port), writes its outputs.
pub trait Block: Send {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize);
}

/// How an engine is built. The stage runs `Build::detect()`; the patcher build is the reference the tests and the bench
/// compare with (every node a block, the sine oscillators and FM voices on fundsp – the stage before #183).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build {
    /// sine oscillators and FM voices batched per level (else every node a block)
    pub batch: bool,
    /// the batches' loops in AVX2 (only where the CPU has it)
    pub avx2: bool,
}

impl Build {
    /// The batch runtime, AVX2 where the CPU has it – `DECK_SIMD=baseline` keeps the baseline loops (to compare).
    pub fn detect() -> Build {
        let baseline = std::env::var("DECK_SIMD").is_ok_and(|v| v == "baseline");
        Build { batch: true, avx2: voices::avx2_available() && !baseline }
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

/// Where the `walk` / `events` hosts run their generators (#236, walk.rs): on the sequencer thread, ahead of the
/// render (the stage's default), or inside the audio block (the fallback – `DECK_EVENTS=block`, the tests'
/// `build_with`). Both play the same samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sequencing {
    InBlock,
    Thread,
}

impl Sequencing {
    /// the thread, unless `DECK_EVENTS=block`
    pub fn detect() -> Sequencing {
        match std::env::var("DECK_EVENTS") {
            Ok(v) if v == "block" => Sequencing::InBlock,
            _ => Sequencing::Thread,
        }
    }
}

/// One step of a block, in level order.
enum Step {
    /// one node (the patcher model): its block, the buffer each input port reads, its outputs from buffer `out` on
    Node { block: Box<dyn Block>, inputs: Vec<Option<usize>>, out: usize, outs: usize },
    /// a level's voices of one kind as one batch
    Voices(Box<Voices>),
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
    /// every output port's buffer (and the audio_out's own), in step order
    bufs: Vec<Stereo>,
    steps: Vec<Step>,
    out_buf: Option<usize>,
    master: Param,
    /// the master gain the last block ended on – a change ramps over one block (no click on a mute)
    gain: f32,
    frames: Arc<AtomicU64>,
    stamp: Arc<Stamp>,
    meters: Option<Arc<Meters>>,
    /// the output's soft cap (#188, deck-core's limiter.rs) – last before the device, instead of a hard clamp – and
    /// its block buffer
    limiter: Limiter,
    mix: Stereo,
    /// the deepest gain reduction (dB, f32 bits) since the frame thread last took it
    reduction: Arc<AtomicU32>,
    /// late blocks (`mark`): the count, and the last mark (frames rendered, heard at)
    underruns: Arc<AtomicU64>,
    last_mark: Option<(u64, u64)>,
    rate: f32,
    /// the glitch indicator's callback half (#208) – the backend takes it into the thread that runs the device callback
    monitor: Option<glitch::Monitor>,
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
    /// fft nodes: their sample ring
    pub taps: Vec<(usize, rtrb::Consumer<f32>)>,
    pub clock: Clock,
    pub master: Param,
    /// the tempo map the frame thread publishes (tempo::Tempo) and the audio thread reads
    pub tempo: Arc<tempo::Shared>,
    /// how the engine was built (the HUD shows it)
    pub build: Build,
    /// the limiter's deepest gain reduction since the last `take_reduction` (#188: the HUD, the being's red face)
    pub reduction: Arc<AtomicU32>,
    /// blocks the device played late (an underrun: `Engine::mark` saw a gap) – one of the glitch causes
    pub underruns: Arc<AtomicU64>,
    /// the glitch indicator (#208): counts per cause, the newest, the load, the device's buffer, the render lead
    pub glitches: Arc<glitch::Glitches>,
    /// each glitch's numbers for the log – the frame thread drains it (never the audio thread)
    pub glitch_log: Option<rtrb::Consumer<glitch::Event>>,
    /// the hosts' events (#236): names, the sequencer, the routing for the dispatcher – `events.start()` runs them
    #[cfg(feature = "events")]
    pub events: dispatch::Events,
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
    /// sequencer thread (`Sequencing::detect` – start it with `handles.events.start()`). `volume` is the master gain
    /// on top of the audio_out node's own.
    pub fn build(graph: &Graph, rate: f32, volume: f32) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, Build::detect(), Sequencing::detect())
    }

    /// `build` with this `build`, the hosts' generators inside the block (no thread to start – the tests)
    pub fn build_with(graph: &Graph, rate: f32, volume: f32, build: Build) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, build, Sequencing::InBlock)
    }

    pub fn build_seq(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing) -> Result<(Engine, Handles)> {
        #[cfg(not(feature = "events"))]
        let _ = seq;
        let (monitor, glitches, glitch_log) = glitch::Monitor::new(rate);
        let mut handles = Handles {
            params: Vec::new(),
            knobs: Vec::new(),
            meters: None,
            music: Vec::new(),
            taps: Vec::new(),
            clock: Clock { frames: Arc::new(AtomicU64::new(0)), rate, stamp: Arc::new(Stamp::default()) },
            master: Param::new(volume),
            tempo: Arc::new(tempo::Shared::default()),
            build,
            reduction: Arc::new(AtomicU32::new(0)),
            underruns: Arc::new(AtomicU64::new(0)),
            #[cfg(feature = "events")]
            events: dispatch::Events::new(seq, glitches.clone()),
            glitches,
            glitch_log: Some(glitch_log),
        };
        // the scene's tempo until the frame thread takes over
        let cfg = tempo::Config::from_graph(graph)?;
        handles.tempo.publish(&cfg.map(rate as f64), cfg.click);
        let audio = |i: usize| graph.nodes[i].kind.domain() == Domain::Audio;
        // levels by the audio links (control links into or out of audio nodes cross threads and never order them)
        let deps: Vec<Vec<usize>> = (0..graph.nodes.len())
            .map(|t| graph.links.iter().filter(|l| l.to.0 == t && l.ty == PortType::Audio).map(|l| l.from.0).collect())
            .collect();
        let level = levels(&deps).map_err(|e| anyhow::anyhow!("the audio graph: {e}"))?;
        let depth = (0..graph.nodes.len()).filter(|&i| audio(i)).map(|i| level[i] + 1).max().unwrap_or(0);
        let mut bufs: Vec<Stereo> = Vec::new();
        let mut steps = Vec::new();
        let mut buf_of: Vec<Option<usize>> = vec![None; graph.nodes.len()];
        for l in 0..depth {
            let nodes: Vec<usize> = graph.order.iter().copied().filter(|&i| audio(i) && level[i] == l).collect();
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
                    let cv = node.kind.input("cv").and_then(|k| audio_in(graph, &buf_of, i, k));
                    batch.push([p("freq"), p("gain"), ratio, index], cv, blocks::env_coefs(rate, node));
                    buf_of[i] = Some(bufs.len());
                    bufs.push(Stereo::new());
                }
                batch.finish();
                steps.push(Step::Voices(Box::new(batch)));
            }
            // the one-off nodes
            for &i in &nodes {
                if buf_of[i].is_some() {
                    continue; // batched above
                }
                let node = &graph.nodes[i];
                let params = node_params(graph, i, &mut handles);
                let inputs = (0..node.kind.inputs().len()).map(|k| audio_in(graph, &buf_of, i, k)).collect();
                let block = blocks::make(node, i, rate, &params, &mut handles)?;
                // the audio_out sink has no output port but keeps its block in one buffer – render() reads it
                let outs = if node.kind == Kind::AudioOut { 1 } else { node.kind.outputs().len() };
                buf_of[i] = Some(bufs.len());
                steps.push(Step::Node { block, inputs, out: bufs.len(), outs });
                bufs.extend((0..outs).map(|_| Stereo::new()));
            }
        }
        let out_buf = graph.first(Kind::AudioOut).and_then(|i| buf_of[i]);
        if graph.first(Kind::Space).is_some() {
            let levels = (0..bufs.len()).map(|_| AtomicU32::new(0)).collect();
            handles.meters = Some(Arc::new(Meters { levels, buf_of: buf_of.clone() }));
        }
        let engine = Engine {
            meters: handles.meters.clone(),
            bufs,
            steps,
            out_buf,
            master: handles.master.clone(),
            gain: volume,
            frames: handles.clock.frames.clone(),
            stamp: handles.clock.stamp.clone(),
            limiter: Limiter::new(rate),
            mix: Stereo::new(),
            reduction: handles.reduction.clone(),
            underruns: handles.underruns.clone(),
            last_mark: None,
            rate,
            monitor: Some(monitor),
        };
        Ok((engine, handles))
    }

    /// The glitch monitor for the thread that runs the device callback (#208) – once; a second call gets a fresh one
    /// whose counts nobody reads.
    pub fn take_monitor(&mut self) -> glitch::Monitor {
        self.monitor.take().unwrap_or_else(|| glitch::Monitor::new(self.rate).0)
    }

    /// the count `mark` raises on a gap in the heard stamps (a backend compares it around a callback)
    pub fn underruns(&self) -> Arc<AtomicU64> {
        self.underruns.clone()
    }

    pub fn rate(&self) -> f32 {
        self.rate
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
        let frames = self.frames.load(Ordering::Relaxed);
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

    /// Fill an interleaved device buffer (`channels` per frame; L, R, the rest silent). Never allocates.
    pub fn render<T: Copy>(&mut self, out: &mut [T], channels: usize, conv: impl Fn(f32) -> T) {
        let channels = channels.max(1);
        let total = out.len() / channels;
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            self.run(n);
            if let Some(m) = &self.meters {
                for (b, level) in self.bufs.iter().zip(m.levels.iter()) {
                    let peak = b.l[..n].iter().chain(&b.r[..n]).fold(0.0f32, |p, x| p.max(x.abs()));
                    level.store(peak.to_bits(), Ordering::Relaxed);
                }
            }
            let (from, to) = (self.gain, self.master.get());
            self.gain = to;
            let frames = &mut out[done * channels..(done + n) * channels];
            match self.out_buf.map(|b| &self.bufs[b]) {
                Some(o) => {
                    // the master's ramp, then the soft limiter (never over 1, NaN = silence – no hard clamp, #188)
                    let mix = &mut self.mix;
                    for f in 0..n {
                        let master = from + (to - from) * (f + 1) as f32 / n as f32;
                        (mix.l[f], mix.r[f]) = (o.l[f] * master, o.r[f] * master);
                    }
                    self.limiter.process(&mut mix.l[..n], &mut mix.r[..n]);
                    self.reduction.fetch_max(self.limiter.take_reduction().to_bits(), Ordering::Relaxed);
                    for (f, frame) in frames.chunks_exact_mut(channels).enumerate() {
                        let (l, r) = (mix.l[f], mix.r[f]);
                        if channels == 1 {
                            frame[0] = conv(0.5 * (l + r));
                            continue;
                        }
                        frame[0] = conv(l);
                        frame[1] = conv(r);
                        for x in &mut frame[2..] {
                            *x = conv(0.0);
                        }
                    }
                }
                None => frames.iter_mut().for_each(|x| *x = conv(0.0)),
            }
            done += n;
            self.frames.fetch_add(n as u64, Ordering::Relaxed);
        }
    }

    fn run(&mut self, n: usize) {
        for step in &mut self.steps {
            match step {
                Step::Node { block, inputs, out, outs } => {
                    let (before, after) = self.bufs.split_at_mut(*out);
                    let mut ins: [Option<&Stereo>; MAX_PORTS] = [None; MAX_PORTS];
                    for (k, src) in inputs.iter().enumerate() {
                        ins[k] = src.map(|b| &before[b]);
                    }
                    block.process(&ins[..inputs.len()], &mut after[..*outs], n);
                }
                Step::Voices(v) => {
                    let (before, after) = self.bufs.split_at_mut(v.out);
                    v.process(before, &mut after[..v.len()], n);
                }
            }
        }
    }
}

/// `node`'s input params, one per input port: its param (or the kind's default); a linked control port's goes to the
/// frame thread (`Handles::params`)
fn node_params(graph: &Graph, i: usize, h: &mut Handles) -> Vec<Param> {
    let node = &graph.nodes[i];
    let ports = node.kind.inputs();
    debug_assert!(ports.len() <= MAX_PORTS);
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

/// the buffer `node`'s input port `k` reads, if an audio link feeds it (its source sits in an earlier level)
fn audio_in(graph: &Graph, buf_of: &[Option<usize>], node: usize, k: usize) -> Option<usize> {
    match graph.link_into(node, k) {
        Some(l) if l.ty == PortType::Audio => buf_of[l.from.0].map(|b| b + l.from.1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

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
        assert_eq!(h.clock.frames.load(Ordering::Relaxed), 4800);
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

    /// The same scenes through the patcher (every node a block, fundsp's sines – the stage before #183) and the batch
    /// runtime, two seconds each: the same samples within a tolerance. Sines differ only by deck-core's sine (< 2e-7);
    /// FM multiplies the modulator's tiny difference by index × its frequency into the carrier's phase, which walks –
    /// at 4.5 × 1.8 kHz of deviation (the stress scene) by ~1e-4 of the voice's 0.06 after 2 s, the default scene's FM
    /// (index ≤ 3.5) by ~1e-5. Measured 2026-10-30: 2.2e-7 · 6.4e-6 · 1.7e-4.
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
        for (name, text, batches, tolerance) in [
            ("12 sines + a saw", voices("osc", 12, extra), 2, 1e-5),
            ("the default scene", crate::scene::MADNESS_SCENE.to_string(), 1, 1e-4),
            ("16 FM voices, index up to 4.5", voices("fm", 16, ""), 2, 1e-3),
        ] {
            // the default scene has chapter nodes (#185: a build without the module refuses it)
            if name == "the default scene" && !cfg!(feature = "chapter") {
                continue;
            }
            let g = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
            let (e, _) = Engine::build_with(&g, 48_000.0, 1.0, BASELINE).unwrap();
            // the sequenced voices sit a level above the free ones: two batches
            assert_eq!(e.shape().1, batches, "{name}: {:?}", e.shape());
            for b in [BASELINE, Build::detect()] {
                let worst = compare(&text, Build::patcher(), b);
                assert!(worst < tolerance, "{name} ({}): the largest difference {worst}", b.label());
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
