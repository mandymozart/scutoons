//! The golden render: the script through the filter, hashed. A change of the hash is a change of the sound.
mod common;
use common::*;
use deck_module::dsp::arith::sin_turns;
use deck_module::Ev;

const GOLDEN: u64 = 0xc901_fade_bd64_439a;

/// the gain at `hz`: a sine of half a second through the filter, the second half measured
fn gain(b: &mut Bench<Filter>, hz: f64) -> f32 {
    let (mut sum, mut count) = (0.0f32, 0);
    for call in 0..188 {
        let x: Vec<f32> = (0..128).map(|i| sin_turns(hz * (b.frame + i) as f64 / RATE) as f32).collect();
        b.rig.set_audio_in(Filter::IN, 0, &x);
        b.rig.set_audio_in(Filter::IN, 1, &x);
        b.run(128);
        if call >= 94 {
            let out = b.rig.audio_out(Filter::OUT);
            assert!(out.chan(0) == out.chan(1), "both channels alike");
            sum += out.chan(0).iter().map(|v| v * v).sum::<f32>();
            count += 128;
        }
    }
    (2.0 * sum / count as f32).sqrt()
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..700).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 40.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn the_modes_have_their_shapes_and_24_db_is_twice_as_steep() {
    let at = |mode: f32, hz: f64| {
        let mut b = bench(1);
        b.set("mode", mode).set("cutoff", 1000.0).set("res", 0.0);
        db(gain(&mut b, hz))
    };
    assert!(at(0.0, 100.0).abs() < 0.2 && (at(0.0, 1000.0) + 3.0).abs() < 0.2, "lp12 passes below and is 3 dB down at the cutoff");
    let (two, four) = (at(0.0, 4000.0), at(1.0, 4000.0));
    assert!((two + 24.0).abs() < 1.5 && (four + 48.0).abs() < 3.0, "two octaves up: {two} dB and {four} dB");
    assert!(at(3.0, 100.0) < -35.0 && at(3.0, 8000.0).abs() < 0.5, "hp");
    assert!(at(2.0, 100.0) < -15.0 && at(2.0, 1000.0) > -4.0 && at(2.0, 10_000.0) < -15.0, "bp");
    assert!(at(4.0, 1000.0) < -30.0 && at(4.0, 100.0).abs() < 0.5, "notch");
}

#[test]
fn the_resonance_the_control_input_and_the_keyboard_move_the_peak() {
    let mut b = bench(1);
    b.rig.apply_preset();
    b.set("drive", 0.0).set("key", 0.0);
    let (peak, off) = (db(gain(&mut b, 400.0)), db(gain(&mut b, 100.0)));
    assert!(peak > 12.0 && off.abs() < 1.0, "res 75 % rings at the cutoff: {peak} dB, {off} dB below it");
    // an octave up by the control input: the peak is at 800 Hz now
    b.rig.set_control_in(Filter::CUTOFF_MOD, 1.0);
    gain(&mut b, 800.0);
    assert!((db(gain(&mut b, 800.0)) - peak).abs() < 0.5);
    b.rig.set_control_in(Filter::CUTOFF_MOD, 0.0);
    // key 100 %: a note two octaves above middle C takes the cutoff two octaves up
    b.set("key", 100.0);
    b.rig.set_events_in(Filter::NOTES, &[Ev::note_on(0, 1, 1046.502, 84.0, 1.0)]);
    gain(&mut b, 1600.0);
    assert!((db(gain(&mut b, 1600.0)) - peak).abs() < 0.5);
}

#[test]
fn drive_saturates_and_the_cut_of_the_blocks_changes_nothing() {
    let mut b = bench(1);
    b.set("cutoff", 20_000.0).set("drive", 100.0);
    let hot = gain(&mut b, 200.0);
    assert!(hot > 1.05 && hot < 1.5, "a full-scale sine driven hard is squared off, not louder than its ceiling: {hot}");
    let run = |block: usize| {
        let mut b = bench(1);
        b.set("mode", 1.0).set("res", 60.0).set("key", 100.0).set("cutoff", 500.0);
        let mut all = vec![];
        while all.len() < 24_000 {
            let at = all.len();
            b.feed(Filter::IN, 0, block, 0.5);
            let note = [Ev::note_on((6000 - at.min(6000)) as u32, 1, 880.0, 81.0, 1.0)];
            b.rig.set_events_in(Filter::NOTES, if (at..at + block).contains(&6000) { &note } else { &[] });
            b.run(block);
            all.extend_from_slice(b.rig.audio_out(Filter::OUT).chan(0));
        }
        all[..24_000].to_vec()
    };
    assert!(run(128) == run(37), "the same samples in calls of 128 and of 37 frames");
}
