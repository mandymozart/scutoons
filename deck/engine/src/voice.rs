//! The `voice` node: a singing voice by formant synthesis. The source is the band-limited saw table (its −6 dB per
//! octave is the radiated glottal pulse's slope) mixed with seeded breath noise and softened by a one-pole low-pass;
//! four parallel band-passes (Simper's SVF, peak gain 1) carry the vowel's formants F1–F4. The vowel is a continuous
//! index into `VOWELS`, interpolated per `UPDATE` samples. Pitch glides linearly in semitones; vibrato fades in after a
//! note starts. libm and plain f32 only (native ≡ wasm), nothing allocates in `process`.
use super::blocks::{at, noise_seed, pitch, Env, Glide, Tables};
use super::{Block, Buf, Param};
use deck_core::math::sin_turns;
use deck_core::smooth::{GLIDE, QUICK};
use deck_module::dsp::flush;
use deck_module::dsp::noise::White;
use deck_module::dsp::wavetable::{self, Wave, WaveTable};
use deck_space::graph::Node;
use std::sync::Arc;

/// One vowel: formant centres (Hz), bandwidths (Hz) and levels (dB, F1 = 0).
pub struct Vowel {
    pub name: &'static str,
    pub freq: [f32; 4],
    pub bw: [f32; 4],
    pub db: [f32; 4],
}

/// A high (female) voice, ordered front to back so that a glide between two vowels passes the ones between. F1–F3 of
/// ee, eh, ah, uh, oo: Peterson & Barney (1952), women's averages; oh and F4, the bandwidths and the levels after the
/// soprano table of the Csound manual's formant appendix (levels adjusted so each vowel's F2 stands out of the source).
pub const VOWELS: [Vowel; 6] = [
    Vowel { name: "ee", freq: [310.0, 2790.0, 3310.0, 3900.0], bw: [60.0, 90.0, 100.0, 120.0], db: [0.0, -10.0, -22.0, -26.0] },
    Vowel { name: "eh", freq: [610.0, 2330.0, 2990.0, 3900.0], bw: [60.0, 100.0, 120.0, 150.0], db: [0.0, -10.0, -16.0, -34.0] },
    Vowel { name: "ah", freq: [850.0, 1220.0, 2810.0, 3900.0], bw: [80.0, 90.0, 120.0, 130.0], db: [0.0, -4.0, -28.0, -22.0] },
    Vowel { name: "uh", freq: [760.0, 1400.0, 2780.0, 3900.0], bw: [80.0, 90.0, 120.0, 130.0], db: [0.0, -4.0, -24.0, -24.0] },
    Vowel { name: "oh", freq: [450.0, 800.0, 2830.0, 3800.0], bw: [70.0, 80.0, 100.0, 130.0], db: [0.0, -6.0, -22.0, -22.0] },
    Vowel { name: "oo", freq: [370.0, 950.0, 2670.0, 3800.0], bw: [50.0, 60.0, 170.0, 180.0], db: [0.0, -8.0, -32.0, -36.0] },
];

/// samples between two formant updates (≈ 0.33 ms at 48 kHz)
const UPDATE: usize = 16;
/// vibrato: silent for this long after a note starts, then fading in over `VIBRATO_FADE`
const VIBRATO_DELAY: f32 = 0.25;
const VIBRATO_FADE: f32 = 0.35;
/// breath noise against the saw's level (white noise is flat; the formants take most of it)
const NOISE: f32 = 0.6;
/// the source's softening low-pass
const TILT_HZ: f32 = 6000.0;
/// brings a sustained vowel at gain 1 to a peak of ≈ 0.5
const LEVEL: f32 = 2.0;

/// one formant: a band-pass SVF with its coefficients and level
#[derive(Clone, Copy, Default)]
struct Band {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    /// 1 / Q – also the factor that brings the band-pass's peak to 1
    k: f32,
    amp: f32,
}

