//! `bamboo` – a grove of two kinds of bamboo, thin and tall, thick and short, swaying in a simulated wind; whenever two
//! stalks hit each other it plays a knock, a note on `hits` – the grove is a sequencer the wind plays. Each kind has a
//! count multiplier (10 stalks × it, at most 40 a kind), a height, a thickness and a hue; the layout (clumps of five) is
//! the node's `seed`. Every stalk is a damped spring bending from its foot, pushed by a wind of `wind` strength from
//! `direction` with seeded gusts travelling across the grove (`gust`) and a flutter of its own. Two neighbouring stalks
//! hit when their bent stalks come closer than their radii: they bounce apart, and the pair knocks once – again only
//! after `debounce`. The knock's key comes from the pair (the shorter, the higher, on `scale` from `root`, two octaves;
//! two thick stalks an octave down), its velocity from how fast they closed.
//!
//! The simulation runs in fixed ticks of 64 frames whatever the calls' cut, so the grove and its notes are the same on
//! every deck. Every call rewrites the instance table: a stalk is six `segment`s with a node ring at each joint and a
//! leafy crown (12 instances; 80 stalks fill 960). Its look may come from the library: the slots `thin` and `thick` each
//! hold a set of internode models (one stem section, its node ring at the top); every segment of a stalk draws the same
//! model of its kind's set, stretched to the segment, and the rings step aside – while a set is empty, the segments and
//! rings as they are. The sway and the hits stay on these segments either way: a model is only the look.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p bamboo-module`.
#![allow(clippy::duplicated_attributes)]
use core::f32::consts::TAU;
use deck_module::instances::shape;
use deck_module::{math, Ctx, Ev, Instance, Module, Outputs, Smooth};

/// stalks of one kind at most, stalks for each 1 of a multiplier, stalks a clump
pub const MOST_A_KIND: usize = 40;
pub const BASE: f32 = 10.0;
const CLUMP: usize = 5;
/// every stalk: thin ones first, then thick ones
pub const STALKS: usize = 2 * MOST_A_KIND;
/// a stalk's segments; its instances (segments, the rings between them, a crown)
pub const SEGMENTS: usize = 6;
pub const PER_STALK: usize = 2 * SEGMENTS;
/// the table's capacity (the port's `capacity`)
pub const CAPACITY: usize = STALKS * PER_STALK;
const _: () = assert!(CAPACITY == 960);
/// the neighbour pairs that can hit
pub const MOST_PAIRS: usize = 384;
/// the simulation's tick, frames
pub const TICK: u64 = 64;
/// the wind at `wind` = 1, m/s (its pressure grows with `wind`)
const STRONGEST: f32 = 15.0;
/// drag: a stalk's push = DRAG · U² · height / width (m/s²)
const DRAG: f32 = 7.0e-4;
/// damping ratio, the restitution of a hit, a hit's least closing speed (m/s), full velocity's (m/s)
const DAMPING: f32 = 0.08;
const BOUNCE: f32 = 0.4;
pub const SLOWEST_HIT: f32 = 0.03;
pub const FULL_HIT: f32 = 3.0;
/// how far apart two stalks' feet can be and still meet: their radii and this share of the shorter one's height
const REACH: f32 = 0.15;
/// where two stalks meet, a share of the shorter one's height
const MEET: f32 = 0.85;
/// the glides (s): wind and gust, the direction
const WIND_GLIDE: f32 = 1.5;
/// a knock's length, beats
const KNOCK: f32 = 0.25;
/// the scales `scale` picks, semitones
pub const SCALES: [&[f32]; 4] = [&[0.0, 2.0, 4.0, 7.0, 9.0], &[0.0, 3.0, 5.0, 7.0, 10.0], &[0.0, 2.0, 3.0, 7.0, 8.0], &[0.0, 2.0, 4.0, 6.0, 8.0, 10.0]];
/// the heights the keys spread over, m (the shortest highest)
const LOWEST: f32 = 2.0;
const TALLEST: f32 = 10.0;

