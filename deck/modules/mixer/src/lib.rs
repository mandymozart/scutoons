//! `mixer` – a rig's own mixer: eight stereo inputs summed into `out`, each with a level, a balance and a mute, and a
//! second sum, `send`, that takes of every input what its send level says – after the input's level and mute, before
//! its balance – for one shared effect (a reverb, a delay) whose return comes back into another input. `master` is the
//! level of `out` alone. The balance leaves the centre as it is and turns the other side down, so a stereo input keeps
//! its level in the middle; a mono cable sounds on both sides. Every level glides over two milliseconds. (The engine's
//! `mixer` kind stays the scene's; this one sits inside a rig.)
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p mixer-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/mixer_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::level;
use deck_module::{Block, Ctx, Dirty, Module, Smooth, QUANTUM};

pub const INPUTS: usize = 8;
/// the time a level takes to move, seconds
const SLEW: f32 = 0.002;
/// this close a glide has arrived (−100 dB)
const ARRIVED: f32 = 1e-5;

#[derive(Module)]
#[module(name = "mixer", doc = "eight stereo inputs into one stereo sum and one send, each with level, balance, send and mute")]
#[port(input, audio, name = "in1", channels = 2, doc = "input 1")]
#[port(input, audio, name = "in2", channels = 2, doc = "input 2")]
#[port(input, audio, name = "in3", channels = 2, doc = "input 3")]
#[port(input, audio, name = "in4", channels = 2, doc = "input 4")]
#[port(input, audio, name = "in5", channels = 2, doc = "input 5")]
#[port(input, audio, name = "in6", channels = 2, doc = "input 6")]
#[port(input, audio, name = "in7", channels = 2, doc = "input 7")]
#[port(input, audio, name = "in8", channels = 2, doc = "input 8")]
#[port(output, audio, name = "out", channels = 2, doc = "the sum of the inputs times master")]
#[port(output, audio, name = "send", channels = 2, doc = "the sum of what the inputs send: into one shared effect")]
#[preset(name = "pair", doc = "Inputs 1 and 2 as a wide pair, both feeding the send a little.", values(pan1 = -0.6, pan2 = 0.6, send1 = -12.0, send2 = -12.0))]
pub struct Mixer {
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 1: its level; at the bottom it is silent")]
    pub gain1: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 2: its level; at the bottom it is silent")]
    pub gain2: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 3: its level; at the bottom it is silent")]
    pub gain3: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 4: its level; at the bottom it is silent")]
    pub gain4: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 5: its level; at the bottom it is silent")]
    pub gain5: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 6: its level; at the bottom it is silent")]
    pub gain6: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 7: its level; at the bottom it is silent")]
    pub gain7: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "input 8: its level; at the bottom it is silent")]
    pub gain8: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 1: its balance, −1 left … 1 right")]
    pub pan1: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 2: its balance, −1 left … 1 right")]
    pub pan2: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 3: its balance, −1 left … 1 right")]
    pub pan3: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 4: its balance, −1 left … 1 right")]
    pub pan4: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 5: its balance, −1 left … 1 right")]
    pub pan5: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 6: its balance, −1 left … 1 right")]
    pub pan6: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 7: its balance, −1 left … 1 right")]
    pub pan7: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "input 8: its balance, −1 left … 1 right")]
    pub pan8: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 1: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send1: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 2: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send2: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 3: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send3: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 4: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send4: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 5: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send5: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 6: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send6: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 7: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send7: f32,
    #[param(range = -60.0..12.0, default = -60.0, unit = "dB", doc = "input 8: how much of it goes to the send output, after its level; at the bottom nothing")]
    pub send8: f32,
    #[param(choices = "off|on", default = 0, doc = "input 1: muted, in out and send")]
    pub mute1: f32,
    #[param(choices = "off|on", default = 0, doc = "input 2: muted, in out and send")]
    pub mute2: f32,
    #[param(choices = "off|on", default = 0, doc = "input 3: muted, in out and send")]
    pub mute3: f32,
    #[param(choices = "off|on", default = 0, doc = "input 4: muted, in out and send")]
    pub mute4: f32,
    #[param(choices = "off|on", default = 0, doc = "input 5: muted, in out and send")]
    pub mute5: f32,
    #[param(choices = "off|on", default = 0, doc = "input 6: muted, in out and send")]
    pub mute6: f32,
    #[param(choices = "off|on", default = 0, doc = "input 7: muted, in out and send")]
    pub mute7: f32,
    #[param(choices = "off|on", default = 0, doc = "input 8: muted, in out and send")]
    pub mute8: f32,
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "the level of out; at the bottom it is silent")]
    pub master: f32,
    /// per input: its factors into out's left and right and into send, gliding
    strips: [[Smooth; 3]; INPUTS],
    total: Smooth,
    slew: f32,
}

