//! The golden render: the script's waves, hashed. A change of the hash is a change of the sound.
mod common;
use common::*;
use deck_module::{Ev, Expr};

const GOLDEN: u64 = 0x2aa4_9e5a_1fd6_569d;

/// one second in calls of `block` frames with `notes` at their absolute frames
fn render(b: &mut Bench<Osc>, block: usize, notes: &[(usize, Ev)]) -> Vec<f32> {
    let mut all = vec![];
    while all.len() < 48_000 {
        let at = all.len();
        let due: Vec<Ev> = notes.iter().filter(|(f, _)| (at..at + block).contains(f)).map(|(f, e)| Ev { frame: (f - at) as u32, ..*e }).collect();
        b.rig.set_events_in(Osc::NOTES, &due);
        b.run(block);
        all.extend_from_slice(b.rig.audio_out(Osc::OUT).chan(0));
    }
    all
}

/// cycles in the render: the upward zero crossings
fn cycles(x: &[f32]) -> usize {
    x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..700).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.5 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn it_plays_the_notes_pitch_moved_by_the_knobs_the_bend_and_the_pitch_input() {
    let a440 = [(0, Ev::note_on(0, 1, 440.0, 69.0, 1.0))];
    let mut b = bench(1);
    b.set("wave", 3.0);
    assert!((261..=262).contains(&cycles(&render(&mut b, 128, &[]))), "middle C before the first note");
    // a second of the change (its glide), then a second to count
    let hz = |b: &mut Bench<Osc>, notes: &[(usize, Ev)]| {
        render(b, 128, notes);
        cycles(&render(b, 128, &[]))
    };
    assert!((439..=441).contains(&hz(&mut b, &a440)));
    b.set("semi", 12.0);
    assert!((879..=881).contains(&hz(&mut b, &[])), "an octave up");
    b.set("semi", 0.0).set("fine", 100.0);
    assert!((465..=467).contains(&hz(&mut b, &[])), "100 cents are a semitone");
    b.set("fine", 0.0);
    b.rig.set_control_in(Osc::PITCH, -12.0);
    assert!((219..=221).contains(&hz(&mut b, &[])), "the pitch input, an octave down");
    b.rig.set_control_in(Osc::PITCH, 0.0);
    let bent = [(0, Ev::note_expr(0, 1, Expr::Bend, 12.0)), (10, Ev::note_expr(0, 99, Expr::Bend, 7.0))];
    assert!((879..=881).contains(&hz(&mut b, &bent)), "the note's bend, and nobody else's");
}

#[test]
fn every_wave_sounds_and_more_voices_are_wider_not_louder() {
    let note = [(0, Ev::note_on(0, 1, 110.0, 45.0, 1.0))];
    let mut levels = vec![];
    for wave in 0..4 {
        let mut b = bench(1);
        b.set("wave", wave as f32);
        let x = render(&mut b, 128, &note);
        assert!((109..=111).contains(&cycles(&x)), "wave {wave}: {}", cycles(&x));
        assert!(x.iter().all(|v| v.abs() < 1.3));
        levels.push(rms(&x));
    }
    // saw 1/√3, pulse 1, triangle 1/√3, sine 1/√2
    for (got, want) in levels.iter().zip([0.577, 1.0, 0.577, 0.707]) {
        assert!((got - want).abs() < 0.02, "{levels:?}");
    }
    let mut b = bench(1);
    b.rig.apply_preset();
    // they beat against each other, so the level wanders around one saw's
    let wide = rms(&render(&mut b, 128, &note));
    assert!(wide > 0.3 && wide < 0.8, "seven detuned saws about as loud as one: {wide}");
    assert_eq!(b.rig.module().count(), 7);
}

#[test]
fn fm_swings_the_frequency_and_the_cut_of_the_blocks_changes_nothing() {
    let mut b = bench(1);
    b.set("wave", 3.0);
    let plain = cycles(&render(&mut b, 128, &[]));
    b.rig.set_audio_in(Osc::FM, 0, &[1.0; 128]);
    assert!((2 * plain - 2..=2 * plain + 2).contains(&cycles(&render(&mut b, 128, &[]))), "+1 doubles the frequency");
    let notes = [(1000, Ev::note_on(0, 1, 220.0, 57.0, 1.0)), (5000, Ev::note_on(0, 2, 330.0, 64.0, 1.0)), (9000, Ev::note_expr(0, 2, Expr::Bend, 3.0))];
    let run = |block: usize| {
        let mut b = bench(1);
        b.set("voices", 3.0).set("detune", 12.0).set("wave", 1.0);
        render(&mut b, block, &notes)[..48_000].to_vec()
    };
    assert!(run(128) == run(37), "the same samples in calls of 128 and of 37 frames");
}
