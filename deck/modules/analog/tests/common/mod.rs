//! A small test bench around the module: parameters by name, note lanes, rendering in blocks of any size.
#![allow(dead_code)]
use analog_module::{Analog, PARAM_NAMES, PRESETS_JSON};
use deck_module::testing::Rig;
use deck_module::Describe;

pub const RATE: f64 = 48_000.0;

pub fn idx(name: &str) -> usize {
    PARAM_NAMES.iter().position(|n| *n == name).unwrap_or_else(|| panic!("no parameter '{name}'"))
}

/// the factory preset `name`: (parameter, value) pairs
pub fn preset(name: &str) -> Vec<(String, f32)> {
    let doc: serde_json::Value = serde_json::from_str(PRESETS_JSON).unwrap();
    let p = doc["presets"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("no preset '{name}'"));
    p["values"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap() as f32)).collect()
}

pub struct Bench {
    pub rig: Rig<Analog>,
    pub frame: f64,
    /// per lane: (pitch Hz, gate)
    pub lanes: [(f32, f32); 4],
    pub sustain: f32,
}

impl Bench {
    pub fn new() -> Bench {
        Bench::seeded(1)
    }

    pub fn seeded(seed: u64) -> Bench {
        Bench { rig: Rig::new(RATE, seed), frame: 0.0, lanes: [(261.63, 0.0); 4], sustain: 0.0 }
    }

    pub fn set(&mut self, name: &str, v: f32) -> &mut Bench {
        self.rig.set_param(idx(name), v);
        self
    }

    /// back to the defaults, then the preset's values
    pub fn apply(&mut self, pairs: &[(String, f32)]) {
        let fresh = Analog::build();
        for i in 0..PARAM_NAMES.len() {
            self.rig.set_param(i, fresh.param(i));
        }
        for (k, v) in pairs {
            self.set(k, *v);
        }
    }

    pub fn note_on(&mut self, lane: usize, hz: f32, velocity: f32) {
        self.lanes[lane] = (hz, velocity);
    }

    pub fn note_off(&mut self, lane: usize) {
        self.lanes[lane].1 = 0.0;
    }

    pub fn glide_to(&mut self, lane: usize, hz: f32) {
        self.lanes[lane].0 = hz;
    }

    /// render `frames` frames in blocks of `block`, the lanes and the sustain pedal as they stand
    pub fn run_in(&mut self, frames: usize, block: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        let mut left = frames;
        while left > 0 {
            let n = left.min(block).min(1024);
            for (k, (hz, gate)) in self.lanes.iter().enumerate() {
                self.rig.io.inp.audio[k][0][..n].fill(*hz);
                self.rig.io.inp.audio[k][1][..n].fill(*gate);
            }
            self.rig.io.inp.control[0] = self.sustain;
            self.rig.process(self.frame, n as u32);
            let (a, b) = self.rig.audio_out(0);
            l.extend_from_slice(a);
            r.extend_from_slice(b);
            self.frame += n as f64;
            left -= n;
        }
        (l, r)
    }

    /// like `run_in`, keeping only the peak: allocates nothing
    pub fn run_peak(&mut self, frames: usize, block: usize) -> f32 {
        let mut top = 0.0f32;
        let mut left = frames;
        while left > 0 {
            let n = left.min(block).min(1024);
            for (k, (hz, gate)) in self.lanes.iter().enumerate() {
                self.rig.io.inp.audio[k][0][..n].fill(*hz);
                self.rig.io.inp.audio[k][1][..n].fill(*gate);
            }
            self.rig.io.inp.control[0] = self.sustain;
            self.rig.process(self.frame, n as u32);
            let (a, b) = self.rig.audio_out(0);
            top = a.iter().chain(b).fold(top, |m, v| m.max(v.abs()));
            self.frame += n as f64;
            left -= n;
        }
        top
    }

    pub fn run(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        self.run_in(frames, 256)
    }
}

pub fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

/// the frequency of a periodic signal from its rising zero crossings
pub fn pitch_of(x: &[f32]) -> f64 {
    let up: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).collect();
    assert!(up.len() >= 3, "no periodic signal");
    RATE * (up.len() - 1) as f64 / (up[up.len() - 1] - up[0]) as f64
}

/// the amplitude of the sinusoid at `hz` in `x` (a Hann-windowed DFT at that one frequency)
pub fn mag(x: &[f32], hz: f64) -> f64 {
    let n = x.len();
    let step = core::f64::consts::TAU * hz / RATE;
    let (mut re, mut im, mut wsum) = (0.0f64, 0.0f64, 0.0f64);
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (core::f64::consts::TAU * i as f64 / n as f64).cos();
        re += f64::from(v) * w * (step * i as f64).cos();
        im += f64::from(v) * w * (step * i as f64).sin();
        wsum += w;
    }
    2.0 * (re * re + im * im).sqrt() / wsum
}

/// how bright a signal is: the mean size of its sample-to-sample change against its own size
pub fn brightness(x: &[f32]) -> f64 {
    let d: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
    rms(&d) / rms(x).max(1e-12)
}
