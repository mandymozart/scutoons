//! `F8` – eight f32 lanes, the vector the batch loops compute in (#183). Every operation is one fixed 8-wide loop of
//! a single kind of instruction, which the compiler always turns into vector code: 2 × 4 lanes (SSE) on baseline
//! x86-64, 1 × 8 inside an AVX2 `#[target_feature]` function, NEON / wasm simd128 elsewhere – no intrinsics, so the
//! same code builds everywhere. Why not plain loops over the voices: a long loop body (two sines, an envelope) is
//! vectorised or not at the optimiser's whim (#183: the same FM step ran 8× slower when LLVM chose not to); op by op it
//! always is. The functions mirror `math` lane by lane, bit for bit (for inputs without NaN).
use crate::math::MAGIC;
use std::ops::{Add, Mul, Sub};

pub const LANES: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct F8(pub [f32; LANES]);

/// `f` applied lane by lane – one short loop of one operation
#[inline(always)]
fn map(a: F8, f: impl Fn(f32) -> f32) -> F8 {
    let mut r = [0.0; LANES];
    for (r, a) in r.iter_mut().zip(a.0) {
        *r = f(a);
    }
    F8(r)
}

#[inline(always)]
fn zip(a: F8, b: F8, f: impl Fn(f32, f32) -> f32) -> F8 {
    let mut r = [0.0; LANES];
    for ((r, a), b) in r.iter_mut().zip(a.0).zip(b.0) {
        *r = f(a, b);
    }
    F8(r)
}

impl F8 {
    #[inline(always)]
    pub fn splat(x: f32) -> F8 {
        F8([x; LANES])
    }

    #[inline(always)]
    pub fn abs(self) -> F8 {
        map(self, f32::abs)
    }

    /// the smaller lane (as `f32::min` without its NaN rule – one `minps`)
    #[inline(always)]
    pub fn min(self, o: F8) -> F8 {
        zip(self, o, |a, b| if a < b { a } else { b })
    }

    /// each lane's magnitude with `sign`'s sign
    #[inline(always)]
    pub fn copysign(self, sign: F8) -> F8 {
        zip(self, sign, f32::copysign)
    }

    /// per lane: `self < o` ? `yes` : `no`
    #[inline(always)]
    pub fn lt_select(self, o: F8, yes: F8, no: F8) -> F8 {
        let mut r = [0.0; LANES];
        for (l, r) in r.iter_mut().enumerate() {
            *r = if self.0[l] < o.0[l] { yes.0[l] } else { no.0[l] };
        }
        F8(r)
    }
}

impl Add for F8 {
    type Output = F8;
    #[inline(always)]
    fn add(self, o: F8) -> F8 {
        zip(self, o, |a, b| a + b)
    }
}

impl Sub for F8 {
    type Output = F8;
    #[inline(always)]
    fn sub(self, o: F8) -> F8 {
        zip(self, o, |a, b| a - b)
    }
}

impl Mul for F8 {
    type Output = F8;
    #[inline(always)]
    fn mul(self, o: F8) -> F8 {
        zip(self, o, |a, b| a * b)
    }
}

/// `math::round_fast` per lane
#[inline(always)]
pub fn round_fast(x: F8) -> F8 {
    let m = F8::splat(MAGIC);
    (x + m) - m
}

/// `math::wrap01` per lane
#[inline(always)]
pub fn wrap01(x: F8) -> F8 {
    x - round_fast(x - F8::splat(0.5))
}

/// `math::sin_turns` per lane
#[inline(always)]
pub fn sin_turns(x: F8) -> F8 {
    let u = x - round_fast(x);
    let a = u.abs();
    let q = a.min(F8::splat(0.5) - a).copysign(u);
    let t = F8::splat(std::f32::consts::TAU) * q;
    let t2 = t * t;
    let c = |k: f32| F8::splat(k);
    t * (c(1.0)
        + t2 * (c(-1.0 / 6.0) + t2 * (c(1.0 / 120.0) + t2 * (c(-1.0 / 5040.0) + t2 * (c(1.0 / 362_880.0) + t2 * c(-1.0 / 39_916_800.0))))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math;

    #[test]
    fn lanes_match_the_scalar_functions_bit_for_bit() {
        for k in -3000..3000 {
            let x = F8(std::array::from_fn(|l| k as f32 * 0.003_71 + l as f32 * 0.125));
            for l in 0..LANES {
                let s = x.0[l];
                assert_eq!(sin_turns(x).0[l].to_bits(), math::sin_turns(s).to_bits(), "sin {s}");
                assert_eq!(wrap01(x).0[l].to_bits(), math::wrap01(s).to_bits(), "wrap {s}");
                assert_eq!(round_fast(x).0[l].to_bits(), math::round_fast(s).to_bits(), "round {s}");
            }
        }
        let (a, b) = (F8([1.0, -2.0, 3.0, 0.5, 0.0, -0.0, 7.0, -7.0]), F8::splat(0.75));
        assert_eq!(a.min(b).0, [0.75, -2.0, 0.75, 0.5, 0.0, -0.0, 0.75, -7.0]);
        assert_eq!(a.lt_select(b, F8::splat(1.0), F8::splat(2.0)).0, [2.0, 1.0, 2.0, 1.0, 1.0, 1.0, 2.0, 1.0]);
    }
}
