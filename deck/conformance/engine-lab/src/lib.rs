//! The lab engine (P0 of the engine refactor, #309): the stage's audio engine lifted into a crate that builds for
//! wasm32, on a **planar N-channel pool** (channel c of the pool = `data[c * MAX_BLOCK ..]`; the stage's `Stereo` is
//! the 2-channel case). Pieces: voices (a copy of stage/src/audio/voices.rs: osc / fm in `F8` groups), mixer, out, the
//! master ramp and deck-core's limiter, the routing ops of plan §2.6 (send with a channel map, folder, `out` map,
//! VBAP, 5.1 fold) and two saws (PolyBLEP, wavetable). Scenarios run the same on native and in the wasm exports.
#![allow(clippy::needless_range_loop)]
use deck_core::lanes::{self as v, F8, LANES};
use deck_core::limiter::Limiter;
use deck_core::smooth::{Ramp, GLIDE, QUICK};
use std::cell::RefCell;

pub const MAX_BLOCK: usize = 1024;
pub const RATE: f32 = 48_000.0;
type Lanes = [f32; LANES];

/// Planar channel buffers: channel `c` is `data[c * MAX_BLOCK ..][.. MAX_BLOCK]`.
pub struct Pool {
    pub channels: usize,
    data: Vec<f32>,
}

impl Pool {
    pub fn new(channels: usize) -> Pool {
        Pool { channels, data: vec![0.0; channels * MAX_BLOCK] }
    }
    #[inline]
    pub fn ch(&self, c: usize) -> &[f32] {
        &self.data[c * MAX_BLOCK..(c + 1) * MAX_BLOCK]
    }
    #[inline]
    pub fn ch_mut(&mut self, c: usize) -> &mut [f32] {
        &mut self.data[c * MAX_BLOCK..(c + 1) * MAX_BLOCK]
    }
    /// `dst[..n] += src[..n] * g` (the two channels differ)
    pub fn add_scaled(&mut self, src: usize, dst: usize, g: f32, n: usize) {
        assert_ne!(src, dst);
        let (a, b) = if src < dst {
            let (lo, hi) = self.data.split_at_mut(dst * MAX_BLOCK);
            (&lo[src * MAX_BLOCK..src * MAX_BLOCK + n], &mut hi[..n])
        } else {
            let (lo, hi) = self.data.split_at_mut(src * MAX_BLOCK);
            (&hi[..n], &mut lo[dst * MAX_BLOCK..dst * MAX_BLOCK + n])
        };
        for (d, s) in b.iter_mut().zip(a) {
            *d += s * g;
        }
    }
    pub fn clear(&mut self, c: usize, n: usize) {
        self.ch_mut(c)[..n].fill(0.0);
    }
}

/// How coefficient maths (`exp`) is done: std (what the stage does: the platform's libm) or the pure-Rust `libm` crate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exp {
    Std,
    Libm,
}

