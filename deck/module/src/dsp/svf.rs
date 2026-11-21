//! The state-variable filter (Andrew Simper's trapezoidal / zero-delay-feedback SVF): low-pass, band-pass, high-pass and
//! notch from one pair of integrators, stable under fast cutoff modulation, with resonance up to self-oscillation's edge
//! and a drive stage in front. Two ways in: `tick` with a `Coef` computed at control rate, `lowpass` with the cutoff and Q
//! of every sample.
use super::arith::{exp2, soft, tan_pi};
use super::flush;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Lp,
    Bp,
    Hp,
    Notch,
}

impl Mode {
    /// from a parameter value: 0 low-pass, 1 band-pass, 2 high-pass, 3 notch
    pub fn from_param(v: f32) -> Mode {
        match v.round() as i32 {
            0 => Mode::Lp,
            1 => Mode::Bp,
            2 => Mode::Hp,
            _ => Mode::Notch,
        }
    }
}

/// damping k = 1/Q for a resonance 0..1: Q 0.707 at 0, about 24 at 1 (exponential, so the knob feels even)
pub fn damping(res: f32) -> f64 {
    core::f64::consts::SQRT_2 * exp2(-5.1 * f64::from(res.clamp(0.0, 1.0)))
}

/// The coefficients for one cutoff and resonance; recomputed at control rate, not per sample.
#[derive(Debug, Clone, Copy, Default)]
pub struct Coef {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Coef {
    /// `cutoff` in Hz (kept between 10 Hz and 0.45 of the rate), `res` 0..1
    pub fn new(cutoff: f64, res: f32, rate: f64) -> Coef {
        let fc = cutoff.clamp(10.0, 0.45 * rate);
        let g = tan_pi(fc / rate);
        let k = damping(res);
        let a1 = 1.0 / (1.0 + g * (g + k));
        Coef { a1: a1 as f32, a2: (g * a1) as f32, a3: (g * g * a1) as f32, k: k as f32 }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    pub fn new() -> Svf {
        Svf::default()
    }

    pub fn reset(&mut self) {
        *self = Svf::default();
    }

    /// one sample through `mode` with the coefficients of a control-rate `Coef`
    #[inline(always)]
    pub fn tick(&mut self, c: &Coef, mode: Mode, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = flush(2.0 * v1 - self.ic1);
        self.ic2 = flush(2.0 * v2 - self.ic2);
        match mode {
            Mode::Lp => v2,
            Mode::Bp => v1,
            Mode::Hp => x - c.k * v1 - v2,
            Mode::Notch => x - c.k * v1,
        }
    }

    /// one sample through the lowpass at `cutoff` Hz (clamped under Nyquist) with `q` (0.5 = critically damped, 0.707 =
    /// Butterworth); `pi_over_rate` = π / sample rate. `g = tan(π fc / fs)` is the bilinear prewarp (libm's `tanf`), so
    /// the response is exactly the bilinear transform of 1 / (1 + s/Q + s²).
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
        self.ic1 = flush(2.0 * v1 - self.ic1);
        self.ic2 = flush(2.0 * v2 - self.ic2);
        v2
    }
}

/// The drive stage: 0 passes the signal untouched, 1 pushes it ten times harder into a soft clip (ceiling ±1); in
/// between it is a crossfade between the two, so the knob has no step at 0.
pub fn drive(x: f32, amount: f32) -> f32 {
    if amount <= 0.0 {
        return x;
    }
    let a = amount.min(1.0);
    let gain = 1.0 + 9.0 * a;
    x + a * (soft(x * gain) - x)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// the steady-state gain (linear) at `f` Hz for a filter at `fc` / `res`
    fn gain(mode: Mode, fc: f64, res: f32, f: f64) -> f64 {
        let c = Coef::new(fc, res, RATE);
        let mut s = Svf::default();
        let n = 48_000;
        let mut sum = 0.0f64;
        for i in 0..n {
            let x = (core::f64::consts::TAU * f * i as f64 / RATE).sin() as f32;
            let y = s.tick(&c, mode, x);
            if i >= n / 2 {
                sum += f64::from(y) * f64::from(y);
            }
        }
        (2.0 * sum / (n / 2) as f64).sqrt() // the amplitude, from the rms: a sample grid can miss the peak
    }

    /// the bilinear-transformed 2nd-order prototype: Ω = tan(π f / fs) / tan(π fc / fs)
    fn expected(mode: Mode, fc: f64, res: f32, f: f64) -> f64 {
        let w = (core::f64::consts::PI * f / RATE).tan() / (core::f64::consts::PI * fc / RATE).tan();
        let k = damping(res);
        let den = ((1.0 - w * w).powi(2) + (k * w).powi(2)).sqrt();
        match mode {
            Mode::Lp => 1.0 / den,
            Mode::Bp => w / den,
            Mode::Hp => w * w / den,
            Mode::Notch => (1.0 - w * w).abs() / den,
        }
    }

    #[test]
    fn all_four_modes_follow_their_analytic_response() {
        for (mode, res) in [(Mode::Lp, 0.0), (Mode::Lp, 0.5), (Mode::Bp, 0.3), (Mode::Hp, 0.0), (Mode::Hp, 0.6), (Mode::Notch, 0.2)] {
            for f in [100.0, 500.0, 1000.0, 2000.0, 8000.0] {
                let (got, want) = (gain(mode, 1000.0, res, f), expected(mode, 1000.0, res, f));
                let db = 20.0 * (got / want).log10();
                // a notch is a point of zero: judge it by the difference, not the ratio
                let ok = if mode == Mode::Notch && want < 0.02 { (got - want).abs() < 0.01 } else { db.abs() < 0.1 };
                assert!(ok, "{mode:?} res {res} at {f} Hz: got {got:.4}, want {want:.4} ({db:.2} dB)");
            }
        }
    }

    #[test]
    fn the_modes_have_the_shapes_they_are_named_for() {
        let at = |m: Mode, f: f64| gain(m, 1000.0, 0.0, f);
        // low-pass: through below the cutoff, −3 dB at it (Q 0.707), 12 dB an octave above (2nd order)
        assert!((at(Mode::Lp, 50.0) - 1.0).abs() < 0.01);
        assert!((at(Mode::Lp, 1000.0) - core::f64::consts::FRAC_1_SQRT_2).abs() < 0.01);
        assert!(at(Mode::Lp, 8000.0) < 0.02);
        // high-pass: the mirror image
        assert!(at(Mode::Hp, 50.0) < 0.01);
        assert!((at(Mode::Hp, 1000.0) - core::f64::consts::FRAC_1_SQRT_2).abs() < 0.01);
        assert!((at(Mode::Hp, 15000.0) - 1.0).abs() < 0.05);
        // band-pass: peaks at the cutoff, with gain 1/k there, and falls away on both sides
        let k = damping(0.0);
        assert!((at(Mode::Bp, 1000.0) - 1.0 / k).abs() < 0.02);
        assert!(at(Mode::Bp, 100.0) < 0.2 * at(Mode::Bp, 1000.0) && at(Mode::Bp, 10000.0) < 0.2 * at(Mode::Bp, 1000.0));
        // notch: through at both ends, a hole at the cutoff
        assert!((at(Mode::Notch, 50.0) - 1.0).abs() < 0.01 && (at(Mode::Notch, 15000.0) - 1.0).abs() < 0.05);
        assert!(at(Mode::Notch, 1000.0) < 0.01);
    }

    #[test]
    fn resonance_raises_the_peak_at_the_cutoff() {
        let peaks: Vec<f64> = [0.0, 0.3, 0.6, 0.9].iter().map(|&r| gain(Mode::Lp, 1000.0, r, 1000.0)).collect();
        assert!(peaks.windows(2).all(|w| w[1] > w[0] * 1.3), "{peaks:?}");
        assert!(peaks[3] > 8.0, "high resonance is a real peak: {peaks:?}");
    }

    #[test]
    fn a_cutoff_swept_hard_stays_finite_and_bounded() {
        let mut s = Svf::default();
        let mut peak = 0.0f32;
        for i in 0..96_000 {
            let fc = 20.0 * 1000f64.powf((i % 4800) as f64 / 4800.0); // saw sweep 20 Hz … 20 kHz
            let c = Coef::new(fc, 0.95, RATE);
            let x = if i % 200 < 100 { 1.0 } else { -1.0 };
            let y = s.tick(&c, Mode::Lp, x);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 60.0, "{peak}");
    }

    #[test]
    fn a_ringing_filter_fed_silence_ends_at_zero_not_in_the_denormals() {
        for res in [0.0, 0.5, 0.99] {
            let c = Coef::new(300.0, res, RATE);
            let (mut a, mut b) = (Svf::new(), Svf::new());
            let mut last = (1.0, 1.0);
            for i in 0..400_000 {
                let x = if i == 0 { 1.0 } else { 0.0 };
                last = (a.tick(&c, Mode::Bp, x), b.lowpass(x, 300.0, 0.7 + 20.0 * res, core::f32::consts::PI / 48_000.0));
                assert!((last.0 == 0.0 || last.0.is_normal()) && (last.1 == 0.0 || last.1.is_normal()), "res {res}, sample {i}: {last:?}");
            }
            assert_eq!(last, (0.0, 0.0), "res {res}");
        }
    }

    #[test]
    fn drive_is_a_bypass_at_zero_and_saturates_above() {
        assert_eq!(drive(0.8, 0.0), 0.8);
        let loud = drive(4.0, 1.0);
        assert!(loud.abs() <= 2.0 && loud > 0.5, "{loud}");
        assert!(drive(0.5, 0.01) > 0.49 && drive(0.5, 0.01) < 0.51, "continuous at 0");
        assert_eq!(drive(0.3, 0.7), -drive(-0.3, 0.7));
    }

    // `lowpass`: the cutoff and Q of every sample

    const RATE32: f32 = 48_000.0;
    const PI: f64 = core::f64::consts::PI;

    fn db(x: f64) -> f64 {
        20.0 * x.max(1e-30).log10()
    }

    /// the filter's gain at `f` for a sine, measured after it settles
    fn svf_gain(cutoff: f32, q: f32, f: f32) -> f64 {
        let mut s = Svf::new();
        let pr = core::f32::consts::PI / RATE32;
        let n = 48_000;
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for i in 0..n {
            let x = libm::sinf(core::f32::consts::TAU * f * i as f32 / RATE32);
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
        let fs = f64::from(RATE32);
        let g = libm::tan(PI * fc / fs);
        let w = libm::tan(PI * f / fs) / g;
        1.0 / ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt()
    }

    #[test]
    fn the_lowpass_is_the_bilinear_second_order_response() {
        for (fc, q) in [(1000.0f32, core::f32::consts::FRAC_1_SQRT_2), (1000.0, 2.0), (4000.0, 0.7), (200.0, 1.0)] {
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
        let pr = core::f32::consts::PI / RATE32;
        let mut w = super::super::noise::White::new(1);
        for i in 0..100_000 {
            let cutoff = if i % 2 == 0 { 20.0 } else { 40_000.0 };
            let y = s.lowpass(w.sample(), cutoff, 0.05 + (i % 7) as f32, pr);
            assert!(y.is_finite() && y.abs() < 1e3, "sample {i}: {y}");
        }
    }
}
