//! `noise` – seeded noise on one channel: white, or pink (−3 dB an octave). `free` it runs all the time and something
//! after it shapes it (an envelope into a vca, a filter); `gated` it is silent until a note on of the `gate` input opens
//! it at the note's velocity, and that note's off closes it – the last note wins, a reset closes. The level and the gate
//! move over two milliseconds, so neither clicks. The same seed is the same noise on every deck.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p noise-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/noise_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::level;
use deck_module::dsp::noise::{Pink, White};
use deck_module::{Block, Ctx, Ev, Kind, Module, Smooth};

/// the time the level and the gate take to move, seconds
const SLEW: f32 = 0.002;
/// this close a glide has arrived (−100 dB)
const ARRIVED: f32 = 1e-5;

#[derive(Module)]
#[module(name = "noise", doc = "seeded white or pink noise, free-running or opened by a gate")]
#[port(input, event, name = "gate", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "gated: a note on opens the noise at its velocity, its note off closes it")]
#[port(output, audio, name = "out", channels = 1, doc = "the noise")]
#[preset(name = "hat", doc = "White noise that sounds only while a gate is open: the source of a hi-hat in front of a filter.", values(mode = 1.0, level = -6.0))]
pub struct Noise {
    #[param(choices = "white|pink", default = 0, doc = "white: every frequency alike; pink: 3 dB less an octave up")]
    pub color: f32,
    #[param(range = -60.0..12.0, default = -12.0, unit = "dB", doc = "the level; at the bottom it is silent")]
    pub level: f32,
    #[param(choices = "free|gated", default = 0, doc = "free: it always sounds; gated: only while a note of the gate input is held")]
    pub mode: f32,
    white: White,
    pink: Option<Pink>,
    amp: Smooth,
    open: Smooth,
    /// the held note and its velocity
    held: Option<(u32, f32)>,
    slew: f32,
}

impl Noise {
    fn gated(&self) -> bool {
        self.mode >= 0.5
    }

    /// how far the gate is open now
    fn gate(&self) -> f32 {
        match (self.gated(), self.held) {
            (false, _) => 1.0,
            (true, Some((_, vel))) => vel,
            (true, None) => 0.0,
        }
    }

    fn event(&mut self, e: &Ev) {
        match e.kind() {
            Some(Kind::NoteOn) => {
                let vel = e.velocity();
                self.held = Some((e.id, if vel.is_finite() { vel.clamp(0.0, 1.0) } else { 1.0 }));
            }
            Some(Kind::NoteOff) if self.held.is_some_and(|(id, _)| id == e.id) => self.held = None,
            Some(Kind::Reset) => self.held = None,
            _ => {}
        }
    }
}

impl Module for Noise {
    fn init(&mut self, ctx: &Ctx) {
        self.white = White::new(ctx.seed());
        self.pink = Some(Pink::new(ctx.seed()));
        self.slew = Smooth::coef(SLEW, 1.0 / ctx.rate() as f32);
        self.amp.jump(level::gain(self.level));
        self.open.jump(self.gate());
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let events = block.events_in(Self::GATE);
        self.amp.to(level::gain(self.level));
        self.amp.land(ARRIVED);
        self.open.land(ARRIVED);
        let closed = self.gated() && self.held.is_none() && self.open.value() == 0.0;
        if closed && events.is_empty() {
            self.amp.jump(self.amp.target());
            block.set_silent(Self::OUT);
            return;
        }
        let pink = self.color >= 0.5;
        let mut next = 0;
        let mut out = block.audio_out(Self::OUT);
        for (i, o) in out.chan(0).iter_mut().enumerate() {
            while let Some(e) = events.get(next).filter(|e| e.frame as usize <= i) {
                self.event(e);
                next += 1;
            }
            self.open.to(self.gate());
            let x = match self.pink.as_mut().filter(|_| pink) {
                Some(p) => p.sample(),
                None => self.white.sample(),
            };
            *o = x * self.amp.step(self.slew) * self.open.step(self.slew);
        }
        for e in &events[next.min(events.len())..] {
            self.event(e);
        }
    }
}
