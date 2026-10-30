//! The batched audio kinds (#183, the engine model): one level's sine oscillators, or its FM voices, run as ONE batch –
//! their state and per-block params in arrays (struct of arrays), a block = per frame one loop over the voices with no
//! loop-carried value, which the compiler vectorises (4 lanes baseline, 8 with AVX2). Per block: the params read once,
//! each voice's pitch and gate gathered into a frame-major matrix (from its `cv` link or its `freq` with the gate open),
//! the loop, then each voice's column copied out to its own output buffer. deck-core's sine and phase wrap – no libm.
//!
//! Same sound as the blocks they replace (blocks.rs: fundsp's `sine()` + our envelope): the phases start where fundsp's
//! sine starts and step by the same seconds-per-sample; only the sine is deck-core's polynomial (< 1e-5).
//! `tests` compare the two paths.
use super::{Param, Stereo, MAX_BLOCK};
use crate::graph::{Kind, Node};
use deck_core::math::{sin_turns, wrap01};

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

pub(super) struct Voices {
    kind: VoiceKind,
    /// the first output buffer: voice i writes buffer `out + i`
    pub out: usize,
    /// per voice: freq, gain, ratio, index (ratio and index: FM only)
    params: [Vec<Param>; 4],
    /// per voice: the buffer its `cv` input reads (pitch left, gate right)
    cv: Vec<Option<usize>>,
    /// seconds per sample, as fundsp steps its phases
    sd: f32,
    avx2: bool,
    /// the carrier's (or the oscillator's) phase, the modulator's phase, the envelope level and its two coefficients
    c: Vec<f32>,
    m: Vec<f32>,
    env: Vec<f32>,
    att: Vec<f32>,
    rel: Vec<f32>,
    /// this block's params, read once
    p: [Vec<f32>; 4],
    /// this block's pitch, gate and output, frame-major: frame s, voice i at `s * n + i`
    f: Vec<f32>,
    gate: Vec<f32>,
    y: Vec<f32>,
}

impl Voices {
    pub(super) fn new(kind: VoiceKind, out: usize, rate: f32, avx2: bool) -> Voices {
        Voices {
            kind,
            out,
            params: Default::default(),
            cv: vec![],
            sd: (1.0 / rate as f64) as f32,
            avx2: avx2 && avx2_available(),
            c: vec![],
            m: vec![],
            env: vec![],
            att: vec![],
            rel: vec![],
            p: Default::default(),
            f: vec![],
            gate: vec![],
            y: vec![],
        }
    }

    /// Add a voice: its params (freq, gain, ratio, index), its cv buffer, its envelope coefficients (attack, release).
    pub(super) fn push(&mut self, params: [Param; 4], cv: Option<usize>, (att, rel): (f32, f32)) {
        for (list, p) in self.params.iter_mut().zip(params) {
            list.push(p);
        }
        self.cv.push(cv);
        let start = start_phase();
        self.c.push(start);
        self.m.push(start);
        self.env.push(0.0);
        self.att.push(att);
        self.rel.push(rel);
    }

    /// Allocate the per-block arrays – after the last `push`, before the callback runs.
    pub(super) fn finish(&mut self) {
        let n = self.len();
        self.p = std::array::from_fn(|_| vec![0.0; n]);
        for v in [&mut self.f, &mut self.gate, &mut self.y] {
            *v = vec![0.0; n * MAX_BLOCK];
        }
    }

    pub(super) fn len(&self) -> usize {
        self.c.len()
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
        let n = self.len();
        for (p, params) in self.p.iter_mut().zip(&self.params) {
            for (x, param) in p.iter_mut().zip(params) {
                *x = param.get();
            }
        }
        for i in 0..n {
            match self.cv[i] {
                Some(b) => {
                    for s in 0..frames {
                        self.f[s * n + i] = before[b].l[s];
                        self.gate[s * n + i] = before[b].r[s];
                    }
                }
                None => {
                    for s in 0..frames {
                        self.f[s * n + i] = self.p[0][i];
                        self.gate[s * n + i] = 1.0;
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
        for (i, o) in outs.iter_mut().enumerate().take(n) {
            for s in 0..frames {
                let v = self.y[s * n + i];
                o.l[s] = v;
                o.r[s] = v;
            }
        }
    }

    /// The same loops compiled for AVX2 (8 lanes); results bit-identical to the baseline (no fused multiply-adds: Rust
    /// never contracts `a * b + c`).
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn kernel_avx2(&mut self, frames: usize) {
        self.kernel(frames)
    }

    /// Per frame, one loop over the voices – element-wise, no value carried from voice to voice, so it vectorises.
    #[inline(always)]
    fn kernel(&mut self, frames: usize) {
        let n = self.len();
        let sd = self.sd;
        let (c, m, env) = (&mut self.c[..n], &mut self.m[..n], &mut self.env[..n]);
        let (att, rel) = (&self.att[..n], &self.rel[..n]);
        let [_, gain, ratio, index] = &self.p;
        let (gain, ratio, index) = (&gain[..n], &ratio[..n], &index[..n]);
        for s in 0..frames {
            let f = &self.f[s * n..][..n];
            let g = &self.gate[s * n..][..n];
            let y = &mut self.y[s * n..][..n];
            match self.kind {
                VoiceKind::Sine => {
                    for i in 0..n {
                        let o = sin_turns(c[i]);
                        c[i] = wrap01(c[i] + f[i] * sd);
                        y[i] = o * envelope(&mut env[i], g[i], att[i], rel[i]) * gain[i];
                    }
                }
                VoiceKind::Fm => {
                    for i in 0..n {
                        let fm = f[i] * ratio[i];
                        let mo = sin_turns(m[i]);
                        m[i] = wrap01(m[i] + fm * sd);
                        let co = sin_turns(c[i]);
                        c[i] = wrap01(c[i] + (f[i] + mo * index[i] * fm).abs() * sd);
                        y[i] = co * envelope(&mut env[i], g[i], att[i], rel[i]) * gain[i];
                    }
                }
            }
        }
    }
}

/// blocks.rs's `Env::next` without branches: toward 1 while the gate is open, toward 0 after
#[inline(always)]
fn envelope(level: &mut f32, gate: f32, att: f32, rel: f32) -> f32 {
    let target = if gate > 0.5 { 1.0 } else { 0.0 };
    let k = if target > *level { att } else { rel };
    *level += (target - *level) * k;
    *level
}
