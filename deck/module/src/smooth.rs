//! `Smooth`: a one-pole step toward a target, once per block (ABI-V2 §2: the host delivers one value per block and does
//! not smooth – the module does, in the domain its DSP wants). `to` sets where it heads, `step(k)` moves a fraction `k`
//! of the way and returns the value; `coef` turns a time constant into that fraction. Never allocates.
//! (A per-sample ramp across a block is deck-core's `Ramp`; it moves into the SDK when deck-core is next touched,
//! instead of a second copy here.)

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Smooth {
    value: f32,
    target: f32,
}

impl Smooth {
    /// at `value`, heading nowhere
    pub const fn new(value: f32) -> Smooth {
        Smooth { value, target: value }
    }

    /// head toward `target` (a non-finite target is ignored)
    pub fn to(&mut self, target: f32) {
        if target.is_finite() {
            self.target = target;
        }
    }

    /// move `k` (0 … 1) of the remaining way; the new value
    pub fn step(&mut self, k: f32) -> f32 {
        self.value += (self.target - self.value) * k.clamp(0.0, 1.0);
        self.value
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    /// there at once
    pub fn jump(&mut self, value: f32) {
        if value.is_finite() {
            self.value = value;
            self.target = value;
        }
    }

    /// the `k` for a time constant of `tau` s at steps `dt` s apart (one block: `frames / rate`); `tau` ≤ 0 = 1
    pub fn coef(tau: f32, dt: f32) -> f32 {
        if tau <= 0.0 {
            1.0
        } else {
            1.0 - libm::expf(-dt / tau)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Smooth;

    #[test]
    fn steps_converge_and_jump_is_immediate() {
        let mut s = Smooth::new(0.0);
        s.to(1.0);
        assert_eq!(s.step(0.5), 0.5);
        assert_eq!(s.step(0.5), 0.75);
        s.to(f32::NAN);
        assert_eq!(s.target(), 1.0);
        s.jump(-2.0);
        assert_eq!((s.value(), s.step(0.3)), (-2.0, -2.0));
        assert_eq!(Smooth::coef(0.0, 0.01), 1.0);
        assert!((Smooth::coef(0.1, 0.1) - (1.0 - (-1.0f32).exp())).abs() < 1e-6);
    }
}
