//! `stepseq` – a 16-step sequencer as a module (ABI v2): every step is one slider, its level is the pitch. Step `k` plays
//! on the tempo clock (`rate` steps a beat), the pattern wraps after `steps` steps. The pitch of a level is
//! `base × 2^(octaves × level)`; with `snap` on (the default) it lands on the nearest semitone, off it is continuous. Every
//! step is a `music.note` on the `notes` event output (a note on with its pitch in Hz and its length in beats, `gate` of a
//! step) – an instrument with a note input, `analog`, plays it; the event host fills the MIDI key from the deck's tuning.
//! No randomness: the same params play the same notes on every deck, however the timeline is cut into blocks.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p stepseq-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/stepseq_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::{math, Block, Ctx, Ev, Module};

/// The default phrase: a minor-pentatonic figure over an octave (semitones 0 5 7 5 10 7 5 7 / 0 5 7 12 10 7 5 7 of
/// `octaves` = 2, i.e. level = semitones / 24), with a rest on steps 4, 8 and 16 (`on4`, `on8`, `on16` off): 0 5 7 – 10 7 5 –
/// 0 5 7 12 10 7 5 –. The defaults of `step1` … `step16` and `on1` … `on16` below are these. A step is on when its `on` is 1.
#[derive(Module)]
#[module(name = "stepseq", doc = "a 16-step sequencer on the tempo clock: each step a slider, its level the pitch")]
#[port(output, event, name = "notes", names = "music.note", props = "pitch|vel|glide", capacity = 64, doc = "every step as a note")]
pub struct StepSeq {
    #[param(range = 0.0..1.0, default = 0.0, doc = "step 1: its level is its pitch (0 = the base, 1 = `octaves` above)")]
    step1: f32,
    #[param(range = 0.0..1.0, default = 0.208, doc = "step 2: its level is its pitch")]
    step2: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 3: its level is its pitch")]
    step3: f32,
    #[param(range = 0.0..1.0, default = 0.208, doc = "step 4: its level is its pitch")]
    step4: f32,
    #[param(range = 0.0..1.0, default = 0.417, doc = "step 5: its level is its pitch")]
    step5: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 6: its level is its pitch")]
    step6: f32,
    #[param(range = 0.0..1.0, default = 0.208, doc = "step 7: its level is its pitch")]
    step7: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 8: its level is its pitch")]
    step8: f32,
    #[param(range = 0.0..1.0, default = 0.0, doc = "step 9: its level is its pitch")]
    step9: f32,
    #[param(range = 0.0..1.0, default = 0.208, doc = "step 10: its level is its pitch")]
    step10: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 11: its level is its pitch")]
    step11: f32,
    #[param(range = 0.0..1.0, default = 0.5, doc = "step 12: its level is its pitch")]
    step12: f32,
    #[param(range = 0.0..1.0, default = 0.417, doc = "step 13: its level is its pitch")]
    step13: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 14: its level is its pitch")]
    step14: f32,
    #[param(range = 0.0..1.0, default = 0.208, doc = "step 15: its level is its pitch")]
    step15: f32,
    #[param(range = 0.0..1.0, default = 0.292, doc = "step 16: its level is its pitch")]
    step16: f32,
    #[param(choices = "off|on", default = 1, doc = "step 1 plays (on) or rests (off)")]
    on1: f32,
    #[param(choices = "off|on", default = 1, doc = "step 2 plays (on) or rests (off)")]
    on2: f32,
    #[param(choices = "off|on", default = 1, doc = "step 3 plays (on) or rests (off)")]
    on3: f32,
    #[param(choices = "off|on", default = 0, doc = "step 4 plays (on) or rests (off)")]
    on4: f32,
    #[param(choices = "off|on", default = 1, doc = "step 5 plays (on) or rests (off)")]
    on5: f32,
    #[param(choices = "off|on", default = 1, doc = "step 6 plays (on) or rests (off)")]
    on6: f32,
    #[param(choices = "off|on", default = 1, doc = "step 7 plays (on) or rests (off)")]
    on7: f32,
    #[param(choices = "off|on", default = 0, doc = "step 8 plays (on) or rests (off)")]
    on8: f32,
    #[param(choices = "off|on", default = 1, doc = "step 9 plays (on) or rests (off)")]
    on9: f32,
    #[param(choices = "off|on", default = 1, doc = "step 10 plays (on) or rests (off)")]
    on10: f32,
    #[param(choices = "off|on", default = 1, doc = "step 11 plays (on) or rests (off)")]
    on11: f32,
    #[param(choices = "off|on", default = 1, doc = "step 12 plays (on) or rests (off)")]
    on12: f32,
    #[param(choices = "off|on", default = 1, doc = "step 13 plays (on) or rests (off)")]
    on13: f32,
    #[param(choices = "off|on", default = 1, doc = "step 14 plays (on) or rests (off)")]
    on14: f32,
    #[param(choices = "off|on", default = 1, doc = "step 15 plays (on) or rests (off)")]
    on15: f32,
    #[param(choices = "off|on", default = 0, doc = "step 16 plays (on) or rests (off)")]
    on16: f32,
    #[param(range = 1.0..16.0, default = 16.0, unit = "steps", doc = "the pattern's length: it wraps after this many steps")]
    steps: f32,
    #[param(range = 0.0..16.0, default = 2.0, unit = "steps/beat", doc = "steps a beat")]
    rate: f32,
    #[param(range = 0.0..1.0, default = 0.7, doc = "of a step: how long the note holds")]
    gate: f32,
    #[param(range = 20.0..2000.0, default = 110.0, unit = "Hz", doc = "the pitch of level 0")]
    base: f32,
    #[param(range = 0.0..4.0, default = 2.0, unit = "oct", doc = "how far level 1 reaches above the base")]
    octaves: f32,
    #[param(choices = "off|on", default = 1, doc = "on: the pitch snaps to the nearest semitone; off: continuous")]
    snap: f32,
}

