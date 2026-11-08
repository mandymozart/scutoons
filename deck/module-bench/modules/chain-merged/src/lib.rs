//! `chain_merged` – the wiring bench's hard-wired chain (#393, agents/research/NATIVE-EDITOR-GUI.md §8): `stages` copies
//! of chain_stage's DSP in one module, each running a whole block before the next, exactly the order the wired chain
//! has, so the output bits must match it. Only the crossings between modules are gone.
#![allow(clippy::duplicated_attributes)]
use deck_module::{Block, Ctx, Module};

#[path = "../../chain-stage/src/dsp.rs"]
mod dsp;

pub const STAGES: usize = 16;

#[derive(Module)]
#[module(
    name = "chain_merged",
    doc = "the wiring bench's stages merged into one module (#393)"
)]
#[port(input, audio, name = "in", channels = 2)]
#[port(output, audio, name = "out", channels = 2)]
pub struct ChainMerged {
    #[param(range = 1.0..16.0, default = 1.0, doc = "filter sections in each stage")]
    sections: f32,
    #[param(range = 1.0..16.0, default = 16.0, doc = "how many stages run")]
    stages: f32,
    chain: [dsp::Stage; STAGES],
}

impl Module for ChainMerged {
    fn init(&mut self, _: &Ctx) {
        self.chain = [dsp::Stage::new(); STAGES];
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let sections = dsp::count(self.sections);
        let stages = dsp::count(self.stages);
        let inp = block.audio_in(Self::IN);
        let mut out = block.audio_out(Self::OUT);
        for c in 0..2 {
            let chan = out.chan(c);
            for (o, i) in chan.iter_mut().zip(inp.chan(c)) {
                *o = self.chain[0].tick(c, *i, sections);
            }
            for stage in self.chain[1..stages].iter_mut() {
                for o in chan.iter_mut() {
                    *o = stage.tick(c, *o, sections);
                }
            }
        }
    }
}
