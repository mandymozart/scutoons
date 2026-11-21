//! `env` – an envelope a gate plays: ADSR, AD (a one-shot: attack, then the decay to zero whatever the gate does) or AR
//! (attack, held at the top while the gate is up, release). It leaves twice: as audio on `out`, a value a sample, to
//! open a vca without steps; and as the control `env`, once a call, for everything slower – a filter's cutoff, a light.
//! A note on of the `gate` input raises the gate (again from the level it finds), the off of the last note lowers it, a
//! reset lowers it. `curve` chooses straight segments or the exponential decay and release of an analog envelope; a
//! note keeps the curve it started with. `velocity` is how much a note's velocity scales the whole envelope.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p env-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/env_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::adsr::{Adsr, Stage, Times};
use deck_module::dsp::line::Line;
use deck_module::{Block, Ctx, Ev, Kind, Module};

#[derive(Module)]
#[module(name = "env", doc = "an envelope a gate plays: ADSR, AD or AR, as audio and as a control value")]
#[port(input, event, name = "gate", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "a note on raises the gate, the last note's off lowers it")]
#[port(output, audio, name = "out", channels = 1, doc = "the envelope a sample, 0 … 1: into a vca's cv")]
#[port(output, control, name = "env", doc = "the envelope once a call, 0 … 1: for cutoffs, lights, pictures")]
#[preset(name = "pluck", doc = "A one-shot that snaps open and falls away in a quarter of a second, louder the harder the note is played.", values(mode = 1.0, a = 0.001, d = 0.25, velocity = 60.0))]
pub struct Env {
    #[param(choices = "adsr|ad|ar", default = 0, doc = "adsr: the four stages; ad: attack and decay to zero, the gate's length ignored; ar: attack, held, release")]
    pub mode: f32,
    #[param(range = 0.0005..10.0, default = 0.005, curve = log, unit = "s", doc = "attack: the time to the top")]
    pub a: f32,
    #[param(range = 0.001..10.0, default = 0.2, curve = log, unit = "s", doc = "decay: the time from the top to the sustain level (ad: to zero)")]
    pub d: f32,
    #[param(range = 0.0..100.0, default = 70.0, unit = "%", doc = "sustain: the level held while the gate is up (adsr only)")]
    pub s: f32,
    #[param(range = 0.001..20.0, default = 0.3, curve = log, unit = "s", doc = "release: the time to zero after the gate falls")]
    pub r: f32,
    #[param(choices = "lin|exp", default = 1, doc = "lin: straight segments; exp: decay and release fall fast first, like a capacitor")]
    pub curve: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "how much the note's velocity scales the envelope: 0 = always full")]
    pub velocity: f32,
    bent: Adsr,
    straight: Line,
    /// the curve the sounding note started with
    lin: bool,
    held: Option<u32>,
    scale: f32,
    rate: f64,
}

impl Env {
    fn one_shot(&self) -> bool {
        self.mode.round() as i32 == 1
    }

    /// the stages' times in this mode
    fn times(&self) -> Times {
        match self.mode.round() as i32 {
            1 => Times { attack: self.a, decay: self.d, sustain: 0.0, release: self.d },
            2 => Times { attack: self.a, decay: 0.0, sustain: 1.0, release: self.r },
            _ => Times { attack: self.a, decay: self.d, sustain: self.s / 100.0, release: self.r },
        }
    }

    fn gate_off(&mut self, t: &Times) {
        self.held = None;
        if !self.one_shot() {
            self.bent.gate_off(t, self.rate);
            self.straight.gate_off(t, self.rate);
        }
    }

    fn event(&mut self, e: &Ev, t: &Times) {
        match e.kind() {
            Some(Kind::NoteOn) => {
                let (vel, amount) = (e.velocity(), self.velocity / 100.0);
                self.scale = 1.0 - amount + amount * if vel.is_finite() { vel.clamp(0.0, 1.0) } else { 1.0 };
                self.held = Some(e.id);
                // a change of curve takes the next note, which then rises from zero
                let lin = self.curve < 0.5;
                if lin != self.lin {
                    self.lin = lin;
                    (self.bent, self.straight) = (Adsr::default(), Line::default());
                }
                if lin {
                    self.straight.gate_on(t, self.rate);
                } else {
                    self.bent.gate_on(t, self.rate);
                }
            }
            Some(Kind::NoteOff) if self.held == Some(e.id) => self.gate_off(t),
            Some(Kind::Reset) => self.gate_off(t),
            _ => {}
        }
    }

    fn idle(&self) -> bool {
        if self.lin {
            self.straight.idle()
        } else {
            self.bent.idle()
        }
    }

    /// one sample of the shape the note started with
    fn tick(&mut self, t: &Times) -> f32 {
        let (level, stage) = if self.lin { (self.straight.tick(t, self.rate), self.straight.stage()) } else { (self.bent.tick(t, self.rate), self.bent.stage()) };
        // a one-shot is over at the bottom of its decay: it ends there instead of waiting on a sustain of zero
        if stage == Stage::Sustain && self.one_shot() {
            self.bent.gate_off(t, self.rate);
            self.straight.gate_off(t, self.rate);
        }
        level * self.scale
    }
}

impl Module for Env {
    fn init(&mut self, ctx: &Ctx) {
        self.rate = ctx.rate().max(1.0);
        self.scale = 1.0;
        self.lin = self.curve < 0.5;
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let t = self.times();
        let events = block.events_in(Self::GATE);
        if self.idle() && events.is_empty() {
            block.set_silent(Self::OUT);
            block.set(Self::ENV, 0.0);
            return;
        }
        let (mut next, mut last) = (0, 0.0);
        {
            let mut out = block.audio_out(Self::OUT);
            for (i, o) in out.chan(0).iter_mut().enumerate() {
                while let Some(e) = events.get(next).filter(|e| e.frame as usize <= i) {
                    self.event(e, &t);
                    next += 1;
                }
                last = self.tick(&t);
                *o = last;
            }
        }
        for e in &events[next.min(events.len())..] {
            self.event(e, &t);
        }
        block.set(Self::ENV, last);
    }
}
