//! The script both tests play: the cloud through its counts, sizes, winds (both ways, still), heights, ranges, colours,
//! rain, splits and swells, the storm's repeater held and let go, synced and free, at its rates; in calls of changing
//! length; each call's table, notes and control outputs hashed.
#![allow(dead_code)]
pub use cloud_module::Cloud;
pub use deck_module::golden::{Bench, Hash};
use deck_module::Instance;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<Cloud> {
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
pub fn step(b: &mut Bench<Cloud>, k: usize, h: &mut Hash) -> Vec<Instance> {
    let n = 16 + (k % 8) * 16;
    b.set("puffs", [9.0, 3.0, 24.0, 14.0][k / 50 % 4]).set("size", [3.0, 0.5, 12.0][k / 70 % 3]);
    b.set("drift", [0.4, -3.0, 0.0, 5.0][k / 40 % 4]).set("height", [4.0, 0.0, 30.0][k / 90 % 3]);
    b.set("range", [16.0, 2.0, 120.0][k / 110 % 3]).set("hue", (k % 11) as f32 / 10.0).set("tint", (k / 20 % 5) as f32 * 25.0).set("light", (k % 5) as f32 * 25.0);
    b.set("rain", [0.0, 0.5, 1.0][k / 60 % 3]).set("split", [0.0, 1.0, 0.3][k / 130 % 3]).set("swell", [0.0, 1.0][k / 150 % 2]);
    b.set("hold", [0.0, 1.0, 1.0][k / 30 % 3]).set("sync", [1.0, 0.0][k / 120 % 2]).set("rate", (k / 25 % 9) as f32 / 8.0);
    b.set("flash", [0.5, 1.0, 0.0][k / 80 % 3]).set("rumble", [0.8, 0.0, 1.0][k / 100 % 3]).set("thump", [1.0, 0.4][k / 70 % 2]);
    b.run(n);
    let rows = b.rig.instances();
    hash_table(h, &rows);
    for port in [Cloud::THUNDER, Cloud::KICK] {
        for e in b.rig.events_out(port) {
            h.u32s(&[e.frame, e.id, u32::from(e.kind)]);
            h.f32s(&[e.key(), e.velocity(), e.len]);
        }
    }
    for i in Cloud::PERIOD..=Cloud::BLUE {
        h.f32s(&[b.rig.control_out(i).0]);
    }
    rows
}
