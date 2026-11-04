// What a new module starts as: the walk module (deck/modules/walk/src/lib.rs, #280) – the SDK's worked example, a seeded random
// walk on a pitch scale. test/starter.test.ts keeps this copy equal to the file in the repository.
import type { ModuleFiles } from './types.ts';

export const STARTER_FILES: ModuleFiles = {
	'lib.rs': `//! \`walk\` as a module (#280; module ABI v2 #343) – the stage's built-in \`walk\` node (deck-events' \`random_walk\` into
//! seq's cv) written against the SDK, the example of one: a seeded random walk on a continuous pitch scale, on the tempo
//! clock. \`spread\` 0 = the base held, 1 = ±\`octaves\`; \`rate\` = steps a beat; \`glide\` rides along for the voice. The same
//! seed plays the same melody on every deck, however the timeline is cut into blocks. Every step is a \`music.note\` on
//! the \`notes\` event output (a note on with its pitch in Hz, its length in beats and its glide – an instrument with a
//! note input, \`analog\`, plays it); the same notes are rendered into the \`cv\` output, pitch (Hz) on the left and the gate
//! on the right – an fm / osc voice with its \`cv\` linked plays that.
//!
//! Build: \`cargo build --release --target wasm32-unknown-unknown -p walk-module\` (the .wasm in
//! \`target/wasm32-unknown-unknown/release/walk_module.wasm\`).
// one \`#[port(…)]\` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::cv::GateCv;
use deck_module::{math, Block, Ctx, Ev, Module};

#[derive(Module)]
#[module(name = "walk", doc = "a seeded random walk on a continuous pitch scale, on the tempo clock")]
#[port(output, cv, name = "cv", doc = "pitch in Hz left, the gate right")]
#[port(output, event, name = "notes", names = "music.note", props = "pitch|vel|glide", capacity = 64, doc = "every step as a note")]
pub struct Walk {
    #[param(range = 0.0..1.0, default = 0.0, doc = "0 = the base held, 1 = ±octaves")]
    spread: f32,
    #[param(range = 0.0..16.0, default = 2.0, unit = "steps/beat", doc = "steps a beat")]
    rate: f32,
    #[param(range = 0.0..1.0, default = 0.5, doc = "of a note's length: how long the voice glides into it")]
    glide: f32,
    #[param(range = 20.0..2000.0, default = 147.0, unit = "Hz", doc = "the pitch the walk centres on")]
    base: f32,
    #[param(range = 0.0..4.0, default = 2.0, unit = "oct", doc = "how far a full spread reaches either way")]
    octaves: f32,
    #[param(range = 0.0..1.0, default = 1.0, doc = "of a step: how long the note holds")]
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
    /// step k's pitch (libm: the same bits native and wasm, as the built-in's)
    pub fn pitch_at(&self, seed: u64, k: i64) -> f32 {
        let r = unit(seed, k) as f32 * 2.0 - 1.0;
        self.base * math::pow(2.0, r * self.spread * self.octaves)
    }
}

impl Module for Walk {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let tempo = ctx.tempo();
        let (from, to) = ctx.beats();
        let per_beat = f64::from(self.rate);
        let dur = f64::from(self.length) / per_beat.max(f64::MIN_POSITIVE);
        for (k, beat) in steps(from, to, per_beat) {
            // the step's frame in this block, rounded as the built-in's events are (deck-events \`resolve\`)
            let at = tempo.frame_of(beat).unwrap_or(ctx.frame());
            let frame = (at - ctx.frame()).max(0.0).round() as u32;
            let pitch = self.pitch_at(ctx.seed(), k);
            // the step is the note's id; the key is the host's to fill (a Hz source)
            let note = Ev::note_on(frame, k as u32, pitch, f32::NAN, 1.0).lasting(dur as f32).gliding(self.glide);
            block.emit(Self::NOTES, note);
        }
        let base = self.base;
        let cv = self.cv.get_or_insert_with(|| GateCv::new(base));
        let (mut out, notes) = block.audio_out_with_events(Self::CV, Self::NOTES);
        let (l, r) = out.stereo();
        cv.render_notes(&tempo, notes, l, r);
    }
}
`
};
