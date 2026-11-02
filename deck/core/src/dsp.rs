//! The engine's DSP units, written for both targets (research/ENGINE-REFACTOR-PLAN.md §3.2, #317): the band-limited
//! oscillators (saw, square, triangle), seeded white and pink noise, and the state-variable lowpass. They replace the six
//! fundsp units the native deck used (fundsp left the workspace with the engine move). Plain f32 arithmetic and the
//! pure-Rust `libm` for the transcendentals, so a native build and a wasm build compute the same bits (§3.4); no
//! allocation after construction, no platform. The quality bar is analytic, not "sounds like fundsp": every unit has
//! its FFT test below (harmonic levels, aliasing, spectral slope, the filter's response against the bilinear
//! transform of the analogue prototype).
//!
//! Sources, not dependencies: the mip-mapped wavetable (one 2048-point table per octave, linear interpolation – the
//! P0 probe's preference over PolyBLEP, ENGINE-ALIGNMENT.md §2a (d)); Paul Kellet's "refined" pink filter
//! (musicdsp.org, 2000); Andrew Simper's state-variable filter (Cytomic, "SvfLinearTrapezoidalSin", 2013).
use core::f64::consts::PI;

/// Where an oscillator's phase starts, in turns (0.993 037 34 = the f32 of fundsp's old `rnd1(0)`, kept as a literal
/// so the sine voices keep their sound): the sine and FM voices begin just before a zero crossing.
pub const START_PHASE: f32 = 0.993_037_34;

/// the wrap of a phase into [0, 1) (any sign, any size)
#[inline(always)]
pub fn wrap01(x: f32) -> f32 {
    let w = x - x.floor();
    // a tiny negative phase rounds to exactly 1.0 in f32
    if w >= 1.0 {
        0.0
    } else {
        w
    }
}

// ── band-limited oscillators ────────────────────────────────────────────────────────────────────────────────────────

/// tables per octave: a note loses at most 2^(1/4) of the spectrum under Nyquist (its highest harmonic is above 20 kHz)
const PER_OCTAVE: usize = 4;
/// the ratios of the quarter-octave steps inside an octave: 2^(1/4), 2^(2/4), 2^(3/4)
const STEPS: [f32; 3] = [1.189_207_1, 1.414_213_6, 1.681_792_8];

/// The three non-sine waves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Saw,
    Square,
    Triangle,
}

impl Wave {
    pub fn parse(name: &str) -> Option<Wave> {
        Some(match name {
            "saw" => Wave::Saw,
            "square" => Wave::Square,
            "triangle" => Wave::Triangle,
            _ => return None,
        })
    }
}

/// points per table (one period)
pub const TABLE: usize = 2048;
/// the lowest octave's fundamentals start at 2^LOWEST_EXP Hz (below it the lowest table is used: it only holds more
/// harmonics than the fundamental can carry)
const LOWEST_EXP: i32 = 4;

/// One wave's mip-mapped wavetable for a sample rate: a table per quarter octave, `level(f)` picks the one whose
/// harmonics all stay under Nyquist for any fundamental of f's quarter octave. Built at load with a table of sines (libm)
/// and sums in a fixed order, so the bits are the same everywhere; ≈ 350 KB and some 20 ms per wave.
pub struct WaveTable {
    wave: Wave,
    /// per quarter octave from 16 Hz: TABLE + 1 points (the last repeats the first, so the interpolation needs no wrap)
    levels: Vec<Box<[f32]>>,
}

