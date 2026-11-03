//! The adaptive output lead (#208; one rule since #365): how far the render thread / the browser worker keeps the audio
//! ahead of the device. One policy for every deck, numbers per device class:
//!
//! - It **starts generous** (`start`, the warm-up lead of #356: a cold engine, a phone ramping its clocks up) and,
//!   while nothing has glitched yet, **shrinks a step every `settle_s`** down to the class's **floor** – once stable,
//!   the latency comes back within seconds.
//! - A **glitch grows it a step** (up to `cap`): the next spike of the same size fits.
//! - After a glitch it shrinks a step only after **a calm `calm_s`** – a one-off spike doesn't cost latency for the rest
//!   of the show, a device that glitched is trusted again slowly.
//!
//! The adapter says only what it knows: its device class (a phone, a desktop browser, the native deck) and the glitch
//! count (a running total of underruns: the one cause a lead can absorb, #230).

/// What a device class runs with (frames at the device's rate; seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeadPolicy {
    /// never below this
    pub floor: u32,
    /// the first lead (≥ floor)
    pub start: u32,
    pub cap: u32,
    pub step: u32,
    /// before the first glitch: a step down after this long
    pub settle_s: f64,
    /// after a glitch: a step down after this long without one
    pub calm_s: f64,
}

/// Where a deck runs – the one fact the adapter gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    /// a browser on a phone or tablet (a coarse pointer): a normal-priority worker on a hot, throttling CPU
    Phone = 0,
    /// a browser on a desktop or laptop
    Desktop = 1,
    /// the native deck: a render thread at audio priority, blocks of 256
    Native = 2,
}

impl DeviceClass {
    /// the class of an adapter's code (0 phone, 1 desktop, 2 native; anything else: desktop)
    pub fn from_code(code: u32) -> DeviceClass {
        match code {
            0 => DeviceClass::Phone,
            2 => DeviceClass::Native,
            _ => DeviceClass::Desktop,
        }
    }

    /// the class by its name ("phone", "desktop", "native")
    pub fn from_name(name: &str) -> Option<DeviceClass> {
        Some(match name {
            "phone" => DeviceClass::Phone,
            "desktop" => DeviceClass::Desktop,
            "native" => DeviceClass::Native,
            _ => return None,
        })
    }

