//! The delay line and the two feedback units built on it: the Schroeder allpass and the damped comb (the reverb's and
//! the echo's building blocks). The buffer is allocated in `new` – call it in `init` – and never again. In one sample:
//! read, then write.
use super::flush;
use super::onepole::OnePole;

/// A ring of the last `max` samples. `read(d)` before this sample's `write` is the input `d` samples ago (1 ≤ d ≤ max).
#[derive(Debug, Clone)]
pub struct Delay {
    buf: Box<[f32]>,
    mask: usize,
    at: usize,
    max: usize,
}

impl Delay {
    /// room for delays up to `max` samples (allocates)
    pub fn new(max: usize) -> Delay {
        let max = max.max(1);
        // two more than the longest delay: the cubic read looks that far past it
        let len = (max + 3).next_power_of_two();
        Delay { buf: vec![0.0; len].into_boxed_slice(), mask: len - 1, at: 0, max }
    }

    /// the longest delay, in samples
    pub fn max(&self) -> usize {
        self.max
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }

    #[inline(always)]
    fn ago(&self, d: usize) -> f32 {
        self.buf[self.at.wrapping_add(1).wrapping_sub(d) & self.mask]
    }

    #[inline(always)]
    pub fn write(&mut self, x: f32) {
        self.at = (self.at + 1) & self.mask;
        self.buf[self.at] = x;
    }

    /// the input `d` samples ago (kept in 1 … `max`)
    #[inline(always)]
    pub fn read(&self, d: usize) -> f32 {
        self.ago(d.clamp(1, self.max))
    }

    /// the input `d` samples ago for a fractional `d` (kept in 1 … `max`): a cubic through the four samples around it
    #[inline(always)]
    pub fn read_frac(&self, d: f32) -> f32 {
        let d = if d.is_finite() { d.clamp(1.0, self.max as f32) } else { 1.0 };
        let i = d as usize;
        let f = d - i as f32;
        let (a, b, c, e) = (self.ago((i - 1).max(1)), self.ago(i), self.ago(i + 1), self.ago(i + 2));
        let c1 = 0.5 * (c - a);
        let c2 = a - 2.5 * b + 2.0 * c - 0.5 * e;
        let c3 = 0.5 * (e - a) + 1.5 * (b - c);
        ((c3 * f + c2) * f + c1) * f + b
    }
}

/// The Schroeder allpass: every frequency passes at its level, smeared in time – (z^−d − g) / (1 − g z^−d).
#[derive(Debug, Clone)]
pub struct Allpass {
    line: Delay,
}

impl Allpass {
    pub fn new(max: usize) -> Allpass {
        Allpass { line: Delay::new(max) }
    }

    /// one sample through a loop of `d` samples with gain `g` (kept inside ±0.999)
    #[inline(always)]
    pub fn tick(&mut self, x: f32, d: usize, g: f32) -> f32 {
        let g = g.clamp(-0.999, 0.999);
        let y = self.line.read(d) - g * x;
        self.line.write(flush(x + g * y));
        y
    }

    pub fn clear(&mut self) {
        self.line.clear();
    }
}

/// The feedback comb with a lowpass in its loop: an echo every `d` samples, each `feedback` of the last and duller.
#[derive(Debug, Clone)]
pub struct Comb {
    line: Delay,
    damp: OnePole,
}

impl Comb {
    pub fn new(max: usize) -> Comb {
        Comb { line: Delay::new(max), damp: OnePole::new() }
    }

    /// one sample: the loop's output `d` samples late; `feedback` is kept inside ±0.999, `damp` is the loop lowpass's
    /// `OnePole::coef` (1 = none)
    #[inline(always)]
    pub fn tick(&mut self, x: f32, d: usize, feedback: f32, damp: f32) -> f32 {
        let y = self.line.read(d);
        let back = self.damp.lowpass(y, damp);
        self.line.write(flush(x + back * feedback.clamp(-0.999, 0.999)));
        y
    }

    pub fn clear(&mut self) {
        self.line.clear();
        self.damp.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::super::probe::{db, fft};
    use super::*;

    const N: usize = 16_384;

    /// the magnitude response (bins 0..N/2) from an impulse response of N samples
    fn response(mut tick: impl FnMut(f32) -> f32) -> Vec<f64> {
        let mut re: Vec<f64> = (0..N).map(|i| f64::from(tick(if i == 0 { 1.0 } else { 0.0 }))).collect();
        let mut im = vec![0.0f64; N];
        fft(&mut re, &mut im);
        (0..N / 2).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt()).collect()
    }

    #[test]
    fn a_whole_delay_is_exact_at_every_length() {
        let mut line = Delay::new(1000);
        assert_eq!(line.max(), 1000);
        for i in 0..5000 {
            for d in [1usize, 2, 137, 999, 1000] {
                let want = if i >= d { (i - d) as f32 } else { 0.0 };
                assert_eq!(line.read(d), want, "sample {i}, delay {d}");
            }
            assert_eq!(line.read(0), line.read(1), "kept in range");
            assert_eq!(line.read(5000), line.read(1000), "kept in range");
            line.write(i as f32);
        }
        line.clear();
        assert_eq!(line.read(1), 0.0);
    }

