//! The chain of the schema, one stage at a time: routing, the two filters (parallel and serial), the two amps and the
//! pan, voice stealing and the sustain pedal, hard sync, the LFO locked to the tempo, noise, and determinism.
mod common;
use analog_module::VOICES;
use common::*;

/// a quiet, wide-open bench: one voice's sine through a filter that does nothing, everything else off
fn clean() -> Bench {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20_000.0).set("f2_cutoff", 20_000.0).set("a1_vel", 0.0).set("a2_vel", 0.0);
    b
}

/// noise only, through two very different filters: 200 Hz low-pass in filter 1, 10 kHz low-pass in filter 2
fn noise_bench(route: f32) -> Bench {
    let mut b = Bench::new();
    b.set("o1_level", 0.0).set("noise_level", 1.0).set("noise_route", route);
    b.set("f1_cutoff", 200.0).set("f2_cutoff", 10_000.0).set("a1_vel", 0.0).set("a2_vel", 0.0);
    b.note_on(0, 261.63, 1.0);
    b
}

// ── routing and filters ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn each_source_goes_to_filter_1_filter_2_or_both() {
    let dark = brightness(&noise_bench(0.0).run(24_000).0[4800..]);
    let bright = brightness(&noise_bench(1.0).run(24_000).0[4800..]);
    let both = noise_bench(2.0).run(24_000).0;
    assert!(bright > 5.0 * dark, "filter 2 is the open one: {bright} against {dark}");
    let (l1, l2) = (noise_bench(0.0).run(24_000).0, noise_bench(1.0).run(24_000).0);
    let sum: Vec<f32> = l1.iter().zip(&l2).map(|(a, b)| a + b).collect();
    for (a, b) in sum.iter().zip(&both) {
        assert!((a - b).abs() < 1e-5, "'both' is the two filters added: {a} {b}");
    }
}

#[test]
fn every_source_has_its_own_route() {
    // osc 2 alone through filter 2 while osc 1 and its sub go through filter 1: only filter 2's amp is the open one
    let mut b = clean();
    b.set("o1_level", 0.0).set("o2_wave", 3.0).set("o2_level", 0.8).set("o2_route", 1.0).set("a2_pan", 1.0).set("a1_pan", -1.0);
    b.note_on(0, 440.0, 1.0);
    let (l, r) = b.run(12_000);
    assert!(peak(&l) < 1e-5 && rms(&r[6000..]) > 0.1, "osc 2 → filter 2 → amp 2 (right): left {} right {}", peak(&l), rms(&r[6000..]));
    for (name, route_param) in [("o1_sub", "sub1_route"), ("o2_sub", "sub2_route")] {
        let mut b = clean();
        b.set("o1_level", 0.0).set(name, 1.0).set(route_param, 1.0).set("a1_pan", -1.0).set("a2_pan", 1.0);
        b.note_on(0, 440.0, 1.0);
        let (l, r) = b.run(12_000);
        assert!(peak(&l) < 1e-5 && rms(&r[6000..]) > 0.1, "{name} follows {route_param}");
    }
}

#[test]
fn serial_feeds_filter_2_from_filter_1_by_the_to_f2_amount() {
    // osc 1 → filter 1 (dark) only; amp 1 is shut. In parallel filter 2 hears nothing; in serial it hears filter 1.
    let build = |serial: f32, amount: f32| {
        let mut b = Bench::new();
        b.set("o1_wave", 0.0).set("f1_cutoff", 300.0).set("f2_cutoff", 20_000.0).set("a1_level", 0.0).set("a2_vel", 0.0);
        b.set("f_serial", serial).set("f1_to_f2", amount);
        b.note_on(0, 220.0, 1.0);
        b.run(24_000).0
    };
    assert_eq!(peak(&build(0.0, 1.0)), 0.0, "parallel: filter 2 has no input");
    let full = build(1.0, 1.0);
    let half = build(1.0, 0.5);
    assert!(rms(&full[4800..]) > 0.05);
    assert!((rms(&half[4800..]) / rms(&full[4800..]) - 0.5).abs() < 0.02, "half the amount, half the level");
    // it is filter 1's sound: dark (a saw's top end is gone), not the open filter 2's
    let open = {
        let mut b = Bench::new();
        b.set("o1_wave", 0.0).set("f1_cutoff", 20_000.0).set("a1_level", 0.0).set("a2_vel", 0.0).set("f2_cutoff", 20_000.0);
        b.set("f_serial", 1.0).set("f1_to_f2", 1.0);
        b.note_on(0, 220.0, 1.0);
        b.run(24_000).0
    };
    assert!(brightness(&full[4800..]) < 0.5 * brightness(&open[4800..]));
}

