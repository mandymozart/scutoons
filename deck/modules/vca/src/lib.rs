//! `vca` – a gain a signal can play: every channel of `in` times `gain`, and, as far as `amount` says, times what
//! arrives at the cv inputs – `cv` as audio (an envelope a sample, a tremolo, another sound) plus `cv_control` (a value a
//! call: an LFO, a fader, a level from elsewhere). At `amount` 0 it is a plain gain; at 100 % the cv alone opens it and
//! nothing connected means silence, as on a hardware VCA. A cv below zero closes it (no phase flip). `gain`, `amount` and
//! the control cv glide over two milliseconds, so a knob or a step from the control rate does not click.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p vca-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/vca_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::level;
use deck_module::{Block, Ctx, Module, Smooth, QUANTUM};

/// the time `gain`, `amount` and the control cv take to move, seconds
const SLEW: f32 = 0.002;
/// this close a glide has arrived (−100 dB)
const ARRIVED: f32 = 1e-5;

#[derive(Module)]
#[module(name = "vca", doc = "a gain a control signal can play: an envelope, an LFO or another sound opens it")]
#[port(input, audio, name = "in", channels = "any", doc = "the signal")]
#[port(input, audio, name = "cv", channels = 1, sidechain, doc = "what opens the vca, a value a sample (0 … 1): an envelope's audio output")]
#[port(input, control, name = "cv_control", doc = "what opens the vca, a value a call (0 … 1); added to cv")]
#[port(output, audio, name = "out", channels = "any", doc = "the signal times the gain")]
#[preset(name = "vca", doc = "Closed until its cv opens it: an envelope into cv makes a note of a running oscillator.", values(amount = 100.0))]
pub struct Vca {
    #[param(range = -60.0..12.0, default = 0.0, unit = "dB", doc = "the gain; at the bottom it is silent")]
    pub gain: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "how much the cv plays the gain: 0 = not at all, 100 = the cv alone opens it")]
    pub amount: f32,
    amp: Smooth,
    depth: Smooth,
    control: Smooth,
    slew: f32,
}

impl Module for Vca {
    fn init(&mut self, ctx: &Ctx) {
        self.slew = Smooth::coef(SLEW, 1.0 / ctx.rate() as f32);
        self.amp.jump(level::gain(self.gain));
        self.depth.jump(self.amount / 100.0);
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        self.amp.to(level::gain(self.gain));
        self.depth.to(self.amount / 100.0);
        self.control.to(block.control_in(Self::CV_CONTROL));
        for s in [&mut self.amp, &mut self.depth, &mut self.control] {
            s.land(ARRIVED);
        }
        let n = block.frames();
        let cv = block.audio_in(Self::CV).chan(0);
        let mut factor = [0.0f32; QUANTUM];
        for (f, cv) in factor[..n].iter_mut().zip(cv) {
            let depth = self.depth.step(self.slew);
            let open = (cv + self.control.step(self.slew)).max(0.0);
            *f = self.amp.step(self.slew) * (1.0 - depth + depth * open);
        }
        let inp = block.audio_in(Self::IN);
        if block.in_silent(Self::IN) {
            block.set_silent(Self::OUT);
            return;
        }
        let mut out = block.audio_out(Self::OUT);
        for c in 0..out.ch() {
            // a mono signal goes to every channel
            let src = inp.chan(if inp.ch() == 1 { 0 } else { c });
            for ((o, x), f) in out.chan(c).iter_mut().zip(src).zip(&factor) {
                *o = x * f;
            }
        }
    }
}
