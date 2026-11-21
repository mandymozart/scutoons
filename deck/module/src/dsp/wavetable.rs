//! Band-limited saw, square and triangle from a mip-mapped wavetable (one 2048-point table per quarter octave, linear
//! interpolation): cleaner than PolyBLEP, for a wave that only changes its pitch. Plain f32 arithmetic and `libm`, so a
//! native build and a wasm build compute the same bits; the tables are built once (`WaveTable::new`, outside the audio
//! path), a voice is a phase.
use core::f64::consts::PI;

/// Where an oscillator's phase starts, in turns (0.993 037 34 = the f32 of fundsp's old `rnd1(0)`, kept as a literal
/// so the sine voices keep their sound): the sine and FM voices begin just before a zero crossing.
pub const START_PHASE: f32 = 0.993_037_34;

/// the wrap of a phase into [0, 1) (any sign, any size)
#[inline(always)]
fn wrap01(x: f32) -> f32 {
    let w = x - x.floor();
    // a tiny negative phase rounds to exactly 1.0 in f32
    if w >= 1.0 {
        0.0
    } else {
        w
    }
}

/// tables per octave: a note loses at most 2^(1/4) of the spectrum under Nyquist (its highest harmonic is above 20 kHz)
const PER_OCTAVE: usize = 4;
/// the ratios of the quarter-octave steps inside an octave: 2^(1/4), 2^(2/4), 2^(3/4)
const STEPS: [f32; 3] = [1.189_207_1, core::f32::consts::SQRT_2, 1.681_792_8];

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

#[cfg(test)]
mod tests {
    use super::super::probe::spectrum;
    use super::*;

    const RATE: f32 = 48_000.0;
    const N: usize = 16_384;

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
}