impl Exp {
    pub fn exp(self, x: f32) -> f32 {
        match self {
            Exp::Std => x.exp(),
            Exp::Libm => libm::expf(x),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceSpec {
    pub fm: bool,
    pub freq: f32,
    pub gain: f32,
    pub ratio: f32,
    pub index: f32,
    pub attack: f32,
    pub release: f32,
}

impl VoiceSpec {
    pub fn osc(freq: f32, gain: f32) -> VoiceSpec {
        VoiceSpec { fm: false, freq, gain, ratio: 1.0, index: 1.0, attack: 0.005, release: 0.15 }
    }
    pub fn fm(freq: f32, ratio: f32, index: f32, gain: f32) -> VoiceSpec {
        VoiceSpec { fm: true, freq, gain, ratio, index, attack: 0.005, release: 0.15 }
    }
}

#[derive(Clone, Copy, Default)]
struct Group {
    c: Lanes,
    m: Lanes,
    env: Lanes,
    att: Lanes,
    rel: Lanes,
    p: [Lanes; 4],
    d: [Lanes; 4],
}

/// One batch of one kind (sine or fm), a copy of the stage's `Voices` for constant params and no cv.
pub struct Batch {
    fm: bool,
    rate: f32,
    ramps: Vec<[Ramp; 4]>,
    specs: Vec<VoiceSpec>,
    /// per voice: the pool channels it writes (left, right: the same sound on both, as the stage does)
    dest: Vec<[usize; 2]>,
    fresh: bool,
    groups: Vec<Group>,
    f: Vec<Lanes>,
    gate: Vec<Lanes>,
    y: Vec<Lanes>,
}

impl Batch {
    pub fn new(fm: bool, rate: f32, start_phase: f32, exp: Exp, voices: &[(VoiceSpec, [usize; 2])]) -> Batch {
        let coef = |t: f32| 1.0 - exp.exp(-1.0 / (t.max(1e-4) * rate));
        let mut b = Batch { fm, rate, ramps: vec![], specs: vec![], dest: vec![], fresh: true, groups: vec![], f: vec![], gate: vec![], y: vec![] };
        for (i, (spec, dest)) in voices.iter().enumerate() {
            let (g, l) = (i / LANES, i % LANES);
            if l == 0 {
                b.groups.push(Group::default());
            }
            let group = &mut b.groups[g];
            (group.c[l], group.m[l]) = (start_phase, start_phase);
            (group.att[l], group.rel[l]) = (coef(spec.attack), coef(spec.release));
            b.specs.push(*spec);
            b.dest.push(*dest);
            b.ramps.push([GLIDE, GLIDE, QUICK, QUICK].map(|t| Ramp::new(0.0, t, rate)));
        }
        let n = b.groups.len() * MAX_BLOCK;
        for a in [&mut b.f, &mut b.gate, &mut b.y] {
            *a = vec![[0.0; LANES]; n];
        }
        b
    }

    pub fn process(&mut self, pool: &mut Pool, frames: usize) {
        let fresh = std::mem::take(&mut self.fresh);
        for (i, (spec, ramps)) in self.specs.iter().zip(&mut self.ramps).enumerate() {
            let (g, l) = (i / LANES, i % LANES);
            let group = &mut self.groups[g];
            let xs = [spec.freq, spec.gain, spec.ratio, spec.index];
            for (k, (x, ramp)) in xs.iter().zip(ramps.iter_mut()).enumerate() {
                if fresh {
                    ramp.jump(*x);
                }
                (group.p[k][l], group.d[k][l]) = ramp.block(*x, frames);
            }
            let (f, gate) = (&mut self.f[g * MAX_BLOCK..][..frames], &mut self.gate[g * MAX_BLOCK..][..frames]);
            let (f0, df) = (group.p[0][l], group.d[0][l]);
            for (s, (f, gate)) in f.iter_mut().zip(gate).enumerate() {
                f[l] = f0 + df * s as f32;
                gate[l] = 1.0;
            }
        }
        let sd = (1.0 / self.rate as f64) as f32;
        for (g, group) in self.groups.iter_mut().enumerate() {
            let r = g * MAX_BLOCK..g * MAX_BLOCK + frames;
            run(self.fm, group, &self.f[r.clone()], &self.gate[r.clone()], &mut self.y[r], sd);
        }
        for (i, d) in self.dest.iter().enumerate() {
            let (g, l) = (i / LANES, i % LANES);
            for (s, y) in self.y[g * MAX_BLOCK..][..frames].iter().enumerate() {
                pool.ch_mut(d[0])[s] = y[l];
                pool.ch_mut(d[1])[s] = y[l];
            }
        }
    }
}

#[inline(always)]
fn run(fm: bool, group: &mut Group, f: &[Lanes], gate: &[Lanes], y: &mut [Lanes], sd: f32) {
    let (mut c, mut m, mut env) = (F8(group.c), F8(group.m), F8(group.env));
    let (att, rel) = (F8(group.att), F8(group.rel));
    let [_, g0, r0, k0] = group.p.map(F8);
    let [_, dg, dr, dk] = group.d.map(F8);
    let (sd, half, one, zero) = (F8::splat(sd), F8::splat(0.5), F8::splat(1.0), F8::splat(0.0));
    let mut envelope = |gate: &Lanes| {
        let target = half.lt_select(F8(*gate), one, zero);
        env = env + (target - env) * env.lt_select(target, att, rel);
        env
    };
    let frames = f.iter().zip(gate).zip(y).enumerate();
    if fm {
        for (s, ((f, gate), y)) in frames {
            let s = F8::splat(s as f32);
            let f = F8(*f);
            let fm = f * (r0 + dr * s);
            let mo = v::sin_turns(m);
            m = v::wrap01(m + fm * sd);
            let o = v::sin_turns(c);
            c = v::wrap01(c + (f + mo * (k0 + dk * s) * fm).abs() * sd);
            *y = (o * envelope(gate) * (g0 + dg * s)).0;
        }
    } else {
        for (s, ((f, gate), y)) in frames {
            let s = F8::splat(s as f32);
            let o = v::sin_turns(c);
            c = v::wrap01(c + F8(*f) * sd);
            *y = (o * envelope(gate) * (g0 + dg * s)).0;
        }
    }
    (group.c, group.m, group.env) = (c.0, m.0, env.0);
}

// ── scenario 0: today's stereo scene ────────────────────────────────────────────────────────────────────────────────

/// osc 220 · fm 330 (2, 1.5) · osc 440 · fm 110 (3, 2) → a mixer (unity gains) → audio_out → master → limiter.
/// The same scene, as TOML, for the stage's `Engine`.
pub const STEREO_TOML: &str = r#"
[[node]]
id = "a"
kind = "osc"
freq = 220.0
gain = 0.2
[[node]]
id = "b"
kind = "fm"
freq = 330.0
ratio = 2.0
index = 1.5
gain = 0.2
[[node]]
id = "c"
kind = "osc"
freq = 440.0
gain = 0.15
[[node]]
id = "d"
kind = "fm"
freq = 110.0
ratio = 3.0
index = 2.0
gain = 0.2
[[node]]
id = "mix"
kind = "mixer"
gain1 = 1.0
gain2 = 1.0
gain3 = 1.0
gain4 = 1.0
[[node]]
id = "out"
kind = "audio_out"
volume = 1.0
[[link]]
from = "a"
to = "mix.in1"
[[link]]
from = "b"
to = "mix.in2"
[[link]]
from = "c"
to = "mix.in3"
[[link]]
from = "d"
to = "mix.in4"
[[link]]
from = "mix"
to = "out.in"
"#;

pub struct StereoScene {
    pool: Pool,
    sine: Batch,
    fm: Batch,
    gains: [Ramp; 4],
    fresh: bool,
    vol: Ramp,
    limiter: Limiter,
    mix: [Vec<f32>; 2],
}

impl StereoScene {
    /// `extra`: pool channels no node uses (the width of the pool must cost nothing)
    pub fn new(start_phase: f32, exp: Exp, extra: usize) -> StereoScene {
        // buffers in the stage's order: sine batch (a, c) = 0, 1; fm batch (b, d) = 2, 3; mixer 4; out 5 – two channels each
        let ch = |b: usize| [2 * b, 2 * b + 1];
        StereoScene {
            pool: Pool::new(12 + extra),
            sine: Batch::new(false, RATE, start_phase, exp, &[(VoiceSpec::osc(220.0, 0.2), ch(0)), (VoiceSpec::osc(440.0, 0.15), ch(1))]),
            fm: Batch::new(true, RATE, start_phase, exp, &[(VoiceSpec::fm(330.0, 2.0, 1.5, 0.2), ch(2)), (VoiceSpec::fm(110.0, 3.0, 2.0, 0.2), ch(3))]),
            gains: [GLIDE; 4].map(|t| Ramp::new(0.0, t, RATE)),
            fresh: true,
            vol: Ramp::new(0.0, GLIDE, RATE),
            limiter: Limiter::new(RATE),
            mix: [vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK]],
        }
    }

    /// `n` frames (≤ MAX_BLOCK), interleaved L R appended to `out`
    pub fn block(&mut self, n: usize, out: &mut Vec<f32>) {
        self.sine.process(&mut self.pool, n);
        self.fm.process(&mut self.pool, n);
        let fresh = std::mem::take(&mut self.fresh);
        // the mixer (buffer 4 = channels 8, 9): in1..in4 = buffers 0, 2, 1, 3
        let (m0, m1) = (8, 9);
        self.pool.clear(m0, n);
        self.pool.clear(m1, n);
        for (k, src) in [0usize, 2, 1, 3].into_iter().enumerate() {
            if fresh {
                self.gains[k].jump(1.0);
            }
            let (g, dg) = self.gains[k].block(1.0, n);
            for ch in 0..2 {
                let (s, d) = (2 * src + ch, m0 + ch);
                for i in 0..n {
                    let x = self.pool.ch(s)[i];
                    self.pool.ch_mut(d)[i] += x * (g + dg * i as f32);
                }
            }
        }
        // audio_out (buffer 5 = channels 10, 11): x * (v + dv * i)
        if fresh {
            self.vol.jump(1.0);
        }
        let (vv, dv) = self.vol.block(1.0, n);
        for ch in 0..2 {
            for i in 0..n {
                let x = self.pool.ch(m0 + ch)[i];
                self.pool.ch_mut(10 + ch)[i] = x * (vv + dv * i as f32);
            }
        }
        // the master (a ramp from the previous master to this one: both 1.0), the limiter, interleave
        for f in 0..n {
            let (from, to) = (1.0f32, 1.0f32);
            let master = from + (to - from) * (f + 1) as f32 / n as f32;
            self.mix[0][f] = self.pool.ch(10)[f] * master;
            self.mix[1][f] = self.pool.ch(11)[f] * master;
        }
        let [l, r] = &mut self.mix;
        self.limiter.process(&mut l[..n], &mut r[..n]);
        for f in 0..n {
            out.push(l[f]);
            out.push(r[f]);
        }
        let _ = m1;
    }
}

/// `frames` of the stereo scene in blocks of `block`, interleaved
pub fn run_stereo(frames: usize, block: usize, start_phase: f32, exp: Exp, extra: usize) -> Vec<f32> {
    let mut s = StereoScene::new(start_phase, exp, extra);
    let mut out = Vec::with_capacity(frames * 2);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        s.block(n, &mut out);
        done += n;
    }
    out
}

// ── scenario 1: the 8-channel routing graph (plan §2.6 / §3.8 P0d) ──────────────────────────────────────────────────

/// Speaker azimuths (degrees, 0 = front, clockwise) of the four-speaker ring.
pub const SPEAKERS: [f32; 4] = [45.0, 135.0, 225.0, 315.0];

/// 2-D VBAP (Pulkki): the gains of the adjacent speaker pair around azimuth `az` (degrees), power-normalised. Only
/// deck-core's libm-free `sin_turns` and IEEE sqrt / division: the same bits on every target.
pub fn vbap4(az: f32) -> [f32; 4] {
    let turns = |deg: f32| deg / 360.0;
    let sin = deck_core::math::sin_turns;
    let cos = |t: f32| deck_core::math::sin_turns(t + 0.25);
    let az = az.rem_euclid(360.0);
    let mut g = [0.0f32; 4];
    for i in 0..4 {
        let (a0, mut a1) = (SPEAKERS[i], SPEAKERS[(i + 1) % 4]);
        if a1 <= a0 {
            a1 += 360.0;
        }
        let mut a = az;
        if a < SPEAKERS[0] {
            a += 360.0;
        }
        if a >= a0 && a <= a1 {
            // the source direction p = (sin az, cos az) = g0 l0 + g1 l1: invert the 2×2 of the speakers' unit vectors
            let (l0x, l0y, l1x, l1y) = (sin(turns(a0)), cos(turns(a0)), sin(turns(a1)), cos(turns(a1)));
            let (px, py) = (sin(turns(a)), cos(turns(a)));
            let det = l0x * l1y - l1x * l0y;
            let (g0, g1) = ((px * l1y - l1x * py) / det, (l0x * py - px * l0y) / det);
            let norm = (g0 * g0 + g1 * g1).sqrt();
            g[i] = g0 / norm;
            g[(i + 1) % 4] = g1 / norm;
            break;
        }
    }
    g
}

/// ITU-R BS.775 down-mix of a 5.1 bus (L R C LFE Ls Rs) to stereo: Lo = L + C/√2 + Ls/√2 (the LFE is dropped)
pub const FOLD51: [[f32; 6]; 2] = [
    [1.0, 0.0, std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2, 0.0],
    [0.0, 1.0, std::f32::consts::FRAC_1_SQRT_2, 0.0, 0.0, std::f32::consts::FRAC_1_SQRT_2],
];

/// Pool layout of the routing scene (23 channels) and its device (8 channels).
pub mod route {
    pub const A: [usize; 2] = [0, 1]; // track A (osc), stereo
    pub const B: [usize; 2] = [2, 3]; // track B (fm), stereo
    pub const SEND: [usize; 4] = [4, 5, 6, 7]; // a 4-channel send bus
    pub const FOLDER: [usize; 2] = [8, 9]; // folder: A + B
    pub const MONO: usize = 10; // a mono source for VBAP
    pub const VBAP: [usize; 4] = [11, 12, 13, 14]; // its four speakers
    pub const BUS51: [usize; 6] = [15, 16, 17, 18, 19, 20]; // L R C LFE Ls Rs
    pub const FOLD51: [usize; 2] = [21, 22];
    pub const CHANNELS: usize = 23;
    pub const DEVICE: usize = 8;
    /// the send's gain, and the VBAP source's azimuth (degrees)
    pub const SEND_GAIN: f32 = 0.5;
    pub const AZIMUTH: f32 = 100.0;
}

pub struct RouteScene {
    pub pool: Pool,
    voices: Vec<Batch>,
    /// the device buffers (planar, 8)
    pub dev: Pool,
}

impl RouteScene {
    pub fn new(start_phase: f32, exp: Exp) -> RouteScene {
        use route::*;
        let mut voices = vec![
            Batch::new(false, RATE, start_phase, exp, &[(VoiceSpec::osc(220.0, 0.3), A), (VoiceSpec::osc(440.0, 0.1), [MONO, MONO])]),
            Batch::new(true, RATE, start_phase, exp, &[(VoiceSpec::fm(330.0, 2.0, 1.2, 0.3), B)]),
        ];
        // the 5.1 bus: six sines at distinct pitches, one per channel (a voice writes both of its dest channels, so a
        // 5.1 voice names its bus channel twice)
        let freqs = [262.0, 294.0, 330.0, 55.0, 392.0, 440.0];
        let six: Vec<(VoiceSpec, [usize; 2])> = freqs.iter().enumerate().map(|(i, f)| (VoiceSpec::osc(*f, 0.15), [BUS51[i], BUS51[i]])).collect();
        voices.push(Batch::new(false, RATE, start_phase, exp, &six));
        RouteScene { pool: Pool::new(CHANNELS), voices, dev: Pool::new(DEVICE) }
    }

