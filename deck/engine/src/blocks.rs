//! The audio kinds' blocks – the patcher model: one block per node (#183). The batch runtime runs sine oscillators and
//! FM voices as batches (voices.rs) and everything else as these blocks; a patcher build (`Build::patcher`, the tests'
//! and the bench's reference) runs every node as a block. The oscillators, the noise and the filter are deck-core's
//! `dsp` units (wavetables built here, outside the callback); the wiring, envelopes, sequencer, player, mixer and FFT
//! tap are ours. Nothing in `process` allocates.
use super::click::Click;
use super::{Block, Buf, Handles, MusicFeed, Param, Seams, Track};
use anyhow::{bail, Result};
use deck_core::tempo::BeatReader;
use deck_space::graph::{Kind, Node};
use deck_core::dsp::{self, Pink, Svf, Wave, WaveTable, White};
use deck_core::math::{round_fast, sin_turns};
use deck_core::smooth::{Ramp, GLIDE, QUICK};
use std::sync::{Arc, Mutex};

/// how many samples the FFT ring holds (≈ 170 ms at 48 kHz; the frame thread drains it every frame)
const TAP_RING: usize = 8192;

/// the engine's default of an audio node's control input (deck-space's params.rs – the being's buttons read it too)
pub use deck_space::params::default_for;

#[cfg_attr(not(feature = "modules"), allow(unused_variables))]
pub fn make(node: &Node, index: usize, rate: f32, params: &[Param], h: &mut Handles, seams: &Seams, tables: &mut Tables) -> Result<Box<dyn Block>> {
    let p = |name: &str| params[node.kind.input(name).expect("a port of this kind")].clone();
    // a param read once per block, gliding across it (#188, deck-core's smooth.rs): gains and frequencies longer
    let glide = |name: &str| Glide::new(p(name), GLIDE, rate);
    let quick = |name: &str| Glide::new(p(name), QUICK, rate);
    Ok(match node.kind {
        Kind::Osc => {
            let shape = match node.str("wave").unwrap_or("sine") {
                "sine" => Shape::Sine { phase: dsp::START_PHASE },
                w => match Wave::parse(w) {
                    Some(wave) => Shape::Table { table: tables.get(wave), osc: dsp::Osc::new() },
                    None => bail!("node '{}': unknown wave '{w}' (sine, saw, square, triangle)", node.id),
                },
            };
            Box::new(Osc { shape, sd: 1.0 / rate, freq: glide("freq"), gain: glide("gain"), env: Env::new(rate, node) })
        }
        Kind::Noise => {
            let seed = noise_seed(node, index);
            let src = match node.str("color").unwrap_or("white") {
                "white" => Source::White(White::new(seed)),
                "pink" => Source::Pink(Pink::new(seed)),
                c => bail!("node '{}': unknown color '{c}' (white, pink)", node.id),
            };
            Box::new(Noise {
                src,
                filter: Svf::new(),
                pi_over_rate: std::f32::consts::PI / rate,
                cutoff: quick("cutoff"),
                q: node.f32("q", 0.7),
                gain: glide("gain"),
            })
        }
        Kind::Fm => Box::new(Fm {
            carrier: dsp::START_PHASE,
            modulator: dsp::START_PHASE,
            sd: 1.0 / rate,
            freq: glide("freq"),
            ratio: quick("ratio"),
            index: quick("index"),
            gain: glide("gain"),
            env: Env::new(rate, node),
        }),
        Kind::Seq => {
            // a `bpm` of its own: free-running; else on the tempo clock's beat
            let beats = (!node.params.contains_key("bpm")).then(|| beats(h));
            Box::new(Seq::new(node, rate, p("rate"), beats))
        }
        Kind::Click => Box::new(Click::new(node, rate, p("gain"), beats(h))),
        #[cfg(feature = "events")]
        Kind::Walk => Box::new(super::walk::Host::walk(node, rate, [p("spread"), p("rate"), p("glide")], beats(h), &mut h.events)?),
        #[cfg(feature = "events")]
        Kind::Events => Box::new(super::walk::Host::events(node, rate, [p("a"), p("b"), p("c"), p("d")], beats(h), &mut h.events)?),
        Kind::ChapterMusic => {
            let (tx, rx) = rtrb::RingBuffer::new(2);
            let (trash_tx, trash_rx) = rtrb::RingBuffer::new(2);
            h.music.push(MusicFeed { node: index, tracks: tx, status: Arc::new(Mutex::new("waiting".into())) });
            Box::new(Music {
                feed: rx,
                trash: trash_tx,
                _trash_rx: trash_rx,
                track: None,
                pos: 0.0,
                rate,
                gain: glide("gain"),
                looped: node.bool("loop", true),
            })
        }
        Kind::Mixer => Box::new(Mixer { gains: [glide("gain1"), glide("gain2"), glide("gain3"), glide("gain4")] }),
        Kind::Fft => {
            let (tx, rx) = rtrb::RingBuffer::new(TAP_RING);
            h.taps.push((index, rx));
            Box::new(FftTap { ring: tx })
        }
        Kind::AudioOut => Box::new(Out { volume: glide("volume") }),
        Kind::Out => Box::new(Out { volume: glide("gain") }),
        #[cfg(feature = "modules")]
        Kind::Module(_) => {
            let source = seams.modules.as_ref().ok_or_else(|| anyhow::anyhow!("node '{}': this engine has no module runtime", node.id))?;
            Box::new(super::module::ModuleBlock::new(node, rate, params, beats(h), source)?)
        }
        k => bail!("{k:?} is not an audio kind"),
    })
}

