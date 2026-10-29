//! Module `synth`: audio blocks (mono, f32). Kept free of fundsp so the module builds for wasm32 with nothing but std;
//! the stage's richer fundsp units stay in deck/stage until they move here (#182 report §4).
use crate::module::{Block, KindInfo, Module, Params, SYNTH_KINDS};
use crate::math::{sin_turns, wrap01};
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
            "osc" => Box::new(Osc { phase: 0.0, rate }),
            "fm" => Box::new(Fm::new(rate)),
            _ => return None,
        })
    }
}

/// A sine oscillator: `freq` (Hz), `gain`.
pub struct Osc {
    phase: f32,
    rate: f32,
}

impl Block for Osc {
    fn audio(&mut self, i: &[f32], out: &mut [f32]) {
        let step = i[0] / self.rate;
        for x in out.iter_mut() {
            *x = i[1] * sin_turns(self.phase);
            self.phase = wrap01(self.phase + step);
        }
    }
}

/// Two-operator FM: `freq`, `ratio` (modulator / carrier), `index` (modulation depth), `gain`.
pub struct Fm {
    carrier: f32,
    modulator: f32,
    rate: f32,
}

impl Fm {
    pub fn new(rate: f32) -> Self {
        Fm { carrier: 0.0, modulator: 0.0, rate }
    }
}

impl Block for Fm {
    fn audio(&mut self, i: &[f32], out: &mut [f32]) {
        let (freq, ratio, index, gain) = (i[0], i[1], i[2], i[3]);
        let (cs, ms) = (freq / self.rate, freq * ratio / self.rate);
        for x in out.iter_mut() {
            let m = sin_turns(self.modulator) * index;
            *x = gain * sin_turns(self.carrier + m * (1.0 / TAU));
            self.carrier = wrap01(self.carrier + cs);
            self.modulator = wrap01(self.modulator + ms);
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
}
