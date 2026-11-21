//! `delay` – echoes of a stereo signal: the time either a note value on the beat grid (`sync`, from the show's tempo)
//! or free in milliseconds (`sync` = free), moved in octaves by the `time_mod` control input. `feedback` sends the
//! echo back in (kept under 98 %), duller each round by the `damp` lowpass in the loop; `pingpong` crosses the two
//! loops so the echoes bounce left – right, the input summed into the left one; `spread` makes the right one longer.
//! `depth` and `rate` move the time with a sine (the right channel a quarter turn later): at a few milliseconds with
//! no feedback that is a chorus, at longer times a tape's wow. `mix` is the share of the echo in the output.
//!
//! The lines (4 s and the modulation's room) are allocated in `init`; a changed time glides, so it bends the pitch
//! of the tail like tape instead of clicking. Every sample is computed on its own (the LFO's phase too), so a render
//! does not depend on how the host cuts the blocks.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p delay-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/delay_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::{exp2, sin_turns};
use deck_module::dsp::delay::Delay as Line;
use deck_module::dsp::flush;
use deck_module::dsp::onepole::OnePole;
use deck_module::{Block, Ctx, Module, Smooth, QUANTUM};

/// the longest time, ms (`time`, a synced note value and `time_mod` are kept under it)
const MAX_MS: f64 = 4000.0;
/// the room past it: `spread` and `depth` at their tops, ms
const ROOM_MS: f64 = 80.0;
/// the note values of `sync` in beats, `free` first
const BEATS: [f64; 15] = [0.0, 0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.375, 0.5, 2.0 / 3.0, 0.75, 1.0, 4.0 / 3.0, 1.5, 2.0, 3.0, 4.0];
/// the loop's gain at the top of `feedback`
const MAX_FEEDBACK: f32 = 0.98;
/// the time a changed time, feedback or mix takes to arrive, seconds
const SLEW: f32 = 0.05;
/// `damp` at its top: no lowpass in the loop
const OPEN_HZ: f32 = 20_000.0;

#[derive(Module)]
#[module(name = "delay", doc = "echoes synced to the beat or free in ms: feedback, damping, ping-pong, and a chorus at short times")]
#[port(input, audio, name = "in", channels = 2, doc = "the signal (a mono one goes to both channels)")]
#[port(input, control, name = "time_mod", doc = "moves the time, in octaves (1 = twice as long)")]
#[port(output, audio, name = "out", channels = 2, doc = "the signal and its echoes")]
#[preset(name = "dub", doc = "Dotted eighths bouncing left and right, each a little duller: the classic dub echo on a snare or a stab.", values(sync = 8.0, feedback = 55.0, damp = 2500.0, pingpong = 100.0, mix = 35.0))]
pub struct Delay {
    #[param(choices = "free|1/32|1/16t|1/16|1/8t|1/16d|1/8|1/4t|1/8d|1/4|1/2t|1/4d|1/2|1/2d|1/1", default = 0, doc = "the time as a note value on the beat grid; free: the time in ms")]
    pub sync: f32,
    #[param(range = 1.0..4000.0, default = 300.0, curve = log, unit = "ms", doc = "the time while sync is free")]
    pub time: f32,
    #[param(range = 0.0..100.0, default = 35.0, unit = "%", doc = "how much of the echo goes back in (kept under 98 %)")]
    pub feedback: f32,
    #[param(range = 200.0..20000.0, default = 8000.0, curve = log, unit = "Hz", doc = "the lowpass in the loop: each echo duller; at the top none")]
    pub damp: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "ping-pong: at 100 the echoes bounce left – right, the input summed into the left")]
    pub pingpong: f32,
    #[param(range = 0.0..50.0, default = 0.0, unit = "ms", doc = "the right channel's time is this much longer")]
    pub spread: f32,
    #[param(range = 0.0..100.0, default = 30.0, unit = "%", doc = "the share of the echoes in the output: 0 = dry, 100 = only the echoes")]
    pub mix: f32,
    #[param(range = 0.0..20.0, default = 0.0, unit = "ms", doc = "how far the sine moves the time either way: a few ms at a short time is a chorus")]
    pub depth: f32,
    #[param(range = 0.05..10.0, default = 0.8, curve = log, unit = "Hz", doc = "the speed of the sine that moves the time")]
    pub rate: f32,
    /// the two lines, built in `init`
    lines: Option<[Line; 2]>,
    damps: [OnePole; 2],
    /// the time in samples, the loop's gain and the echo's share, gliding
    span: Smooth,
    gain: Smooth,
    wet: Smooth,
    slew: f32,
    /// the LFO's phase in turns
    phase: f64,
    started: bool,
    fs: f64,
}

