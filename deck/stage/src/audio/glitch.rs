//! The glitch indicator (#208, owner 2026-10-31: "we should have an indicator that warns us about crackles – this is a
//! tool for live performances"). Every device callback is measured: the interval since the last one against the period
//! the last one covered, and the time spent in it against its budget (its own period). A callback is a glitch when
//! - the device ran dry (an **underrun**: the ring was short, or `Engine::mark` saw a gap in the heard stamps),
//! - it took longer than its budget (a **long render**: the next one is late by construction), or
//! - it came later than the last one's period + half of it + 2 ms (**late**: the OS didn't run the audio thread).
//!
//! One counter per cause and the newest glitch live in atomics (`Glitches`, read by the frame thread for the HUD);
//! each glitch's numbers go into a lock-free ring (rtrb, preallocated) that the frame thread drains into the log – the
//! callback never allocates, never locks, never logs.
use crate::clock::now_ns;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// what made a callback a glitch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Late = 1,
    Long = 2,
    Underrun = 3,
}

impl Cause {
    pub fn name(self) -> &'static str {
        match self {
            Cause::Late => "late",
            Cause::Long => "long render",
            Cause::Underrun => "underrun",
        }
    }

    fn of(v: u64) -> Option<Cause> {
        Some(match v {
            1 => Cause::Late,
            2 => Cause::Long,
            3 => Cause::Underrun,
            _ => return None,
        })
    }
}

/// a callback's jitter allowance on top of half the period: WASAPI's and the kernel's wake-up jitter
pub const SLACK_NS: u64 = 2_000_000;
/// no glitch is counted in the first half second (the device and the caches settle; a first callback fills the whole
/// buffer)
pub const SETTLE_NS: u64 = 500_000_000;

/// One callback's numbers (ns).
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    /// since the previous callback started (None: the first)
    pub interval: Option<u64>,
    /// the period the previous callback's frames covered
    pub covered: u64,
    /// the time spent in this callback
    pub render: u64,
    /// this callback's own period (its frames at the rate)
    pub budget: u64,
    /// the device ran dry before or during it
    pub underrun: bool,
}

/// The cause of a glitch, if the callback was one (the most direct cause wins: underrun, long render, late).
pub fn classify(t: &Timing) -> Option<Cause> {
    if t.underrun {
        return Some(Cause::Underrun);
    }
    if t.budget > 0 && t.render > t.budget {
        return Some(Cause::Long);
    }
    match t.interval {
        Some(i) if t.covered > 0 && i > t.covered + t.covered / 2 + SLACK_NS => Some(Cause::Late),
        _ => None,
    }
}

/// What the frame thread reads: the counts, the newest glitch, the load, the device's buffer and latency, the lead.
#[derive(Default)]
pub struct Glitches {
    by_cause: [AtomicU64; 3],
    /// the newest glitch: its moment (now_ns) in µs << 8 | the cause
    last: AtomicU64,
    /// the largest render time / budget (‰) since the frame thread last took it
    load: AtomicU32,
    /// the largest callback seen (frames) – the device's buffer (WASAPI's first callback fills all of it)
    device_frames: AtomicU32,
    /// the device's output latency (µs) as the backend reports it (cpal: playback − callback)
    latency_us: AtomicU32,
    /// the render thread's lead (frames; 0 = rendered in the callback)
    lead: AtomicU32,
}

impl Glitches {
    pub fn count(&self) -> u64 {
        self.by_cause.iter().map(|c| c.load(Ordering::Relaxed)).sum()
    }

    /// (late, long render, underrun)
    pub fn counts(&self) -> [u64; 3] {
        std::array::from_fn(|i| self.by_cause[i].load(Ordering::Relaxed))
    }

    /// the newest glitch: its cause and moment (now_ns)
    pub fn last(&self) -> Option<(Cause, u64)> {
        let v = self.last.load(Ordering::Relaxed);
        Cause::of(v & 0xff).map(|c| (c, (v >> 8) * 1000))
    }

