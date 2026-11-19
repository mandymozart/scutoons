//! The UI's own short tones (a node focused in the editor's node view): the frame thread asks for one, the engine's audio
//! thread plays it on top of the output, a sine with a quick attack and an exponential tail. Lock-free and allocation-free
//! on the audio side; a newer ask replaces the one before (latest wins).
use crate::math::sin_turns;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// how long a tone rises and how fast it dies away (s)
pub const ATTACK: f32 = 0.004;
pub const DECAY: f32 = 0.045;
/// the loudest a tone may be (a full volume setting)
pub const MAX_GAIN: f32 = 0.25;
/// below this the voice is silent and ends
const FLOOR: f32 = 1e-4;

/// The frame thread's side: the newest ask (pitch and gain) and how many asks there were.
#[derive(Debug, Default)]
pub struct Tones {
    asked: AtomicU32,
    what: AtomicU64,
}

impl Tones {
    /// a tone at `hz`, `volume` 0..1 of `MAX_GAIN`
    pub fn play(&self, hz: f32, volume: f32) {
        let gain = volume.clamp(0.0, 1.0) * MAX_GAIN;
        if !(hz.is_finite() && hz > 0.0) || gain <= 0.0 {
            return;
        }
        self.what.store(((hz.to_bits() as u64) << 32) | gain.to_bits() as u64, Ordering::Relaxed);
        self.asked.fetch_add(1, Ordering::Release);
    }

    fn take(&self, seen: &mut u32) -> Option<(f32, f32)> {
        let n = self.asked.load(Ordering::Acquire);
        if n == *seen {
            return None;
        }
        *seen = n;
        let w = self.what.load(Ordering::Relaxed);
        Some((f32::from_bits((w >> 32) as u32), f32::from_bits(w as u32)))
    }
}

/// The audio thread's side: the tone that sounds now.
#[derive(Debug, Clone)]
pub struct Voice {
    rate: f32,
    seen: u32,
    phase: f32,
    inc: f32,
    gain: f32,
    /// the envelope: rising to 1, then each sample times `fall`
    env: f32,
    rise: f32,
    fall: f32,
    rising: bool,
}

impl Voice {
    pub fn new(rate: f32) -> Voice {
        let rate = rate.max(1.0);
        Voice { rate, seen: 0, phase: 0.0, inc: 0.0, gain: 0.0, env: 0.0, rise: 1.0 / (ATTACK * rate), fall: libm::expf(-1.0 / (DECAY * rate)), rising: false }
    }

    pub fn sounding(&self) -> bool {
        self.rising || self.env > FLOOR
    }

    /// The next `n` samples of the tone times `scale`, added onto every channel slice of `outs` (one mono tone on all). A
    /// new ask starts the tone again from where its envelope is, so a quick Tab run does not click.
    pub fn add(&mut self, tones: &Tones, outs: &mut [&mut [f32]], n: usize, scale: f32) {
        if let Some((hz, gain)) = tones.take(&mut self.seen) {
            self.inc = hz / self.rate;
            self.gain = gain;
            self.rising = true;
        }
        if !self.sounding() {
            self.env = 0.0;
            return;
        }
        for i in 0..n {
            if self.rising {
                self.env += self.rise;
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.rising = false;
                }
            } else {
                self.env *= self.fall;
            }
            let s = sin_turns(self.phase) * self.env * self.gain * scale;
            self.phase += self.inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            for o in outs.iter_mut() {
                if let Some(x) = o.get_mut(i) {
                    *x += s;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(v: &mut Voice, t: &Tones, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; frames];
        for chunk in out.chunks_mut(128) {
            let n = chunk.len();
            v.add(t, &mut [chunk], n, 1.0);
        }
        out
    }

    #[test]
    fn a_tone_sounds_short_at_its_pitch_and_ends() {
        let (t, mut v) = (Tones::default(), Voice::new(48_000.0));
        assert!(render(&mut v, &t, 256).iter().all(|&x| x == 0.0), "silent until asked");
        t.play(1000.0, 1.0);
        let out = render(&mut v, &t, 48_000);
        let peak = out.iter().fold(0.0f32, |p, x| p.max(x.abs()));
        assert!(peak > MAX_GAIN * 0.9 && peak <= MAX_GAIN + 1e-4, "peak {peak}");
        let crossings = out[..4800].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((95..=105).contains(&crossings), "≈100 cycles in 0.1 s at 1 kHz: {crossings}");
        assert!(out[24_000..].iter().all(|x| x.abs() < 1e-3), "gone after half a second");
        assert!(!v.sounding());
    }

    #[test]
    fn the_newest_ask_wins_and_a_silent_one_is_ignored() {
        let (t, mut v) = (Tones::default(), Voice::new(48_000.0));
        t.play(500.0, 0.0);
        t.play(f32::NAN, 1.0);
        assert!(render(&mut v, &t, 512).iter().all(|&x| x == 0.0));
        t.play(300.0, 1.0);
        t.play(2000.0, 0.5);
        let out = render(&mut v, &t, 4800);
        let crossings = out.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((190..=210).contains(&crossings), "2 kHz, not 300 Hz: {crossings}");
        assert!(out.iter().fold(0.0f32, |p, x| p.max(x.abs())) <= MAX_GAIN * 0.5 + 1e-4);
    }

    #[test]
    fn it_adds_onto_every_channel() {
        let (t, mut v) = (Tones::default(), Voice::new(48_000.0));
        let (mut l, mut r) = (vec![0.5f32; 128], vec![0.0f32; 128]);
        t.play(440.0, 1.0);
        v.add(&t, &mut [&mut l, &mut r], 128, 1.0);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - 0.5 - b).abs() < 1e-6), "the same tone on both");
        assert!(r.iter().any(|x| x.abs() > 0.01));
    }
}
