//! The golden run: the script's notes as they leave, hashed. A change of the hash is a change of what a drum rig plays.
//! And what the sequencer does, measured: the groove's notes at their sample positions at 160 BPM, however the calls are
//! cut; the accents; the swing; the transport stopping, starting and jumping; the preset.
mod common;
use common::*;
use deck_module::Kind;

const GOLDEN: u64 = 0x9ceb_b06d_997e_b03a;

/// 160 BPM at 48 kHz, 4 steps a beat: a beat is 18 000 frames, a step 4500
const STEP: u32 = 4500;

/// a note as it leaves: (lane 1 …, absolute frame, step id, key, velocity, length in beats)
type Note = (usize, u32, u32, f32, f32, f32);

/// the notes of `frames` frames in calls of `call` frames
fn play(b: &mut Bench<DrumSeq>, frames: usize, call: usize) -> Vec<Note> {
    let mut notes = vec![];
    let end = b.frame as usize + frames;
    while (b.frame as usize) < end {
        let start = b.frame as u32;
        b.run(call.min(end - b.frame as usize));
        for lane in 0..LANES {
            for e in b.rig.events_out(DrumSeq::LANE1 + lane) {
                assert_eq!(e.kind(), Some(Kind::NoteOn));
                assert!(e.pitch_hz().is_nan(), "the pitch is the host's to fill from the key");
                notes.push((lane + 1, start + e.frame, e.id, e.key(), e.velocity(), e.len));
            }
        }
    }
    notes.sort_by_key(|n| (n.1, n.0));
    notes
}

fn at_160() -> Bench<DrumSeq> {
    let mut b = bench(1);
    b.rig.set_tempo(0.0, 0.0, 160.0, 4, true);
    b
}

/// §4.4's groove: (lane, step 1 … 16, accented)
const GROOVE: [(usize, u32, bool); 14] = [
    (1, 1, true), (1, 9, true), (1, 11, false),
    (2, 5, true), (2, 13, true),
    (3, 1, true), (3, 3, false), (3, 5, true), (3, 7, false), (3, 9, true), (3, 11, false), (3, 13, true),
    (4, 15, false),
    (0, 0, false),
];
const KEYS: [f32; 4] = [36.0, 38.0, 42.0, 46.0];

/// the groove's notes over `bars` bars from bar 0
fn groove(bars: u32) -> Vec<Note> {
    let mut want: Vec<Note> = (0..bars)
        .flat_map(|bar| GROOVE.iter().filter(|g| g.0 > 0).map(move |&(lane, s, x)| {
            let k = bar * 16 + s - 1;
            (lane, k * STEP, k, KEYS[lane - 1], if x { 1.0 } else { 0.7 }, 0.125)
        }))
        .collect();
    want.sort_by_key(|n| (n.1, n.0));
    want
}

#[test]
fn the_notes_are_the_golden_ones() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let count: f32 = (0..3000).map(|k| step(&mut b, k, &mut h)).sum();
    assert!(count > 25.0, "{count} notes");
    assert_eq!(h.0, GOLDEN, "the notes changed: {:#018x}", h.0);
}

#[test]
fn the_default_is_the_groove_at_its_sample_positions_however_the_calls_are_cut() {
    let two_bars = 2 * 16 * STEP as usize;
    let want = groove(2);
    assert_eq!(want.len(), 26);
    for call in [128, 37, 1, 100] {
        assert_eq!(play(&mut at_160(), two_bars, call), want, "calls of {call} frames");
    }
}

#[test]
fn accents_play_at_the_accent_velocity_and_the_rest_at_normal() {
    let mut b = at_160();
    b.set("accent", 90.0).set("normal", 40.0);
    let notes = play(&mut b, 16 * STEP as usize, 128);
    let kick: Vec<f32> = notes.iter().filter(|n| n.0 == 1).map(|n| n.4).collect();
    assert_eq!(kick, [0.9, 0.9, 0.4], "X on 1 and 9, x on 11");
    // a cell turned: off is a rest, on and accent its velocity; the others do not move
    b.set("l2s5", 0.0).set("l2s6", 1.0).set("l2s13", 2.0);
    let snare: Vec<(u32, f32)> = play(&mut b, 16 * STEP as usize, 128).iter().filter(|n| n.0 == 2).map(|n| (n.2 % 16, n.4)).collect();
    assert_eq!(snare, [(5, 0.4), (12, 0.9)]);
}