    /// the largest render time / budget since the last call (1 = the whole budget)
    pub fn take_load(&self) -> f32 {
        self.load.swap(0, Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn device_frames(&self) -> u32 {
        self.device_frames.load(Ordering::Relaxed)
    }

    pub fn latency_us(&self) -> u32 {
        self.latency_us.load(Ordering::Relaxed)
    }

    pub fn set_latency_us(&self, us: u32) {
        self.latency_us.store(us, Ordering::Relaxed)
    }

    pub fn lead(&self) -> u32 {
        self.lead.load(Ordering::Relaxed)
    }

    pub fn set_lead(&self, frames: u32) {
        self.lead.store(frames, Ordering::Relaxed)
    }

    /// a render of `render` ns for a budget of `budget` ns (the callback's, or the render thread's block)
    pub fn note_load(&self, render: u64, budget: u64) {
        if let Some(permille) = render.saturating_mul(1000).checked_div(budget) {
            self.load.fetch_max(permille.min(u32::MAX as u64) as u32, Ordering::Relaxed);
        }
    }

    pub fn note_device_frames(&self, frames: u32) {
        self.device_frames.fetch_max(frames, Ordering::Relaxed);
    }

    fn report(&self, cause: Cause, at_ns: u64) {
        self.by_cause[cause as usize - 1].fetch_add(1, Ordering::Relaxed);
        self.last.store((at_ns / 1000) << 8 | cause as u64, Ordering::Relaxed);
    }
}

/// One glitch for the log (the frame thread writes it with a timestamp).
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub at_ns: u64,
    pub cause: Cause,
    pub timing: Timing,
    pub frames: u32,
}

impl Event {
    /// the log line's text
    pub fn describe(&self) -> String {
        let ms = |ns: u64| ns as f64 / 1e6;
        let t = &self.timing;
        format!(
            "audio glitch: {} – {} frames, interval {:.2} ms (the last period {:.2} ms), callback {:.2} ms of {:.2} ms",
            self.cause.name(),
            self.frames,
            ms(t.interval.unwrap_or(0)),
            ms(t.covered),
            ms(t.render),
            ms(t.budget)
        )
    }
}

/// What the device has queued when a callback returns – the deadline of the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// one period: the callback's frames (PipeWire's quantum, ASIO's half buffer, an ALSA period)
    Period,
    /// the whole buffer: each callback tops it up (WASAPI shared – it asks for what the engine took since the last
    /// callback). Its callbacks come in a jittery 10 ms rhythm, sometimes 20 ms and then a double, which is no glitch
    /// while the buffer holds; the device ran dry only when a callback finds the whole buffer free.
    Buffer,
}

/// The callback's half: measures each callback, reports glitches. Owned by the thread that runs the device callback.
pub struct Monitor {
    pub shared: Arc<Glitches>,
    log: rtrb::Producer<Event>,
    rate: f64,
    last_start: Option<u64>,
    last_frames: usize,
    /// the first callback's start + SETTLE_NS
    settle_until: Option<u64>,
    /// false for a timer (the null backend's sleep is no device deadline)
    pub check_late: bool,
    pub deadline: Deadline,
}

/// glitches the log ring holds until the frame thread drains it (more in a burst are counted, not logged)
const LOG: usize = 64;

impl Monitor {
    pub fn new(rate: f32) -> (Monitor, Arc<Glitches>, rtrb::Consumer<Event>) {
        let shared = Arc::new(Glitches::default());
        let (log, rx) = rtrb::RingBuffer::new(LOG);
        let m = Monitor { shared: shared.clone(), log, rate: rate as f64, last_start: None, last_frames: 0, settle_until: None, check_late: true, deadline: Deadline::Period };
        (m, shared, rx)
    }

    fn ns(&self, frames: usize) -> u64 {
        (frames as f64 * 1e9 / self.rate) as u64
    }

    /// now_ns – what a callback calls first
    pub fn begin(&self) -> u64 {
        now_ns()
    }

    /// A callback of `frames` that started at `start` (its `begin`) ends now; `underrun` = the device ran dry. Returns
    /// the glitch's cause, if it was one. Never allocates.
    pub fn end(&mut self, start: u64, frames: usize, underrun: bool) -> Option<Cause> {
        self.measure(start, now_ns(), frames, underrun)
    }

