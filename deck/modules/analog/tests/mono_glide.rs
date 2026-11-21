//! Glide, mono / legato and the two control outputs: the glide's time measured on the pitch, the held-note stack (the
//! newest sounds, its note off goes back to the one under it), legato keeping the envelopes, mono restarting them, and
//! `env` / `level` following amp envelope 1 and the output.
mod common;
use analog_module::Analog;
use common::*;
use deck_module::Ev;

const MS: usize = 48;

/// one clean sine on filter 1, wide open: what sounds is the voice's pitch
fn sine_bench() -> Bench {
    let mut b = Bench::new();
    b.apply(&[]);
    b.set("o1_wave", 3.0).set("f1_cutoff", 20_000.0).set("a1_vel", 0.0).set("ae1_a", 0.001).set("ae1_d", 0.05).set("ae1_r", 0.01).set("ae2_r", 0.01);
    b
}

/// the pitch over `ms` milliseconds
fn pitch_over(b: &mut Bench, ms: usize) -> f64 {
    let (l, _) = b.run(ms * MS);
    pitch_of(&l)
}

fn near(got: f64, want: f64, tol: f64) -> bool {
    (got / want - 1.0).abs() < tol
}

#[test]
fn a_glide_takes_its_time_in_even_semitone_steps() {
    for mode in [0.0, 1.0, 2.0] {
        let mut b = sine_bench();
        b.set("mono", mode).set("glide", 0.1);
        b.note_on(0, 220.0, 1.0);
        b.run(200 * MS);
        b.note_off(0);
        b.note_on(1, 440.0, 1.0);
        // the middle of the glide (40–60 ms) is half way in pitch: half an octave up
        b.run(40 * MS);
        let mid = pitch_over(&mut b, 20);
        assert!(near(mid, 220.0 * 2f64.sqrt(), 0.02), "mode {mode}: half way at 50 ms: {mid}");
        // 60–80 ms: 0.7 octave up
        let late = pitch_over(&mut b, 20);
        assert!(near(late, 220.0 * 2f64.powf(0.7), 0.02), "mode {mode}: still gliding at 70 ms: {late}");
        // from 100 ms on: the note itself
        b.run(25 * MS);
        let end = pitch_over(&mut b, 30);
        assert!(near(end, 440.0, 0.003), "mode {mode}: there after the glide time: {end}");
    }
}

#[test]
fn without_glide_the_next_note_is_there_at_once() {
    for mode in [0.0, 1.0, 2.0] {
        let mut b = sine_bench();
        b.set("mono", mode);
        b.note_on(0, 220.0, 1.0);
        b.run(100 * MS);
        b.note_off(0);
        b.note_on(1, 440.0, 1.0);
        b.run(2 * MS);
        let now = pitch_over(&mut b, 20);
        assert!(near(now, 440.0, 0.003), "mode {mode}: {now}");
    }
}

#[test]
fn mono_plays_one_voice_and_goes_back_to_the_note_still_held() {
    for mode in [1.0, 2.0] {
        let mut b = sine_bench();
        b.set("mono", mode);
        b.note_on(0, 220.0, 1.0);
        b.run(50 * MS);
        b.note_on(1, 330.0, 1.0);
        b.run(2 * MS);
        assert!(near(pitch_over(&mut b, 30), 330.0, 0.003), "mode {mode}: the newest note sounds");
        assert_eq!(b.rig.module().active_voices(), 1, "mode {mode}: one voice");
        b.note_on(2, 440.0, 1.0);
        b.run(2 * MS);
        assert!(near(pitch_over(&mut b, 30), 440.0, 0.003), "mode {mode}");
        // the middle note lets go: nothing changes
        b.note_off(1);
        b.run(2 * MS);
        assert!(near(pitch_over(&mut b, 30), 440.0, 0.003), "mode {mode}: a note under the sounding one");
        // the sounding note lets go: back to the one still held
        b.note_off(2);
        b.run(2 * MS);
        assert!(near(pitch_over(&mut b, 30), 220.0, 0.003), "mode {mode}: back to the held note");
        assert_eq!(b.rig.module().active_voices(), 1);
        b.note_off(0);
        b.run(200 * MS);
        assert_eq!(b.rig.module().active_voices(), 0, "mode {mode}: the last note off releases");
    }
}

#[test]
fn poly_plays_a_voice_a_note() {
    let mut b = sine_bench();
    b.note_on(0, 220.0, 1.0);
    b.note_on(1, 330.0, 1.0);
    b.note_on(2, 440.0, 1.0);
    b.run(20 * MS);
    assert_eq!(b.rig.module().active_voices(), 3);
}

