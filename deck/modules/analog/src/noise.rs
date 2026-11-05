//! Seeded white noise: xorshift64*, uniform in [−1, 1). The seed is the module's (`Ctx::seed`) mixed with the note, so
//! two decks that play the same rig play the same noise; never a clock.

#[derive(Debug, Clone, Copy)]
pub struct Noise(u64);

impl Default for Noise {
    fn default() -> Self {
        Noise::new(0)
    }
}

/// splitmix64's finaliser: one well-mixed 64-bit number from any input
pub fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Noise {
    pub fn new(seed: u64) -> Noise {
        Noise(mix(seed) | 1)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r >> 40) as f32 * (1.0 / 8_388_608.0) - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_noise_other_seed_other_noise() {
        let (mut a, mut b, mut c) = (Noise::new(5), Noise::new(5), Noise::new(6));
        let xs: Vec<f32> = (0..64).map(|_| a.next()).collect();
        assert!(xs.iter().all(|&x| x == b.next()));
        assert!(xs.iter().any(|&x| x != c.next()));
    }

    #[test]
    fn is_white_uniform_and_bounded() {
        let mut n = Noise::new(1);
        let xs: Vec<f64> = (0..200_000).map(|_| n.next() as f64).collect();
        assert!(xs.iter().all(|&x| (-1.0..1.0).contains(&x)));
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
        assert!(mean.abs() < 0.01, "mean {mean}");
        assert!((var - 1.0 / 3.0).abs() < 0.01, "variance {var}");
        // white: neighbouring samples are uncorrelated
        let lag1 = xs.windows(2).map(|w| w[0] * w[1]).sum::<f64>() / (xs.len() - 1) as f64 / var;
        assert!(lag1.abs() < 0.01, "lag-1 correlation {lag1}");
    }
}
