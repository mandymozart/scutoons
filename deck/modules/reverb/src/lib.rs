//! `reverb` – an algorithmic reverb: the stereo input, after a pre-delay, into the SDK's eight-line feedback delay
//! network (a Householder mix, lines of prime length), the left channel into four lines and the right into the other
//! four. `decay` is the time the tail takes to fall 60 dB below the low frequencies; `damp` is the lowpass in every
//! loop, so the highs die sooner (at the top of its range there is none). `size` scales the lines from a small room to a
//! hall; `width` goes from a mono tail to the full stereo one; `mix` sets the wet against the dry.
//!
//! The knobs glide, the loop gains and lengths are recomputed every sixteen samples of the module's own clock (a render
//! does not depend on how the host cuts the blocks), and a changed size glides the lines to their new prime lengths.
//! With nothing linked to the input and the tail run out, it writes silence and costs nothing.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p reverb-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/reverb_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::{exp2, log2};
use deck_module::dsp::delay::Delay;
use deck_module::dsp::fdn::{self, Fdn, LINES};
use deck_module::dsp::onepole::OnePole;
use deck_module::{Block, Ctx, Dirty, Module, Smooth};

/// samples between two updates of the gains and lengths
const CTRL: usize = 16;
/// the line lengths at `size` 100 %, milliseconds
const BASE_MS: [f64; LINES] = [43.3, 49.1, 55.7, 61.9, 68.3, 75.1, 83.9, 93.7];
/// the longest pre-delay, milliseconds
pub const MAX_PREDELAY_MS: f64 = 250.0;
/// the time the knobs take to arrive, and the slower glide of the lengths (a faster one bends the pitch), seconds
const SLEW: f32 = 0.02;
const SLEW_LEN: f32 = 0.1;
/// this close a glide has arrived (−100 dB): a mix at 0 is the dry signal exactly
const ARRIVED: f32 = 1e-5;
/// and a length in samples: a one-pole in f32 stalls up to a few hundredths short of thousands; there it is whole
const SAMPLE_ARRIVED: f32 = 0.05;
/// which lines a channel feeds (left the even ones, right the odd ones) and with which sign: each four sum to zero
const SIGN: [f32; LINES] = [1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0];
/// the two output taps: orthogonal to each other and to the Householder's own direction
const TAP_L: [f32; LINES] = [1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0];
const TAP_R: [f32; LINES] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
const IN_GAIN: f32 = 0.5;
const OUT_GAIN: f32 = 0.35;
/// at this cutoff and above there is no damping
const NO_DAMP_HZ: f32 = 19_999.0;

#[derive(Module)]
#[module(name = "reverb", doc = "an algorithmic stereo reverb: an eight-line feedback delay network with pre-delay, damping and width")]
#[port(input, audio, name = "in", channels = 2, doc = "the signal (a mono cable feeds both sides)")]
#[port(output, audio, name = "out", channels = 2, doc = "the dry signal and the reverb, by mix")]
#[preset(name = "hall", doc = "A large, bright-but-soft hall with a short gap before the tail: put it on a send at full mix.", values(size = 80.0, decay = 3.5, damp = 6000.0, predelay = 25.0, width = 100.0, mix = 100.0))]
pub struct Reverb {
    #[param(range = 0.0..100.0, default = 50.0, unit = "%", doc = "the room: from a small room (0) to a hall (100), the lengths of the lines")]
    pub size: f32,
    #[param(range = 0.2..20.0, default = 2.0, curve = log, unit = "s", doc = "the time the tail takes to fall 60 dB (RT60, of the lows)")]
    pub decay: f32,
    #[param(range = 500.0..20000.0, default = 8000.0, curve = log, unit = "Hz", doc = "the lowpass in the loops: the highs die sooner; at the top there is none")]
    pub damp: f32,
    #[param(range = 0.0..250.0, default = 10.0, unit = "ms", doc = "the gap between the sound and its reverb")]
    pub predelay: f32,
    #[param(range = 0.0..100.0, default = 100.0, unit = "%", doc = "the tail's stereo width: 0 mono, 100 full")]
    pub width: f32,
    #[param(range = 0.0..100.0, default = 30.0, unit = "%", doc = "the reverb against the dry signal: 0 dry only, 100 reverb only")]
    pub mix: f32,
    /// built in `init`
    net: Option<Fdn>,
    pre: Option<[Delay; 2]>,
    glide: Glide,
    clock: usize,
    /// samples in a row with no input and a silent tail, and how many make the state all zero
    quiet: usize,
    drained: usize,
}

/// the knobs as the network hears them: gliding, and turned into loop gains every `CTRL` samples
#[derive(Default)]
struct Glide {
    /// every line's length in samples, gliding to its prime
    len: [Smooth; LINES],
    lens: [f32; LINES],
    gain: [f32; LINES],
    k: f32,
    decay: Smooth,
    damp_oct: Smooth,
    pre: Smooth,
    width: Smooth,
    wet: Smooth,
    slew: f32,
    slew_len: f32,
    rate: f64,
}

impl Glide {
    fn all(&mut self) -> impl Iterator<Item = &mut Smooth> {
        self.len.iter_mut().chain([&mut self.decay, &mut self.damp_oct, &mut self.pre, &mut self.width, &mut self.wet])
    }