fn beats(h: &Handles) -> BeatReader {
    BeatReader { shared: h.tempo.clone(), frames: h.clock.frames.clone() }
}

/// The wavetables a build needs, made once per wave (a few ms each) and shared by the nodes that use them.
pub struct Tables {
    rate: f32,
    made: Vec<Arc<WaveTable>>,
}

impl Tables {
    pub fn new(rate: f32) -> Tables {
        Tables { rate, made: Vec::new() }
    }

    fn get(&mut self, wave: Wave) -> Arc<WaveTable> {
        if let Some(t) = self.made.iter().find(|t| t.wave() == wave) {
            return t.clone();
        }
        let t = Arc::new(WaveTable::new(wave, self.rate));
        self.made.push(t.clone());
        t
    }
}

/// A noise node's seed: its `seed` param (default 1) and its place in the graph, so two noise nodes never play the same
/// noise and a scene always plays the same (the seed rule, plan 3.4).
fn noise_seed(node: &Node, index: usize) -> u64 {
    (node.f32("seed", 1.0) as u64).wrapping_mul(0x0001_0000_0001).wrapping_add(index as u64)
}

fn write(out: &mut Buf, i: usize, v: f32) {
    out.put2(i, v, v);
}

/// attack / release on a gate (seconds from `attack` / `release`, defaults 5 ms / 150 ms); no gate = always open
struct Env {
    level: f32,
    att: f32,
    rel: f32,
}

/// the envelope's per-sample coefficients (attack, release) – the batched voices (voices.rs) use them too
pub(super) fn env_coefs(rate: f32, node: &Node) -> (f32, f32) {
    let coef = |t: f32| 1.0 - libm::expf(-1.0 / (t.max(1e-4) * rate));
    (coef(node.f32("attack", 0.005)), coef(node.f32("release", 0.15)))
}

impl Env {
    fn new(rate: f32, node: &Node) -> Env {
        let (att, rel) = env_coefs(rate, node);
        Env { level: 0.0, att, rel }
    }
    fn next(&mut self, gate: f32) -> f32 {
        let target = if gate > 0.5 { 1.0 } else { 0.0 };
        let c = if target > self.level { self.att } else { self.rel };
        self.level += (target - self.level) * c;
        self.level
    }
}

/// A control input read once per block that glides across it (#188, owner 2026-10-31: "no crackling"): deck-core's
/// Ramp from where the last block left it toward the Param's new value – sample i is `first + step · i` (`at`), the
/// arithmetic the batched voices use (voices.rs). Starts where the first block's value is.
pub(super) struct Glide {
    param: Param,
    ramp: Ramp,
    fresh: bool,
}

