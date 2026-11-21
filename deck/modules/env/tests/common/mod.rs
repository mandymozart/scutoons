//! The script both tests play: the three modes with both curves, gates held and cut short, retriggers, a reset.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
use deck_module::Ev;
pub use env_module::Env;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Env> {
    Bench::new(RATE, seed)
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Env>, k: usize, h: &mut Hash) -> f32 {
    let n = 32 + (k % 4) * 32;
    b.set("mode", (k / 60 % 3) as f32).set("curve", (k / 90 % 2) as f32).set("velocity", (k / 45 % 3) as f32 * 50.0);
    b.set("a", 0.002 + 0.001 * (k % 5) as f32).set("d", 0.01 + 0.002 * (k % 3) as f32).set("s", 40.0 + (k % 3) as f32 * 20.0).set("r", 0.02);
    let mut events = vec![];
    match k % 12 {
        0 => events.push(Ev::note_on(9, k as u32, 220.0, 57.0, 0.25 + 0.25 * (k % 4) as f32)),
        2 if k % 24 == 2 => events.push(Ev::note_on(0, k as u32, 220.0, 57.0, 1.0)),
        7 => events.push(Ev::note_off(n as u32 - 1, if k % 24 == 7 { k as u32 - 7 } else { k as u32 - 5 })),
        _ => {}
    }
    if k % 131 == 130 {
        events.push(Ev::reset(3));
    }
    b.rig.set_events_in(Env::GATE, &events);
    b.run(n);
    let out = b.rig.audio_out(Env::OUT);
    h.f32s(out.chan(0));
    h.f32s(&[b.rig.control_out(Env::ENV).0]);
    out.chan(0).iter().fold(0.0, |m, v| m.max(v.abs()))
}
