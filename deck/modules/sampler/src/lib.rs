//! `sampler` – a recorded sound played by notes: the sound asset in the slot `sample` (mono or stereo, at its own rate),
//! a voice a note. `mode` says how a voice plays its part of the sound (`start` and `length`, in % of the whole):
//! `oneshot` through to the end whatever the note does (a drum); `gate` while the note is held, then its release;
//! `loop` round and round while held, then its release. `pitch` = `note` plays the sound faster or slower by how far the
//! note's key lies from `root` (the key it was recorded at), `fixed` always at its own pitch; `tune` and a note's bend
//! move it on top. The velocity scales the level by `velocity` %; a reset releases every voice. At most `voices` sound at
//! once – a note past them takes the oldest. At its own pitch and at the deck's rate a voice plays the sound's samples
//! as they are; between them it reads a cubic through the four around. Without a sound (the node names none, or it
//! could not be had) it is silent.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p sampler-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/sampler_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::exp2;
use deck_module::dsp::level;
use deck_module::{Asset, Block, Ctx, Ev, Expr, Kind, Module, Smooth};

/// the most voices
pub const VOICES: usize = 8;
/// samples between two steps of the gain's glide
const CTRL: usize = 16;
const SLEW: f32 = 0.02;
/// this close the gain has arrived: at 0 dB a voice is the sound exactly
const ARRIVED: f32 = 1e-5;

#[derive(Module)]
#[module(name = "sampler", doc = "a recorded sound from an asset played by notes: one-shot, gated or looped, pitched by the note or fixed, a few voices")]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "a note on starts a voice, its off releases it (gate, loop), a bend moves its pitch")]
#[port(output, audio, name = "out", channels = 2, doc = "the voices summed: a mono sound on both sides")]
#[asset(name = "sample", kind = "pcm", max_bytes = 16777216, doc = "the sound: mono or stereo, at most 16 MiB (about 43 s stereo at 48 kHz); none: silent")]
#[preset(name = "pad", doc = "A held loop that fades out slowly after the note: a drone from any sound.", values(mode = 2.0, release = 1.5, velocity = 30.0))]
pub struct Sampler {
    #[param(choices = "oneshot|gate|loop", default = 0, doc = "oneshot: to the end whatever the note does; gate: while held, then the release; loop: round and round while held, then the release")]
    pub mode: f32,
    #[param(choices = "note|fixed", default = 0, doc = "note: faster or slower by the key's distance from root; fixed: always at the sound's own pitch")]
    pub pitch: f32,
    #[param(range = 0.0..127.0, default = 60.0, doc = "the key (MIDI) the sound was recorded at: that note plays it at its own pitch")]
    pub root: f32,
    #[param(range = -24.0..24.0, default = 0.0, unit = "st", doc = "every voice moved by semitones, in both pitch modes")]
    pub tune: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "the level")]
    pub gain: f32,
    #[param(range = 0.0..100.0, default = 100.0, unit = "%", doc = "how much a note's velocity scales its level: 0 = always full")]
    pub velocity: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "where in the sound a voice starts")]
    pub start: f32,
    #[param(range = 0.0..100.0, default = 100.0, unit = "%", doc = "how much of the sound a voice plays from its start (a loop's length)")]
    pub length: f32,
    #[param(range = 0.001..10.0, default = 0.05, curve = log, unit = "s", doc = "the fade after a note's off (gate, loop) or a reset")]
    pub release: f32,
    #[param(range = 1.0..8.0, default = 8.0, doc = "how many voices sound at once: a note past them takes the oldest")]
    pub voices: f32,
    voice: [Voice; VOICES],
    gain_glide: Smooth,
    slew: f32,
    rate: f64,
    clock: usize,
    /// notes started, the age of a voice
    started: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Voice {
    on: bool,
    id: u32,
    /// the frame of the sound it reads next
    pos: f64,
    /// frames of the sound a sample
    step: f64,
    key: f32,
    bend: f32,
    /// the velocity's scale
    level: f32,
    /// 1 until released, then down to 0
    fade: f32,
    released: bool,
    age: u64,
}

/// the part of the sound a voice plays, frames `[begin, end)` of `frames`
fn region(frames: usize, start: f32, length: f32) -> (usize, usize) {
    if frames == 0 {
        return (0, 0);
    }
    let begin = ((f64::from(start.clamp(0.0, 100.0)) / 100.0 * frames as f64) as usize).min(frames - 1);
    let len = ((f64::from(length.clamp(0.0, 100.0)) / 100.0 * frames as f64) as usize).max(1);
    (begin, (begin + len).min(frames))
}

/// channel `x` at `pos`: the sample itself on a whole frame, else a cubic through the four around (the ends held)
fn read(x: &[f32], pos: f64) -> f32 {
    let i = pos as usize;
    let f = (pos - i as f64) as f32;
    let last = x.len() - 1;
    let b = x[i.min(last)];
    if f == 0.0 {
        return b;
    }
    let (a, c, e) = (x[i.saturating_sub(1).min(last)], x[(i + 1).min(last)], x[(i + 2).min(last)]);
    let c1 = 0.5 * (c - a);
    let c2 = a - 2.5 * b + 2.0 * c - 0.5 * e;
    let c3 = 0.5 * (e - a) + 1.5 * (b - c);
    ((c3 * f + c2) * f + c1) * f + b
}

impl Sampler {
    /// how many voices sound (a release counts)
    pub fn active_voices(&self) -> usize {
        self.voice.iter().filter(|v| v.on).count()
    }