impl Delay {
    /// the time in samples this call: the note value at the tempo (free or no tempo: `time`), moved by `time_mod`
    fn target(&self, ctx: &Ctx, moved: f32) -> f32 {
        let beats = BEATS[(self.sync.round().max(0.0) as usize).min(BEATS.len() - 1)];
        let bpm = ctx.tempo().bpm;
        let ms = if beats > 0.0 && bpm.is_finite() && bpm > 0.0 { beats * 60_000.0 / bpm } else { f64::from(self.time) };
        let ms = if moved.is_finite() && moved != 0.0 { ms * exp2(f64::from(moved.clamp(-8.0, 8.0))) } else { ms };
        (ms.clamp(0.0, MAX_MS) * self.fs / 1000.0).max(1.0) as f32
    }
}

/// the feedback's own ceiling: untouched up to 1, then bent toward 2 – a loop driven past the top saturates instead
/// of growing
#[inline(always)]
fn limit(v: f32) -> f32 {
    let a = v.abs();
    if a <= 1.0 {
        v
    } else {
        v.signum() * (2.0 - 1.0 / a)
    }
}

impl Module for Delay {
    fn init(&mut self, ctx: &Ctx) {
        self.fs = ctx.rate().max(1.0);
        let max = ((MAX_MS + ROOM_MS) * self.fs / 1000.0).ceil() as usize + 4;
        self.lines = Some([Line::new(max), Line::new(max)]);
        self.slew = Smooth::coef(SLEW, 1.0 / self.fs as f32);
    }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let target = self.target(ctx, block.control_in(Self::TIME_MOD));
        let (gain, wet) = ((self.feedback / 100.0).min(MAX_FEEDBACK), self.mix / 100.0);
        if self.started {
            self.span.to(target);
            self.gain.to(gain);
            self.wet.to(wet);
        } else {
            // the first call starts where the host is now (its tempo, the parameters it set), without a glide
            self.span.jump(target);
            self.gain.jump(gain);
            self.wet.jump(wet);
            self.started = true;
        }
        self.span.land(1e-3);
        self.gain.land(1e-6);
        self.wet.land(1e-6);
        let fs = self.fs as f32;
        let damp = if self.damp >= OPEN_HZ { 1.0 } else { OnePole::coef(self.damp, fs) };
        let cross = (self.pingpong / 100.0).clamp(0.0, 1.0);
        let spread = self.spread.clamp(0.0, 50.0) * fs / 1000.0;
        let depth = self.depth.clamp(0.0, 20.0) * fs / 1000.0;
        let step = f64::from(self.rate.clamp(0.05, 10.0)) / self.fs;
        let Some([left, right]) = self.lines.as_mut() else { return };
        let n = block.frames();
        let inp = block.audio_in(Self::IN);
        let (a, b) = (inp.chan(0), inp.chan(if inp.ch() > 1 { 1 } else { 0 }));
        let (mut l, mut r) = ([0.0f32; QUANTUM], [0.0f32; QUANTUM]);
        for i in 0..n {
            let span = self.span.step(self.slew);
            let fb = self.gain.step(self.slew);
            let wet = self.wet.step(self.slew);
            let (ml, mr) = if depth > 0.0 { (depth * sin_turns(self.phase) as f32, depth * sin_turns(self.phase + 0.25) as f32) } else { (0.0, 0.0) };
            self.phase += step;
            self.phase -= self.phase.floor();
            let (xl, xr) = (a[i], b[i]);
            let (yl, yr) = (left.read_frac(span + ml), right.read_frac(span + spread + mr));
            let (bl, br) = (self.damps[0].lowpass(yl, damp), self.damps[1].lowpass(yr, damp));
            let feed_l = xl * (1.0 - cross) + 0.5 * (xl + xr) * cross;
            let feed_r = xr * (1.0 - cross);
            left.write(flush(feed_l + limit(fb * ((1.0 - cross) * bl + cross * br))));
            right.write(flush(feed_r + limit(fb * ((1.0 - cross) * br + cross * bl))));
            l[i] = xl * (1.0 - wet) + yl * wet;
            r[i] = xr * (1.0 - wet) + yr * wet;
        }
        let mut out = block.audio_out(Self::OUT);
        out.chan(0)[..n].copy_from_slice(&l[..n]);
        if out.ch() > 1 {
            out.chan(1)[..n].copy_from_slice(&r[..n]);
        }
    }
}