#[derive(Module)]
#[module(name = "bamboo", domain = "control", doc = "a grove of thin, tall and thick, short bamboo swaying in the wind; a knock whenever two stalks hit")]
#[port(output, instances, name = "shapes", capacity = 960, doc = "the stalks: segments, node rings and crowns, drawn at the node's place")]
#[port(output, event, name = "hits", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "a note each time two stalks hit: the key from the pair, the velocity from how fast they closed")]
#[port(output, control, name = "hit_rate", doc = "hits a second (a one-second average)")]
#[port(output, control, name = "breeze", doc = "the wind in the grove's middle now, 0 … 1 (more in a gust)")]
#[port(output, control, name = "sway", doc = "how far the stalks bend, their tips' mean share of their height")]
#[asset(name = "thin", kind = "model", max_bytes = 4194304, doc = "the thin bamboo's internodes: a set of models of one stem section, its node ring at the top (.glb, each at most 4 MiB, stretched to a segment); each stalk one of them; none: the drawn stalks")]
#[asset(name = "thick", kind = "model", max_bytes = 4194304, doc = "the thick bamboo's internodes: a set of models of one stem section, its node ring at the top (.glb, each at most 4 MiB, stretched to a segment); each stalk one of them; none: the drawn stalks")]
#[preset(name = "storm", doc = "a dense grove in a gusty gale", values(thin = 3.0, thick = 2.0, wind = 0.9, gust = 0.8))]
pub struct Bamboo {
    #[param(range = 0.0..4.0, default = 1.0, doc = "the thin, tall bamboo: how many (10 stalks × this, at most 40)")]
    thin: f32,
    #[param(range = 0.0..4.0, default = 1.0, doc = "the thick, short bamboo: how many (10 stalks × this, at most 40)")]
    thick: f32,
    #[param(range = 2.0..12.0, default = 7.0, unit = "m", doc = "the thin bamboo's height")]
    thin_height: f32,
    #[param(range = 1.0..8.0, default = 4.0, unit = "m", doc = "the thick bamboo's height")]
    thick_height: f32,
    #[param(range = 0.02..0.2, default = 0.06, unit = "m", doc = "the thin bamboo's thickness")]
    thin_width: f32,
    #[param(range = 0.05..0.4, default = 0.14, unit = "m", doc = "the thick bamboo's thickness")]
    thick_width: f32,
    #[param(range = 0.0..1.0, default = 0.22, doc = "the thin bamboo's hue")]
    thin_hue: f32,
    #[param(range = 0.0..1.0, default = 0.3, doc = "the thick bamboo's hue")]
    thick_hue: f32,
    #[param(range = 1.0..30.0, default = 6.0, unit = "m", doc = "how wide the grove is")]
    spread: f32,
    #[param(range = 0.0..1.0, default = 0.3, doc = "the wind's strength (a change glides)")]
    wind: f32,
    #[param(range = 0.0..1.0, default = 0.4, doc = "how gusty: gusts travelling across the grove and each stalk's flutter")]
    gust: f32,
    #[param(range = 0.0..360.0, default = 0.0, unit = "°", doc = "where the wind blows to: 0 along the stage's x, 90 along z")]
    direction: f32,
    #[param(range = 24.0..96.0, default = 60.0, doc = "the knocks' lowest key (MIDI)")]
    root: f32,
    #[param(choices = "major pentatonic|minor pentatonic|hirajoshi|whole tone", default = 0, doc = "the knocks' scale")]
    scale: f32,
    #[param(range = 0.02..1.0, default = 0.12, unit = "s", doc = "a pair knocks again only after this long")]
    debounce: f32,
    grove: Grove,
    /// frames since the start (the ticks)
    clock: u64,
    /// seconds simulated
    time: f64,
    /// what the layout was made of
    laid: [f32; 7],
    blowing: Smooth,
    gusting: Smooth,
    heading: [Smooth; 2],
    hits: u32,
    hit_rate: f32,
    breeze: f32,
    started: bool,
}