impl Band {
    fn set(&mut self, freq: f32, bw: f32, amp: f32, pi_over_rate: f32) {
        let g = libm::tanf((freq * pi_over_rate).min(1.5));
        let k = bw / freq;
        self.a1 = 1.0 / (1.0 + g * (g + k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
        self.k = k;
        self.amp = amp;
    }

    #[inline(always)]
    fn tick(&mut self, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = flush(2.0 * v1 - self.ic1);
        self.ic2 = flush(2.0 * v2 - self.ic2);
        v1 * self.k * self.amp
    }
}

pub(super) struct Voice {
    table: Arc<WaveTable>,
    osc: wavetable::Osc,
    noise: White,
    /// seconds per sample
    sd: f32,
    rate: f32,
    pi_over_rate: f32,
    nyquist: f32,
    freq: Glide,
    gain: Glide,
    vowel: Glide,
    glide: Param,
    vibrato: Glide,
    vib_rate: Glide,
    breath: Glide,
    env: Env,
    /// pitch in semitones from A4, where it glides to, the step per sample, the Hz the target came from
    st: f32,
    target: f32,
    step: f32,
    target_hz: f32,
    open: bool,
    /// seconds since the note started (the vibrato's delay)
    held: f32,
    vib_phase: f32,
    tilt: f32,
    tilt_a: f32,
    bands: [Band; 4],
    /// samples left until the next formant update
    countdown: usize,
    amps: [[f32; 4]; 6],
}

impl Voice {
    pub(super) fn new(node: &Node, index: usize, rate: f32, p: impl Fn(&str) -> Param, tables: &mut Tables) -> Voice {
        let amps = VOWELS.map(|v| v.db.map(|d| libm::powf(10.0, d / 20.0)));
        Voice {
            table: tables.get(Wave::Saw),
            osc: wavetable::Osc::new(),
            noise: White::new(noise_seed(node, index)),
            sd: 1.0 / rate,
            rate,
            pi_over_rate: std::f32::consts::PI / rate,
            nyquist: 0.45 * rate,
            freq: Glide::new(p("freq"), GLIDE, rate),
            gain: Glide::new(p("gain"), GLIDE, rate),
            vowel: Glide::new(p("vowel"), QUICK, rate),
            glide: p("glide"),
            vibrato: Glide::new(p("vibrato"), QUICK, rate),
            vib_rate: Glide::new(p("rate"), QUICK, rate),
            breath: Glide::new(p("breath"), QUICK, rate),
            env: Env::new(rate, node),
            st: f32::NAN,
            target: 0.0,
            step: 0.0,
            target_hz: 0.0,
            open: false,
            held: 0.0,
            vib_phase: 0.0,
            tilt: 0.0,
            tilt_a: 1.0 - libm::expf(-2.0 * std::f32::consts::PI * TILT_HZ / rate),
            bands: [Band::default(); 4],
            countdown: 0,
            amps,
        }
    }

    /// the formants of the (fractional) vowel `v`
    fn formants(&mut self, v: f32) {
        let v = v.clamp(0.0, (VOWELS.len() - 1) as f32);
        let i = (v as usize).min(VOWELS.len() - 2);
        let t = v - i as f32;
        let (a, b) = (&VOWELS[i], &VOWELS[i + 1]);
        for k in 0..4 {
            let lerp = |x: f32, y: f32| x + (y - x) * t;
            let f = lerp(a.freq[k], b.freq[k]).min(self.nyquist);
            self.bands[k].set(f, lerp(a.bw[k], b.bw[k]), lerp(self.amps[i][k], self.amps[i + 1][k]), self.pi_over_rate);
        }
    }

    /// a new pitch to reach: at once when nothing sounds, else in `glide` seconds
    fn aim(&mut self, hz: f32) {
        self.target_hz = hz;
        self.target = 12.0 * libm::log2f(hz / 440.0);
        let samples = self.glide.get().max(0.0) * self.rate;
        if self.st.is_nan() || self.env.level() < 1e-3 || samples < 1.0 {
            self.st = self.target;
            self.step = 0.0;
        } else {
            self.step = (self.target - self.st) / samples;
        }
    }
}

impl Block for Voice {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (freq, gain, vowel) = (self.freq.block(n), self.gain.block(n), self.vowel.block(n));
        let (depth, vib_rate, breath) = (self.vibrato.block(n), self.vib_rate.block(n), self.breath.block(n));
        for i in 0..n {
            let (hz, gate) = pitch(ins[0], freq, i);
            let open = gate > 0.5;
            if open && !self.open {
                self.held = 0.0;
            }
            self.open = open;
            if hz > 0.0 && hz != self.target_hz {
                self.aim(hz);
            }
            if self.step != 0.0 {
                self.st += self.step;
                if (self.step > 0.0) == (self.st >= self.target) {
                    self.st = self.target;
                    self.step = 0.0;
                }
            }
            let fade = ((self.held - VIBRATO_DELAY) / VIBRATO_FADE).clamp(0.0, 1.0);
            let vib = at(depth, i) * fade * sin_turns(self.vib_phase);
            self.vib_phase = deck_core::math::wrap01(self.vib_phase + at(vib_rate, i) * self.sd);
            self.held += self.sd;
            let f = 440.0 * libm::exp2f((self.st + vib) / 12.0);

            if self.countdown == 0 {
                self.formants(at(vowel, i));
                self.countdown = UPDATE;
            }
            self.countdown -= 1;

            let b = at(breath, i).clamp(0.0, 1.0);
            let src = self.osc.tick(&self.table, f, self.sd) * (1.0 - b) + self.noise.sample() * NOISE * b;
            self.tilt += (src - self.tilt) * self.tilt_a;
            let x = self.tilt;
            let y = self.bands.iter_mut().fold(0.0, |s, band| s + band.tick(x));
            let v = y * LEVEL * self.env.next(gate) * at(gain, i);
            outs[0].put2(i, v, v);
        }
    }
}
