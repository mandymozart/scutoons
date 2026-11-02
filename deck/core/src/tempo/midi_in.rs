//! MIDI clock in (24 ticks per beat), the follower (#181, moved here with #308): every tick is stamped on arrival (the
//! caller's seconds) and a least squares line through the last two beats of ticks gives the tempo and the phase – USB /
//! ALSA jitter averages out, a tempo change is caught after three ticks off the line on the same side. Start / continue
//! / stop / song position move the transport. Platform-free; the ports (midir) are deck/stage's.
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
}

impl Default for ClockIn {
    /// a clock that never sent start is followed from its first tick (beat 0 there)
    fn default() -> Self {
        ClockIn { ticks: VecDeque::with_capacity(WINDOW + 1), count: 0.0, pos: -1.0, next: 0.0, running: true, last: None, misses: 0, side: 0.0 }
    }
}

impl ClockIn {
    /// one message (status byte first) that arrived at `t` (s)
    pub fn message(&mut self, t: f64, msg: &[u8]) {
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
            self.ticks.clear();
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
