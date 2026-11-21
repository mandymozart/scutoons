//! The golden run: the script's notes as they leave, hashed. A change of the hash is a change of what a rig plays.
mod common;
use common::*;
use deck_module::{Ev, Kind};

const GOLDEN: u64 = 0x3478_b813_b253_0b97;

#[test]
fn the_notes_are_the_golden_ones() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let count: f32 = (0..400).map(|k| step(&mut b, k, &mut h)).sum();
    assert!(count > 300.0, "{count} events");
    assert_eq!(h.0, GOLDEN, "the notes changed: {:#018x}", h.0);
}

#[test]
fn a_note_moves_in_hz_and_key_and_ends_under_its_id() {
    let mut b = bench(1);
    b.set("semi", 12.0);
    b.rig.set_events_in(Transpose::NOTES, &[Ev::note_on(5, 7, 220.0, 57.0, 0.8).gliding(0.25), Ev::note_on(9, 8, f32::NAN, 60.0, 1.0)]);
    b.run(128);
    let out = b.rig.events_out(Transpose::OUT);
    assert_eq!((out[0].frame, out[0].id, out[0].pitch_hz(), out[0].key(), out[0].velocity(), out[0].glide()), (5, 7, 440.0, 69.0, 0.8, 0.25));
    assert!(out[1].pitch_hz().is_nan() && out[1].key() == 72.0, "a note with only its key keeps only its key");
    // the knob moves while the note sounds: its off still ends it
    b.set("semi", -5.0).set("fine", 50.0);
    b.rig.set_events_in(Transpose::NOTES, &[Ev::note_off(3, 7), Ev::reset(4), Ev::note_on(6, 9, 440.0, 69.0, 1.0)]);
    b.run(64);
    let out = b.rig.events_out(Transpose::OUT);
    assert_eq!((out[0].kind(), out[0].frame, out[0].id), (Some(Kind::NoteOff), 3, 7));
    assert_eq!(out[1].kind(), Some(Kind::Reset));
    let down = 440.0 * 2f32.powf(-4.5 / 12.0);
    assert!((out[2].pitch_hz() - down).abs() < 0.01 && out[2].key() == 64.5, "{} Hz, key {}", out[2].pitch_hz(), out[2].key());
}

#[test]
fn the_preset_is_an_octave_up() {
    let mut b = bench(1);
    b.rig.apply_preset();
    b.rig.set_events_in(Transpose::NOTES, &[Ev::note_on(0, 1, 100.0, f32::NAN, 1.0)]);
    b.run(16);
    assert_eq!(b.rig.events_out(Transpose::OUT)[0].pitch_hz(), 200.0);
}
