//! `analog` – a classic two-oscillator, two-filter, two-amp subtractive synth as one universal module (#331, README.md):
//! Osc 1 / Osc 2 (saw, pulse, triangle, sine; a sub each; Osc 2 hard-syncs to Osc 1) and noise → routing → Filter 1 /
//! Filter 2 (LP, BP, HP, notch; parallel or serial) → Amp 1 / Amp 2 (level, pan) → stereo out, modulated by two LFOs and
//! four ADSRs, eight voices. Notes come in on one event input (module ABI v2, #343: note on / off paired by id, with
//! velocity; a bend per note; a reset) and a sustain control; every control is a module parameter (`params.rs`, units and
//! docs in the descriptor); the factory preset `pad` is the descriptor's, the other presets are documents
//! (`analog.preset.json` holds three to import).
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p analog-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/analog_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
pub mod dsp;
mod engine;
mod params;
mod voice;

pub use engine::VOICES;
pub use params::{Analog, Patch, PARAM_NAMES};

use deck_module::{Block, Ctx, Module};

impl Analog {
    /// how many voices sound (a tail counts)
    pub fn active_voices(&self) -> usize {
        self.engine.active_voices()
    }
}

impl Module for Analog {
    fn init(&mut self, ctx: &Ctx) {
        self.engine.init(ctx.rate(), ctx.seed());
    }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let patch = self.patch();
        let sustain = block.control_in(Self::SUSTAIN) >= 0.5;
        let notes = block.events_in(Self::NOTES);
        let mut out = block.audio_out(Self::OUT);
        let (left, right) = out.stereo();
        self.engine.render(&patch, notes, sustain, &ctx.tempo(), ctx.frame(), (left, right));
    }
}
