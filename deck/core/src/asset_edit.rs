//! An asset's edits (decisions/deck.md [media]): trim, crop, fade in and out, speed, loudness – on the asset, in the
//! source's time, applied while playing (no file is rewritten, no derivative baked). A use's trim (`Trim`, a scene's
//! `video` node) cuts inside the asset's span, never outside it. `Edits::playback` makes what one use of one derivative
//! plays: the file span (through the derivative's time map offset), the speed, the fades, the gain and the crop.
use crate::trim::{Trim, SHORTEST};

/// the slowest and the fastest speed, file seconds per heard second
pub const SPEED: (f64, f64) = (0.25, 4.0);
/// the quietest and the loudest gain, dB
pub const GAIN_DB: (f64, f64) = (-60.0, 24.0);
/// the most a fade lasts, s
pub const FADE: f64 = 30.0;
/// the most a picture is cropped across (or down): at least this much of it stays
pub const CROP_KEEPS: f32 = 0.05;

/// What is cut off each side of the picture, as parts of its width (left, right) or height (top, bottom).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Crop {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Crop {
    pub const NONE: Crop = Crop { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };

    /// stored parts, cleaned: each 0..1, and at least `CROP_KEEPS` of the picture left both ways (the far side gives way)
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Crop {
        let part = |x: f32| if x.is_finite() { x.clamp(0.0, 1.0 - CROP_KEEPS) } else { 0.0 };
        let (left, top) = (part(left), part(top));
        let right = part(right).min(1.0 - CROP_KEEPS - left);
        let bottom = part(bottom).min(1.0 - CROP_KEEPS - top);
        Crop { left, top, right, bottom }
    }

    pub fn none(self) -> bool {
        self == Crop::NONE
    }

    /// the part kept, as texture coordinates `[u, v, width, height]`
    pub fn rect(self) -> [f32; 4] {
        [self.left, self.top, 1.0 - self.left - self.right, 1.0 - self.top - self.bottom]
    }

    /// width over height of the part kept of a picture `aspect` wide
    pub fn aspect(self, aspect: f32) -> f32 {
        let [_, _, w, h] = self.rect();
        aspect * w / h.max(f32::MIN_POSITIVE)
    }

    /// `fit`, texture coordinates on the cropped picture (deck-core `fit::cover` of its aspect), on the whole texture
    pub fn within(self, fit: [f32; 4]) -> [f32; 4] {
        let [u, v, w, h] = self.rect();
        [u + fit[0] * w, v + fit[1] * h, fit[2] * w, fit[3] * h]
    }
}

/// An asset's edit list. `trim` in source seconds; fades in heard seconds; `speed` file seconds per heard second;
/// `gain_db` the loudness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edits {
    pub trim: Trim,
    pub crop: Crop,
    pub fade_in: f64,
    pub fade_out: f64,
    pub speed: f64,
    pub gain_db: f64,
}

impl Default for Edits {
    fn default() -> Edits {
        Edits::NONE
    }
}

impl Edits {
    /// the asset as it is
    pub const NONE: Edits = Edits { trim: Trim::WHOLE, crop: Crop::NONE, fade_in: 0.0, fade_out: 0.0, speed: 1.0, gain_db: 0.0 };

    /// stored numbers, cleaned into their ranges (not finite: none)
    pub fn new(trim: Trim, crop: Crop, fade_in: f64, fade_out: f64, speed: f64, gain_db: f64) -> Edits {
        let or = |x: f64, d: f64| if x.is_finite() { x } else { d };
        Edits {
            trim,
            crop,
            fade_in: or(fade_in, 0.0).clamp(0.0, FADE),
            fade_out: or(fade_out, 0.0).clamp(0.0, FADE),
            speed: or(speed, 1.0).clamp(SPEED.0, SPEED.1),
            gain_db: or(gain_db, 0.0).clamp(GAIN_DB.0, GAIN_DB.1),
        }
    }

    pub fn none(&self) -> bool {
        *self == Edits::NONE
    }