    /// The table. Browser: the worker renders 128-frame quanta into an 8192-frame ring – the cap (6144 ≈ 128 ms at
    /// 48 kHz) leaves room in it; the floors are the old fixed starts (3072 on a phone, 2048 on a desktop – the S22
    /// numbers of #356), the start two steps above them. Native: one 256-frame block (5.3 ms) up to 16 (85 ms), and no
    /// warm-up start – native code is compiled ahead and the render thread runs at audio priority.
    pub fn policy(self) -> LeadPolicy {
        match self {
            DeviceClass::Phone => LeadPolicy { floor: 3072, start: 4096, cap: 6144, step: 512, settle_s: 10.0, calm_s: 60.0 },
            DeviceClass::Desktop => LeadPolicy { floor: 2048, start: 3072, cap: 6144, step: 512, settle_s: 10.0, calm_s: 60.0 },
            DeviceClass::Native => LeadPolicy { floor: 256, start: 256, cap: 4096, step: 256, settle_s: 10.0, calm_s: 60.0 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveLead {
    pub policy: LeadPolicy,
    current: u32,
    /// since when it is calm (s), the glitch count last seen, whether one was ever seen
    calm_since: Option<f64>,
    seen: u64,
    glitched: bool,
}

impl AdaptiveLead {
    pub fn new(policy: LeadPolicy) -> AdaptiveLead {
        let floor = policy.floor;
        let p = LeadPolicy { start: policy.start.max(floor), cap: policy.cap.max(floor), step: policy.step.max(1), ..policy };
        AdaptiveLead { current: p.start.min(p.cap), policy: p, calm_since: None, seen: 0, glitched: false }
    }

    pub fn of(class: DeviceClass) -> AdaptiveLead {
        AdaptiveLead::new(class.policy())
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    /// The glitch count `glitches` (a running total) at `now` (s): new glitches since the last call grow the lead one
    /// step; a calm `settle_s` (before the first glitch) or `calm_s` (after one) shrinks it one step. Returns the lead.
    pub fn update(&mut self, now: f64, glitches: u64) -> u32 {
        let p = self.policy;
        if glitches != self.seen {
            self.seen = glitches;
            self.glitched = true;
            self.current = (self.current + p.step).min(p.cap);
            self.calm_since = Some(now);
            return self.current;
        }
        let since = *self.calm_since.get_or_insert(now);
        if now - since >= if self.glitched { p.calm_s } else { p.settle_s } {
            self.current = self.current.saturating_sub(p.step).max(p.floor);
            self.calm_since = Some(now);
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(floor: u32, cap: u32, step: u32) -> AdaptiveLead {
        AdaptiveLead::new(LeadPolicy { floor, start: floor, cap, step, settle_s: 60.0, calm_s: 60.0 })
    }

    #[test]
    fn a_glitch_grows_the_lead_up_to_the_cap() {
        let mut l = fixed(256, 1024, 256);
        assert_eq!(l.update(0.0, 0), 256, "calm: the floor");
        assert_eq!(l.update(1.0, 1), 512, "one glitch: a step");
        assert_eq!(l.update(1.1, 1), 512, "the same glitch counts once");
        assert_eq!(l.update(2.0, 3), 768, "two more at once: still one step per look");
        assert_eq!(l.update(3.0, 4), 1024);
        assert_eq!(l.update(4.0, 5), 1024, "the cap");
    }

    #[test]
    fn a_calm_minute_shrinks_it_a_step_down_to_the_floor() {
        let mut l = fixed(256, 2048, 256);
        l.update(0.0, 0);
        l.update(10.0, 1);
        l.update(11.0, 2);
        assert_eq!(l.current(), 768);
        assert_eq!(l.update(69.0, 2), 768, "59 s calm: not yet");
        assert_eq!(l.update(71.0, 2), 512, "a calm minute after the last glitch: a step down");
        assert_eq!(l.update(100.0, 2), 512, "the next minute starts over");
        assert_eq!(l.update(131.5, 2), 256);
        assert_eq!(l.update(400.0, 2), 256, "never below the floor");
        assert_eq!(l.update(401.0, 3), 512, "and up again on the next glitch");
        assert_eq!(l.update(460.0, 3), 512, "59 s after it");
    }

    /// #356's warm-up lead: a phone starts two steps above its floor and settles there in 20 calm seconds
    #[test]
    fn a_generous_start_shrinks_once_stable() {
        let mut l = AdaptiveLead::of(DeviceClass::Phone);
        assert_eq!(l.update(0.0, 0), 4096);
        assert_eq!(l.update(9.9, 0), 4096);
        assert_eq!(l.update(10.0, 0), 3584, "ten calm seconds: a step");
        assert_eq!(l.update(20.0, 0), 3072, "the floor");
        assert_eq!(l.update(90.0, 0), 3072, "and there it stays");
        assert_eq!(l.update(91.0, 1), 3584, "a glitch: a step up");
        assert_eq!(l.update(101.0, 1), 3584, "after a glitch the settle is not enough");
        assert_eq!(l.update(151.0, 1), 3072, "a calm minute is");
    }

    #[test]
    fn a_glitch_in_the_warm_up_ends_it() {
        let mut l = AdaptiveLead::of(DeviceClass::Desktop);
        assert_eq!(l.update(0.0, 0), 3072);
        assert_eq!(l.update(2.0, 1), 3584, "a cold glitch grows it");
        assert_eq!(l.update(30.0, 1), 3584, "no quick settle after it");
        assert_eq!(l.update(62.0, 1), 3072);
        assert_eq!(AdaptiveLead::of(DeviceClass::Native).current(), 256, "native starts at its floor");
        assert_eq!(DeviceClass::from_code(0), DeviceClass::Phone);
        assert_eq!(DeviceClass::from_code(9), DeviceClass::Desktop);
    }
}
