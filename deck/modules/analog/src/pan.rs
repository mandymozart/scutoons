//! The pan law: constant power. Centre is −3 dB on each side, so a source keeps its loudness across the field.
use super::math::{cos_turns, sin_turns};

/// (left, right) gains for `pan` in −1 (left) … 1 (right): cos θ, sin θ with θ = (pan + 1) π/4
pub fn gains(pan: f32) -> (f32, f32) {
    let t = (f64::from(pan.clamp(-1.0, 1.0)) + 1.0) * 0.125; // θ in turns
    (cos_turns(t) as f32, sin_turns(t) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_is_constant_across_the_field() {
        for i in -100..=100 {
            let (l, r) = gains(i as f32 / 100.0);
            assert!((l * l + r * r - 1.0).abs() < 1e-6, "pan {i}: {l} {r}");
        }
    }

    #[test]
    fn ends_and_centre() {
        let (l, r) = gains(-1.0);
        assert!((l - 1.0).abs() < 1e-6 && r.abs() < 1e-6);
        let (l, r) = gains(1.0);
        assert!(l.abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
        let (l, r) = gains(0.0);
        assert!((l - core::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6 && (r - l).abs() < 1e-7, "−3 dB each side");
        assert_eq!(gains(7.0), gains(1.0), "clamped");
    }
}
