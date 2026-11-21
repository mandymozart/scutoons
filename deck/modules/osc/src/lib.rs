//! `osc` – one oscillator as a building block: saw, pulse, triangle or sine (the SDK's PolyBLEP oscillator), always
//! running; what makes a note of it is a vca behind it. Its pitch is the last note's of the `notes` input (middle C
//! before the first), moved by `semi` and `fine`, by a bend of that note, and by the `pitch` control input in
//! semitones (an LFO for vibrato, a sequencer's lane); `fm` is audio that swings the frequency around it, linear: +1
//! doubles it, −1 stops it. `voices` stacks up to seven oscillators on the one pitch, spread evenly over ±`detune`
//! cents (the supersaw), levelled so more voices are not louder.
//!
//! The pitch is recomputed every sixteen samples of the module's own clock and at every note, so a render does not
//! depend on how the host cuts the blocks.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p osc-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/osc_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::{exp2, C4};
use deck_module::dsp::osc::{Osc as Unit, Wave};
use deck_module::{Block, Ctx, Ev, Expr, Kind, Module, Smooth};

pub const VOICES: usize = 7;
/// samples between two pitch updates
const CTRL: u32 = 16;
/// the time the `pitch` input and the knobs take to arrive, seconds
const SLEW: f32 = 0.005;
/// 1 / √n: n voices as loud as one
const LEVEL: [f32; VOICES] = [1.0, 0.707_106_77, 0.577_350_26, 0.5, 0.447_213_6, 0.408_248_3, 0.377_964_47];

#[derive(Module)]
#[module(name = "osc", doc = "one oscillator: saw, pulse, triangle or sine, up to seven detuned voices, pitched by notes")]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "a note on sets the pitch; a bend of that note moves it")]
#[port(input, audio, name = "fm", channels = 1, sidechain, doc = "frequency modulation, linear: +1 doubles the frequency, −1 stops it")]
#[port(input, control, name = "pitch", doc = "added to the pitch, in semitones")]
#[port(output, audio, name = "out", channels = 1, doc = "the wave, −1 … 1")]
#[preset(name = "supersaw", doc = "Seven saws a quarter of a semitone wide: the wide, beating lead.", values(wave = 0.0, voices = 7.0, detune = 25.0))]
pub struct Osc {
    #[param(choices = "saw|pulse|triangle|sine", default = 0, doc = "the waveform")]
    pub wave: f32,
    #[param(range = -48.0..48.0, default = 0.0, unit = "st", doc = "pitch offset in semitones")]
    pub semi: f32,
    #[param(range = -100.0..100.0, default = 0.0, unit = "cent", doc = "fine tuning")]
    pub fine: f32,
    #[param(range = 5.0..95.0, default = 50.0, unit = "%", doc = "pulse width (the pulse wave only)")]
    pub pw: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "cent", doc = "how far the outermost voices sit from the pitch")]
    pub detune: f32,
    #[param(range = 1.0..7.0, default = 1.0, doc = "how many oscillators play the pitch")]
    pub voices: f32,
    units: [Unit; VOICES],
    /// cycles a sample, per voice
    dt: [f64; VOICES],
    /// the note's pitch, the note that set it and its bend in semitones
    hz: f64,
    note: Option<u32>,
    bend: f32,
    /// the offset in semitones: `semi`, `fine`, the bend and the `pitch` input
    offset: Smooth,
    slew: f32,
    clock: u32,
    rate: f64,
}

impl Osc {
    pub fn count(&self) -> usize {
        (self.voices.round() as usize).clamp(1, VOICES)
    }

    fn event(&mut self, e: &Ev) {
        match e.kind() {
            Some(Kind::NoteOn) => {
                let hz = e.pitch_hz();
                if hz.is_finite() && hz > 0.0 {
                    (self.hz, self.note, self.bend) = (f64::from(hz), Some(e.id), 0.0);
                    // the note's pitch is there at its frame
                    self.clock = 0;
                }
            }
            Some(Kind::NoteExpr) if self.note == Some(e.id) => {
                if let Some((Expr::Bend, semis)) = e.expr().filter(|(_, v)| v.is_finite()) {
                    self.bend = semis;
                }
            }
            _ => {}
        }
    }

    fn pitch(&mut self, input: f32) {
        self.offset.to(self.semi + self.fine / 100.0 + self.bend + input);
        let st = f64::from(self.offset.step(self.slew));
        let n = self.count();
        for (k, dt) in self.dt[..n].iter_mut().enumerate() {
            let spread = if n == 1 { 0.0 } else { 2.0 * k as f64 / (n - 1) as f64 - 1.0 };
            *dt = self.hz * exp2((st + spread * f64::from(self.detune) / 100.0) / 12.0) / self.rate;
        }
    }
}

impl Module for Osc {
    fn init(&mut self, ctx: &Ctx) {
        self.rate = ctx.rate().max(1.0);
        self.hz = C4;
        self.slew = Smooth::coef(SLEW, CTRL as f32 / self.rate as f32);
        self.offset.jump(self.semi + self.fine / 100.0);
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let events = block.events_in(Self::NOTES);
        let input = block.control_in(Self::PITCH);
        let fm = block.audio_in(Self::FM).chan(0);
        let (wave, pw) = (Wave::from_param(self.wave), f64::from(self.pw) / 100.0);
        let mut next = 0;
        let mut out = block.audio_out(Self::OUT);
        for (i, o) in out.chan(0).iter_mut().enumerate() {
            while let Some(e) = events.get(next).filter(|e| e.frame as usize <= i) {
                self.event(e);
                next += 1;
            }
            if self.clock % CTRL == 0 {
                self.pitch(input);
            }
            self.clock = self.clock.wrapping_add(1);
            let n = self.count();
            let swing = 1.0 + f64::from(fm[i]);
            let mut sum = 0.0;
            for (unit, dt) in self.units[..n].iter_mut().zip(&self.dt) {
                sum += unit.tick(wave, dt * swing, pw, None).value;
            }
            *o = sum * LEVEL[n - 1];
        }
        for e in &events[next.min(events.len())..] {
            self.event(e);
        }
    }
}
