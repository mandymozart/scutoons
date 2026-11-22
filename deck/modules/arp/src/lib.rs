//! `arp` – an arpeggiator: the notes held on `notes` (a note on at key down, its off at key up) leave one after another
//! on `out`, a note a step, up, down, up and down or in the order they were played, through `octaves`, moved by
//! `transpose`. Step `k` plays at beat `k / rate` and its number alone picks the note (the SDK's `dsp::arp`, the one
//! copy deck-events' `arp` generator plays too), so the pattern is on the beat grid however the blocks are cut. A step
//! lasts `gate` of itself; with `latch` on the released keys keep playing until a key is pressed again. A reset ends
//! the set and passes on.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p arp-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/arp_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arp::{steps, Held, Mode, Note};
use deck_module::{math, Block, Ctx, Ev, Kind, Module};

/// steps a beat of each `rate` choice
const RATES: [f64; 9] = [0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

#[derive(Module)]
#[module(name = "arp", doc = "an arpeggiator: the held notes one after another on the beat grid")]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "the held notes: a note on at key down, its off at key up")]
#[port(output, event, name = "out", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "a note a step")]
#[preset(name = "crow", doc = "Up in eighths over two octaves, each note half a step long.", values(octaves = 2.0))]
pub struct Arp {
    #[param(choices = "up|down|updown|played", default = 0, doc = "up: low to high; down; updown: up and back; played: in the order the keys went down")]
    pub mode: f32,
    #[param(choices = "1/1|1/2|1/4|1/4t|1/8|1/8t|1/16|1/16t|1/32", default = 4, doc = "the step as a note value (t = a triplet)")]
    pub rate: f32,
    #[param(range = 5.0..100.0, default = 50.0, unit = "%", doc = "how long a note holds, of its step")]
    pub gate: f32,
    #[param(range = 1.0..4.0, default = 1.0, unit = "oct", doc = "how many octaves the pattern climbs")]
    pub octaves: f32,
    #[param(range = -24.0..24.0, default = 0.0, unit = "st", doc = "every note moved by semitones")]
    pub transpose: f32,
    #[param(choices = "off|on", default = 0, doc = "on: released keys keep playing until a key is pressed again")]
    pub latch: f32,
    held: Held,
}

impl Arp {
    /// steps a beat
    pub fn per_beat(&self) -> f64 {
        RATES[(math::round(self.rate).max(0.0) as usize).min(RATES.len() - 1)]
    }

    fn event(&mut self, block: &mut Block, e: &Ev) {
        match e.kind() {
            Some(Kind::NoteOn) => self.held.press(Note { id: e.id, hz: e.pitch_hz(), key: e.key(), velocity: e.velocity(), glide: e.glide() }),
            Some(Kind::NoteOff) => self.held.release(e.id),
            Some(Kind::Reset) => {
                self.held.clear();
                block.emit(Self::OUT, Ev::reset(e.frame));
            }
            _ => {}
        }
    }
}

impl Module for Arp {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        self.held.set_latch(self.latch >= 0.5);
        let mode = Mode::from_index(math::round(self.mode));
        let octaves = math::round(self.octaves).clamp(1.0, 4.0) as u32;
        let per_beat = self.per_beat();
        let len = (f64::from(self.gate) / 100.0 / per_beat) as f32;
        let tempo = ctx.tempo();
        let start = ctx.frame();
        // a step belongs to the call its rounded frame falls in: every cut plays it on the same frame
        let (from, to) = (tempo.beat_at(start - 0.5), tempo.beat_at(start + ctx.frames() as f64 - 0.5));
        let input = block.events_in(Self::NOTES);
        let mut next = 0;
        for (k, beat) in steps(from, to, per_beat) {
            let Some(at) = tempo.frame_of(beat) else { break };
            let frame = (at - start + 0.5).max(0.0) as u32;
            // what arrived up to the step's frame plays in it
            while next < input.len() && input[next].frame <= frame {
                self.event(block, &input[next]);
                next += 1;
            }
            if let Some((note, octave)) = self.held.step(k, mode, octaves) {
                let n = note.moved(12.0 * f64::from(octave) + f64::from(self.transpose));
                // the step is the note's id
                block.emit(Self::OUT, Ev::note_on(frame, k as u32, n.hz, n.key, n.velocity).gliding(n.glide).lasting(len));
            }
        }
        for e in &input[next..] {
            self.event(block, e);
        }
    }
}
