//! One-pole lowpass and highpass (6 dB per octave): `y += k (x − y)`. The per-sample sibling of `Smooth`, which takes
//! the same step once a block.
use super::flush;
use crate::smooth::Smooth;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OnePole {
    y: f32,
}

impl OnePole {
    pub const fn new() -> OnePole {
        OnePole { y: 0.0 }
    }

    pub fn reset(&mut self) {
        self.y = 0.0;
    }

    /// the `k` whose −3 dB point is `cutoff` Hz at `rate` (the cutoff kept under Nyquist; 0 or less holds the value)
    pub fn coef(cutoff: f32, rate: f32) -> f32 {
        if cutoff <= 0.0 || rate <= 0.0 {
            return 0.0;
        }
        Smooth::coef(rate / (core::f32::consts::TAU * cutoff.min(0.5 * rate)), 1.0)
    }

    /// one sample of the lowpass
    #[inline(always)]
    pub fn lowpass(&mut self, x: f32, k: f32) -> f32 {
        self.y = flush(self.y + k * (x - self.y));
        self.y
    }

    /// one sample of the highpass: the input less its lowpass
    #[inline(always)]
    pub fn highpass(&mut self, x: f32, k: f32) -> f32 {
        x - self.lowpass(x, k)
    }

    pub fn value(&self) -> f32 {
        self.y
    }
}

#[cfg(test)]
mod tests {
    use super::super::probe::db;
    use super::*;

    const RATE: f32 = 48_000.0;

    /// the settled gain at `f` Hz of the lowpass (or the highpass) at `fc`
    fn gain(fc: f32, f: f32, high: bool) -> f64 {
        let k = OnePole::coef(fc, RATE);
        let mut p = OnePole::new();
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for i in 0..48_000 {
            let x = libm::sinf(core::f32::consts::TAU * f * i as f32 / RATE);
            let y = if high { p.highpass(x, k) } else { p.lowpass(x, k) };
            if i >= 24_000 {
                num += f64::from(y).powi(2);
                den += f64::from(x).powi(2);
            }
        }
        (num / den).sqrt()
    }

    #[test]
    fn the_lowpass_is_three_db_down_at_its_cutoff_and_falls_six_db_an_octave() {
        for fc in [100.0f32, 1000.0, 4000.0] {
            assert!((db(gain(fc, fc, false)) + 3.01).abs() < 0.15, "fc {fc}: {:.3} dB", db(gain(fc, fc, false)));
            assert!(db(gain(fc, fc / 20.0, false)).abs() < 0.05, "the passband is flat");
        }
        // far above the cutoff (and far under Nyquist): 6 dB an octave
        let slope = db(gain(100.0, 1600.0, false)) - db(gain(100.0, 800.0, false));
        assert!((slope + 6.02).abs() < 0.1, "{slope:.3} dB an octave");
    }

    #[test]
    fn the_highpass_mirrors_it() {
        // a (1 − z⁻¹) / (1 − a z⁻¹), a = 1 − k: its level at the cutoff, a little under −3 dB
        let (a, w) = (1.0 - f64::from(OnePole::coef(1000.0, RATE)), core::f64::consts::TAU * 1000.0 / f64::from(RATE));
        let want = (a * a * (2.0 - 2.0 * w.cos()) / (1.0 - 2.0 * a * w.cos() + a * a)).sqrt();
        assert!((db(gain(1000.0, 1000.0, true)) - db(want)).abs() < 0.02 && (db(want) + 3.6).abs() < 0.1, "{:.3} dB", db(want));
        // the passband tops out at 2a / (1 + a): −0.6 dB for this cutoff
        assert!(db(gain(1000.0, 12_000.0, true)).abs() < 0.7);
        let slope = db(gain(1000.0, 50.0, true)) - db(gain(1000.0, 25.0, true));
        assert!((slope - 6.02).abs() < 0.1, "{slope:.3} dB an octave");
    }

    #[test]
    fn the_coefficient_stays_in_range() {
        assert_eq!(OnePole::coef(0.0, RATE), 0.0);
        assert_eq!(OnePole::coef(-5.0, RATE), 0.0);
        let top = OnePole::coef(1.0e9, RATE);
        assert!(top > 0.9 && top < 1.0, "{top}");
        let mut p = OnePole::new();
        assert_eq!(p.lowpass(1.0, 1.0), 1.0);
        assert_eq!(p.lowpass(5.0, 0.0), 1.0);
    }

    #[test]
    fn a_decaying_state_never_stays_in_the_denormals() {
        let k = OnePole::coef(20.0, RATE);
        let mut p = OnePole::new();
        p.lowpass(1.0, 1.0);
        for _ in 0..200_000 {
            let y = p.lowpass(0.0, k);
            assert!(y == 0.0 || y.is_normal(), "{y:e}");
        }
        assert_eq!(p.value(), 0.0);
    }
}