impl Mixer {
    /// where every factor heads, from the parameters
    fn aim(&mut self) {
        let gains = [self.gain1, self.gain2, self.gain3, self.gain4, self.gain5, self.gain6, self.gain7, self.gain8];
        let pans = [self.pan1, self.pan2, self.pan3, self.pan4, self.pan5, self.pan6, self.pan7, self.pan8];
        let sends = [self.send1, self.send2, self.send3, self.send4, self.send5, self.send6, self.send7, self.send8];
        let mutes = [self.mute1, self.mute2, self.mute3, self.mute4, self.mute5, self.mute6, self.mute7, self.mute8];
        for (k, [left, right, send]) in self.strips.iter_mut().enumerate() {
            let gain = if mutes[k] >= 0.5 { 0.0 } else { level::gain(gains[k]) };
            left.to(gain * (1.0 - pans[k]).min(1.0));
            right.to(gain * (1.0 + pans[k]).min(1.0));
            send.to(gain * level::gain(sends[k]));
        }
        self.total.to(level::gain(self.master));
    }
}

impl Module for Mixer {
    fn init(&mut self, ctx: &Ctx) {
        self.slew = Smooth::coef(SLEW, 1.0 / ctx.rate() as f32);
        self.aim();
        for s in self.strips.iter_mut().flatten().chain([&mut self.total]) {
            s.jump(s.target());
        }
    }

    fn params_changed(&mut self, _: Dirty) {
        self.aim();
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let n = block.frames();
        for s in self.strips.iter_mut().flatten().chain([&mut self.total]) {
            s.land(ARRIVED);
        }
        // out left, out right, send left, send right
        let mut sums = [[0.0f32; QUANTUM]; 4];
        let mut sounding = false;
        for (k, [left, right, send]) in self.strips.iter_mut().enumerate() {
            if block.in_silent(k) {
                // nothing to hear: the levels are where they head when a signal comes
                for s in [left, right, send] {
                    s.jump(s.target());
                }
                continue;
            }
            sounding = true;
            let (l, r) = block.audio_in(k).stereo();
            for i in 0..n {
                let (a, b, c) = (left.step(self.slew), right.step(self.slew), send.step(self.slew));
                sums[0][i] += l[i] * a;
                sums[1][i] += r[i] * b;
                sums[2][i] += l[i] * c;
                sums[3][i] += r[i] * c;
            }
        }
        if !sounding {
            self.total.jump(self.total.target());
            block.set_silent(Self::OUT);
            block.set_silent(Self::SEND);
            return;
        }
        {
            let mut out = block.audio_out(Self::OUT);
            let (l, r) = out.stereo();
            for i in 0..n.min(l.len()).min(r.len()) {
                let master = self.total.step(self.slew);
                l[i] = sums[0][i] * master;
                r[i] = sums[1][i] * master;
            }
        }
        let mut send = block.audio_out(Self::SEND);
        let (l, r) = send.stereo();
        l.copy_from_slice(&sums[2][..l.len()]);
        r.copy_from_slice(&sums[3][..r.len()]);
    }
}
