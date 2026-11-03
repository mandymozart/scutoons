//! The output's soft limiter (#188, owner 2026-10-31: "a soft cap at the master … so overloaded parameters don't
//! crackle", "with the gain reduction measured"). It replaces the hard `clamp(-1, 1)` in front of the device: several
//! voices, a nudge "voice louder" and a high FM index summed over 1 used to clip – a crackle. Native (deck-native's
//! Engine) and the browser (live-core) run the same one, last before the device.
//!
//! - **Look-ahead:** the signal is delayed by LOOK samples (1.3 ms at 48 kHz); a peak follower on the undelayed input
//!   (instant attack, `RELEASE` back) asks for `CEILING / peak`, and the gain glides down within the look-ahead – it has
//!   reached the peak's gain by the time the peak comes out – and back up over `RELEASE`. Below the ceiling the gain
//!   stays 1: the output is the input, LOOK samples later (transparent).
//! - **Soft clip:** what the glide misses (a peak steeper than the look-ahead) is bent above `KNEE` by a tanh toward 1 –
//!   the output never reaches 1. Not finite = silence (a broken patch never blasts the speakers).
//! - **Gain reduction:** the deepest since the last `take_reduction`, in dB (≥ 0) – the HUD, the being's red face.
//!
//! Never allocates (a fixed ring).

/// the look-ahead (samples) – the output's extra latency
pub const LOOK: usize = 64;
/// the peaks are held to this (≈ −1 dBFS)
pub const CEILING: f32 = 0.891;
/// the soft clip begins here
pub const KNEE: f32 = 0.95;
/// how long the gain takes back up (s)
pub const RELEASE: f32 = 0.15;

pub struct Limiter {
    /// the delayed frames (left, right), a ring
    ring: [[f32; 2]; LOOK],
    at: usize,
    /// the peak follower and the gain
    env: f32,
    gain: f32,
    /// per sample: the gain's glide down (within the look-ahead), the follower's and the gain's way back
    down: f32,
    back: f32,
    /// the lowest gain since the last `take_reduction`
    least: f32,
}

impl Limiter {
    pub fn new(rate: f32) -> Limiter {
        let coef = |samples: f32| 1.0 - libm::expf(-1.0 / samples.max(1.0));
        Limiter { ring: [[0.0; 2]; LOOK], at: 0, env: 0.0, gain: 1.0, down: coef(LOOK as f32 / 5.0), back: coef(RELEASE * rate), least: 1.0 }
    }

    /// one frame in, the frame LOOK samples ago out (limited)
    #[inline]
    fn frame(&mut self, l: f32, r: f32) -> (f32, f32) {
        let (l, r) = (finite(l), finite(r));
        let peak = l.abs().max(r.abs());
        self.env = if peak > self.env { peak } else { self.env + (peak - self.env) * self.back };
        let want = if self.env > CEILING { CEILING / self.env } else { 1.0 };
        self.gain += (want - self.gain) * if want < self.gain { self.down } else { self.back };
        self.least = self.least.min(self.gain);
        let [dl, dr] = std::mem::replace(&mut self.ring[self.at], [l, r]);
        self.at = (self.at + 1) % LOOK;
        (soft(dl * self.gain), soft(dr * self.gain))
    }

    /// A stereo block in place.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            (*a, *b) = self.frame(*a, *b);
        }
    }

    /// A mono block in place (the browser's).
    pub fn process_mono(&mut self, x: &mut [f32]) {
        for a in x.iter_mut() {
            *a = self.frame(*a, *a).0;
        }
    }

    /// The deepest gain reduction since the last call, in dB (0 = untouched); starts counting anew.
    pub fn take_reduction(&mut self) -> f32 {
        let db = -20.0 * libm::log10f(self.least.max(1e-6));
        self.least = self.gain;
        db.max(0.0)
    }

    /// the gain reduction now, in dB
    pub fn reduction(&self) -> f32 {
        (-20.0 * libm::log10f(self.gain.max(1e-6))).max(0.0)
    }
}

