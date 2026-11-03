//! The control inputs as parameters (#188 phase 3): each kind's ports with a range, a curve and the default the stage
//! uses when neither a link nor a param sets them – one table for the renderer's passes, the being's 3D buttons and
//! (later) MIDI / OSC learn and #189's float-vector mapping. Audio kinds' defaults are the engine's (`default_for`).
use crate::graph::Kind;

/// an LFO's shapes by name (its `shape` param), in the order its shape index counts them (deck-native's control.rs,
/// the being's waveform button)
pub const LFO_SHAPES: [&str; 3] = ["sine", "saw", "square"];

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

/// the space node's control inputs that are post knobs (#204, post.rs): no button unless `expose` names them
pub const POST_PORTS: [&str; 7] = ["bloom", "threshold", "lift", "gamma", "gain", "pixel", "levels"];

/// Whether a scene shows the control input `node.port` – the being's 3D button, the deck page's knob (#360): the being's
/// `expose` names the node ("node", all its inputs) or the port ("node.port"); empty = every control input – except the
/// space node's post knobs, shown only when named.
pub fn shown(kind: Kind, node: &str, port: &str, expose: &[String]) -> bool {
    let named = expose.iter().any(|e| e.split_once('.') == Some((node, port)));
    if kind == Kind::Space && POST_PORTS.contains(&port) {
        return named;
    }
    named || expose.is_empty() || expose.iter().any(|e| e == node)
}

/// the default of `kind`'s control input `port` (what an unlinked input without a param reads)
pub fn default(kind: Kind, port: &str) -> f32 {
    spec(kind, port).default
}

/// a control input's value when neither the node nor a link sets it: the catalog's default (deck-catalog, #315; NaN =
/// not driven until a param or a link sets it, an events node's knobs), a module's own for `module:*`, else 0
pub fn default_for(kind: Kind, port: &str) -> f32 {
    match kind {
        // a module's parameters: the defaults its descriptor declares (modreg.rs)
        Kind::Module(m) => m.default_for(port),
        _ => kind.descriptor().and_then(|d| d.param(port)).map_or(0.0, |p| p.default.unwrap_or(f32::NAN)),
    }
}

/// `kind`'s control input `port` as a parameter: its range, curve and default from the catalog; a module's ports and
/// unknown ones read 0..1 linear
pub fn spec(kind: Kind, port: &str) -> Spec {
    match kind.descriptor().and_then(|d| d.param(port)) {
        Some(p) => Spec {
            min: p.min,
            max: p.max,
            default: p.default.unwrap_or(f32::NAN),
            curve: match p.curve {
                deck_catalog::Curve::Lin => Curve::Lin,
                deck_catalog::Curve::Log => Curve::Log,
            },
        },
        None => Spec { min: 0.0, max: 1.0, default: default_for(kind, port), curve: Curve::Lin },
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
