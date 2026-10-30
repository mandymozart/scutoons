//! The batched audio kinds (#183, the engine model): one level's sine oscillators, or its FM voices, run as ONE batch.
//! The voices sit in groups of eight (struct of arrays: one lane per voice, a short last group padded with silent
//! lanes); a block runs group by group, the group's state in registers for the whole block, per frame one step of
//! deck-core's `F8` vector ops (2 × 4 lanes on baseline x86-64, 1 × 8 in the AVX2 copy). Per block: the params read
//! once, each voice's pitch and gate gathered into its lane (from its `cv` link, or its `freq` with the gate open), the
//! loop, each lane copied out to its voice's output buffer. deck-core's sine and phase wrap – no libm.
//!
//! Same sound as the blocks they replace (blocks.rs: fundsp's `sine()` + our envelope): the phases start where fundsp's
//! sine starts and step by the same seconds-per-sample; only the sine is deck-core's polynomial (< 2e-7).
//! audio/mod.rs's tests compare the two paths.
use super::{Param, Stereo, MAX_BLOCK};
use crate::graph::{Kind, Node};
use deck_core::lanes::{self as v, F8, LANES};

type Lanes = [f32; LANES];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VoiceKind {
    /// `osc` with `wave = "sine"`
    Sine,
    /// `fm`: a sine modulator at freq × ratio bending the carrier's frequency by index × that
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

/// Where fundsp's `sine()` starts its phase (its hash 0) – the batch starts there too.
fn start_phase() -> f32 {
    fundsp::math::rnd1(0) as f32
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
    /// freq, gain, ratio, index – read once per block
    p: [Lanes; 4],
}

pub(super) struct Voices {
    kind: VoiceKind,
    /// the first output buffer: voice i writes buffer `out + i`
    pub out: usize,
    /// per voice: freq, gain, ratio, index (ratio and index: FM only)
    params: Vec<[Param; 4]>,
    /// per voice: the buffer its `cv` input reads (pitch left, gate right)
    cv: Vec<Option<usize>>,
    /// seconds per sample, as fundsp steps its phases
    sd: f32,
    avx2: bool,
    groups: Vec<Group>,
    /// this block's pitch, gate and output per group and frame: group g, frame s at `g * MAX_BLOCK + s`
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
        (group.c[l], group.m[l]) = (start_phase(), start_phase());
        (group.att[l], group.rel[l]) = (att, rel);
        self.params.push(params);
        self.cv.push(cv);
    }

    /// Allocate the per-block arrays – after the last `push`, before the callback runs.
    pub(super) fn finish(&mut self) {
        let n = self.groups.len() * MAX_BLOCK;
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
    pub(super) fn process(&mut self, before: &[Stereo], outs: &mut [Stereo], frames: usize) {
        for (i, (params, cv)) in self.params.iter().zip(&self.cv).enumerate() {
            let (g, l) = (i / LANES, i % LANES);
            let group = &mut self.groups[g];
            for (p, param) in group.p.iter_mut().zip(params) {
                p[l] = param.get();
            }
            let (f, gate) = (&mut self.f[g * MAX_BLOCK..][..frames], &mut self.gate[g * MAX_BLOCK..][..frames]);
            match *cv {
                Some(b) => {
                    for (s, (f, gate)) in f.iter_mut().zip(gate).enumerate() {
                        f[l] = before[b].l[s];
                        gate[l] = before[b].r[s];
                    }
                }
                None => {
                    for (f, gate) in f.iter_mut().zip(gate) {
                        f[l] = group.p[0][l];
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
            for (s, y) in self.y[g * MAX_BLOCK..][..frames].iter().enumerate() {
                (o.l[s], o.r[s]) = (y[l], y[l]);
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
            let r = g * MAX_BLOCK..g * MAX_BLOCK + frames;
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
    let [_, gain, ratio, index] = group.p.map(F8);
    let (sd, half, one, zero) = (F8::splat(sd), F8::splat(0.5), F8::splat(1.0), F8::splat(0.0));
    let mut envelope = |gate: &Lanes| {
        let target = half.lt_select(F8(*gate), one, zero);
        env = env + (target - env) * env.lt_select(target, att, rel);
        env
    };
    let frames = f.iter().zip(gate).zip(y);
    match kind {
        VoiceKind::Sine => {
            for ((f, gate), y) in frames {
                let o = v::sin_turns(c);
                c = v::wrap01(c + F8(*f) * sd);
                *y = (o * envelope(gate) * gain).0;
            }
        }
        VoiceKind::Fm => {
            for ((f, gate), y) in frames {
                let f = F8(*f);
                let fm = f * ratio;
                let mo = v::sin_turns(m);
                m = v::wrap01(m + fm * sd);
                let o = v::sin_turns(c);
                c = v::wrap01(c + (f + mo * index * fm).abs() * sd);
                *y = (o * envelope(gate) * gain).0;
            }
        }
    }
    (group.c, group.m, group.env) = (c.0, m.0, env.0);
}
