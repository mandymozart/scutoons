//! #223: how good one client's link is, and the rate its state-like messages go out at. Strain steps the rate down one
//! level (30 → 15 → 10 → 5 / s, at most one step per `STEP_GAP`); `CALM` without strain steps it up one level – the same
//! rule as the adaptive audio lead (deck/native/src/audio/lead.rs, #208): take the margin at once, give it back slowly.
//!
//! Strain is a link that cannot carry what it gets, not a one-off stall (a lost segment on Wi-Fi stalls TCP ~25 ms, a
//! lost retransmit 200 ms – the outbox's coalescing absorbs that, and a paced rate would only add delay to every value):
//! - the smoothed ping round trip stays `STRAIN_RTT` over the best one seen (a queue built up somewhere on the way), or
//! - the writer stays behind for `STRAIN_BEHIND` without a break (each batch: the socket took > `SLOW_FLUSH` to accept
//!   it, or a message waited > `SLOW_WAIT` in the outbox beyond its pacing).
//!
//! Level 0 is unpaced (as fast as the decks send, ≤ 30 / s – session.rs `VALUES_RATE`), so a good link pays no delay.
//! Pure and clock-free (the caller passes `now`), like session.rs.

use std::time::{Duration, Instant};

/// the value rate per level (/ s); level 0 = unpaced
pub const HZ: [u32; 4] = [30, 15, 10, 5];
/// a step back up after this long without strain
pub const CALM: Duration = Duration::from_secs(5);
/// at most one step down per this long
pub const STEP_GAP: Duration = Duration::from_millis(500);
/// the smoothed round trip this much over the best one is strain (a LAN's best is ~1 ms, a phone on 4G ~40 ms)
pub const STRAIN_RTT: Duration = Duration::from_millis(80);
/// behind this long without a break is strain
pub const STRAIN_BEHIND: Duration = Duration::from_millis(500);
/// a batch the socket took this long to accept: its buffer was full
pub const SLOW_FLUSH: Duration = Duration::from_millis(30);
/// a message waited this long in the outbox (beyond its pacing) before the writer got to it
pub const SLOW_WAIT: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Default)]
pub struct Link {
    level: usize,
    calm_since: Option<Instant>,
    last_step: Option<Instant>,
    /// since when every batch was behind
    behind_since: Option<Instant>,
    /// the smoothed round trip and the best one (ms)
    rtt_ms: Option<f64>,
    best_ms: Option<f64>,
}

impl Link {
    pub fn level(&self) -> usize {
        self.level
    }

    /// the rate state-like messages go out at (/ s)
    pub fn hz(&self) -> u32 {
        HZ[self.level]
    }

    /// the pause between two batches of state-like messages; none at level 0
    pub fn interval(&self) -> Option<Duration> {
        (self.level > 0).then(|| Duration::from_secs_f64(1.0 / HZ[self.level] as f64))
    }

    /// the smoothed ping round trip (ms), once one came back
    pub fn rtt_ms(&self) -> Option<f64> {
        self.rtt_ms
    }

    /// One look at `now`: strained steps the rate down (not twice within `STEP_GAP`), `CALM` without strain steps it
    /// up. Returns whether the level changed.
    pub fn look(&mut self, now: Instant, strained: bool) -> bool {
        let before = self.level;
        if strained {
            self.calm_since = Some(now);
            let may = self.last_step.map_or(true, |t| now.saturating_duration_since(t) >= STEP_GAP);
            if may && self.level + 1 < HZ.len() {
                self.level += 1;
                self.last_step = Some(now);
                // the slower rate gets its own chance to catch up
                self.behind_since = None;
            }
        } else {
            let since = *self.calm_since.get_or_insert(now);
            if self.level > 0 && now.saturating_duration_since(since) >= CALM {
                self.level -= 1;
                self.calm_since = Some(now);
            }
        }
        self.level != before
    }

    /// A ping came back after `rtt`. Returns whether the level changed.
    pub fn rtt(&mut self, now: Instant, rtt: Duration) -> bool {
        let ms = rtt.as_secs_f64() * 1000.0;
        let best = self.best_ms.map_or(ms, |b| b.min(ms));
        self.best_ms = Some(best);
        let smooth = self.rtt_ms.map_or(ms, |r| r * 0.75 + ms * 0.25);
        self.rtt_ms = Some(smooth);
        self.look(now, smooth > best + STRAIN_RTT.as_secs_f64() * 1000.0)
    }

