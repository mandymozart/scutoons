//! The script both tests play: a stereo signal, a cv ramp as audio, a stepping control cv, gain and amount turned.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
pub use vca_module::Vca;

pub const RATE: f64 = 48_000.0;

/// a stereo cable in and out
pub fn bench(seed: u64) -> Bench<Vca> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_channels(true, Vca::IN, 2);
    b.rig.set_channels(false, Vca::OUT, 2);
    b
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Vca>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("gain", -12.0 + (k % 20) as f32).set("amount", (k / 25 % 5) as f32 * 25.0);
    b.feed(Vca::IN, 0, n, 0.5);
    b.feed(Vca::IN, 1, n, 0.5);
    let mut cv = [0.0f32; 128];
    for (i, v) in cv.iter_mut().enumerate() {
        *v = (k % 5) as f32 / 4.0 * i as f32 / 128.0 - if k % 9 == 8 { 0.5 } else { 0.0 };
    }
    b.rig.set_audio_in(Vca::CV, 0, &cv[..n]);
    b.rig.set_control_in(Vca::CV_CONTROL, (k % 3) as f32 * 0.25);
    b.run(n);
    let out = b.rig.audio_out(Vca::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}