impl Glide {
    pub(super) fn new(param: Param, tau: f32, rate: f32) -> Glide {
        Glide { param, ramp: Ramp::new(0.0, tau, rate), fresh: true }
    }

    /// this block's (first value, step per sample)
    #[inline]
    pub(super) fn block(&mut self, n: usize) -> (f32, f32) {
        let x = self.param.get();
        if std::mem::take(&mut self.fresh) {
            self.ramp.jump(x);
        }
        self.ramp.block(x, n)
    }
}

/// a gliding input at sample `i`
#[inline(always)]
fn at((first, step): (f32, f32), i: usize) -> f32 {
    first + step * i as f32
}

/// (pitch Hz, gate) for frame i: from the cv input (a seq) or the (gliding) freq param with the gate open
fn pitch(cv: Option<&Buf>, freq: (f32, f32), i: usize) -> (f32, f32) {
    match cv {
        Some(c) => (c.ch(0)[i], c.ch(1)[i]),
        None => (at(freq, i), 1.0),
    }
}

/// an oscillator's wave: the sine (the batched voices' arithmetic, one lane) or a band-limited table
enum Shape {
    Sine { phase: f32 },
    Table { table: Arc<WaveTable>, osc: dsp::Osc },
}

struct Osc {
    shape: Shape,
    /// seconds per sample
    sd: f32,
    freq: Glide,
    gain: Glide,
    env: Env,
}

/// `wrap01` on one value – voices.rs's lanes do the same on eight
#[inline(always)]
fn wrap01(x: f32) -> f32 {
    x - round_fast(x - 0.5)
}

impl Block for Osc {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (freq, gain) = (self.freq.block(n), self.gain.block(n));
        for i in 0..n {
            let (f, gate) = pitch(ins[0], freq, i);
            let y = match &mut self.shape {
                Shape::Sine { phase } => {
                    let y = sin_turns(*phase);
                    *phase = wrap01(*phase + f * self.sd);
                    y
                }
                Shape::Table { table, osc } => osc.tick(table, f, self.sd),
            };
            write(&mut outs[0], i, y * self.env.next(gate) * at(gain, i));
        }
    }
}

/// the noise node's source: its colour
enum Source {
    White(White),
    Pink(Pink),
}

struct Noise {
    src: Source,
    filter: Svf,
    pi_over_rate: f32,
    cutoff: Glide,
    q: f32,
    gain: Glide,
}

impl Block for Noise {
    fn process(&mut self, _: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (cutoff, gain) = (self.cutoff.block(n), self.gain.block(n));
        for i in 0..n {
            let x = match &mut self.src {
                Source::White(w) => w.sample(),
                Source::Pink(p) => p.sample(),
            };
            let y = self.filter.lowpass(x, at(cutoff, i).clamp(20.0, 20_000.0), self.q, self.pi_over_rate);
            write(&mut outs[0], i, y * at(gain, i));
        }
    }
}

/// True frequency modulation (owner 2026-11-02, DECISIONS "fm is true frequency modulation"): the carrier's phase
/// accumulates the instantaneous frequency f + index · fm · sin(modulator) with fm = f · ratio; a negative instantaneous
/// frequency runs the phase backward (no `abs()` fold). voices.rs runs the same formula eight voices at a time.
struct Fm {
    carrier: f32,
    modulator: f32,
    /// seconds per sample
    sd: f32,
    freq: Glide,
    ratio: Glide,
    index: Glide,
    gain: Glide,
    env: Env,
}

impl Block for Fm {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (freq, ratio, index, gain) = (self.freq.block(n), self.ratio.block(n), self.index.block(n), self.gain.block(n));
        for i in 0..n {
            let (f, gate) = pitch(ins[0], freq, i);
            let fm = f * at(ratio, i);
            let mo = sin_turns(self.modulator);
            self.modulator = wrap01(self.modulator + fm * self.sd);
            let o = sin_turns(self.carrier);
            self.carrier = wrap01(self.carrier + (f + mo * at(index, i) * fm) * self.sd);
            write(&mut outs[0], i, o * self.env.next(gate) * at(gain, i));
        }
    }
}

