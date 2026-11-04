//! One voice: the whole chain of the schema – Osc 1 / Osc 2 (+ their subs) / noise → routing → Filter 1 / Filter 2 →
//! Amp 1 / Amp 2 → stereo – with its two LFOs and four envelopes. Modulation is read every sample; what is costly (the
//! pitch, the cutoffs, the pan) is recomputed every `CTRL` samples of the voice's own clock, so a render does not
//! depend on how the host cut the audio into blocks.
use crate::dsp::adsr::{Adsr, Times};
use crate::dsp::lfo::{self, Lfo, Settings as LfoSettings, Shape};
use crate::dsp::math::{exp2, log2, C4};
use crate::dsp::noise::{mix, Noise};
use crate::dsp::osc::{Osc, Sub, Wave};
use crate::dsp::pan;
use crate::dsp::svf::{self, Coef, Mode, Svf};
use crate::params::Patch;

/// samples between the control-rate updates (0.33 ms at 48 kHz)
pub const CTRL: u32 = 16;
/// samples a stolen voice takes to fade out before the new note starts (1 ms)
pub const FADE: u32 = 48;

/// a note on its way into a voice
#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub hz: f32,
    pub vel: f32,
    /// the note event's id: its note off and bends find the voice by it
    pub id: u32,
    /// the note counter: smaller = older
    pub age: u64,
    pub seed: u64,
}

fn fe_times(p: &Patch, n: usize) -> Times {
    if n == 0 {
        Times { attack: p.fe1_a, decay: p.fe1_d, sustain: p.fe1_s, release: p.fe1_r }
    } else {
        Times { attack: p.fe2_a, decay: p.fe2_d, sustain: p.fe2_s, release: p.fe2_r }
    }
}

fn ae_times(p: &Patch, n: usize) -> Times {
    if n == 0 {
        Times { attack: p.ae1_a, decay: p.ae1_d, sustain: p.ae1_s, release: p.ae1_r }
    } else {
        Times { attack: p.ae2_a, decay: p.ae2_d, sustain: p.ae2_s, release: p.ae2_r }
    }
}

fn lfo_settings(p: &Patch, n: usize) -> LfoSettings {
    let (shape, rate_hz, sync, delay, attack) = if n == 0 {
        (p.l1_shape, p.l1_rate, p.l1_sync, p.l1_delay, p.l1_attack)
    } else {
        (p.l2_shape, p.l2_rate, p.l2_sync, p.l2_delay, p.l2_attack)
    };
    LfoSettings { shape: Shape::from_param(shape), rate_hz, beats: lfo::beats_per_cycle(sync), delay, attack }
}

#[derive(Default)]
pub struct Voice {
    active: bool,
    /// the note counter of the note it plays (steal the smallest)
    age: u64,
    /// the note that holds it (its id; none once released)
    id: Option<u32>,
    /// the note's own pitch (a bend moves from here)
    base: f32,
    /// the gate is gone but the sustain pedal holds it
    sustained: bool,
    hz: f32,
    vel: f32,
    ctr: u32,
    osc: [Osc; 2],
    sub: [Sub; 2],
    noise: Noise,
    svf: [Svf; 2],
    coef: [Coef; 2],
    fenv: [Adsr; 2],
    aenv: [Adsr; 2],
    lfo: [Lfo; 2],
    lfo_v: [f32; 2],
    fe_v: [f32; 2],
    inc: [f64; 2],
    pw: [f64; 2],
    amp: [f32; 2],
    pan: [(f32, f32); 2],
    /// a note waiting for this voice's fade-out to finish (the voice was stolen)
    pending: Option<Note>,
    /// the pending note's gate fell before it began
    cancel: bool,
    fade: u32,
}

