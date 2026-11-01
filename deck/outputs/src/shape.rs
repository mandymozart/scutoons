//! The mapping math (#211): a parameter's raw value → 0..1 through its range, a curve, invert and the output range;
//! the one-pole smoothing; the wire resolutions (MIDI 7 / 14 bit, DMX 8 / 16 bit). Pure functions, no allocation.

/// How the normalised value bends before it goes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Curve {
    #[default]
    Linear,
    /// x²: slow at the bottom (a dimmer's perceived brightness, a filter's cutoff)
    Exp,
    /// 1 − (1 − x)²: fast at the bottom
    Log,
    /// smoothstep: soft at both ends
    S,
}

impl Curve {
    pub const NAMES: &'static str = "linear, exp, log, s";

    pub fn parse(s: &str) -> Option<Curve> {
        Some(match s {
            "linear" | "lin" => Curve::Linear,
            "exp" => Curve::Exp,
            "log" => Curve::Log,
            "s" | "smooth" => Curve::S,
            _ => return None,
        })
    }

    /// `x` in 0..1 → 0..1 (0 → 0, 1 → 1 for every curve)
    pub fn apply(self, x: f32) -> f32 {
        match self {
            Curve::Linear => x,
            Curve::Exp => x * x,
            Curve::Log => 1.0 - (1.0 - x) * (1.0 - x),
            Curve::S => x * x * (3.0 - 2.0 * x),
        }
    }
}

/// A target's shape: the input range (raw values; default 0..1), the curve, invert, the output range (0..1 of the
/// target's full scale – a DMX channel's 0..255, a CC's 0..127).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub input: (f32, f32),
    pub curve: Curve,
    pub invert: bool,
    pub output: (f32, f32),
}

impl Default for Shape {
    fn default() -> Self {
        Shape { input: (0.0, 1.0), curve: Curve::Linear, invert: false, output: (0.0, 1.0) }
    }
}

impl Shape {
    /// raw → 0..1 of the target's scale. A non-finite raw value is the input's low end (never NaN on a wire).
    pub fn map(&self, raw: f32) -> f32 {
        let (lo, hi) = self.input;
        let x = if hi == lo { 0.0 } else { (raw - lo) / (hi - lo) };
        let x = if x.is_finite() { x.clamp(0.0, 1.0) } else { 0.0 };
        let y = self.curve.apply(x);
        let y = if self.invert { 1.0 - y } else { y };
        let (a, b) = self.output;
        (a + (b - a) * y).clamp(0.0, 1.0)
    }
}

/// One-pole smoothing with time constant `tau` s (0 = none): reaches 63 % of a step after `tau`, never overshoots.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Smooth {
    pub tau: f32,
    y: Option<f32>,
}

impl Smooth {
    pub fn new(tau: f32) -> Smooth {
        Smooth { tau: tau.max(0.0), y: None }
    }

    /// the next value towards `x` after `dt` s (the first call starts at `x`)
    pub fn step(&mut self, x: f32, dt: f32) -> f32 {
        let y = match self.y {
            Some(y) if self.tau > 0.0 => y + (x - y) * (1.0 - (-dt.max(0.0) / self.tau).exp()),
            _ => x,
        };
        self.y = Some(y);
        y
    }

    pub fn value(&self) -> Option<f32> {
        self.y
    }
}

/// 0..1 → a MIDI data byte 0..127
pub fn to_7bit(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 127.0).round() as u8
}

/// 0..1 → 14 bit 0..16383 (pitch bend: 8192 = the middle)
pub fn to_14bit(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 16383.0).round() as u16
}

