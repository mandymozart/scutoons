//! Notes → the cv the stage's `seq` node writes (pitch in Hz on the left channel, the gate 0 / 1 on the right), so an
//! fm / osc voice with its `cv` linked plays a module's notes like any built-in melody. The same arithmetic as
//! deck-events' `GateCv` and `Glide` (sample-accurate, exponential glide over `glide` × the note's length); no
//! allocation. A module with a `cv` audio output calls `GateCv::render_notes` from `process` with the notes it emitted
//! (`Block::audio_out_with_events`).
use crate::ctx::Tempo;
use crate::ev::{Ev, Kind};
use crate::math;

/// notes one call resolves at most (a port's capacity beyond this is not rendered into the cv)
const MAX_NOTES: usize = 64;

/// A pitch glide: `from → to` over a number of frames, exponentially (equal ratio per frame). 0 frames = a jump.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glide {
    from: f32,
    to: f32,
    frames: u32,
}

impl Glide {
    pub fn hold(pitch: f32) -> Glide {
        Glide { from: pitch, to: pitch, frames: 0 }
    }

    pub fn start(&mut self, from: f32, to: f32, frames: u32) {
        *self = Glide { from, to, frames };
    }

    /// the pitch `offset` frames after the start (past the end: `to`)
    pub fn at(&self, offset: u32) -> f32 {
        if offset >= self.frames || self.from <= 0.0 || self.to <= 0.0 {
            return self.to;
        }
        // libm (`math`): the same bits native and wasm, as deck-events' glide
        self.from * math::pow(self.to / self.from, offset as f32 / self.frames as f32)
    }
}

/// One note resolved to this call's frames.
#[derive(Clone, Copy)]
struct Resolved {
    frame: u32,
    /// frames the gate stays open; `HELD` = until a note off with the same id
    len: u32,
    pitch: Option<f32>,
    glide: f32,
    /// a note off: closes the gate when `id` is the sounding note's
    off: bool,
    id: u32,
}

const HELD: u32 = u32::MAX;
const NONE: Resolved = Resolved { frame: 0, len: 0, pitch: None, glide: 0.0, off: false, id: 0 };

#[derive(Debug, Clone)]
pub struct GateCv {
    /// the pitch now (Hz)
    pub pitch: f32,
    glide: Glide,
    elapsed: u32,
    gate_left: u32,
    /// the sounding note's id
    id: u32,
}

impl GateCv {
    /// holding `pitch` until the first note
    pub fn new(pitch: f32) -> GateCv {
        GateCv { pitch, glide: Glide::hold(pitch), elapsed: 0, gate_left: 0, id: 0 }
    }

    /// The note events of this call (`Ev`, frames already in the block) into `left` / `right`. A note on
    /// with a length (beats) opens the gate that long, gliding over `glide` × its length; one without (0) holds the gate
    /// until the note off with its id (no glide: its length is unknown); a reset closes the gate.
    pub fn render_notes(&mut self, tempo: &Tempo, events: &[Ev], left: &mut [f32], right: &mut [f32]) {
        let per_frame = tempo.per_frame();
        let mut notes = [NONE; MAX_NOTES];
        let mut count = 0;
        for e in events.iter() {
            if count == MAX_NOTES {
                break;
            }
            let r = match e.kind() {
                Some(Kind::NoteOn) => {
                    let held = !(e.len > 0.0 && per_frame > 0.0);
                    Resolved {
                        frame: e.frame,
                        len: if held { HELD } else { (e.len as f64 / per_frame).round() as u32 },
                        pitch: Some(e.pitch_hz()).filter(|p| p.is_finite()),
                        glide: if held { 0.0 } else { e.glide().clamp(0.0, 1.0) },
                        off: false,
                        id: e.id,
                    }
                }
                Some(Kind::NoteOff) => Resolved { frame: e.frame, off: true, id: e.id, ..NONE },
                Some(Kind::Reset) => Resolved { frame: e.frame, off: true, id: self.id, ..NONE },
                _ => continue,
            };
            notes[count] = r;
            count += 1;
        }
        self.play(&mut notes[..count], left, right);
    }

    /// the one renderer: `notes` by frame (stable), then frame by frame
    fn play(&mut self, notes: &mut [Resolved], left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let count = notes.len();
        // by frame, stable (an insertion sort: a handful of notes, nothing to allocate)
        for i in 1..count {
            let mut j = i;
            while j > 0 && notes[j - 1].frame > notes[j].frame {
                notes.swap(j - 1, j);
                j -= 1;
            }
        }
        let mut next = 0;
        for i in 0..n {
            while next < count && notes[next].frame as usize <= i {
                let e = notes[next];
                next += 1;
                if e.off {
                    if e.id == self.id {
                        self.gate_left = 0;
                    }
                    continue;
                }
                if let Some(p) = e.pitch {
                    let glide = if e.len == HELD { 0 } else { (e.glide * e.len as f32).round() as u32 };
                    self.glide.start(self.pitch, p, glide);
                    self.elapsed = 0;
                }
                self.gate_left = e.len.max(1);
                self.id = e.id;
            }
            self.pitch = self.glide.at(self.elapsed);
            self.elapsed = self.elapsed.saturating_add(1);
            left[i] = self.pitch;
            right[i] = if self.gate_left > 0 { 1.0 } else { 0.0 };
            if self.gate_left != HELD {
                self.gate_left = self.gate_left.saturating_sub(1);
            }
        }
    }
}
