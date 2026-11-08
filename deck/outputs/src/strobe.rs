//! The clock-synced strobe (#211, owner 2026-10-31: "strobe rates are synced to the clock … the neutral position is a
//! musical division (e.g. 1/4 = once per beat), and the parameter deviates from it: stepping through divisions (1/1,
//! 1/2, 1/4, 1/8, 1/16, triplets) or a free offset … flashes land on the clock's grid, sent ahead by the lead"; "no
//! limit … an optional per-fixture max rate stays available as a setting (off)").
//!
//! The clock is a line in seconds on the host's monotonic clock (`Clock`: the beat at `t`, beats per second – the
//! stage's tempo map through its time base, MIDI clock out's way). The grid restarts at every bar line, so the
//! downbeat always flashes, whatever the division. `Strobe::peek` answers the next flash and when to send it (its
//! moment − the lead); the host waits until then, sends, and calls `fired` – a changed rate or tempo is simply the
//! next peek. Nothing here sleeps or reads a clock: the tests run the grid at any BPM without waiting.

/// The tempo as a line in seconds: beat `beat` at `t`, `bps` beats per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    pub t: f64,
    pub beat: f64,
    pub bps: f64,
    pub running: bool,
    pub per_bar: u32,
}

impl Clock {
    pub fn new(t: f64, beat: f64, bpm: f64, per_bar: u32) -> Clock {
        Clock { t, beat, bps: bpm / 60.0, running: true, per_bar: per_bar.max(1) }
    }

    pub fn bpm(&self) -> f64 {
        self.bps * 60.0
    }

    pub fn beat_at(&self, t: f64) -> f64 {
        if self.running {
            self.beat + (t - self.t) * self.bps
        } else {
            self.beat
        }
    }

    /// when `beat` is heard (None while stopped)
    pub fn time_of(&self, beat: f64) -> Option<f64> {
        (self.running && self.bps > 0.0).then(|| self.t + (beat - self.beat) / self.bps)
    }
}

/// A musical division: its name and its length in beats (a beat = a quarter note).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Division {
    pub name: &'static str,
    pub beats: f64,
}

/// Slow → fast. The parameter steps through these from the neutral one.
pub const DIVISIONS: [Division; 9] = [
    Division { name: "1/1", beats: 4.0 },
    Division { name: "1/2", beats: 2.0 },
    Division { name: "1/2T", beats: 4.0 / 3.0 },
    Division { name: "1/4", beats: 1.0 },
    Division { name: "1/4T", beats: 2.0 / 3.0 },
    Division { name: "1/8", beats: 0.5 },
    Division { name: "1/8T", beats: 1.0 / 3.0 },
    Division { name: "1/16", beats: 0.25 },
    Division { name: "1/16T", beats: 1.0 / 6.0 },
];

/// The neutral division when a strobe names none: once per beat.
pub const NEUTRAL: &str = "1/4";

pub fn division(name: &str) -> Option<usize> {
    DIVISIONS.iter().position(|d| d.name.eq_ignore_ascii_case(name))
}

pub fn division_names() -> String {
    DIVISIONS.iter().map(|d| d.name).collect::<Vec<_>>().join(", ")
}

/// How the parameter deviates from the neutral division.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// 0.5 = neutral; down to 1/1 at 0, up to 1/16T at 1, whole divisions only
    #[default]
    Steps,
    /// a free factor: 0.5 = neutral, each end FREE_OCTAVES slower / faster, continuously
    Free,
}

/// Free mode's reach each way from neutral (2 octaves: 1/4 → 1/1 … 1/16).
pub const FREE_OCTAVES: f64 = 2.0;

/// Below this a flash that should have gone out already is skipped instead of sent late (a stall, a phase jump).
pub const LATE_S: f64 = 0.005;

/// The flash rate: the neutral division, the mode, the optional cap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    pub neutral: usize,
    pub mode: Mode,
    pub max_hz: Option<f64>,
}

impl Default for Rate {
    fn default() -> Self {
        Rate { neutral: division(NEUTRAL).expect("1/4"), mode: Mode::Steps, max_hz: None }
    }
}