/// `steps` = MIDI notes (0 = a rest), `division` steps per beat (4 = 16ths), `gate` = the note's share of a step.
/// On the tempo clock (no `bpm` param) step k plays from beat k / division – the steps snap to the beat, follow every
/// tempo change and stand while the clock is stopped; `rate` scales the division. With `bpm` it runs on its own.
struct Seq {
    hz: Vec<f32>,
    step: usize,
    phase: f32,
    held: f32,
    per_sample: f32,
    gate: f32,
    rate: Param,
    division: f64,
    beats: Option<BeatReader>,
}

impl Seq {
    fn new(node: &Node, sr: f32, rate: Param, beats: Option<BeatReader>) -> Seq {
        let transpose = node.f32("transpose", 0.0);
        let mut hz: Vec<f32> = node
            .f32s("steps")
            .iter()
            .map(|&n| if n <= 0.0 { 0.0 } else { 440.0 * libm::powf(2.0, (n + transpose - 69.0) / 12.0) })
            .collect();
        if hz.is_empty() {
            hz.push(0.0);
        }
        let held = hz.iter().copied().find(|&f| f > 0.0).unwrap_or(220.0);
        let steps_per_sec = node.f32("bpm", 120.0) / 60.0 * node.f32("division", 4.0);
        let division = node.f32("division", 4.0) as f64;
        Seq { hz, step: 0, phase: 0.0, held, per_sample: steps_per_sec / sr, gate: node.f32("gate", 0.5), rate, division, beats }
    }

    /// on the clock: the step and its phase from the beat of each frame
    fn synced(&mut self, outs: &mut [Buf], n: usize) -> bool {
        let Some((map, _, start)) = self.beats.as_ref().and_then(|b| b.block()) else { return false };
        let per_beat = self.division * self.rate.get().max(0.0) as f64;
        let len = self.hz.len() as f64;
        for i in 0..n {
            let s = map.beat_at(start + i as f64) * per_beat;
            let f = self.hz[s.floor().rem_euclid(len) as usize];
            if f > 0.0 {
                self.held = f;
            }
            let gate = if f > 0.0 && map.running && (s.rem_euclid(1.0) as f32) < self.gate { 1.0 } else { 0.0 };
            outs[0].put2(i, self.held, gate);
        }
        true
    }
}

impl Block for Seq {
    fn process(&mut self, _: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        if self.synced(outs, n) {
            return;
        }
        let inc = self.per_sample * self.rate.get().max(0.0);
        for i in 0..n {
            self.phase += inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.step = (self.step + 1) % self.hz.len();
            }
            let f = self.hz[self.step];
            if f > 0.0 {
                self.held = f;
            }
            let gate = if f > 0.0 && self.phase < self.gate { 1.0 } else { 0.0 };
            outs[0].put2(i, self.held, gate);
        }
    }
}

/// plays the track its loader sends (silence until then), linear resampling to the device rate, looped by default
struct Music {
    feed: rtrb::Consumer<Arc<Track>>,
    /// a replaced track goes back out instead of being freed in the callback
    trash: rtrb::Producer<Arc<Track>>,
    _trash_rx: rtrb::Consumer<Arc<Track>>,
    track: Option<Arc<Track>>,
    pos: f64,
    rate: f32,
    gain: Glide,
    looped: bool,
}