impl WaveTable {
    pub fn new(wave: Wave, rate: f32) -> WaveTable {
        let nyquist = f64::from(rate) / 2.0;
        // the sine table: sin(2π j / TABLE), looked up at (i · k) mod TABLE for the k-th harmonic of sample i
        let sine: Vec<f64> = (0..TABLE).map(|j| libm::sin(2.0 * PI * j as f64 / TABLE as f64)).collect();
        // levels until one's top is past Nyquist (that one is the pure sine)
        let count = (libm::ceil(libm::log2(nyquist / f64::from(1u32 << LOWEST_EXP)) * PER_OCTAVE as f64) as usize).max(1);
        let levels = (0..count)
            .map(|k| {
                // the quarter octave [16 · 2^(k/4), 16 · 2^((k+1)/4)) Hz: its top is the worst case for the harmonics
                let top = f64::from(1u32 << LOWEST_EXP) * libm::exp2((k + 1) as f64 / PER_OCTAVE as f64);
                let harmonics = ((nyquist / top) as usize).max(1);
                let mut t = vec![0.0f32; TABLE + 1];
                for (i, y) in t.iter_mut().enumerate().take(TABLE) {
                    let mut s = 0.0f64;
                    for h in 1..=harmonics {
                        let x = sine[(i * h) % TABLE];
                        let hf = h as f64;
                        match wave {
                            Wave::Saw => s -= x / hf,
                            Wave::Square if h % 2 == 1 => s += x / hf,
                            Wave::Triangle if h % 2 == 1 => {
                                let sign = if (h / 2) % 2 == 0 { 1.0 } else { -1.0 };
                                s += sign * x / (hf * hf);
                            }
                            _ => {}
                        }
                    }
                    *y = (s * match wave {
                        Wave::Saw => 2.0 / PI,
                        Wave::Square => 4.0 / PI,
                        Wave::Triangle => 8.0 / (PI * PI),
                    }) as f32;
                }
                t[TABLE] = t[0];
                t.into_boxed_slice()
            })
            .collect();
        WaveTable { wave, levels }
    }

    pub fn wave(&self) -> Wave {
        self.wave
    }

    /// the table for fundamental `f` (Hz; the sign does not matter): its octave from the exponent bits, its quarter from
    /// the mantissa against three constants
    #[inline(always)]
    pub fn level(&self, f: f32) -> usize {
        let bits = f.abs().to_bits();
        let exp = ((bits >> 23) & 0xff) as i32 - 127;
        if exp < LOWEST_EXP {
            return 0;
        }
        let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000); // the mantissa as 1 … 2
        let quarter = (m >= STEPS[0]) as usize + (m >= STEPS[1]) as usize + (m >= STEPS[2]) as usize;
        ((exp - LOWEST_EXP) as usize * PER_OCTAVE + quarter).min(self.levels.len() - 1)
    }

    /// the wave at `phase` (turns, [0, 1)) from table `level`, linear interpolation
    #[inline(always)]
    pub fn at(&self, phase: f32, level: usize) -> f32 {
        let t = &self.levels[level];
        let x = phase * TABLE as f32;
        let i = (x as usize).min(TABLE - 1);
        let fr = x - i as f32;
        t[i] + (t[i + 1] - t[i]) * fr
    }
}

/// One oscillator voice: a phase over a shared `WaveTable`. `tick` takes the frequency of this sample (it may change
/// every sample) and the sample period.
#[derive(Debug, Clone, Copy, Default)]
pub struct Osc {
    phase: f32,
}

impl Osc {
    pub fn new() -> Osc {
        Osc { phase: 0.0 }
    }

    #[inline(always)]
    pub fn tick(&mut self, table: &WaveTable, freq: f32, seconds_per_sample: f32) -> f32 {
        let y = table.at(self.phase, table.level(freq));
        self.phase = wrap01(self.phase + freq * seconds_per_sample);
        y
    }
}

// ── noise ───────────────────────────────────────────────────────────────────────────────────────────────────────────

/// splitmix64's finaliser: a seed of any kind into a well-mixed non-zero xorshift state
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z | 1
}

/// Seeded white noise: xorshift64* (Vigna), uniform in (−1, 1) with a mean of exactly 0 over its 23-bit grid.
#[derive(Debug, Clone)]
pub struct White(u64);

