//! What the `outlet` node's pass reads per frame (#186) – built with or without the `live` module: a composed cell
//! (`Frame`, from the composer thread) and the crossfade between the last two (`Crossfade`, on the frame path: no
//! allocation – tests/no_alloc.rs).
use image::RgbaImage;
use std::path::PathBuf;

/// One composed cell: its flat layers (slices, panels, pictures, text, colour) as one plane – transparent where the
/// backdrop shows through.
pub struct Frame {
    pub plane: RgbaImage,
    /// the cell shows the backdrop: a 3D set, i.e. the scene's space node through the outlet's `in`
    pub backdrop: bool,
    /// how long the new plane fades in and the old one out (ms)
    pub enter_ms: u32,
    pub exit_ms: u32,
}

/// What the scene's space node should show for a cell with a 3D set (#193: set + spot): its 3D file (None = keep
/// the set it has) and the spot the camera and the being go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceTarget {
    /// the set's 3D file (None: the space node keeps its set)
    pub set: Option<PathBuf>,
    pub spot: Option<String>,
}

/// The crossfade between the outlet's previous and current plane, in stage time (s).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Crossfade {
    /// when the current plane arrived
    pub at: f32,
    pub enter: f32,
    pub exit: f32,
    pub current: Option<bool>,
    pub previous: Option<bool>,
}

impl Crossfade {
    /// a new plane arrived at `time` (its backdrop flag, its fades): the current one becomes the previous
    pub fn cut(&mut self, time: f32, backdrop: bool, enter_ms: u32, exit_ms: u32) {
        self.previous = self.current;
        self.current = Some(backdrop);
        self.at = time;
        self.enter = enter_ms as f32 / 1000.0;
        self.exit = exit_ms as f32 / 1000.0;
    }

    /// The pass's four numbers at `time`: how far the current plane is in (0–1), how far the previous one is gone
    /// (0–1), and whether each shows the backdrop (0 / 1). No plane yet: nothing in, everything gone.
    pub fn mix(&self, time: f32) -> [f32; 4] {
        let age = (time - self.at).max(0.0);
        let ramp = |len: f32| if len <= 0.0 { 1.0 } else { (age / len).clamp(0.0, 1.0) };
        let flag = |b: Option<bool>| if b == Some(true) { 1.0 } else { 0.0 };
        let a = if self.current.is_some() { ramp(self.enter) } else { 0.0 };
        let b = if self.previous.is_some() { ramp(self.exit) } else { 1.0 };
        [a, b, flag(self.current), flag(self.previous)]
    }

    /// still fading (the renderer keeps drawing every frame anyway; the HUD says so)
    pub fn fading(&self, time: f32) -> bool {
        let [a, b, ..] = self.mix(time);
        a < 1.0 || b < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_fades_the_new_in_and_the_old_out() {
        let mut x = Crossfade::default();
        assert_eq!(x.mix(0.0), [0.0, 1.0, 0.0, 0.0], "nothing yet");
        x.cut(1.0, false, 500, 0);
        assert_eq!(x.mix(1.25), [0.5, 1.0, 0.0, 0.0], "the first plane fades in, no previous");
        x.cut(2.0, true, 1000, 250);
        let [a, b, cur, prev] = x.mix(2.125);
        assert!((a - 0.125).abs() < 1e-6 && (b - 0.5).abs() < 1e-6);
        assert_eq!((cur, prev), (1.0, 0.0), "the new cell shows the backdrop, the old one didn't");
        assert!(x.fading(2.5));
        assert_eq!(x.mix(3.5), [1.0, 1.0, 1.0, 0.0]);
        assert!(!x.fading(3.5));
        x.cut(4.0, true, 0, 0);
        assert_eq!(x.mix(4.0), [1.0, 1.0, 1.0, 1.0], "0 ms = a cut");
    }
}