#[test]
fn swing_makes_every_second_step_late_and_leaves_the_others() {
    let hats = |swing: f32| {
        let mut b = at_160();
        b.set("swing", swing).set("l3s2", 1.0).set("l3s4", 1.0);
        play(&mut b, 4 * STEP as usize, 37).iter().filter(|n| n.0 == 3).map(|n| n.1).collect::<Vec<u32>>()
    };
    assert_eq!(hats(0.0), [0, STEP, 2 * STEP, 3 * STEP]);
    // 100 %: a third of a step late (1500 frames), 50 %: a sixth (750)
    assert_eq!(hats(100.0), [0, STEP + 1500, 2 * STEP, 3 * STEP + 1500]);
    assert_eq!(hats(50.0), [0, STEP + 750, 2 * STEP, 3 * STEP + 750]);
    // a swung step whose grid beat is in one call (4500 in 4480 … 4607) and its time in a later one (6000 in 5888 … 6015)
    // is played once, in the later call
    let mut b = at_160();
    b.set("swing", 100.0).set("l3s2", 1.0);
    let crossing = play(&mut b, 2 * STEP as usize, 128);
    assert_eq!(crossing.iter().filter(|n| n.0 == 3 && n.2 == 1).map(|n| n.1).collect::<Vec<_>>(), [STEP + 1500]);
}

#[test]
fn it_follows_the_transport_stop_start_and_position() {
    let mut b = at_160();
    let first = play(&mut b, 5 * STEP as usize, 128);
    assert_eq!(b.rig.control_out(DrumSeq::STEP).0, 5.0, "the step under the clock");
    assert_eq!(first.last().unwrap().2, 4, "the snare on step 5");
    // stopped: nothing plays, the step reads 0
    let at = b.frame as f64;
    b.rig.set_tempo(at, 5.0 / 4.0, 160.0, 4, false);
    assert!(play(&mut b, 8 * STEP as usize, 128).is_empty(), "stopped, no notes");
    assert_eq!(b.rig.control_out(DrumSeq::STEP).0, 0.0);
    // started where it stood: step 6 is next, one step later
    let at = b.frame as f64;
    b.rig.set_tempo(at, 5.0 / 4.0, 160.0, 4, true);
    let after = play(&mut b, 4 * STEP as usize, 128);
    assert_eq!(after.first().map(|n| (n.0, n.1 - at as u32, n.2)), Some((3, STEP, 6)), "the closed hat on step 7, a step on");
    // a jump to bar 3's beat 2: the snare plays there, at once
    let at = b.frame as f64;
    b.rig.set_tempo(at, 9.0, 160.0, 4, true);
    let jumped = play(&mut b, 128, 128);
    assert_eq!(jumped.iter().map(|n| (n.0, n.1 - at as u32, n.2)).collect::<Vec<_>>(), [(2, 0, 36), (3, 0, 36)]);
}

#[test]
fn rate_length_and_note_follow_their_knobs() {
    let mut b = at_160();
    b.set("rate", 2.0).set("steps", 4.0).set("note1", 24.0).set("length1", 100.0);
    // eighths: a step is 9000 frames; four steps wrap: the kick on 1 only (steps 9 and 11 are past the length)
    let kick: Vec<(u32, u32, f32, f32)> = play(&mut b, 8 * 9000, 128).iter().filter(|n| n.0 == 1).map(|n| (n.1, n.2, n.3, n.5)).collect();
    assert_eq!(kick, [(0, 0, 24.0, 0.5), (36_000, 4, 24.0, 0.5)]);
}

#[test]
fn the_preset_is_the_skank() {
    let mut b = at_160();
    b.rig.apply_preset();
    let notes = play(&mut b, 16 * STEP as usize, 128);
    let steps = |lane: usize| notes.iter().filter(|n| n.0 == lane).map(|n| n.2 + 1).collect::<Vec<u32>>();
    assert_eq!(steps(1), [1, 5, 9, 13]);
    assert_eq!(steps(2), [3, 7, 11, 15]);
    assert_eq!(steps(3), [1, 3, 5, 7, 9, 11, 13, 15]);
    assert!(steps(4).is_empty());
}
