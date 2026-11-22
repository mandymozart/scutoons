//! The script both tests play: keys pressed and let go, chords, a reset, while the knobs move, at 240 bpm.
#![allow(dead_code)]
pub use arp_module::Arp;
pub use deck_module::golden::{Bench, Hash};
use deck_module::Ev;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Arp> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_tempo(0.0, 0.0, 240.0, 4, true);
    b
}

/// what the module wrote, into the hash (an absent value as that, not as its NaN's bits)
pub fn hash(h: &mut Hash, events: &[Ev]) {
    for e in events {
        h.u32s(&[e.frame, u32::from(e.kind), e.id]);
        h.f32s(&[e.len]);
        for v in e.v {
            h.u32s(&[u32::from(v.is_nan())]);
            h.f32s(&[if v.is_nan() { 0.0 } else { v }]);
        }
    }
}

/// step `k` of the script; how many events left the module
pub fn step(b: &mut Bench<Arp>, k: usize, h: &mut Hash) -> f32 {
    let n = 32 + (k % 4) * 32;
    if k % 50 == 0 {
        b.set("mode", (k / 50 % 4) as f32).set("rate", (4 + k / 100 % 5) as f32).set("octaves", (1 + k / 150 % 4) as f32);
        b.set("transpose", (k / 70 % 7) as f32 * 3.0 - 9.0).set("latch", (k / 400 % 2) as f32).set("gate", 20.0 + (k / 90 % 5) as f32 * 20.0);
    }
    let id = (k / 6) as u32;
    let events = match k % 24 {
        0 | 6 | 12 => vec![Ev::note_on(5, id, f32::NAN, 48.0 + (k % 19) as f32, 0.6)],
        3 => vec![Ev::note_on(1, id + 1000, 220.0 * (1 + k % 3) as f32, f32::NAN, 1.0), Ev::note_on(2, id + 2000, f32::NAN, 55.0, 0.9)],
        9 | 15 => vec![Ev::note_off(n as u32 - 1, id - 1)],
        18 => vec![Ev::note_off(0, id + 1000 - 3), Ev::note_off(4, id + 2000 - 3)],
        23 if k % 240 == 239 => vec![Ev::reset(7)],
        21 => vec![Ev::note_off(3, id - 3)],
        _ => vec![],
    };
    b.rig.set_events_in(Arp::NOTES, &events);
    b.run(n);
    let out = b.rig.events_out(Arp::OUT);
    hash(h, &out);
    out.len() as f32
}
