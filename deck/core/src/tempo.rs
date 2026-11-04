//! Module `tempo` (#181, moved here with #185): the tempo map – beat = f(audio frame position), never a second clock. A
//! map is a line through one anchor – frame `frame` is beat `beat`, from there on `bpm` – on the time base's frame axis
//! (the one the audio thread counts; deck/native's clock.rs maps it to the heard moment). A tempo change re-anchors where
//! it happens, so the beat never jumps; a phase set (a tap, an external clock) moves the anchor's beat. Positions,
//! quantize grids, the least squares fit (MIDI clock in, tap) and tap tempo. Platform-free: the tempo controller (`Controller`:
//! sources, keys, tap, the followers of MIDI clock in / Link / OSC), the seqlock `Shared` / `BeatReader` and the time
//! types are in here too (#308); deck/native only opens the sources' ports and feeds events in. The show clock over the
//! relay – the offset filter, the anchor, when to ping and when a leader sends – is `sync` (#321). The transport as
//! a node plays it – the epoch, the edges, who follows – is `transport` (#359). How far off the show's line a follower
//! played over the last minute – the window and its words – is `window` (#387).

mod clock_out;
mod controller;
mod midi_in;
mod mode;
mod shared;
pub mod sync;
mod time;
mod transport;
mod window;

pub use clock_out::{ClockOut, Out};
pub use window::{Summary as SyncSummary, SyncWindow, SPAN_S as SYNC_SPAN_S};
pub use transport::{Edge, Follow, Played, JUMP_BEATS};
pub use controller::{Cmd, Config, Controller, Event, Host, Key, NoHost, Now, Show, Source, What};
pub use midi_in::{ClockIn, WallLine, CLOCK_STATUS, CONTINUE, LOST_S, POSITION, PPQN, START, STOP, TICK};
pub use mode::{ClockMode, Flash, FLASH_AHEAD_S, FLASH_S};
pub use shared::{BeatReader, Shared};
pub use sync::{Anchor, Filter};
pub use time::{Line, Moment, SeqLock, TimeBase};

pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 300.0;

pub fn clamp_bpm(bpm: f64) -> f64 {
    if bpm.is_finite() {
        bpm.clamp(MIN_BPM, MAX_BPM)
    } else {
        120.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoMap {
    /// the anchor: frame position `frame` is beat `beat`
    pub frame: f64,
    pub beat: f64,
    pub bpm: f64,
    /// frames per second of audio (the time base's nominal rate)
    pub rate: f64,
    /// stopped: the beat stands still (the bpm stays)
    pub running: bool,
    /// beats per bar
    pub per_bar: u32,
}

/// Where a beat lies in musical terms.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    /// beats since beat 0 (negative before it)
    pub beat: f64,
    /// bars since bar 0
    pub bar: i64,
    /// 0 .. per_bar − 1
    pub beat_in_bar: u32,
    /// 0..1 within the beat
    pub phase: f64,
    /// 0..1 within the bar
    pub bar_phase: f64,
}

impl TempoMap {
    pub fn new(bpm: f64, per_bar: u32, rate: f64) -> TempoMap {
        TempoMap { frame: 0.0, beat: 0.0, bpm: clamp_bpm(bpm), rate, running: true, per_bar: per_bar.max(1) }
    }

    /// beats per frame (0 while stopped)
    pub fn per_frame(&self) -> f64 {
        if self.running {
            self.bpm / 60.0 / self.rate
        } else {
            0.0
        }
    }

    pub fn beat_at(&self, frame: f64) -> f64 {
        self.beat + (frame - self.frame) * self.per_frame()
    }

    /// the frame at which `beat` is reached (None while stopped)
    pub fn frame_of(&self, beat: f64) -> Option<f64> {
        let p = self.per_frame();
        (p > 0.0).then(|| self.frame + (beat - self.beat) / p)
    }

    /// the same line, anchored at `frame`
    pub fn at(&self, frame: f64) -> TempoMap {
        TempoMap { frame, beat: self.beat_at(frame), ..*self }
    }

    /// from `frame` on at `bpm` – the beat at `frame` stays (a tempo change never jumps)
    pub fn with_bpm(&self, frame: f64, bpm: f64) -> TempoMap {
        TempoMap { bpm: clamp_bpm(bpm), ..self.at(frame) }
    }

    /// `beat` at `frame` (a phase set: a tap, OSC /tempo/beat, an external clock)
    pub fn with_beat(&self, frame: f64, beat: f64) -> TempoMap {
        TempoMap { frame, beat, ..*self }
    }

    /// started or stopped at `frame` (the beat there stays)
    pub fn with_running(&self, frame: f64, running: bool) -> TempoMap {
        TempoMap { running, ..self.at(frame) }
    }

    pub fn position(&self, frame: f64) -> Position {
        position(self.beat_at(frame), self.per_bar)
    }
}

pub fn position(beat: f64, per_bar: u32) -> Position {
    let per_bar = per_bar.max(1) as f64;
    let bar = (beat / per_bar).floor();
    let within = beat - bar * per_bar;
    Position {
        beat,
        bar: bar as i64,
        beat_in_bar: (within.floor() as u32).min(per_bar as u32 - 1),
        phase: beat.rem_euclid(1.0),
        bar_phase: within / per_bar,
    }
}

/// Where steps and cuts snap: off, every beat, every bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantize {
    Off,
    Beat,
    Bar,
}

