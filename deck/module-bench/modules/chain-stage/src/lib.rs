//! `chain_stage` – one stage of the wiring bench (#393, agents/research/NATIVE-EDITOR-GUI.md §8): audio in, `sections`
//! filter sections, audio out. The bench wires K of these through the host, as the engine wires module nodes.
#![allow(clippy::duplicated_attributes)]
use deck_module::{Block, Ctx, Module};

mod dsp;

#[derive(Module)]
#[module(
    name = "chain_stage",
    doc = "one filter stage of the wiring bench (#393)"
)]
#[port(input, audio, name = "in", channels = 2)]
#[port(output, audio, name = "out", channels = 2)]
pub struct ChainStage {
    #[param(range = 1.0..16.0, default = 1.0, doc = "filter sections in this stage")]
    sections: f32,
    stage: dsp::Stage,
}

impl Module for ChainStage {
    fn init(&mut self, _: &Ctx) {
        self.stage = dsp::Stage::new();
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let sections = dsp::count(self.sections);
        let inp = block.audio_in(Self::IN);
        let mut out = block.audio_out(Self::OUT);
        for c in 0..2 {
            for (o, i) in out.chan(c).iter_mut().zip(inp.chan(c)) {
                *o = self.stage.tick(c, *i, sections);
            }
        }
    }
}
