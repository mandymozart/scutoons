//! Module `analysis`: sound → control values – the stage's FFT (#185: deck/stage/src/fft.rs only drains its audio ring
//! into an `Analyzer`): level (RMS) + the peak magnitude of bass 20–250 Hz, mid 250–2000 Hz, high 2–8 kHz, each in 0–1
//! (dB-scaled). `Analyzer` keeps a history and looks back by the output lead, so the values are what is heard now (a
//! fast rise, a `release` in seconds); `FftBlock` is the patcher's `fft` kind (one window, no lead, no smoothing).
use crate::module::{Block, KindInfo, Module, Params, ANALYSIS_KINDS};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub const SIZE: usize = 1024;
/// the bins `Bands::spectrum` fills: 0 Hz … rate / 2 in `SIZE / 2` steps (46.9 Hz at 48 kHz)
pub const SPECTRUM: usize = SIZE / 2;
/// how far back the window may end (≈ 170 ms at 48 kHz – more than any sane output latency + block)
pub const MAX_LEAD: usize = 8192;
const HIST: usize = SIZE + MAX_LEAD;
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
        self.analyse_by(|i| window.get(i).copied().unwrap_or(0.0))
    }

    /// The last analysed window's magnitudes, bin k = k · rate / `SIZE` Hz, each in 0–1 (dB-scaled like the bands):
    /// fills `out[..min(out.len(), SPECTRUM)]` – a visualiser's frame or a vector to send (#184, #189).
    pub fn spectrum(&self, out: &mut [f32]) {
        let n = self.buf.len();
        for (o, c) in out.iter_mut().zip(&self.buf[..n / 2]) {
            *o = to_unit(c.norm() * 4.0 / n as f32);
        }
    }

    /// The four values of the `SIZE` samples `sample(0..SIZE)` (in time order).
    fn analyse_by(&mut self, sample: impl Fn(usize) -> f32) -> [f32; 4] {
        for (i, b) in self.buf.iter_mut().enumerate() {
            *b = Complex::new(sample(i) * self.hann[i], 0.0);
        }
        bands(&mut self.buf, &*self.fft, self.rate)
    }
}

/// The stage's FFT node on the frame thread: samples are pushed as they come (the audio thread renders ahead of the
/// speaker), each `update` analyses the `SIZE` samples that end `lead` samples before the newest – the heard moment –
/// and smooths them into `values` (level, bass, mid, high). Allocates only in `new`.
pub struct Analyzer {
    bands: Bands,
    window: Vec<f32>,
    pos: usize,
    release: f32,
    pub values: [f32; 4],
}

impl Analyzer {
    pub fn new(rate: f32, release: f32) -> Analyzer {
        Analyzer { bands: Bands::new(rate), window: vec![0.0; HIST], pos: 0, release: release.max(0.01), values: [0.0; 4] }
    }

    /// one sample, the newest
    pub fn push(&mut self, x: f32) {
        self.window[self.pos] = x;
        self.pos = (self.pos + 1) % HIST;
    }

    /// analyse the window ending `lead` samples before the newest (at most `MAX_LEAD`), smooth into `values`
    pub fn update(&mut self, dt: f32, lead: usize) -> [f32; 4] {
        let start = self.pos + HIST - SIZE - lead.min(MAX_LEAD);
        let window = &self.window;
        let raw = self.bands.analyse_by(|i| window[(start + i) % HIST]);
        let fall = (-dt / self.release).exp();
        for (v, r) in self.values.iter_mut().zip(raw) {
            *v = if r > *v { *v + (r - *v) * 0.6 } else { (*v * fall).max(r) };
        }
        self.values
    }

    /// the spectrum of the last `update`'s window, unsmoothed (`Bands::spectrum`)
    pub fn spectrum(&self, out: &mut [f32]) {
        self.bands.spectrum(out);
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

    /// the stage's frame-thread analyzer, fed `SIZE` samples of a sine, after 8 frames
    fn follow(f: f32) -> [f32; 4] {
        let mut a = Analyzer::new(48_000.0, 0.2);
        sine(f).into_iter().for_each(|x| a.push(x));
        for _ in 0..8 {
            a.update(1.0 / 60.0, 0);
        }
        a.values
    }

    #[test]
    fn the_analyzer_hears_bass_and_high() {
        let [level, bass, mid, high] = follow(100.0);
        assert!(level > 0.7, "level {level}");
        assert!(bass > 0.8 && bass > mid + 0.2 && bass > high + 0.3, "{bass} {mid} {high}");
        let [_, bass, _, high] = follow(4000.0);
        assert!(high > 0.8 && high > bass + 0.3, "{bass} {high}");
    }

    #[test]
    fn the_window_ends_at_the_heard_moment() {
        // a bass tone, then 2048 samples of silence rendered ahead: with that lead the analyzer still hears the tone
        let mut a = Analyzer::new(48_000.0, 0.2);
        sine(100.0).into_iter().chain(std::iter::repeat(0.0).take(2048)).for_each(|x| a.push(x));
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
    fn the_spectrum_peaks_at_the_tone() {
        let mut b = Bands::new(48_000.0);
        b.analyse(&sine(1500.0));
        let mut s = [0.0; SPECTRUM];
        b.spectrum(&mut s);
        let peak = (0..SPECTRUM).max_by(|&i, &j| s[i].total_cmp(&s[j])).unwrap();
        assert_eq!(peak, (1500.0 / (48_000.0 / SIZE as f32)).round() as usize, "the bin of 1500 Hz");
        assert!(s[peak] > 0.8 && s[400] < 0.3, "{} {}", s[peak], s[400]);
    }

    #[test]
    fn silence_is_zero() {
        assert_eq!(Analyzer::new(48_000.0, 0.2).update(0.016, 0), [0.0; 4]);
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
