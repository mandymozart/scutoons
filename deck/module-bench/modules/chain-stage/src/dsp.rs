//! The wiring bench's DSP (#393), one file included by both modules, so a wired chain and a merged one run the same
//! code: `sections` state-variable low-pass sections in a row. Only + − ×, so the bits are the same native and in wasm.

/// the most sections one stage runs
pub const SECTIONS: usize = 16;
/// the frequency coefficient (about 1.2 kHz at 48 kHz)
const F: f32 = 0.156;
/// the damping
const Q: f32 = 0.9;

/// one stereo stage's state
#[derive(Clone, Copy, Default)]
pub struct Stage {
    low: [[f32; SECTIONS]; 2],
    band: [[f32; SECTIONS]; 2],
}

impl Stage {
    pub const fn new() -> Stage {
        Stage {
            low: [[0.0; SECTIONS]; 2],
            band: [[0.0; SECTIONS]; 2],
        }
    }

    /// one sample of channel `c` through the first `sections` sections
    #[inline(always)]
    pub fn tick(&mut self, c: usize, x: f32, sections: usize) -> f32 {
        let (low, band) = (&mut self.low[c], &mut self.band[c]);
        let mut y = x;
        for s in 0..sections {
            low[s] += F * band[s];
            let high = y - low[s] - Q * band[s];
            band[s] += F * high;
            y = low[s];
        }
        y
    }
}

/// the `sections` parameter as a count
#[inline(always)]
pub fn count(v: f32) -> usize {
    (v as usize).clamp(1, SECTIONS)
}