/// the steps on a grid of `per_beat` steps a beat whose start falls in [from, to): (step, its beat)
fn steps(from: f64, to: f64, per_beat: f64) -> impl Iterator<Item = (i64, f64)> {
    let ok = per_beat > 0.0 && to > from;
    let first = if ok { (from * per_beat).ceil() as i64 } else { 0 };
    let end = if ok { (to * per_beat).ceil() as i64 } else { 0 };
    (first..end).map(move |k| (k, k as f64 / per_beat))
}

impl StepSeq {
    fn levels(&self) -> [f32; 16] {
        [
            self.step1, self.step2, self.step3, self.step4, self.step5, self.step6, self.step7, self.step8, self.step9, self.step10,
            self.step11, self.step12, self.step13, self.step14, self.step15, self.step16,
        ]
    }

    /// whether the pattern's step `at` (0-based) plays; off, it is a rest: no note (what sounds of the step before ends at
    /// its own gate)
    pub fn plays(&self, at: usize) -> bool {
        let on = [
            self.on1, self.on2, self.on3, self.on4, self.on5, self.on6, self.on7, self.on8, self.on9, self.on10, self.on11,
            self.on12, self.on13, self.on14, self.on15, self.on16,
        ];
        on[at.min(15)] >= 0.5
    }

    /// the pitch (Hz) of the pattern's step `at` (0-based)
    pub fn pitch_of(&self, at: usize) -> f32 {
        let span = self.octaves * self.levels()[at.min(15)].clamp(0.0, 1.0);
        let octaves = if self.snap >= 0.5 { math::round(span * 12.0) / 12.0 } else { span };
        self.base * math::pow(2.0, octaves)
    }

    /// the pattern's length in steps (1 … 16)
    pub fn length(&self) -> i64 {
        (math::round(self.steps) as i64).clamp(1, 16)
    }
}

impl Module for StepSeq {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let tempo = ctx.tempo();
        let (from, to) = ctx.beats();
        let per_beat = f64::from(self.rate);
        let dur = f64::from(self.gate) / per_beat.max(f64::MIN_POSITIVE);
        let len = self.length();
        for (k, beat) in steps(from, to, per_beat) {
            // the step's frame in this block, rounded as the built-in events are (deck-events `resolve`)
            if !self.plays(k.rem_euclid(len) as usize) {
                continue;
            }
            let at = tempo.frame_of(beat).unwrap_or(ctx.frame());
            let frame = (at - ctx.frame()).max(0.0).round() as u32;
            let pitch = self.pitch_of(k.rem_euclid(len) as usize);
            // the step is the note's id; the key is the host's to fill (a Hz source)
            block.emit(Self::NOTES, Ev::note_on(frame, k as u32, pitch, f32::NAN, 1.0).lasting(dur as f32));
        }
    }
}
