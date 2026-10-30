//! The control inputs as parameters (#188 phase 3): each kind's ports with a range, a curve and the default the stage
//! uses when neither a link nor a param sets them – one table for the renderer's passes, the being's 3D buttons and
//! (later) MIDI / OSC learn and #189's float-vector mapping. Audio kinds' defaults are the engine's (`audio::default_for`).
use crate::graph::{Domain, Kind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Lin,
    /// equal ratios per equal travel (frequencies, times, rates); min must be > 0
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub curve: Curve,
}

impl Spec {
    /// value → 0..1 (clamped)
    pub fn norm(&self, v: f32) -> f32 {
        let n = match self.curve {
            Curve::Lin => (v - self.min) / (self.max - self.min),
            Curve::Log => (v.max(self.min).ln() - self.min.ln()) / (self.max.ln() - self.min.ln()),
        };
        if n.is_finite() {
            n.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// 0..1 → value
    pub fn value(&self, n: f32) -> f32 {
        let n = n.clamp(0.0, 1.0);
        match self.curve {
            Curve::Lin => self.min + n * (self.max - self.min),
            Curve::Log => (self.min.ln() + n * (self.max.ln() - self.min.ln())).exp(),
        }
    }
}

const fn lin(min: f32, max: f32, default: f32) -> Spec {
    Spec { min, max, default, curve: Curve::Lin }
}

const fn log(min: f32, max: f32, default: f32) -> Spec {
    Spec { min, max, default, curve: Curve::Log }
}

/// the default of `kind`'s control input `port` (what an unlinked input without a param reads)
pub fn default(kind: Kind, port: &str) -> f32 {
    spec(kind, port).default
}

/// `kind`'s control input `port` as a parameter
pub fn spec(kind: Kind, port: &str) -> Spec {
    use Kind::*;
    let audio = |s: Spec| Spec { default: crate::audio::default_for(kind, port), ..s };
    match (kind, port) {
        (TestPattern, "speed") => lin(0.0, 4.0, 1.0),
        (TestPattern | Chapter, "zoom") => log(0.25, 4.0, 1.0),
        (TestPattern | Space, "hue") => lin(0.0, 1.0, if kind == Space { 0.52 } else { 0.0 }),
        (Shader, "a" | "b" | "c") => lin(0.0, 1.0, 0.5),
        (Chapter, "hold") => log(0.5, 30.0, 6.0),
        (Overlay | Outlet, "opacity") => lin(0.0, 1.0, 1.0),
        (Lfo, "rate") => log(0.01, 20.0, 0.25),
        (Lfo, "depth") => lin(0.0, 2.0, 1.0),
        (Space, "glow") => lin(0.0, 2.0, 0.6),
        (Space, "light") => lin(0.0, 1.5, 0.85),
        (Osc | Fm, "freq") => audio(log(20.0, 4000.0, 0.0)),
        (Fm, "ratio") => audio(log(0.25, 8.0, 0.0)),
        (Fm, "index") => audio(lin(0.0, 10.0, 0.0)),
        (Noise, "cutoff") => audio(log(50.0, 16_000.0, 0.0)),
        (Seq, "rate") => audio(log(0.25, 4.0, 0.0)),
        (Mixer, "gain1" | "gain2" | "gain3" | "gain4") => audio(lin(0.0, 1.5, 0.0)),
        (AudioOut, "volume") => audio(lin(0.0, 1.0, 0.0)),
        (_, "gain") => audio(lin(0.0, 1.0, 0.0)),
        _ if kind.domain() == Domain::Audio => audio(lin(0.0, 1.0, 0.0)),
        // pulses, triggers, shakes, the shader's d …
        _ => lin(0.0, 1.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_and_value_round_trip() {
        for s in [spec(Kind::Fm, "freq"), spec(Kind::Fm, "index"), spec(Kind::Lfo, "rate"), spec(Kind::Space, "hue")] {
            for n in [0.0, 0.25, 0.5, 1.0] {
                assert!((s.norm(s.value(n)) - n).abs() < 1e-4, "{s:?} at {n}");
            }
            assert_eq!(s.norm(s.max * 10.0), 1.0);
        }
        let f = spec(Kind::Osc, "freq");
        assert!((f.value(0.5) - (20.0f32 * 4000.0).sqrt()).abs() < 0.1, "log: the middle is the geometric mean");
    }

    #[test]
    fn defaults_are_the_engines_and_the_passes() {
        assert_eq!(default(Kind::Fm, "index"), 1.0);
        assert_eq!(default(Kind::Noise, "cutoff"), 4000.0);
        assert_eq!(default(Kind::AudioOut, "volume"), 0.8);
        assert_eq!(default(Kind::Shader, "a"), 0.5);
        assert_eq!(default(Kind::Shader, "d"), 0.0);
        assert_eq!(default(Kind::Lfo, "rate"), 0.25);
        assert_eq!(default(Kind::Chapter, "hold"), 6.0);
    }
}
