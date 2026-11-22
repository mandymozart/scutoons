//! The feedback delay network: eight delay lines whose outputs are mixed by a Householder reflection (`y − 2/8 Σy`,
//! lossless) and fed back, each line's loop with its own gain and a one-pole lowpass – the algorithmic reverb's tail.
//! A line's gain comes from the time the tail takes to fall 60 dB (`gain`), so every line decays at the same rate
//! whatever its length. The lines are allocated in `new` – call it in `init` – and never again.
use super::arith::exp2;
use super::delay::Delay;
use super::flush;
use super::onepole::OnePole;

pub const LINES: usize = 8;

/// log2(1000): −60 dB in octaves of amplitude
const LOG2_1000: f64 = 9.965_784_284_662_087;

#[derive(Debug, Clone)]
pub struct Fdn {
    lines: [Delay; LINES],
    damp: [OnePole; LINES],
}

impl Fdn {
    /// room for lines up to `max` samples (allocates)
    pub fn new(max: usize) -> Fdn {
        Fdn { lines: core::array::from_fn(|_| Delay::new(max)), damp: [OnePole::new(); LINES] }
    }

    /// the longest line, in samples
    pub fn max(&self) -> usize {
        self.lines[0].max()
    }

    pub fn clear(&mut self) {
        self.lines.iter_mut().for_each(Delay::clear);
        self.damp.iter_mut().for_each(OnePole::reset);
    }

    /// one sample: `inject` added into each line; the lines' outputs (after their gain and lowpass) – the taps.
    /// `len` in samples (fractional reads), `gain` from `gain`, `damp` a `OnePole::coef` (1 = none).
    #[inline(always)]
    pub fn tick(&mut self, inject: &[f32; LINES], len: &[f32; LINES], gain: &[f32; LINES], damp: f32) -> [f32; LINES] {
        let mut y = [0.0f32; LINES];
        for i in 0..LINES {
            y[i] = self.damp[i].lowpass(self.lines[i].read_frac(len[i]), damp) * gain[i];
        }
        let s = y.iter().sum::<f32>() * (2.0 / LINES as f32);
        for i in 0..LINES {
            self.lines[i].write(flush(y[i] - s + inject[i]));
        }
        y
    }
}

/// the loop gain of a line of `len` samples whose echoes fall 60 dB in `rt60` samples
pub fn gain(len: f64, rt60: f64) -> f32 {
    if rt60 <= 0.0 {
        return 0.0;
    }
    exp2(-LOG2_1000 * len / rt60) as f32
}

fn is_prime(n: usize) -> bool {
    if n < 4 {
        return n >= 2;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut d = 3;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

/// the smallest primes at or above `lens` (rising), each above the one before: mutually prime line lengths, so the
/// echoes of two lines never meet on a common period
pub fn primes(lens: &[f64; LINES]) -> [usize; LINES] {
    let mut out = [0usize; LINES];
    let mut floor = 2;
    for (o, l) in out.iter_mut().zip(lens) {
        let mut n = (l.max(2.0) as usize).max(floor);
        while !is_prime(n) {
            n += 1;
        }
        *o = n;
        floor = n + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primes_rise_and_are_prime() {
        assert_eq!(primes(&[2.0, 2.0, 10.0, 10.0, 24.0, 90.0, 90.5, 1000.0]), [2, 3, 11, 13, 29, 97, 101, 1009]);
        assert!(!is_prime(1) && !is_prime(9) && !is_prime(4087) && is_prime(4093));
    }

    #[test]
    fn the_gain_takes_60_db_over_rt60() {
        let g = gain(480.0, 48_000.0);
        // 100 trips of 480 samples: −60 dB
        assert!((f64::from(g).powi(100) - 0.001).abs() < 1e-7, "{g}");
        assert_eq!(gain(10.0, 0.0), 0.0);
    }

    #[test]
    fn at_unit_gain_the_network_neither_grows_nor_fades_and_below_it_falls_silent() {
        let lens = primes(&[101.0, 113.0, 127.0, 139.0, 151.0, 163.0, 173.0, 181.0]).map(|l| l as f32);
        let window = |f: &mut Fdn, g: f32| (0..8000).map(|_| f.tick(&[0.0; LINES], &lens, &[g; LINES], 1.0).iter().map(|v| f64::from(v * v)).sum::<f64>()).sum::<f64>();
        let mut f = Fdn::new(200);
        f.tick(&[0.5, -0.25, 0.5, -0.25, 0.5, -0.25, 0.5, -0.25], &lens, &[1.0; LINES], 1.0);
        let (early, _, late) = (window(&mut f, 1.0), window(&mut f, 1.0), window(&mut f, 1.0));
        assert!(early > 0.0 && (late / early - 1.0).abs() < 0.01, "lossless: {early} then {late}");
        for _ in 0..100 {
            window(&mut f, 0.99);
        }
        assert_eq!(f.tick(&[0.0; LINES], &lens, &[0.99; LINES], 1.0), [0.0; LINES], "the tail ends at zero, not in the denormals");
    }
}