#[test]
fn the_filter_modes_cut_where_they_should_in_the_voice() {
    // noise through filter 1 in each mode at 2 kHz: low-pass is dark, high-pass bright, band-pass and notch between
    let level = |mode: f32| {
        let mut b = Bench::new();
        b.set("o1_level", 0.0).set("noise_level", 1.0).set("f1_mode", mode).set("f1_cutoff", 2000.0).set("a1_vel", 0.0);
        b.note_on(0, 261.63, 1.0);
        let l = b.run(24_000).0;
        (brightness(&l[4800..]), rms(&l[4800..]))
    };
    let (lp, hp, bp, notch) = (level(0.0), level(2.0), level(1.0), level(3.0));
    assert!(lp.0 < bp.0 && bp.0 < hp.0, "brightness: lp {} bp {} hp {}", lp.0, bp.0, hp.0);
    assert!(notch.1 > bp.1, "the notch lets most of the noise through, the band-pass only a band");
}

#[test]
fn filter_envelope_and_key_tracking_move_the_cutoff() {
    let bright = |setup: &dyn Fn(&mut Bench), hz: f32, at: usize| {
        let mut b = Bench::new();
        b.set("o1_wave", 0.0).set("f1_cutoff", 400.0).set("fe1_a", 0.001).set("fe1_d", 0.5).set("fe1_s", 0.0).set("a1_vel", 0.0);
        setup(&mut b);
        b.note_on(0, hz, 1.0);
        let l = b.run(at + 2400).0;
        brightness(&l[at..])
    };
    let flat = bright(&|_| {}, 220.0, 100);
    let opened = bright(&|b| { b.set("f1_env", 1.0); }, 220.0, 100);
    assert!(opened > 1.5 * flat, "the envelope opens the filter at the start: {opened} against {flat}");
    let later = bright(&|b| { b.set("f1_env", 1.0); }, 220.0, 30_000);
    assert!(later < 0.7 * opened, "and it closes again with the decay: {later} against {opened}");
    // key tracking: a note two octaves up keeps its harmonic balance when the cutoff follows the key
    let tracked_low = bright(&|b| { b.set("f1_key", 1.0); }, 110.0, 24_000);
    let tracked_high = bright(&|b| { b.set("f1_key", 1.0); }, 440.0, 24_000);
    let free_low = bright(&|_| {}, 110.0, 24_000);
    let free_high = bright(&|_| {}, 440.0, 24_000);
    // the same shape an octave up has twice the change from sample to sample: two octaves, four times
    assert!((tracked_high / tracked_low / 4.0 - 1.0).abs() < 0.1, "tracked: {tracked_low} {tracked_high}");
    assert!(free_high / free_low < 0.8 * tracked_high / tracked_low, "an untracked cutoff leaves the high note duller: {free_low} {free_high}");
}

