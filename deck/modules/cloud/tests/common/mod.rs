//! The script both tests play: the cloud through its counts, sizes, winds (both ways, still), heights, ranges, colours,
//! rain, splits and swells, in calls of changing length; each call's table hashed.
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
    b.run(n);
    let rows = b.rig.instances();
    hash_table(h, &rows);
    rows
}
