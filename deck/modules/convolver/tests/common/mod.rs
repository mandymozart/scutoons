//! The script both tests play: stereo noise that comes and goes (the tail rings out between), every knob turned, through
//! a stereo response loaded as the asset; and the helpers: a response, a direct convolution to measure against.
#![allow(dead_code, unused_imports)]
pub use convolver_module::{prepare, Convolver, PART};
pub use deck_module::golden::{Bench, Hash};
use deck_module::dsp::noise::White;
use deck_module::AssetKind;

pub const RATE: f64 = 48_000.0;

/// a stereo response of `seconds`: seeded noise, a side each, falling 60 dB in `rt60` (a first tap of 1 at sample 0)
pub fn response(seconds: f64, rt60: f64, rate: f64) -> [Vec<f32>; 2] {
    let n = (seconds * rate) as usize;
    let k = (-3.0 * std::f64::consts::LN_10 / (rt60 * rate)).exp() as f32;
    [3u64, 4].map(|seed| {
        let (mut w, mut env) = (White::new(seed), 1.0f32);
        let mut v: Vec<f32> = (0..n)
            .map(|_| {
                let s = w.sample() * env;
                env *= k;
                s
            })
            .collect();
        v[0] = 1.0;
        v
    })
}

/// the convolver with `ir` (planar sides at `rate`) in its asset slot, a stereo cable in and out
pub fn bench_ir(seed: u64, ir: &[Vec<f32>], rate: f32) -> Bench<Convolver> {
    let frames = ir[0].len();
    let bytes: Vec<u8> = ir.iter().flat_map(|c| c.iter().flat_map(|v| v.to_le_bytes())).collect();
    let mut b = Bench::with(RATE, seed, |rig| rig.load_asset(Convolver::IR, AssetKind::Pcm, &bytes, ir.len() as u8, frames as u32, rate));
    b.rig.set_channels(true, Convolver::IN, 2);
    b.rig.set_channels(false, Convolver::OUT, 2);
    b
}

/// the convolver with its built-in room
pub fn bench(seed: u64) -> Bench<Convolver> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_channels(true, Convolver::IN, 2);
    b.rig.set_channels(false, Convolver::OUT, 2);
    b
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Convolver>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("mix", 20.0 + (k / 40 % 5) as f32 * 20.0).set("gain", -12.0 + (k / 35 % 4) as f32 * 6.0);
    b.set("predelay", (k / 25 % 4) as f32 * 30.0).set("length", 0.1 + (k / 50 % 5) as f32 * 0.2);
    let sounding = k % 100 < 30;
    b.rig.set_in_silent(Convolver::IN, !sounding);
    if sounding {
        b.feed(Convolver::IN, 0, n, 0.5);
        b.feed(Convolver::IN, 1, n, 0.5);
    }
    b.run(n);
    let out = b.rig.audio_out(Convolver::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}

/// `x` (two sides) through the bench in calls of `block` frames; the two outputs
pub fn play(b: &mut Bench<Convolver>, x: &[Vec<f32>; 2], block: usize) -> [Vec<f32>; 2] {
    let mut out = [Vec::with_capacity(x[0].len()), Vec::with_capacity(x[0].len())];
    let mut at = 0;
    while at < x[0].len() {
        let n = block.min(x[0].len() - at);
        for c in 0..2 {
            b.rig.set_audio_in(Convolver::IN, c, &x[c][at..at + n]);
        }
        b.run(n);
        let o = b.rig.audio_out(Convolver::OUT);
        for (c, side) in out.iter_mut().enumerate() {
            side.extend_from_slice(o.chan(c));
        }
        at += n;
    }
    out
}

/// the convolution of `x` with `h`, in f64, the length of `x`
pub fn direct(x: &[f32], h: &[f32]) -> Vec<f64> {
    (0..x.len()).map(|n| (0..h.len().min(n + 1)).map(|k| f64::from(x[n - k]) * f64::from(h[k])).sum()).collect()
}

/// the error's energy against the signal's, dB
pub fn error_db(got: &[f32], want: &[f64]) -> f64 {
    let err: f64 = got.iter().zip(want).map(|(a, b)| (f64::from(*a) - b).powi(2)).sum();
    let sig: f64 = want.iter().map(|b| b * b).sum();
    10.0 * (err / sig).log10()
}

/// a second of silence in calls of 128: the knobs arrive where the test set them
pub fn settle(b: &mut Bench<Convolver>) {
    for _ in 0..(RATE / 128.0) as usize {
        b.rig.set_audio_in(Convolver::IN, 0, &[0.0; 128]);
        b.rig.set_audio_in(Convolver::IN, 1, &[0.0; 128]);
        b.run(128);
    }
}
