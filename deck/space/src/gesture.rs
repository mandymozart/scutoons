//! The stage's gestures (#376): the fingers of a touch screen, whatever the platform delivers them – winit's `Touch` on
//! the deck, the page's pointer events in the browser. One finger drives the pointer (a press with a bigger reach, a drag,
//! a lift); a second finger lets the first go and the two **pinch** – their distance zooms the camera by its own step.
//! Sans-IO and platform-free: events in (`event`), at most one act out; `Space::pointers` applies it, so a recorded
//! sequence gives the same camera on the deck, in the browser and in the headless host (deck-headless, scenario
//! `x-gesture`).

/// a camera zoom step: the distance is multiplied by this per wheel notch (12 % in); a pinch's steps are counted in it
pub const ZOOM_STEP: f32 = 0.88;

/// a finger's phase (winit's `TouchPhase`, without winit; a cancelled finger is an ended one)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    Started,
    Moved,
    Ended,
}

impl Touch {
    /// the phase as an adapter hands it over a boundary of numbers (the page's wasm calls, a scenario's events):
    /// 0 = down, 1 = moved, anything else = up
    pub fn from_code(code: u32) -> Touch {
        match code {
            0 => Touch::Started,
            1 => Touch::Moved,
            _ => Touch::Ended,
        }
    }
}

/// what the stage does for a finger's event
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Act {
    /// the first finger went down at (x, y): it drives the pointer
    Press(f32, f32),
    /// the driving finger moved
    Drag(f32, f32),
    /// the driving finger lets go at (x, y) – it lifted, or a second finger arrived
    Lift(f32, f32),
    /// the two fingers moved: zoom the camera by this many steps (+ = in, `ZOOM_STEP` each)
    Pinch(f32),
}

/// the fingers on the screen and the gesture they make
#[derive(Debug, Default)]
pub struct Fingers {
    /// every finger down, in the order they came, with its last place
    down: Vec<(u64, (f32, f32))>,
    /// the finger that drives the pointer (the first one, until a second arrives)
    driver: Option<u64>,
    /// the pinch's last distance (device px); `None` = no pinch (two fingers have not been down together)
    pinch: Option<f32>,
}

impl Fingers {
    pub fn new() -> Self {
        Self::default()
    }

    /// the finger that drives the pointer, if one does
    pub fn driver(&self) -> Option<u64> {
        self.driver
    }

    /// any finger is down (the screen's own mouse emulation is then ignored)
    pub fn touching(&self) -> bool {
        !self.down.is_empty()
    }

    fn spread(&self) -> f32 {
        match self.down.as_slice() {
            [(_, a), (_, b)] => (a.0 - b.0).hypot(a.1 - b.1),
            _ => 0.0,
        }
    }

