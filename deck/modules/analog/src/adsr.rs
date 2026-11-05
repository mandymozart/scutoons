//! The ADSR envelope. Attack is linear to 1; decay and release are exponential and take exactly their time: the level
//! has fallen to 0.1 % of the way's distance at the end (an analog RC's "time to 99.9 %"), then it is snapped to the
//! target. Times are read when a stage begins; the sustain level is live.
use super::math::exp2;

/// log2(0.001): the exponent that makes a decay of `n` samples end 0.1 % short of its target
const LOG2_THOUSANDTH: f64 = -9.965_784_284_662_087;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A, D and R in seconds, S as a level 0..1
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Times {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    /// samples into the stage and its length
    at: u32,
    len: u32,
    /// the exponential stages: the factor per sample; the attack: the step
    coef: f32,
    /// where the attack started (a retrigger rises from the level it found)
    from: f32,
}

fn samples(seconds: f32, rate: f64) -> u32 {
    (f64::from(seconds.max(0.0)) * rate).round().max(1.0) as u32
}

impl Adsr {
    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    /// the gate rises: attack from the current level
    pub fn gate_on(&mut self, t: &Times, rate: f64) {
        self.from = self.level;
        self.begin(Stage::Attack, samples(t.attack, rate), 0.0);
    }

    /// the gate falls: release from the current level (nothing to do when idle)
    pub fn gate_off(&mut self, t: &Times, rate: f64) {
        if self.stage != Stage::Idle {
            self.release(t, rate);
        }
    }

    fn release(&mut self, t: &Times, rate: f64) {
        let len = samples(t.release, rate);
        self.begin(Stage::Release, len, exp2(LOG2_THOUSANDTH / f64::from(len)) as f32);
    }

    fn begin(&mut self, stage: Stage, len: u32, coef: f32) {
        self.stage = stage;
        self.at = 0;
        self.len = len;
        self.coef = coef;
    }

    /// one sample; returns the level
    pub fn tick(&mut self, t: &Times, rate: f64) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.at += 1;
                if self.at >= self.len {
                    self.level = 1.0;
                    let len = samples(t.decay, rate);
                    self.begin(Stage::Decay, len, exp2(LOG2_THOUSANDTH / f64::from(len)) as f32);
                } else {
                    self.level = self.from + (1.0 - self.from) * (self.at as f32 / self.len as f32);
                }
            }
            Stage::Decay => {
                self.at += 1;
                let s = t.sustain.clamp(0.0, 1.0);
                if self.at >= self.len {
                    self.level = s;
                    self.stage = Stage::Sustain;
                } else {
                    self.level = s + (self.level - s) * self.coef;
                }
            }
            Stage::Sustain => self.level = t.sustain.clamp(0.0, 1.0),
            Stage::Release => {
                self.at += 1;
                if self.at >= self.len {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                } else {
                    self.level *= self.coef;
                }
            }
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;
    const T: Times = Times { attack: 0.01, decay: 0.2, sustain: 0.4, release: 0.5 };

    /// run the envelope with the gate held for `held` samples, then released, and return every level
    fn run(t: &Times, held: usize, total: usize) -> Vec<f32> {
        let mut e = Adsr::default();
        e.gate_on(t, RATE);
        (0..total)
            .map(|i| {
                if i == held {
                    e.gate_off(t, RATE);
                }
                e.tick(t, RATE)
            })
            .collect()
    }

    #[test]
    fn the_attack_takes_its_time_and_is_linear() {
        let x = run(&T, 100_000, 1000);
        let a = (T.attack as f64 * RATE).round() as usize; // 480
        assert!((x[a / 2 - 1] - 0.5).abs() < 0.01, "half-way through the attack: {}", x[a / 2 - 1]);
        assert!(x[a - 2] < 1.0 && x[a - 1] == 1.0, "peaks at sample {a}: {} {}", x[a - 2], x[a - 1]);
        assert!(x[..a].windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn the_decay_ends_on_the_sustain_after_its_time() {
        let x = run(&T, 100_000, 20_000);
        let (a, d) = (480usize, 9600usize);
        // the decay's exponential: at its end only 0.1 % of the way is left
        let end = x[a + d - 2];
        assert!((end - T.sustain).abs() < 0.0012 * (1.0 - T.sustain) + 1e-4, "{end}");
        assert_eq!(x[a + d], T.sustain, "snapped to the sustain");
        assert!(x[a..a + d].windows(2).all(|w| w[1] <= w[0]), "falls monotonically");
        // half-way (in time) it is far from done: an exponential falls fast first
        assert!(x[a + d / 2] < T.sustain + 0.05 * (1.0 - T.sustain));
    }

    #[test]
    fn the_sustain_holds_and_follows_the_knob() {
        let mut e = Adsr::default();
        e.gate_on(&T, RATE);
        for _ in 0..20_000 {
            e.tick(&T, RATE);
        }
        assert_eq!(e.stage(), Stage::Sustain);
        assert_eq!(e.level(), 0.4);
        let t2 = Times { sustain: 0.9, ..T };
        assert_eq!(e.tick(&t2, RATE), 0.9);
    }

    #[test]
    fn the_release_falls_from_where_it_was_released_in_its_time() {
        let held = 20_000;
        let x = run(&T, held, held + 30_000);
        let r = (T.release as f64 * RATE) as usize; // 24000
        let start = x[held - 1];
        assert_eq!(start, 0.4);
        let end = x[held + r - 2];
        assert!((end / start - 0.001).abs() < 0.0001, "0.1 % left at the end: {}", end / start);
        assert_eq!(x[held + r - 1], 0.0, "silent and idle at the end of the release");
        assert_eq!(x[held + r + 100], 0.0);
        let mut e = Adsr::default();
        e.gate_on(&T, RATE);
        for _ in 0..held {
            e.tick(&T, RATE);
        }
        e.gate_off(&T, RATE);
        for _ in 0..r {
            assert!(!e.idle());
            e.tick(&T, RATE);
        }
        assert!(e.idle());
    }

    #[test]
    fn releasing_during_the_attack_falls_from_the_level_reached() {
        let x = run(&T, 240, 2000); // half-way through the 480-sample attack
        let peak = x[239];
        assert!((peak - 0.5).abs() < 0.01);
        assert!(x[240] < peak && x[240] > peak * 0.99, "continues from {peak}: {}", x[240]);
        assert!(x[240..].windows(2).all(|w| w[1] <= w[0]));
    }

    #[test]
    fn a_retrigger_rises_from_the_level_it_found() {
        let mut e = Adsr::default();
        e.gate_on(&T, RATE);
        for _ in 0..20_000 {
            e.tick(&T, RATE);
        }
        e.gate_on(&T, RATE);
        let first = e.tick(&T, RATE);
        assert!(first > 0.4 && first < 0.45, "no dip to zero: {first}");
    }

    #[test]
    fn a_gate_off_while_idle_does_nothing() {
        let mut e = Adsr::default();
        e.gate_off(&T, RATE);
        assert!(e.idle() && e.tick(&T, RATE) == 0.0);
    }
}