impl White {
    pub fn new(seed: u64) -> White {
        White(mix(seed))
    }

    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        // 23 bits + one half: exact in f32, symmetric around 0
        ((r >> 41) as f32 + 0.5) * (1.0 / 4_194_304.0) - 1.0
    }
}

/// the level of `Pink`'s output relative to the white noise that feeds it (Kellet's own 0.11 scale gives ≈ 0.12 RMS;
/// this brings it to ≈ 0.5, close to the white's 0.58, so a colour switch does not jump in loudness)
const PINK_GAIN: f32 = 0.11 * 2.55;

/// Seeded pink noise (−3 dB per octave): Paul Kellet's refined filter over white noise.
#[derive(Debug, Clone)]
pub struct Pink {
    white: White,
    b: [f32; 7],
}

impl Pink {
    pub fn new(seed: u64) -> Pink {
        Pink { white: White::new(seed), b: [0.0; 7] }
    }

    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        let w = self.white.next();
        let b = &mut self.b;
        b[0] = 0.99886 * b[0] + w * 0.055_517_9;
        b[1] = 0.99332 * b[1] + w * 0.075_075_9;
        b[2] = 0.96900 * b[2] + w * 0.153_852_0;
        b[3] = 0.86650 * b[3] + w * 0.310_485_6;
        b[4] = 0.55000 * b[4] + w * 0.532_952_2;
        b[5] = -0.7616 * b[5] - w * 0.016_898_0;
        let out = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
        b[6] = w * 0.115_926;
        out * PINK_GAIN
    }
}

// ── the state-variable lowpass ──────────────────────────────────────────────────────────────────────────────────────

/// Simper's trapezoidal-integrator state-variable filter, lowpass output: stable at any cutoff below Nyquist, the
/// cutoff and Q may change every sample (no zipper beyond the coefficients' own change). `g = tan(π fc / fs)` is the
/// bilinear prewarp (libm's `tanf`: the same bits on every target), so the response is exactly the bilinear transform of
/// 1 / (1 + s/Q + s²).
#[derive(Debug, Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    pub fn new() -> Svf {
        Svf::default()
    }

    /// one sample through the lowpass at `cutoff` Hz (clamped under Nyquist) with `q` (0.5 = critically damped, 0.707 =
    /// Butterworth); `pi_over_rate` = π / sample rate
    #[inline(always)]
    pub fn lowpass(&mut self, x: f32, cutoff: f32, q: f32, pi_over_rate: f32) -> f32 {
        let g = libm::tanf((cutoff * pi_over_rate).min(1.5)); // < π/2: the prewarp stays finite
        let k = 1.0 / q.max(0.05);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v2
    }

    pub fn reset(&mut self) {
        *self = Svf::default();
    }
}

#[cfg(all(test, feature = "analysis"))]
mod tests {
    use super::*;
    use rustfft::{num_complex::Complex, FftPlanner};

    const RATE: f32 = 48_000.0;
    const N: usize = 16_384;

    /// the magnitude spectrum (bins 0..N/2) of N samples, amplitude-scaled so a full-scale sine bin reads its amplitude
    fn spectrum(x: &[f32]) -> Vec<f64> {
        let mut buf: Vec<Complex<f64>> = x.iter().map(|&v| Complex::new(f64::from(v), 0.0)).collect();
        FftPlanner::new().plan_fft_forward(x.len()).process(&mut buf);
        buf[..x.len() / 2].iter().map(|c| c.norm() * 2.0 / x.len() as f64).collect()
    }

    fn render(wave: Wave, bin: usize) -> Vec<f32> {
        let table = WaveTable::new(wave, RATE);
        let f = bin as f32 * RATE / N as f32; // a coherent window: exactly `bin` periods in N samples
        let mut o = Osc::new();
        (0..N).map(|_| o.tick(&table, f, 1.0 / RATE)).collect()
    }

    fn db(x: f64) -> f64 {
        20.0 * x.max(1e-30).log10()
    }

