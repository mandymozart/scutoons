//! A 2D canvas's zoom and pan (the node views): where a point of the canvas lands in its area, a wheel's zoom around
//! the pointer, a drag's pan, and framing what is drawn. A point on screen is `pan + world × zoom` from the area's top
//! left.
use crate::base::draw::Rect;

/// how far a view zooms out and in
pub const MIN: f32 = 0.25;
pub const MAX: f32 = 4.0;
/// a wheel notch's factor
pub const STEP: f32 = 1.15;
/// framing never zooms in past this: a few nodes do not fill a big screen
pub const FRAME_MAX: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub zoom: f32,
    pub pan: (f32, f32),
}

impl Default for View {
    fn default() -> View {
        View { zoom: 1.0, pan: (0.0, 0.0) }
    }
}

impl View {
    /// where canvas point `p` lands, relative to the area's top left
    pub fn to_screen(&self, p: (f32, f32)) -> (f32, f32) {
        (self.pan.0 + p.0 * self.zoom, self.pan.1 + p.1 * self.zoom)
    }

    /// the canvas point under `s` (relative to the area's top left)
    pub fn to_world(&self, s: (f32, f32)) -> (f32, f32) {
        ((s.0 - self.pan.0) / self.zoom, (s.1 - self.pan.1) / self.zoom)
    }

    /// `steps` wheel notches (up = in) around `at`: the canvas point under it stays under it
    pub fn zoom_at(&mut self, at: (f32, f32), steps: f32) {
        self.zoom_to(at, self.zoom * STEP.powf(steps));
    }

    /// to `zoom` (held within MIN..MAX) around `at`
    pub fn zoom_to(&mut self, at: (f32, f32), zoom: f32) {
        let under = self.to_world(at);
        self.zoom = zoom.clamp(MIN, MAX);
        self.pan = (at.0 - under.0 * self.zoom, at.1 - under.1 * self.zoom);
    }

    pub fn pan_by(&mut self, (dx, dy): (f32, f32)) {
        self.pan = (self.pan.0 + dx, self.pan.1 + dy);
    }

    /// The view that shows `bounds` (canvas) whole and centred in an area of `size`, `margin` free round it; never
    /// closer than `FRAME_MAX`.
    pub fn framing(bounds: Rect, size: (f32, f32), margin: f32) -> View {
        let room = ((size.0 - 2.0 * margin).max(1.0), (size.1 - 2.0 * margin).max(1.0));
        let zoom = (room.0 / bounds.w.max(1.0)).min(room.1 / bounds.h.max(1.0)).clamp(MIN, FRAME_MAX);
        let centre = (bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5);
        View { zoom, pan: (size.0 * 0.5 - centre.0 * zoom, size.1 * 0.5 - centre.1 * zoom) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn screen_and_canvas_are_each_others_inverse() {
        let v = View { zoom: 1.7, pan: (30.0, -12.0) };
        for p in [(0.0, 0.0), (100.0, 50.0), (-40.0, 7.5)] {
            assert!(near(v.to_world(v.to_screen(p)), p));
        }
    }

    #[test]
    fn the_wheel_zooms_round_the_pointer() {
        let mut v = View { zoom: 1.0, pan: (10.0, 20.0) };
        let at = (300.0, 200.0);
        let under = v.to_world(at);
        v.zoom_at(at, 3.0);
        assert!((v.zoom - STEP.powi(3)).abs() < 1e-4);
        assert!(near(v.to_screen(under), at), "the point under the pointer stays under it");
        v.zoom_at(at, -3.0);
        assert!((v.zoom - 1.0).abs() < 1e-4 && near(v.pan, (10.0, 20.0)), "back out: where it was");
    }

    #[test]
    fn the_zoom_is_held_within_its_range() {
        let mut v = View::default();
        v.zoom_at((0.0, 0.0), 100.0);
        assert_eq!(v.zoom, MAX);
        v.zoom_at((0.0, 0.0), -100.0);
        assert_eq!(v.zoom, MIN);
    }

    #[test]
    fn framing_shows_everything_centred() {
        let b = Rect { x: 100.0, y: 50.0, w: 800.0, h: 200.0 };
        let v = View::framing(b, (500.0, 400.0), 20.0);
        let (tl, br) = (v.to_screen((b.x, b.y)), v.to_screen((b.x + b.w, b.y + b.h)));
        assert!(tl.0 >= 19.9 && br.0 <= 480.1 && tl.1 >= 0.0 && br.1 <= 400.0, "{tl:?} {br:?}");
        assert!(((tl.0 + br.0) * 0.5 - 250.0).abs() < 1e-3 && ((tl.1 + br.1) * 0.5 - 200.0).abs() < 1e-3);
        let small = View::framing(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, (1000.0, 1000.0), 0.0);
        assert_eq!(small.zoom, FRAME_MAX, "a little graph is not blown up");
    }

    #[test]
    fn a_drag_pans() {
        let mut v = View::default();
        v.pan_by((15.0, -5.0));
        assert!(near(v.to_screen((0.0, 0.0)), (15.0, -5.0)));
    }
}