    /// The asset's span on a source `length` s long, cut by a use's trim `used` (inside the asset's span, at least
    /// `SHORTEST`), source seconds.
    pub fn span(&self, used: Trim, length: f64) -> (f64, f64) {
        let (a, b) = self.trim.span(length);
        if used.whole() {
            return (a, b);
        }
        let (ua, ub) = used.span(length);
        let start = ua.clamp(a, (b - SHORTEST).max(a));
        let end = if used.end > 0.0 { ub.min(b) } else { b };
        (start, end.max(start + SHORTEST).min(b).max(start))
    }

    /// What a use with the trim `used` plays of a derivative whose time map puts its 0 at `offset` s of a source
    /// `length` s long.
    pub fn playback(&self, used: Trim, length: f64, offset: f64) -> Playback {
        let (a, b) = self.span(used, length);
        let lap = (b - a) / self.speed;
        // both fades fit into one lap, in proportion
        let fades = self.fade_in + self.fade_out;
        let k = if fades > lap && fades > 0.0 { lap / fades } else { 1.0 };
        Playback {
            start: (a - offset).max(0.0),
            end: (b - offset).max(0.0),
            speed: self.speed,
            fade_in: self.fade_in * k,
            fade_out: self.fade_out * k,
            gain: libm::pow(10.0, self.gain_db / 20.0) as f32,
            crop: self.crop,
        }
    }
}

/// What one use of one derivative plays, looped: the file span `start`..`end` (s on the derivative), its speed, the
/// fades at each lap's start and end (heard s), the gain (linear) and the crop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playback {
    pub start: f64,
    pub end: f64,
    pub speed: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    pub gain: f32,
    pub crop: Crop,
}

impl Playback {
    /// a file `length` s long, whole and as it is
    pub fn whole(length: f64) -> Playback {
        Edits::NONE.playback(Trim::WHOLE, length, 0.0)
    }

    /// heard seconds per lap
    pub fn lap(&self) -> f64 {
        ((self.end - self.start) / self.speed).max(0.0)
    }

    /// heard `clock` s since the use started → seconds into the current lap
    pub fn into_lap(&self, clock: f64) -> f64 {
        let lap = self.lap();
        if lap <= 0.0 {
            return 0.0;
        }
        clock.max(0.0).rem_euclid(lap)
    }

    /// the file's time at heard `clock`
    pub fn file_at(&self, clock: f64) -> f64 {
        self.start + self.into_lap(clock) * self.speed
    }

    /// the heard clock (in the first lap) at the file's time `file`
    pub fn clock_at(&self, file: f64) -> f64 {
        ((file.clamp(self.start, self.end) - self.start) / self.speed).max(0.0)
    }

    /// The fades' level at `t` s into a lap, 0..1: up over `fade_in` at its start, down over `fade_out` to its end.
    pub fn fade(&self, t: f64) -> f32 {
        let lap = self.lap();
        let mut level = 1.0f64;
        if self.fade_in > 0.0 {
            level = level.min(t / self.fade_in);
        }
        if self.fade_out > 0.0 {
            level = level.min((lap - t) / self.fade_out);
        }
        level.clamp(0.0, 1.0) as f32
    }

