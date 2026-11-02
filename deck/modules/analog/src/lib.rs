//! `analog` – a classic two-oscillator, two-filter, two-amp subtractive synth as one universal module (#331, README.md):
//! Osc 1 / Osc 2 (saw, pulse, triangle, sine; a sub each; Osc 2 hard-syncs to Osc 1) and noise → routing → Filter 1 /
//! Filter 2 (LP, BP, HP, notch; parallel or serial) → Amp 1 / Amp 2 (level, pan) → stereo out, modulated by two LFOs and
//! four ADSRs, eight voices. Notes come in as four cv lanes (pitch Hz left, gate = velocity right) and a sustain control;
//! every control is a module parameter (`params.rs`), four factory presets are data (`data/presets.json`).
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p analog-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/analog_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
pub mod dsp;
mod engine;
mod params;
mod voice;

pub use engine::{LANES, VOICES};
pub use params::{Analog, Patch, PARAMS_JSON, PARAM_NAMES};

use deck_module::abi::MAX_BLOCK;
use deck_module::{Block, Ctx, Module};

/// the factory presets: `{"presets": [{"name", "doc", "values": {parameter: value, …}}, …]}`; a preset lists only what
/// differs from the defaults, so applying one means resetting every parameter to its default and then setting these
pub const PRESETS_JSON: &str = include_str!("../data/presets.json");

/// a JSON constant as a section's bytes
#[cfg(target_arch = "wasm32")]
const fn bytes<const N: usize>(s: &str) -> [u8; N] {
    let b = s.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = b[i];
        i += 1;
    }
    out
}

// the unit / choices / doc of every parameter and the factory presets, readable from the wasm without running it
// (custom sections `deck.params` and `deck.presets`, like `deck.module` for the descriptor)
#[cfg(target_arch = "wasm32")]
mod sections {
    #[link_section = "deck.params"]
    #[used]
    static PARAMS: [u8; super::PARAMS_JSON.len()] = super::bytes::<{ super::PARAMS_JSON.len() }>(super::PARAMS_JSON);
    #[link_section = "deck.presets"]
    #[used]
    static PRESETS: [u8; super::PRESETS_JSON.len()] = super::bytes::<{ super::PRESETS_JSON.len() }>(super::PRESETS_JSON);
}

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
        let n = block.frames();
        let patch = self.patch();
        let sustain = block.control_in(0) >= 0.5;
        let (mut left, mut right) = ([0.0f32; MAX_BLOCK], [0.0f32; MAX_BLOCK]);
        let lanes = [block.audio_in(0), block.audio_in(1), block.audio_in(2), block.audio_in(3)];
        self.engine.render(&patch, &lanes, sustain, &ctx.tempo(), ctx.frame(), (&mut left[..n], &mut right[..n]));
        let (ol, or) = block.audio_out(0);
        ol.copy_from_slice(&left[..n]);
        or.copy_from_slice(&right[..n]);
    }
}
