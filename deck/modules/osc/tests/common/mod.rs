//! The script both tests play: every wave, one to seven voices, notes and bends, the pitch input, noise into fm.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
use deck_module::{Ev, Expr};
pub use osc_module::Osc;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Osc> {
    Bench::new(RATE, seed)
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Osc>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("wave", (k / 40 % 4) as f32).set("voices", (1 + k / 55 % 7) as f32).set("detune", (k % 9) as f32 * 5.0);
    b.set("semi", (k / 13 % 5) as f32 - 2.0).set("fine", (k % 21) as f32 - 10.0).set("pw", 20.0 + (k % 60) as f32);
    let mut events = vec![];
    match k % 8 {
        0 => events.push(Ev::note_on(7, k as u32, 55.0 * (1 + k % 7) as f32, f32::NAN, 1.0)),
        3 => events.push(Ev::note_expr(n as u32 / 2, k as u32 - 3, Expr::Bend, (k % 5) as f32 - 2.0)),
        _ => {}
    }
    b.rig.set_events_in(Osc::NOTES, &events);
    b.rig.set_control_in(Osc::PITCH, ((k % 11) as f32 - 5.0) * 0.1);
    b.feed(Osc::FM, 0, n, if k % 3 == 0 { 0.05 } else { 0.0 });
    b.run(n);
    let out = b.rig.audio_out(Osc::OUT);
    h.f32s(out.chan(0));
    out.chan(0).iter().fold(0.0, |m, v| m.max(v.abs()))
}
