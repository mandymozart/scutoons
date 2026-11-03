//! MIDI clock in (24 ticks per beat), the follower (#181, moved here with #308): every tick is stamped on arrival (the
//! caller's seconds) and a least squares line through the last two beats of ticks gives the tempo and the phase – USB /
//! ALSA jitter averages out, a tempo change is caught after three ticks off the line on the same side. Start / continue
//! / stop / song position move the transport. Platform-free; the ports (midir) are deck/native's.
//! The hardware cases (#321, an Elektron on USB or DIN): clock while stopped (the ticks keep the tempo, the song stands),
//! clock only while playing (start after silence: beat 0 at the first tick), song position before continue, active
//! sensing and every other message ignored; a dropped tick (one interval of two ticks, then the old spacing again) is
//! counted, so the song position never slips a tick; a clock regained while it plays without a start (a cable pulled
//! and plugged back) goes on from where the held tempo says, not from where the ticks stopped.
use super::time::Line;
use super::{fit, TempoMap};
use std::collections::VecDeque;

pub const PPQN: f64 = 24.0;
/// the ticks the line is fitted through (two beats)
const WINDOW: usize = 48;
/// no tick for this long: the clock is lost (the stage runs on at the last tempo)
pub const LOST_S: f64 = 0.5;
/// a tick this far off the line counts as a miss; MISSES in a row on one side = a tempo change
const JUMP_S: f64 = 0.004;
const MISSES: u32 = 3;
/// an interval this many ticks long (at most) may hide dropped ticks; within this share of a tick of a whole number
const MAX_DROPPED: f64 = 4.0;
const DROP_SLACK: f64 = 0.25;

/// A clock as a line in wall seconds (clock::now_ns's clock): the beat at `t`, beats per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallLine {
    pub t: f64,
    pub beat: f64,
    pub bps: f64,
    pub running: bool,
}

impl WallLine {
    pub fn beat_at(&self, t: f64) -> f64 {
        if self.running {
            self.beat + (t - self.t) * self.bps
        } else {
            self.beat
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bps * 60.0
    }

    /// the same clock as a tempo map on the time base's frame axis (`line`: seconds ↔ frames, the DLL's measured
    /// seconds per frame – a device crystal off by 0.1 % stays locked)
    pub fn to_map(&self, line: &Line, rate: f64, per_bar: u32) -> TempoMap {
        let bpf = self.bps * line.spf;
        TempoMap { frame: line.frames_at(self.t), beat: self.beat, bpm: bpf * rate * 60.0, rate, running: self.running, per_bar }
    }
}

/// MIDI clock in: fed the clock's messages with their arrival (s), asked for its line.
pub struct ClockIn {
    /// (tick count, arrival s) – every tick counts, running or not (a DAW sends its clock while stopped too)
    ticks: VecDeque<(f64, f64)>,
    count: f64,
    /// the song position (ticks) of the newest tick and of the next one
    pos: f64,
    next: f64,
    running: bool,
    last: Option<f64>,
    misses: u32,
    side: f64,
    /// a start / continue / stop / song position came since the last tick (a regained clock then goes on from it)
    moved: bool,
    /// the last interval was about `n` + 1 ticks at the line's seconds per tick: `n` dropped ticks, or a tempo change –
    /// the next interval decides
    dropped: Option<(f64, f64)>,
}

impl Default for ClockIn {
    /// a clock that never sent start is followed from its first tick (beat 0 there)
    fn default() -> Self {
        ClockIn {
            ticks: VecDeque::with_capacity(WINDOW + 1),
            count: 0.0,
            pos: -1.0,
            next: 0.0,
            running: true,
            last: None,
            misses: 0,
            side: 0.0,
            moved: false,
            dropped: None,
        }
    }
}

impl ClockIn {
    /// one message (status byte first) that arrived at `t` (s)
    pub fn message(&mut self, t: f64, msg: &[u8]) {
        if matches!(msg.first(), Some(0xFA | 0xFB | 0xFC | 0xF2)) {
            self.moved = true;
        }
        match msg.first() {
            Some(0xF8) => self.tick(t),
            // start: the next tick is beat 0; continue: from the song position
            Some(0xFA) => {
                self.next = 0.0;
                self.pos = -1.0;
                self.running = true;
            }
            Some(0xFB) => {
                self.pos = self.next - 1.0;
                self.running = true;
            }
            Some(0xFC) => {
                self.next = self.pos + 1.0;
                self.running = false;
            }
            // song position: 16ths (6 ticks), 14 bits LSB first
            Some(0xF2) if msg.len() >= 3 => {
                self.next = (((msg[2] as u16) << 7) | msg[1] as u16) as f64 * 6.0;
                self.pos = self.next - 1.0;
            }
            _ => {}
        }
    }