    /// the fades' level at heard `clock`
    pub fn level(&self, clock: f64) -> f32 {
        self.fade(self.into_lap(clock))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn no_edits_play_the_file_whole() {
        let p = Playback::whole(10.0);
        assert_eq!((p.start, p.end, p.speed, p.gain, p.crop), (0.0, 10.0, 1.0, 1.0, Crop::NONE));
        assert_eq!((p.lap(), p.file_at(12.5), p.level(3.0)), (10.0, 2.5, 1.0));
        assert!(Edits::default().none());
    }

    #[test]
    fn stored_numbers_are_cleaned_into_their_ranges() {
        let e = Edits::new(Trim::WHOLE, Crop::new(0.7, f32::NAN, 0.7, -1.0), -1.0, 99.0, 0.0, 80.0);
        assert_eq!((e.fade_in, e.fade_out, e.speed, e.gain_db), (0.0, FADE, SPEED.0, GAIN_DB.1));
        assert_eq!(e.crop, Crop::new(0.7, 0.0, 0.25, 0.0), "the far side gives way: 5 % of the width stays");
        assert!((e.crop.rect()[2] - CROP_KEEPS).abs() < 1e-6);
        assert_eq!(Edits::new(Trim::WHOLE, Crop::NONE, f64::NAN, 0.0, f64::INFINITY, f64::NAN), Edits::NONE);
    }

    #[test]
    fn a_uses_trim_cuts_inside_the_assets_span_never_outside() {
        let e = Edits { trim: Trim::new(2.0, 8.0), ..Edits::NONE };
        assert_eq!(e.span(Trim::WHOLE, 10.0), (2.0, 8.0), "no use trim: the asset's span");
        assert_eq!(e.span(Trim::new(3.0, 5.0), 10.0), (3.0, 5.0), "inside: the use's");
        assert_eq!(e.span(Trim::new(1.0, 9.0), 10.0), (2.0, 8.0), "wider: the asset's");
        assert_eq!(e.span(Trim::new(4.0, 0.0), 10.0), (4.0, 8.0), "to the use's end: the asset's out point");
        let (a, b) = e.span(Trim::new(9.0, 9.5), 10.0);
        assert!(near(a, 8.0 - SHORTEST) && near(b, 8.0), "past the asset's out point: its last moment");
    }

    #[test]
    fn a_derivative_plays_the_span_through_its_offset_at_the_speed() {
        let e = Edits { trim: Trim::new(2.0, 8.0), speed: 2.0, ..Edits::NONE };
        let p = e.playback(Trim::WHOLE, 10.0, 0.5);
        assert_eq!((p.start, p.end), (1.5, 7.5), "a derivative starting half a second into the source");
        assert!(near(p.lap(), 3.0), "six seconds at twice the speed");
        assert!(near(p.file_at(1.0), 3.5) && near(p.file_at(3.0), 1.5), "looped");
        assert!(near(p.clock_at(3.5), 1.0));
    }

    #[test]
    fn fades_rise_and_fall_in_each_lap_and_fit_a_short_one() {
        let e = Edits { fade_in: 1.0, fade_out: 2.0, ..Edits::NONE };
        let p = e.playback(Trim::WHOLE, 10.0, 0.0);
        assert_eq!([0.0, 0.5, 1.0, 5.0, 9.0, 10.0].map(|t| p.fade(t)), [0.0, 0.5, 1.0, 1.0, 0.5, 0.0]);
        assert_eq!(p.level(10.5), 0.5, "the next lap fades in again");
        let short = Edits { trim: Trim::new(0.0, 1.5), ..e }.playback(Trim::WHOLE, 10.0, 0.0);
        assert!(near(short.fade_in, 0.5) && near(short.fade_out, 1.0), "three seconds of fades into a lap of 1.5: in proportion");
    }

    #[test]
    fn loudness_is_a_gain_in_db() {
        let gain = |db: f64| Edits { gain_db: db, ..Edits::NONE }.playback(Trim::WHOLE, 1.0, 0.0).gain;
        assert_eq!(gain(0.0), 1.0);
        assert!((gain(-6.0) - 0.501).abs() < 1e-3 && (gain(20.0) - 10.0).abs() < 1e-4);
    }

    #[test]
    fn a_crop_keeps_its_part_and_a_fit_lands_inside_it() {
        let c = Crop::new(0.25, 0.0, 0.25, 0.5);
        assert_eq!(c.rect(), [0.25, 0.0, 0.5, 0.5]);
        assert_eq!(c.aspect(16.0 / 9.0), 16.0 / 9.0, "half of each way: the same shape");
        assert_eq!(c.within([0.0, 0.0, 1.0, 1.0]), [0.25, 0.0, 0.5, 0.5]);
        assert_eq!(c.within([0.1, 0.0, 0.8, 1.0]), [0.3, 0.0, 0.4, 0.5]);
    }
}