impl Block for Music {
    fn process(&mut self, _: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        if let Ok(t) = self.feed.pop() {
            if let Some(old) = self.track.replace(t) {
                let _ = self.trash.push(old);
            }
            self.pos = 0.0;
        }
        let out = &mut outs[0];
        let Some(t) = self.track.as_deref() else {
            out.clear(n);
            return;
        };
        let (len, step, gain) = (t.l.len(), t.rate as f64 / self.rate as f64, self.gain.block(n));
        for i in 0..n {
            let at = self.pos as usize;
            if at + 1 >= len {
                if self.looped && len > 1 {
                    self.pos = 0.0;
                } else {
                    out.put2(i, 0.0, 0.0);
                    continue;
                }
            }
            let at = self.pos as usize;
            let frac = (self.pos - at as f64) as f32;
            let g = gain.0 + gain.1 * i as f32;
            out.put2(i, (t.l[at] + (t.l[at + 1] - t.l[at]) * frac) * g, (t.r[at] + (t.r[at + 1] - t.r[at]) * frac) * g);
            self.pos += step;
        }
    }
}

struct Mixer {
    gains: [Glide; 4],
}

impl Block for Mixer {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let out = &mut outs[0];
        out.clear(n);
        for (k, g) in self.gains.iter_mut().enumerate() {
            let Some(src) = ins[k] else { continue };
            let (g, dg) = g.block(n);
            // over slices, not indices: no bounds check per sample, so the loops vectorise
            for c in 0..out.channels().min(src.channels()) {
                for (i, (o, x)) in out.ch_mut(c)[..n].iter_mut().zip(&src.ch(c)[..n]).enumerate() {
                    *o += x * (g + dg * i as f32);
                }
            }
        }
    }
}

/// hands the mono sum to the frame thread's analyzer; a full ring drops samples (the analyzer is behind, not the audio)
struct FftTap {
    ring: rtrb::Producer<f32>,
}

impl Block for FftTap {
    fn process(&mut self, ins: &[Option<&Buf>], _: &mut [Buf], n: usize) {
        let Some(src) = ins[0] else { return };
        let (l, r) = (src.ch(0), src.ch(1));
        for i in 0..n {
            if self.ring.push(0.5 * (l[i] + r[i])).is_err() {
                break;
            }
        }
    }
}

struct Out {
    volume: Glide,
}

