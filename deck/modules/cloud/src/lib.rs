//! `cloud` – a low-poly cloud floating above its place: a lumpy cluster of puffs (the core's placeholder shape until the
//! owner's models come) that drifts with the wind, breathes a little and wraps back when it has drifted past its
//! range. Its layout is the node's `seed`; every step rewrites its instance table, so the table is the same on every deck.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p cloud-module`.
#![allow(clippy::duplicated_attributes)]
use deck_module::instances::shape;
use deck_module::{math, Ctx, Instance, Module, Outputs};

/// the most puffs (the table's capacity)
pub const MOST: usize = 24;

#[derive(Module)]
#[module(name = "cloud", domain = "control", doc = "a low-poly cloud of puffs floating above its place, drifting with the wind")]
#[port(output, instances, name = "shapes", capacity = 24, doc = "the puffs, drawn at the node's place")]
#[preset(name = "storm", doc = "a dark, wide cloud in a strong wind", values(puffs = 18.0, size = 6.0, drift = 2.5, light = 35.0))]
pub struct Cloud {
    #[param(range = 3.0..24.0, default = 9.0, doc = "how many puffs")]
    puffs: f32,
    #[param(range = 0.5..20.0, default = 3.0, unit = "m", doc = "how wide the cloud is")]
    size: f32,
    #[param(range = -5.0..5.0, default = 0.4, doc = "the wind: metres a second along the stage's x")]
    drift: f32,
    #[param(range = 0.0..50.0, default = 4.0, unit = "m", doc = "how high above its place it floats")]
    height: f32,
    #[param(range = 2.0..200.0, default = 16.0, unit = "m", doc = "how far it drifts before it wraps back")]
    range: f32,
    #[param(range = 0.0..1.0, default = 0.6, doc = "its colour's hue")]
    hue: f32,
    #[param(range = 0.0..100.0, default = 6.0, unit = "%", doc = "how much colour")]
    tint: f32,
    #[param(range = 0.0..100.0, default = 92.0, unit = "%", doc = "how bright")]
    light: f32,
    /// metres drifted, inside the range
    drifted: f64,
    /// seconds since the start (the breathing)
    time: f64,
}

/// a number in [0, 1) from the seed and two indices – the same on every platform
fn unit(seed: u64, i: u64, k: u64) -> f32 {
    let mut z = seed ^ i.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ k.wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// hue, saturation, value (0 … 1) → linear RGB
fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let k = |n: f32| {
        let k = math::fmod(n + h * 6.0, 6.0);
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [k(5.0), k(3.0), k(1.0)].map(|c| math::pow(c, 2.2))
}

impl Module for Cloud {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let dt = ctx.frames() as f64 / ctx.rate().max(1.0);
        self.time += dt;
        let range = f64::from(self.range.max(2.0));
        self.drifted = math::libm::fmod(self.drifted + f64::from(self.drift) * dt, range);
        if self.drifted < 0.0 {
            self.drifted += range;
        }
        // the cluster's middle runs from one end of the range to the other
        let x = (self.drifted - range * 0.5) as f32;
        let n = (math::round(self.puffs) as usize).clamp(3, MOST);
        let size = self.size.max(0.1);
        let seed = ctx.seed();
        let base = hsv(self.hue, self.tint * 0.01, self.light * 0.01);
        let t = self.time as f32;
        let mut table = out.instances();
        table.clear();
        for i in 0..n as u64 {
            let r = |k: u64| unit(seed, i, k);
            // across the cloud, a flat bottom and a heaped middle
            let across = r(0) - 0.5;
            let middle = 1.0 - 2.0 * across.abs();
            let at = [x + across * size, self.height + r(1) * size * 0.3 * (0.4 + 0.6 * middle), (r(2) - 0.5) * size * 0.45];
            let breath = 1.0 + 0.05 * math::sin(core::f32::consts::TAU * (t * 0.11 + r(3)));
            let radius = size * (0.16 + 0.14 * r(4)) * (0.6 + 0.4 * middle) * breath;
            let bob = 0.03 * size * math::sin(core::f32::consts::TAU * (t * 0.07 + r(5)));
            let yaw = core::f32::consts::TAU * r(6) + 0.05 * t;
            let turn = [0.0, math::sin(yaw * 0.5), 0.0, math::cos(yaw * 0.5)];
            // the underside a little darker
            let shade = 0.82 + 0.18 * r(1);
            table.push(
                Instance::new(shape::PUFF, [at[0], at[1] + bob, at[2]])
                    .turned(turn)
                    .scaled([radius * 1.25, radius * 0.85, radius])
                    .coloured([base[0] * shade, base[1] * shade, base[2] * shade, 1.0]),
            );
        }
    }
}
