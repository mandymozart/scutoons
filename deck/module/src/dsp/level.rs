//! A level parameter in decibels as the factor a signal is multiplied with. A gain knob's range starts at `OFF_DB`, and
//! there it is silence, so a fader pulled down closes.
use super::arith::exp2;

/// a gain parameter's bottom: at and below it the level is 0
pub const OFF_DB: f32 = -60.0;

/// log2(10) / 20: decibels as octaves of amplitude
const OCT_PER_DB: f64 = 0.166_096_404_744_368_12;

/// the factor for `db` decibels (0 at `OFF_DB` and below, and for NaN)
pub fn gain(db: f32) -> f32 {
    if db > OFF_DB {
        exp2(f64::from(db) * OCT_PER_DB) as f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decibels_become_factors_and_the_bottom_is_silence() {
        assert_eq!(gain(0.0), 1.0);
        assert!((gain(20.0) - 10.0).abs() < 1e-5 && (gain(-20.0) - 0.1).abs() < 1e-7);
        assert!((gain(6.0) - 1.995_262_3).abs() < 1e-6);
        assert_eq!((gain(OFF_DB), gain(-200.0), gain(f32::NAN)), (0.0, 0.0, 0.0));
        assert!(gain(OFF_DB + 0.01) > 0.0009);
    }
}