    /// `n` frames; the device block is appended to `out` interleaved (8 per frame)
    pub fn block(&mut self, n: usize, out: &mut Vec<f32>) {
        use route::*;
        for v in &mut self.voices {
            v.process(&mut self.pool, n);
        }
        let p = &mut self.pool;
        // a send: A's channels 1/2 → the send bus channels 3/4 (the channel map), gain 0.5
        for c in SEND {
            p.clear(c, n);
        }
        p.add_scaled(A[0], SEND[2], SEND_GAIN, n);
        p.add_scaled(A[1], SEND[3], SEND_GAIN, n);
        // a folder summing two buses
        for (k, c) in FOLDER.into_iter().enumerate() {
            p.clear(c, n);
            p.add_scaled(A[k], c, 1.0, n);
            p.add_scaled(B[k], c, 1.0, n);
        }
        // VBAP: the mono source to four speakers
        let g = vbap4(AZIMUTH);
        for (k, c) in VBAP.into_iter().enumerate() {
            p.clear(c, n);
            p.add_scaled(MONO, c, g[k], n);
        }
        // 5.1 → stereo by the spec's matrix
        for (k, c) in FOLD51.into_iter().enumerate() {
            p.clear(c, n);
            for (s, w) in crate::FOLD51[k].iter().enumerate() {
                if *w != 0.0 {
                    p.add_scaled(BUS51[s], c, *w, n);
                }
            }
        }
        // the `out` nodes: folder → device 1/2, send bus channels 3/4 → device 3/4, VBAP → 5..8, the 5.1 fold added to 1/2
        let d = &mut self.dev;
        for c in 0..DEVICE {
            d.clear(c, n);
        }
        d.ch_mut(0)[..n].copy_from_slice(&p.ch(FOLDER[0])[..n]);
        d.ch_mut(1)[..n].copy_from_slice(&p.ch(FOLDER[1])[..n]);
        d.ch_mut(2)[..n].copy_from_slice(&p.ch(SEND[2])[..n]);
        d.ch_mut(3)[..n].copy_from_slice(&p.ch(SEND[3])[..n]);
        for k in 0..4 {
            d.ch_mut(4 + k)[..n].copy_from_slice(&p.ch(VBAP[k])[..n]);
        }
        for k in 0..2 {
            for f in 0..n {
                let x = p.ch(FOLD51[k])[f];
                d.ch_mut(k)[f] += x;
            }
        }
        for f in 0..n {
            for c in 0..DEVICE {
                out.push(d.ch(c)[f]);
            }
        }
    }
}

pub fn run_route(frames: usize, block: usize, start_phase: f32, exp: Exp) -> Vec<f32> {
    let mut s = RouteScene::new(start_phase, exp);
    let mut out = Vec::with_capacity(frames * route::DEVICE);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        s.block(n, &mut out);
        done += n;
    }
    out
}

