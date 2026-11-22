//! The golden run: the script's steps as they leave, hashed. A change of the hash is a change of what a rig plays. And
//! the arp's promises: the held notes on the beat grid, the same events however the blocks are cut.
mod common;
use common::*;
use deck_module::{Ev, Kind};

const GOLDEN: u64 = 0x6fa7_7134_7c4d_c090;

#[test]
fn the_steps_are_the_golden_ones() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let count: f32 = (0..1500).map(|k| step(&mut b, k, &mut h)).sum();
    assert!(count > 40.0, "{count} events");
    assert_eq!(h.0, GOLDEN, "the steps changed: {:#018x}", h.0);
}

/// (absolute frame, kind, id, Hz, key, velocity, beats) of every event that left
type Heard = Vec<(u64, u8, u32, u32, u32, u32, u32)>;

/// `frames` frames at 120 bpm in calls of `block` frames; `script` holds (absolute frame, event) in order
fn render(block: usize, frames: usize, script: &[(u64, Ev)], set: &[(&str, f32)]) -> Heard {
    let mut b = Bench::<Arp>::new(RATE, 1);
    for (name, v) in set {
        b.set(name, *v);
    }
    let mut heard = Heard::new();
    while (b.frame as usize) < frames {
        let (start, n) = (b.frame, block.min(frames - b.frame as usize));
        let events: Vec<Ev> = script
            .iter()
            .filter(|(at, _)| (start..start + n as u64).contains(at))
            .map(|(at, e)| Ev { frame: (at - start) as u32, ..*e })
            .collect();
        b.rig.set_events_in(Arp::NOTES, &events);
        b.run(n);
        for e in b.rig.events_out(Arp::OUT) {
            heard.push((start + u64::from(e.frame), e.kind, e.id, e.pitch_hz().to_bits(), e.key().to_bits(), e.velocity().to_bits(), e.len.to_bits()));
        }
    }
    heard
}

fn chord() -> Vec<(u64, Ev)> {
    vec![
        (0, Ev::note_on(0, 1, 261.6256, 60.0, 0.8)),
        (5_000, Ev::note_on(0, 2, 329.6276, 64.0, 0.7)),
        (12_000, Ev::note_on(0, 3, 391.9954, 67.0, 0.9)),
        (40_001, Ev::note_off(0, 1)),
        (61_111, Ev::reset(0)),
        (70_000, Ev::note_on(0, 4, f32::NAN, 50.0, 1.0)),
        (90_000, Ev::note_off(0, 4)),
    ]
}

#[test]
fn every_cut_plays_the_same_steps_on_the_grid() {
    let set = [("mode", 2.0), ("rate", 6.0), ("octaves", 2.0), ("transpose", 3.0)];
    let a = render(37, 120_000, &chord(), &set);
    assert_eq!(a, render(128, 120_000, &chord(), &set), "37- and 128-frame calls play the same");
    assert_eq!(a, render(1, 120_000, &chord(), &set), "and so does a frame a call");
    let notes: Vec<_> = a.iter().filter(|e| e.1 == Kind::NoteOn as u8).collect();
    // 1/16 at 120 bpm: a step every 6000 frames, from the first held note to the reset, then the lone key to its off
    assert_eq!(notes.len(), 11 + 3, "{notes:?}");
    assert!(notes.iter().all(|e| e.0 % 6_000 == 0 && e.2 as u64 == e.0 / 6_000), "on the grid, the step its id");
    assert!(a.iter().any(|e| e.1 == Kind::Reset as u8 && e.0 == 61_111), "the reset passes on at its frame");
}

#[test]
fn a_held_chord_climbs_in_eighths() {
    let mut b = Bench::<Arp>::new(RATE, 1);
    b.rig.apply_preset();
    let mut keys = Vec::new();
    for k in 0..(48_000 * 3 / 2 / 128) {
        let e = if k == 0 { vec![Ev::note_on(0, 1, f32::NAN, 60.0, 1.0), Ev::note_on(0, 2, f32::NAN, 63.0, 1.0)] } else { vec![] };
        b.rig.set_events_in(Arp::NOTES, &e);
        b.run(128);
        for e in b.rig.events_out(Arp::OUT) {
            assert_eq!((e.len, e.velocity()), (0.25, 1.0), "half of an eighth");
            keys.push(e.key());
        }
    }
    // the preset: up, 1/8 (12 000 frames at 120 bpm), two octaves – six steps in a second and a half
    assert_eq!(keys, [60.0, 63.0, 72.0, 75.0, 60.0, 63.0]);
}

#[test]
fn latch_keeps_the_released_chord() {
    let script = vec![(0, Ev::note_on(0, 1, f32::NAN, 60.0, 1.0)), (100, Ev::note_off(0, 1)), (30_000, Ev::note_on(0, 2, f32::NAN, 62.0, 1.0))];
    let latched = render(128, 48_000, &script, &[("latch", 1.0), ("rate", 6.0)]);
    let keys: Vec<f32> = latched.iter().map(|e| f32::from_bits(e.4)).collect();
    assert_eq!(keys, [60.0, 60.0, 60.0, 60.0, 60.0, 62.0, 62.0, 62.0], "held on after release, then the new key alone");
    let plain = render(128, 48_000, &script, &[("rate", 6.0)]);
    assert_eq!(plain.iter().map(|e| e.0).collect::<Vec<_>>(), [0, 30_000, 36_000, 42_000], "unlatched: silent between");
}
