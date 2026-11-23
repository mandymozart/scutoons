//! The golden render: the script through the sampler (a stereo sound at 44.1 kHz, read at the deck's 48 kHz), hashed. A
//! change of the hash is a change of the sound. And what the sampler promises: the sound's samples as they are at its
//! own pitch, an octave by a note, the voices and the oldest taken, the release, the loop, the part of the sound, the
//! velocity, silence without a sound.
mod common;
use common::*;
use deck_module::Ev;

const GOLDEN: u64 = 0xea60_3f09_186e_02a2;

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench_with(1, &sound(2, 30_000), 44_100.0);
    let mut h = Hash::new();
    let peak = (0..1200).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.05 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn at_the_root_it_plays_the_sounds_samples_as_they_are() {
    for channels in [1, 2] {
        let s = sound(channels, 5000);
        for block in [128, 37, 1] {
            let mut b = bench_with(1, &s, RATE as f32);
            let y = play(&mut b, 6000, block, &[(100, on(1, 60.0))]);
            for c in 0..2 {
                let want = &s[c.min(channels - 1)];
                assert!(y[c][..100].iter().all(|v| *v == 0.0), "{channels} ch: nothing before the note");
                assert!(y[c][100..5100] == want[..], "{channels} ch, calls of {block}, side {c}: the samples as they are");
                assert!(y[c][5100..].iter().all(|v| *v == 0.0), "and nothing after the end");
            }
            assert_eq!(b.rig.module().active_voices(), 0, "the voice ended with the sound");
        }
    }
    // fixed pitch: any key plays the sound as it is
    let s = sound(1, 2000);
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("pitch", 1.0);
    assert!(play(&mut b, 2000, 128, &[(0, on(1, 31.0))])[0] == s[0]);
}

#[test]
fn a_note_moves_the_pitch_and_the_rate_is_the_sounds() {
    let s = sound(1, 4000);
    // an octave up: every second sample, in half the time
    let mut b = bench_with(1, &s, RATE as f32);
    let y = play(&mut b, 4000, 128, &[(0, on(1, 72.0))]);
    assert!((0..2000).all(|i| y[0][i] == s[0][2 * i]), "an octave up reads every second frame");
    assert!(y[0][2000..].iter().all(|v| *v == 0.0), "and ends in half the time");
    // an octave down: the sound's frames on every second sample, a cubic between
    let mut b = bench_with(1, &s, RATE as f32);
    let y = play(&mut b, 8000, 128, &[(0, on(1, 48.0))]);
    assert!((0..4000).all(|i| y[0][2 * i] == s[0][i]), "an octave down");
    let between = y[0][2 * 1000 + 1];
    assert!((between - 0.5 * (s[0][1000] + s[0][1001])).abs() < 0.5, "a cubic between: {between}");
    // tune and a sound recorded at half the deck's rate: the same octave down
    let mut b = bench_with(1, &s, 24_000.0);
    let y = play(&mut b, 8000, 100, &[(0, on(1, 60.0))]);
    assert!((0..4000).all(|i| y[0][2 * i] == s[0][i]), "24 kHz at 48 kHz");
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("tune", 24.0).set("root", 48.0);
    let y = play(&mut b, 4000, 128, &[(0, on(1, 36.0))]);
    assert!((0..2000).all(|i| y[0][i] == s[0][2 * i]), "the key an octave under the root, tuned two up");
    // a bend moves a sounding voice
    let mut b = bench_with(1, &s, RATE as f32);
    let y = play(&mut b, 3000, 128, &[(0, on(1, 60.0)), (1000, Ev::note_expr(0, 1, deck_module::Expr::Bend, 12.0))]);
    assert!(y[0][..1000] == s[0][..1000] && (0..1000).all(|i| y[0][1000 + i] == s[0][1000 + 2 * i]), "bent an octave up at 1000");
}