#[test]
fn resonance_and_drive_change_the_sound() {
    let play = |res: f32, drive: f32| {
        let mut b = Bench::new();
        b.set("o1_wave", 0.0).set("f1_cutoff", 1500.0).set("f1_res", res).set("f1_drive", drive).set("a1_vel", 0.0);
        b.note_on(0, 110.0, 1.0);
        b.run(24_000).0
    };
    let (plain, res, drive) = (play(0.0, 0.0), play(0.8, 0.0), play(0.0, 1.0));
    assert!(rms(&res[4800..]) > 1.3 * rms(&plain[4800..]), "a resonant peak near the cutoff adds energy");
    assert!(peak(&drive) > 0.0 && rms(&drive[4800..]) != rms(&plain[4800..]));
    assert!(peak(&play(1.0, 1.0)) < 8.0, "the loudest setting stays bounded");
}

// ── amps ────────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn pan_places_each_amp_with_constant_power() {
    let at = |pan: f32| {
        let mut b = clean();
        b.set("a1_pan", pan);
        b.note_on(0, 440.0, 1.0);
        let (l, r) = b.run(12_000);
        (rms(&l[6000..]), rms(&r[6000..]))
    };
    let (cl, cr) = at(0.0);
    assert!((cl - cr).abs() < 1e-6 && cl > 0.0);
    let (ll, lr) = at(-1.0);
    assert!(lr < 1e-6, "hard left: nothing on the right");
    let (rl, _) = at(1.0);
    assert!(rl < 1e-6, "hard right: nothing on the left");
    for pan in [-0.6, -0.2, 0.3, 0.9] {
        let (l, r) = at(pan);
        assert!(((l * l + r * r) / (ll * ll) - 1.0).abs() < 1e-3, "pan {pan}: power {}", (l * l + r * r) / (ll * ll));
    }
    assert!((cl * cl + cr * cr - ll * ll).abs() / (ll * ll) < 1e-3);
}

#[test]
fn two_amps_sum_to_stereo_with_their_own_levels() {
    let mut b = clean();
    b.set("o1_route", 2.0).set("a1_level", 0.4).set("a2_level", 0.8).set("a1_pan", -1.0).set("a2_pan", 1.0);
    b.note_on(0, 440.0, 1.0);
    let (l, r) = b.run(12_000);
    assert!((rms(&r[6000..]) / rms(&l[6000..]) - 2.0).abs() < 0.01, "amp 2 is twice amp 1");
}

#[test]
fn the_amp_envelope_shapes_the_note() {
    let mut b = clean();
    b.set("ae1_a", 0.5).set("ae1_d", 0.2).set("ae1_s", 0.5).set("ae1_r", 0.4);
    b.note_on(0, 440.0, 1.0);
    let l = b.run(48_000).0;
    let win = |from: usize| rms(&l[from..from + 480]);
    assert!(win(1000) < 0.5 * win(23_000), "rising through the 0.5 s attack");
    assert!((win(40_000) / win(23_000) - 0.5).abs() < 0.2 || win(40_000) < win(23_000), "settled on the sustain");
    b.note_off(0);
    let tail = b.run((0.4 * RATE) as usize + 480).0;
    assert!(rms(&tail[0..480]) > rms(&tail[8000..8480]) && rms(&tail[8000..8480]) > rms(&tail[16_000..16_480]), "falling through the release");
    assert_eq!(b.rig.module().active_voices(), 0);
}

// ── voices ──────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_ninth_note_steals_the_oldest_voice() {
    let mut b = clean();
    b.set("ae1_a", 0.001).set("ae1_s", 1.0).set("ae1_r", 10.0).set("volume", 0.5);
    let bins: Vec<f64> = (0..9).map(|i| 400.0 + 150.0 * i as f64).collect();
    for hz in &bins {
        b.note_on(0, *hz as f32, 1.0);
        b.run(2400);
        b.note_off(0);
        b.run(480); // each note keeps sounding in its release tail on its own voice
    }
    let l = b.run(8192).0;
    assert_eq!(b.rig.module().active_voices(), VOICES, "eight voices sound, never nine");
    assert!(mag(&l, bins[0]) < 0.005, "the first note was stolen: {}", mag(&l, bins[0]));
    for hz in &bins[1..] {
        assert!(mag(&l, *hz) > 0.01, "{hz} Hz was stolen: {}", mag(&l, *hz));
    }
    assert!(l.iter().all(|v| v.is_finite()));
}

