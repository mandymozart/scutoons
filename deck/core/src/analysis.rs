//! Module `analysis`: sound → control values. `bands` is the stage's FFT analysis (deck/stage/src/fft.rs uses it):
//! level (RMS) + the peak magnitude of bass 20–250 Hz, mid 250–2000 Hz, high 2–8 kHz, each in 0–1 (dB-scaled).
use crate::module::{Block, KindInfo, Module, Params, ANALYSIS_KINDS};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub const SIZE: usize = 1024;
/// the dB range mapped onto 0–1
const FLOOR_DB: f32 = -70.0;
const RANGE_DB: f32 = 60.0;

pub struct AnalysisModule;

impl Module for AnalysisModule {
    fn name(&self) -> &'static str {
        "analysis"
    }
    fn kinds(&self) -> &'static [KindInfo] {
        ANALYSIS_KINDS
    }
    fn build(&self, kind: &str, _params: &Params, rate: f32) -> Option<Box<dyn Block>> {
        (kind == "fft").then(|| Box::new(FftBlock { bands: Bands::new(rate), window: vec![0.0; SIZE], values: [0.0; 4] }) as Box<dyn Block>)
    }
}

/// The analysis with its plan and buffers (one allocation, at build).
pub struct Bands {
    fft: Arc<dyn Fft<f32>>,
    hann: Vec<f32>,
    buf: Vec<Complex<f32>>,
    rate: f32,
}

impl Bands {
    pub fn new(rate: f32) -> Self {
        let hann = (0..SIZE).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / SIZE as f32).cos()).collect();
        Bands { fft: FftPlanner::new().plan_fft_forward(SIZE), hann, buf: vec![Complex::default(); SIZE], rate }
    }

    /// The four values of the newest `SIZE` samples (`window` is in time order).
    pub fn analyse(&mut self, window: &[f32]) -> [f32; 4] {
        for (i, b) in self.buf.iter_mut().enumerate() {
            *b = Complex::new(window.get(i).copied().unwrap_or(0.0) * self.hann[i], 0.0);
        }
        bands(&mut self.buf, &*self.fft, self.rate)
    }
}

/// level (RMS of the window) + the peak magnitude of bass, mid, high, each in 0–1; `buf` holds the windowed samples
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

/// The `fft` kind as a block: `audio` takes the input block (the window slides), `control` puts out the four values.
pub struct FftBlock {
    bands: Bands,
    window: Vec<f32>,
    values: [f32; 4],
}

impl Block for FftBlock {
    fn audio(&mut self, _i: &[f32], input: &mut [f32]) {
        let n = input.len().min(SIZE);
        self.window.drain(..n);
        self.window.extend_from_slice(&input[input.len() - n..]);
    }
    fn control(&mut self, _i: &[f32], o: &mut [f32], _dt: f32) {
        self.values = self.bands.analyse(&self.window);
        o[..4].copy_from_slice(&self.values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(f: f32) -> Vec<f32> {
        (0..SIZE).map(|i| 0.5 * (std::f32::consts::TAU * f * i as f32 / 48_000.0).sin()).collect()
    }

    #[test]
    fn a_low_sine_is_bass() {
        let [level, bass, mid, high] = Bands::new(48_000.0).analyse(&sine(100.0));
        assert!(level > 0.7, "level {level}");
        assert!(bass > 0.8 && bass > mid + 0.2 && bass > high + 0.3, "{bass} {mid} {high}");
    }

    #[test]
    fn a_high_sine_is_high() {
        let [_, bass, _, high] = Bands::new(48_000.0).analyse(&sine(4000.0));
        assert!(high > 0.8 && high > bass + 0.3, "{bass} {high}");
    }

    #[test]
    fn the_block_slides_its_window() {
        let mut b = AnalysisModule.build("fft", &Params::new(), 48_000.0).unwrap();
        let mut s = sine(100.0);
        for chunk in s.chunks_mut(128) {
            b.audio(&[], chunk);
        }
        let mut o = [0.0; 4];
        b.control(&[], &mut o, 0.016);
        assert!(o[1] > 0.8);
    }
}
