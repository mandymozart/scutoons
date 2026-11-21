//! Seeded noise: white (xorshift64*) and pink (Paul Kellet's refined filter over it, −3 dB per octave). The seed is the
//! caller's (`Ctx::seed`, a node's seed), so two decks that play the same thing play the same noise; never a clock.

/// splitmix64's finaliser: one well-mixed 64-bit number from any input
pub fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Seeded white noise: xorshift64* (Vigna). Two readings of the one generator: `sample` (the engine's `noise`) and
/// `next` (the analog synth's) – a render is the bits of the one it was made with.
#[derive(Debug, Clone, Copy)]
pub struct White(u64);

impl Default for White {
    fn default() -> Self {
        White::new(0)
    }
}

impl White {
    pub fn new(seed: u64) -> White {
        White(mix(seed) | 1)
    }

    #[inline(always)]
    fn bits(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// uniform in (−1, 1) with a mean of exactly 0 over its 23-bit grid
    #[inline(always)]
    pub fn sample(&mut self) -> f32 {
        // 23 bits + one half: exact in f32, symmetric around 0
        ((self.bits() >> 41) as f32 + 0.5) * (1.0 / 4_194_304.0) - 1.0
    }

    /// uniform in [−1, 1) on a 24-bit grid
    #[inline(always)]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        (self.bits() >> 40) as f32 * (1.0 / 8_388_608.0) - 1.0
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
    pub fn sample(&mut self) -> f32 {
        let w = self.white.sample();
        let b = &mut self.b;
        b[0] = 0.99886 * b[0] + w * 0.055_517_9;
        b[1] = 0.99332 * b[1] + w * 0.075_075_9;
        b[2] = 0.96900 * b[2] + w * 0.153_852;
        b[3] = 0.86650 * b[3] + w * 0.310_485_6;
        b[4] = 0.55000 * b[4] + w * 0.532_952_2;
        b[5] = -0.7616 * b[5] - w * 0.016_898_0;
        let out = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
        b[6] = w * 0.115_926;
        out * PINK_GAIN
    }
}

#[cfg(test)]
mod tests {
    use super::super::probe::fft;
    use super::*;
    use core::f64::consts::PI;

    const RATE: f32 = 48_000.0;

    fn db(x: f64) -> f64 {
        20.0 * x.max(1e-30).log10()
    }

    /// the summed power spectrum (bins 0..l/2) of Hann windows of `l` points, 50 % overlap, and how many
    fn periodogram(x: &[f32], l: usize) -> (Vec<f64>, usize) {
        let window: Vec<f64> = (0..l).map(|i| 0.5 - 0.5 * libm::cos(2.0 * PI * i as f64 / l as f64)).collect();
        let mut acc = vec![0.0f64; l / 2];
        let (mut count, mut at) = (0, 0);
        while at + l <= x.len() {
            let mut re: Vec<f64> = (0..l).map(|i| f64::from(x[at + i]) * window[i]).collect();
            let mut im = vec![0.0f64; l];
            fft(&mut re, &mut im);
            for (i, a) in acc.iter_mut().enumerate() {
                *a += re[i] * re[i] + im[i] * im[i];
            }
            count += 1;
            at += l / 2;
        }
        (acc, count)
    }

    #[test]
    fn same_seed_same_noise_other_seed_other_noise() {
        let (mut a, mut b, mut c) = (White::new(5), White::new(5), White::new(6));
        let xs: Vec<f32> = (0..64).map(|_| a.next()).collect();
        assert!(xs.iter().all(|&x| x == b.next()));
        assert!(xs.iter().any(|&x| x != c.next()));
    }

    #[test]
    fn is_white_uniform_and_bounded() {
        let mut n = White::new(1);
        let xs: Vec<f64> = (0..200_000).map(|_| n.next() as f64).collect();
        assert!(xs.iter().all(|&x| (-1.0..1.0).contains(&x)));
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
        assert!(mean.abs() < 0.01, "mean {mean}");
        assert!((var - 1.0 / 3.0).abs() < 0.01, "variance {var}");
        // white: neighbouring samples are uncorrelated
        let lag1 = xs.windows(2).map(|w| w[0] * w[1]).sum::<f64>() / (xs.len() - 1) as f64 / var;
        assert!(lag1.abs() < 0.01, "lag-1 correlation {lag1}");
    }

    #[test]
    fn white_noise_is_seeded_uniform_and_flat() {
        let mut a = White::new(7);
        let mut b = White::new(7);
        let mut c = White::new(8);
        let x: Vec<f32> = (0..1 << 18).map(|_| a.sample()).collect();
        assert!(x.iter().zip((0..).map(|_| b.sample())).all(|(p, q)| *p == q), "the same seed, the same noise");
        assert!((0..64).any(|_| c.sample() != a.sample()), "another seed, other noise");
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
        let (acc, count) = periodogram(x, L);
        let per_hz = |lo: f64, hi: f64| {
            let (a, b) = ((lo / f64::from(RATE) * L as f64) as usize, (hi / f64::from(RATE) * L as f64) as usize);
            acc[a..b].iter().sum::<f64>() / count as f64 / (b - a) as f64
        };
        (0..8).map(|i| per_hz(500.0 + i as f64 * 2500.0, 500.0 + (i + 1) as f64 * 2500.0)).collect()
    }

    #[test]
    fn pink_noise_falls_three_db_per_octave() {
        let mut p = Pink::new(3);
        let x: Vec<f32> = (0..1 << 20).map(|_| p.sample()).collect();
        // power per hertz at the centres of octave bands 100 Hz … 12.8 kHz: −3 dB per octave ± 0.5
        const L: usize = 4096;
        let (acc, count) = periodogram(&x, L);
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
}
