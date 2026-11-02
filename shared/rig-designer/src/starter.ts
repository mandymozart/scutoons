// What a new module starts as: the walk module (deck/modules/walk/src/lib.rs, #280) – the SDK's worked example, a seeded random
// walk on a pitch scale. test/starter.test.ts keeps this copy equal to the file in the repository.
import type { ModuleFiles } from './types.ts';

export const STARTER_FILES: ModuleFiles = {
	'lib.rs': `//! \`walk\` as a module (#280) – the stage's built-in \`walk\` node (deck-events' \`random_walk\` into seq's cv) written
//! against the SDK, the example of one: a seeded random walk on a continuous pitch scale, on the tempo clock.
//! \`spread\` 0 = the base held, 1 = ±\`octaves\`; \`rate\` = steps a beat; \`glide\` rides along for the voice. The same seed
//! plays the same melody on every deck, however the timeline is cut into blocks. Its output \`cv\` is pitch (Hz) on the
//! left and the gate on the right – an fm / osc voice with its \`cv\` linked plays it – and every step is also emitted as
//! a \`music.note\` event.
//!
//! Build: \`cargo build --release --target wasm32-unknown-unknown -p walk-module\` (the .wasm in
//! \`target/wasm32-unknown-unknown/release/walk_module.wasm\`).
// one \`#[port(…)]\` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::cv::GateCv;
use deck_module::{Block, Ctx, Event, Module};

#[derive(Module)]
#[module(name = "walk")]
#[port(output, audio, name = "cv")]
#[port(output, event, name = "notes")]
pub struct Walk {
    /// 0 = the base held, 1 = ±\`octaves\`
    #[param(range = 0.0..1.0, default = 0.0)]
    spread: f32,
    /// steps a beat
    #[param(range = 0.0..16.0, default = 2.0)]
    rate: f32,
    /// of a note's length: how long the voice glides into it
    #[param(range = 0.0..1.0, default = 0.5)]
    glide: f32,
    /// Hz
    #[param(range = 20.0..2000.0, default = 147.0)]
    base: f32,
    #[param(range = 0.0..4.0, default = 2.0)]
    octaves: f32,
    /// of a step: how long the note holds
    #[param(range = 0.0..1.0, default = 1.0)]
    length: f32,
    cv: Option<GateCv>,
}

/// a uniform number in [0, 1) for (seed, step) – splitmix64, so step k's value never depends on the steps before
fn unit(seed: u64, step: i64) -> f64 {
    let mut z = seed ^ (step as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// the steps on a grid of \`per_beat\` steps a beat whose start falls in [from, to): (step, its beat)
fn steps(from: f64, to: f64, per_beat: f64) -> impl Iterator<Item = (i64, f64)> {
    let ok = per_beat > 0.0 && to > from;
    let first = if ok { (from * per_beat).ceil() as i64 } else { 0 };
    let end = if ok { (to * per_beat).ceil() as i64 } else { 0 };
    (first..end).map(move |k| (k, k as f64 / per_beat))
}

impl Walk {
    /// step k's pitch
    pub fn pitch_at(&self, seed: u64, k: i64) -> f32 {
        let r = unit(seed, k) as f32 * 2.0 - 1.0;
        self.base * 2f32.powf(r * self.spread * self.octaves)
    }
}

impl Module for Walk {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let (from, to) = ctx.beats();
        let per_beat = f64::from(self.rate);
        let dur = f64::from(self.length) / per_beat.max(f64::MIN_POSITIVE);
        for (k, beat) in steps(from, to, per_beat) {
            let pitch = self.pitch_at(ctx.seed(), k);
            block.emit(Event::at_beat("music.note", beat).lasting(dur).with("pitch", pitch).with("glide", self.glide));
        }
        let base = self.base;
        let cv = self.cv.get_or_insert_with(|| GateCv::new(base));
        let (l, r, notes) = block.audio_out_with_events(0);
        cv.render(ctx, notes, "music.note", l, r);
    }
}
`
};
