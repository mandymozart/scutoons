//! The MIDI adapter (EVENTS.md §12–13): raw MIDI messages of one input port (one per call, the `device.in` /
//! `DeviceMsg::Midi` bytes) → live events. A key down is a `midi.note` with no length – held until its off – carrying
//! `pitch` in Hz (A4 = 440, equal temperament), `vel` 0–1, `note` (the key), `ch` (0–15) and `port` (the input, 1 …);
//! its key up is a `midi.note_off` with the same `note`, `ch` and `port`. A controller → `midi.cc { cc, value 0–1, ch,
//! port }`. Times are seconds on the deck's timeline (the caller's clock). A key struck again while held ends the first
//! note there; `release_all` ends what is still held (a panic, a lost device). Everything else (pitch bend, program
//! change, clock …) is ignored – `feed` says so.
//!
//! `Recorder` turns the live stream into the recorded form: one `midi.note` with a duration, born when it ends.
use crate::event::Event;
use crate::names::{Names, Sym};
use crate::Time;

/// the name of a live note's end
pub const NOTE_OFF: &str = "midi.note_off";

pub struct Midi {
    note: Sym,
    off: Sym,
    cc: Sym,
    pitch: Sym,
    vel: Sym,
    key: Sym,
    ch: Sym,
    port_key: Sym,
    cc_no: Sym,
    value: Sym,
    port: u8,
    /// per channel × key
    held: [bool; 16 * 128],
    source: Option<String>,
}

/// a key's pitch in Hz
pub fn hz(note: u8) -> f32 {
    440.0 * libm::powf(2.0, (f32::from(note) - 69.0) / 12.0)
}

/// the prop keys of the adapter's events
struct Keys {
    note: Sym,
    off: Sym,
    key: Sym,
    ch: Sym,
    port: Sym,
}

impl Keys {
    fn new(names: &mut Names) -> Keys {
        let mut k = |n: &str| names.key(n).expect("plain words");
        let (key, ch, port) = (k("note"), k("ch"), k("port"));
        Keys { note: names.event("midi.note").expect("a name"), off: names.event(NOTE_OFF).expect("a name"), key, ch, port }
    }

    /// which key on which channel of which port an event names
    fn which(&self, e: &Event) -> [Option<f32>; 3] {
        [e.props.get(self.port), e.props.get(self.ch), e.props.get(self.key)]
    }
}

impl Midi {
    /// `source`: where the bytes come from (`midi:keys`); `port`: the input they arrive on (1 = the first)
    pub fn new(names: &mut Names, source: Option<&str>, port: u8) -> Midi {
        let k = Keys::new(names);
        let mut key = |n: &str| names.key(n).expect("plain words");
        let (pitch, vel, cc_no, value) = (key("pitch"), key("vel"), key("cc"), key("value"));
        Midi {
            note: k.note,
            off: k.off,
            cc: names.event("midi.cc").expect("a name"),
            pitch,
            vel,
            key: k.key,
            ch: k.ch,
            port_key: k.port,
            cc_no,
            value,
            port,
            held: [false; 16 * 128],
            source: source.map(str::to_string),
        }
    }

    /// One message at `at` seconds; its events go to `out`. False when the message is not a note or a controller (or is
    /// malformed), so the caller can pass it on elsewhere.
    pub fn feed(&mut self, bytes: &[u8], at: f64, out: &mut Vec<Event>) -> bool {
        self.feed_at(bytes, Time::Seconds(at), out)
    }

    /// `feed` at any time – `Time::Now` for a live input played as it arrives
    pub fn feed_at(&mut self, bytes: &[u8], at: Time, out: &mut Vec<Event>) -> bool {
        let [status, a, b] = match bytes {
            [s, a, b] => [*s, *a, *b],
            _ => return false,
        };
        if a > 127 || b > 127 {
            return false;
        }
        let ch = status & 0x0F;
        let slot = usize::from(ch) * 128 + usize::from(a);
        match status & 0xF0 {
            0x90 if b > 0 => {
                if self.held[slot] {
                    out.push(self.off(ch, a, at));
                }
                self.held[slot] = true;
                out.push(self.on(ch, a, b, at));
                true
            }
            0x80 | 0x90 => {
                if std::mem::take(&mut self.held[slot]) {
                    out.push(self.off(ch, a, at));
                }
                true
            }
            0xB0 => {
                let e = Event::new(self.cc, at).with(self.cc_no, f32::from(a)).with(self.value, f32::from(b) / 127.0);
                out.push(self.stamp(e, ch));
                true
            }
            _ => false,
        }
    }

    /// end every held note at `at`
    pub fn release_all(&mut self, at: f64, out: &mut Vec<Event>) {
        self.release_all_at(Time::Seconds(at), out);
    }

    /// `release_all` at any time
    pub fn release_all_at(&mut self, at: Time, out: &mut Vec<Event>) {
        for slot in 0..self.held.len() {
            if std::mem::take(&mut self.held[slot]) {
                out.push(self.off((slot / 128) as u8, (slot % 128) as u8, at));
            }
        }
    }

    fn on(&self, ch: u8, note: u8, vel: u8, at: Time) -> Event {
        let e = Event::new(self.note, at).with(self.pitch, hz(note)).with(self.vel, f32::from(vel) / 127.0).with(self.key, f32::from(note));
        self.stamp(e, ch)
    }

    fn off(&self, ch: u8, note: u8, at: Time) -> Event {
        self.stamp(Event::new(self.off, at).with(self.key, f32::from(note)), ch)
    }

    fn stamp(&self, e: Event, ch: u8) -> Event {
        let mut e = e.with(self.ch, f32::from(ch)).with(self.port_key, f32::from(self.port));
        e.source = self.source.clone();
        e
    }
}

/// The recorded form of a live MIDI stream: each `midi.note` and its `midi.note_off` (or a strike of the same key, or
/// `end_all`) become one `midi.note` with a duration, at the note's start, with its props and source.
pub struct Recorder {
    keys: Keys,
    held: Vec<([Option<f32>; 3], Event)>,
}

impl Recorder {
    pub fn new(names: &mut Names) -> Recorder {
        Recorder { keys: Keys::new(names), held: Vec::new() }
    }

    /// One live event; the notes it completes go to `out`. False when it is neither a live note nor its off.
    pub fn take(&mut self, e: &Event, out: &mut Vec<Event>) -> bool {
        let live_note = e.name == self.keys.note && e.dur.is_none();
        if !live_note && e.name != self.keys.off {
            return false;
        }
        let which = self.keys.which(e);
        if let Some(k) = self.held.iter().position(|(w, _)| *w == which) {
            let (_, on) = self.held.remove(k);
            out.push(ended(on, e.start));
        }
        if live_note {
            self.held.push((which, e.clone()));
        }
        true
    }

    /// end every note still held at `at`
    pub fn end_all(&mut self, at: Time, out: &mut Vec<Event>) {
        out.extend(self.held.drain(..).map(|(_, on)| ended(on, at)));
    }
}

fn ended(on: Event, at: Time) -> Event {
    let dur = match (on.start, at) {
        (Time::Seconds(a), Time::Seconds(b)) => Time::Seconds((b - a).max(0.0)),
        (Time::Beats(a), Time::Beats(b)) => Time::Beats((b - a).max(0.0)),
        _ => Time::Seconds(0.0),
    };
    on.lasting(dur)
}
