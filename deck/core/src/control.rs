//! Module `control`: the control domain's small blocks – one f32 per port per frame.
use crate::math::{sin_turns, wrap01};
use crate::module::{param, Block, KindInfo, Module, Params, CONTROL_KINDS};

pub struct ControlModule;

impl Module for ControlModule {
    fn name(&self) -> &'static str {
        "control"
    }
    fn kinds(&self) -> &'static [KindInfo] {
        CONTROL_KINDS
    }
    fn build(&self, kind: &str, params: &Params, _rate: f32) -> Option<Box<dyn Block>> {
        Some(match kind {
            "lfo" => Box::new(Lfo { phase: param(params, "phase", 0.0) }),
            "add" => Box::new(Math(|a, b| a + b)),
            "mul" => Box::new(Math(|a, b| a * b)),
            "smooth" => Box::new(Smooth { y: 0.0 }),
            _ => return None,
        })
    }
}

/// A sine LFO: `rate` in Hz, `depth` the amplitude.
pub struct Lfo {
    phase: f32,
}

impl Block for Lfo {
    fn control(&mut self, i: &[f32], o: &mut [f32], dt: f32) {
        self.phase = wrap01(self.phase + i[0] * dt);
        o[0] = i[1] * sin_turns(self.phase);
    }
}

pub struct Math(fn(f32, f32) -> f32);

impl Block for Math {
    fn control(&mut self, i: &[f32], o: &mut [f32], _dt: f32) {
        o[0] = (self.0)(i[0], i[1]);
    }
}

/// One-pole smoothing towards `in` with the time constant `time` (s).
pub struct Smooth {
    y: f32,
}

impl Block for Smooth {
    fn control(&mut self, i: &[f32], o: &mut [f32], dt: f32) {
        let k = if i[1] <= 0.0 { 1.0 } else { 1.0 - (-dt / i[1]).exp() };
        self.y += (i[0] - self.y) * k;
        o[0] = self.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfo_quarter_turn() {
        let mut l = ControlModule.build("lfo", &Params::new(), 48_000.0).unwrap();
        let mut o = [0.0];
        l.control(&[1.0, 2.0], &mut o, 0.25);
        assert!((o[0] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn smooth_converges() {
        let mut s = ControlModule.build("smooth", &Params::new(), 48_000.0).unwrap();
        let mut o = [0.0];
        for _ in 0..600 {
            s.control(&[1.0, 0.1], &mut o, 1.0 / 60.0);
        }
        assert!((o[0] - 1.0).abs() < 1e-3);
    }
}
