//! The script both tests play: notes on and off, bends, a reset, a note that carries only its key, while the knobs move.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
use deck_module::{Ev, Expr};
pub use transpose_module::Transpose;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Transpose> {
    Bench::new(RATE, seed)
}

/// what the module wrote, into the hash (an absent value as that, not as its NaN's bits)
pub fn hash(h: &mut Hash, events: &[Ev]) {
    for e in events {
        h.u32s(&[e.frame, u32::from(e.kind), e.id]);
        for v in e.v {
            h.u32s(&[u32::from(v.is_nan())]);
            h.f32s(&[if v.is_nan() { 0.0 } else { v }]);
        }
    }
}

/// step `k` of the script; how many events left the module
pub fn step(b: &mut Bench<Transpose>, k: usize, h: &mut Hash) -> f32 {
    let n = 32 + (k % 4) * 32;
    b.set("semi", (k / 7 % 25) as f32 - 12.0).set("fine", (k / 3 % 9) as f32 * 12.5 - 50.0);
    let id = (k / 4) as u32;
    let events = match k % 4 {
        0 => vec![Ev::note_on(3, id, 110.0 * (1 + k % 6) as f32, 45.0 + (k % 24) as f32, 0.5)],
        1 => vec![Ev::note_expr(0, id, Expr::Bend, 0.5), Ev::note_on(n as u32 - 1, id + 1000, f32::NAN, 60.0, 1.0)],
        2 => vec![Ev::note_off(7, id), Ev::note_off(9, id + 1000)],
        _ if k % 16 == 15 => vec![Ev::reset(0)],
        _ => vec![],
    };
    b.rig.set_events_in(Transpose::NOTES, &events);
    b.run(n);
    let out = b.rig.events_out(Transpose::OUT);
    hash(h, &out);
    out.len() as f32
}