    /// `end` with the end given (the tests' synthetic timings)
    pub fn measure(&mut self, start: u64, end: u64, frames: usize, underrun: bool) -> Option<Cause> {
        let buffer = self.shared.device_frames() as usize;
        let (covered, dry) = match self.deadline {
            Deadline::Period => (self.last_frames, false),
            // the whole buffer free again (after the first callback, which fills it): it ran dry
            Deadline::Buffer => (buffer, buffer > 0 && frames >= buffer),
        };
        let t = Timing {
            interval: self.last_start.map(|s| start.saturating_sub(s)),
            covered: self.ns(covered),
            render: end.saturating_sub(start),
            budget: self.ns(frames),
            underrun: underrun || dry,
        };
        self.last_start = Some(start);
        self.last_frames = frames;
        self.shared.note_device_frames(frames as u32);
        self.shared.note_load(t.render, t.budget);
        let settled = start >= *self.settle_until.get_or_insert(start + SETTLE_NS);
        let cause = classify(&t).filter(|c| settled && (self.check_late || *c != Cause::Late))?;
        self.shared.report(cause, start);
        let _ = self.log.push(Event { at_ns: start, cause, timing: t, frames: frames as u32 });
        Some(cause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    #[test]
    fn the_classifier_names_the_most_direct_cause() {
        let ok = Timing { interval: Some(10 * MS), covered: 10 * MS, render: 3 * MS, budget: 10 * MS, underrun: false };
        assert_eq!(classify(&ok), None);
        assert_eq!(classify(&Timing { interval: None, ..ok }), None, "the first callback");
        assert_eq!(classify(&Timing { interval: Some(16 * MS), ..ok }), None, "a little late: within half a period + 2 ms");
        assert_eq!(classify(&Timing { interval: Some(17_100_000), ..ok }), Some(Cause::Late));
        assert_eq!(classify(&Timing { render: 11 * MS, ..ok }), Some(Cause::Long), "over the budget");
        assert_eq!(classify(&Timing { render: 11 * MS, interval: Some(30 * MS), ..ok }), Some(Cause::Long), "long before late");
        assert_eq!(classify(&Timing { underrun: true, render: 11 * MS, ..ok }), Some(Cause::Underrun), "underrun first");
        // WASAPI: a big first callback (the whole buffer), then the engine's period
        assert_eq!(classify(&Timing { interval: Some(10 * MS), covered: 22 * MS, ..ok }), None);
    }

    /// synthetic callback timings at 48 kHz, 480 frames (10 ms): steady, then one of each kind
    #[test]
    fn the_monitor_counts_and_logs_glitches_after_settling() {
        let (mut m, g, mut log) = Monitor::new(48_000.0);
        let mut at = 1_000 * MS;
        let mut cb = |m: &mut Monitor, gap: u64, render: u64, under: bool| {
            at += gap;
            m.measure(at, at + render, 480, under)
        };
        // a glitch in the first half second is not counted (the device settles)
        assert_eq!(cb(&mut m, 0, 12 * MS, false), None);
        for _ in 0..60 {
            assert_eq!(cb(&mut m, 10 * MS, 2 * MS, false), None);
        }
        assert!((g.take_load() - 1.2).abs() < 0.01, "the load counts from the start: the first callback's 12 ms of 10");
        assert_eq!(cb(&mut m, 25 * MS, 2 * MS, false), Some(Cause::Late));
        assert_eq!(cb(&mut m, 10 * MS, 11 * MS, false), Some(Cause::Long));
        assert_eq!(cb(&mut m, 10 * MS, MS, true), Some(Cause::Underrun));
        assert_eq!(cb(&mut m, 10 * MS, 2 * MS, false), None);
        assert_eq!(g.counts(), [1, 1, 1]);
        assert_eq!(g.count(), 3);
        let (cause, when) = g.last().unwrap();
        assert_eq!(cause, Cause::Underrun);
        assert_eq!(when, 1_645 * MS, "the underrun's moment: 1000 + 60 × 10 + 25 + 10 + 10 ms");
        assert_eq!(g.device_frames(), 480);
        assert!((g.take_load() - 1.1).abs() < 0.01, "the peak load: 11 ms of 10");
        assert_eq!(g.take_load(), 0.0, "taken");
        let causes: Vec<Cause> = std::iter::from_fn(|| log.pop().ok()).map(|e| e.cause).collect();
        assert_eq!(causes, [Cause::Late, Cause::Long, Cause::Underrun]);
    }

    /// WASAPI shared (measured on the RME, 2026-10-31): a 1056-frame buffer, callbacks of 480 every 10 ms – sometimes
    /// 20 ms and then 960. No glitch while the buffer holds; a callback that finds all 1056 free means it ran dry.
    #[test]
    fn a_buffer_deadline_tolerates_wasapi_jitter_and_sees_a_drained_buffer() {
        let (mut m, g, _) = Monitor::new(48_000.0);
        m.deadline = Deadline::Buffer;
        let mut at = 0;
        let mut cb = |m: &mut Monitor, gap: u64, frames: usize| {
            at += gap;
            m.measure(at, at + MS / 2, frames, false)
        };
        assert_eq!(cb(&mut m, 0, 1056), None, "the first fills the buffer");
        for k in 0..100 {
            assert_eq!(cb(&mut m, 10 * MS, 480), None);
            if k % 10 == 5 {
                assert_eq!(cb(&mut m, 20 * MS, 480), None, "a skipped period");
                assert_eq!(cb(&mut m, 10 * MS, 960), None, "and the double");
            }
        }
        assert_eq!(cb(&mut m, 21 * MS, 1056), Some(Cause::Underrun), "the whole buffer free: it ran dry");
        assert_eq!(cb(&mut m, 36 * MS, 480), Some(Cause::Late), "later than the buffer + half of it + 2 ms");
        assert_eq!(g.counts(), [1, 0, 1]);
    }

    #[test]
    fn a_timer_is_never_late() {
        let (mut m, g, _) = Monitor::new(48_000.0);
        m.check_late = false;
        m.measure(0, MS, 480, false);
        m.measure(SETTLE_NS + 50 * MS, SETTLE_NS + 51 * MS, 480, false);
        assert_eq!(g.count(), 0);
    }
}
