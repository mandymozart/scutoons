//! `filter` – the SDK's state-variable filter on every channel of `in`: low-pass with 12 or 24 dB an octave (two
//! filters in a row, the resonance on the second), band-pass, high-pass or notch, with a drive stage in front. The
//! cutoff is the knob, moved in octaves by the `cutoff_mod` control input (an envelope, an LFO) and, as far as `key`
//! says, by the last note of the `notes` input: at 100 % it follows the keyboard, neutral at middle C.
//!
//! The cutoff and the resonance glide and the coefficients are recomputed every sixteen samples of the module's own
//! clock, so a sweep has no steps and a render does not depend on how the host cuts the blocks.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p filter-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/filter_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::arith::{exp2, log2, C4};
use deck_module::dsp::svf::{self, Coef, Mode, Svf};
use deck_module::{Block, Ctx, Ev, Kind, Module, Smooth};

/// the channels a cable can carry
const CHANNELS: usize = 32;
/// samples between two coefficient updates
const CTRL: usize = 16;
/// the time the cutoff and the resonance take to arrive, seconds
const SLEW: f32 = 0.005;

#[derive(Module)]
#[module(name = "filter", doc = "a state-variable filter: low-pass 12 / 24 dB, band-pass, high-pass, notch, with drive and key tracking")]
#[port(input, audio, name = "in", channels = "any", doc = "the signal")]
#[port(input, control, name = "cutoff_mod", doc = "moves the cutoff, in octaves")]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "the note the cutoff follows (key)")]
#[port(output, audio, name = "out", channels = "any", doc = "the filtered signal")]
#[preset(name = "acid", doc = "A steep, ringing low-pass with some drive that follows the keyboard half-way: sweep cutoff_mod with an envelope.", values(mode = 1.0, cutoff = 400.0, res = 75.0, drive = 30.0, key = 50.0))]
pub struct Filter {
    #[param(choices = "lp12|lp24|bp|hp|notch", default = 0, doc = "low-pass 12 or 24 dB an octave, band-pass, high-pass, notch")]
    pub mode: f32,
    #[param(range = 20.0..20000.0, default = 2000.0, curve = log, unit = "Hz", doc = "the cutoff frequency")]
    pub cutoff: f32,
    #[param(range = 0.0..100.0, default = 10.0, unit = "%", doc = "resonance: the peak at the cutoff, close to ringing at the top")]
    pub res: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "how hard the signal is pushed into a soft clip before the filter")]
    pub drive: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", doc = "key tracking: how far the cutoff follows the last note (100 = an octave an octave)")]
    pub key: f32,
    /// two filters a channel: the second only for 24 dB
    stages: [[Svf; 2]; CHANNELS],
    /// the cutoff as octaves (log2 Hz) and the resonance 0 … 1, gliding
    octave: Smooth,
    peak: Smooth,
    /// the last note's distance from middle C in octaves
    note: f32,
    plain: Coef,
    tuned: Coef,
    slew: f32,
    clock: usize,
    rate: f64,
}

impl Filter {
    fn cutoff_octave(&self, moved: f32) -> f32 {
        log2(f64::from(self.cutoff)) as f32 + moved + self.key / 100.0 * self.note
    }

    fn track(&mut self, e: &Ev) {
        if e.kind() == Some(Kind::NoteOn) && e.pitch_hz().is_finite() && e.pitch_hz() > 0.0 {
            self.note = log2(f64::from(e.pitch_hz()) / C4) as f32;
        }
    }

    /// the coefficients for the next `CTRL` samples
    fn control(&mut self, moved: f32) {
        self.octave.to(self.cutoff_octave(moved));
        self.peak.to(self.res / 100.0);
        let hz = exp2(f64::from(self.octave.step(self.slew)));
        self.tuned = Coef::new(hz, self.peak.step(self.slew), self.rate);
        self.plain = Coef::new(hz, 0.0, self.rate);
    }
}

impl Module for Filter {
    fn init(&mut self, ctx: &Ctx) {
        self.rate = ctx.rate().max(1.0);
        self.slew = Smooth::coef(SLEW, CTRL as f32 / self.rate as f32);
        self.octave.jump(self.cutoff_octave(0.0));
        self.peak.jump(self.res / 100.0);
    }

    fn process(&mut self, block: &mut Block, _: &Ctx) {
        let events = block.events_in(Self::NOTES);
        let moved = block.control_in(Self::CUTOFF_MOD);
        let (steep, mode) = match self.mode.round() as i32 {
            0 => (false, Mode::Lp),
            1 => (true, Mode::Lp),
            2 => (false, Mode::Bp),
            3 => (false, Mode::Hp),
            _ => (false, Mode::Notch),
        };
        let drive = self.drive / 100.0;
        let inp = block.audio_in(Self::IN);
        let n = block.frames();
        let mut out = block.audio_out(Self::OUT);
        let (mut i, mut next) = (0, 0);
        while i < n {
            while let Some(e) = events.get(next).filter(|e| e.frame as usize <= i) {
                self.track(e);
                next += 1;
            }
            if self.clock % CTRL == 0 {
                self.control(moved);
            }
            // up to the next update or the next note, whichever is first
            let end = n.min(i + CTRL - self.clock % CTRL).min(events.get(next).map_or(n, |e| e.frame as usize));
            for c in 0..out.ch().min(CHANNELS) {
                // a mono signal goes to every channel
                let src = &inp.chan(if inp.ch() == 1 { 0 } else { c })[i..end];
                let [first, second] = &mut self.stages[c];
                for (o, x) in out.chan(c)[i..end].iter_mut().zip(src) {
                    let x = svf::drive(*x, drive);
                    *o = if steep { second.tick(&self.tuned, mode, first.tick(&self.plain, mode, x)) } else { first.tick(&self.tuned, mode, x) };
                }
            }
            self.clock = self.clock.wrapping_add(end - i);
            i = end;
        }
        for e in &events[next.min(events.len())..] {
            self.track(e);
        }
    }
}
