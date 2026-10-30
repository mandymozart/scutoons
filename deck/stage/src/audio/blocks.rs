//! The audio kinds' blocks – the patcher model: one block per node (#183). The batch runtime runs sine oscillators and
//! FM voices as batches (voices.rs) and everything else as these blocks; a patcher build (`Build::patcher`, the tests'
//! and the bench's reference) runs every node as a block. Oscillators, noise and the filter are fundsp units (built and
//! warmed up here, outside the callback); the wiring, envelopes, sequencer, player, mixer and FFT tap are ours. Nothing
//! in `process` allocates.
use super::click::Click;
use super::{Block, Handles, MusicFeed, Param, Stereo, Track};
use crate::graph::{Kind, Node};
use crate::tempo::BeatReader;
use anyhow::{bail, Result};
use fundsp::hacker32::{lowpass, noise, pink, saw, sine, square, triangle, AudioUnit};
use std::sync::{Arc, Mutex};

/// how many samples the FFT ring holds (≈ 170 ms at 48 kHz; the frame thread drains it every frame)
const TAP_RING: usize = 8192;

/// a control input's value when neither the node nor a link sets it
pub fn default_for(kind: Kind, port: &str) -> f32 {
    match (kind, port) {
        (_, "gain" | "gain1" | "gain2" | "gain3" | "gain4" | "rate") => 1.0,
        (Kind::AudioOut, "volume") => 0.8,
        (_, "freq") => 220.0,
        (Kind::Noise, "cutoff") => 4000.0,
        (Kind::Fm, "ratio") => 2.0,
        (Kind::Fm, "index") => 1.0,
        _ => 0.0,
    }
}

pub fn make(node: &Node, index: usize, rate: f32, params: &[Param], h: &mut Handles) -> Result<Box<dyn Block>> {
    let p = |name: &str| params[node.kind.input(name).expect("a port of this kind")].clone();
    Ok(match node.kind {
        Kind::Osc => {
            let unit: Box<dyn AudioUnit> = match node.str("wave").unwrap_or("sine") {
                "sine" => Box::new(sine()),
                "saw" => Box::new(saw()),
                "square" => Box::new(square()),
                "triangle" => Box::new(triangle()),
                w => bail!("node '{}': unknown wave '{w}' (sine, saw, square, triangle)", node.id),
            };
            Box::new(Osc { unit: warm(unit, rate, 1), freq: p("freq"), gain: p("gain"), env: Env::new(rate, node) })
        }
        Kind::Noise => {
            let src: Box<dyn AudioUnit> = match node.str("color").unwrap_or("white") {
                "white" => Box::new(noise()),
                "pink" => Box::new(pink()),
                c => bail!("node '{}': unknown color '{c}' (white, pink)", node.id),
            };
            Box::new(Noise {
                src: warm(src, rate, 0),
                filter: warm(Box::new(lowpass()), rate, 3),
                cutoff: p("cutoff"),
                q: node.f32("q", 0.7),
                gain: p("gain"),
            })
        }
        Kind::Fm => Box::new(Fm {
            carrier: warm(Box::new(sine()), rate, 1),
            modulator: warm(Box::new(sine()), rate, 1),
            freq: p("freq"),
            ratio: p("ratio"),
            index: p("index"),
            gain: p("gain"),
            env: Env::new(rate, node),
        }),
        Kind::Seq => {
            // a `bpm` of its own: free-running; else on the tempo clock's beat
            let beats = (!node.params.contains_key("bpm")).then(|| beats(h));
            Box::new(Seq::new(node, rate, p("rate"), beats))
        }
        Kind::Click => Box::new(Click::new(node, rate, p("gain"), beats(h))),
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
                gain: p("gain"),
                looped: node.bool("loop", true),
            })
        }
        Kind::Mixer => Box::new(Mixer { gains: [p("gain1"), p("gain2"), p("gain3"), p("gain4")] }),
        Kind::Fft => {
            let (tx, rx) = rtrb::RingBuffer::new(TAP_RING);
            h.taps.push((index, rx));
            Box::new(FftTap { ring: tx })
        }
        Kind::AudioOut => Box::new(Out { volume: p("volume") }),
        k => bail!("{k:?} is not an audio kind"),
    })
}

fn beats(h: &Handles) -> BeatReader {
    BeatReader { shared: h.tempo.clone(), frames: h.clock.frames.clone() }
}

