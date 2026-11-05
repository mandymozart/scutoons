//! Maths from `+ − × ÷ floor` only – IEEE-exact and so identical on every target – instead of the platform's
//! `exp` / `sin` / `powf` / `tan` (ENGINE-REFACTOR-PLAN §3.4: native ≠ wasm there). Accuracy is far better than f32 needs.

pub const LN2: f64 = core::f64::consts::LN_2;
/// middle C in Hz: where key tracking is neutral
pub const C4: f64 = 261.625_565_300_598_6;

/// 2^x (x clamped to ±1000)
pub fn exp2(x: f64) -> f64 {
    let x = x.clamp(-1000.0, 1000.0);
    let i = x.floor();
    let z = (x - i) * LN2; // e^z, z in [0, ln 2): the Taylor series to z^14
    let mut p = 1.0;
    let mut k = 14.0;
    while k >= 1.0 {
        p = 1.0 + p * z / k;
        k -= 1.0;
    }
    p * f64::from_bits(((i as i64 + 1023) as u64) << 52)
}

/// log2(x) for x > 0 (anything else gives −1000)
pub fn log2(x: f64) -> f64 {
    if x <= 1e-300 || !x.is_finite() {
        return -1000.0;
    }
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023u64 << 52));
    if m > core::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // ln m = 2 (s + s³/3 + s⁵/5 + …), s = (m − 1) / (m + 1), |s| < 0.172
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut sum = 0.0;
    let mut k = 21.0;
    while k >= 1.0 {
        sum = 1.0 / k + s2 * sum;
        k -= 2.0;
    }
    e as f64 + 2.0 * s * sum / LN2
}

/// sin(2π t), t in turns (any value; the fold keeps full precision)
pub fn sin_turns(t: f64) -> f64 {
    let mut u = t - (t + 0.5).floor(); // [−0.5, 0.5)
    if u > 0.25 {
        u = 0.5 - u;
    } else if u < -0.25 {
        u = -0.5 - u;
    }
    let x = u * (2.0 * core::f64::consts::PI);
    let x2 = x * x;
    // x (1 − x²/6 (1 − x²/20 (1 − x²/42 (… x²/210)))): the Taylor series to x^15, error < 1e-11 at π/2
    let mut p = 1.0;
    let mut k = 15.0;
    while k >= 3.0 {
        p = 1.0 - p * x2 / (k * (k - 1.0));
        k -= 2.0;
    }
    x * p
}

/// cos(2π t)
pub fn cos_turns(t: f64) -> f64 {
    sin_turns(t + 0.25)
}

/// tan(π x) for x in turns of the sample rate, 0 < x < 0.5 (the filter's prewarp)
pub fn tan_pi(x: f64) -> f64 {
    sin_turns(x * 0.5) / cos_turns(x * 0.5)
}

/// A soft clip with a rational curve: x (27 + x²) / (27 + 9x²), ±1 from |x| = 3 on (tanh's near twin, no exp).
pub fn soft(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp2_matches_std() {
        let mut worst = 0.0f64;
        for i in -2000..=2000 {
            let x = i as f64 * 0.01;
            worst = worst.max(((exp2(x) - x.exp2()) / x.exp2()).abs());
        }
        assert!(worst < 1e-12, "{worst}");
        assert_eq!(exp2(0.0), 1.0);
        assert_eq!(exp2(3.0), 8.0);
        assert_eq!(exp2(-1.0), 0.5);
    }

    #[test]
    fn log2_matches_std() {
        let mut worst = 0.0f64;
        let mut x = 1e-6;
        while x < 1e7 {
            worst = worst.max((log2(x) - x.log2()).abs());
            x *= 1.0137;
        }
        assert!(worst < 1e-12, "{worst}");
        assert_eq!(log2(1.0), 0.0);
        assert_eq!(log2(1024.0), 10.0);
        assert_eq!(log2(0.0), -1000.0);
    }

    #[test]
    fn sin_and_tan_match_std() {
        let mut worst = 0.0f64;
        for i in -5000..=5000 {
            let t = i as f64 * 0.00137;
            worst = worst.max((sin_turns(t) - (t * core::f64::consts::TAU).sin()).abs());
            worst = worst.max((cos_turns(t) - (t * core::f64::consts::TAU).cos()).abs());
        }
        assert!(worst < 1e-10, "{worst}");
        for i in 1..450 {
            let x = i as f64 * 0.001;
            let want = (core::f64::consts::PI * x).tan();
            assert!(((tan_pi(x) - want) / want).abs() < 1e-9, "{x}");
        }
    }

    #[test]
    fn soft_is_odd_bounded_and_linear_near_zero() {
        assert_eq!(soft(0.0), 0.0);
        assert_eq!(soft(5.0), 1.0);
        assert_eq!(soft(-5.0), -1.0);
        assert!((soft(0.01) - 0.01).abs() < 1e-6);
        assert_eq!(soft(0.7), -soft(-0.7));
    }
}