impl Rate {
    /// the division index the parameter `p` (0..1, 0.5 = neutral) steps to (Steps mode, before the cap)
    pub fn step(&self, p: f32) -> usize {
        let p = f64::from(if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.5 });
        let n = self.neutral as f64;
        let last = (DIVISIONS.len() - 1) as f64;
        let i = if p < 0.5 { n * p / 0.5 } else { n + (last - n) * (p - 0.5) / 0.5 };
        (i.round() as usize).min(DIVISIONS.len() - 1)
    }

    /// the flash period in beats for `p` at `bps` beats per second, the cap applied
    pub fn period(&self, p: f32, bps: f64) -> f64 {
        let fastest = |beats: f64| self.max_hz.is_none_or(|hz| bps / beats <= hz + 1e-9);
        match self.mode {
            Mode::Steps => {
                let mut i = self.step(p);
                while i > 0 && !fastest(DIVISIONS[i].beats) {
                    i -= 1;
                }
                DIVISIONS[i].beats
            }
            Mode::Free => {
                let p = f64::from(if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.5 });
                let beats = DIVISIONS[self.neutral].beats * 2f64.powf(-(p - 0.5) * 2.0 * FREE_OCTAVES);
                match self.max_hz {
                    Some(hz) if bps / beats > hz => bps / hz,
                    _ => beats,
                }
            }
        }
    }
}

/// The first grid line at or after `beat`: lines every `period` beats from each bar line (the downbeat always one).
pub fn next_line(beat: f64, period: f64, per_bar: u32) -> f64 {
    let bar = f64::from(per_bar.max(1));
    let start = (beat / bar).floor() * bar;
    let k = ((beat - start) / period - 1e-9).ceil().max(0.0);
    let line = start + k * period;
    if line >= start + bar - 1e-9 {
        start + bar
    } else {
        line
    }
}

/// The next flash: its beat and when to send it (its moment − the lead), on the host's clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Next {
    pub beat: f64,
    pub at: f64,
}

/// One strobe's schedule: the rate and the last flash it fired.
#[derive(Debug, Clone, Default)]
pub struct Strobe {
    pub rate: Rate,
    last: Option<f64>,
}

impl Strobe {
    pub fn new(rate: Rate) -> Strobe {
        Strobe { rate, last: None }
    }

    /// The next flash for the rate parameter `p` at `now` with the path's `lead` (s); None while the clock stands.
    pub fn peek(&mut self, clock: &Clock, p: f32, now: f64, lead: f64) -> Option<Next> {
        if !clock.running || clock.bps <= 0.0 {
            return None;
        }
        let period = self.rate.period(p, clock.bps);
        let heard = clock.beat_at(now + lead);
        // the beat moved back (a phase set, a new anchor): forget the last flash
        if self.last.is_some_and(|l| l > heard + 2.0 * period) {
            self.last = None;
        }
        let mut from = heard - LATE_S * clock.bps;
        if let Some(l) = self.last {
            from = from.max(l + 1e-6);
        }
        let beat = next_line(from, period, clock.per_bar);
        Some(Next { beat, at: clock.time_of(beat)? - lead })
    }

    /// the flash of `beat` went out
    pub fn fired(&mut self, beat: f64) {
        self.last = Some(beat);
    }

    pub fn reset(&mut self) {
        self.last = None;
    }