    fn gated(&self) -> bool {
        self.mode.round() as i32 != 0
    }

    fn looped(&self) -> bool {
        self.mode.round() as i32 == 2
    }

    /// a voice's frames of the sound a sample: the sound's rate against the deck's, by the key and the bend
    fn step_of(&self, v: &Voice, sound_rate: f64) -> f64 {
        let by_key = if self.pitch.round() as i32 == 0 && v.key.is_finite() { f64::from(v.key) - f64::from(self.root) } else { 0.0 };
        let semis = by_key + f64::from(self.tune) + f64::from(v.bend);
        sound_rate / self.rate * exp2(semis / 12.0)
    }

    fn event(&mut self, e: &Ev, sound_rate: f64, begin: usize) {
        match e.kind() {
            Some(Kind::NoteOn) => {
                let n = (self.voices.round() as usize).clamp(1, VOICES);
                let at = self.voice[..n].iter().position(|v| !v.on).unwrap_or_else(|| (0..n).min_by_key(|&i| self.voice[i].age).unwrap_or(0));
                let (vel, amount) = (e.velocity(), self.velocity.clamp(0.0, 100.0) / 100.0);
                let vel = if vel.is_finite() { vel.clamp(0.0, 1.0) } else { 1.0 };
                self.started += 1;
                let mut v = Voice { on: true, id: e.id, pos: begin as f64, key: e.key(), level: 1.0 - amount + amount * vel, fade: 1.0, age: self.started, ..Voice::default() };
                v.step = self.step_of(&v, sound_rate);
                self.voice[at] = v;
            }
            Some(Kind::NoteOff) if self.gated() => {
                for v in self.voice.iter_mut().filter(|v| v.on && v.id == e.id) {
                    v.released = true;
                }
            }
            Some(Kind::NoteExpr) => {
                if let Some((Expr::Bend, semis)) = e.expr() {
                    for i in 0..VOICES {
                        if self.voice[i].on && self.voice[i].id == e.id && semis.is_finite() {
                            self.voice[i].bend = semis;
                            self.voice[i].step = self.step_of(&self.voice[i], sound_rate);
                        }
                    }
                }
            }
            Some(Kind::Reset) => self.voice.iter_mut().for_each(|v| v.released = true),
            _ => {}
        }
    }
}

impl Module for Sampler {
    fn init(&mut self, ctx: &Ctx) {
        self.rate = ctx.rate().max(1.0);
        self.slew = Smooth::coef(SLEW, CTRL as f32 / self.rate as f32);
        self.gain_glide.jump(level::gain(self.gain.clamp(-60.0, 12.0)));
    }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let n = block.frames();
        let events = block.events_in(Self::NOTES);
        let sound: Option<Asset> = ctx.asset(Self::SAMPLE).filter(|a| a.channels > 0 && a.frames > 0);
        let Some(sound) = sound else {
            self.voice = [Voice::default(); VOICES];
            self.clock = self.clock.wrapping_add(n);
            block.set_silent(Self::OUT);
            return;
        };
        if self.active_voices() == 0 && events.is_empty() {
            self.clock = self.clock.wrapping_add(n);
            block.set_silent(Self::OUT);
            return;
        }
        let (l, r) = (sound.pcm(0), sound.pcm(if sound.channels > 1 { 1 } else { 0 }));
        let sound_rate = f64::from(sound.rate);
        let (begin, end) = region(l.len(), self.start, self.length);
        let looped = self.looped();
        let fall = 1.0 / (f64::from(self.release.max(0.001)) * self.rate) as f32;
        self.gain_glide.to(level::gain(self.gain.clamp(-60.0, 12.0)));
        // a turned tune takes the sounding voices along
        for i in 0..VOICES {
            if self.voice[i].on {
                self.voice[i].step = self.step_of(&self.voice[i], sound_rate);
            }
        }
        let mut out = block.audio_out(Self::OUT);
        let (outl, outr) = out.stereo();
        let mono = outr.is_empty();
        let mut next = 0;
        for i in 0..n {
            while let Some(e) = events.get(next).filter(|e| e.frame as usize <= i) {
                self.event(e, sound_rate, begin);
                next += 1;
            }
            if self.clock % CTRL == 0 {
                self.gain_glide.step(self.slew);
                self.gain_glide.land(ARRIVED);
            }
            self.clock = self.clock.wrapping_add(1);
            let k = self.gain_glide.value();
            let mut y = [0.0f32; 2];
            for v in self.voice.iter_mut().filter(|v| v.on) {
                if v.pos >= end as f64 {
                    if looped && end > begin {
                        v.pos = begin as f64 + (v.pos - end as f64) % (end - begin) as f64;
                    } else {
                        v.on = false;
                        continue;
                    }
                }
                if v.released {
                    v.fade -= fall;
                    if v.fade <= 0.0 {
                        v.on = false;
                        continue;
                    }
                }
                let g = v.level * v.fade * k;
                y[0] += read(l, v.pos) * g;
                y[1] += read(r, v.pos) * g;
                v.pos += v.step;
            }
            if mono {
                if let Some(o) = outl.get_mut(i) {
                    *o = 0.5 * (y[0] + y[1]);
                }
            } else {
                outl[i] = y[0];
                outr[i] = y[1];
            }
        }
        for e in &events[next.min(events.len())..] {
            self.event(e, sound_rate, begin);
        }
    }
}