/// fundsp units set up outside the callback: sample rate, state reset, one tick so lazy tables exist before RT
fn warm(mut unit: Box<dyn AudioUnit>, rate: f32, inputs: usize) -> Box<dyn AudioUnit> {
    unit.set_sample_rate(rate as f64);
    unit.reset();
    let ins = [220.0, 1000.0, 0.7];
    let mut out = [0.0; 2];
    unit.tick(&ins[..inputs], &mut out[..unit.outputs()]);
    unit.reset();
    unit
}

fn write(out: &mut Stereo, i: usize, v: f32) {
    out.l[i] = v;
    out.r[i] = v;
}

/// attack / release on a gate (seconds from `attack` / `release`, defaults 5 ms / 150 ms); no gate = always open
struct Env {
    level: f32,
    att: f32,
    rel: f32,
}

/// the envelope's per-sample coefficients (attack, release) – the batched voices (voices.rs) use them too
pub(super) fn env_coefs(rate: f32, node: &Node) -> (f32, f32) {
    let coef = |t: f32| 1.0 - (-1.0 / (t.max(1e-4) * rate)).exp();
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

/// (pitch Hz, gate) for frame i: from the cv input (a seq) or the freq param with the gate open
fn pitch(cv: Option<&Stereo>, freq: f32, i: usize) -> (f32, f32) {
    match cv {
        Some(c) => (c.l[i], c.r[i]),
        None => (freq, 1.0),
    }
}

struct Osc {
    unit: Box<dyn AudioUnit>,
    freq: Param,
    gain: Param,
    env: Env,
}

impl Block for Osc {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let (freq, gain) = (self.freq.get(), self.gain.get());
        let mut o = [0.0];
        for i in 0..n {
            let (f, gate) = pitch(ins[0], freq, i);
            self.unit.tick(&[f], &mut o);
            write(&mut outs[0], i, o[0] * self.env.next(gate) * gain);
        }
    }
}

struct Noise {
    src: Box<dyn AudioUnit>,
    filter: Box<dyn AudioUnit>,
    cutoff: Param,
    q: f32,
    gain: Param,
}

impl Block for Noise {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let (cutoff, gain) = (self.cutoff.get().clamp(20.0, 20_000.0), self.gain.get());
        let (mut x, mut y) = ([0.0], [0.0]);
        for i in 0..n {
            self.src.tick(&[], &mut x);
            self.filter.tick(&[x[0], cutoff, self.q], &mut y);
            write(&mut outs[0], i, y[0] * gain);
        }
    }
}

struct Fm {
    carrier: Box<dyn AudioUnit>,
    modulator: Box<dyn AudioUnit>,
    freq: Param,
    ratio: Param,
    index: Param,
    gain: Param,
    env: Env,
}

impl Block for Fm {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let (freq, ratio, index, gain) = (self.freq.get(), self.ratio.get(), self.index.get(), self.gain.get());
        let (mut m, mut c) = ([0.0], [0.0]);
        for i in 0..n {
            let (f, gate) = pitch(ins[0], freq, i);
            let fm = f * ratio;
            self.modulator.tick(&[fm], &mut m);
            self.carrier.tick(&[(f + m[0] * index * fm).abs()], &mut c);
            write(&mut outs[0], i, c[0] * self.env.next(gate) * gain);
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
            .map(|&n| if n <= 0.0 { 0.0 } else { 440.0 * 2f32.powf((n + transpose - 69.0) / 12.0) })
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
    fn synced(&mut self, outs: &mut [Stereo], n: usize) -> bool {
        let Some((map, _, start)) = self.beats.as_ref().and_then(|b| b.block()) else { return false };
        let per_beat = self.division * self.rate.get().max(0.0) as f64;
        let len = self.hz.len() as f64;
        for i in 0..n {
            let s = map.beat_at(start + i as f64) * per_beat;
            let f = self.hz[s.floor().rem_euclid(len) as usize];
            if f > 0.0 {
                self.held = f;
            }
            outs[0].l[i] = self.held;
            outs[0].r[i] = if f > 0.0 && map.running && (s.rem_euclid(1.0) as f32) < self.gate { 1.0 } else { 0.0 };
        }
        true
    }
}

impl Block for Seq {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
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
            outs[0].l[i] = self.held;
            outs[0].r[i] = if f > 0.0 && self.phase < self.gate { 1.0 } else { 0.0 };
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
    gain: Param,
    looped: bool,
}

impl Block for Music {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        if let Ok(t) = self.feed.pop() {
            if let Some(old) = self.track.replace(t) {
                let _ = self.trash.push(old);
            }
            self.pos = 0.0;
        }
        let out = &mut outs[0];
        let Some(t) = self.track.as_deref() else {
            out.l[..n].fill(0.0);
            out.r[..n].fill(0.0);
            return;
        };
        let (len, step, gain) = (t.l.len(), t.rate as f64 / self.rate as f64, self.gain.get());
        for i in 0..n {
            let at = self.pos as usize;
            if at + 1 >= len {
                if self.looped && len > 1 {
                    self.pos = 0.0;
                } else {
                    out.l[i] = 0.0;
                    out.r[i] = 0.0;
                    continue;
                }
            }
            let at = self.pos as usize;
            let frac = (self.pos - at as f64) as f32;
            out.l[i] = (t.l[at] + (t.l[at + 1] - t.l[at]) * frac) * gain;
            out.r[i] = (t.r[at] + (t.r[at + 1] - t.r[at]) * frac) * gain;
            self.pos += step;
        }
    }
}