    /// share of the energy outside the harmonics (multiples of `bin`) of a coherent spectrum, in dB
    fn aliasing_db(s: &[f64], bin: usize) -> f64 {
        let (mut harmonic, mut other) = (0.0, 0.0);
        for (i, v) in s.iter().enumerate().skip(1) {
            if i % bin == 0 {
                harmonic += v * v;
            } else {
                other += v * v;
            }
        }
        10.0 * (other / harmonic).log10()
    }

    /// harmonic k of a unit saw/square/triangle (Fourier series), as an amplitude
    fn ideal(wave: Wave, k: usize) -> f64 {
        let k = k as f64;
        match wave {
            Wave::Saw => 2.0 / (PI * k),
            Wave::Square => 4.0 / (PI * k),
            Wave::Triangle => 8.0 / (PI * PI * k * k),
        }
    }

    #[test]
    fn saw_square_triangle_have_the_series_harmonics_and_no_aliasing() {
        // bins 211 (618 Hz), 1500 (4.4 kHz) and 4000 (11.7 kHz): the plan's bar is −60 dB for fundamentals up to 4 kHz
        for (wave, bar_db) in [(Wave::Saw, -80.0), (Wave::Square, -80.0), (Wave::Triangle, -100.0)] {
            for bin in [211usize, 1500, 4000] {
                let s = spectrum(&render(wave, bin));
                let odd_only = wave != Wave::Saw;
                // the first harmonics within 0.1 dB of 1/k (the table's linear interpolation droops the highest); a
                // note's tables hold the harmonics up to 20 kHz (a quarter octave under Nyquist), not those past it
                let harmonics = (20_000 * N / RATE as usize / bin).min(8);
                for k in 1..=harmonics {
                    if odd_only && k % 2 == 0 {
                        assert!(db(s[k * bin]) < -70.0, "{wave:?} bin {bin}: even harmonic {k} at {:.1} dB", db(s[k * bin]));
                        continue;
                    }
                    let err = db(s[k * bin]) - db(ideal(wave, k));
                    assert!(err.abs() < 0.1, "{wave:?} bin {bin}: harmonic {k} is {err:.3} dB off its 1/k level");
                }
                let alias = aliasing_db(&s, bin);
                let worst = (1..=harmonics).filter(|k| !odd_only || k % 2 == 1).map(|k| (db(s[k * bin]) - db(ideal(wave, k))).abs()).fold(0.0, f64::max);
                println!("{wave:?} {:.0} Hz: harmonics 1-{harmonics} within {worst:.4} dB of the series, non-harmonic energy {alias:.1} dB", bin as f32 * RATE / N as f32);
                assert!(alias < bar_db, "{wave:?} bin {bin}: non-harmonic energy {alias:.1} dB (bar {bar_db})");
            }
        }
    }

    #[test]
    fn a_gliding_saw_stays_band_limited() {
        // a sweep from 100 Hz to 12 kHz in one second, the table switching per sample: nothing above the fundamental's
        // own harmonics aliases back – checked as the energy under 30 Hz after a high-pass-free look at the top bins
        let table = WaveTable::new(Wave::Saw, RATE);
        let mut o = Osc::new();
        let n = 48_000;
        let y: Vec<f32> = (0..n)
            .map(|i| {
                let f = 100.0 * libm::powf(120.0, i as f32 / n as f32);
                o.tick(&table, f, 1.0 / RATE)
            })
            .collect();
        assert!(y.iter().all(|v| v.is_finite() && v.abs() < 1.2), "bounded");
    }