/// The same limiter over a group of n channels with one linked gain (plan 2.6, #318): the peak follower hears the loudest
/// channel of the group, so the gain it takes off lowers every channel by the same amount and the image does not shift. The
/// device's outlet groups (a stereo pair, a 5.1 bed, a cue pair) each get one. For two channels it is `Limiter`, bit for bit.
/// Planar in place: channel `c` of a block is `data[c * stride..]`. Never allocates after `new`.
pub struct LinkedLimiter {
    n: usize,
    /// the delayed frames, LOOK of n samples
    ring: Vec<f32>,
    at: usize,
    env: f32,
    gain: f32,
    down: f32,
    back: f32,
    least: f32,
}

impl LinkedLimiter {
    pub fn new(rate: f32, n: usize) -> LinkedLimiter {
        let coef = |samples: f32| 1.0 - libm::expf(-1.0 / samples.max(1.0));
        LinkedLimiter { n, ring: vec![0.0; LOOK * n], at: 0, env: 0.0, gain: 1.0, down: coef(LOOK as f32 / 5.0), back: coef(RELEASE * rate), least: 1.0 }
    }

    pub fn channels(&self) -> usize {
        self.n
    }

    /// `frames` frames of the channels `chans` of a planar block (`stride` apart), in place
    pub fn process_planar(&mut self, data: &mut [f32], stride: usize, chans: &[usize], frames: usize) {
        debug_assert_eq!(chans.len(), self.n);
        for i in 0..frames {
            let mut peak = 0.0f32;
            for &c in chans {
                let x = finite(data[c * stride + i]);
                data[c * stride + i] = x;
                peak = peak.max(x.abs());
            }
            self.env = if peak > self.env { peak } else { self.env + (peak - self.env) * self.back };
            let want = if self.env > CEILING { CEILING / self.env } else { 1.0 };
            self.gain += (want - self.gain) * if want < self.gain { self.down } else { self.back };
            self.least = self.least.min(self.gain);
            let slot = &mut self.ring[self.at * self.n..(self.at + 1) * self.n];
            for (k, &c) in chans.iter().enumerate() {
                let delayed = std::mem::replace(&mut slot[k], data[c * stride + i]);
                data[c * stride + i] = soft(delayed * self.gain);
            }
            self.at = (self.at + 1) % LOOK;
        }
    }

    /// The deepest gain reduction since the last call, in dB (0 = untouched); starts counting anew.
    pub fn take_reduction(&mut self) -> f32 {
        let db = -20.0 * libm::log10f(self.least.max(1e-6));
        self.least = self.gain;
        db.max(0.0)
    }
}

fn finite(x: f32) -> f32 {
    if x.is_finite() {
        x
    } else {
        0.0
    }
}

