//! The adaptive lead (#208): how far the render thread keeps the ring ahead of the device. A glitch grows it a step (up
//! to a cap) – the next spike of the same size then fits; a calm minute shrinks it a step (down to the minimum), so a
//! one-off spike doesn't cost latency for the rest of the show. live-core's worker runs the same rule
//! (shared/live-core/src/lead.ts).

/// a lead shrinks a step after this long without a glitch (s)
pub const CALM_S: f64 = 60.0;

#[derive(Debug, Clone)]
pub struct AdaptiveLead {
    pub min: u32,
    pub cap: u32,
    pub step: u32,
    current: u32,
    /// since when it is calm (s), and the glitch count last seen
    calm_since: Option<f64>,
    seen: u64,
}

impl AdaptiveLead {
    /// starts at `min` (frames); grows and shrinks by `step` within `min..=cap`
    pub fn new(min: u32, cap: u32, step: u32) -> AdaptiveLead {
        let cap = cap.max(min);
        AdaptiveLead { min, cap, step: step.max(1), current: min, calm_since: None, seen: 0 }
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    /// The glitch count `glitches` (a running total) at `now` (s): new glitches since the last call grow the lead one
    /// step, a calm minute shrinks it one step (and the next minute starts). Returns the lead.
    pub fn update(&mut self, now: f64, glitches: u64) -> u32 {
        if glitches != self.seen {
            self.seen = glitches;
            self.current = (self.current + self.step).min(self.cap);
            self.calm_since = Some(now);
            return self.current;
        }
        let since = *self.calm_since.get_or_insert(now);
        if now - since >= CALM_S {
            self.current = self.current.saturating_sub(self.step).max(self.min);
            self.calm_since = Some(now);
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glitch_grows_the_lead_up_to_the_cap() {
        let mut l = AdaptiveLead::new(256, 1024, 256);
        assert_eq!(l.update(0.0, 0), 256, "calm: the minimum");
        assert_eq!(l.update(1.0, 1), 512, "one glitch: a step");
        assert_eq!(l.update(1.1, 1), 512, "the same glitch counts once");
        assert_eq!(l.update(2.0, 3), 768, "two more at once: still one step per look");
        assert_eq!(l.update(3.0, 4), 1024);
        assert_eq!(l.update(4.0, 5), 1024, "the cap");
    }

    #[test]
    fn a_calm_minute_shrinks_it_a_step_down_to_the_minimum() {
        let mut l = AdaptiveLead::new(256, 2048, 256);
        l.update(0.0, 0);
        l.update(10.0, 1);
        l.update(11.0, 2);
        assert_eq!(l.current(), 768);
        assert_eq!(l.update(69.0, 2), 768, "59 s calm: not yet");
        assert_eq!(l.update(71.0, 2), 512, "a calm minute after the last glitch: a step down");
        assert_eq!(l.update(100.0, 2), 512, "the next minute starts over");
        assert_eq!(l.update(131.5, 2), 256);
        assert_eq!(l.update(400.0, 2), 256, "never below the minimum");
        assert_eq!(l.update(401.0, 3), 512, "and up again on the next glitch");
        assert_eq!(l.update(460.0, 3), 512, "59 s after it");
    }
}
