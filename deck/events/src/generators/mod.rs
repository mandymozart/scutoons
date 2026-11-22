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

/// the steps on a grid of `per_beat` steps a beat whose start falls in [from, to): (step, its beat) – the arp module's
pub use deck_module::dsp::arp::steps;

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
}
