//! Notes → the cv the stage's `seq` node writes (pitch in Hz on the left channel, the gate 0 / 1 on the right), so an
//! fm / osc voice with its `cv` linked plays a module's notes like any built-in melody. The same arithmetic as
//! deck-events' `GateCv` and `Glide` (sample-accurate, exponential glide over `glide` × the note's length); no
//! allocation. A module with a `cv` audio output calls `GateCv::render` from `process`.
use crate::abi::{get_str, EventRec, MAX_EVENTS};
use crate::ctx::Ctx;

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
        self.from * (self.to / self.from).powf(offset as f32 / self.frames as f32)
    }
}

/// One note resolved to this call's frames.
#[derive(Clone, Copy)]
struct Resolved {
    frame: u32,
    len: u32,
    pitch: Option<f32>,
    glide: f32,
}

#[derive(Debug, Clone)]
pub struct GateCv {
    /// the pitch now (Hz)
    pub pitch: f32,
    glide: Glide,
    elapsed: u32,
    gate_left: u32,
}

impl GateCv {
    /// holding `pitch` until the first note
    pub fn new(pitch: f32) -> GateCv {
        GateCv { pitch, glide: Glide::hold(pitch), elapsed: 0, gate_left: 0 }
    }

    /// One call: the `events` this call emitted (those named `note`, with a `pitch` prop and an optional `glide` prop
    /// 0..1) at their frames on the tempo, into `left` / `right`. A moment (length 0) opens the gate for one frame.
    pub fn render(&mut self, ctx: &Ctx, events: &[EventRec], note: &str, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let tempo = ctx.tempo();
        let from = ctx.frame();
        let per_frame = tempo.per_frame();
        let mut notes = [Resolved { frame: 0, len: 0, pitch: None, glide: 0.0 }; MAX_EVENTS];
        let mut count = 0;
        for e in events.iter().filter(|e| get_str(&e.name) == note).take(MAX_EVENTS) {
            let at = if e.start.is_nan() { from } else { tempo.frame_of(e.start).unwrap_or(from) };
            let len = if per_frame > 0.0 { e.len / per_frame } else { 0.0 };
            let prop = |key: &str| (0..(e.nprops as usize).min(e.keys.len())).find(|&i| get_str(&e.keys[i]) == key).map(|i| e.values[i]);
            notes[count] = Resolved {
                frame: (at - from).max(0.0).round() as u32,
                len: len.round() as u32,
                pitch: prop("pitch"),
                glide: prop("glide").unwrap_or(0.0).clamp(0.0, 1.0),
            };
            count += 1;
        }
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
                if let Some(p) = e.pitch {
                    self.glide.start(self.pitch, p, (e.glide * e.len as f32).round() as u32);
                    self.elapsed = 0;
                }
                self.gate_left = e.len.max(1);
                next += 1;
            }
            self.pitch = self.glide.at(self.elapsed);
            self.elapsed = self.elapsed.saturating_add(1);
            left[i] = self.pitch;
            right[i] = if self.gate_left > 0 { 1.0 } else { 0.0 };
            self.gate_left = self.gate_left.saturating_sub(1);
        }
    }
}