// ── the saws: PolyBLEP and a mip-mapped wavetable (plan §3.2 / P0d) ─────────────────────────────────────────────────

#[inline(always)]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// A saw of unit amplitude (−1..1): the naive ramp minus the PolyBLEP residual.
pub fn saw_polyblep(freq: f32, frames: usize) -> Vec<f32> {
    let dt = freq / RATE;
    let mut ph = 0.0f32;
    (0..frames)
        .map(|_| {
            let y = 2.0 * ph - 1.0 - poly_blep(ph, dt);
            ph += dt;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            y
        })
        .collect()
}

pub const TABLE: usize = 2048;

/// Band-limited saw tables, one per octave from 40 Hz: table k holds the harmonics under Nyquist for its top frequency.
/// Computed at load with deck-core's libm-free sine (the same bits everywhere).
pub struct SawTables {
    tables: Vec<Vec<f32>>,
}

impl SawTables {
    pub fn new() -> SawTables {
        let mut tables = Vec::new();
        let mut top = 40.0f32;
        while top / 2.0 < RATE / 2.0 {
            let harmonics = ((RATE / 2.0) / top).floor() as usize;
            let mut t = vec![0.0f32; TABLE + 1];
            for (i, x) in t.iter_mut().enumerate().take(TABLE) {
                let ph = i as f32 / TABLE as f32;
                let mut s = 0.0f32;
                for k in 1..=harmonics {
                    let q = ph * k as f32;
                    s += deck_core::math::sin_turns(q - q.floor()) / k as f32;
                }
                *x = -s * (2.0 / std::f32::consts::PI);
            }
            t[TABLE] = t[0];
            tables.push(t);
            top *= 2.0;
        }
        SawTables { tables }
    }

