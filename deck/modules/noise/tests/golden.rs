//! The golden render: the script's noise, hashed. A change of the hash is a change of the sound.
mod common;
use common::*;
use deck_module::Ev;

const GOLDEN: u64 = 0xbe06_6c61_9be7_ef65;

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// `calls` calls of 128 frames, every sample
fn render(b: &mut Bench<Noise>, calls: usize) -> Vec<f32> {
    let mut all = vec![];
    for _ in 0..calls {
        b.run(128);
        all.extend_from_slice(b.rig.audio_out(Noise::OUT).chan(0));
    }
    all
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..600).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.05 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn the_seed_is_the_noise_and_the_level_is_decibels() {
    let (mut a, mut b, mut c) = (bench(5), bench(5), bench(6));
    for x in [&mut a, &mut b, &mut c] {
        x.set("level", 0.0);
    }
    let (xa, xb, xc) = (render(&mut a, 40), render(&mut b, 40), render(&mut c, 40));
    assert!(xa == xb && xa != xc);
    // white noise in (−1, 1): rms 1/√3
    assert!((rms(&xa) - 0.577).abs() < 0.02, "{}", rms(&xa));
    a.set("level", -20.0);
    render(&mut a, 10);
    let quiet = rms(&render(&mut a, 40));
    assert!((quiet - 0.0577).abs() < 0.003, "{quiet}");
    a.set("level", -60.0);
    render(&mut a, 20);
    assert!(rms(&render(&mut a, 10)) < 1e-6, "the bottom is silence");
    // pink is about as loud and darker: less of it changes from sample to sample
    b.set("color", 1.0);
    let pink = render(&mut b, 100);
    let step = |x: &[f32]| rms(&x.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()) / rms(x);
    assert!((rms(&pink) - 0.5).abs() < 0.12 && step(&pink) < 0.6 * step(&xa), "rms {}, {} against {}", rms(&pink), step(&pink), step(&xa));
}

#[test]
fn gated_it_sounds_only_while_a_note_is_held() {
    let mut b = bench(1);
    b.rig.apply_preset();
    // the preset came after the start: the free noise fades out, then it is closed until a gate comes
    render(&mut b, 20);
    assert!(rms(&render(&mut b, 20)) == 0.0);
    b.rig.set_events_in(Noise::GATE, &[Ev::note_on(64, 1, 440.0, 69.0, 0.5)]);
    b.run(128);
    let first = b.rig.audio_out(Noise::OUT).chan(0).to_vec();
    assert!(first[..64].iter().all(|v| *v == 0.0) && first[64..].iter().any(|v| *v != 0.0), "it opens at the note's frame");
    let half = rms(&render(&mut b, 40));
    // −6 dB × velocity 0.5 × white's 0.577
    assert!((half - 0.577 * 0.501 * 0.5).abs() < 0.01, "{half}");
    // another note takes the gate; the first one's off no longer closes it
    b.rig.set_events_in(Noise::GATE, &[Ev::note_on(0, 2, 440.0, 69.0, 1.0), Ev::note_off(1, 1)]);
    render(&mut b, 10);
    assert!(rms(&render(&mut b, 20)) > 1.8 * half);
    b.rig.set_events_in(Noise::GATE, &[Ev::note_off(0, 2)]);
    render(&mut b, 20);
    assert!(rms(&render(&mut b, 10)) == 0.0, "closed after the off");
}
