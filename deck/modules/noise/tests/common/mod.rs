//! The script both tests play: both colours, free and gated, gates of every velocity, a reset, the level turned.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
use deck_module::Ev;
pub use noise_module::Noise;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Noise> {
    Bench::new(RATE, seed)
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Noise>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("color", (k / 50 % 2) as f32).set("mode", (k / 120 % 2) as f32).set("level", -24.0 + (k % 30) as f32);
    let mut events = vec![];
    match k % 10 {
        0 => events.push(Ev::note_on(5, k as u32, 220.0, 57.0, 0.3 + 0.1 * (k % 7) as f32)),
        6 => events.push(Ev::note_off(n as u32 - 1, k as u32 - 6)),
        _ => {}
    }
    if k % 97 == 96 {
        events.push(Ev::reset(0));
    }
    b.rig.set_events_in(Noise::GATE, &events);
    b.run(n);
    let out = b.rig.audio_out(Noise::OUT);
    h.f32s(out.chan(0));
    out.chan(0).iter().fold(0.0, |m, v| m.max(v.abs()))
}
