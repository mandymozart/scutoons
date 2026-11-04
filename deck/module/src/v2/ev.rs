//! `Ev`: one event record in an event port's region (ABI-V2 §1.4) – 40 bytes, little-endian. It is deck-events'
//! `Event` (a name, a time, ≤ 6 positional props) minus what must never touch the audio thread, plus MIDI-like notes:
//! note on / off paired by an id, a per-note expression, an all-notes-off reset. A note carries its pitch both ways,
//! Hz in `v[0]` and the MIDI key (fractional, 60.0 = middle C) in `v[3]`; the core's event host fills whichever the
//! source lacked, so a module reads the one its design wants and never converts.

/// `Ev::kind`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// a named deck-events event: `name` indexes the port's `names`, `v` holds the port's `props` in order
    Event = 0,
    /// `v`: pitch Hz, velocity 0…1, glide 0…1, key (MIDI, fractional)
    NoteOn = 1,
    NoteOff = 2,
    /// `v[0]`: which (`Expr`), `v[1]`: the value
    NoteExpr = 3,
    /// all notes off (the panic)
    Reset = 4,
}

/// what a `NoteExpr` changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Expr {
    /// semitones
    Bend = 0,
    Pressure = 1,
    Slide = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ev {
    /// the sample in this block, 0 … frames − 1
    pub frame: u32,
    /// `Kind as u8`
    pub kind: u8,
    /// the declared event port (per direction)
    pub port: u8,
    /// `Kind::Event`: the index into the port's `names`
    pub name: u16,
    /// notes: the note's id – host-assigned on input, module-assigned on output; on and off share it
    pub id: u32,
    /// beats; 0 = a moment / until `NoteOff` (outputs only: the host expands an input's length into a `NoteOff`)
    pub len: f32,
    /// the values (NaN = absent): see `Kind`
    pub v: [f32; 6],
}

const NONE: [f32; 6] = [f32::NAN; 6];

impl Ev {
    /// a named event (the port's `names[name]`) at `frame`, every prop absent
    pub const fn event(frame: u32, name: u16) -> Ev {
        Ev { frame, kind: Kind::Event as u8, port: 0, name, id: 0, len: 0.0, v: NONE }
    }

    /// a note on: pitch in Hz and / or the MIDI key (NaN = let the host fill it), velocity 0…1
    pub const fn note_on(frame: u32, id: u32, hz: f32, key: f32, velocity: f32) -> Ev {
        Ev { frame, kind: Kind::NoteOn as u8, port: 0, name: 0, id, len: 0.0, v: [hz, velocity, 0.0, key, f32::NAN, f32::NAN] }
    }

    pub const fn note_off(frame: u32, id: u32) -> Ev {
        Ev { frame, kind: Kind::NoteOff as u8, port: 0, name: 0, id, len: 0.0, v: NONE }
    }

    pub const fn note_expr(frame: u32, id: u32, which: Expr, value: f32) -> Ev {
        Ev { frame, kind: Kind::NoteExpr as u8, port: 0, name: 0, id, len: 0.0, v: [which as u8 as f32, value, f32::NAN, f32::NAN, f32::NAN, f32::NAN] }
    }

    pub const fn reset(frame: u32) -> Ev {
        Ev { frame, kind: Kind::Reset as u8, port: 0, name: 0, id: 0, len: 0.0, v: NONE }
    }

    /// prop slot `i` (0 … 5) set to `value`
    pub const fn with(mut self, i: usize, value: f32) -> Ev {
        if i < 6 {
            self.v[i] = value;
        }
        self
    }

    /// lasting `beats` (an output; the host turns it into a deck-events `dur`)
    pub const fn lasting(mut self, beats: f32) -> Ev {
        self.len = beats;
        self
    }

    /// a note's glide 0…1 (`v[2]`)
    pub const fn gliding(self, glide: f32) -> Ev {
        self.with(2, glide)
    }

    /// `None` for a byte no `Kind` has (a host bug or a newer ABI)
    pub fn kind(&self) -> Option<Kind> {
        [Kind::Event, Kind::NoteOn, Kind::NoteOff, Kind::NoteExpr, Kind::Reset].get(self.kind as usize).copied()
    }

    /// prop slot `i`; `None` when absent (NaN) or past the six
    pub fn prop(&self, i: usize) -> Option<f32> {
        self.v.get(i).copied().filter(|v| !v.is_nan())
    }

    /// `NoteOn`: the pitch in Hz
    pub fn pitch_hz(&self) -> f32 {
        self.v[0]
    }

    /// `NoteOn`: the MIDI key, fractional (60.0 = middle C)
    pub fn key(&self) -> f32 {
        self.v[3]
    }

    /// `NoteOn`: 0…1
    pub fn velocity(&self) -> f32 {
        self.v[1]
    }

    /// `NoteOn`: 0…1 (absent = 0)
    pub fn glide(&self) -> f32 {
        self.prop(2).unwrap_or(0.0)
    }

    /// `NoteExpr`: which and the value
    pub fn expr(&self) -> Option<(Expr, f32)> {
        let which = [Expr::Bend, Expr::Pressure, Expr::Slide].get(self.v[0] as usize).copied()?;
        Some((which, self.v[1]))
    }
}

const _: () = assert!(core::mem::size_of::<Ev>() == 40);

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::offset_of;

    #[test]
    fn an_ev_is_forty_bytes_in_the_order_of_the_design() {
        assert_eq!(
            [offset_of!(Ev, frame), offset_of!(Ev, kind), offset_of!(Ev, port), offset_of!(Ev, name), offset_of!(Ev, id), offset_of!(Ev, len), offset_of!(Ev, v)],
            [0, 4, 5, 6, 8, 12, 16]
        );
    }

    #[test]
    fn notes_carry_hz_and_key_and_absent_props_read_none() {
        let on = Ev::note_on(5, 7, 440.0, 69.0, 0.5).gliding(0.25);
        assert_eq!((on.kind(), on.pitch_hz(), on.key(), on.velocity(), on.glide(), on.prop(4)), (Some(Kind::NoteOn), 440.0, 69.0, 0.5, 0.25, None));
        assert_eq!(Ev::note_expr(0, 7, Expr::Pressure, 0.3).expr(), Some((Expr::Pressure, 0.3)));
        let e = Ev::event(3, 1).with(0, 2.0).lasting(0.5);
        assert_eq!((e.kind(), e.name, e.prop(0), e.prop(1), e.len), (Some(Kind::Event), 1, Some(2.0), None, 0.5));
        assert_eq!(Ev { kind: 9, ..Ev::reset(0) }.kind(), None);
    }
}