    /// the flash length (s) for `duration` s at this period: at most half the period (a flash always ends before the
    /// next), at least 1 ms
    pub fn flash_length(&self, duration: f64, p: f32, clock: &Clock) -> f64 {
        let period_s = self.rate.period(p, clock.bps) / clock.bps.max(1e-9);
        duration.min(period_s * 0.5).max(0.001)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_from_the_neutral_quarter() {
        let r = Rate::default();
        assert_eq!(DIVISIONS[r.step(0.5)].name, "1/4");
        assert_eq!(DIVISIONS[r.step(0.0)].name, "1/1");
        assert_eq!(DIVISIONS[r.step(1.0)].name, "1/16T");
        // each side spreads its divisions evenly: 3 below neutral, 5 above
        assert_eq!(DIVISIONS[r.step(1.0 / 6.0)].name, "1/2");
        assert_eq!(DIVISIONS[r.step(0.6)].name, "1/4T");
        assert_eq!(DIVISIONS[r.step(0.8)].name, "1/8T");
        assert_eq!(DIVISIONS[r.step(f32::NAN)].name, "1/4", "no value = neutral");
        // another neutral
        let r = Rate { neutral: division("1/8").unwrap(), ..Rate::default() };
        assert_eq!(DIVISIONS[r.step(0.5)].name, "1/8");
        assert_eq!(division("1/16t"), Some(8));
        assert_eq!(division("1/3"), None);
    }

    #[test]
    fn no_cap_by_default_an_optional_one_steps_slower() {
        let r = Rate::default();
        // 300 BPM, 1/16T: 30 flashes a second – allowed (owner: no limit)
        assert!((5.0 / r.period(1.0, 5.0) - 30.0).abs() < 1e-9);
        // capped at 3 Hz at 120 BPM (2 bps): 1/16T → … → 1/4T (3 Hz) is the fastest allowed
        let capped = Rate { max_hz: Some(3.0), ..r };
        assert!((capped.period(1.0, 2.0) - 2.0 / 3.0).abs() < 1e-9);
        // slower than the cap: untouched
        assert_eq!(capped.period(0.5, 2.0), 1.0);
        // free mode: the cap stretches the period
        let free = Rate { mode: Mode::Free, max_hz: Some(3.0), ..r };
        assert!((free.period(1.0, 2.0) - 2.0 / 3.0).abs() < 1e-9);
        let free = Rate { mode: Mode::Free, ..r };
        assert!((free.period(0.5, 2.0) - 1.0).abs() < 1e-12);
        assert!((free.period(1.0, 2.0) - 0.25).abs() < 1e-12, "two octaves up = 1/16");
        assert!((free.period(0.0, 2.0) - 4.0).abs() < 1e-12);
        assert!((free.period(0.75, 2.0) - 0.5).abs() < 1e-12, "continuous: an octave up = 1/8");
    }

    #[test]
    fn grid_lines_restart_at_each_bar() {
        assert_eq!(next_line(0.0, 1.0, 4), 0.0);
        assert_eq!(next_line(0.2, 1.0, 4), 1.0);
        assert_eq!(next_line(3.5, 1.0, 4), 4.0);
        // triplet halves (4/3) in 4/4: 0, 4/3, 8/3, then the next bar's downbeat
        assert!((next_line(1.0, 4.0 / 3.0, 4) - 4.0 / 3.0).abs() < 1e-12);
        assert_eq!(next_line(2.7, 4.0 / 3.0, 4), 4.0);
        // a whole note in 3/4 still lands on each downbeat
        assert_eq!(next_line(0.5, 4.0, 3), 3.0);
        // negative beats (before the song's start)
        assert_eq!(next_line(-0.5, 1.0, 4), 0.0);
    }

    /// Runs the strobe as a host would: wake at each send time, send, mark fired. Returns the beats and the send times.
    fn run(clock: &Clock, p: f32, lead: f64, from: f64, secs: f64) -> Vec<(f64, f64)> {
        let mut s = Strobe::default();
        let mut now = from;
        let mut out = vec![];
        while now < from + secs {
            let n = s.peek(clock, p, now, lead).unwrap();
            if n.at <= now + 1e-12 {
                out.push((n.beat, now));
                s.fired(n.beat);
            } else {
                now = n.at;
            }
        }
        out
    }

    #[test]
    fn flashes_land_on_the_grid_at_any_bpm_and_division() {
        let lead = 0.02;
        for bpm in [60.0, 84.0, 120.0, 128.0, 174.0, 300.0] {
            // a clock anchored mid-song at an odd time
            let clock = Clock::new(1_000.123, 37.25, bpm, 4);
            for (p, name) in [(0.0f32, "1/1"), (0.5, "1/4"), (0.6, "1/4T"), (1.0, "1/16T"), (0.875, "1/16")] {
                let beats = DIVISIONS[division(name).unwrap()].beats;
                assert_eq!(DIVISIONS[Rate::default().step(p)].name, name);
                let flashes = run(&clock, p, lead, 1_001.0, 8.0);
                assert!(flashes.len() >= 2, "{bpm} {name}");
                for (i, &(beat, sent)) in flashes.iter().enumerate() {
                    // sent exactly the lead before its beat is heard (the first may be up to LATE_S late: the strobe
                    // started just after a grid line)
                    let heard = clock.time_of(beat).unwrap();
                    let late = sent + lead - heard;
                    let ok = if i == 0 { (-1e-9..=LATE_S).contains(&late) } else { late.abs() < 1e-9 };
                    assert!(ok, "{bpm} {name}: beat {beat} sent {sent} heard {heard}");
                    // on the grid: a whole number of divisions from its bar line
                    let bar = (beat / 4.0 + 1e-9).floor() * 4.0;
                    let k = (beat - bar) / beats;
                    assert!((k - k.round()).abs() < 1e-6, "{bpm} {name}: beat {beat} off the grid");
                }
                // the spacing: the division in seconds, except across a bar line for divisions that don't fill a bar
                let gap = beats * 60.0 / bpm;
                let even = (4.0 / beats).fract().abs() < 1e-9 || beats >= 4.0;
                // (from the second: the first may be a few ms late)
                for w in flashes.windows(2).skip(1) {
                    let d = w[1].1 - w[0].1;
                    if even {
                        assert!((d - gap).abs() < 1e-9, "{bpm} {name}: {d} vs {gap}");
                    } else {
                        assert!(d <= gap + 1e-9 && d > 0.0, "{bpm} {name}: {d}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_rate_change_takes_the_next_line_and_never_bursts() {
        let clock = Clock::new(0.0, 0.0, 120.0, 4);
        let mut s = Strobe::default();
        // neutral: beat 1 at 0.5 s
        let n = s.peek(&clock, 0.5, 0.3, 0.0).unwrap();
        assert_eq!((n.beat, n.at), (1.0, 0.5));
        s.fired(1.0);
        // turned to 1/16 at 0.6 s: the next sixteenth after 0.6 s (beat 1.2) is beat 1.25
        let n = s.peek(&clock, 0.875, 0.6, 0.0).unwrap();
        assert!((n.beat - 1.25).abs() < 1e-12);
        // a stall: woken 0.4 s late – the missed flashes are skipped, the next one ahead comes (no burst)
        s.fired(1.25);
        let n = s.peek(&clock, 0.875, 1.0, 0.0).unwrap();
        assert!((n.beat - 2.0).abs() < 1e-12, "{}", n.beat);
        // a flash a few ms late still goes (within LATE_S)
        let n = s.peek(&clock, 0.875, 1.003, 0.0).unwrap();
        assert!((n.beat - 2.0).abs() < 1e-12);
        // stopped: nothing
        let stopped = Clock { running: false, ..clock };
        assert!(s.peek(&stopped, 0.5, 2.0, 0.0).is_none());
        // the beat jumps back (a new anchor): the strobe follows at once
        s.fired(40.0);
        let n = s.peek(&clock, 0.5, 1.1, 0.0).unwrap();
        assert!((n.beat - 3.0).abs() < 1e-12, "{}", n.beat);
    }

    #[test]
    fn a_flash_ends_before_the_next() {
        let s = Strobe::default();
        let clock = Clock::new(0.0, 0.0, 120.0, 4);
        assert!((s.flash_length(0.03, 0.5, &clock) - 0.03).abs() < 1e-12);
        // 1/16T at 120: 83 ms apart → at most 41.7 ms
        assert!((s.flash_length(0.2, 1.0, &clock) - 1.0 / 24.0).abs() < 1e-9);
        assert_eq!(s.flash_length(0.0, 0.5, &clock), 0.001);
    }
}
