//! The first generators (research/EVENTS.md §8): `random_walk` (the phone page's melody, seeded), `pulse` (the tempo
//! grid), `arp` (events → events). Each is pure – its params, a seed and the step number decide every event – so a
//! window gives the same events however the timeline is cut, and every deck with the same seed plays the same.
pub mod arp;
pub mod pulse;
pub mod random_walk;

pub use arp::{Arp, ArpMode};
pub use pulse::Pulse;
pub use random_walk::RandomWalk;

/// a uniform number in [0, 1) for (seed, step) – splitmix64, so step k's value never depends on the steps before
pub fn unit(seed: u64, step: i64) -> f64 {
    let mut z = seed ^ (step as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// the steps on a grid of `per_beat` steps a beat whose start falls in [from, to): (step, its beat)
pub fn steps(from: f64, to: f64, per_beat: f64) -> impl Iterator<Item = (i64, f64)> {
    let ok = per_beat > 0.0 && to > from;
    let first = if ok { (from * per_beat).ceil() as i64 } else { 0 };
    let end = if ok { (to * per_beat).ceil() as i64 } else { 0 };
    (first..end).map(move |k| (k, k as f64 / per_beat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_is_uniform_ish_and_pure() {
        assert_eq!(unit(42, 7), unit(42, 7));
        assert_ne!(unit(42, 7), unit(43, 7));
        let mean = (0..10_000).map(|k| unit(1, k)).sum::<f64>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
        assert!((0..1000).all(|k| (0.0..1.0).contains(&unit(9, k))));
    }

    #[test]
    fn steps_cover_a_span_once() {
        let a: Vec<i64> = steps(0.0, 1.0, 4.0).map(|(k, _)| k).collect();
        let b: Vec<i64> = steps(0.0, 0.3, 4.0).chain(steps(0.3, 1.0, 4.0)).map(|(k, _)| k).collect();
        assert_eq!(a, [0, 1, 2, 3]);
        assert_eq!(a, b, "cut anywhere, the same steps");
        assert_eq!(steps(1.0, 1.0, 4.0).count(), 0, "a stopped tempo has no steps");
    }
}
