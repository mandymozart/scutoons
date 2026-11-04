//! Deterministic maths for modules (ABI-V2 §6): the pure-Rust `libm` – the same bits native and wasm, so a module's
//! native `testing::Rig` run equals its wasm run. `f32::sin` and friends use the platform's libm natively (the build
//! service flags them, #344). `f32` here; the f64 functions are `math::libm::*`.
pub use libm;

macro_rules! unary {
    ($($name:ident => $f:ident),* $(,)?) => {$(
        #[inline]
        pub fn $name(x: f32) -> f32 {
            libm::$f(x)
        }
    )*};
}

unary! {
    sin => sinf, cos => cosf, tan => tanf, asin => asinf, acos => acosf, atan => atanf,
    sinh => sinhf, cosh => coshf, tanh => tanhf,
    exp => expf, exp2 => exp2f, ln => logf, log2 => log2f, log10 => log10f,
    sqrt => sqrtf, cbrt => cbrtf, floor => floorf, ceil => ceilf, round => roundf, trunc => truncf, abs => fabsf,
}

#[inline]
pub fn pow(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}

#[inline]
pub fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

/// the remainder of `x / y` with the sign of `x`
#[inline]
pub fn fmod(x: f32, y: f32) -> f32 {
    libm::fmodf(x, y)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_is_libm() {
        assert_eq!(super::exp2(3.0), 8.0);
        assert_eq!(super::pow(2.0, 0.5).to_bits(), libm::powf(2.0, 0.5).to_bits());
    }
}
