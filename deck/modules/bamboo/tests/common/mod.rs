//! The script both tests play: the grove through its counts, heights, thicknesses, hues, spreads, winds, gusts,
//! directions, roots, scales and debounces, in calls of changing length; each call's table, notes and control outputs
//! hashed.
#![allow(dead_code)]
pub use bamboo_module::Bamboo;
pub use deck_module::golden::{Bench, Hash};
use deck_module::{Ev, Instance, Kind};

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Bamboo> {
    Bench::new(RATE, seed)
}

pub fn hash_table(h: &mut Hash, rows: &[Instance]) {
    for i in rows {
        h.u32s(&[i.shape]);
        h.f32s(&i.position);
        h.f32s(&i.rotation);
        h.f32s(&i.scale);
        h.f32s(&i.colour);
    }
}

/// step `k` of the script; the table it left
pub fn step(b: &mut Bench<Bamboo>, k: usize, h: &mut Hash) -> Vec<Instance> {
    let n = 16 + (k % 8) * 16;
    b.set("thin", [1.0, 0.0, 4.0, 2.5][k / 60 % 4]).set("thick", [1.0, 3.0, 0.0, 4.0][k / 80 % 4]);
    b.set("thin_height", [7.0, 2.0, 12.0][k / 90 % 3]).set("thick_height", [4.0, 8.0, 1.0][k / 110 % 3]);
    b.set("thin_width", [0.06, 0.2][k / 130 % 2]).set("thick_width", [0.14, 0.05, 0.4][k / 70 % 3]);
    b.set("thin_hue", (k % 11) as f32 / 10.0).set("thick_hue", (k % 7) as f32 / 6.0).set("spread", [6.0, 1.0, 30.0, 3.0][k / 100 % 4]);
    b.set("wind", [0.6, 1.0, 0.0, 0.3][k / 50 % 4]).set("gust", [0.5, 1.0, 0.0][k / 40 % 3]).set("direction", [0.0, 90.0, 225.0][k / 120 % 3]);
    b.set("root", [60.0, 24.0, 96.0][k / 150 % 3]).set("scale", (k / 75 % 4) as f32).set("debounce", [0.12, 0.02, 1.0][k / 140 % 3]);
    b.run(n);
    let rows = b.rig.instances();
    hash_table(h, &rows);
    for e in b.rig.events_out(Bamboo::HITS) {
        h.u32s(&[e.frame, e.id, u32::from(e.kind)]);
        h.f32s(&[e.key(), e.velocity(), e.len]);
    }
    for i in Bamboo::HIT_RATE..=Bamboo::SWAY {
        h.f32s(&[b.rig.control_out(i).0]);
    }
    rows
}

/// A hit heard: its absolute frame, the pair (`a × 80 + b`), key, velocity
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub at: u64,
    pub pair: u32,
    pub key: f32,
    pub velocity: f32,
}

/// `frames` of the grove in calls of the sizes `cut` cycles through: the hits, the table at the end
pub fn play(b: &mut Bench<Bamboo>, frames: usize, cut: &[usize]) -> (Vec<Hit>, Vec<Instance>) {
    let mut hits = vec![];
    let end = b.frame + frames as u64;
    let mut k = 0;
    while b.frame < end {
        let n = cut[k % cut.len()].min((end - b.frame) as usize);
        let start = b.frame;
        b.run(n);
        let out = b.rig.events_out(Bamboo::HITS);
        hits.extend(out.iter().filter(|e: &&Ev| e.kind() == Some(Kind::NoteOn)).map(|e| Hit { at: start + u64::from(e.frame), pair: e.id >> 16, key: e.key(), velocity: e.velocity() }));
        k += 1;
    }
    (hits, b.rig.instances())
}
