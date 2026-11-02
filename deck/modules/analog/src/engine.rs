//! The voice pool: eight voices, four note lanes. A lane is one cv pair as the stage's `seq` and the `walk` module write
//! it (pitch in Hz left, gate right); here the gate's value is the velocity (the 1.0 of `seq` is full velocity). A
//! rising gate starts a note on a free voice – or steals the oldest – and a falling one releases it, so a note's tail keeps
//! sounding on its own voice while the lane plays the next note. While a gate stays up, a changing pitch glides the
//! lane's voice (legato). The sustain control holds released notes while it is up.
use crate::params::Patch;
use crate::voice::{Note, Voice};
use deck_module::Tempo;

pub const VOICES: usize = 8;
pub const LANES: usize = 4;

#[derive(Default)]
pub struct Engine {
    voices: [Voice; VOICES],
    lane_high: [bool; LANES],
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

    fn note_on(&mut self, lane: usize, hz: f32, vel: f32, p: &Patch) {
        let age = self.next_age;
        self.next_age += 1;
        let note = Note { hz, vel, lane: lane as u8, age, seed: self.seed ^ age.wrapping_mul(0x9E37_79B9_7F4A_7C15) };
        match self.voices.iter().position(|v| !v.active()) {
            Some(i) => self.voices[i].start(note, p, self.rate),
            None => {
                let oldest = (0..VOICES).min_by_key(|&i| self.voices[i].age()).unwrap_or(0);
                self.voices[oldest].steal(note);
            }
        }
    }

    fn lane_in(&mut self, lane: usize, hz: f32, gate: f32, p: &Patch) {
        let on = gate > 0.0;
        let held = self.voices.iter().position(|v| v.active() && v.lane() == Some(lane as u8));
        match (self.lane_high[lane], on) {
            (false, true) => self.note_on(lane, hz, gate.min(1.0), p),
            (true, true) => {
                if let Some(i) = held {
                    self.voices[i].set_pitch(hz);
                }
            }
            (true, false) => {
                if let Some(i) = held {
                    if self.sustain {
                        self.voices[i].hold();
                    } else {
                        self.voices[i].release(p, self.rate);
                    }
                }
            }
            (false, false) => {}
        }
        self.lane_high[lane] = on;
    }

    /// Render `out.0.len()` frames starting at audio frame `frame0` of the tempo map. `lanes` are the four cv pairs
    /// (pitch, gate), `sustain` the pedal for this call.
    pub fn render(&mut self, p: &Patch, lanes: &[(&[f32], &[f32]); LANES], sustain: bool, tempo: &Tempo, frame0: f64, out: (&mut [f32], &mut [f32])) {
        let (left, right) = out;
        if self.sustain && !sustain {
            for i in 0..VOICES {
                if self.voices[i].active() && self.voices[i].sustained() {
                    self.voices[i].release(p, self.rate);
                }
            }
        }
        self.sustain = sustain;
        for i in 0..left.len().min(right.len()) {
            for (k, (hz, gate)) in lanes.iter().enumerate() {
                self.lane_in(k, hz[i], gate[i], p);
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
    }
}
