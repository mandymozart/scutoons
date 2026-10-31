//! Module `synth`: audio blocks (mono, f32). Kept free of fundsp so the module builds for wasm32 with nothing but std;
//! the stage's richer fundsp units stay in deck/stage until they move here (#182 report §4).
use crate::module::{Block, KindInfo, Module, Params, SYNTH_KINDS};
use crate::math::{sin_turns, wrap01};
use crate::smooth::{Ramp, GLIDE, QUICK};
use std::f32::consts::TAU;

pub struct SynthModule;

impl Module for SynthModule {
    fn name(&self) -> &'static str {
        "synth"
    }
    fn kinds(&self) -> &'static [KindInfo] {
        SYNTH_KINDS
    }
    fn build(&self, kind: &str, _params: &Params, rate: f32) -> Option<Box<dyn Block>> {
        Some(match kind {
            "osc" => Box::new(Osc::new(rate)),
            "fm" => Box::new(Fm::new(rate)),
            _ => return None,
        })
    }
}

/// Each input's smoother (#188): `GLIDE` for the first `glides` inputs (frequency … gain), `QUICK` for the others;
/// they start where the first block's inputs are.
pub struct Inputs<const N: usize> {
    ramps: [Ramp; N],
    fresh: bool,
}

impl<const N: usize> Inputs<N> {
    pub fn new(rate: f32, taus: [f32; N]) -> Self {
        Inputs { ramps: taus.map(|t| Ramp::new(0.0, t, rate)), fresh: true }
    }

    /// this block's (first value, step per sample) of every input
    #[inline]
    pub fn block(&mut self, i: &[f32], n: usize) -> [(f32, f32); N] {
        if self.fresh {
            self.fresh = false;
            for (r, v) in self.ramps.iter_mut().zip(i) {
                r.jump(*v);
            }
        }
        std::array::from_fn(|k| self.ramps[k].block(i[k], n))
    }
}

/// a block's input at sample `s`: `from + step · s` (the same in the patcher and the batch – bit for bit)
#[inline(always)]
pub fn at((from, step): (f32, f32), s: usize) -> f32 {
    from + step * s as f32
}

/// A sine oscillator: `freq` (Hz), `gain` – both glide (smooth.rs).
pub struct Osc {
    phase: f32,
    rate: f32,
    inputs: Inputs<2>,
}

impl Osc {
    pub fn new(rate: f32) -> Self {
        Osc { phase: 0.0, rate, inputs: Inputs::new(rate, [GLIDE, GLIDE]) }
    }
}

impl Block for Osc {
    fn audio(&mut self, i: &[f32], out: &mut [f32]) {
        let [f, g] = self.inputs.block(i, out.len());
        for (s, x) in out.iter_mut().enumerate() {
            *x = at(g, s) * sin_turns(self.phase);
            self.phase = wrap01(self.phase + at(f, s) / self.rate);
        }
    }
}

/// Two-operator FM: `freq`, `ratio` (modulator / carrier), `index` (modulation depth), `gain` – each glides.
pub struct Fm {
    carrier: f32,
    modulator: f32,
    rate: f32,
    inputs: Inputs<4>,
}

impl Fm {
    pub fn new(rate: f32) -> Self {
        Fm { carrier: 0.0, modulator: 0.0, rate, inputs: Inputs::new(rate, [GLIDE, QUICK, QUICK, GLIDE]) }
    }
}

impl Block for Fm {
    fn audio(&mut self, i: &[f32], out: &mut [f32]) {
        let [f, r, k, g] = self.inputs.block(i, out.len());
        for (s, x) in out.iter_mut().enumerate() {
            let freq = at(f, s);
            let m = sin_turns(self.modulator) * at(k, s);
            *x = at(g, s) * sin_turns(self.carrier + m * (1.0 / TAU));
            self.carrier = wrap01(self.carrier + freq / self.rate);
            self.modulator = wrap01(self.modulator + freq * at(r, s) / self.rate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc_is_a_sine_at_its_frequency() {
        let mut o = SynthModule.build("osc", &Params::new(), 48_000.0).unwrap();
        let mut buf = vec![0.0; 480];
        o.audio(&[100.0, 0.5], &mut buf);
        // 100 Hz at 48 kHz: a quarter period is 120 samples
        assert!((buf[120] - 0.5).abs() < 1e-3, "{}", buf[120]);
        let crossings = buf.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert_eq!(crossings, 0, "one period in 480 samples has no upward crossing after the first sample");
    }

    #[test]
    fn fm_stays_in_gain() {
        let mut f = SynthModule.build("fm", &Params::new(), 48_000.0).unwrap();
        let mut buf = vec![0.0; 4800];
        f.audio(&[220.0, 2.0, 3.0, 0.25], &mut buf);
        assert!(buf.iter().all(|x| x.abs() <= 0.2501));
        assert!(buf.iter().any(|x| x.abs() > 0.2));
    }

    /// #188: a gain jumped between blocks glides in – no step between samples beyond the tone's own (a 50 Hz sine at
    /// gain 1 steps by ≤ 2π · 50 / 48000 ≈ 0.0065); held flat it would jump by the sine's value at once (≈ 0.74 here)
    #[test]
    fn a_jumped_input_glides_across_the_block() {
        for kind in ["osc", "fm"] {
            let mut o = SynthModule.build(kind, &Params::new(), 48_000.0).unwrap();
            let ins = |gain: f32| if kind == "osc" { vec![50.0, gain] } else { vec![50.0, 1.0, 0.0, gain] };
            let mut buf = vec![0.0; 128];
            let mut all = Vec::new();
            o.audio(&ins(0.0), &mut buf);
            all.extend_from_slice(&buf);
            for _ in 0..40 {
                o.audio(&ins(1.0), &mut buf);
                all.extend_from_slice(&buf);
            }
            let worst = all.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
            assert!(worst < 0.0075, "{kind}: the largest step {worst}");
            assert!(all[all.len() - 400..].iter().any(|x| x.abs() > 0.95), "{kind}: settled at full gain");
        }
    }
}
