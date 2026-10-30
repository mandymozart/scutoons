//! The core's own small math – no libm calls in a per-sample or per-node loop. Why: the platform's `sinf` / `roundf` /
//! `truncf` differ a lot (MSVC's CRT `sinf` took ~25 ns; without SSE4.1 – the x86-64 baseline – `round` and `fract` are
//! library calls too; wasm has its own), and the stage's audio must cost the same on every deck and in a browser worker (#182 report §4).
use std::f32::consts::TAU;

/// 1.5 · 2²³: adding and subtracting it rounds an f32 to the nearest integer (ties to even) in the FPU itself.
pub(crate) const MAGIC: f32 = 12_582_912.0;

/// `x` rounded to the nearest integer (ties to even), for |x| < 2²² – two float adds: no libm, no int cast (Rust's
/// saturating `as` costs compares on x86), no branch, so loops over it vectorise.
#[inline(always)]
pub fn round_fast(x: f32) -> f32 {
    // (wasm's own f32.nearest measured the same, 2026-10-30)
    (x + MAGIC) - MAGIC
}

/// A phase wrapped into 0 … 1 (|x| < 2²²; −0.25 → 0.75; a whole number may come out as 0 or 1 – the same phase).
#[inline(always)]
pub fn wrap01(x: f32) -> f32 {
    x - round_fast(x - 0.5)
}

/// `x` rounded toward zero, exactly like `f32::trunc` for |x| < 2²² – without the libm call baseline x86-64 makes
/// (no SSE4.1 `roundps` there), branch-free, so loops over it vectorise.
#[inline(always)]
pub fn trunc_fast(x: f32) -> f32 {
    let r = round_fast(x);
    // nearest rounding went away from zero: one step back toward it
    let back = if r.abs() > x.abs() { 1.0f32 } else { 0.0 };
    r - back.copysign(x)
}

/// `x - x.trunc()` – exactly `f32::fract` for |x| < 2²² (negative `x` → a negative fraction), see `trunc_fast`.
#[inline(always)]
pub fn fract_fast(x: f32) -> f32 {
    x - trunc_fast(x)
}

/// sin(2π·x) – the sine of `x` full turns, |x| < 2²². Range-reduced to a quarter turn without branches, then a
/// 11th-order odd polynomial: error < 2e-7 against the exact sine (−134 dB, f32's own rounding; the 9th order's 4e-6 –
/// #182 – let FM voices drift audibly far from fundsp's, #183). The f32 `(TAU * x).sin()` is itself off by ~4e-6 at 15
/// turns (TAU * x rounds).
#[inline(always)]
pub fn sin_turns(x: f32) -> f32 {
    let u = x - round_fast(x); // −0.5 … 0.5
    let q = u.abs().min(0.5 - u.abs()).copysign(u); // −0.25 … 0.25: sin(π − a) = sin(a)
    let a = TAU * q;
    let a2 = a * a;
    a * (1.0 + a2 * (-1.0 / 6.0 + a2 * (1.0 / 120.0 + a2 * (-1.0 / 5040.0 + a2 * (1.0 / 362_880.0 + a2 * (-1.0 / 39_916_800.0))))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_turns_matches_sin() {
        let worst = (-40_000..40_000)
            .map(|i| i as f32 * 0.000_37)
            .map(|x| (sin_turns(x) - (TAU * x).sin()).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "worst error {worst}");
        // against the exact sine (f64), over the turns a phase lives in
        let exact = (-20_000..20_000)
            .map(|i| i as f32 * 0.000_051)
            .map(|x| (sin_turns(x) as f64 - (std::f64::consts::TAU * x as f64).sin()).abs())
            .fold(0.0f64, f64::max);
        assert!(exact < 2e-7, "worst error against f64 {exact}");
    }

    #[test]
    fn rounding_and_wrapping() {
        assert_eq!(round_fast(2.4), 2.0);
        assert_eq!(round_fast(2.6), 3.0);
        assert_eq!(round_fast(-2.6), -3.0);
        assert_eq!(round_fast(-0.2), 0.0);
        assert!((wrap01(3.25) - 0.25).abs() < 1e-6);
        assert!(wrap01(0.0) == 0.0 || wrap01(0.0) == 1.0);
        assert!((wrap01(-0.25) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn trunc_and_fract_match_std_exactly() {
        let edge = [0.0f32, -0.0, 0.5, -0.5, 1.0, -1.0, 1.5, 2.5, -2.5, 3.5, 0.999_999_9, -0.999_999_9, 4_194_303.5];
        let sweep = (-30_000..30_000).map(|i| i as f32 * 0.000_731 + (i % 7) as f32 * 0.25);
        for x in edge.into_iter().chain(sweep) {
            assert_eq!(trunc_fast(x), x.trunc(), "trunc {x}");
            assert_eq!(fract_fast(x).to_bits() & !(1 << 31), x.fract().to_bits() & !(1 << 31), "fract {x}");
        }
    }
}