impl Voice {
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn age(&self) -> u64 {
        self.age
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    /// the pitch the note started with (Hz)
    pub fn base(&self) -> f32 {
        self.base
    }

    pub fn sustained(&self) -> bool {
        self.sustained
    }

    pub fn start(&mut self, n: Note, p: &Patch, rate: f64) {
        *self = Voice::default();
        self.active = true;
        self.age = n.age;
        self.id = Some(n.id);
        self.hz = n.hz.clamp(8.0, 12_000.0);
        self.base = self.hz;
        self.vel = n.vel.clamp(0.0, 1.0);
        self.noise = Noise::new(mix(n.seed));
        self.lfo[0].start(mix(n.seed ^ 1));
        self.lfo[1].start(mix(n.seed ^ 2));
        for k in 0..2 {
            self.fenv[k].gate_on(&fe_times(p, k), rate);
            self.aenv[k].gate_on(&ae_times(p, k), rate);
        }
    }

    /// taken over by a newer note: fades out for `FADE` samples, then the note starts in its place
    pub fn steal(&mut self, n: Note) {
        self.pending = Some(n);
        self.cancel = false;
        self.fade = FADE;
        self.age = n.age;
        self.id = Some(n.id);
        self.base = n.hz.clamp(8.0, 12_000.0);
        self.sustained = false;
    }

    /// a legato pitch change (a note on for a sounding id, a bend)
    pub fn set_pitch(&mut self, hz: f32) {
        let hz = hz.clamp(8.0, 12_000.0);
        match &mut self.pending {
            Some(n) => n.hz = hz,
            None => self.hz = hz,
        }
    }

    /// the note off (or the pedal came up): release both envelopes
    pub fn release(&mut self, p: &Patch, rate: f64) {
        self.id = None;
        self.sustained = false;
        if self.pending.is_some() {
            self.cancel = true;
            return;
        }
        for k in 0..2 {
            self.fenv[k].gate_off(&fe_times(p, k), rate);
            self.aenv[k].gate_off(&ae_times(p, k), rate);
        }
    }

    /// the note off came but the pedal holds the note
    pub fn hold(&mut self) {
        self.id = None;
        self.sustained = true;
    }

    /// control rate: everything that needs `exp2`, `log2` or `tan`
    fn update(&mut self, p: &Patch, rate: f64) {
        let lfo = [f64::from(self.lfo_v[0]), f64::from(self.lfo_v[1])];
        let fe = [f64::from(self.fe_v[0]), f64::from(self.fe_v[1])];
        let vel = f64::from(self.vel);
        let oct = log2(f64::from(self.hz)) - log2(C4); // octaves above middle C
        let common = lfo[0] * f64::from(p.l1_pitch) + lfo[1] * f64::from(p.l2_pitch) + vel * f64::from(p.vel_pitch);
        let pwm = lfo[0] * f64::from(p.l1_pwm) + lfo[1] * f64::from(p.l2_pwm) + vel * f64::from(p.vel_pwm);
        let oscs = [(p.o1_semi, p.o1_fine, p.o1_key, p.o1_pw), (p.o2_semi, p.o2_fine, p.o2_key, p.o2_pw)];
        // oscillator 2 alone: its own envelope and LFO sends (they move it against a synced oscillator 1)
        let only2 = [0.0, fe[1] * f64::from(p.fe2_p2) + lfo[0] * f64::from(p.l1_p2) + lfo[1] * f64::from(p.l2_p2)];
        for (n, &(semi, fine, key, width)) in oscs.iter().enumerate() {
            let st = f64::from(semi) + f64::from(fine) / 100.0 + common + only2[n] + 12.0 * oct * f64::from(key);
            self.inc[n] = C4 * exp2(st / 12.0) / rate;
            self.pw[n] = (f64::from(width) + pwm).clamp(0.02, 0.98);
        }
        let filters = [
            (p.f1_cutoff, p.f1_res, p.f1_key, p.f1_env, p.f1_vel, p.f1_velres, p.l1_f1, p.l2_f1),
            (p.f2_cutoff, p.f2_res, p.f2_key, p.f2_env, p.f2_vel, p.f2_velres, p.l1_f2, p.l2_f2),
        ];
        for (n, &(cut, res, key, env, fvel, velres, l1, l2)) in filters.iter().enumerate() {
            let octaves =
                f64::from(key) * oct + 6.0 * f64::from(env) * fe[n] + 4.0 * f64::from(fvel) * vel + f64::from(l1) * lfo[0] + f64::from(l2) * lfo[1];
            let res = (f64::from(res) + f64::from(velres) * vel).clamp(0.0, 1.0) as f32;
            self.coef[n] = Coef::new(f64::from(cut) * exp2(octaves), res, rate);
        }
        let amps = [(p.a1_level, p.a1_pan, p.a1_vel, p.a1_key, p.l1_a1, p.l2_a1), (p.a2_level, p.a2_pan, p.a2_vel, p.a2_key, p.l1_a2, p.l2_a2)];
        for (n, &(level, pn, avel, key, t1, t2)) in amps.iter().enumerate() {
            let by_vel = (1.0 - f64::from(avel)) + f64::from(avel) * vel;
            let by_key = exp2(f64::from(key) * oct * 0.5).clamp(0.1, 4.0);
            let trem = (1.0 + 0.5 * (f64::from(t1) * (lfo[0] - 1.0) + f64::from(t2) * (lfo[1] - 1.0))).max(0.0);
            self.amp[n] = (f64::from(level) * by_vel * by_key * trem) as f32;
            self.pan[n] = pan::gains(pn);
        }
    }

    /// One sample at the absolute `beat`: (left, right). The voice frees itself when both amp envelopes are done.
    pub fn render(&mut self, p: &Patch, rate: f64, beat: f64) -> (f32, f32) {
        let mut gain = 1.0;
        if let Some(n) = self.pending {
            self.fade -= 1;
            if self.fade == 0 {
                let cancel = self.cancel;
                self.start(n, p, rate);
                if cancel {
                    self.release(p, rate);
                }
            } else {
                gain = self.fade as f32 / FADE as f32;
            }
        }
        if self.ctr % CTRL == 0 {
            self.update(p, rate);
        }
        self.ctr = self.ctr.wrapping_add(1);

        for k in 0..2 {
            self.lfo_v[k] = self.lfo[k].tick(&lfo_settings(p, k), rate, beat);
            self.fe_v[k] = self.fenv[k].tick(&fe_times(p, k), rate);
        }
        let ae = [self.aenv[0].tick(&ae_times(p, 0), rate), self.aenv[1].tick(&ae_times(p, 1), rate)];

        let t1 = self.osc[0].tick(Wave::from_param(p.o1_wave), self.inc[0], self.pw[0], None);
        let master = if p.o2_sync >= 0.5 { t1.wrap } else { None };
        let t2 = self.osc[1].tick(Wave::from_param(p.o2_wave), self.inc[1], self.pw[1], master);
        let s1 = self.sub[0].tick(t1.wrap);
        let s2 = self.sub[1].tick(t2.wrap);
        let sources = [
            (t1.value * p.o1_level, p.o1_route),
            (s1 * p.o1_sub, p.sub1_route),
            (t2.value * p.o2_level, p.o2_route),
            (s2 * p.o2_sub, p.sub2_route),
            (self.noise.next() * p.noise_level, p.noise_route),
        ];
        let (mut in1, mut in2) = (0.0f32, 0.0f32);
        for (v, route) in sources {
            let route = route.round() as i32; // 0 filter 1, 1 filter 2, 2 both
            if route != 1 {
                in1 += v;
            }
            if route >= 1 {
                in2 += v;
            }
        }
        let y1 = self.svf[0].tick(&self.coef[0], Mode::from_param(p.f1_mode), svf::drive(in1, p.f1_drive));
        if p.f_serial >= 0.5 {
            in2 += p.f1_to_f2 * y1;
        }
        let y2 = self.svf[1].tick(&self.coef[1], Mode::from_param(p.f2_mode), svf::drive(in2, p.f2_drive));

        let (a1, a2) = (y1 * ae[0] * self.amp[0] * gain, y2 * ae[1] * self.amp[1] * gain);
        let (l, r) = (a1 * self.pan[0].0 + a2 * self.pan[1].0, a1 * self.pan[0].1 + a2 * self.pan[1].1);
        if self.aenv[0].idle() && self.aenv[1].idle() && self.pending.is_none() {
            self.active = false;
        }
        (l, r)
    }
}
