//! The envelope with straight segments: attack, decay and release are lines that take exactly their time (`adsr` bends
//! decay and release exponentially). The same gate, `Times` and stages, so a module switches between the two shapes.
use super::adsr::{samples, Stage, Times};

#[derive(Debug, Clone, Copy, Default)]
pub struct Line {
    stage: Stage,
    level: f32,
    /// the level's change a sample and the samples left in the stage
    step: f32,
    left: u32,
}

impl Line {
    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    fn begin(&mut self, stage: Stage, to: f32, seconds: f32, rate: f64) {
        self.stage = stage;
        self.left = samples(seconds, rate);
        self.step = (to - self.level) / self.left as f32;
    }

    /// the gate rises: attack from the current level
    pub fn gate_on(&mut self, t: &Times, rate: f64) {
        self.begin(Stage::Attack, 1.0, t.attack, rate);
    }

    /// the gate falls: release from the current level (nothing to do when idle)
    pub fn gate_off(&mut self, t: &Times, rate: f64) {
        if self.stage != Stage::Idle {
            self.begin(Stage::Release, 0.0, t.release, rate);
        }
    }

    /// one sample; returns the level
    pub fn tick(&mut self, t: &Times, rate: f64) -> f32 {
        let sustain = t.sustain.clamp(0.0, 1.0);
        match self.stage {
            Stage::Idle => {}
            Stage::Sustain => self.level = sustain,
            Stage::Attack | Stage::Decay | Stage::Release => {
                self.left -= 1;
                self.level += self.step;
                if self.left == 0 {
                    match self.stage {
                        Stage::Attack => {
                            self.level = 1.0;
                            self.begin(Stage::Decay, sustain, t.decay, rate);
                        }
                        Stage::Decay => {
                            self.level = sustain;
                            self.stage = Stage::Sustain;
                        }
                        _ => {
                            self.level = 0.0;
                            self.stage = Stage::Idle;
                        }
                    }
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
    const T: Times = Times { attack: 0.01, decay: 0.02, sustain: 0.5, release: 0.04 };

    fn run(held: usize, total: usize) -> Vec<f32> {
        let mut e = Line::default();
        e.gate_on(&T, RATE);
        (0..total)
            .map(|i| {
                if i == held {
                    e.gate_off(&T, RATE);
                }
                e.tick(&T, RATE)
            })
            .collect()
    }

    #[test]
    fn every_segment_is_a_line_that_takes_its_time() {
        let x = run(4000, 8000);
        let (a, d, r) = (480, 960, 1920);
        assert!((x[a / 2 - 1] - 0.5).abs() < 1e-3 && x[a - 1] == 1.0);
        assert!((x[a + d / 2 - 1] - 0.75).abs() < 1e-3, "half-way down the decay: {}", x[a + d / 2 - 1]);
        assert_eq!((x[a + d - 1], x[3999]), (0.5, 0.5));
        assert!((x[4000 + r / 2 - 1] - 0.25).abs() < 1e-3, "half-way down the release: {}", x[4000 + r / 2 - 1]);
        assert_eq!((x[4000 + r - 1], x[7999]), (0.0, 0.0));
        assert!(x[4000 + r - 2] > 0.0);
    }

    #[test]
    fn a_retrigger_rises_from_the_level_it_found_and_idle_ignores_a_gate_off() {
        let mut e = Line::default();
        e.gate_off(&T, RATE);
        assert!(e.idle() && e.tick(&T, RATE) == 0.0);
        e.gate_on(&T, RATE);
        for _ in 0..3000 {
            e.tick(&T, RATE);
        }
        assert_eq!(e.stage(), Stage::Sustain);
        e.gate_on(&T, RATE);
        let first = e.tick(&T, RATE);
        assert!(first > 0.5 && first < 0.51, "{first}");
    }
}
