//! `fm16` – the #306 benchmark's FM workload (agents/research/MODULE-BENCH.md): sixteen two-operator voices always
//! sounding, true FM as the engine's `fm` (the carrier's phase accumulates `f + index · f · sin(modulator)`, a negative
//! frequency runs it backward), the sines from `deck_module::math` (libm: the same bits native and wasm). Voice k plays
//! `base · (1 + k / 4)`; `ratio` sets the modulator, `index` the depth, `gain` the level; the voices spread over the
//! stereo field. No events, no allocation: what it costs is the DSP and the host's call.
#![allow(clippy::duplicated_attributes)]
use deck_module::{math, Block, Ctx, Module};

pub const VOICES: usize = 16;
const TAU: f32 = core::f32::consts::TAU;

#[derive(Module)]
#[module(
    name = "fm16",
    doc = "sixteen two-operator FM voices (the #306 benchmark)"
)]
#[port(output, audio, name = "out", channels = 2)]
pub struct Fm16 {
    #[param(range = 20.0..2000.0, default = 110.0, unit = "Hz", doc = "voice 0's pitch; voice k plays base · (1 + k / 4)")]
    base: f32,
    #[param(range = 0.0..16.0, default = 1.5, doc = "modulator frequency / carrier frequency")]
    ratio: f32,
    #[param(range = 0.0..10.0, default = 2.0, doc = "modulation depth")]
    index: f32,
    #[param(range = 0.0..1.0, default = 0.5, doc = "output level")]
    gain: f32,
    rate: f32,
    carrier: [f32; VOICES],
    modulator: [f32; VOICES],
}

/// a phase in cycles kept in [0, 1)
fn wrap(p: f32) -> f32 {
    p - math::floor(p)
}

impl Module for Fm16 {
    fn init(&mut self, ctx: &Ctx) {
        self.rate = ctx.rate() as f32;
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let n = block.frames();
        let inv = 1.0 / self.rate.max(1.0);
        let level = self.gain / VOICES as f32;
        let mut out = block.audio_out(Self::OUT);
        let (l, r) = out.stereo();
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        for k in 0..VOICES {
            let f = self.base * (1.0 + k as f32 / 4.0);
            let fm = f * self.ratio;
            let pan = k as f32 / (VOICES - 1) as f32;
            let (gl, gr) = (level * (1.0 - pan), level * pan);
            let (mut pc, mut pm) = (self.carrier[k], self.modulator[k]);
            for i in 0..n {
                let m = math::sin(TAU * pm);
                pm = wrap(pm + fm * inv);
                let s = math::sin(TAU * pc);
                pc = wrap(pc + (f + self.index * f * m) * inv);
                l[i] += s * gl;
                r[i] += s * gr;
            }
            (self.carrier[k], self.modulator[k]) = (pc, pm);
        }
    }
}