/// 0..1 → a DMX value 0..255
pub fn to_8bit(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// 0..1 → 16 bit split into (coarse, fine) – the coarse channel first, as DMX charts list them
pub fn split_16bit(v: f32) -> (u8, u8) {
    let n = (f64::from(v.clamp(0.0, 1.0)) * 65535.0).round() as u16;
    ((n >> 8) as u8, (n & 0xFF) as u8)
}

/// Control change, `channel` 0..15
pub fn cc(channel: u8, controller: u8, value: u8) -> [u8; 3] {
    [0xB0 | (channel & 0x0F), controller & 0x7F, value & 0x7F]
}

/// Pitch bend, `channel` 0..15, 14 bit LSB first
pub fn bend(channel: u8, value: u16) -> [u8; 3] {
    [0xE0 | (channel & 0x0F), (value & 0x7F) as u8, ((value >> 7) & 0x7F) as u8]
}

pub fn note_on(channel: u8, note: u8, velocity: u8) -> [u8; 3] {
    [0x90 | (channel & 0x0F), note & 0x7F, velocity.clamp(1, 127)]
}

pub fn note_off(channel: u8, note: u8) -> [u8; 3] {
    [0x80 | (channel & 0x0F), note & 0x7F, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn curves_keep_their_ends_and_bend_between() {
        for c in [Curve::Linear, Curve::Exp, Curve::Log, Curve::S] {
            assert!(close(c.apply(0.0), 0.0) && close(c.apply(1.0), 1.0), "{c:?}");
        }
        assert!(close(Curve::Exp.apply(0.5), 0.25));
        assert!(close(Curve::Log.apply(0.5), 0.75));
        assert!(close(Curve::S.apply(0.5), 0.5) && Curve::S.apply(0.1) < 0.1 && Curve::S.apply(0.9) > 0.9);
        assert_eq!(Curve::parse("exp"), Some(Curve::Exp));
        assert_eq!(Curve::parse("cubic"), None);
    }

    #[test]
    fn shape_normalises_clamps_inverts_and_scales() {
        // a filter cutoff 200..2000 Hz → 0..1, linear
        let s = Shape { input: (200.0, 2000.0), ..Shape::default() };
        assert!(close(s.map(1100.0), 0.5));
        assert!(close(s.map(50.0), 0.0) && close(s.map(9000.0), 1.0), "clamped");
        // inverted
        let s = Shape { invert: true, ..Shape::default() };
        assert!(close(s.map(0.2), 0.8));
        // into a part of the target's scale, with a curve: 0.5 → exp 0.25 → 0.2 + 0.6 × 0.25
        let s = Shape { curve: Curve::Exp, output: (0.2, 0.8), ..Shape::default() };
        assert!(close(s.map(0.5), 0.35));
        // inverted output range (max < min) works too, and invert + curve: the curve first
        let s = Shape { output: (1.0, 0.0), ..Shape::default() };
        assert!(close(s.map(0.25), 0.75));
        let s = Shape { curve: Curve::Exp, invert: true, ..Shape::default() };
        assert!(close(s.map(0.5), 0.75));
        // never NaN
        assert!(close(Shape::default().map(f32::NAN), 0.0));
        assert!(close(Shape { input: (1.0, 1.0), ..Shape::default() }.map(3.0), 0.0));
    }

    #[test]
    fn smoothing_follows_a_step_without_overshoot() {
        let mut s = Smooth::new(0.1);
        assert!(close(s.step(0.0, 0.025), 0.0), "starts where it is");
        let mut y = 0.0;
        for _ in 0..4 {
            y = s.step(1.0, 0.025);
        }
        // after tau (4 × 25 ms): 1 − e⁻¹
        assert!((y - (1.0 - (-1.0f32).exp())).abs() < 1e-4, "{y}");
        for _ in 0..400 {
            y = s.step(1.0, 0.025);
            assert!(y <= 1.0);
        }
        assert!(close(y, 1.0));
        let mut none = Smooth::new(0.0);
        none.step(0.0, 0.025);
        assert!(close(none.step(0.7, 0.025), 0.7), "tau 0 = no smoothing");
    }

    #[test]
    fn resolutions_and_the_16_bit_split() {
        assert_eq!((to_7bit(0.0), to_7bit(0.5), to_7bit(1.0), to_7bit(2.0)), (0, 64, 127, 127));
        assert_eq!((to_14bit(0.0), to_14bit(0.5), to_14bit(1.0)), (0, 8192, 16383));
        assert_eq!((to_8bit(0.0), to_8bit(0.5), to_8bit(1.0)), (0, 128, 255));
        assert_eq!(split_16bit(0.0), (0, 0));
        assert_eq!(split_16bit(1.0), (255, 255));
        // 0.5 → 32768 = 0x8000
        assert_eq!(split_16bit(0.5), (0x80, 0x00));
        // a quarter → 16384 = 0x4000; one step above coarse 0x40 lives in the fine byte
        assert_eq!(split_16bit(0.25), (0x40, 0x00));
        assert_eq!(split_16bit(16385.0 / 65535.0), (0x40, 0x01));
        // the split recombines to the value within half a step
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            let (c, f) = split_16bit(v);
            let n = (u32::from(c) << 8 | u32::from(f)) as f32 / 65535.0;
            assert!((n - v).abs() <= 0.5 / 65535.0 + 1e-6, "{v}");
        }
    }

    #[test]
    fn midi_bytes() {
        assert_eq!(cc(0, 74, 100), [0xB0, 74, 100]);
        assert_eq!(cc(15, 200, 255), [0xBF, 72, 127]);
        assert_eq!(bend(1, 8192), [0xE1, 0x00, 0x40]);
        assert_eq!(bend(0, 16383), [0xE0, 0x7F, 0x7F]);
        assert_eq!(note_on(9, 36, 0), [0x99, 36, 1], "velocity 0 would be a note off");
        assert_eq!(note_off(9, 36), [0x89, 36, 0]);
    }
}