    /// the saw of `freq`: the table of its octave, linear interpolation
    pub fn render(&self, freq: f32, frames: usize) -> Vec<f32> {
        let mut k = 0;
        let mut top = 40.0f32;
        while freq > top && k + 1 < self.tables.len() {
            top *= 2.0;
            k += 1;
        }
        let t = &self.tables[k];
        let dt = freq / RATE;
        let mut ph = 0.0f32;
        (0..frames)
            .map(|_| {
                let x = ph * TABLE as f32;
                let i = x as usize;
                let fr = x - i as f32;
                let y = t[i] + (t[i + 1] - t[i]) * fr;
                ph += dt;
                if ph >= 1.0 {
                    ph -= 1.0;
                }
                y
            })
            .collect()
    }
}

impl Default for SawTables {
    fn default() -> Self {
        Self::new()
    }
}

// ── the wasm exports (also callable natively) ───────────────────────────────────────────────────────────────────────

/// The saws' test pitch: bin 211 of a 16384-point FFT at 48 kHz (a coherent window)
pub const SAW_HZ: f32 = 211.0 * 48_000.0 / 16_384.0;

/// Scenario numbers of `lab_run`: 0 stereo, 1 routing, 2 polyblep saw, 3 wavetable saw.
pub fn scenario_run(scenario: u32, frames: usize, block: usize, start_phase: f32, exp: Exp) -> Vec<f32> {
    match scenario {
        0 => run_stereo(frames, block, start_phase, exp, 0),
        1 => run_route(frames, block, start_phase, exp),
        2 => saw_polyblep(SAW_HZ, frames),
        3 => SawTables::new().render(SAW_HZ, frames),
        _ => Vec::new(),
    }
}