    fn tick(&mut self, t: f64) {
        if self.last.is_some_and(|l| t - l > LOST_S) {
            // regained: without a transport message the song goes on where the held tempo says (the stage ran on at
            // it), to the nearest tick – not from where the ticks stopped
            if self.running && !self.moved {
                if let Some(held) = self.line() {
                    self.next = (held.beat_at(t) * PPQN).round();
                }
            }
            self.ticks.clear();
            self.dropped = None;
        }
        if let (Some((n, spt)), Some(last)) = (self.dropped.take(), self.last) {
            // the interval after a long one is the old spacing again: the long one hid `n` dropped ticks
            if ((t - last) / spt - 1.0).abs() < DROP_SLACK {
                if let Some(back) = self.ticks.back_mut() {
                    back.0 += n;
                }
                self.count += n;
                if self.running {
                    self.pos += n;
                    self.next += n;
                }
            }
        }
        if let (Some(last), Some((_, spt))) = (self.last, fit(self.ticks.iter().copied()).filter(|_| self.ticks.len() >= 6)) {
            let k = (t - last) / spt;
            if (2.0 - DROP_SLACK..=MAX_DROPPED + DROP_SLACK).contains(&k) && (k - k.round()).abs() < DROP_SLACK {
                self.dropped = Some((k.round() - 1.0, spt));
            }
        }
        if self.ticks.len() >= 6 {
            if let Some((a, b)) = fit(self.ticks.iter().copied()) {
                let e = t - (a + b * self.count);
                if e.abs() > JUMP_S && (self.misses == 0 || e.signum() == self.side) {
                    self.misses += 1;
                    self.side = e.signum();
                } else {
                    self.misses = 0;
                }
                if self.misses >= MISSES {
                    // a new tempo: keep the ticks since it began
                    let keep = MISSES as usize;
                    self.ticks.drain(..self.ticks.len() - keep);
                    self.misses = 0;
                }
            }
        }
        self.ticks.push_back((self.count, t));
        if self.ticks.len() > WINDOW {
            self.ticks.pop_front();
        }
        self.count += 1.0;
        if self.running {
            self.pos = self.next;
            self.next += 1.0;
        }
        self.last = Some(t);
        self.moved = false;
    }

    pub fn running(&self) -> bool {
        self.running
    }

    /// no tick for LOST_S (or none yet)
    pub fn lost(&self, now: f64) -> bool {
        self.last.map_or(true, |l| now - l > LOST_S)
    }

