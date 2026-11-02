// What a new module starts as: the SDK's smallest instrument (deck/module/README.md) – a sine with a pitch and a level.
import type { ModuleFiles } from './types.ts';

export const STARTER_FILES: ModuleFiles = {
	'lib.rs': `use deck_module::{Block, Ctx, Module};

#[derive(Module)]
#[module(name = "sine")]
#[port(output, audio, name = "out")]
pub struct Sine {
    /// Hz
    #[param(range = 20.0..2000.0, default = 220.0)]
    freq: f32,
    #[param(range = 0.0..1.0, default = 0.2)]
    gain: f32,
    phase: f32,
}

impl Module for Sine {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let step = self.freq / ctx.rate() as f32;
        let n = block.frames();
        let (l, r) = block.audio_out(0);
        for i in 0..n {
            let s = (self.phase * std::f32::consts::TAU).sin() * self.gain;
            l[i] = s;
            r[i] = s;
            self.phase = (self.phase + step).fract();
        }
    }
}
`
};