struct Mixer {
    gains: [Param; 4],
}

impl Block for Mixer {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let out = &mut outs[0];
        out.l[..n].fill(0.0);
        out.r[..n].fill(0.0);
        for (k, g) in self.gains.iter().enumerate() {
            let Some(src) = ins[k] else { continue };
            let g = g.get();
            // over slices, not indices: no bounds check per sample, so the loops vectorise
            for (o, x) in out.l[..n].iter_mut().zip(&src.l[..n]) {
                *o += x * g;
            }
            for (o, x) in out.r[..n].iter_mut().zip(&src.r[..n]) {
                *o += x * g;
            }
        }
    }
}

/// hands the mono sum to the frame thread's analyzer; a full ring drops samples (the analyzer is behind, not the audio)
struct FftTap {
    ring: rtrb::Producer<f32>,
}

impl Block for FftTap {
    fn process(&mut self, ins: &[Option<&Stereo>], _: &mut [Stereo], n: usize) {
        let Some(src) = ins[0] else { return };
        for i in 0..n {
            if self.ring.push(0.5 * (src.l[i] + src.r[i])).is_err() {
                break;
            }
        }
    }
}

struct Out {
    volume: Param,
}

impl Block for Out {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let out = &mut outs[0];
        let Some(src) = ins[0] else {
            out.l[..n].fill(0.0);
            out.r[..n].fill(0.0);
            return;
        };
        let v = self.volume.get();
        for (o, x) in out.l[..n].iter_mut().zip(&src.l[..n]) {
            *o = x * v;
        }
        for (o, x) in out.r[..n].iter_mut().zip(&src.r[..n]) {
            *o = x * v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Engine;
    use crate::graph::Graph;
    use crate::scene::Scene;

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
        let mut out = [Stereo::new()];
        s.process(&[], &mut out, 300);
        assert_eq!(out[0].l[10], 440.0); // step 0: A4, gate open in the first half
        assert_eq!(out[0].r[10], 1.0);
        assert_eq!(out[0].r[70], 0.0); // second half of the step: gate closed
        assert_eq!(out[0].l[150], 440.0); // the rest holds the last pitch, gate closed
        assert_eq!(out[0].r[150], 0.0);
        assert_eq!(out[0].l[210], 880.0); // A5
    }

    #[test]
    fn the_sequencer_on_the_clock_snaps_to_the_beat() {
        use crate::tempo::{Shared, TempoMap};
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
        let mut out = [Stereo::new()];
        s.process(&[], &mut out, 200);
        assert_eq!((out[0].l[0], out[0].r[0]), (440.0, 1.0), "step 0 on beat 0");
        assert_eq!(out[0].r[24], 1.0);
        assert_eq!(out[0].r[25], 0.0, "the gate is half a step");
        assert_eq!((out[0].l[49], out[0].l[50]), (440.0, 880.0), "step 1 exactly at frame 50");
        assert_eq!(out[0].l[100], 440.0, "beat 1: the pattern again");
        // 120 BPM from frame 200 (beat 2): steps every 25 frames
        frames.store(200, Ordering::Relaxed);
        shared.publish(&TempoMap::new(60.0, 4, 100.0).with_bpm(200.0, 120.0), false);
        s.process(&[], &mut out, 100);
        assert_eq!((out[0].l[24], out[0].l[25], out[0].l[50]), (440.0, 880.0, 440.0));
        // stopped: the gate closes, the pitch holds
        frames.store(300, Ordering::Relaxed);
        shared.publish(&TempoMap::new(60.0, 4, 100.0).with_bpm(200.0, 120.0).with_running(300.0, false), false);
        s.process(&[], &mut out, 100);
        assert!(out[0].r[..100].iter().all(|g| *g == 0.0));
    }

    #[cfg(feature = "chapter")]
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
        assert_eq!((buf[0], buf[1]), (0.25, -0.25));
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
