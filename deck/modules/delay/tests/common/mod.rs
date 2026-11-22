//! The script both tests play: bursts of stereo noise with silences between them (so the tails are heard), through
//! free and synced times, the control input, feedback up to the top, damping, ping-pong, spread and the modulation.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
pub use delay_module::Delay;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Delay> {
    Bench::new(RATE, seed)
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Delay>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("sync", [0.0, 3.0, 8.0, 6.0, 0.0, 14.0][k / 60 % 6]).set("time", [7.0, 40.0, 125.0, 300.0][k / 25 % 4]);
    b.set("feedback", (k / 30 % 5) as f32 * 25.0).set("damp", [20_000.0, 6000.0, 800.0][k / 45 % 3]);
    b.set("pingpong", (k / 70 % 3) as f32 * 50.0).set("spread", (k / 90 % 2) as f32 * 12.0).set("mix", (k % 5) as f32 * 25.0);
    b.set("depth", [0.0, 2.0, 15.0][k / 55 % 3]).set("rate", [0.3, 1.5, 7.0][k / 33 % 3]);
    let level = if k % 40 < 6 { 0.5 } else { 0.0 };
    b.feed(Delay::IN, 0, n, level);
    b.feed(Delay::IN, 1, n, level);
    b.rig.set_control_in(Delay::TIME_MOD, ((k % 7) as f32 - 3.0) * 0.1);
    b.run(n);
    let out = b.rig.audio_out(Delay::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}
