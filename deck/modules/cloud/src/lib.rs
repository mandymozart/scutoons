//! `cloud` – a low-poly cloud floating above its place: a lumpy cluster of puffs (the core's placeholder shape until the
//! owner's models come) that drifts with the wind, breathes a little and wraps back when it has drifted past its
//! range. It can rain (drops falling from its underside to the place), split into smaller sub-clouds that drift apart and
//! come back together, and grow and shrink; a new size glides. Its layout is the node's `seed`; every step rewrites its
//! instance table, so the table is the same on every deck.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p cloud-module`.
#![allow(clippy::duplicated_attributes)]
use core::f32::consts::TAU;
use deck_module::instances::shape;
use deck_module::{math, Ctx, Instance, Module, Outputs, Smooth};

/// the most puffs
pub const MOST: usize = 24;
/// the rain's drops, a fixed pool after the puffs
pub const DROPS: usize = 72;
/// the table's capacity (the port's `capacity`)
pub const CAPACITY: usize = 96;
const _: () = assert!(MOST + DROPS <= CAPACITY);

/// time constants (s): a new size or swell gliding in, the parts drifting apart or together
const SIZE_GLIDE: f32 = 0.8;
const SPLIT_GLIDE: f32 = 1.5;
/// a drop's fall, metres a second
const FALL: f32 = 7.0;

#[derive(Module)]
#[module(name = "cloud", domain = "control", doc = "a low-poly cloud of puffs floating above its place, drifting with the wind; it rains, splits and swells")]
#[port(output, instances, name = "shapes", capacity = 96, doc = "the puffs and the rain's drops, drawn at the node's place")]
#[preset(name = "storm", doc = "a dark, wide cloud in a strong wind, raining hard", values(puffs = 18.0, size = 6.0, drift = 2.5, light = 35.0, rain = 0.8))]
pub struct Cloud {
    #[param(range = 3.0..24.0, default = 9.0, doc = "how many puffs")]
    puffs: f32,
    #[param(range = 0.5..20.0, default = 3.0, unit = "m", doc = "how wide the cloud is (a change glides)")]
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
    #[param(range = 0.0..1.0, default = 0.0, doc = "how hard it rains: the share of the drops falling")]
    rain: f32,
    #[param(range = 0.0..1.0, default = 0.0, doc = "how far it has come apart into smaller sub-clouds (back to 0: together again)")]
    split: f32,
    #[param(range = 0.0..1.0, default = 0.0, doc = "how much it slowly grows and shrinks")]
    swell: f32,
    /// metres drifted, inside the range
    drifted: f64,
    /// seconds since the start (the breathing)
    time: f64,
    /// the size, the swell and the split on their way to the params
    sized: Smooth,
    swelled: Smooth,
    parted: Smooth,
    rainfall: Rain,
    started: bool,
}

/// each drop's way down (0 at the underside, 1 at the place) and how often it has fallen
struct Rain {
    down: [f32; DROPS],
    falls: [u32; DROPS],
}

impl Default for Rain {
    fn default() -> Rain {
        Rain { down: [0.0; DROPS], falls: [0; DROPS] }
    }
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
        let seed = ctx.seed();
        if !self.started {
            self.started = true;
            self.sized = Smooth::new(self.size);
            self.swelled = Smooth::new(self.swell);
            self.parted = Smooth::new(self.split);
            for (i, d) in self.rainfall.down.iter_mut().enumerate() {
                *d = unit(seed, i as u64, 20);
            }
        }
        let dt = ctx.frames() as f64 / ctx.rate().max(1.0);
        self.time += dt;
        let range = f64::from(self.range.max(2.0));
        self.drifted = math::libm::fmod(self.drifted + f64::from(self.drift) * dt, range);
        if self.drifted < 0.0 {
            self.drifted += range;
        }
        let t = self.time as f32;
        self.sized.to(self.size.max(0.1));
        self.swelled.to(self.swell.clamp(0.0, 1.0));
        self.parted.to(self.split.clamp(0.0, 1.0));
        let sized = self.sized.step(Smooth::coef(SIZE_GLIDE, dt as f32));
        let swell = self.swelled.step(Smooth::coef(SIZE_GLIDE, dt as f32));
        self.parted.step(Smooth::coef(SPLIT_GLIDE, dt as f32));
        self.parted.land(1e-4);
        let split = self.parted.value();
        // a slow swell, about twenty seconds a breath
        let size = sized * (1.0 + 0.4 * swell * math::sin(TAU * (t * 0.05 + unit(seed, 0, 41))));

