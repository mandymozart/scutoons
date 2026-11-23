//! `cloud` – a low-poly cloud floating above its place: a cluster of puffs (each one of the models in the slot `puff`, .glb files
//! from the library; the core's lumpy placeholder while there is none) that drifts with the wind, breathes a little and wraps
//! back when it has drifted past its range. It can rain (drops falling from its underside to the place), split into smaller sub-clouds that drift apart and
//! come back together, and grow and shrink; a new size glides. Its layout is the node's `seed`; every step rewrites its
//! instance table, so the table is the same on every deck.
//!
//! The storm: while `hold` is held a repeater fires – each repeat a note on `thunder` and on `kick` (a cable a voice) and
//! a flash that lights and swells the puffs for a moment. `rate` (it glides) sets how fast: with `sync` on it picks a
//! note value from 1/1 to 1/32 and every repeat lands on that grid of the tempo clock (a stopped clock: none); off, it
//! runs free from 0.5 to 32 a second, the first repeat at the press. `period`, `phase`, `pulse` and `gate` send the
//! repeater's time out, `red` `green` `blue` the cloud's colour – for lights and pictures on the same beat. `gate` is 1
//! from each repeat for the first half of the period and 0 for the second (0 while not held), so it rises at every repeat
//! at every rate – a strobe's trigger; `pulse`'s fade does not fall far enough between fast repeats.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p cloud-module`.
#![allow(clippy::duplicated_attributes)]
use core::f32::consts::TAU;
use deck_module::instances::shape;
use deck_module::dsp::grid::{frame_in, steps};
use deck_module::{math, Ctx, Ev, Instance, Module, Outputs, Smooth};

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
/// the repeater: how fast a new rate glides (s), how fast a flash fades (s), the free range (repeats a second)
const RATE_GLIDE: f32 = 0.5;
const FLASH_FADE: f32 = 0.12;
const FREE_SLOWEST: f64 = 0.5;
const FREE_FASTEST: f64 = 32.0;
/// the synced note values, in beats: 1/1 … 1/32
pub const DIVISIONS: [f64; 6] = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125];
/// the keys of the two notes (the voices may ignore them)
const THUNDER_KEY: f32 = 40.0;
const KICK_KEY: f32 = 36.0;

#[derive(Module)]
#[module(name = "cloud", domain = "control", doc = "a low-poly cloud of puffs floating above its place, drifting with the wind; it rains, splits and swells")]
#[port(output, instances, name = "shapes", capacity = 96, doc = "the puffs and the rain's drops, drawn at the node's place")]
#[port(output, event, name = "thunder", names = "music.note", props = "pitch|vel|glide|key", capacity = 16, doc = "a note each repeat, for the thunder's voice")]
#[port(output, event, name = "kick", names = "music.note", props = "pitch|vel|glide|key", capacity = 16, doc = "a note each repeat, for the kick's voice")]
#[port(output, control, name = "period", doc = "the repeat's period in seconds")]
#[port(output, control, name = "phase", doc = "how far into the period, 0 at a repeat … 1")]
#[port(output, control, name = "pulse", doc = "the flash: 1 at a repeat, fading")]
#[port(output, control, name = "gate", doc = "1 from each repeat for the first half of the period, 0 for the second; 0 while not held")]
#[port(output, control, name = "red", doc = "the cloud's colour, red 0 … 1")]
#[port(output, control, name = "green", doc = "the cloud's colour, green 0 … 1")]
#[port(output, control, name = "blue", doc = "the cloud's colour, blue 0 … 1")]
#[asset(name = "puff", kind = "model", max_bytes = 4194304, doc = "the puffs' shapes: a set of low-poly models (.glb, each at most 4 MiB, fitted to a metre), each puff one of them; none: the placeholder")]
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
    #[param(range = 0.0..1.0, default = 0.0, doc = "while held (at 0.5 or more) the repeater fires")]
    hold: f32,
    #[param(range = 0.0..1.0, default = 0.5, doc = "how fast it repeats (a change glides): a note value with sync, else 0.5 … 32 a second")]
    rate: f32,
    #[param(choices = "off|on", default = 1, doc = "on: the repeats land on the tempo's grid; off: free time")]
    sync: f32,
    #[param(range = 0.0..1.0, default = 0.8, doc = "the thunder note's velocity (0: no thunder notes)")]
    rumble: f32,
    #[param(range = 0.0..1.0, default = 1.0, doc = "the kick note's velocity (0: no kick notes)")]
    thump: f32,
    #[param(range = 0.0..1.0, default = 0.5, doc = "how much a repeat lights and swells the cloud")]
    flash: f32,
    /// metres drifted, inside the range
    drifted: f64,
    /// seconds since the start (the breathing)
    time: f64,
    /// the size, the swell and the split on their way to the params
    sized: Smooth,
    swelled: Smooth,
    parted: Smooth,
    rainfall: Rain,
    rated: Smooth,
    repeater: Repeater,
    started: bool,
}

