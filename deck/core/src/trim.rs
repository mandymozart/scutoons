//! A use's in and out points on its file (decisions/deck.md [delegated]): trimming is non-destructive – the
//! numbers live on the use (a scene's `video` node: `start`, `end`), the file is never rewritten. The use plays only the
//! span between them, looped; a span too short or outside the file falls back to what the file has.

/// the shortest span a use plays (s): a frame at 25 fps
pub const SHORTEST: f64 = 0.04;

/// In and out on the file, s. `end` 0 = the file's end.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Trim {
    pub start: f64,
    pub end: f64,
}

impl Trim {
    /// the whole file
    pub const WHOLE: Trim = Trim { start: 0.0, end: 0.0 };

    /// a use's numbers as stored (negative or not finite = none)
    pub fn new(start: f64, end: f64) -> Trim {
        let clean = |x: f64| if x.is_finite() { x.max(0.0) } else { 0.0 };
        Trim { start: clean(start), end: clean(end) }
    }

    pub fn whole(self) -> bool {
        self.start <= 0.0 && self.end <= 0.0
    }

    /// The span played on a file `length` s long: `start` inside the file, `end` after it by at least `SHORTEST`
    /// (or the file's end); a file shorter than that plays whole.
    pub fn span(self, length: f64) -> (f64, f64) {
        let length = length.max(0.0);
        if length <= SHORTEST {
            return (0.0, length);
        }
        let end = if self.end > 0.0 { self.end.min(length) } else { length };
        let start = self.start.min(length - SHORTEST);
        if end - start < SHORTEST {
            return (start, (start + SHORTEST).min(length));
        }
        (start, end)
    }

    /// how long one lap of the use lasts, s
    pub fn seconds(self, length: f64) -> f64 {
        let (a, b) = self.span(length);
        b - a
    }

    /// The time on the file for `clock` s since the use started: looped inside the span.
    pub fn at(self, clock: f64, length: f64) -> f64 {
        let (a, b) = self.span(length);
        let lap = b - a;
        if lap <= 0.0 {
            return a;
        }
        a + clock.max(0.0).rem_euclid(lap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whole_file_plays_when_nothing_is_trimmed() {
        let t = Trim::WHOLE;
        assert!(t.whole());
        assert_eq!(t.span(10.0), (0.0, 10.0));
        assert_eq!(t.at(3.5, 10.0), 3.5);
        assert_eq!(t.at(12.0, 10.0), 2.0, "looped");
    }

    #[test]
    fn a_trimmed_use_loops_inside_its_span() {
        let t = Trim::new(2.0, 5.0);
        assert_eq!(t.span(10.0), (2.0, 5.0));
        assert_eq!(t.seconds(10.0), 3.0);
        assert_eq!(t.at(0.0, 10.0), 2.0);
        assert_eq!(t.at(2.5, 10.0), 4.5);
        assert_eq!(t.at(3.0, 10.0), 2.0, "back to the in point");
        assert_eq!(t.at(-1.0, 10.0), 2.0, "before the start is the start");
    }

    #[test]
    fn points_outside_the_file_fall_back_to_what_it_has() {
        assert_eq!(Trim::new(2.0, 50.0).span(10.0), (2.0, 10.0), "an end past the file is its end");
        assert_eq!(Trim::new(20.0, 0.0).span(10.0), (10.0 - SHORTEST, 10.0), "a start past the file is its last moment");
        let (a, b) = Trim::new(4.0, 3.0).span(10.0);
        assert_eq!((a, (b * 1e6).round() / 1e6), (4.0, 4.04), "an end before the start: the shortest span");
        assert_eq!(Trim::new(f64::NAN, -3.0), Trim::WHOLE);
        assert_eq!(Trim::new(1.0, 2.0).span(0.02), (0.0, 0.02), "a file shorter than a frame plays whole");
    }
}
