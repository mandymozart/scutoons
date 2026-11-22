//! The golden render: the script through the reverb, hashed. A change of the hash is a change of the sound. And what
//! the reverb promises: its decay time, a stable 20 s tail, no denormals, the same samples however the blocks are cut.
mod common;
use common::*;

const GOLDEN: u64 = 0xe654_3a3a_988c_129c;

/// the reverb alone, at once, undamped
fn wet(b: &mut Bench<Reverb>) -> &mut Bench<Reverb> {
    b.set("mix", 100.0).set("predelay", 0.0).set("width", 100.0).set("damp", 20_000.0)
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..900).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn the_tail_falls_60_db_in_decay_within_10_percent() {
    for (size, decay) in [(0.0f32, 0.5f32), (20.0, 1.0), (50.0, 2.0), (100.0, 5.0)] {
        let mut b = bench(1);
        wet(&mut b).set("size", size).set("decay", decay);
        let (l, r) = impulse(&mut b, f64::from(decay) * 1.5 + 0.5);
        let t = rt60(&l, &r);
        assert!((t / f64::from(decay) - 1.0).abs() < 0.1, "size {size} %, decay {decay} s: measured {t:.3} s");
    }
}

#[test]
fn a_20_second_tail_is_stable_and_takes_its_time() {
    let mut b = bench(1);
    wet(&mut b).set("size", 100.0).set("decay", 20.0);
    let (l, r) = impulse(&mut b, 25.0);
    assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 1.0), "bounded");
    let t = rt60(&l, &r);
    assert!((t / 20.0 - 1.0).abs() < 0.1, "measured {t:.2} s");
    // a full-scale signal held for seconds into the longest tail stays bounded
    let mut b = bench(2);
    b.set("size", 100.0).set("decay", 20.0).set("mix", 100.0).set("damp", 3000.0);
    let mut peak = 0.0f32;
    for _ in 0..(10.0 * RATE / 128.0) as usize {
        b.feed(Reverb::IN, 0, 128, 1.0);
        b.feed(Reverb::IN, 1, 128, 1.0);
        b.run(128);
        let out = b.rig.audio_out(Reverb::OUT);
        peak = out.chan(0).iter().chain(out.chan(1)).fold(peak, |m, v| m.max(v.abs()));
    }
    assert!(peak.is_finite() && peak < 20.0, "{peak}");
}

#[test]
fn the_tail_ends_at_zero_never_in_the_denormals_and_then_costs_nothing() {
    let mut b = bench(1);
    wet(&mut b).set("size", 0.0).set("decay", 0.2).set("damp", 2000.0);
    let (l, r) = impulse(&mut b, 12.0);
    for (i, v) in l.iter().chain(&r).enumerate() {
        assert!(*v == 0.0 || v.is_normal(), "sample {i}: {v:e}");
    }
    assert!(l[l.len() - 48_000..].iter().all(|v| *v == 0.0), "silent at the end");
    // nothing linked and the tail run out: the output is marked silent
    b.rig.set_in_silent(Reverb::IN, true);
    b.run(128);
    assert_ne!(b.rig.out_hdr().out_silent & 1, 0);
}

#[test]
fn damping_dulls_the_highs_and_width_and_mix_do_what_they_say() {
    // the share of the tail's energy in its sample-to-sample differences: a measure of the highs
    let highs = |damp: f32| {
        let mut b = bench(1);
        wet(&mut b).set("decay", 2.0).set("damp", damp);
        let (l, _) = impulse(&mut b, 1.0);
        let all: f64 = l.iter().map(|v| f64::from(v * v)).sum();
        let diff: f64 = l.windows(2).map(|w| f64::from((w[1] - w[0]) * (w[1] - w[0]))).sum();
        diff / all
    };
    let (dull, bright) = (highs(1000.0), highs(20_000.0));
    assert!(dull < 0.5 * bright, "{dull} against {bright}");
    let mut b = bench(1);
    wet(&mut b).set("width", 0.0);
    let (l, r) = impulse(&mut b, 0.5);
    assert!(l == r, "width 0: a mono tail");
    let mut b = bench(1);
    wet(&mut b);
    let (l, r) = impulse(&mut b, 0.5);
    assert!(l != r, "width 100: two sides");
    let mut b = bench(1);
    b.set("mix", 0.0);
    settle(&mut b);
    let x: Vec<f32> = (0..128).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
    b.rig.set_audio_in(Reverb::IN, 0, &x);
    b.rig.set_audio_in(Reverb::IN, 1, &x);
    b.run(128);
    assert_eq!(b.rig.audio_out(Reverb::OUT).chan(0), &x[..], "mix 0: the dry signal");
}

#[test]
fn the_predelay_holds_the_tail_back_and_the_cut_of_the_blocks_changes_nothing() {
    let first = |predelay: f32| {
        let mut b = bench(1);
        wet(&mut b).set("predelay", predelay);
        let (l, _) = impulse(&mut b, 0.5);
        l.iter().position(|v| *v != 0.0).unwrap()
    };
    assert_eq!(first(100.0) - first(50.0), 2400, "50 ms later");
    let run = |block: usize| {
        let mut b = bench(1);
        b.set("size", 70.0).set("decay", 1.5).set("damp", 4000.0).set("predelay", 12.5).set("width", 60.0);
        let mut all = vec![];
        while all.len() < 24_000 {
            if all.len() == 9472 {
                // 128 × 74 = 37 × 256: both runs reach this sample at the start of a call
                b.set("size", 20.0);
            }
            // a signal of the sample's index, whatever the call
            for c in 0..2 {
                let x: Vec<f32> = (all.len()..all.len() + block).map(|i| ((i * 2_654_435_761 + c * 97) % 1000) as f32 / 1000.0 - 0.5).collect();
                b.rig.set_audio_in(Reverb::IN, c, &x);
            }
            b.run(block);
            all.extend_from_slice(b.rig.audio_out(Reverb::OUT).chan(1));
        }
        all[..24_000].to_vec()
    };
    let (a, b) = (run(128), run(37));
    assert_eq!(a.iter().zip(&b).position(|(x, y)| x != y), None, "the same samples in calls of 128 and of 37 frames");
}