/// one stalk: where it stands, what it is, how it bends (its tip's offset and speed across the floor, x and z)
#[derive(Clone, Copy, Default)]
struct Stalk {
    foot: [f32; 2],
    height: f32,
    width: f32,
    mass: f32,
    omega: f32,
    flutter: f32,
    phase: f32,
    thick: bool,
    live: bool,
    bend: [f32; 2],
    speed: [f32; 2],
}

/// two stalks that can meet: touching in the last tick, how long until they may knock again
#[derive(Clone, Copy, Default)]
struct Pair {
    a: u8,
    b: u8,
    touching: bool,
    wait: f32,
}

struct Grove {
    stalks: [Stalk; STALKS],
    pairs: [Pair; MOST_PAIRS],
    paired: usize,
}

impl Default for Grove {
    fn default() -> Grove {
        Grove { stalks: [Stalk::default(); STALKS], pairs: [Pair::default(); MOST_PAIRS], paired: 0 }
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

/// hue, saturation, value (0 … 1) → display RGB (0 … 1)
fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let k = |n: f32| {
        let k = math::fmod(n + h * 6.0, 6.0);
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [k(5.0), k(3.0), k(1.0)]
}

fn length(v: [f32; 2]) -> f32 {
    math::sqrt(v[0] * v[0] + v[1] * v[1])
}

/// the travelling gusts: 0 … about 1 at `along` m down the wind at `t` s
fn gusts(seed: u64, along: f32, t: f32) -> f32 {
    let mut g = 0.0;
    for k in 0..3u64 {
        let wave = 6.0 + 8.0 * unit(seed, k, 60);
        let pace = 3.0 + 3.0 * unit(seed, k, 61);
        let s = math::sin(TAU * ((along - pace * t) / wave + unit(seed, k, 62))).max(0.0);
        g += s * s * (0.5 + 0.5 * unit(seed, k, 63));
    }
    g * 0.8
}

impl Bamboo {
    /// stalks of a kind for a multiplier
    pub fn count(multiplier: f32) -> usize {
        (math::round(BASE * multiplier.clamp(0.0, 4.0)) as usize).min(MOST_A_KIND)
    }

    /// The knock of two stalks of heights `a` and `b` (m), each thick or not: two octaves of `scale` from `root`, the
    /// shorter the pair (their mean height) the higher; two thick stalks an octave down.
    pub fn key(root: f32, scale: usize, a: (f32, bool), b: (f32, bool)) -> f32 {
        let steps = SCALES[scale.min(SCALES.len() - 1)];
        let n = 2 * steps.len();
        let t = (((a.0 + b.0) * 0.5 - LOWEST) / (TALLEST - LOWEST)).clamp(0.0, 1.0);
        let degree = (math::round((1.0 - t) * (n - 1) as f32) as usize).min(n - 1);
        let low = if a.1 && b.1 { 12.0 } else { 0.0 };
        root + 12.0 * (degree / steps.len()) as f32 + steps[degree % steps.len()] - low
    }

    /// a knock's velocity for a closing speed (m/s)
    pub fn velocity(closing: f32) -> f32 {
        (closing / FULL_HIT).clamp(0.05, 1.0)
    }

    /// the layout for this call's counts, sizes and spread: stalks placed, the pairs that can meet found
    fn lay_out(&mut self, seed: u64) {
        let key = [Bamboo::count(self.thin) as f32, Bamboo::count(self.thick) as f32, self.thin_height, self.thick_height, self.thin_width, self.thick_width, self.spread];
        if self.started && key == self.laid {
            return;
        }
        self.laid = key;
        let spread = self.spread.clamp(1.0, 30.0);
        let clump = (0.6 * spread / 6.0).max(0.3);
        let g = &mut self.grove;
        for (i, s) in g.stalks.iter_mut().enumerate() {
            let thick = i >= MOST_A_KIND;
            let j = i % MOST_A_KIND;
            let kind = u64::from(thick);
            let c = (j / CLUMP) as u64;
            let r = |k: u64| unit(seed, i as u64, k);
            let middle = [(unit(seed, 1000 + kind * 64 + c, 0) - 0.5) * spread, (unit(seed, 1000 + kind * 64 + c, 1) - 0.5) * spread * 0.6];
            let (away, turn) = (clump * math::sqrt(r(2)), TAU * r(3));
            let (height, width) = if thick { (self.thick_height, self.thick_width) } else { (self.thin_height, self.thin_width) };
            let height = height * (0.8 + 0.35 * r(4));
            let width = width * (0.85 + 0.3 * r(5));
            let was = s.live;
            s.live = j < Bamboo::count(if thick { self.thick } else { self.thin });
            s.thick = thick;
            s.foot = [middle[0] + away * math::cos(turn), middle[1] + away * math::sin(turn)];
            s.height = height;
            s.width = width;
            s.mass = width * width * height;
            s.omega = TAU * (0.4 + 60.0 * width / (height * height)) * (0.75 + 0.5 * r(6));
            s.flutter = 0.3 + 0.9 * r(7);
            s.phase = r(8);
            if !s.live || !was {
                s.bend = [0.0; 2];
                s.speed = [0.0; 2];
            }
        }
        g.paired = 0;
        for a in 0..STALKS {
            for b in a + 1..STALKS {
                let (sa, sb) = (&g.stalks[a], &g.stalks[b]);
                if !sa.live || !sb.live || g.paired == MOST_PAIRS {
                    continue;
                }
                let apart = length([sa.foot[0] - sb.foot[0], sa.foot[1] - sb.foot[1]]);
                let radii = (sa.width + sb.width) * 0.5;
                if apart > radii + 0.004 && apart < radii + REACH * sa.height.min(sb.height) {
                    g.pairs[g.paired] = Pair { a: a as u8, b: b as u8, touching: false, wait: 0.0 };
                    g.paired += 1;
                }
            }
        }
    }

    /// One tick of `h` s: the wind pushes, the springs bend back, touching neighbours bounce apart and knock (at `at`,
    /// the call's frame). The wind in the grove's middle.
    fn tick(&mut self, seed: u64, h: f32, at: u32, out: &mut Outputs) -> f32 {
        let k = Smooth::coef(WIND_GLIDE, h);
        self.blowing.to(self.wind.clamp(0.0, 1.0));
        self.gusting.to(self.gust.clamp(0.0, 1.0));
        let turn = self.direction.to_radians();
        self.heading[0].to(math::cos(turn));
        self.heading[1].to(math::sin(turn));
        let wind = self.blowing.step(k);
        let gust = self.gusting.step(k);
        let mut dir = [self.heading[0].step(k), self.heading[1].step(k)];
        let l = length(dir).max(1e-6);
        dir = [dir[0] / l, dir[1] / l];
        let t = self.time as f32;
        let middle = wind * (1.0 - 0.4 * gust + 1.6 * gust * gusts(seed, 0.0, t));
        for s in self.grove.stalks.iter_mut().filter(|s| s.live) {
            let along = s.foot[0] * dir[0] + s.foot[1] * dir[1];
            // the pressure along the wind and across it, 1 = a steady gale's
            let along = wind * (1.0 - 0.4 * gust + 1.6 * gust * gusts(seed, along, t));
            let across = wind * 0.6 * (0.3 + gust) * math::sin(TAU * (t * s.flutter + s.phase));
            let push = DRAG * STRONGEST * STRONGEST * s.height / s.width;
            let force = [push * (along * dir[0] - across * dir[1]), push * (along * dir[1] + across * dir[0])];
            let limit = 0.3 * s.height;
            let bent = length(s.bend) / limit;
            let stiff = s.omega * s.omega * (1.0 + bent * bent);
            for (c, f) in force.into_iter().enumerate() {
                s.speed[c] += h * (f - stiff * s.bend[c] - 2.0 * DAMPING * s.omega * s.speed[c]);
                s.bend[c] += h * s.speed[c];
            }
        }
        let (scale, root, debounce) = (self.scale as usize, self.root, self.debounce.max(0.02));
        let g = &mut self.grove;
        for p in g.pairs[..g.paired].iter_mut() {
            let (a, b) = (g.stalks[p.a as usize], g.stalks[p.b as usize]);
            let meet = MEET * a.height.min(b.height);
            let (fa, fb) = ((meet / a.height) * (meet / a.height), (meet / b.height) * (meet / b.height));
            let gap = [a.foot[0] + fa * a.bend[0] - b.foot[0] - fb * b.bend[0], a.foot[1] + fa * a.bend[1] - b.foot[1] - fb * b.bend[1]];
            let apart = length(gap).max(1e-6);
            let radii = (a.width + b.width) * 0.4;
            let touching = apart < radii;
            p.wait -= h;
            if touching {
                let n = [gap[0] / apart, gap[1] / apart];
                let closing = -((fa * a.speed[0] - fb * b.speed[0]) * n[0] + (fa * a.speed[1] - fb * b.speed[1]) * n[1]);
                if !p.touching && p.wait <= 0.0 && closing > SLOWEST_HIT {
                    p.wait = debounce;
                    let id = (u32::from(p.a) * STALKS as u32 + u32::from(p.b)) << 16 | (self.hits & 0xffff);
                    self.hits = self.hits.wrapping_add(1);
                    self.hit_rate += 1.0;
                    let key = Bamboo::key(root, scale, (a.height, a.thick), (b.height, b.thick));
                    out.emit(Self::HITS, Ev::note_on(at, id, f32::NAN, key, Bamboo::velocity(closing)).lasting(KNOCK));
                }
                let (ia, ib) = (1.0 / a.mass, 1.0 / b.mass);
                let sa = &mut g.stalks[p.a as usize];
                if closing > 0.0 {
                    let j = (1.0 + BOUNCE) * closing / (fa * fa * ia + fb * fb * ib);
                    sa.speed[0] += j * fa * ia * n[0];
                    sa.speed[1] += j * fa * ia * n[1];
                }
                let overlap = radii - apart;
                let share = ia / (ia + ib);
                sa.bend[0] += n[0] * overlap * share / fa;
                sa.bend[1] += n[1] * overlap * share / fa;
                let sb = &mut g.stalks[p.b as usize];
                if closing > 0.0 {
                    let j = (1.0 + BOUNCE) * closing / (fa * fa * ia + fb * fb * ib);
                    sb.speed[0] -= j * fb * ib * n[0];
                    sb.speed[1] -= j * fb * ib * n[1];
                }
                sb.bend[0] -= n[0] * overlap * (1.0 - share) / fb;
                sb.bend[1] -= n[1] * overlap * (1.0 - share) / fb;
            }
            p.touching = touching;
        }
        self.hit_rate *= math::exp(-h);
        self.time += f64::from(h);
        middle
    }

    /// every live stalk into the table
    fn draw(&self, out: &mut Outputs) {
        let mut table = out.instances();
        table.clear();
        // each kind's cane, ring and leaves (linear)
        let colours = [self.thin_hue, self.thick_hue].map(|hue| {
            [hsv(hue, 0.55, 0.78), hsv(hue - 0.03, 0.5, 0.5), hsv(hue + 0.08, 0.65, 0.5)].map(|c| c.map(|v| math::pow(v, 2.2)))
        });
        for (n, s) in self.grove.stalks.iter().enumerate().filter(|(_, s)| s.live) {
            let [cane, ring, leaves] = colours[usize::from(s.thick)];
            // its kind's internode models: one for the whole stalk (its group)
            let slot = if s.thick { Self::THICK_SLOT } else { Self::THIN_SLOT };
            let group = n as u32 + 1;
            let lean = length(s.bend) / s.height;
            let at = |f: f32| {
                let r = f * f;
                [s.foot[0] + s.bend[0] * r, s.height * f * (1.0 - 0.25 * lean * lean * r), s.foot[1] + s.bend[1] * r]
            };
            for k in 0..SEGMENTS {
                let (p0, p1) = (at(k as f32 / SEGMENTS as f32), at((k + 1) as f32 / SEGMENTS as f32));
                let v = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                let long = math::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).max(1e-6);
                let u = [v[0] / long, v[1] / long, v[2] / long];
                // the turn of +y onto u
                let q = [u[2], 0.0, -u[0], 1.0 + u[1]];
                let ql = math::sqrt(q[0] * q[0] + q[2] * q[2] + q[3] * q[3]).max(1e-6);
                let q = [q[0] / ql, 0.0, q[2] / ql, q[3] / ql];
                let w = s.width * (1.0 - 0.25 * k as f32 / SEGMENTS as f32);
                let shade = if k % 2 == 0 { 1.0 } else { 0.9 };
                table.push(Instance::new(shape::SEGMENT, p0).turned(q).scaled([w, long, w]).coloured([cane[0] * shade, cane[1] * shade, cane[2] * shade, 1.0]).looks(slot, group));
                let band = (w * 0.5).max(0.03);
                if k + 1 < SEGMENTS {
                    let foot = [p1[0] - u[0] * band * 0.5, p1[1] - u[1] * band * 0.5, p1[2] - u[2] * band * 0.5];
                    table.push(Instance::new(shape::SEGMENT, foot).turned(q).scaled([w * 1.18, band, w * 1.18]).coloured([ring[0], ring[1], ring[2], 1.0]).standing_in(slot));
                } else {
                    let crown = 0.25 + 0.06 * s.height;
                    let yaw = TAU * s.phase;
                    table.push(
                        Instance::new(shape::PUFF, [p1[0], p1[1] + crown * 0.2, p1[2]])
                            .turned([0.0, math::sin(yaw * 0.5), 0.0, math::cos(yaw * 0.5)])
                            .scaled([crown, crown * 0.8, crown])
                            .coloured([leaves[0], leaves[1], leaves[2], 1.0]),
                    );
                }
            }
        }
    }
}

impl Module for Bamboo {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let seed = ctx.seed();
        self.lay_out(seed);
        if !self.started {
            self.started = true;
            self.blowing = Smooth::new(self.wind.clamp(0.0, 1.0));
            self.gusting = Smooth::new(self.gust.clamp(0.0, 1.0));
            let turn = self.direction.to_radians();
            self.heading = [Smooth::new(math::cos(turn)), Smooth::new(math::sin(turn))];
        }
        let h = (TICK as f64 / ctx.rate().max(1.0)) as f32;
        let frames = ctx.frames() as u64;
        // the ticks that fall in this call, at their frame
        let mut next = self.clock.div_ceil(TICK) * TICK;
        while next < self.clock + frames {
            self.breeze = self.tick(seed, h, (next - self.clock) as u32, out);
            next += TICK;
        }
        self.clock += frames;
        out.set(Self::BREEZE, self.breeze);
        out.set(Self::HIT_RATE, self.hit_rate);
        let live = self.grove.stalks.iter().filter(|s| s.live);
        let (n, bent) = live.fold((0, 0.0), |(n, b), s| (n + 1, b + length(s.bend) / s.height));
        out.set(Self::SWAY, if n > 0 { bent / n as f32 } else { 0.0 });
        self.draw(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_most_stalks() {
        assert_eq!(Bamboo::count(4.0) * 2 * PER_STALK, CAPACITY);
        assert_eq!(Bamboo::count(9.0), MOST_A_KIND);
    }
}