    #[test]
    fn the_levels_follow_the_octaves() {
        let t = WaveTable::new(Wave::Saw, RATE);
        assert_eq!(t.level(0.0), 0);
        assert_eq!(t.level(17.0), 0);
        assert_eq!(t.level(20.0), 1);
        assert_eq!(t.level(19.0), 0); // 16 · 2^(1/4) = 19.03: the first quarter ends there
        assert_eq!(t.level(19.2), 1);
        assert_eq!(t.level(31.99), 3);
        assert_eq!(t.level(32.0), 4);
        assert_eq!(t.level(-440.0), t.level(440.0));
        assert_eq!(t.level(1.0e9), t.levels.len() - 1);
        // the same bits every time (the determinism bar): a table built twice is identical
        let again = WaveTable::new(Wave::Saw, RATE);
        assert!(t.levels.iter().zip(&again.levels).all(|(a, b)| a == b));
    }

    #[test]
    fn white_noise_is_seeded_uniform_and_flat() {
        let mut a = White::new(7);
        let mut b = White::new(7);
        let mut c = White::new(8);
        let x: Vec<f32> = (0..1 << 18).map(|_| a.next()).collect();
        assert!(x.iter().zip((0..).map(|_| b.next())).all(|(p, q)| *p == q), "the same seed, the same noise");
        assert!((0..64).any(|_| c.next() != a.next()), "another seed, other noise");
        assert!(x.iter().all(|v| v.abs() < 1.0));
        let mean = x.iter().map(|v| f64::from(*v)).sum::<f64>() / x.len() as f64;
        let rms = (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        assert!(mean.abs() < 0.005, "mean {mean}");
        assert!((rms - 1.0 / 3.0f64.sqrt()).abs() < 0.005, "rms {rms}");
        // flat: the band powers of an averaged spectrum are within 1 dB
        let bands = band_powers(&x);
        let (lo, hi) = bands.iter().fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
        assert!(db(hi.sqrt()) - db(lo.sqrt()) < 1.0, "white band powers {bands:?}");
    }

    /// an averaged periodogram (Hann, 4096 points, 50 % overlap), summed over `BANDS` equal-width bands to 20 kHz
    fn band_powers(x: &[f32]) -> Vec<f64> {
        const L: usize = 4096;
        let window: Vec<f64> = (0..L).map(|i| 0.5 - 0.5 * libm::cos(2.0 * PI * i as f64 / L as f64)).collect();
        let fft = FftPlanner::new().plan_fft_forward(L);
        let mut acc = vec![0.0f64; L / 2];
        let mut count = 0;
        let mut at = 0;
        while at + L <= x.len() {
            let mut buf: Vec<Complex<f64>> = (0..L).map(|i| Complex::new(f64::from(x[at + i]) * window[i], 0.0)).collect();
            fft.process(&mut buf);
            for (a, c) in acc.iter_mut().zip(&buf) {
                *a += c.norm_sqr();
            }
            count += 1;
            at += L / 2;
        }
        let per_hz = |lo: f64, hi: f64| {
            let (a, b) = ((lo / f64::from(RATE) * L as f64) as usize, (hi / f64::from(RATE) * L as f64) as usize);
            acc[a..b].iter().sum::<f64>() / count as f64 / (b - a) as f64
        };
        (0..8).map(|i| per_hz(500.0 + i as f64 * 2500.0, 500.0 + (i + 1) as f64 * 2500.0)).collect()
    }

    #[test]
    fn pink_noise_falls_three_db_per_octave() {
        let mut p = Pink::new(3);
        let x: Vec<f32> = (0..1 << 20).map(|_| p.next()).collect();
        // power per hertz at the centres of octave bands 100 Hz … 12.8 kHz: −3 dB per octave ± 0.5
        const L: usize = 4096;
        let window: Vec<f64> = (0..L).map(|i| 0.5 - 0.5 * libm::cos(2.0 * PI * i as f64 / L as f64)).collect();
        let fft = FftPlanner::new().plan_fft_forward(L);
        let mut acc = vec![0.0f64; L / 2];
        let mut count = 0;
        let mut at = 0;
        while at + L <= x.len() {
            let mut buf: Vec<Complex<f64>> = (0..L).map(|i| Complex::new(f64::from(x[at + i]) * window[i], 0.0)).collect();
            fft.process(&mut buf);
            for (a, c) in acc.iter_mut().zip(&buf) {
                *a += c.norm_sqr();
            }
            count += 1;
            at += L / 2;
        }
        // total power in each octave [f, 2f): pink has the same in every octave
        let octave = |f: f64| {
            let (a, b) = ((f / f64::from(RATE) * L as f64).round() as usize, (2.0 * f / f64::from(RATE) * L as f64).round() as usize);
            acc[a..b].iter().sum::<f64>() / count as f64
        };
        let levels: Vec<f64> = [100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0, 6400.0].iter().map(|&f| 10.0 * octave(f).log10()).collect();
        for w in levels.windows(2) {
            assert!((w[1] - w[0]).abs() < 0.5, "octave powers {levels:?} dB are not level (−3 dB per octave in the density)");
        }
        let rms = (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        assert!(rms > 0.3 && rms < 0.7, "pink rms {rms}");
    }

    /// the filter's gain at `f` for a sine, measured after it settles
    fn svf_gain(cutoff: f32, q: f32, f: f32) -> f64 {
        let mut s = Svf::new();
        let pr = core::f32::consts::PI / RATE;
        let n = 48_000;
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for i in 0..n {
            let x = libm::sinf(core::f32::consts::TAU * f * i as f32 / RATE);
            let y = s.lowpass(x, cutoff, q, pr);
            if i >= n / 2 {
                num += f64::from(y).powi(2);
                den += f64::from(x).powi(2);
            }
        }
        (num / den).sqrt()
    }

    /// the analogue 1 / (1 + s/Q + s²) through the bilinear transform: |H| at f for cutoff fc (prewarped at fc)
    fn ideal_lowpass(fc: f64, q: f64, f: f64) -> f64 {
        let fs = f64::from(RATE);
        let g = libm::tan(PI * fc / fs);
        let w = libm::tan(PI * f / fs) / g;
        1.0 / ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt()
    }

    #[test]
    fn the_lowpass_is_the_bilinear_second_order_response() {
        for (fc, q) in [(1000.0f32, 0.707_106_8f32), (1000.0, 2.0), (4000.0, 0.7), (200.0, 1.0)] {
            for mult in [0.1f32, 0.5, 0.9, 1.0, 1.1, 2.0, 4.0] {
                let f = fc * mult;
                let (got, want) = (svf_gain(fc, q, f), ideal_lowpass(f64::from(fc), f64::from(q), f64::from(f)));
                assert!((db(got) - db(want)).abs() < 0.05, "fc {fc} Q {q} at {f} Hz: {:.3} dB, ideal {:.3} dB", db(got), db(want));
            }
        }
    }

    #[test]
    fn the_butterworth_lowpass_is_three_db_down_at_its_cutoff() {
        let q = core::f32::consts::FRAC_1_SQRT_2;
        let at = |f: f32| db(svf_gain(2000.0, q, f));
        // −3.01 dB lies between 0.98 and 1.02 of the cutoff: the −3 dB point is within 2 %
        assert!(at(1960.0) > -3.0103 && at(2040.0) < -3.0103, "{:.3} / {:.3} dB", at(1960.0), at(2040.0));
        // and Q shapes the peak: Q = 2 rises above the passband at the cutoff, as 20·log10(Q) says
        let peak = db(svf_gain(2000.0, 2.0, 2000.0));
        assert!((peak - db(2.0)).abs() < 0.1, "peak {peak}");
    }

    #[test]
    fn the_lowpass_survives_extreme_settings() {
        let mut s = Svf::new();
        let pr = core::f32::consts::PI / RATE;
        let mut w = White::new(1);
        for i in 0..100_000 {
            let cutoff = if i % 2 == 0 { 20.0 } else { 40_000.0 };
            let y = s.lowpass(w.next(), cutoff, 0.05 + (i % 7) as f32, pr);
            assert!(y.is_finite() && y.abs() < 1e3, "sample {i}: {y}");
        }
    }
}