/// identity up to KNEE, then a tanh bend toward 1 (never reaching it)
#[inline]
pub fn soft(x: f32) -> f32 {
    let a = x.abs();
    if a <= KNEE {
        return x;
    }
    let y = KNEE + (1.0 - KNEE) * libm::tanhf((a - KNEE) / (1.0 - KNEE));
    y.min(0.999_999).copysign(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn sine(f: f32, a: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| a * (TAU * f * i as f32 / 48_000.0).sin()).collect()
    }

    /// below the knee the limiter is a delay – the same samples, LOOK later; nothing reduced
    #[test]
    fn transparent_below_the_ceiling() {
        let mut lim = Limiter::new(48_000.0);
        let x = sine(440.0, 0.85, 4800);
        let (mut l, mut r) = (x.clone(), x.clone());
        lim.process(&mut l, &mut r);
        assert!(l[..LOOK].iter().all(|v| *v == 0.0));
        for i in LOOK..x.len() {
            assert_eq!(l[i], x[i - LOOK], "sample {i}");
        }
        assert_eq!(l, r);
        assert_eq!(lim.take_reduction(), 0.0);
    }

    /// owner 2026-10-31: overload never crackles – the output never reaches 1, however hot the input, and the gain
    /// reduction says how hard it works
    #[test]
    fn the_output_never_exceeds_one() {
        let mut lim = Limiter::new(48_000.0);
        // a quiet start, then 4× too loud, then a single full-scale click out of silence, then garbage
        let mut x = sine(220.0, 0.3, 4800);
        x.extend(sine(220.0, 4.0, 9600));
        x.resize(x.len() + 2400, 0.0);
        x.push(30.0);
        x.resize(x.len() + 2400, 0.0);
        x.extend([f32::NAN, f32::INFINITY, -f32::INFINITY, 0.5]);
        let mut y = x.clone();
        lim.process_mono(&mut y);
        assert!(y.iter().all(|v| v.is_finite() && v.abs() < 1.0), "max {}", y.iter().fold(0.0f32, |m, v| m.max(v.abs())));
        let gr = lim.take_reduction();
        assert!(gr > 12.0 - 1.0, "a 4× signal held to the ceiling: {gr} dB");
        // the loud part settles at the ceiling, not bent by the clip: the gain did the work
        let settled = y[4800 + 4800..4800 + 9600].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((settled - CEILING).abs() < 0.02, "{settled}");
        // and it comes back: quiet again after the release, nothing reduced
        let mut z = sine(220.0, 0.3, 96_000);
        lim.process_mono(&mut z);
        let _ = lim.take_reduction();
        let mut z = sine(220.0, 0.3, 4800);
        lim.process_mono(&mut z);
        assert!(lim.take_reduction() < 0.05);
    }

    /// a group of two is the stereo limiter, bit for bit; a group of four takes the loudest channel's gain off all of them
    #[test]
    fn a_linked_group_limits_together() {
        let (mut l, r) = (sine(220.0, 1.6, 9600), sine(331.0, 0.2, 9600));
        let mut lin = Limiter::new(48_000.0);
        let (mut lo, mut ro) = (l.clone(), r.clone());
        lin.process(&mut lo, &mut ro);
        let mut planar = [l.clone(), r.clone()].concat();
        let mut group = LinkedLimiter::new(48_000.0, 2);
        group.process_planar(&mut planar, 9600, &[0, 1], 9600);
        assert!(planar[..9600].iter().zip(&lo).all(|(a, b)| a.to_bits() == b.to_bits()), "left");
        assert!(planar[9600..].iter().zip(&ro).all(|(a, b)| a.to_bits() == b.to_bits()), "right");
        assert_eq!(group.take_reduction(), lin.take_reduction());
        // four channels, only the first loud: the quiet ones are lowered by the same gain (the image keeps its balance)
        l = sine(220.0, 3.0, 9600);
        let quiet = sine(220.0, 0.3, 9600);
        let mut data = [l, quiet.clone(), quiet.clone(), quiet].concat();
        let mut g4 = LinkedLimiter::new(48_000.0, 4);
        g4.process_planar(&mut data, 9600, &[0, 1, 2, 3], 9600);
        let (loud, soft1, soft3) = (&data[..9600], &data[9600..2 * 9600], &data[3 * 9600..]);
        assert_eq!(soft1, soft3, "channels with the same input leave the same");
        let ratio = soft1[6000..8000].iter().fold(0.0f32, |m, v| m.max(v.abs())) / 0.3;
        assert!(ratio < 0.45, "the quiet channels are pulled down with the loud one: {ratio}");
        assert!(loud.iter().all(|v| v.abs() < 1.0));
        // a channel outside the group is not touched
        let mut two = [sine(220.0, 3.0, 4800), sine(220.0, 0.3, 4800)].concat();
        let mut solo = LinkedLimiter::new(48_000.0, 1);
        solo.process_planar(&mut two, 4800, &[1], 4800);
        assert!(two[..4800].iter().zip(sine(220.0, 3.0, 4800)).all(|(a, b)| *a == b), "channel 0 is outside the group");
    }

    #[test]
    fn the_soft_clip_is_monotonic_and_bounded() {
        assert_eq!(soft(0.5), 0.5);
        assert_eq!(soft(-KNEE), -KNEE);
        let mut last = 0.0;
        for i in 0..2000 {
            let v = soft(i as f32 * 0.01);
            assert!(v >= last && v < 1.0);
            last = v;
        }
        assert!(soft(-100.0) > -1.0);
    }
}
