//! The state-variable filter (a trapezoidal / zero-delay-feedback SVF): low-pass, band-pass, high-pass and notch from
//! one pair of integrators, stable under fast cutoff modulation, with resonance up to self-oscillation's edge and a
//! drive stage in front.
use super::math::{exp2, soft, tan_pi};

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
    pub fn reset(&mut self) {
        *self = Svf::default();
    }

    pub fn tick(&mut self, c: &Coef, mode: Mode, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match mode {
            Mode::Lp => v2,
            Mode::Bp => v1,
            Mode::Hp => x - c.k * v1 - v2,
            Mode::Notch => x - c.k * v1,
        }
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
    fn drive_is_a_bypass_at_zero_and_saturates_above() {
        assert_eq!(drive(0.8, 0.0), 0.8);
        let loud = drive(4.0, 1.0);
        assert!(loud.abs() <= 2.0 && loud > 0.5, "{loud}");
        assert!(drive(0.5, 0.01) > 0.49 && drive(0.5, 0.01) < 0.51, "continuous at 0");
        assert_eq!(drive(0.3, 0.7), -drive(-0.3, 0.7));
    }
}
