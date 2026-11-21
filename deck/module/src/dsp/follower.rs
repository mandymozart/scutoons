//! The envelope follower: the size of a signal as a level that rises with `attack` and falls with `release` (each the
//! time to cover 63 % of the way) – the bridge from sound to a control value.
use super::flush;
use crate::smooth::Smooth;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Follower {
    level: f32,
    up: f32,
    down: f32,
}

impl Follower {
    /// `attack` and `release` in seconds at `rate` Hz (0 = at once)
    pub fn new(attack: f32, release: f32, rate: f32) -> Follower {
        let mut f = Follower::default();
        f.times(attack, release, rate);
        f
    }

    /// new times; the level stays
    pub fn times(&mut self, attack: f32, release: f32, rate: f32) {
        let dt = if rate > 0.0 { 1.0 / rate } else { 0.0 };
        self.up = Smooth::coef(attack, dt);
        self.down = Smooth::coef(release, dt);
    }

    /// one sample in, the level out (≥ 0)
    #[inline(always)]
    pub fn tick(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let k = if a > self.level { self.up } else { self.down };
        self.level = flush(self.level + k * (a - self.level));
        self.level
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn reset(&mut self) {
        self.level = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    #[test]
    fn it_rises_in_the_attack_time_and_falls_in_the_release_time() {
        let (attack, release) = (0.01, 0.2);
        let mut f = Follower::new(attack, release, RATE);
        let e = core::f32::consts::E;
        let up: Vec<f32> = (0..4800).map(|_| f.tick(1.0)).collect();
        assert!((up[479] - (1.0 - 1.0 / e)).abs() < 0.002, "63 % after the attack time: {}", up[479]);
        assert!(up.windows(2).all(|w| w[1] >= w[0]) && up[4799] > 0.9999);
        let down: Vec<f32> = (0..48_000).map(|_| f.tick(0.0)).collect();
        assert!((down[9599] - 1.0 / e).abs() < 0.002, "37 % left after the release time: {}", down[9599]);
        assert!(down.windows(2).all(|w| w[1] <= w[0]));
    }

    #[test]
    fn it_follows_the_size_of_a_tone_whatever_its_sign() {
        for amp in [0.1f32, 0.5, 1.0] {
            let mut f = Follower::new(0.001, 0.3, RATE);
            let mut last = 0.0;
            for i in 0..48_000 {
                last = f.tick(-amp * libm::sinf(core::f32::consts::TAU * 440.0 * i as f32 / RATE));
            }
            assert!(last > 0.85 * amp && last <= amp, "amplitude {amp}: {last}");
        }
    }

    #[test]
    fn zero_times_follow_at_once_and_silence_ends_at_zero() {
        let mut f = Follower::new(0.0, 0.0, RATE);
        assert_eq!(f.tick(-0.7), 0.7);
        assert!((f.tick(0.2) - 0.2).abs() < 1e-6);
        let mut f = Follower::new(0.001, 0.005, RATE);
        f.tick(1.0);
        for _ in 0..100_000 {
            let y = f.tick(0.0);
            assert!(y == 0.0 || y.is_normal(), "{y:e}");
        }
        assert_eq!(f.level(), 0.0);
    }
}