    /// the gains and the damping for the next `CTRL` samples
    fn control(&mut self) {
        let rt60 = f64::from(self.decay.step(self.slew)) * self.rate;
        for j in 0..LINES {
            self.len[j].step(self.slew_len);
            self.len[j].land(SAMPLE_ARRIVED);
            self.lens[j] = self.len[j].value();
            self.gain[j] = fdn::gain(f64::from(self.lens[j]), rt60);
        }
        let hz = exp2(f64::from(self.damp_oct.step(self.slew))) as f32;
        self.k = if hz >= NO_DAMP_HZ { 1.0 } else { OnePole::coef(hz, self.rate as f32) };
        for (s, within) in [(&mut self.pre, SAMPLE_ARRIVED), (&mut self.width, ARRIVED), (&mut self.wet, ARRIVED)] {
            s.step(self.slew);
            s.land(within);
        }
    }
}

impl Reverb {
    /// where every glide heads, from the parameters
    fn aim(&mut self) {
        let g = &mut self.glide;
        let scale = 0.2 + 0.8 * f64::from(self.size.clamp(0.0, 100.0)) / 100.0;
        let lens = fdn::primes(&BASE_MS.map(|ms| ms * scale * g.rate / 1000.0));
        for (s, l) in g.len.iter_mut().zip(lens) {
            s.to(l as f32);
        }
        g.decay.to(self.decay.clamp(0.2, 20.0));
        g.damp_oct.to(log2(f64::from(self.damp.clamp(500.0, 20_000.0))) as f32);
        g.pre.to((f64::from(self.predelay.clamp(0.0, MAX_PREDELAY_MS as f32)) * g.rate / 1000.0) as f32);
        g.width.to(self.width.clamp(0.0, 100.0) / 100.0);
        g.wet.to(self.mix.clamp(0.0, 100.0) / 100.0);
    }
}

impl Module for Reverb {
    fn init(&mut self, ctx: &Ctx) {
        let rate = ctx.rate().max(1.0);
        let longest = (BASE_MS[LINES - 1] * rate / 1000.0) as usize * 11 / 10 + 8;
        let pre = (MAX_PREDELAY_MS * rate / 1000.0) as usize + 2;
        self.net = Some(Fdn::new(longest));
        self.pre = Some([Delay::new(pre), Delay::new(pre)]);
        self.drained = longest + pre + CTRL;
        let g = &mut self.glide;
        g.rate = rate;
        g.slew = Smooth::coef(SLEW, CTRL as f32 / rate as f32);
        g.slew_len = Smooth::coef(SLEW_LEN, CTRL as f32 / rate as f32);
        self.aim();
        self.glide.all().for_each(|s| s.jump(s.target()));
        self.glide.control();
    }

    fn params_changed(&mut self, _: Dirty) {
        self.aim();
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let n = block.frames();
        let (Some(net), Some([pl, pr])) = (self.net.as_mut(), self.pre.as_mut()) else {
            block.set_silent(Self::OUT);
            return;
        };
        let g = &mut self.glide;
        if block.in_silent(Self::IN) && self.quiet >= self.drained {
            // the tail has run out: the state starts from zero when a signal comes
            if self.quiet != usize::MAX {
                net.clear();
                pl.clear();
                pr.clear();
                self.quiet = usize::MAX;
            }
            self.clock = self.clock.wrapping_add(n);
            block.set_silent(Self::OUT);
            return;
        }
        let (inl, inr) = block.audio_in(Self::IN).stereo();
        let mut out = block.audio_out(Self::OUT);
        let (outl, outr) = out.stereo();
        let mono = outr.is_empty();
        for i in 0..n {
            if self.clock % CTRL == 0 {
                g.control();
            }
            let (l, r) = (inl[i], inr[i]);
            let (dl, dr) = (pl.read_frac(g.pre.value()), pr.read_frac(g.pre.value()));
            pl.write(l);
            pr.write(r);
            let mut inject = [0.0f32; LINES];
            for (j, x) in inject.iter_mut().enumerate() {
                *x = if j % 2 == 0 { dl } else { dr } * SIGN[j] * IN_GAIN;
            }
            let y = net.tick(&inject, &g.lens, &g.gain, g.k);
            let (mut a, mut b) = (0.0f32, 0.0f32);
            for j in 0..LINES {
                a += TAP_L[j] * y[j];
                b += TAP_R[j] * y[j];
            }
            let (mid, side) = (0.5 * (a + b) * OUT_GAIN, 0.5 * (a - b) * OUT_GAIN * g.width.value());
            let wet = g.wet.value();
            let (wl, wr) = (l * (1.0 - wet) + (mid + side) * wet, r * (1.0 - wet) + (mid - side) * wet);
            if mono {
                if let Some(o) = outl.get_mut(i) {
                    *o = 0.5 * (wl + wr);
                }
            } else {
                outl[i] = wl;
                outr[i] = wr;
            }
            let still = l == 0.0 && r == 0.0 && y.iter().all(|v| *v == 0.0);
            self.quiet = if still { self.quiet.saturating_add(1).min(usize::MAX - 1) } else { 0 };
            self.clock = self.clock.wrapping_add(1);
        }
    }
}