#[test]
fn the_sustain_pedal_holds_released_notes() {
    let mut b = clean();
    b.set("ae1_r", 0.05).set("ae2_r", 0.05);
    b.note_on(0, 440.0, 1.0);
    b.run(4800);
    b.sustain = 1.0;
    b.note_off(0);
    let held = b.run(24_000).0;
    assert!(rms(&held[20_000..]) > 0.1, "the note rings on with the pedal down");
    b.sustain = 0.0;
    let after = b.run(9600).0;
    assert!(rms(&after[7200..]) < 1e-4, "and releases when it comes up");
    assert_eq!(b.rig.module().active_voices(), 0);
}

#[test]
fn a_note_played_while_the_pedal_is_down_still_ends_with_the_pedal() {
    let mut b = clean();
    b.set("ae1_r", 0.05).set("ae2_r", 0.05);
    b.sustain = 1.0;
    b.note_on(0, 440.0, 1.0);
    b.run(2400);
    b.note_off(0);
    b.note_on(1, 660.0, 1.0);
    b.run(2400);
    b.note_off(1);
    b.run(9600);
    assert_eq!(b.rig.module().active_voices(), 2);
    b.sustain = 0.0;
    b.run(4800);
    assert_eq!(b.rig.module().active_voices(), 0);
}

// ── oscillators in the voice ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn osc_2_syncs_to_osc_1_in_the_voice() {
    let make = |sync: f32| {
        let mut b = clean();
        // osc 1 muted but running; osc 2 a saw 2.7 times faster (+17 semitones ≈ ×2.67) through an open filter
        b.set("o1_level", 0.0).set("o2_wave", 0.0).set("o2_level", 0.6).set("o2_semi", 17.0).set("o2_sync", sync).set("o2_route", 0.0);
        b.set("ae1_a", 0.001);
        b.note_on(0, 200.0, 1.0);
        b.run(24_000).0
    };
    let (synced, free) = (make(1.0), make(0.0));
    let lag = 240; // 48000 / 200: osc 1's period
    let mismatch = |x: &[f32]| (6000..20_000).filter(|&i| (x[i] - x[i + lag]).abs() > 0.02).count();
    assert!(mismatch(&synced) < 50, "osc 2 repeats with osc 1's period: {} mismatches", mismatch(&synced));
    assert!(mismatch(&free) > 5000, "unsynced it does not: {}", mismatch(&free));
}

#[test]
fn the_sub_oscillators_sound_an_octave_below_their_oscillator() {
    let mut b = clean();
    b.set("o1_level", 0.0).set("o1_sub", 1.0).set("sub1_route", 0.0);
    b.note_on(0, 880.0, 1.0);
    let l = b.run(24_000).0;
    assert!((pitch_of(&l[4800..]) - 440.0).abs() < 1.0, "{}", pitch_of(&l[4800..]));
}

#[test]
fn pulse_width_changes_the_pulse() {
    let play = |pw: f32| {
        let mut b = Bench::new();
        b.set("o1_wave", 1.0).set("o1_pw", pw).set("f1_cutoff", 20_000.0);
        b.note_on(0, 200.0, 1.0);
        let l = b.run(24_000).0;
        l[6000..].iter().map(|&v| f64::from(v)).sum::<f64>() / 18_000.0
    };
    // the mean (dc) of a pulse is 2·width − 1; the amp env, level and a1 gain scale it alike, so compare the ratio
    let (narrow, wide) = (play(0.2), play(0.8));
    assert!(narrow < -0.05 && wide > 0.05 && (narrow + wide).abs() < 0.01, "{narrow} {wide}");
}

