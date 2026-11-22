//! `convolver` – a convolution reverb: the stereo input through an impulse response, the sound of a real room (or a
//! spring, a plate, a cabinet) recorded as a click's echo. The response is the sound asset in the slot `ir` (mono or
//! stereo, up to 4 s); in `init` it is resampled to the deck's rate, cut to 4 s and normalised (the louder channel's
//! energy 1), and its partitions of 128 frames are transformed once. Without an asset (a deck binds none yet:
//! TODO(#782)) the built-in room plays: a second of seeded noise falling 60 dB in 0.8 s, two sides apart.
//!
//! The convolution is the SDK's partitioned one (`dsp::conv::Split`: the first 2048 frames of the response in partitions
//! of 128, the rest in partitions of 1024), a block of 128 frames at a time, so the whole
//! output – the dry signal too – is exactly 128 frames late, the latency the descriptor declares for the engine to
//! compensate. `length` cuts the response while it plays, `predelay` holds the input back before the response, `gain`
//! is the level of the wet, `mix` the wet against the dry. The knobs glide on the module's own clock (a render does not
//! depend on how the host cuts the blocks); with nothing linked to the input and the tail run out, it writes silence and
//! costs nothing.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p convolver-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/convolver_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::conv::Split;
use deck_module::dsp::delay::Delay;
use deck_module::dsp::noise::White;
use deck_module::dsp::{flush, level};
use deck_module::{math, Block, Ctx, Dirty, Module, Smooth};

/// the partition and the latency, frames
pub const PART: usize = 128;
/// the partition of the response past its first `2 · TAIL_PART` frames
pub const TAIL_PART: usize = 1024;
/// the longest response, seconds
pub const MAX_SECONDS: f64 = 4.0;
/// the longest pre-delay, milliseconds
pub const MAX_PREDELAY_MS: f64 = 250.0;
/// samples between two steps of the glides
const CTRL: usize = 16;
const SLEW: f32 = 0.02;
/// this close a glide has arrived: a mix at 0 is the dry signal exactly
const ARRIVED: f32 = 1e-5;
const SAMPLE_ARRIVED: f32 = 0.05;
/// the built-in room: its length and its RT60, seconds
const ROOM_SECONDS: f64 = 1.0;
const ROOM_RT60: f64 = 0.8;

#[derive(Module)]
#[module(name = "convolver", latency = 128, doc = "a convolution reverb: the input through an impulse response from a sound asset, 128 frames late")]
#[port(input, audio, name = "in", channels = 2, doc = "the signal (a mono cable feeds both sides)")]
#[port(output, audio, name = "out", channels = 2, doc = "the dry signal and the convolved one, by mix, 128 frames late")]
#[asset(name = "ir", kind = "pcm", max_bytes = 3145728, doc = "the impulse response: a sound, mono or stereo, up to 4 s (at most 4 s stereo at 96 kHz); none plays the built-in room")]
#[preset(name = "room", doc = "The response alone, for a send bus: put the room on a snare.", values(mix = 100.0))]
pub struct Convolver {
    #[param(range = 0.0..100.0, default = 30.0, unit = "%", doc = "the convolved signal against the dry: 0 dry only, 100 convolved only")]
    pub mix: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "the level of the convolved signal")]
    pub gain: f32,
    #[param(range = 0.0..250.0, default = 0.0, unit = "ms", doc = "the gap between the sound and its response")]
    pub predelay: f32,
    #[param(range = 0.05..4.0, default = 4.0, unit = "s", doc = "the response cut to this length")]
    pub length: f32,
    /// built in `init`
    conv: Vec<Split>,
    pre: Vec<Delay>,
    /// per side, `PART` samples each: the predelayed input of this block, the convolution of the last one, the dry
    /// input of this block and of the last one
    x: Vec<f32>,
    wet: Vec<f32>,
    raw: Vec<f32>,
    raw_last: Vec<f32>,
    fill: usize,
    glide: Glide,
    clock: usize,
    quiet: usize,
    drained: usize,
}

