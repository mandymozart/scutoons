//! The script both tests play: stereo noise that comes and goes (the tail rings out between), every knob turned, the
//! size changed while the tail sounds; and the measuring helpers.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
pub use reverb_module::Reverb;

pub const RATE: f64 = 48_000.0;

/// a stereo cable in and out
pub fn bench(seed: u64) -> Bench<Reverb> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_channels(true, Reverb::IN, 2);
    b.rig.set_channels(false, Reverb::OUT, 2);
    b
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Reverb>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("size", (k / 60 % 5) as f32 * 25.0).set("decay", 0.3 + (k / 45 % 6) as f32 * 0.8).set("damp", 1000.0 * (1 + k / 30 % 20) as f32);
    b.set("predelay", (k / 25 % 4) as f32 * 30.0).set("width", (k / 70 % 3) as f32 * 50.0).set("mix", 20.0 + (k / 40 % 5) as f32 * 20.0);
    let sounding = k % 100 < 30;
    b.rig.set_in_silent(Reverb::IN, !sounding);
    if sounding {
        b.feed(Reverb::IN, 0, n, 0.5);
        b.feed(Reverb::IN, 1, n, 0.5);
    }
    b.run(n);
    let out = b.rig.audio_out(Reverb::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}

/// the response, both channels, to a click of one sample, `seconds` long, the knobs settled first
pub fn impulse(b: &mut Bench<Reverb>, seconds: f64) -> (Vec<f32>, Vec<f32>) {
    settle(b);
    let (mut l, mut r) = (vec![], vec![]);
    let total = (seconds * RATE) as usize;
    while l.len() < total {
        let mut x = [0.0f32; 128];
        x[0] = if l.is_empty() { 1.0 } else { 0.0 };
        b.rig.set_audio_in(Reverb::IN, 0, &x);
        b.rig.set_audio_in(Reverb::IN, 1, &x);
        b.run(128);
        let out = b.rig.audio_out(Reverb::OUT);
        l.extend_from_slice(out.chan(0));
        r.extend_from_slice(out.chan(1));
    }
    (l, r)
}

/// RT60 in seconds from an impulse response: Schroeder's backward integration of the energy, the line from −5 to
/// −25 dB, extended to 60 dB
pub fn rt60(l: &[f32], r: &[f32]) -> f64 {
    let mut edc: Vec<f64> = l.iter().zip(r).map(|(a, b)| f64::from(a * a + b * b)).collect();
    for i in (0..edc.len() - 1).rev() {
        edc[i] += edc[i + 1];
    }
    let total = edc[0];
    let at = |db: f64| edc.iter().position(|e| 10.0 * (e / total).log10() <= db).expect("the tail falls that far") as f64 / RATE;
    (at(-25.0) - at(-5.0)) * 3.0
}

/// two seconds of silence: the knobs arrive where the test set them
pub fn settle(b: &mut Bench<Reverb>) {
    for _ in 0..(2.0 * RATE / 128.0) as usize {
        b.rig.set_audio_in(Reverb::IN, 0, &[0.0; 128]);
        b.rig.set_audio_in(Reverb::IN, 1, &[0.0; 128]);
        b.run(128);
    }
}
