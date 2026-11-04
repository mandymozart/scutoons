//! A small test bench around the module: parameters by name, presets, four note **lanes** (a lane plays one note at a
//! time – note on, note off, a pitch change while it holds – turned into the module's note events, the way a MIDI
//! keyboard's channel would), rendering in blocks of any size (cut into calls of at most `QUANTUM` frames).
#![allow(dead_code)]
use analog_module::{Analog, PARAM_NAMES};
use deck_module::testing::Rig;
use deck_module::{Describe, Ev, Expr, QUANTUM};

pub const RATE: f64 = 48_000.0;

/// the presets that are documents, as a preset file (format `scutoons.preset`)
pub const PRESET_FILE: &str = include_str!("../../analog.preset.json");

pub fn idx(name: &str) -> usize {
    PARAM_NAMES.iter().position(|n| *n == name).unwrap_or_else(|| panic!("no parameter '{name}'"))
}

/// a preset by name: the descriptor's factory preset (`pad`) or one of the preset file's – (parameter, value) pairs
pub fn preset(name: &str) -> Vec<(String, f32)> {
    let doc: serde_json::Value = serde_json::from_str(match name {
        "pad" => Analog::DESCRIPTOR,
        _ => PRESET_FILE,
    })
    .unwrap();
    let values = match name {
        "pad" => &doc["preset"]["values"],
        _ => &doc["presets"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("no preset '{name}'"))["values"],
    };
    values.as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap() as f32)).collect()
}

/// what a lane's state change becomes: the module's note events (at frame 0 of the next call)
#[derive(Default)]
pub struct Lanes {
    /// per lane: (pitch Hz, gate = velocity, 0 = off)
    pub state: [(f32, f32); 4],
    /// per lane: what the module was last told – (the sounding note's id, its pitch at the note on, the pitch now)
    sent: [Option<(u32, f32, f32)>; 4],
    next_id: u32,
}

impl Lanes {
    pub fn new() -> Lanes {
        Lanes { state: [(261.63, 0.0); 4], ..Lanes::default() }
    }

    /// the events that bring the module to the lanes' state: note offs, then bends, then note ons
    pub fn events(&mut self) -> Vec<Ev> {
        let mut out = vec![];
        for (k, &(hz, gate)) in self.state.iter().enumerate() {
            match (self.sent[k], gate > 0.0) {
                (Some((id, _, _)), false) => {
                    out.push(Ev::note_off(0, id));
                    self.sent[k] = None;
                }
                (Some((id, from, now)), true) if now != hz => {
                    out.push(Ev::note_expr(0, id, Expr::Bend, 12.0 * (hz / from).log2()));
                    self.sent[k] = Some((id, from, hz));
                }
                (None, true) => {
                    self.next_id += 1;
                    out.push(Ev::note_on(0, self.next_id, hz, f32::NAN, gate));
                    self.sent[k] = Some((self.next_id, hz, hz));
                }
                _ => {}
            }
        }
        out
    }
}

pub struct Bench {
    pub rig: Rig<Analog>,
    pub frame: f64,
    pub lanes: Lanes,
    pub sustain: f32,
}

impl Bench {
    pub fn new() -> Bench {
        Bench::seeded(1)
    }

    pub fn seeded(seed: u64) -> Bench {
        Bench { rig: Rig::new(RATE, seed), frame: 0.0, lanes: Lanes::new(), sustain: 0.0 }
    }

    pub fn set(&mut self, name: &str, v: f32) -> &mut Bench {
        self.rig.set_param(idx(name), v);
        self
    }

    /// back to the defaults, then the preset's values (what the host's `Preset::apply` does)
    pub fn apply(&mut self, pairs: &[(String, f32)]) {
        for (i, p) in <Analog as Describe>::PARAMS.iter().enumerate() {
            self.rig.set_param(i, p.default);
        }
        for (k, v) in pairs {
            self.set(k, *v);
        }
    }

    pub fn note_on(&mut self, lane: usize, hz: f32, velocity: f32) {
        self.lanes.state[lane] = (hz, velocity);
    }

    pub fn note_off(&mut self, lane: usize) {
        self.lanes.state[lane].1 = 0.0;
    }

    pub fn glide_to(&mut self, lane: usize, hz: f32) {
        self.lanes.state[lane].0 = hz;
    }

    /// one call of `n` ≤ `QUANTUM` frames: the lanes' events at its first frame, the pedal
    fn call(&mut self, n: usize) {
        let events = self.lanes.events();
        if !events.is_empty() {
            assert!(self.rig.set_events_in(0, &events));
        }
        self.rig.set_control_in(0, self.sustain);
        self.rig.run(self.frame, n as u32);
        self.frame += n as f64;
    }

    /// render `frames` frames in blocks of `block` (each cut into calls of at most `QUANTUM`), the lanes and the
    /// sustain pedal as they stand
    pub fn run_in(&mut self, frames: usize, block: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        let mut left = frames;
        while left > 0 {
            let n = left.min(block).min(QUANTUM);
            self.call(n);
            let out = self.rig.audio_out(0);
            let (a, b) = out.stereo();
            l.extend_from_slice(a);
            r.extend_from_slice(b);
            left -= n;
        }
        (l, r)
    }

    /// like `run_in`, keeping only the peak
    pub fn run_peak(&mut self, frames: usize, block: usize) -> f32 {
        let mut top = 0.0f32;
        let mut left = frames;
        while left > 0 {
            let n = left.min(block).min(QUANTUM);
            self.call(n);
            let out = self.rig.audio_out(0);
            let (a, b) = out.stereo();
            top = a.iter().chain(b).fold(top, |m, v| m.max(v.abs()));
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
