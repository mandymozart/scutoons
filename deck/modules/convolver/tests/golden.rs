//! The golden render: the script through the convolver (a stereo response at 44.1 kHz, resampled), hashed. A change of
//! the hash is a change of the sound. And what the convolver promises: the direct convolution within −90 dB, exactly
//! 128 frames of latency (declared and measured), the same samples however the blocks are cut, predelay, length, gain,
//! the built-in room, a tail that ends in silence.
mod common;
use common::*;
use deck_module::Describe;

const GOLDEN: u64 = 0x6d85_6364_1aa6_5de0;

/// the response alone, with `more` set on top, the knobs arrived
fn wet(b: &mut Bench<Convolver>, more: &[(&str, f32)]) {
    b.set("mix", 100.0).set("gain", 0.0).set("predelay", 0.0).set("length", 4.0);
    for (name, v) in more {
        b.set(name, *v);
    }
    settle(b);
}

fn noise(seconds: f64, seed: u64) -> [Vec<f32>; 2] {
    let mut w = deck_module::dsp::noise::White::new(seed);
    let n = (seconds * RATE) as usize;
    [(0..n).map(|_| w.sample() * 0.5).collect(), (0..n).map(|_| w.sample() * 0.5).collect()]
}

fn click(len: usize) -> [Vec<f32>; 2] {
    let mut x = vec![0.0f32; len];
    x[0] = 1.0;
    [x.clone(), x]
}

/// the first sample above −120 dB (the transforms leave their rounding, ~1e-9, around a block's sound)
fn first(x: &[f32]) -> Option<usize> {
    x.iter().position(|v| v.abs() > 1e-6)
}

fn last(x: &[f32]) -> Option<usize> {
    x.iter().rposition(|v| v.abs() > 1e-6)
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench_ir(1, &response(0.6, 0.4, 44_100.0), 44_100.0);
    let mut h = Hash::new();
    let peak = (0..900).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.05 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn the_output_is_the_direct_convolution_within_90_db() {
    let stereo = response(0.3, 0.2, RATE);
    let mono = vec![stereo[0].clone()];
    for ir in [stereo.to_vec(), mono] {
        let refs: Vec<&[f32]> = ir.iter().map(|c| c.as_slice()).collect();
        let h = prepare(&refs, RATE, RATE);
        let mut b = bench_ir(1, &ir, RATE as f32);
        wet(&mut b, &[]);
        let x = noise(0.6, 9);
        let y = play(&mut b, &x, 128);
        let n = x[0].len() - PART;
        for c in 0..2 {
            let want = direct(&x[c][..n], &h[c]);
            let db = error_db(&y[c][PART..], &want);
            assert!(db < -90.0, "{} side {c}: {db:.1} dB", ir.len());
        }
        if ir.len() == 1 {
            assert_eq!(h[0], h[1], "a mono response plays on both sides");
        }
    }
}

#[test]
fn the_latency_is_128_frames_declared_and_measured() {
    let d = deck_module::host::parse(Convolver::DESCRIPTOR).unwrap();
    assert_eq!(d.latency, 128);
    // the response alone: its first tap 128 frames after the click
    let mut b = bench_ir(1, &response(0.1, 0.05, RATE), RATE as f32);
    wet(&mut b, &[]);
    let y = play(&mut b, &click(4096), 128);
    assert_eq!((first(&y[0]), first(&y[1])), (Some(128), Some(128)));
    // the dry alone: the input itself, 128 frames late, in calls of any length
    let mut b = bench(1);
    b.set("mix", 0.0);
    settle(&mut b);
    let x = noise(0.1, 2);
    let y = play(&mut b, &x, 37);
    for c in 0..2 {
        assert_eq!(&y[c][..PART], &[0.0; PART][..]);
        assert_eq!(&y[c][PART..], &x[c][..x[c].len() - PART], "side {c}");
    }
}

#[test]
fn the_cut_of_the_blocks_changes_nothing() {
    let run = |block: usize| {
        let mut b = bench_ir(1, &response(0.2, 0.15, 44_100.0), 44_100.0);
        b.set("mix", 60.0).set("gain", -3.0).set("predelay", 12.5).set("length", 0.15);
        settle(&mut b);
        play(&mut b, &noise(0.5, 4), block)
    };
    let a = run(128);
    for block in [37, 1, 100] {
        assert!(run(block) == a, "calls of {block} frames against 128");
    }
}

#[test]
fn predelay_length_and_gain_do_what_they_say() {
    let ir = response(0.5, 0.4, RATE);
    let at = |predelay: f32| {
        let mut b = bench_ir(1, &ir, RATE as f32);
        wet(&mut b, &[("predelay", predelay)]);
        first(&play(&mut b, &click(8192), 128)[0]).unwrap()
    };
    assert_eq!(at(50.0) - at(0.0), 2400, "50 ms later");
    // length 0.1 s (4800 frames): the first 2048 and three partitions of 1024 of the tail sound, nothing after them
    let mut b = bench_ir(1, &ir, RATE as f32);
    wet(&mut b, &[("length", 0.1)]);
    let y = play(&mut b, &click(24_000), 128);
    let last = last(&y[0]).unwrap();
    assert!(last < PART + 5 * 1024 && last > PART + 4 * 1024, "the last sample at {last}");
    // +6 dB: the response twice as loud
    let tap = |gain: f32| {
        let mut b = bench_ir(1, &ir, RATE as f32);
        wet(&mut b, &[("gain", gain)]);
        play(&mut b, &click(1024), 128)[0][PART]
    };
    assert!((tap(6.0) / tap(0.0) - 1.995).abs() < 1e-3, "{}", tap(6.0) / tap(0.0));
}

#[test]
fn the_response_is_resampled_cut_and_normalised() {
    let ir = response(5.0, 1.0, 44_100.0);
    let h = prepare(&[&ir[0], &ir[1]], 44_100.0, RATE);
    assert_eq!(h[0].len(), 4 * 48_000, "cut to 4 s");
    let short = response(0.5, 0.3, 44_100.0);
    let h = prepare(&[&short[0], &short[1]], 44_100.0, RATE);
    assert!((h[0].len() as f64 - 0.5 * RATE).abs() < 2.0, "{}", h[0].len());
    let energy = |s: &[f32]| s.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    let louder = energy(&h[0]).max(energy(&h[1]));
    assert!((louder - 1.0).abs() < 1e-4, "{louder}");
    assert!(prepare(&[&[0.0; 64]], RATE, RATE)[1].iter().all(|v| *v == 0.0), "a silent response stays silent");
}

#[test]
fn without_an_asset_the_built_in_room_plays_and_its_tail_ends_in_silence() {
    let mut b = bench(1);
    wet(&mut b, &[]);
    let y = play(&mut b, &click(3 * 48_000), 128);
    assert_eq!(first(&y[0]), Some(PART));
    assert!(y[0] != y[1], "two sides apart");
    let last = y.iter().map(|side| last(side).unwrap()).max().unwrap();
    assert!(last <= PART + 48_000, "the room lasts a second: {last}");
    // nothing linked and the tail run out: the output is marked silent
    b.rig.set_in_silent(Convolver::IN, true);
    b.run(128);
    assert_ne!(b.rig.out_hdr().out_silent & 1, 0);
}