        // the cluster's middle runs from one end of the range to the other
        let x = (self.drifted - range * 0.5) as f32;
        let n = (math::round(self.puffs) as usize).clamp(3, MOST);
        let parts = 2 + (unit(seed, 0, 40) * 3.0) as usize;
        let base = hsv(self.hue, self.tint * 0.01, self.light * 0.01);
        let mut table = out.instances();
        table.clear();
        for i in 0..n as u64 {
            let r = |k: u64| unit(seed, i, k);
            // across the cloud, a flat bottom and a heaped middle
            let across = r(0) - 0.5;
            let middle = 1.0 - 2.0 * across.abs();
            // its part: a slice across the cloud, which drifts off on its own and draws its puffs together
            let part = ((across + 0.5) * parts as f32).min(parts as f32 - 1.0) as u64;
            let centre = (part as f32 + 0.5) / parts as f32 - 0.5;
            let along = centre * (1.0 + 1.6 * split) + (across - centre) * (1.0 - 0.4 * split);
            let away = [(unit(seed, part, 30) - 0.5) * 1.2 * split, (unit(seed, part, 31) - 0.5) * 0.3 * split];
            let at = [
                x + along * size,
                self.height + r(1) * size * 0.3 * (0.4 + 0.6 * middle) + away[1] * size,
                (r(2) - 0.5) * size * 0.45 + away[0] * size,
            ];
            let breath = 1.0 + 0.05 * math::sin(TAU * (t * 0.11 + r(3)));
            let radius = size * (0.16 + 0.14 * r(4)) * (0.6 + 0.4 * middle) * breath * (1.0 - 0.3 * split);
            let bob = 0.03 * size * math::sin(TAU * (t * 0.07 + r(5)));
            let yaw = TAU * r(6) + 0.05 * t;
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

        // the rain: every drop falls whether it shows or not; at the place it starts again under a puff picked anew
        // and shows in this fall when its draw is under `rain`, so drops come and go only at the underside
        let water = [base[0] * 0.55 + 0.08, base[1] * 0.6 + 0.12, base[2] * 0.7 + 0.25, 1.0];
        for i in 0..DROPS {
            let fall = self.rainfall.falls[i];
            let k = |j: u64| unit(seed, i as u64, 100 + 4 * u64::from(fall) + j);
            let puff = table.rows()[((k(0) * n as f32) as usize).min(n - 1)];
            let r = puff.scale[2];
            let top = puff.position[1] - r * 0.6;
            let way = top.max(1.0);
            let speed = FALL * (0.85 + 0.3 * unit(seed, i as u64, 21));
            let down = self.rainfall.down[i];
            let shows = k(3) < self.rain;
            let next = down + speed * dt as f32 / way;
            if next >= 1.0 {
                self.rainfall.down[i] = math::fmod(next, 1.0);
                self.rainfall.falls[i] = fall.wrapping_add(1);
            } else {
                self.rainfall.down[i] = next;
            }
            if shows {
                let streak = 0.25 + 0.03 * size.min(10.0);
                table.push(
                    Instance::new(shape::SEGMENT, [puff.position[0] + (k(1) - 0.5) * r * 1.6, top - down * way - streak, puff.position[2] + (k(2) - 0.5) * r * 1.2])
                        .scaled([0.04, streak, 0.04])
                        .coloured(water),
                );
            }
        }
    }
}