    /// the clock's line – None below two ticks
    pub fn line(&self) -> Option<WallLine> {
        let (a, spt) = fit(self.ticks.iter().copied())?;
        if spt <= 0.0 {
            return None;
        }
        let &(n, _) = self.ticks.back()?;
        let t = a + spt * n;
        let bps = 1.0 / (spt * PPQN);
        Some(if self.running {
            WallLine { t, beat: self.pos / PPQN, bps, running: true }
        } else {
            WallLine { t, beat: self.next / PPQN, bps, running: false }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// a clock at `bpm` from `t0` for `beats`, each tick up to `jit` s late or early; returns the clock and the
    /// true time of the last tick
    fn feed(c: &mut ClockIn, t0: f64, first: u64, bpm: f64, beats: f64, jit: f64) -> f64 {
        let spt = 60.0 / bpm / PPQN;
        let n = (beats * PPQN) as u64;
        for k in 0..n {
            c.message(t0 + k as f64 * spt + jit * jitter(first + k), &[0xF8]);
        }
        t0 + (n - 1) as f64 * spt
    }

    #[test]
    fn a_jittery_clock_gives_tempo_and_phase() {
        // 24 ppqn at 120 BPM, each tick ±1 ms (USB, the ALSA sequencer)
        let mut c = ClockIn::default();
        c.message(10.0 - 0.001, &[0xFA]);
        let last = feed(&mut c, 10.0, 0, 120.0, 8.0, 0.001);
        let l = c.line().unwrap();
        assert!((l.bpm() - 120.0).abs() < 0.1, "{}", l.bpm());
        // the beat at the last tick's true moment: 8 beats less one tick, within 1 ms (1/500 beat)
        let want = 8.0 - 1.0 / PPQN;
        assert!((l.beat_at(last) - want).abs() < 0.002, "{} vs {want}", l.beat_at(last));
        // and half a beat later, still on the clock's grid
        assert!((l.beat_at(last + 0.25) - want - 0.5).abs() < 0.003);
    }

    #[test]
    fn a_tempo_change_is_followed_within_a_beat() {
        let mut c = ClockIn::default();
        let last = feed(&mut c, 10.0, 0, 120.0, 4.0, 0.001);
        let t1 = last + 60.0 / 140.0 / PPQN;
        let last = feed(&mut c, t1, 500, 140.0, 1.0, 0.001);
        let l = c.line().unwrap();
        assert!((l.bpm() - 140.0).abs() < 1.5, "after one beat: {}", l.bpm());
        let _ = feed(&mut c, last + 60.0 / 140.0 / PPQN, 900, 140.0, 2.0, 0.001);
        assert!((c.line().unwrap().bpm() - 140.0).abs() < 0.2, "after three: {}", c.line().unwrap().bpm());
    }

    #[test]
    fn start_stop_continue_and_song_position() {
        let mut c = ClockIn::default();
        feed(&mut c, 1.0, 0, 120.0, 2.0, 0.0);
        c.message(2.0, &[0xFC]); // stop: the beat stands, ticks keep the tempo
        let stopped = c.line().unwrap();
        assert!(!stopped.running);
        assert!((stopped.beat_at(5.0) - 2.0).abs() < 1e-9, "{}", stopped.beat_at(5.0));
        feed(&mut c, 2.02, 100, 120.0, 1.0, 0.0);
        assert!((c.line().unwrap().beat - 2.0).abs() < 1e-9, "ticks while stopped don't move the song");
        // the stopped clock's ticks ran 2.02 … 2.499; the next one is due at 2.52
        c.message(2.505, &[0xF2, 16, 0]); // song position 16 sixteenths = beat 4
        c.message(2.51, &[0xFB]); // continue
        c.message(2.52, &[0xF8]);
        let l = c.line().unwrap();
        assert!(l.running);
        assert!((l.beat_at(2.52) - 4.0).abs() < 1e-3, "{}", l.beat_at(2.52));
        c.message(2.53, &[0xFA]); // start: the next tick is beat 0
        let t = 2.52 + 1.0 / 48.0;
        c.message(t, &[0xF8]);
        assert!(c.line().unwrap().beat_at(t).abs() < 1e-3);
        assert!(!c.lost(t + 0.1) && c.lost(t + 0.6));
    }

    /// ticks of a clock whose tempo is `bpm(beat)`, from beat `b0` at `t0`, `beats` long, each `jit` s off; ticks for
    /// which `skip(k)` holds are not delivered (dropped, a pulled cable). Returns the true (time, beat) of every tick.
    fn feed_curve(c: &mut ClockIn, t0: f64, b0: f64, beats: f64, bpm: impl Fn(f64) -> f64, jit: f64, skip: impl Fn(u64) -> bool) -> Vec<(f64, f64)> {
        let mut t = t0;
        let mut out = Vec::new();
        for k in 0..(beats * PPQN) as u64 {
            let b = b0 + k as f64 / PPQN;
            if !skip(k) {
                c.message(t + jit * jitter(k * 7 + 3), &[0xF8]);
            }
            out.push((t, b));
            t += 60.0 / bpm(b) / PPQN;
        }
        out
    }

    /// how far (s) the clock's line is off a true tick
    fn off(c: &ClockIn, (t, b): (f64, f64)) -> f64 {
        let l = c.line().unwrap();
        (l.beat_at(t) - b) / l.bps
    }

    #[test]
    fn ten_minutes_of_a_jittery_clock_neither_drift_nor_wander() {
        let mut c = ClockIn::default();
        c.message(0.999, &[0xFA]);
        let ticks = feed_curve(&mut c, 1.0, 0.0, 128.0 * 10.0, |_| 128.0, 0.001, |_| false);
        let last = *ticks.last().unwrap();
        assert!(off(&c, last).abs() < 0.001, "after 10 min: {} ms", off(&c, last) * 1e3);
        assert!((c.line().unwrap().bpm() - 128.0).abs() < 0.05, "{}", c.line().unwrap().bpm());
    }

    #[test]
    fn a_tempo_ramp_is_followed_and_the_beat_stays_on_the_clock() {
        // 90 → 140 BPM over 8 beats (an Elektron's tempo knob turned), then 2 beats at 140; ±1 ms jitter
        let mut c = ClockIn::default();
        c.message(0.999, &[0xFA]);
        let ramp = |b: f64| if b < 4.0 { 90.0 } else if b < 12.0 { 90.0 + (b - 4.0) / 8.0 * 50.0 } else { 140.0 };
        let ticks = feed_curve(&mut c, 1.0, 0.0, 14.0, ramp, 0.001, |_| false);
        let last = *ticks.last().unwrap();
        assert!((c.line().unwrap().bpm() - 140.0).abs() < 1.0, "{}", c.line().unwrap().bpm());
        assert!(off(&c, last).abs() < 0.003, "{} ms", off(&c, last) * 1e3);
        // during the ramp the line stays within a few ms of the clock (it is behind by its window, never a tick)
        let mut c = ClockIn::default();
        c.message(0.999, &[0xFA]);
        let ticks = feed_curve(&mut c, 1.0, 0.0, 9.0, ramp, 0.001, |_| false);
        assert!(off(&c, *ticks.last().unwrap()).abs() < 0.006, "mid-ramp: {} ms", off(&c, *ticks.last().unwrap()) * 1e3);
    }

    #[test]
    fn a_dropped_tick_does_not_slip_the_song_position() {
        for dropped in [1u64, 2, 3] {
            let mut c = ClockIn::default();
            c.message(0.999, &[0xFA]);
            // tick 100 (and the ones after it, `dropped` of them) never arrive
            let ticks = feed_curve(&mut c, 1.0, 0.0, 8.0, |_| 120.0, 0.001, |k| (100..100 + dropped).contains(&k));
            let last = *ticks.last().unwrap();
            assert!(off(&c, last).abs() < 0.002, "{dropped} dropped: {} ms (a tick is 20.8)", off(&c, last) * 1e3);
        }
        // a real halving of the tempo is not taken for drops: the beat follows the clock's own ticks
        let mut c = ClockIn::default();
        c.message(0.999, &[0xFA]);
        let ticks = feed_curve(&mut c, 1.0, 0.0, 6.0, |b| if b < 4.0 { 120.0 } else { 60.0 }, 0.001, |_| false);
        assert!(off(&c, *ticks.last().unwrap()).abs() < 0.004, "{} ms", off(&c, *ticks.last().unwrap()) * 1e3);
        assert!((c.line().unwrap().bpm() - 60.0).abs() < 1.0);
    }

    #[test]
    fn a_clock_lost_while_playing_and_regained_goes_on_where_the_held_tempo_says() {
        // 4 beats, then 3 s without ticks (the cable out; the Elektron plays on), then 2 beats: no start was sent
        let mut c = ClockIn::default();
        c.message(0.999, &[0xFA]);
        let ticks = feed_curve(&mut c, 1.0, 0.0, 12.0, |_| 120.0, 0.001, |k| (96..168).contains(&k));
        assert!(ticks[168].0 - ticks[95].0 > LOST_S, "the gap is a lost clock");
        let last = *ticks.last().unwrap();
        assert!(off(&c, last).abs() < 0.002, "regained: {} ms", off(&c, last) * 1e3);
    }

    #[test]
    fn an_elektron_clock_only_while_playing_with_active_sensing_and_notes_between() {
        let mut c = ClockIn::default();
        // played 4 beats, stopped, silence (only active sensing every 300 ms)
        c.message(0.999, &[0xFA]);
        feed(&mut c, 1.0, 0, 120.0, 4.0, 0.001);
        c.message(3.0, &[0xFC]);
        for k in 0..20 {
            c.message(3.1 + k as f64 * 0.3, &[0xFE]);
        }
        assert!(c.lost(9.0));
        // start again at 9.0 – the first tick right after it is beat 0; notes, CC and active sensing in between change nothing
        c.message(9.0, &[0xFA]);
        let mut t = 9.001;
        for k in 0..48u64 {
            c.message(t + 0.0005 * jitter(k), &[0xF8]);
            c.message(t + 0.0003, &[0x90, 36, 100]);
            c.message(t + 0.0005, &[0xB0, 74, k as u8]);
            if k % 14 == 0 {
                c.message(t + 0.0007, &[0xFE]);
            }
            t += 60.0 / 120.0 / PPQN;
        }
        let l = c.line().unwrap();
        assert!(l.running);
        assert!((l.beat_at(9.001)).abs() < 0.004, "beat 0 at the first tick: {}", l.beat_at(9.001));
        assert!((l.bpm() - 120.0).abs() < 0.2, "{}", l.bpm());
        // stopped with the clock still ticking (an Elektron that sends clock while stopped): the song stands
        c.message(t, &[0xFC]);
        let stood = c.line().unwrap().beat;
        let last = feed(&mut c, t + 0.02, 300, 120.0, 2.0, 0.001);
        assert_eq!(c.line().unwrap().beat, stood);
        // song position (bar 3 = 32 sixteenths) then continue between two ticks: the next tick is beat 8
        let next = last + 60.0 / 120.0 / PPQN;
        c.message(last + 0.005, &[0xF2, 32, 0]);
        c.message(last + 0.006, &[0xFB]);
        c.message(next, &[0xF8]);
        assert!((c.line().unwrap().beat_at(next) - 8.0).abs() < 2e-3, "{}", c.line().unwrap().beat_at(next));
    }

    #[test]
    fn a_wall_line_becomes_a_map_on_the_frame_axis() {
        let line = Line { t: 100.0, f: 0.0, spf: 1.0 / RATE };
        let w = WallLine { t: 101.0, beat: 3.0, bps: 2.0, running: true };
        let m = w.to_map(&line, RATE, 4);
        assert!((m.bpm - 120.0).abs() < 1e-9);
        assert!((m.beat_at(48_000.0) - 3.0).abs() < 1e-9);
        assert!((m.beat_at(72_000.0) - 4.0).abs() < 1e-9);
    }
}
