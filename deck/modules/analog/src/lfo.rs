//! The LFO: five shapes, a free rate in Hz or locked to the tempo, a delay before it starts and an attack that fades it
//! in. Output −1..1. Free, it starts at phase 0 with the note; locked, its phase is the tempo's beat position (the
//! same on every voice and every deck that plays the same tempo map), sample & hold is seeded.
use super::math::sin_turns;
use super::noise::Noise;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Sine,
    Tri,
    /// a rising ramp
    Saw,
    Square,
    SampleHold,
}

impl Shape {
    /// from a parameter value: 0 sine, 1 triangle, 2 saw, 3 square, 4 sample & hold
    pub fn from_param(v: f32) -> Shape {
        match v.round() as i32 {
            0 => Shape::Sine,
            1 => Shape::Tri,
            2 => Shape::Saw,
            3 => Shape::Square,
            _ => Shape::SampleHold,
        }
    }
}

/// beats in one cycle for a sync setting: 0 off (free), then 2 bars, 1 bar, 1/2, 1/4 (a beat), 1/8, 1/16, 1/32 note
pub const SYNC_BEATS: [f64; 8] = [0.0, 8.0, 4.0, 2.0, 1.0, 0.5, 0.25, 0.125];

pub fn beats_per_cycle(sync: f32) -> f64 {
    SYNC_BEATS[(sync.round() as usize).min(SYNC_BEATS.len() - 1)]
}

/// One call's settings.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub shape: Shape,
    pub rate_hz: f32,
    /// 0 = free; otherwise the beats in a cycle
    pub beats: f64,
    pub delay: f32,
    pub attack: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Lfo {
    phase: f64,
    last: f64,
    held: f32,
    /// samples since the note began
    age: u64,
    noise: Noise,
}

impl Lfo {
    pub fn start(&mut self, seed: u64) {
        *self = Lfo { noise: Noise::new(seed), ..Lfo::default() };
        self.held = self.noise.next();
    }

    /// One sample at the absolute `beat` of the tempo map.
    pub fn tick(&mut self, s: &Settings, rate: f64, beat: f64) -> f32 {
        let phase = if s.beats > 0.0 {
            let c = beat / s.beats;
            c - c.floor()
        } else {
            let p = self.phase;
            let next = p + f64::from(s.rate_hz) / rate;
            self.phase = next - next.floor();
            p
        };
        if phase < self.last {
            self.held = self.noise.next();
        }
        self.last = phase;
        let v = match s.shape {
            Shape::Sine => sin_turns(phase) as f32,
            Shape::Tri => {
                let u = phase + 0.25;
                (1.0 - 4.0 * (u - u.floor() - 0.5).abs()) as f32
            }
            Shape::Saw => (2.0 * phase - 1.0) as f32,
            Shape::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Shape::SampleHold => self.held,
        };
        let delay = (f64::from(s.delay.max(0.0)) * rate) as u64;
        let fade = if self.age < delay {
            0.0
        } else if s.attack > 0.0 {
            (((self.age - delay) as f64 / (f64::from(s.attack) * rate)).min(1.0)) as f32
        } else {
            1.0
        };
        self.age += 1;
        v * fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn set(shape: Shape, hz: f32) -> Settings {
        Settings { shape, rate_hz: hz, beats: 0.0, delay: 0.0, attack: 0.0 }
    }

    fn run(s: &Settings, n: usize) -> Vec<f32> {
        let mut l = Lfo::default();
        l.start(1);
        (0..n).map(|i| l.tick(s, RATE, i as f64 * 2.0 / RATE)).collect()
    }

    #[test]
    fn every_shape_has_the_rate_it_is_set_to() {
        for shape in [Shape::Sine, Shape::Tri, Shape::Saw, Shape::Square] {
            let x = run(&set(shape, 5.0), 48_000);
            // the rising zero crossings (the square crosses at its step) are a period apart: 5 Hz = 9600 samples
            let up: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).collect();
            assert!(up.len() >= 4, "{shape:?}");
            assert!(up.windows(2).all(|w| w[1] - w[0] == 9600), "{shape:?}: {up:?}");
            assert!(x.iter().all(|v| v.abs() <= 1.0));
        }
    }

    #[test]
    fn the_shapes_are_what_they_are_called() {
        let at = |shape, i| run(&set(shape, 1.0), 48_000)[i];
        assert!(at(Shape::Sine, 0).abs() < 1e-6 && (at(Shape::Sine, 12_000) - 1.0).abs() < 1e-6, "sine");
        assert!((at(Shape::Tri, 12_000) - 1.0).abs() < 1e-6 && (at(Shape::Tri, 36_000) + 1.0).abs() < 1e-6, "triangle");
        assert!((at(Shape::Saw, 0) + 1.0).abs() < 1e-6 && at(Shape::Saw, 47_999) > 0.999, "saw rises");
        assert_eq!((at(Shape::Square, 100), at(Shape::Square, 30_000)), (1.0, -1.0), "square");
    }

    #[test]
    fn sample_and_hold_holds_a_new_seeded_value_each_cycle() {
        let x = run(&set(Shape::SampleHold, 10.0), 48_000);
        let mut values: Vec<f32> = x.clone();
        values.dedup();
        assert!(values.len() >= 9 && values.len() <= 11, "{} held values in 10 cycles", values.len());
        assert_eq!(x, run(&set(Shape::SampleHold, 10.0), 48_000), "the same seed holds the same values");
        let mut other = Lfo::default();
        other.start(2);
        assert_ne!(x[0], other.tick(&set(Shape::SampleHold, 10.0), RATE, 0.0));
    }

    #[test]
    fn delay_and_attack_hold_and_fade_it_in() {
        let s = Settings { shape: Shape::Square, rate_hz: 1.0, beats: 0.0, delay: 0.5, attack: 0.25 };
        let x = run(&s, 48_000);
        assert!(x[..24_000].iter().all(|&v| v == 0.0), "silent through the delay");
        // after the delay it fades in over the attack: half-way through it, half strength (the square is still high)
        assert!((x[24_000 + 6_000].abs() - 0.5).abs() < 0.001, "{}", x[30_000]);
        assert_eq!(x[24_000 + 12_000].abs(), 1.0);
        assert!(x[24_000 + 11_999].abs() < 1.0, "just short of full");
    }

    #[test]
    fn locked_to_the_tempo_it_follows_the_beat_not_the_note() {
        // one cycle a beat: 120 bpm = 2 beats a second = 2 Hz; the phase is the beat's fraction, whenever the note began
        let s = Settings { shape: Shape::Saw, rate_hz: 99.0, beats: 1.0, delay: 0.0, attack: 0.0 };
        let (mut a, mut b) = (Lfo::default(), Lfo::default());
        a.start(1);
        b.start(1);
        for i in 0..5000 {
            b.tick(&s, RATE, i as f64 / 24_000.0); // b has been running longer
        }
        for i in 0..100 {
            let beat = 1000.25 + i as f64 / 24_000.0;
            let (va, vb) = (a.tick(&s, RATE, beat), b.tick(&s, RATE, beat));
            assert!((va - vb).abs() < 1e-6, "same beat, same phase");
            assert!((f64::from(va) - (2.0 * (beat - beat.floor()) - 1.0)).abs() < 1e-5);
        }
        assert_eq!(beats_per_cycle(0.0), 0.0);
        assert_eq!(beats_per_cycle(4.0), 1.0);
        assert_eq!(beats_per_cycle(9.0), 0.125, "clamped to the last division");
    }
}
