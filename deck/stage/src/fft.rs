//! Sound → control values (the FFT node's frame-thread half): the audio callback pushes the mono sum into a ring
//! (audio/blocks.rs FftTap), the analyzer drains it every frame and turns the newest window into level / bass / mid /
//! high in 0–1 (dB-scaled; a fast rise and a `release` in seconds). The window ends at the heard moment, not at the
//! newest sample: the callback renders ahead of the speaker by the output latency + a block (`lead`, from the time base),
//! so the analyzer keeps that much more history and looks back by it – the picture reacts to what is heard now.
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub const SIZE: usize = 1024;
/// how far back the window may end (≈ 170 ms at 48 kHz – more than any sane output latency + block)
pub const MAX_LEAD: usize = 8192;
const HIST: usize = SIZE + MAX_LEAD;
/// the dB range mapped onto 0–1
const FLOOR_DB: f32 = -70.0;
const RANGE_DB: f32 = 60.0;

pub struct Analyzer {
    ring: rtrb::Consumer<f32>,
    window: Vec<f32>,
    pos: usize,
    hann: Vec<f32>,
    fft: Arc<dyn Fft<f32>>,
    buf: Vec<Complex<f32>>,
    rate: f32,
    release: f32,
    pub values: [f32; 4],
}

impl Analyzer {
    pub fn new(ring: rtrb::Consumer<f32>, rate: f32, release: f32) -> Analyzer {
        let hann = (0..SIZE).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / SIZE as f32).cos()).collect();
        Analyzer {
            ring,
            window: vec![0.0; HIST],
            pos: 0,
            hann,
            fft: FftPlanner::new().plan_fft_forward(SIZE),
            buf: vec![Complex::default(); SIZE],
            rate,
            release: release.max(0.01),
            values: [0.0; 4],
        }
    }

    /// drain the ring, analyse the SIZE samples that end `lead` samples before the newest (the heard moment), smooth
    /// into `values` (level, bass, mid, high)
    pub fn update(&mut self, dt: f32, lead: usize) -> [f32; 4] {
        while let Ok(x) = self.ring.pop() {
            self.window[self.pos] = x;
            self.pos = (self.pos + 1) % HIST;
        }
        let start = self.pos + HIST - SIZE - lead.min(MAX_LEAD);
        for i in 0..SIZE {
            let x = self.window[(start + i) % HIST];
            self.buf[i] = Complex::new(x * self.hann[i], 0.0);
        }
        let raw = bands(&mut self.buf, &*self.fft, self.rate);
        let fall = (-dt / self.release).exp();
        for (v, r) in self.values.iter_mut().zip(raw) {
            *v = if r > *v { *v + (r - *v) * 0.6 } else { (*v * fall).max(r) };
        }
        self.values
    }
}

/// level (RMS of the window) + the mean magnitude of bass 20–250 Hz, mid 250–2000 Hz, high 2–8 kHz, each in 0–1
pub fn bands(buf: &mut [Complex<f32>], fft: &dyn Fft<f32>, rate: f32) -> [f32; 4] {
    let n = buf.len();
    // the Hann window halves the amplitude on average – the RMS is taken before the transform, corrected by 1/0.5
    let rms = (buf.iter().map(|c| c.re * c.re).sum::<f32>() / n as f32).sqrt() * 2.0;
    fft.process(buf);
    let bin_hz = rate / n as f32;
    let band = |lo: f32, hi: f32| {
        let (a, b) = (((lo / bin_hz) as usize).max(1), ((hi / bin_hz) as usize).min(n / 2));
        if b <= a {
            return 0.0;
        }
        // magnitude → amplitude of a sine: |X| * 2 / n, then / 0.5 for the window
        let peak = buf[a..b].iter().map(|c| c.norm()).fold(0.0f32, f32::max);
        to_unit(peak * 4.0 / n as f32)
    };
    [to_unit(rms), band(20.0, 250.0), band(250.0, 2000.0), band(2000.0, 8000.0)]
}

fn to_unit(amp: f32) -> f32 {
    if amp <= 0.0 {
        return 0.0;
    }
    ((20.0 * amp.log10() - FLOOR_DB) / RANGE_DB).clamp(0.0, 1.0)
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
        a.values
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
