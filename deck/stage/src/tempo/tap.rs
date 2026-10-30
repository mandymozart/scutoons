//! Tap tempo: the BPM from the taps' moments by least squares (a sloppy tap moves it less than an average of
//! intervals would), the last eight taps; a pause of two seconds starts a new count.
use super::map::{clamp_bpm, fit};

const KEEP: usize = 8;
const RESET_NS: u64 = 2_000_000_000;

#[derive(Default)]
pub struct Tap {
    taps: [u64; KEEP],
    n: usize,
}

impl Tap {
    /// a tap at `ns` (clock::now_ns); Some(bpm) from the second tap of a run on
    pub fn tap(&mut self, ns: u64) -> Option<f64> {
        if let Some(&last) = self.recent().last() {
            if ns <= last || ns - last > RESET_NS {
                self.n = 0;
            }
        }
        self.taps[self.n % KEEP] = ns;
        self.n += 1;
        let taps = self.recent();
        let first = *taps.first()?;
        let (_, spb) = fit(taps.iter().enumerate().map(|(i, &t)| (i as f64, (t - first) as f64 * 1e-9)))?;
        (spb > 0.0).then(|| clamp_bpm(60.0 / spb))
    }

    /// the taps of this run, oldest first
    fn recent(&self) -> Vec<u64> {
        let k = self.n.min(KEEP);
        (self.n - k..self.n).map(|i| self.taps[i % KEEP]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    #[test]
    fn taps_give_the_tempo() {
        let mut t = Tap::default();
        assert_eq!(t.tap(1_000_000_000), None, "one tap is no tempo");
        let mut bpm = None;
        // 120 BPM, each tap up to 15 ms early or late (a human)
        for i in 1..8u64 {
            bpm = t.tap((1_000_000_000 + i as i64 * 500_000_000 + (15e6 * jitter(i)) as i64) as u64);
        }
        let bpm = bpm.unwrap();
        assert!((bpm - 120.0).abs() < 2.0, "{bpm}");
    }

    #[test]
    fn a_pause_starts_a_new_count() {
        let mut t = Tap::default();
        t.tap(1_000_000_000);
        t.tap(1_500_000_000); // 120
        assert_eq!(t.tap(5_000_000_000), None, "after a pause the first tap counts again");
        let bpm = t.tap(6_000_000_000).unwrap();
        assert!((bpm - 60.0).abs() < 1e-6, "{bpm}");
    }

    #[test]
    fn it_keeps_the_last_eight_and_clamps() {
        let mut t = Tap::default();
        for i in 0..20u64 {
            t.tap(1_000_000_000 + i * 100_000_000); // 600 BPM → the ceiling
        }
        assert_eq!(t.tap(3_100_000_000), Some(300.0));
        let mut t = Tap::default();
        for i in 0..10u64 {
            t.tap(1_000_000_000 + i * 600_000_000); // 100 BPM
        }
        let bpm = t.tap(1_000_000_000 + 10 * 600_000_000).unwrap();
        assert!((bpm - 100.0).abs() < 1e-6, "{bpm}");
    }
}