#[derive(Default)]
struct Glide {
    wet: Smooth,
    gain: Smooth,
    pre: Smooth,
    slew: f32,
    rate: f64,
}

impl Glide {
    fn all(&mut self) -> [&mut Smooth; 3] {
        [&mut self.wet, &mut self.gain, &mut self.pre]
    }

    fn control(&mut self) {
        for (s, within) in [(&mut self.wet, ARRIVED), (&mut self.gain, ARRIVED), (&mut self.pre, SAMPLE_ARRIVED)] {
            s.step(self.slew);
            s.land(within);
        }
    }
}

/// The response as the convolution takes it, two sides: `channels` (planar, a mono one plays on both sides) at `from`
/// Hz resampled to `to` Hz (a cubic through the four samples around each point), cut to `MAX_SECONDS`, the louder
/// side's energy made 1 (a silent response stays silent), what is too small to hear set to 0. Allocates.
pub fn prepare(channels: &[&[f32]], from: f64, to: f64) -> [Vec<f32>; 2] {
    let max = (MAX_SECONDS * to) as usize;
    let side = |c: usize| -> Vec<f32> {
        let Some(src) = channels.get(c.min(channels.len().saturating_sub(1))) else { return Vec::new() };
        if from <= 0.0 || to <= 0.0 || src.is_empty() || from == to {
            return src[..src.len().min(max)].to_vec();
        }
        let step = from / to;
        let len = (((src.len() - 1) as f64 / step) as usize + 1).min(max);
        let at = |i: isize| src[i.clamp(0, src.len() as isize - 1) as usize];
        (0..len)
            .map(|j| {
                let t = j as f64 * step;
                let i = t as isize;
                let f = (t - i as f64) as f32;
                let (a, b, c, e) = (at(i - 1), at(i), at(i + 1), at(i + 2));
                let c1 = 0.5 * (c - a);
                let c2 = a - 2.5 * b + 2.0 * c - 0.5 * e;
                let c3 = 0.5 * (e - a) + 1.5 * (b - c);
                ((c3 * f + c2) * f + c1) * f + b
            })
            .collect()
    };
    let mut sides = [side(0), side(1)];
    let energy = sides.iter().map(|s| s.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>()).fold(0.0, f64::max);
    if energy > 0.0 {
        let k = (1.0 / math::libm::sqrt(energy)) as f32;
        for v in sides.iter_mut().flatten() {
            *v = flush(*v * k);
        }
    }
    sides
}

/// the built-in room at `rate`: seeded noise, a side each, falling 60 dB in `ROOM_RT60`
pub fn room(rate: f64) -> [Vec<f32>; 2] {
    let n = (ROOM_SECONDS * rate) as usize;
    // per sample the factor that makes 60 dB in RT60: 10^(−3 / (RT60 · rate))
    let k = math::libm::exp(-3.0 * core::f64::consts::LN_10 / (ROOM_RT60 * rate)) as f32;
    [0x6c65_6674u64, 0x7269_6768].map(|seed| {
        let (mut w, mut env) = (White::new(seed), 1.0f32);
        (0..n)
            .map(|_| {
                let v = w.sample() * env;
                env *= k;
                v
            })
            .collect()
    })
}

impl Convolver {
    fn aim(&mut self) {
        let g = &mut self.glide;
        g.wet.to(self.mix.clamp(0.0, 100.0) / 100.0);
        g.gain.to(level::gain(self.gain.clamp(-60.0, 12.0)));
        g.pre.to((f64::from(self.predelay.clamp(0.0, MAX_PREDELAY_MS as f32)) * g.rate / 1000.0) as f32);
        let samples = (f64::from(self.length.clamp(0.05, MAX_SECONDS as f32)) * g.rate) as usize;
        for c in self.conv.iter_mut() {
            c.set_length(samples);
        }
    }