#[test]
fn past_its_voices_a_note_takes_the_oldest() {
    let s = sound(1, 6000);
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("voices", 2.0);
    let y = play(&mut b, 3000, 128, &[(0, on(1, 60.0)), (500, on(2, 60.0)), (1000, on(3, 60.0))]);
    for i in 0..3000 {
        let want: f32 = match i {
            0..500 => s[0][i],
            500..1000 => s[0][i] + s[0][i - 500],
            _ => s[0][i - 500] + s[0][i - 1000],
        };
        assert_eq!(y[0][i], want, "frame {i}");
    }
    assert_eq!(b.rig.module().active_voices(), 2);
    // with all eight, three play together
    let mut b = bench_with(1, &s, RATE as f32);
    play(&mut b, 1200, 128, &[(0, on(1, 60.0)), (500, on(2, 60.0)), (1000, on(3, 60.0))]);
    assert_eq!(b.rig.module().active_voices(), 3);
}

#[test]
fn a_gate_releases_at_the_notes_off_and_a_one_shot_plays_on() {
    let s = vec![vec![0.5f32; 48_000]];
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("mode", 1.0).set("release", 0.01);
    let y = play(&mut b, 6000, 128, &[(0, on(1, 60.0)), (2000, off(1))]);
    assert!(y[0][..2000].iter().all(|v| *v == 0.5), "held: the sound");
    assert!(y[0][2000] < 0.5 && y[0][2300] > 0.0 && y[0][2300] < 0.25, "a line down over 480 frames: {}", y[0][2300]);
    assert!(y[0][2000 + 481..].iter().all(|v| *v == 0.0), "then silent");
    assert_eq!(b.rig.module().active_voices(), 0);
    // a one-shot does not hear the off; a reset releases it
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("release", 0.01);
    let y = play(&mut b, 6000, 128, &[(0, on(1, 60.0)), (2000, off(1)), (4000, Ev::reset(0))]);
    assert!(y[0][..4000].iter().all(|v| *v == 0.5), "the one-shot plays on");
    assert!(y[0][4000 + 481..].iter().all(|v| *v == 0.0), "the reset released it");
}

#[test]
fn a_loop_goes_round_its_part_while_held() {
    let s = sound(1, 1000);
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("mode", 2.0).set("start", 20.0).set("length", 30.0).set("release", 0.001);
    let y = play(&mut b, 4000, 128, &[(0, on(1, 60.0)), (3000, off(1))]);
    assert!((0..3000).all(|i| y[0][i] == s[0][200 + i % 300]), "frames 200 … 500, round and round");
    assert!(y[0][3100..].iter().all(|v| *v == 0.0), "released");
    // the same part as a one-shot: once
    let mut b = bench_with(1, &s, RATE as f32);
    b.set("start", 20.0).set("length", 30.0);
    let y = play(&mut b, 1000, 128, &[(0, on(1, 60.0))]);
    assert!(y[0][..300] == s[0][200..500] && y[0][300..].iter().all(|v| *v == 0.0));
}

#[test]
fn velocity_and_gain_scale_the_level() {
    let s = vec![vec![0.5f32; 20_000]];
    // the gain glides: read where it has arrived
    let at = |vel: f32, amount: f32, gain: f32| {
        let mut b = bench_with(1, &s, RATE as f32);
        b.set("velocity", amount).set("gain", gain);
        play(&mut b, 16_000, 128, &[(0, Ev::note_on(0, 1, f32::NAN, 60.0, vel))])[0][15_000]
    };
    assert_eq!(at(0.5, 100.0, 0.0), 0.25);
    assert_eq!(at(0.5, 0.0, 0.0), 0.5, "velocity 0 %: always full");
    assert!((at(1.0, 100.0, 6.0) / 0.5 - 1.995).abs() < 1e-3);
    assert_eq!(at(1.0, 100.0, -60.0), 0.0, "the gain's bottom is silence");
}

#[test]
fn without_a_sound_it_is_silent() {
    let mut b = bench_empty(1);
    let y = play(&mut b, 1024, 128, &[(0, on(1, 60.0))]);
    assert!(y.iter().flatten().all(|v| *v == 0.0));
    assert_ne!(b.rig.out_hdr().out_silent & 1, 0, "the output is marked silent");
    assert_eq!(b.rig.module().active_voices(), 0);
}
