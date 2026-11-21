//! The script both tests play: three of the eight inputs carry a signal (one comes and goes), levels, balances, sends,
//! mutes and the master move.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
pub use mixer_module::{Mixer, INPUTS};

pub const RATE: f64 = 48_000.0;

/// nothing linked to any input
pub fn bench(seed: u64) -> Bench<Mixer> {
    let mut b = Bench::new(RATE, seed);
    for p in 0..INPUTS {
        b.rig.set_in_silent(p, true);
    }
    b
}

/// a signal on input `p` (0-based), both channels their own
pub fn feed(b: &mut Bench<Mixer>, p: usize, frames: usize, level: f32) {
    b.rig.set_in_silent(p, false);
    b.feed(p, 0, frames, level);
    b.feed(p, 1, frames, level);
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Mixer>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("gain1", -6.0 + (k % 12) as f32).set("pan1", ((k % 21) as f32 - 10.0) / 10.0).set("send1", -30.0 + (k % 30) as f32);
    b.set("mute2", (k / 40 % 2) as f32).set("pan2", 0.5).set("send2", -6.0);
    b.set("gain5", -20.0 + (k % 25) as f32).set("send5", -60.0 + (k / 10 % 7) as f32 * 10.0).set("master", -3.0 + (k % 6) as f32);
    feed(b, 0, n, 0.4);
    feed(b, 1, n, 0.4);
    if k / 30 % 2 == 0 {
        feed(b, 4, n, 0.4);
    } else {
        b.rig.set_in_silent(4, true);
    }
    b.run(n);
    let mut peak = 0.0f32;
    for port in [Mixer::OUT, Mixer::SEND] {
        let out = b.rig.audio_out(port);
        h.f32s(out.chan(0));
        h.f32s(out.chan(1));
        peak = out.chan(0).iter().chain(out.chan(1)).fold(peak, |m, v| m.max(v.abs()));
    }
    peak
}