    #[test]
    fn a_fractional_delay_lands_between_the_samples() {
        let rate = 48_000.0f64;
        let mut line = Delay::new(512);
        for (hz, bound) in [(200.0f64, 2e-5), (1000.0, 5e-4), (4000.0, 2e-2)] {
            let mut worst = 0.0f64;
            for i in 0..4096 {
                let t = i as f64;
                if i > 600 {
                    for d in [1.0f32, 2.5, 100.37, 300.999, 512.0] {
                        let want = (core::f64::consts::TAU * hz * (t - f64::from(d)) / rate).sin();
                        worst = worst.max((f64::from(line.read_frac(d)) - want).abs());
                    }
                }
                line.write((core::f64::consts::TAU * hz * t / rate).sin() as f32);
            }
            assert!(worst < bound, "{hz} Hz: off by {worst:e}");
        }
        // on a whole delay it is the sample itself
        for d in [1usize, 2, 77, 512] {
            assert_eq!(line.read_frac(d as f32), line.read(d));
        }
        assert_eq!(line.read_frac(f32::NAN), line.read(1));
        assert_eq!(line.read_frac(1.0e9), line.read(512));
    }

    #[test]
    fn the_allpass_passes_every_frequency_at_its_level() {
        for (d, g) in [(113usize, 0.5f32), (37, -0.7), (441, 0.6)] {
            let mut a = Allpass::new(500);
            let h = response(|x| a.tick(x, d, g));
            let worst = h.iter().map(|v| db(*v).abs()).fold(0.0, f64::max);
            assert!(worst < 0.01, "d {d} g {g}: {worst:.4} dB from flat");
        }
        // but not in time: the first echo is 1 − g² at d
        let mut a = Allpass::new(500);
        let y: Vec<f32> = (0..300).map(|i| a.tick(if i == 0 { 1.0 } else { 0.0 }, 100, 0.5)).collect();
        assert_eq!((y[0], y[100], y[200]), (-0.5, 0.75, 0.375));
    }

    #[test]
    fn the_comb_echoes_decay_by_the_feedback_and_take_their_time_to_60_db() {
        let (d, fb) = (480usize, 0.8f32);
        let mut c = Comb::new(480);
        let y: Vec<f32> = (0..48_000).map(|i| c.tick(if i == 0 { 1.0 } else { 0.0 }, d, fb, 1.0)).collect();
        for k in 1..20 {
            let want = fb.powi(k as i32 - 1);
            assert!((y[k * d] - want).abs() < 1e-5 * want.max(1e-3), "echo {k}: {} against {want}", y[k * d]);
            assert_eq!(y[k * d + 1], 0.0, "and silence between the echoes");
        }
        // −60 dB after ln(0.001) / ln(fb) echoes = 30.96: the 32nd echo (fb^31) is the first under it
        let under = (1..100).find(|k| y[k * d] < 0.001).unwrap();
        assert_eq!(under, 32);
    }

    #[test]
    fn the_comb_has_its_peaks_on_the_loop_s_harmonics_and_damping_dulls_the_high_ones() {
        let (d, fb) = (64usize, 0.7f32);
        let mut c = Comb::new(64);
        let h = response(|x| c.tick(x, d, fb, 1.0));
        let step = N / d; // the loop's frequency in bins
        for k in [1usize, 2, 10, 31] {
            assert!((h[k * step] - 1.0 / (1.0 - f64::from(fb))).abs() < 1e-3, "peak {k}: {}", h[k * step]);
            assert!((h[k * step + step / 2] - 1.0 / (1.0 + f64::from(fb))).abs() < 1e-3, "trough {k}");
        }
        let mut c = Comb::new(64);
        let damped = response(|x| c.tick(x, d, fb, 0.3));
        assert!((damped[0] - h[0]).abs() < 1e-3, "the lows keep their decay");
        assert!(damped[31 * step] < 0.5 * h[31 * step], "the highs lose theirs: {} against {}", damped[31 * step], h[31 * step]);
    }

    #[test]
    fn the_feedback_is_kept_under_one_and_the_tail_ends_at_zero() {
        let mut c = Comb::new(8);
        let mut a = Allpass::new(8);
        let mut peak = 0.0f32;
        for i in 0..400_000 {
            let x = if i < 100 { 1.0 } else { 0.0 };
            let (y, z) = (c.tick(x, 8, 5.0, 0.5), a.tick(x, 8, 5.0));
            peak = peak.max(y.abs()).max(z.abs());
            assert!((y == 0.0 || y.is_normal()) && (z == 0.0 || z.is_normal()), "sample {i}: {y:e} {z:e}");
        }
        assert!(peak < 20.0, "{peak}");
        for _ in 0..8000 {
            c.tick(0.0, 8, 0.5, 1.0);
            a.tick(0.0, 8, 0.5);
        }
        assert_eq!((c.tick(0.0, 8, 0.5, 1.0), a.tick(0.0, 8, 0.5)), (0.0, 0.0), "silent in the end");
    }
}
