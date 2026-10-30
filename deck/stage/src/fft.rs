//! Sound → control values (the FFT node's frame-thread half): the audio callback pushes the mono sum into a ring
//! (audio/blocks.rs FftTap), the analyzer drains it every frame into deck-core's `analysis::Analyzer` (#185), which
//! turns the newest window into level / bass / mid / high in 0–1 (dB-scaled; a fast rise and a `release` in seconds).
//! The window ends at the heard moment, not at the newest sample: the callback renders ahead of the speaker by the
//! output latency + a block (`lead`, from the time base), so the analyzer keeps that much more history and looks back
//! by it – the picture reacts to what is heard now.
pub use deck_core::analysis::{MAX_LEAD, SIZE};

pub struct Analyzer {
    ring: rtrb::Consumer<f32>,
    core: deck_core::analysis::Analyzer,
}

impl Analyzer {
    pub fn new(ring: rtrb::Consumer<f32>, rate: f32, release: f32) -> Analyzer {
        Analyzer { ring, core: deck_core::analysis::Analyzer::new(rate, release) }
    }

    /// level, bass, mid, high – the last update's
    pub fn values(&self) -> [f32; 4] {
        self.core.values
    }

    /// drain the ring, analyse the SIZE samples that end `lead` samples before the newest (the heard moment), smooth
    /// into the values (level, bass, mid, high)
    pub fn update(&mut self, dt: f32, lead: usize) -> [f32; 4] {
        while let Ok(x) = self.ring.pop() {
            self.core.push(x);
        }
        self.core.update(dt, lead)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(f: f32) -> [f32; 4] {
        let (mut tx, rx) = rtrb::RingBuffer::new(SIZE * 2);
        for i in 0..SIZE {
            tx.push(0.5 * (std::f32::consts::TAU * f * i as f32 / 48_000.0).sin()).unwrap();
        }
        let mut a = Analyzer::new(rx, 48_000.0, 0.2);
        for _ in 0..8 {
            a.update(1.0 / 60.0, 0);
        }
        a.values()
    }

    #[test]
    fn a_low_sine_is_bass() {
        let [level, bass, mid, high] = analyse(100.0);
        assert!(level > 0.7, "level {level}");
        assert!(bass > 0.8 && bass > mid + 0.2 && bass > high + 0.3, "{bass} {mid} {high}");
    }

    #[test]
    fn a_high_sine_is_high() {
        let [_, bass, _, high] = analyse(4000.0);
        assert!(high > 0.8 && high > bass + 0.3, "{bass} {high}");
    }

    #[test]
    fn the_window_ends_at_the_heard_moment() {
        // a bass tone, then 2048 samples of silence rendered ahead: with that lead the analyzer still hears the tone
        let (mut tx, rx) = rtrb::RingBuffer::new(SIZE * 4);
        for i in 0..SIZE {
            tx.push(0.5 * (std::f32::consts::TAU * 100.0 * i as f32 / 48_000.0).sin()).unwrap();
        }
        for _ in 0..2048 {
            tx.push(0.0).unwrap();
        }
        let mut a = Analyzer::new(rx, 48_000.0, 0.2);
        let mut heard = 0.0;
        for _ in 0..8 {
            heard = a.update(1.0 / 60.0, 2048)[1];
        }
        assert!(heard > 0.8, "heard: the tone ({heard})");
        let mut b = a.update(1.0 / 60.0, 0)[1];
        for _ in 0..90 {
            b = a.update(1.0 / 60.0, 0)[1];
        }
        assert!(b < 0.05, "the newest samples: silence ({b})");
    }

    #[test]
    fn silence_is_zero() {
        let (_, rx) = rtrb::RingBuffer::new(16);
        let mut a = Analyzer::new(rx, 48_000.0, 0.2);
        assert_eq!(a.update(0.016, 0), [0.0; 4]);
    }
}
