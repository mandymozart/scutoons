//! The MIDI adapter (EVENTS.md §12–13): raw MIDI messages (one per call, the `device.in` / `DeviceMsg::Midi` bytes) →
//! `midi.note` and `midi.cc`. A note on and its off become **one note with a duration**, born when it ends (a note's
//! length is not known before): `pitch` in Hz (A4 = 440, equal temperament), `vel` 0–1, plus `note` (the key) and `ch`
//! (0–15). A controller → `midi.cc { cc, value 0–1, ch }`. Times are seconds on the deck's timeline (the caller's
//! clock). A key struck again while held ends the first note there; `release_all` ends what is still held (a panic, a
//! lost device). Everything else (pitch bend, program change, clock …) is ignored – `feed` says so.
use crate::event::Event;
use crate::names::{Names, Sym};
use crate::Time;

pub struct Midi {
    note: Sym,
    cc: Sym,
    pitch: Sym,
    vel: Sym,
    key: Sym,
    ch: Sym,
    cc_no: Sym,
    value: Sym,
    /// (on-time, velocity) per channel × key
    held: [Option<(f64, u8)>; 16 * 128],
    source: Option<String>,
}

/// a key's pitch in Hz
pub fn hz(note: u8) -> f32 {
    440.0 * 2f32.powf((f32::from(note) - 69.0) / 12.0)
}

impl Midi {
    /// `source`: where the bytes come from (`midi:keys`)
    pub fn new(names: &mut Names, source: Option<&str>) -> Midi {
        let mut k = |n: &str| names.key(n).expect("plain words");
        let (pitch, vel, key, ch, cc_no, value) = (k("pitch"), k("vel"), k("note"), k("ch"), k("cc"), k("value"));
        Midi {
            note: names.event("midi.note").expect("a name"),
            cc: names.event("midi.cc").expect("a name"),
            pitch,
            vel,
            key,
            ch,
            cc_no,
            value,
            held: [None; 16 * 128],
            source: source.map(str::to_string),
        }
    }

    /// One message at `at` seconds; the events it completes go to `out`. False when the message is not a note or a
    /// controller (or is malformed), so the caller can pass it on elsewhere.
    pub fn feed(&mut self, bytes: &[u8], at: f64, out: &mut Vec<Event>) -> bool {
        let [status, a, b] = match bytes {
            [s, a, b] => [*s, *a, *b],
            _ => return false,
        };
        if a > 127 || b > 127 {
            return false;
        }
        let ch = status & 0x0F;
        match status & 0xF0 {
            0x90 if b > 0 => {
                let slot = usize::from(ch) * 128 + usize::from(a);
                if let Some((on, v)) = self.held[slot].take() {
                    out.push(self.make(ch, a, on, at, v));
                }
                self.held[slot] = Some((at, b));
                true
            }
            0x80 | 0x90 => {
                if let Some((on, v)) = self.held[usize::from(ch) * 128 + usize::from(a)].take() {
                    out.push(self.make(ch, a, on, at, v));
                }
                true
            }
            0xB0 => {
                let mut e = Event::new(self.cc, Time::Seconds(at)).with(self.cc_no, f32::from(a)).with(self.value, f32::from(b) / 127.0).with(self.ch, f32::from(ch));
                e.source = self.source.clone();
                out.push(e);
                true
            }
            _ => false,
        }
    }

    /// end every held note at `at`
    pub fn release_all(&mut self, at: f64, out: &mut Vec<Event>) {
        for slot in 0..self.held.len() {
            if let Some((on, v)) = self.held[slot].take() {
                out.push(self.make((slot / 128) as u8, (slot % 128) as u8, on, at, v));
            }
        }
    }

    fn make(&self, ch: u8, note: u8, on: f64, off: f64, vel: u8) -> Event {
        let mut e = Event::new(self.note, Time::Seconds(on))
            .with(self.pitch, hz(note))
            .with(self.vel, f32::from(vel) / 127.0)
            .with(self.key, f32::from(note))
            .with(self.ch, f32::from(ch))
            .lasting(Time::Seconds((off - on).max(0.0)));
        e.source = self.source.clone();
        e
    }
}
