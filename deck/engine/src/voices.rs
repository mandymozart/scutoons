//! The batched audio kinds (#183, the engine model): one level's sine oscillators, or its FM voices, run as ONE batch.
//! The voices sit in groups of eight (struct of arrays: one lane per voice, a short last group padded with silent
//! lanes); a block runs group by group, the group's state in registers for the whole block, per frame one step of
//! deck-core's `F8` vector ops (2 × 4 lanes on baseline x86-64, 1 × 8 in the AVX2 copy). Per block: the params read
//! once, each voice's pitch and gate gathered into its lane (from its `cv` link, or its `freq` with the gate open), the
//! loop, each lane copied out to its voice's output buffer. deck-core's sine and phase wrap – no libm.
//!
//! Same sound as the blocks they replace (blocks.rs: the patcher build's scalar sine / FM + the same envelope): the
//! phases start at `START_PHASE` and step by the same seconds-per-sample, the sine is deck-core's polynomial. lib.rs's
//! tests compare the two paths.
use super::{Buf, Param, QUANTUM};
use deck_space::graph::{Kind, Node};
use deck_core::dsp::START_PHASE;
use deck_core::lanes::{self as v, F8, LANES};
use deck_core::smooth::{Ramp, GLIDE, QUICK};

type Lanes = [f32; LANES];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VoiceKind {
    /// `osc` with `wave = "sine"`
    Sine,
    /// `fm`: true frequency modulation – a sine modulator at freq × ratio bends the carrier's frequency by index × that
    /// (negative frequency runs the phase backward, no fold)
    Fm,
}

impl VoiceKind {
    /// The kinds that batch; every other audio node stays a block of its own (the patcher model).
    pub(super) fn of(node: &Node) -> Option<VoiceKind> {
        match node.kind {
            Kind::Osc if node.str("wave").unwrap_or("sine") == "sine" => Some(VoiceKind::Sine),
            Kind::Fm => Some(VoiceKind::Fm),
            _ => None,
        }
    }
}

/// Whether this CPU runs the AVX2 loops (x86-64-v3's vector part: AVX2 + FMA). Checked once, at build.
pub fn avx2_available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// One group of eight voices: state, envelope coefficients and this block's params, a lane each.
#[derive(Clone, Copy, Default)]
struct Group {
    /// the carrier's (or the oscillator's) phase, the modulator's phase, the envelope level
    c: Lanes,
    m: Lanes,
    env: Lanes,
    att: Lanes,
    rel: Lanes,
    /// freq, gain, ratio, index – read once per block: the first value and the step per frame (#188: each glides
    /// across the block, deck-core's smooth.rs)
    p: [Lanes; 4],
    d: [Lanes; 4],
}

pub(super) struct Voices {
    kind: VoiceKind,
    /// the first output buffer: voice i writes buffer `out + i`
    pub out: usize,
    /// per voice: freq, gain, ratio, index (ratio and index: FM only) and their smoothers
    params: Vec<[Param; 4]>,
    ramps: Vec<[Ramp; 4]>,
    /// the smoothers start where the first block's params are
    fresh: bool,
    rate: f32,
    /// per voice: the buffer its `cv` input reads (pitch left, gate right)
    cv: Vec<Option<usize>>,
    /// seconds per sample
    sd: f32,
    avx2: bool,
    groups: Vec<Group>,
    /// this block's pitch, gate and output per group and frame: group g, frame s at `g * QUANTUM + s`
    f: Vec<Lanes>,
    gate: Vec<Lanes>,
    y: Vec<Lanes>,
}

impl Voices {
    pub(super) fn new(kind: VoiceKind, out: usize, rate: f32, avx2: bool) -> Voices {
        Voices {
            kind,
            out,
            params: vec![],
            ramps: vec![],
            fresh: true,
            rate,
            cv: vec![],
            sd: (1.0 / rate as f64) as f32,
            avx2: avx2 && avx2_available(),
            groups: vec![],
            f: vec![],
            gate: vec![],
            y: vec![],
        }
    }

    /// Add a voice: its params (freq, gain, ratio, index), its cv buffer, its envelope coefficients (attack, release).
    pub(super) fn push(&mut self, params: [Param; 4], cv: Option<usize>, (att, rel): (f32, f32)) {
        let (g, l) = (self.len() / LANES, self.len() % LANES);
        if l == 0 {
            self.groups.push(Group::default());
        }
        let group = &mut self.groups[g];
        (group.c[l], group.m[l]) = (START_PHASE, START_PHASE);
        (group.att[l], group.rel[l]) = (att, rel);
        self.params.push(params);
        self.ramps.push([GLIDE, GLIDE, QUICK, QUICK].map(|t| Ramp::new(0.0, t, self.rate)));
        self.cv.push(cv);
    }

    /// Allocate the per-block arrays – after the last `push`, before the callback runs.
    pub(super) fn finish(&mut self) {
        let n = self.groups.len() * QUANTUM;
        for v in [&mut self.f, &mut self.gate, &mut self.y] {
            *v = vec![[0.0; LANES]; n];
        }
    }

