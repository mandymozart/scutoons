//! `transpose` – notes in, the same notes out, moved in pitch: a note on leaves with its Hz and its key shifted by
//! `semi` semitones and `fine` cents, at its frame and under its id; note offs, bends and a reset pass as they are, so
//! a note ends on the voice that started it whatever the knobs did in between. No state.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p transpose-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/transpose_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::exp2;
use deck_module::{Block, Ctx, Kind, Module};

#[derive(Module)]
#[module(name = "transpose", doc = "notes moved in pitch by semitones and cents")]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "the notes to move")]
#[port(output, event, name = "out", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "the same notes, their pitch moved")]
#[preset(name = "octave up", doc = "Every note an octave higher.", values(semi = 12.0))]
pub struct Transpose {
    #[param(range = -48.0..48.0, default = 0.0, unit = "st", doc = "how far every note moves, in semitones")]
    pub semi: f32,
    #[param(range = -100.0..100.0, default = 0.0, unit = "cent", doc = "a fine shift on top, in cents")]
    pub fine: f32,
}

impl Transpose {
    /// the shift in semitones
    pub fn semitones(&self) -> f64 {
        f64::from(self.semi) + f64::from(self.fine) / 100.0
    }
}

impl Module for Transpose {
    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let st = self.semitones();
        let ratio = exp2(st / 12.0);
        for e in block.events_in(Self::NOTES) {
            let mut moved = *e;
            if e.kind() == Some(Kind::NoteOn) {
                // an absent Hz or key stays absent (NaN): the next event host fills it from the other
                moved.v[0] = (f64::from(e.pitch_hz()) * ratio) as f32;
                moved.v[3] = (f64::from(e.key()) + st) as f32;
            }
            block.emit(Self::OUT, moved);
        }
    }
}