impl Quantize {
    pub fn parse(s: &str) -> Option<Quantize> {
        Some(match s {
            "off" | "none" => Quantize::Off,
            "beat" => Quantize::Beat,
            "bar" => Quantize::Bar,
            _ => return None,
        })
    }

    /// the grid in beats (None = off)
    pub fn unit(self, per_bar: u32) -> Option<f64> {
        match self {
            Quantize::Off => None,
            Quantize::Beat => Some(1.0),
            Quantize::Bar => Some(per_bar.max(1) as f64),
        }
    }

    /// whether a grid line lies in (prev, now] – off: always
    pub fn crossed(self, prev: f64, now: f64, per_bar: u32) -> bool {
        match self.unit(per_bar) {
            None => true,
            Some(u) => (now / u).floor() > (prev / u).floor(),
        }
    }

    /// the first grid line at or after `beat`
    pub fn next(self, beat: f64, per_bar: u32) -> f64 {
        match self.unit(per_bar) {
            None => beat,
            Some(u) => (beat / u).ceil() * u,
        }
    }
}

/// least squares y = a + b x over the points (relative to the first x for precision); None below two points
pub fn fit(points: impl Iterator<Item = (f64, f64)> + Clone) -> Option<(f64, f64)> {
    let mut it = points.clone();
    let (x0, _) = it.next()?;
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (x, y) in points {
        let x = x - x0;
        n += 1.0;
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let d = n * sxx - sx * sx;
    if n < 2.0 || d.abs() < 1e-12 {
        return None;
    }
    let b = (n * sxy - sx * sy) / d;
    let a = (sy - b * sx) / n - b * x0;
    Some((a, b))
}

/// Tap tempo: the BPM from the taps' moments by least squares (a sloppy tap moves it less than an average of
/// intervals would), the last eight taps; a pause of two seconds starts a new count.
const KEEP: usize = 8;
const RESET_NS: u64 = 2_000_000_000;

#[derive(Default)]
pub struct Tap {
    taps: [u64; KEEP],
    n: usize,
}

impl Tap {
    /// a tap at `ns` (clock::now_ns); Some(bpm) from the second tap of a run on
    pub fn tap(&mut self, ns: u64) -> Option<f64> {
        if let Some(&last) = self.recent().last() {
            if ns <= last || ns - last > RESET_NS {
                self.n = 0;
            }
        }
        self.taps[self.n % KEEP] = ns;
        self.n += 1;
        let taps = self.recent();
        let first = *taps.first()?;
        let (_, spb) = fit(taps.iter().enumerate().map(|(i, &t)| (i as f64, (t - first) as f64 * 1e-9)))?;
        (spb > 0.0).then(|| clamp_bpm(60.0 / spb))
    }

    /// the taps of this run, oldest first
    fn recent(&self) -> Vec<u64> {
        let k = self.n.min(KEEP);
        (self.n - k..self.n).map(|i| self.taps[i % KEEP]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    #[test]
    fn the_beat_is_a_function_of_the_frame_position() {
        let m = TempoMap::new(120.0, 4, RATE);
        assert_eq!(m.beat_at(0.0), 0.0);
        assert_eq!(m.beat_at(24_000.0), 1.0); // 120 BPM: a beat every half second
        assert_eq!(m.frame_of(4.0), Some(96_000.0));
        let p = m.position(5.5 * 24_000.0);
        assert_eq!((p.bar, p.beat_in_bar, p.phase, p.bar_phase), (1, 1, 0.5, 0.375));
    }

    #[test]
    fn a_tempo_change_never_jumps_the_beat() {
        let m = TempoMap::new(120.0, 4, RATE);
        let at = 30_000.0; // beat 1.25
        let n = m.with_bpm(at, 60.0);
        assert!((n.beat_at(at) - 1.25).abs() < 1e-12);
        // from there at half the speed: one beat later is 48 000 frames later
        assert!((n.beat_at(at + 48_000.0) - 2.25).abs() < 1e-12);
        // re-anchoring is exact: the same line every frame
        let mut r = n;
        for f in (40_000..200_000).step_by(800) {
            r = r.at(f as f64);
        }
        assert!((r.beat_at(250_000.0) - n.beat_at(250_000.0)).abs() < 1e-9);
        assert_eq!(TempoMap::new(1000.0, 4, RATE).bpm, MAX_BPM);
    }

    #[test]
    fn a_stopped_map_stands_still_and_resumes_where_it_stopped() {
        let m = TempoMap::new(120.0, 4, RATE).with_running(36_000.0, false);
        assert_eq!(m.beat_at(100_000.0), 1.5);
        assert_eq!(m.frame_of(2.0), None);
        let r = m.with_running(200_000.0, true);
        assert_eq!(r.beat_at(212_000.0), 2.0);
    }

    #[test]
    fn negative_beats_are_positions_before_bar_zero() {
        let p = position(-0.25, 4);
        assert_eq!((p.bar, p.beat_in_bar), (-1, 3));
        assert!((p.phase - 0.75).abs() < 1e-12);
    }

    #[test]
    fn quantize_snaps_to_beats_and_bars() {
        assert!(Quantize::Beat.crossed(0.9, 1.0, 4));
        assert!(!Quantize::Beat.crossed(1.0, 1.9, 4));
        assert!(!Quantize::Bar.crossed(1.0, 3.99, 4));
        assert!(!Quantize::Bar.crossed(3.99, 4.01, 3)); // 3 beats a bar: the lines are 3, 6, …
        assert!(Quantize::Bar.crossed(2.9, 3.0, 3));
        assert!(Quantize::Off.crossed(1.0, 1.0, 4));
        assert_eq!(Quantize::Bar.next(4.2, 4), 8.0);
        assert_eq!(Quantize::Beat.next(3.0, 4), 3.0);
        assert_eq!(Quantize::parse("bar"), Some(Quantize::Bar));
    }

    #[test]
    fn fit_finds_the_line() {
        let pts = [(10.0, 1.0), (11.0, 3.0), (12.0, 5.0)];
        let (a, b) = fit(pts.iter().copied()).unwrap();
        assert!((b - 2.0).abs() < 1e-12 && (a + 19.0).abs() < 1e-9);
        assert!(fit([(1.0, 1.0)].iter().copied()).is_none());
    }
}

#[cfg(test)]
mod tap_tests {
    use super::*;

    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    #[test]
    fn taps_give_the_tempo() {
        let mut t = Tap::default();
        assert_eq!(t.tap(1_000_000_000), None, "one tap is no tempo");
        let mut bpm = None;
        // 120 BPM, each tap up to 15 ms early or late (a human)
        for i in 1..8u64 {
            bpm = t.tap((1_000_000_000 + i as i64 * 500_000_000 + (15e6 * jitter(i)) as i64) as u64);
        }
        let bpm = bpm.unwrap();
        assert!((bpm - 120.0).abs() < 2.0, "{bpm}");
    }

    #[test]
    fn a_pause_starts_a_new_count() {
        let mut t = Tap::default();
        t.tap(1_000_000_000);
        t.tap(1_500_000_000); // 120
        assert_eq!(t.tap(5_000_000_000), None, "after a pause the first tap counts again");
        let bpm = t.tap(6_000_000_000).unwrap();
        assert!((bpm - 60.0).abs() < 1e-6, "{bpm}");
    }

    #[test]
    fn it_keeps_the_last_eight_and_clamps() {
        let mut t = Tap::default();
        for i in 0..20u64 {
            t.tap(1_000_000_000 + i * 100_000_000); // 600 BPM → the ceiling
        }
        assert_eq!(t.tap(3_100_000_000), Some(300.0));
        let mut t = Tap::default();
        for i in 0..10u64 {
            t.tap(1_000_000_000 + i * 600_000_000); // 100 BPM
        }
        let bpm = t.tap(1_000_000_000 + 10 * 600_000_000).unwrap();
        assert!((bpm - 100.0).abs() < 1e-6, "{bpm}");
    }
}