type Bench = (Batch, Pool, Limiter, Vec<f32>);

thread_local! {
    static OUT: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static BENCH: RefCell<Option<Bench>> = const { RefCell::new(None) };
}

/// Render a scenario; returns the number of f32 in the output, at `lab_out()`. `libm_exp` 1 = the pure-Rust libm.
#[no_mangle]
pub extern "C" fn lab_run(scenario: u32, frames: u32, block: u32, start_phase: f32, libm_exp: u32) -> u32 {
    let exp = if libm_exp == 1 { Exp::Libm } else { Exp::Std };
    let out = scenario_run(scenario, frames as usize, block as usize, start_phase, exp);
    OUT.with(|o| {
        *o.borrow_mut() = out;
        o.borrow().len() as u32
    })
}

#[no_mangle]
pub extern "C" fn lab_out() -> *const f32 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// The FM16 bench: 16 fm voices in one batch (the plan's speed probe); `lab_bench_block` renders one block.
#[no_mangle]
pub extern "C" fn lab_bench_init(start_phase: f32) {
    let voices: Vec<(VoiceSpec, [usize; 2])> = (0..16).map(|i| (VoiceSpec::fm(110.0 + 13.0 * i as f32, 2.0, 1.5, 0.05), [2 * i, 2 * i + 1])).collect();
    BENCH.with(|b| *b.borrow_mut() = Some((Batch::new(true, RATE, start_phase, Exp::Std, &voices), Pool::new(32), Limiter::new(RATE), vec![0.0; MAX_BLOCK])));
}

/// One block of the bench; `full` 1 = also what live-core's render does after the voices (the sum of the 16 voices into
/// one master and deck-core's mono limiter), 0 = the voice batch alone. Returns a sample (the work cannot be optimised away).
#[no_mangle]
pub extern "C" fn lab_bench_block(frames: u32, full: u32) -> f32 {
    BENCH.with(|b| {
        let mut b = b.borrow_mut();
        let (batch, pool, limiter, master) = b.as_mut().expect("lab_bench_init first");
        let n = frames as usize;
        batch.process(pool, n);
        if full == 1 {
            master[..n].fill(0.0);
            for i in 0..16 {
                for (m, x) in master[..n].iter_mut().zip(pool.ch(2 * i)) {
                    *m += x;
                }
            }
            limiter.process_mono(&mut master[..n]);
            return master[n - 1];
        }
        pool.ch(0)[n - 1]
    })
}