#[test]
fn the_lfo_moves_the_pitch_and_the_pulse_width() {
    let mut b = clean();
    b.set("l1_shape", 0.0).set("l1_rate", 5.0).set("l1_pitch", 12.0);
    b.note_on(0, 440.0, 1.0);
    let l = b.run(48_000).0;
    // an octave of vibrato either way: the zero-crossing spacing swings by a factor of ~4
    let up: Vec<usize> = (1..l.len()).filter(|&i| l[i - 1] < 0.0 && l[i] >= 0.0).collect();
    let gaps: Vec<usize> = up.windows(2).map(|w| w[1] - w[0]).collect();
    let (min, max) = (*gaps.iter().min().unwrap() as f64, *gaps.iter().max().unwrap() as f64);
    assert!(max / min > 3.0, "{min} … {max} samples a cycle");
}

// ── tempo ───────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_synced_lfo_follows_the_tempo_not_its_rate() {
    // a square LFO, one cycle a beat, on amp 1's tremolo: loud for the first half beat, silent for the second
    let play = |bpm: f64| {
        let mut b = clean();
        b.set("l1_shape", 3.0).set("l1_rate", 33.0).set("l1_sync", 4.0).set("l1_a1", 1.0).set("ae1_a", 0.001);
        b.rig.set_tempo(0.0, 0.0, bpm, 4, true);
        b.note_on(0, 440.0, 1.0);
        b.run(48_000).0
    };
    let w = |x: &[f32], from: f64, to: f64| rms(&x[(from * RATE) as usize..(to * RATE) as usize]);
    let fast = play(120.0); // a beat is 0.5 s: loud 0.05–0.2 s, silent 0.3–0.45 s
    assert!(w(&fast, 0.05, 0.2) > 0.1 && w(&fast, 0.3, 0.45) < 0.005, "120 bpm: {} then {}", w(&fast, 0.05, 0.2), w(&fast, 0.3, 0.45));
    let slow = play(60.0); // a beat is a second: loud to 0.5 s
    assert!(w(&slow, 0.3, 0.45) > 0.1 && w(&slow, 0.6, 0.9) < 0.005, "60 bpm: {} then {}", w(&slow, 0.3, 0.45), w(&slow, 0.6, 0.9));
}

// ── noise and determinism ───────────────────────────────────────────────────────────────────────────────────────

#[test]
fn noise_is_seeded_the_same_seed_the_same_noise() {
    let play = |seed: u64| {
        let mut b = Bench::seeded(seed);
        b.set("o1_level", 0.0).set("noise_level", 0.8).set("f1_cutoff", 20_000.0);
        b.note_on(0, 261.63, 1.0);
        b.run(4800).0
    };
    assert_eq!(play(7), play(7));
    assert_ne!(play(7), play(8));
    assert!(rms(&play(7)[2400..]) > 0.05);
}

#[test]
fn a_render_does_not_depend_on_how_the_host_cuts_the_blocks() {
    let render = |block: usize| {
        let mut b = Bench::new();
        b.apply(&preset("lead"));
        b.set("noise_level", 0.1).set("noise_route", 2.0).set("l2_shape", 4.0).set("l2_rate", 9.0).set("l2_f1", 1.0);
        b.note_on(0, 196.0, 0.8);
        b.note_on(1, 294.0, 0.6);
        let mut out = b.run_in(12_000, block).0;
        b.note_off(0);
        out.extend(b.run_in(12_000, block).0);
        out
    };
    let whole = render(1024);
    assert!(rms(&whole) > 0.01);
    for block in [64, 100, 333, 480] {
        assert_eq!(whole, render(block), "blocks of {block}");
    }
}

#[test]
fn two_synths_with_the_same_inputs_render_the_same_bits() {
    let play = || {
        let mut b = Bench::new();
        b.apply(&preset("sync lead"));
        b.note_on(0, 233.0, 0.7);
        b.run(20_000)
    };
    assert_eq!(play(), play());
}