impl Block for Out {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let out = &mut outs[0];
        let Some(src) = ins[0] else {
            out.clear(n);
            return;
        };
        let (v, dv) = self.volume.block(n);
        for c in 0..out.channels().min(src.channels()) {
            for (i, (o, x)) in out.ch_mut(c)[..n].iter_mut().zip(&src.ch(c)[..n]).enumerate() {
                *o = x * (v + dv * i as f32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;
    use deck_space::graph::Graph;
    use deck_space::scene::Scene;

    fn build(text: &str) -> (Engine, Handles) {
        Engine::build(&Graph::build(&Scene::parse(text).unwrap()).unwrap(), 48_000.0, 1.0).unwrap()
    }

    #[test]
    fn the_sequencer_steps_and_gates() {
        let mut s = Seq::new(
            &Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"seq\"\nbpm=60\ndivision=1\nsteps=[69, 0, 81]\n").unwrap())
                .unwrap()
                .nodes[0],
            100.0,
            Param::new(1.0),
            None,
        );
        let mut out = [Buf::default()];
        s.process(&[], &mut out, 300);
        assert_eq!(out[0].ch(0)[10], 440.0); // step 0: A4, gate open in the first half
        assert_eq!(out[0].ch(1)[10], 1.0);
        assert_eq!(out[0].ch(1)[70], 0.0); // second half of the step: gate closed
        assert_eq!(out[0].ch(0)[150], 440.0); // the rest holds the last pitch, gate closed
        assert_eq!(out[0].ch(1)[150], 0.0);
        assert_eq!(out[0].ch(0)[210], 880.0); // A5
    }

    #[test]
    fn the_sequencer_on_the_clock_snaps_to_the_beat() {
        use deck_core::tempo::{Shared, TempoMap};
        use std::sync::atomic::{AtomicU64, Ordering};
        let node = &Graph::build(&Scene::parse("[[node]]
id=\"s\"
kind=\"seq\"
division=2
gate=0.5
steps=[69, 81]
").unwrap())
            .unwrap()
            .nodes[0];
        let shared = Arc::new(Shared::default());
        let frames = Arc::new(AtomicU64::new(0));
        // 60 BPM at 100 Hz: a beat = 100 frames, a step (division 2) = 50
        shared.publish(&TempoMap::new(60.0, 4, 100.0), false);
        let mut s = Seq::new(node, 100.0, Param::new(1.0), Some(BeatReader { shared: shared.clone(), frames: frames.clone() }));
        let mut out = [Buf::default()];
        s.process(&[], &mut out, 200);
        assert_eq!((out[0].ch(0)[0], out[0].ch(1)[0]), (440.0, 1.0), "step 0 on beat 0");
        assert_eq!(out[0].ch(1)[24], 1.0);
        assert_eq!(out[0].ch(1)[25], 0.0, "the gate is half a step");
        assert_eq!((out[0].ch(0)[49], out[0].ch(0)[50]), (440.0, 880.0), "step 1 exactly at frame 50");
        assert_eq!(out[0].ch(0)[100], 440.0, "beat 1: the pattern again");
        // 120 BPM from frame 200 (beat 2): steps every 25 frames
        frames.store(200, Ordering::Relaxed);
        shared.publish(&TempoMap::new(60.0, 4, 100.0).with_bpm(200.0, 120.0), false);
        s.process(&[], &mut out, 100);
        assert_eq!((out[0].ch(0)[24], out[0].ch(0)[25], out[0].ch(0)[50]), (440.0, 880.0, 440.0));
        // stopped: the gate closes, the pitch holds
        frames.store(300, Ordering::Relaxed);
        shared.publish(&TempoMap::new(60.0, 4, 100.0).with_bpm(200.0, 120.0).with_running(300.0, false), false);
        s.process(&[], &mut out, 100);
        assert!(out[0].ch(1)[..100].iter().all(|g| *g == 0.0));
    }

    #[test]
    fn music_plays_a_sent_track_and_loops() {
        let (mut e, mut h) = build(
            "[[node]]\nid=\"m\"\nkind=\"chapter_music\"\ntoon=\"x\"\nchapter=1\n\
             [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"m\"\nto=\"out.in\"\n",
        );
        let mut buf = vec![0.0f32; 200];
        e.render(&mut buf, 2, |x| x);
        assert!(buf.iter().all(|x| *x == 0.0), "silent before the track arrives");
        let t = Track { rate: 48_000.0, l: vec![0.25; 64], r: vec![-0.25; 64] };
        h.music[0].tracks.push(Arc::new(t)).ok().unwrap();
        e.render(&mut buf, 2, |x| x);
        // out after the limiter's look-ahead (#188); the first call took 100 frames of a 128-frame quantum, so the track
        // (read at the next quantum) starts 28 frames into this one
        let first = 28 + deck_core::limiter::LOOK;
        assert_eq!((buf[2 * first - 2], buf[2 * first], buf[2 * first + 1]), (0.0, 0.25, -0.25));
        assert_eq!((buf[198], buf[199]), (0.25, -0.25), "looped past the 64-frame track");
    }

    #[test]
    fn the_mixer_sums_with_gains_and_fm_and_noise_sound() {
        let (mut e, _) = build(
            "[[node]]\nid=\"f\"\nkind=\"fm\"\nfreq=110\ngain=0.5\n[[node]]\nid=\"n\"\nkind=\"noise\"\ncolor=\"pink\"\ngain=0.5\n\
             [[node]]\nid=\"x\"\nkind=\"mixer\"\ngain2=0\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
             [[link]]\nfrom=\"f\"\nto=\"x.in1\"\n[[link]]\nfrom=\"n\"\nto=\"x.in2\"\n[[link]]\nfrom=\"x\"\nto=\"out.in\"\n",
        );
        let mut buf = vec![0.0f32; 9600];
        e.render(&mut buf, 2, |x| x);
        let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.2 && peak <= 0.5 + 1e-3, "fm alone (noise at gain 0): peak {peak}");
    }

    #[test]
    fn unknown_waves_are_errors() {
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"o\"\nkind=\"osc\"\nwave=\"wobble\"\n").unwrap()).unwrap();
        assert!(Engine::build(&g, 48_000.0, 1.0).is_err());
    }
}
