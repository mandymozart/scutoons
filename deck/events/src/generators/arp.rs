//! The arpeggiator as a generator (owner 2026-11-01: "arpeggiators are just like instruments, but can be rigged … ableton
//! chain"): events → events. An incoming note sets the root and holds its chord, the root × the ratios (a continuous
//! chord shape – no 12-tone grid required); the steps and the note each plays are the `arp` module's rule
//! (`deck_module::dsp::arp`), so the pattern stays on the grid however the windows are cut.
use super::steps;
use crate::event::{Event, Time};
use crate::generator::{Generator, Inbox, Outbox, Ports, Span};
use crate::names::Sym;
use deck_module::dsp::arp::{Held, Note, HELD};
pub use deck_module::dsp::arp::Mode as ArpMode;

#[derive(Debug, Clone)]
pub struct Arp {
    pub note: Sym,
    pub pitch: Sym,
    /// the chord as frequency ratios to the root, low to high (1, 5/4, 3/2 = a just major triad); at most 16
    pub ratios: Vec<f32>,
    pub octaves: u32,
    pub mode: ArpMode,
    pub per_beat: f64,
    pub length: f64,
    /// the chord on the last root heard (empty = silent until a note comes)
    held: Held,
}

impl Arp {
    pub fn new(note: Sym, pitch: Sym, ratios: Vec<f32>, octaves: u32, mode: ArpMode, per_beat: f64, length: f64) -> Arp {
        Arp { note, pitch, ratios, octaves, mode, per_beat, length, held: Held::new() }
    }

    fn root(&mut self, e: &Event) {
        if e.name != self.note {
            return;
        }
        if let Some(root) = e.props.get(self.pitch) {
            self.held.clear();
            for (id, r) in self.ratios.iter().take(HELD).enumerate() {
                self.held.press(Note { id: id as u32, hz: root * r, ..Note::SILENT });
            }
        }
    }
}

impl Generator for Arp {
    fn ports(&self) -> Ports {
        Ports { ins: &["notes"], outs: &["notes"], params: &["rate"] }
    }

    fn set(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.per_beat = f64::from(value.max(0.0));
        }
    }

    fn query(&mut self, span: &Span, ins: &Inbox, out: &mut Outbox) {
        let input = ins.port(0);
        let mut next = 0;
        let dur = self.length.clamp(0.0, 1.0) / self.per_beat.max(f64::MIN_POSITIVE);
        for (k, beat) in steps(span.from, span.to, self.per_beat) {
            let at = span.tempo.frame_of(beat).unwrap_or(span.from_frame);
            // every input note up to this step sets the root (inputs are sorted by start)
            while next < input.len() && input[next].frame(span.tempo, span.from_frame) <= at {
                self.root(&input[next]);
                next += 1;
            }
            let Some((note, octave)) = self.held.step(k, self.mode, self.octaves) else { continue };
            let pitch = note.hz * 2f32.powi(octave as i32);
            out.push(0, Event::new(self.note, Time::Beats(beat)).lasting(Time::Beats(dur)).with(self.pitch, pitch));
        }
        // notes after the last step still set the root for the next window
        for e in &input[next..] {
            self.root(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Names;
    use deck_core::tempo::TempoMap;

    fn arp(n: &mut Names, mode: ArpMode) -> Arp {
        Arp::new(n.event("music.note").unwrap(), n.key("pitch").unwrap(), vec![1.0, 1.25, 1.5], 2, mode, 4.0, 0.5)
    }

    fn run(a: &mut Arp, input: Vec<Event>, beats: f64) -> Vec<f32> {
        let t = TempoMap::new(120.0, 4, 48_000.0);
        let span = Span::frames(&t, 0.0, beats * 24_000.0);
        let mut out = Outbox::new(1, 256);
        a.query(&span, &Inbox { ports: &[input] }, &mut out);
        out.port(0).iter().map(|e| e.props.get(a.pitch).unwrap()).collect()
    }

    #[test]
    fn an_arp_climbs_the_chord_through_the_octaves() {
        let mut n = Names::new();
        let mut a = arp(&mut n, ArpMode::Up);
        let root = Event::beats(a.note, 0.0).with(a.pitch, 100.0);
        assert_eq!(run(&mut a, vec![root], 2.0), [100.0, 125.0, 150.0, 200.0, 250.0, 300.0, 100.0, 125.0]);
    }

    #[test]
    fn up_down_turns_without_repeating_the_ends_and_waits_for_a_root() {
        let mut n = Names::new();
        let mut a = arp(&mut n, ArpMode::UpDown);
        a.octaves = 1;
        assert!(run(&mut a, vec![], 1.0).is_empty(), "silent until a note comes");
        let root = Event::beats(a.note, 0.0).with(a.pitch, 100.0);
        assert_eq!(run(&mut a, vec![root], 2.0), [100.0, 125.0, 150.0, 125.0, 100.0, 125.0, 150.0, 125.0]);
        let mut d = arp(&mut n, ArpMode::Down);
        d.octaves = 1;
        let root = Event::beats(d.note, 0.0).with(d.pitch, 100.0);
        assert_eq!(run(&mut d, vec![root], 0.75), [150.0, 125.0, 100.0]);
    }

    #[test]
    fn a_new_note_moves_the_root_from_its_step_on() {
        let mut n = Names::new();
        let mut a = arp(&mut n, ArpMode::Up);
        a.octaves = 1;
        let notes = vec![Event::beats(a.note, 0.0).with(a.pitch, 100.0), Event::beats(a.note, 0.5).with(a.pitch, 200.0)];
        assert_eq!(run(&mut a, notes, 1.0), [100.0, 125.0, 300.0, 200.0]);
    }
}