/// the repeater between calls
#[derive(Default)]
struct Repeater {
    /// held in the last call
    held: bool,
    /// free time: how far to the next repeat, 0 … 1
    phase: f64,
    /// repeats so far (each note's id)
    count: u32,
    /// the flash, 1 at a repeat
    pulse: f32,
    /// a repeat has fired since the press
    fired: bool,
    /// frames from the last repeat to the end of the last call
    since: f64,
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

/// hue, saturation, value (0 … 1) → display RGB (0 … 1)
fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let k = |n: f32| {
        let k = math::fmod(n + h * 6.0, 6.0);
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [k(5.0), k(3.0), k(1.0)]
}

impl Cloud {
    /// the repeater's period in beats with sync (the note value `rate` picks)
    pub fn division(rate: f32) -> f64 {
        DIVISIONS[(math::round(rate.clamp(0.0, 1.0) * (DIVISIONS.len() - 1) as f32) as usize).min(DIVISIONS.len() - 1)]
    }

    /// the repeater's free speed, repeats a second
    pub fn free_hz(rate: f32) -> f64 {
        FREE_SLOWEST * math::libm::pow(FREE_FASTEST / FREE_SLOWEST, f64::from(rate.clamp(0.0, 1.0)))
    }

    /// the call's repeats: their notes out, the flash lit; the period (s) and the phase after the call
    fn repeat(&mut self, ctx: &Ctx, out: &mut Outputs, rate: f32) -> (f64, f64) {
        let held = self.hold >= 0.5;
        let pressed = held && !self.repeater.held;
        self.repeater.held = held;
        if !held {
            self.repeater.fired = false;
        }
        let tempo = ctx.tempo();
        let (frame, frames) = (ctx.frame(), ctx.frames());
        let seconds_a_beat = 60.0 / tempo.bpm.max(1.0);
        let mut hits = [0u32; 16];
        let mut n = 0;
        let (period, phase) = if self.sync >= 0.5 {
            let beats = Cloud::division(rate);
            let (from, to) = ctx.beats();
            if held {
                // a repeat belongs to the call its rounded frame falls in: every cut plays it on the same frame
                let margin = tempo.per_frame();
                for (_, beat) in steps(from - margin, to + margin, 1.0 / beats) {
                    if let Some(at) = frame_in(&tempo, frame, frames, beat) {
                        if n < hits.len() {
                            hits[n] = at;
                            n += 1;
                        }
                    }
                }
            }
            (beats * seconds_a_beat, if tempo.running { math::libm::fmod(to / beats, 1.0) } else { 0.0 })
        } else {
            let hz = Cloud::free_hz(rate);
            let per_frame = hz / ctx.rate().max(1.0);
            if pressed {
                self.repeater.phase = 1.0;
            }
            if held {
                // a repeat plays on its nearest frame, in the call that frame belongs to
                let mut at = (1.0 - self.repeater.phase) / per_frame;
                while at + 0.5 < frames as f64 && n < hits.len() {
                    hits[n] = (at + 0.5).max(0.0) as u32;
                    n += 1;
                    at += 1.0 / per_frame;
                }
                self.repeater.phase = 1.0 - (at - frames as f64) * per_frame;
            } else {
                self.repeater.phase = 0.0;
            }
            (1.0 / hz, self.repeater.phase.clamp(0.0, 1.0))
        };
        let len = (0.5 * period / seconds_a_beat) as f32;
        for &at in &hits[..n] {
            let id = self.repeater.count;
            self.repeater.count = id.wrapping_add(1);
            if self.rumble > 0.0 {
                out.emit(Self::THUNDER, Ev::note_on(at, id, f32::NAN, THUNDER_KEY, self.rumble.min(1.0)).lasting(len));
            }
            if self.thump > 0.0 {
                out.emit(Self::KICK, Ev::note_on(at, id, f32::NAN, KICK_KEY, self.thump.min(1.0)).lasting(len));
            }
        }
        if n > 0 {
            self.repeater.pulse = 1.0;
            self.repeater.fired = true;
            self.repeater.since = f64::from(frames as u32 - hits[n - 1]);
        } else {
            self.repeater.since += frames as f64;
        }
        (period, phase)
    }
}

impl Module for Cloud {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let seed = ctx.seed();
        if !self.started {
            self.started = true;
            self.sized = Smooth::new(self.size);
            self.swelled = Smooth::new(self.swell);
            self.parted = Smooth::new(self.split);
            self.rated = Smooth::new(self.rate);
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
        self.rated.to(self.rate.clamp(0.0, 1.0));
        let rate = self.rated.step(Smooth::coef(RATE_GLIDE, dt as f32));
        let (period, phase) = self.repeat(ctx, out, rate);
        let pulse = self.repeater.pulse;
        let ticking = self.sync < 0.5 || ctx.tempo().running;
        // counted in frames from the repeat itself: a call ending on the grid is still the period's second half
        let gate = self.repeater.held && self.repeater.fired && ticking && self.repeater.since < 0.5 * period * ctx.rate();
        let lit = self.flash.clamp(0.0, 1.0) * pulse;
        self.repeater.pulse = pulse * math::exp(-(dt as f32) / FLASH_FADE);
        // a slow swell, about twenty seconds a breath
        let size = sized * (1.0 + 0.4 * swell * math::sin(TAU * (t * 0.05 + unit(seed, 0, 41))));

        // the cluster's middle runs from one end of the range to the other
        let x = (self.drifted - range * 0.5) as f32;
        let n = (math::round(self.puffs) as usize).clamp(3, MOST);
        let parts = 2 + (unit(seed, 0, 40) * 3.0) as usize;
        let shown = hsv(self.hue, self.tint * 0.01, self.light * 0.01);
        out.set(Self::PERIOD, period as f32);
        out.set(Self::PHASE, phase as f32);
        out.set(Self::PULSE, pulse);
        out.set(Self::GATE, if gate { 1.0 } else { 0.0 });
        out.set(Self::RED, shown[0]);
        out.set(Self::GREEN, shown[1]);
        out.set(Self::BLUE, shown[2]);
        let base = shown.map(|c| math::pow(c, 2.2));
        // a repeat's flash: the puffs toward white and a little bigger
        let glow = base.map(|c| c + (1.0 - c) * 0.7 * lit);
        let bulge = 1.0 + 0.12 * lit;
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
            let radius = size * (0.16 + 0.14 * r(4)) * (0.6 + 0.4 * middle) * breath * (1.0 - 0.3 * split) * bulge;
            let bob = 0.03 * size * math::sin(TAU * (t * 0.07 + r(5)));
            let yaw = TAU * r(6) + 0.05 * t;
            let turn = [0.0, math::sin(yaw * 0.5), 0.0, math::cos(yaw * 0.5)];
            // the underside a little darker
            let shade = 0.82 + 0.18 * r(1);
            table.push(
                Instance::new(shape::PUFF, [at[0], at[1] + bob, at[2]])
                    .turned(turn)
                    .scaled([radius * 1.25, radius * 0.85, radius])
                    .coloured([glow[0] * shade, glow[1] * shade, glow[2] * shade, 1.0]),
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
