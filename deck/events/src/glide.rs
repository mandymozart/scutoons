//! A pitch glide, the same on every deck (#236): `from → to` over a number of frames, exponentially – equal ratio per
//! frame, so every semitone takes the same time (the ear hears a straight line on a log-frequency axis). Plain
//! numbers, no allocation: the stage's `GateCv` reads it per sample, live-core reads it at the start of each sub-block.
//! `frames == 0` is a jump; after the glide the pitch holds at `to`. A pitch that is not above 0 cannot be a ratio, so
//! it jumps too.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glide {
    from: f32,
    to: f32,
    frames: u32,
}

impl Glide {
    /// Holding `pitch` (no glide running).
    pub fn hold(pitch: f32) -> Glide {
        Glide { from: pitch, to: pitch, frames: 0 }
    }

    /// Starts a glide `from → to` over `frames` frames; offset 0 reads `from`.
    pub fn start(&mut self, from: f32, to: f32, frames: u32) {
        *self = Glide { from, to, frames };
    }

    /// The pitch `offset` frames after the start (past the end: `to`).
    pub fn at(&self, offset: u32) -> f32 {
        if offset >= self.frames || self.from <= 0.0 || self.to <= 0.0 {
            return self.to;
        }
        self.from * (self.to / self.from).powf(offset as f32 / self.frames as f32)
    }

    /// Whether `offset` is still inside the glide.
    pub fn running(&self, offset: u32) -> bool {
        offset < self.frames
    }

    /// Where the glide ends.
    pub fn to(&self) -> f32 {
        self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_midpoint_is_the_geometric_mean() {
        let mut g = Glide::hold(100.0);
        g.start(100.0, 400.0, 1000);
        assert_eq!(g.at(0), 100.0);
        assert!((g.at(500) - 200.0).abs() < 1e-3, "{}", g.at(500));
        assert!(g.running(999) && !g.running(1000));
        // equal ratio per frame
        let (a, b, c) = (g.at(100), g.at(200), g.at(300));
        assert!(((b / a) - (c / b)).abs() < 1e-4);
        // downwards too
        g.start(400.0, 100.0, 10);
        assert!((g.at(5) - 200.0).abs() < 1e-3);
    }

    #[test]
    fn zero_frames_jump_and_the_end_holds() {
        let mut g = Glide::hold(100.0);
        g.start(100.0, 300.0, 0);
        assert_eq!(g.at(0), 300.0);
        assert!(!g.running(0));
        g.start(100.0, 300.0, 8);
        assert_eq!(g.at(8), 300.0);
        assert_eq!(g.at(1_000_000), 300.0);
        assert_eq!(g.to(), 300.0);
    }

    #[test]
    fn a_pitch_at_or_below_zero_jumps() {
        let mut g = Glide::hold(0.0);
        g.start(0.0, 220.0, 100);
        assert_eq!(g.at(10), 220.0);
        g.start(220.0, -1.0, 100);
        assert_eq!(g.at(10), -1.0);
    }
}