    /// A finger's event → what the stage does. A third finger and a finger left over after a pinch do nothing; a pinch
    /// ends when the last finger lifts (a new one starts it again with two).
    pub fn event(&mut self, id: u64, phase: Touch, x: f32, y: f32) -> Option<Act> {
        match phase {
            Touch::Started => {
                self.down.retain(|f| f.0 != id);
                self.down.push((id, (x, y)));
                match self.down.len() {
                    1 => {
                        self.driver = Some(id);
                        self.pinch = None;
                        Some(Act::Press(x, y))
                    }
                    2 => {
                        self.pinch = Some(self.spread());
                        // the first lets go where it was
                        self.driver.take().and_then(|d| self.down.iter().find(|f| f.0 == d)).map(|f| Act::Lift(f.1 .0, f.1 .1))
                    }
                    _ => None,
                }
            }
            Touch::Moved => {
                self.down.iter_mut().find(|f| f.0 == id)?.1 = (x, y);
                if self.driver == Some(id) {
                    return Some(Act::Drag(x, y));
                }
                let (last, d) = (self.pinch?, self.spread());
                if self.down.len() != 2 || d <= 0.0 {
                    return None;
                }
                self.pinch = Some(d);
                (last > 0.0).then(|| Act::Pinch((d / last).ln() / -ZOOM_STEP.ln()))
            }
            Touch::Ended => {
                let was = self.down.len();
                self.down.retain(|f| f.0 != id);
                if self.down.len() == was {
                    return None;
                }
                if self.down.is_empty() {
                    self.pinch = None;
                } else if self.down.len() == 2 {
                    // back to two fingers: they pinch from where they are
                    self.pinch = Some(self.spread());
                }
                (self.driver == Some(id)).then(|| {
                    self.driver = None;
                    Act::Lift(x, y)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(acts: &[Option<Act>]) -> f32 {
        acts.iter().filter_map(|a| if let Some(Act::Pinch(s)) = a { Some(*s) } else { None }).sum()
    }

    #[test]
    fn one_finger_drives_the_pointer() {
        let mut f = Fingers::new();
        assert_eq!(f.event(1, Touch::Started, 10.0, 20.0), Some(Act::Press(10.0, 20.0)));
        assert_eq!((f.driver(), f.touching()), (Some(1), true));
        assert_eq!(f.event(1, Touch::Moved, 12.0, 22.0), Some(Act::Drag(12.0, 22.0)));
        assert_eq!(f.event(1, Touch::Ended, 13.0, 23.0), Some(Act::Lift(13.0, 23.0)));
        assert_eq!((f.driver(), f.touching()), (None, false));
        assert_eq!(f.event(1, Touch::Moved, 0.0, 0.0), None, "a finger that is not down");
    }

    #[test]
    fn a_second_finger_lets_the_first_go_and_the_two_pinch() {
        let mut f = Fingers::new();
        f.event(1, Touch::Started, 100.0, 300.0);
        assert_eq!(f.event(2, Touch::Started, 200.0, 300.0), Some(Act::Lift(100.0, 300.0)));
        assert_eq!(f.driver(), None);
        // spreading by 100/88 per step: one step in
        let acts = [f.event(1, Touch::Moved, 100.0 - 6.0, 300.0), f.event(2, Touch::Moved, 200.0 + 6.0, 300.0)];
        let want = (112.0f32 / 100.0).ln() / -ZOOM_STEP.ln();
        assert!((steps(&acts) - want).abs() < 1e-4 && want > 0.0, "{acts:?}");
        // pinching in zooms out
        assert!(matches!(f.event(2, Touch::Moved, 200.0, 300.0), Some(Act::Pinch(s)) if s < 0.0));
        // the finger left over after a pinch does nothing, a lift is not a tap
        assert_eq!(f.event(2, Touch::Ended, 200.0, 300.0), None);
        assert_eq!(f.event(1, Touch::Moved, 50.0, 300.0), None);
        assert_eq!(f.event(1, Touch::Ended, 50.0, 300.0), None);
        assert!(!f.touching());
    }

    #[test]
    fn a_third_finger_is_ignored_and_the_pinch_resumes_from_where_two_are() {
        let mut f = Fingers::new();
        f.event(1, Touch::Started, 0.0, 0.0);
        f.event(2, Touch::Started, 100.0, 0.0);
        assert_eq!(f.event(3, Touch::Started, 50.0, 50.0), None);
        assert_eq!(f.event(2, Touch::Moved, 300.0, 0.0), None, "three fingers: no pinch");
        assert_eq!(f.event(3, Touch::Ended, 50.0, 50.0), None);
        // two again, 300 apart: no jump from the old 100
        assert_eq!(f.event(2, Touch::Moved, 300.0, 0.0), Some(Act::Pinch(0.0)));
    }

    #[test]
    fn fingers_on_one_point_do_not_blow_up() {
        let mut f = Fingers::new();
        f.event(1, Touch::Started, 5.0, 5.0);
        f.event(2, Touch::Started, 5.0, 5.0);
        assert_eq!(f.event(2, Touch::Moved, 5.0, 5.0), None);
        assert_eq!(f.event(2, Touch::Moved, 25.0, 5.0), None, "from zero there is no ratio");
        assert!(matches!(f.event(2, Touch::Moved, 45.0, 5.0), Some(Act::Pinch(s)) if s.is_finite() && s > 0.0));
    }
}