    fn clear(&mut self) {
        self.conv.iter_mut().for_each(Split::clear);
        self.pre.iter_mut().for_each(Delay::clear);
        for b in [&mut self.x, &mut self.wet, &mut self.raw, &mut self.raw_last] {
            b.fill(0.0);
        }
    }
}

impl Module for Convolver {
    fn init(&mut self, ctx: &Ctx) {
        let rate = ctx.rate().max(1.0);
        let ir = match ctx.asset(Self::IR) {
            Some(a) if a.channels > 0 && a.frames > 0 => {
                let chans = [a.pcm(0), a.pcm(1)];
                prepare(&chans[..(a.channels as usize).min(2)], f64::from(a.rate), rate)
            }
            _ => {
                let [l, r] = room(rate);
                prepare(&[&l, &r], rate, rate)
            }
        };
        self.conv = ir.iter().map(|h| Split::new(h, PART, TAIL_PART)).collect();
        let pre = (MAX_PREDELAY_MS * rate / 1000.0) as usize + 2;
        self.pre = vec![Delay::new(pre), Delay::new(pre)];
        for b in [&mut self.x, &mut self.wet, &mut self.raw, &mut self.raw_last] {
            *b = vec![0.0; 2 * PART];
        }
        self.drained = self.conv[0].len() + 2 * TAIL_PART + pre + CTRL;
        let g = &mut self.glide;
        g.rate = rate;
        g.slew = Smooth::coef(SLEW, CTRL as f32 / rate as f32);
        self.aim();
        self.glide.all().into_iter().for_each(|s| s.jump(s.target()));
    }

    fn params_changed(&mut self, _: Dirty) {
        self.aim();
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let n = block.frames();
        if self.conv.len() != 2 {
            block.set_silent(Self::OUT);
            return;
        }
        if block.in_silent(Self::IN) && self.quiet >= self.drained {
            if self.quiet != usize::MAX {
                self.clear();
                self.quiet = usize::MAX;
            }
            self.clock = self.clock.wrapping_add(n);
            self.fill = (self.fill + n) % PART;
            block.set_silent(Self::OUT);
            return;
        }
        let (inl, inr) = block.audio_in(Self::IN).stereo();
        let mut out = block.audio_out(Self::OUT);
        let (outl, outr) = out.stereo();
        let mono = outr.is_empty();
        let g = &mut self.glide;
        for i in 0..n {
            if self.clock % CTRL == 0 {
                g.control();
            }
            let p = self.fill;
            let (wet, k) = (g.wet.value(), g.gain.value());
            let mut y = [0.0f32; 2];
            for (c, x) in [inl[i], inr[i]].into_iter().enumerate() {
                let at = c * PART + p;
                let line = &mut self.pre[c];
                let pre = g.pre.value();
                let delayed = if pre < 1.0 { x } else { line.read_frac(pre) };
                line.write(x);
                self.x[at] = delayed;
                y[c] = self.raw_last[at] * (1.0 - wet) + self.wet[at] * k * wet;
                self.raw[at] = x;
            }
            if mono {
                if let Some(o) = outl.get_mut(i) {
                    *o = 0.5 * (y[0] + y[1]);
                }
            } else {
                outl[i] = y[0];
                outr[i] = y[1];
            }
            let still = inl[i] == 0.0 && inr[i] == 0.0 && y[0] == 0.0 && y[1] == 0.0;
            self.quiet = if still { self.quiet.saturating_add(1).min(usize::MAX - 1) } else { 0 };
            self.clock = self.clock.wrapping_add(1);
            self.fill += 1;
            if self.fill == PART {
                self.fill = 0;
                for (c, conv) in self.conv.iter_mut().enumerate() {
                    let side = c * PART..(c + 1) * PART;
                    conv.block(&self.x[side.clone()], &mut self.wet[side]);
                }
                self.raw_last.copy_from_slice(&self.raw);
            }
        }
    }
}
