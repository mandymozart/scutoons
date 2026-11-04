//! The voice pool: eight voices, played by note events (module ABI v2, #343). A note on starts a note on a free voice – or
//! steals the oldest – keyed by the note's **id**, never by its pitch; its note off releases that voice, so a note's tail
//! keeps sounding on its own voice while the next notes start. A note on for an id that still sounds is a legato pitch
//! change of that voice; a bend (a per-note expression, semitones) moves the voice's pitch from the note's own; a reset
//! releases everything. The sustain control holds released notes while it is up. Events take effect at their frame: the
//! render cuts the block there, so a note lands on its sample however the host cuts the blocks.
use crate::dsp::math::exp2;
use crate::params::Patch;
use crate::voice::{Note, Voice};
use deck_module::{Ev, Expr, Kind, Tempo};

pub const VOICES: usize = 8;

#[derive(Default)]
pub struct Engine {
    voices: [Voice; VOICES],
    sustain: bool,
    next_age: u64,
    seed: u64,
    rate: f64,
}

impl Engine {
    pub fn init(&mut self, rate: f64, seed: u64) {
        *self = Engine { rate: rate.max(1.0), seed, ..Engine::default() };
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active()).count()
    }

    /// the voice the note `id` holds
    fn held(&self, id: u32) -> Option<usize> {
        self.voices.iter().position(|v| v.active() && v.id() == Some(id))
    }

    fn note_on(&mut self, id: u32, hz: f32, vel: f32, p: &Patch) {
        if let Some(i) = self.held(id) {
            self.voices[i].set_pitch(hz);
            return;
        }
        let age = self.next_age;
        self.next_age += 1;
        let note = Note { hz, vel, id, age, seed: self.seed ^ age.wrapping_mul(0x9E37_79B9_7F4A_7C15) };
        match self.voices.iter().position(|v| !v.active()) {
            Some(i) => self.voices[i].start(note, p, self.rate),
            None => {
                let oldest = (0..VOICES).min_by_key(|&i| self.voices[i].age()).unwrap_or(0);
                self.voices[oldest].steal(note);
            }
        }
    }

    fn note_off(&mut self, id: u32, p: &Patch) {
        if let Some(i) = self.held(id) {
            if self.sustain {
                self.voices[i].hold();
            } else {
                self.voices[i].release(p, self.rate);
            }
        }
    }

    fn event(&mut self, e: &Ev, p: &Patch) {
        match e.kind() {
            Some(Kind::NoteOn) => {
                // the host fills Hz from a key and a missing velocity with 1; a note without a pitch is no note
                let (hz, vel) = (e.pitch_hz(), e.velocity());
                if hz.is_finite() && hz > 0.0 {
                    self.note_on(e.id, hz, if vel.is_finite() { vel.min(1.0) } else { 1.0 }, p);
                }
            }
            Some(Kind::NoteOff) => self.note_off(e.id, p),
            Some(Kind::NoteExpr) => {
                if let (Some((Expr::Bend, semis)), Some(i)) = (e.expr(), self.held(e.id)) {
                    if semis.is_finite() {
                        let base = self.voices[i].base();
                        self.voices[i].set_pitch((f64::from(base) * exp2(f64::from(semis) / 12.0)) as f32);
                    }
                }
            }
            Some(Kind::Reset) => {
                for i in 0..VOICES {
                    if self.voices[i].active() {
                        self.voices[i].release(p, self.rate);
                    }
                }
            }
            _ => {}
        }
    }

    /// Render `out.0.len()` frames starting at audio frame `frame0` of the tempo map. `notes` are this call's note
    /// events (sorted by frame), `sustain` the pedal for this call.
    pub fn render(&mut self, p: &Patch, notes: &[Ev], sustain: bool, tempo: &Tempo, frame0: f64, out: (&mut [f32], &mut [f32])) {
        let (left, right) = out;
        if self.sustain && !sustain {
            for i in 0..VOICES {
                if self.voices[i].active() && self.voices[i].sustained() {
                    self.voices[i].release(p, self.rate);
                }
            }
        }
        self.sustain = sustain;
        let mut next = 0;
        for i in 0..left.len().min(right.len()) {
            while let Some(e) = notes.get(next).filter(|e| e.frame as usize <= i) {
                self.event(e, p);
                next += 1;
            }
            let beat = tempo.beat_at(frame0 + i as f64);
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in self.voices.iter_mut().filter(|v| v.active()) {
                let (a, b) = v.render(p, self.rate, beat);
                l += a;
                r += b;
            }
            left[i] = l * p.volume;
            right[i] = r * p.volume;
        }
        // an event past the call's frames (a host never sends one) still counts, at its end
        for e in &notes[next.min(notes.len())..] {
            self.event(e, p);
        }
    }
}
