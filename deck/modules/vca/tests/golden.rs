//! The golden render: the script through the vca, hashed. A change of the hash is a change of the sound.
mod common;
use common::*;

const GOLDEN: u64 = 0x463a_379c_6636_6301;

/// one call of 128 frames: the signal on both channels, `cv` on the audio cv, `control` on the control cv; (in, out left, out right)
fn call(b: &mut Bench<Vca>, cv: f32, control: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let x: Vec<f32> = (0..128).map(|_| b.signal.sample()).collect();
    b.rig.set_audio_in(Vca::IN, 0, &x);
    b.rig.set_audio_in(Vca::IN, 1, &x);
    b.rig.set_audio_in(Vca::CV, 0, &[cv; 128]);
    b.rig.set_control_in(Vca::CV_CONTROL, control);
    b.run(128);
    let out = b.rig.audio_out(Vca::OUT);
    (x, out.chan(0).to_vec(), out.chan(1).to_vec())
}

fn settle(b: &mut Bench<Vca>, cv: f32, control: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    for _ in 0..40 {
        call(b, cv, control);
    }
    call(b, cv, control)
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..600).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn without_cv_amount_it_is_a_plain_gain() {
    let mut b = bench(1);
    let (x, l, r) = call(&mut b, 0.0, 0.0);
    assert!(x == l && x == r, "0 dB passes the signal bit for bit, whatever the cv");
    b.set("gain", -20.0);
    let (x, l, _) = settle(&mut b, 0.7, 0.2);
    assert!(x.iter().zip(&l).all(|(x, y)| (x * 0.1 - y).abs() < 1e-6));
    b.set("gain", -60.0);
    assert!(settle(&mut b, 1.0, 1.0).1.iter().all(|v| v.abs() < 1e-7), "the bottom is silence");
}

#[test]
fn the_cv_opens_it_as_far_as_amount_says() {
    let mut b = bench(1);
    b.rig.apply_preset();
    assert!(settle(&mut b, 0.0, 0.0).1.iter().all(|v| v.abs() < 1e-7), "amount 100 %: nothing at cv, nothing out");
    let (x, l, _) = settle(&mut b, 0.5, 0.25);
    assert!(x.iter().zip(&l).all(|(x, y)| (x * 0.75 - y).abs() < 1e-6), "audio cv + control cv");
    assert!(settle(&mut b, -0.5, 0.0).1.iter().all(|v| v.abs() < 1e-7), "a cv below zero closes it, no phase flip");
    b.set("amount", 50.0);
    let (x, l, _) = settle(&mut b, 0.0, 0.0);
    assert!(x.iter().zip(&l).all(|(x, y)| (x * 0.5 - y).abs() < 1e-6), "half-way: half the gain stays open");
}

#[test]
fn a_step_of_the_gain_glides_and_a_mono_signal_reaches_both_channels() {
    let mut b = bench(1);
    b.rig.set_channels(true, Vca::IN, 1);
    b.rig.set_audio_in(Vca::IN, 0, &[1.0; 128]);
    b.set("gain", -60.0);
    b.run(128);
    let out = b.rig.audio_out(Vca::OUT);
    assert!(out.chan(0) == out.chan(1));
    let l = out.chan(0);
    assert!(l[0] > 0.98 && l.windows(2).all(|w| w[1] < w[0]) && l[127] < 0.3, "from {} down to {}", l[0], l[127]);
}
