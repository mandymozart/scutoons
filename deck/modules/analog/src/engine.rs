//! The voice pool: eight voices, played by note events (module ABI v2, #343). A note on starts a note on a free voice – or
//! steals the oldest – keyed by the note's **id**, never by its pitch; its note off releases that voice, so a note's tail
//! keeps sounding on its own voice while the next notes start. A note on for an id that still sounds is a legato pitch
//! change of that voice; a bend (a per-note expression, semitones) moves the voice's pitch from the note's own; a reset
//! releases everything. The sustain control holds released notes while it is up. Events take effect at their frame: the
//! render cuts the block there, so a note lands on its sample however the host cuts the blocks.
//!
//! `mono` and `legato` play voice 0 alone from a stack of the held notes (the newest sounds; its note off goes back to the
//! one under it); mono restarts the envelopes on every note, legato only when no other note is held. `glide` slides every
//! new note from the last note's pitch. Two control outputs, once a block: amp envelope 1 of the newest sounding voice and
//! the output's level.
use deck_module::dsp::arith::exp2;
use deck_module::dsp::follower::Follower;
use crate::params::Patch;
use crate::voice::{Note, Voice};
use deck_module::{Ev, Expr, Kind, Tempo};

pub const VOICES: usize = 8;
/// the held notes mono and legato remember (more: the oldest is forgotten)
pub const HELD: usize = 16;
/// the level output's follower: up, down (s)
const LEVEL_TIMES: (f32, f32) = (0.005, 0.15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Poly,
    Mono,
    Legato,
}

impl Mode {
    fn of(p: &Patch) -> Mode {
        match p.mono.round() as i32 {
            1 => Mode::Mono,
            2 => Mode::Legato,
            _ => Mode::Poly,
        }
    }
}

#[derive(Default)]
pub struct Engine {
    voices: [Voice; VOICES],
    sustain: bool,
    next_age: u64,
    seed: u64,
    rate: f64,
    /// mono / legato: the held notes (id, Hz, velocity), the newest last
    held: [(u32, f32, f32); HELD],
    held_n: usize,
    /// the pitch of the last note on (where a glide starts; 0 = none yet)
    last_hz: f32,
    follower: Follower,
    outputs: (f32, f32),
}

impl Engine {
    pub fn init(&mut self, rate: f64, seed: u64) {
        let rate = rate.max(1.0);
        let follower = Follower::new(LEVEL_TIMES.0, LEVEL_TIMES.1, rate as f32);
        *self = Engine { rate, seed, follower, ..Engine::default() };
    }

    /// the control outputs after the last render: (amp envelope 1 of the newest sounding voice, the output's level)
    pub fn outputs(&self) -> (f32, f32) {
        self.outputs
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active()).count()
    }

    /// the voice the note `id` holds
    fn held(&self, id: u32) -> Option<usize> {
        self.voices.iter().position(|v| v.active() && v.id() == Some(id))
    }

    fn new_note(&mut self, id: u32, hz: f32, vel: f32, p: &Patch) -> Note {
        let age = self.next_age;
        self.next_age += 1;
        let from = if p.glide > 0.0 { self.last_hz } else { 0.0 };
        self.last_hz = hz;
        Note { hz, vel, id, age, seed: self.seed ^ age.wrapping_mul(0x9E37_79B9_7F4A_7C15), from }
    }

    fn note_on(&mut self, id: u32, hz: f32, vel: f32, p: &Patch) {
        let mode = Mode::of(p);
        if mode != Mode::Poly {
            return self.mono_on(id, hz, vel, mode, p);
        }
        self.held_n = 0;
        if let Some(i) = self.held(id) {
            self.voices[i].set_pitch(hz);
            return;
        }
        let note = self.new_note(id, hz, vel, p);
        match self.voices.iter().position(|v| !v.active()) {
            Some(i) => self.voices[i].start(note, p, self.rate),
            None => {
                let oldest = (0..VOICES).min_by_key(|&i| self.voices[i].age()).unwrap_or(0);
                self.voices[oldest].steal(note);
            }
        }
    }

    /// mono / legato: voice 0 takes the note
    fn mono_on(&mut self, id: u32, hz: f32, vel: f32, mode: Mode, p: &Patch) {
        if let Some(k) = self.held[..self.held_n].iter().position(|h| h.0 == id) {
            self.held[k].1 = hz;
            if self.voices[0].id() == Some(id) {
                self.voices[0].legato(id, hz, p, self.rate);
            }
            return;
        }
        let overlap = self.held_n > 0 && self.voices[0].active() && self.voices[0].id().is_some();
        if self.held_n == HELD {
            self.held.copy_within(1.., 0);
            self.held_n -= 1;
        }
        self.held[self.held_n] = (id, hz, vel);
        self.held_n += 1;
        if mode == Mode::Legato && overlap {
            self.last_hz = hz;
            self.voices[0].legato(id, hz, p, self.rate);
        } else {
            let note = self.new_note(id, hz, vel, p);
            self.voices[0].retrigger(note, p, self.rate);
        }
    }

    /// mono / legato: a held note lets go; the sounding one hands voice 0 to the newest note still held
    fn mono_off(&mut self, id: u32, mode: Mode, p: &Patch) -> bool {
        let Some(k) = self.held[..self.held_n].iter().position(|h| h.0 == id) else {
            return false;
        };
        self.held.copy_within(k + 1..self.held_n, k);
        self.held_n -= 1;
        if self.voices[0].id() != Some(id) {
            return true;
        }
        match self.held_n.checked_sub(1).map(|top| self.held[top]) {
            Some((next, hz, _)) if mode == Mode::Legato => {
                self.last_hz = hz;
                self.voices[0].legato(next, hz, p, self.rate);
            }
            Some((next, hz, vel)) => {
                let note = self.new_note(next, hz, vel, p);
                self.voices[0].retrigger(note, p, self.rate);
            }
            None if self.sustain => self.voices[0].hold(),
            None => self.voices[0].release(p, self.rate),
        }
        true
    }

    fn note_off(&mut self, id: u32, p: &Patch) {
        let mode = Mode::of(p);
        if mode != Mode::Poly && self.mono_off(id, mode, p) {
            return;
        }
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
                self.held_n = 0;
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
            self.follower.tick(left[i].abs().max(right[i].abs()));
        }
        // an event past the call's frames (a host never sends one) still counts, at its end
        for e in &notes[next.min(notes.len())..] {
            self.event(e, p);
        }
        let newest = self.voices.iter().filter(|v| v.active()).max_by_key(|v| v.age());
        self.outputs = (newest.map_or(0.0, |v| v.env()), self.follower.level().min(1.0));
    }
}