    pub(super) fn len(&self) -> usize {
        self.params.len()
    }

    pub(super) fn label(&self) -> &'static str {
        if self.avx2 {
            "avx2"
        } else {
            "baseline"
        }
    }

    /// One block: `before` = every buffer of the earlier steps (the cv sources), `outs` = this batch's voices.
    /// Never allocates.
    pub(super) fn process(&mut self, before: &[Buf], outs: &mut [Buf], frames: usize) {
        let fresh = std::mem::take(&mut self.fresh);
        for (i, ((params, cv), ramps)) in self.params.iter().zip(&self.cv).zip(&mut self.ramps).enumerate() {
            let (g, l) = (i / LANES, i % LANES);
            let group = &mut self.groups[g];
            for (k, (param, ramp)) in params.iter().zip(ramps.iter_mut()).enumerate() {
                let x = param.get();
                if fresh {
                    ramp.jump(x);
                }
                (group.p[k][l], group.d[k][l]) = ramp.block(x, frames);
            }
            let (f, gate) = (&mut self.f[g * QUANTUM..][..frames], &mut self.gate[g * QUANTUM..][..frames]);
            match *cv {
                Some(b) => {
                    for (s, (f, gate)) in f.iter_mut().zip(gate).enumerate() {
                        f[l] = before[b].ch(0)[s];
                        gate[l] = before[b].ch(1)[s];
                    }
                }
                None => {
                    let (f0, df) = (group.p[0][l], group.d[0][l]);
                    for (s, (f, gate)) in f.iter_mut().zip(gate).enumerate() {
                        f[l] = f0 + df * s as f32;
                        gate[l] = 1.0;
                    }
                }
            }
        }
        #[cfg(target_arch = "x86_64")]
        if self.avx2 {
            // SAFETY: `avx2` is only set when the CPU has AVX2 and FMA (`avx2_available`, checked in `new`)
            unsafe { self.kernel_avx2(frames) };
        } else {
            self.kernel(frames);
        }
        #[cfg(not(target_arch = "x86_64"))]
        self.kernel(frames);
        for (i, o) in outs.iter_mut().enumerate().take(self.len()) {
            let (g, l) = (i / LANES, i % LANES);
            for (s, y) in self.y[g * QUANTUM..][..frames].iter().enumerate() {
                o.put2(s, y[l], y[l]);
            }
        }
    }

    /// The same loops compiled for AVX2 (8 lanes in one register); results bit-identical to the baseline (no fused
    /// multiply-adds: Rust never contracts `a * b + c`).
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn kernel_avx2(&mut self, frames: usize) {
        self.kernel(frames)
    }

    #[inline(always)]
    fn kernel(&mut self, frames: usize) {
        for (g, group) in self.groups.iter_mut().enumerate() {
            let r = g * QUANTUM..g * QUANTUM + frames;
            run(self.kind, group, &self.f[r.clone()], &self.gate[r.clone()], &mut self.y[r], self.sd);
        }
    }
}

/// One group over a block, its state in registers: per frame the voice step, then blocks.rs's envelope (toward 1 while
/// the gate is open, toward 0 after) – all lane-wise, without branches. One loop per kind (no match per frame).
#[inline(always)]
fn run(kind: VoiceKind, group: &mut Group, f: &[Lanes], gate: &[Lanes], y: &mut [Lanes], sd: f32) {
    let (mut c, mut m, mut env) = (F8(group.c), F8(group.m), F8(group.env));
    let (att, rel) = (F8(group.att), F8(group.rel));
    // gain, ratio, index at frame s: first + step · s (they glide across the block – blocks.rs's arithmetic)
    let [_, g0, r0, k0] = group.p.map(F8);
    let [_, dg, dr, dk] = group.d.map(F8);
    let (sd, half, one, zero) = (F8::splat(sd), F8::splat(0.5), F8::splat(1.0), F8::splat(0.0));
    let mut envelope = |gate: &Lanes| {
        let target = half.lt_select(F8(*gate), one, zero);
        env = env + (target - env) * env.lt_select(target, att, rel);
        env
    };
    let frames = f.iter().zip(gate).zip(y).enumerate();
    match kind {
        VoiceKind::Sine => {
            for (s, ((f, gate), y)) in frames {
                let s = F8::splat(s as f32);
                let o = v::sin_turns(c);
                c = v::wrap01(c + F8(*f) * sd);
                *y = (o * envelope(gate) * (g0 + dg * s)).0;
            }
        }
        VoiceKind::Fm => {
            for (s, ((f, gate), y)) in frames {
                let s = F8::splat(s as f32);
                let f = F8(*f);
                let fm = f * (r0 + dr * s);
                let mo = v::sin_turns(m);
                m = v::wrap01(m + fm * sd);
                let o = v::sin_turns(c);
                c = v::wrap01(c + (f + mo * (k0 + dk * s) * fm) * sd);
                *y = (o * envelope(gate) * (g0 + dg * s)).0;
            }
        }
    }
    (group.c, group.m, group.env) = (c.0, m.0, env.0);
}