#[test]
fn legato_keeps_the_envelopes_on_an_overlapping_note_and_mono_restarts_them() {
    // amp envelope 1: a slow attack to 1, a quick decay to 0.5
    let env_after_second_note = |mode: f32, overlap: bool| -> (f32, f32) {
        let mut b = sine_bench();
        b.set("mono", mode).set("ae1_a", 0.05).set("ae1_d", 0.01).set("ae1_s", 0.5).set("ae1_r", 0.5);
        b.note_on(0, 220.0, 1.0);
        b.run(200 * MS);
        let before = b.rig.control_out(Analog::ENV).0;
        if !overlap {
            b.note_off(0);
        }
        b.note_on(1, 330.0, 1.0);
        b.run(25 * MS);
        (before, b.rig.control_out(Analog::ENV).0)
    };
    let (before, legato) = env_after_second_note(2.0, true);
    assert!((before - 0.5).abs() < 1e-3, "at the sustain level: {before}");
    assert!((legato - 0.5).abs() < 1e-3, "legato: the envelope goes on: {legato}");
    let (_, mono) = env_after_second_note(1.0, true);
    assert!(mono > 0.7, "mono: the envelope starts again from where it was: {mono}");
    let (_, apart) = env_after_second_note(2.0, false);
    assert!(apart > 0.7, "legato, the notes apart: the envelope starts again: {apart}");
}

#[test]
fn the_control_outputs_follow_the_envelope_and_the_level() {
    let mut b = sine_bench();
    b.set("ae1_s", 0.8);
    assert_eq!(b.rig.control_out(Analog::ENV).0, 0.0);
    b.note_on(0, 220.0, 1.0);
    let (l, r) = b.run(300 * MS);
    let (env, written) = b.rig.control_out(Analog::ENV);
    assert!(written && (env - 0.8).abs() < 1e-4, "env holds the sustain level: {env}");
    let (level, written) = b.rig.control_out(Analog::LEVEL);
    let top = peak(&l[l.len() - 4800..]).max(peak(&r[r.len() - 4800..]));
    assert!(written && level > 0.8 * top && level <= top + 1e-6 && level <= 1.0, "level follows the output's size: {level} (peak {top})");
    b.note_off(0);
    b.run(5 * MS);
    let falling = b.rig.control_out(Analog::LEVEL).0;
    assert!(falling > 0.5 * level, "it falls over its release, not at once: {falling}");
    b.run(1500 * MS);
    assert_eq!(b.rig.control_out(Analog::ENV).0, 0.0, "no voice, no envelope");
    assert!(b.rig.control_out(Analog::LEVEL).0 < 1e-4, "silence: {}", b.rig.control_out(Analog::LEVEL).0);
}

#[test]
fn the_level_output_stays_within_one_however_loud() {
    let mut b = Bench::new();
    b.apply(&preset("pad"));
    b.set("volume", 1.0).set("f1_res", 1.0).set("f1_drive", 1.0).set("o1_sub", 1.0).set("noise_level", 1.0);
    for k in 0..4 {
        b.note_on(k, 55.0 * (k + 1) as f32, 1.0);
    }
    b.run(500 * MS);
    let level = b.rig.control_out(Analog::LEVEL).0;
    assert!(level > 0.0 && level <= 1.0, "{level}");
}

#[test]
fn more_held_notes_than_the_stack_keeps_forget_the_oldest() {
    let mut b = sine_bench();
    b.set("mono", 1.0);
    let mut frame = 0.0;
    let mut call = |b: &mut Bench, events: &[Ev], frames: u32| {
        assert!(b.rig.set_events_in(0, events));
        b.rig.run(frame, frames);
        frame += f64::from(frames);
    };
    let on: Vec<Ev> = (1..=20u32).map(|id| Ev::note_on(0, id, 100.0 + 10.0 * id as f32, f32::NAN, 1.0)).collect();
    call(&mut b, &on, 64);
    assert_eq!(b.rig.module().active_voices(), 1);
    // the newest 16 (5 … 20) are kept: letting go of 20 … 6 goes back down the stack, 5 is the last
    for id in (6..=20u32).rev() {
        call(&mut b, &[Ev::note_off(0, id)], 64);
    }
    let (l, _) = b.run(30 * MS);
    assert!(near(pitch_of(&l), 150.0, 0.003), "note 5 sounds: {}", pitch_of(&l));
    call(&mut b, &[Ev::note_off(0, 5)], 64);
    b.run(200 * MS);
    assert_eq!(b.rig.module().active_voices(), 0, "1 … 4 were forgotten: the last kept note off releases");
}