    /// The writer sent a batch: the socket took `flush` to accept it, its oldest message had waited `waited` (beyond
    /// its pacing). Returns whether the level changed.
    pub fn batch(&mut self, now: Instant, flush: Duration, waited: Duration) -> bool {
        if flush > SLOW_FLUSH || waited > SLOW_WAIT {
            let since = *self.behind_since.get_or_insert(now);
            self.look(now, now.saturating_duration_since(since) >= STRAIN_BEHIND)
        } else {
            self.behind_since = None;
            self.look(now, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    const SLOW: Duration = Duration::from_millis(100);
    const FAST: Duration = Duration::from_millis(1);

    #[test]
    fn strain_steps_down_one_level_per_gap() {
        let t0 = Instant::now();
        let mut l = Link::default();
        assert_eq!((l.hz(), l.interval()), (30, None), "a new client: as fast as the decks send");
        assert!(l.look(at(t0, 0), true));
        assert_eq!(l.hz(), 15);
        assert!(!l.look(at(t0, 100), true), "not twice within the gap");
        assert!(l.look(at(t0, 600), true));
        assert_eq!(l.hz(), 10);
        assert!(l.look(at(t0, 1200), true));
        assert!(!l.look(at(t0, 1800), true), "5 / s is the floor");
        assert_eq!(l.hz(), 5);
        assert_eq!(l.interval(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn calm_steps_up_one_level_at_a_time() {
        let t0 = Instant::now();
        let mut l = Link::default();
        l.look(at(t0, 0), true);
        l.look(at(t0, 600), true);
        assert_eq!(l.hz(), 10);
        assert!(!l.look(at(t0, 5_500), false), "4.9 s calm: not yet");
        assert!(l.look(at(t0, 5_600), false), "5 s after the last strain: a step up");
        assert_eq!(l.hz(), 15);
        assert!(!l.look(at(t0, 9_000), false), "the next 5 s start over");
        assert!(l.look(at(t0, 10_600), false));
        assert_eq!((l.hz(), l.interval()), (30, None));
        assert!(!l.look(at(t0, 60_000), false), "never above level 0");
        // strain while calming: the calm starts again
        l.look(at(t0, 61_000), true);
        l.look(at(t0, 65_000), true);
        assert_eq!(l.hz(), 10);
        assert!(!l.look(at(t0, 69_000), false), "4 s after the last strain");
    }

    /// a one-off stall (a lost retransmit: 200 ms behind, then caught up) costs nothing; a link that stays behind steps
    /// down
    #[test]
    fn only_a_link_that_stays_behind_is_strained() {
        let t0 = Instant::now();
        let mut l = Link::default();
        assert!(!l.batch(at(t0, 0), SLOW, Duration::ZERO));
        assert!(!l.batch(at(t0, 200), Duration::ZERO, Duration::from_millis(200)), "the burst after the stall");
        assert!(!l.batch(at(t0, 210), FAST, FAST), "caught up");
        assert!(!l.batch(at(t0, 300), SLOW, Duration::ZERO), "behind again: the 500 ms start over");
        assert!(!l.batch(at(t0, 700), SLOW, Duration::ZERO));
        assert!(l.batch(at(t0, 800), SLOW, Duration::ZERO), "500 ms behind without a break");
        assert_eq!(l.hz(), 15);
        assert!(!l.batch(at(t0, 1000), SLOW, Duration::ZERO), "the slower rate gets 500 ms of its own");
        assert!(!l.batch(at(t0, 1300), SLOW, Duration::ZERO));
        assert!(l.batch(at(t0, 1500), SLOW, Duration::ZERO));
        assert_eq!(l.hz(), 10);
    }

    #[test]
    fn a_round_trip_that_stays_long_is_strain() {
        let t0 = Instant::now();
        let mut l = Link::default();
        assert!(!l.rtt(at(t0, 0), Duration::from_millis(2)));
        assert!(!l.rtt(at(t0, 1000), Duration::from_millis(250)), "one late ping: smoothed 64 ms – a stall, not strain");
        assert!(!l.rtt(at(t0, 2000), Duration::from_millis(2)));
        assert!(l.rtt(at(t0, 3000), Duration::from_millis(200)), "late again: smoothed 86 ms – a queue on the way");
        assert_eq!(l.hz(), 15);
        let r = l.rtt_ms().unwrap();
        assert!(r > 82.0 && r < 200.0, "smoothed: {r}");
    }
}
