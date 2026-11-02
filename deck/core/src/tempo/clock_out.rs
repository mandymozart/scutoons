//! MIDI clock out's schedule (#181, moved here with #308): which message is next and when, derived from the tempo
//! map, the time base's line and the caller's time (s, any monotonic clock). The caller sends the bytes (deck/stage's
//! `tempo::midi` port thread). Platform-free.
use super::time::Line;
use super::{TempoMap, PPQN};

/// What the clock out sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    Tick,
    Start,
    Continue,
    Stop,
    /// song position in 16ths
    Position(u16),
}

impl Out {
    pub fn bytes(self) -> Vec<u8> {
        match self {
            Out::Tick => vec![0xF8],
            Out::Start => vec![0xFA],
            Out::Continue => vec![0xFB],
            Out::Stop => vec![0xFC],
            Out::Position(p) => vec![0xF2, (p & 0x7F) as u8, ((p >> 7) & 0x7F) as u8],
        }
    }
}

/// MIDI clock out's schedule: which message is next and when (s, on clock::now_ns's clock). `lead` = the receiver's
/// latency (s): it gets each tick that much before the beat is heard.
#[derive(Default)]
pub struct ClockOut {
    running: bool,
    /// the next tick's song position (ticks)
    next: Option<i64>,
    /// a transport message waiting to go (Continue follows a Position)
    pending: Option<Out>,
}

impl ClockOut {
    pub fn peek(&mut self, map: &TempoMap, line: &Line, now: f64, lead: f64) -> Option<(f64, Out)> {
        if let Some(o) = self.pending {
            return Some((now, o));
        }
        let beat_now = map.beat_at(line.frames_at(now + lead));
        match (self.running, map.running) {
            (false, false) => None,
            (true, false) => Some((now, Out::Stop)),
            (false, true) => {
                // from the next 16th: Start at the top, else the song position + Continue
                let j = ((beat_now * PPQN / 6.0).ceil().max(0.0) as i64 * 6).min(16383 * 6);
                self.next = Some(j);
                let o = if j == 0 { Out::Start } else { Out::Position((j / 6) as u16) };
                self.pending = Some(o);
                Some((now, o))
            }
            (true, true) => {
                let due = |j: i64| Some(line.time_of(map.frame_of(j as f64 / PPQN)?) - lead);
                let j = self.next.unwrap_or_else(|| (beat_now * PPQN).ceil() as i64);
                let mut t = due(j)?;
                // the beat jumped (a phase set, a relocation): go on from the tick ahead, never a burst of late ticks
                if t < now - 0.05 || t > now + 0.25 {
                    let j = (beat_now * PPQN).ceil() as i64;
                    self.next = Some(j);
                    t = due(j)?;
                } else {
                    self.next = Some(j);
                }
                Some((t, Out::Tick))
            }
        }
    }

    /// `out` went out
    pub fn sent(&mut self, out: Out) {
        self.pending = None;
        match out {
            Out::Tick => self.next = self.next.map(|j| j + 1),
            Out::Position(_) => self.pending = Some(Out::Continue),
            Out::Start | Out::Continue => self.running = true,
            Out::Stop => {
                self.running = false;
                self.next = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// run the clock out against a map: the moments of its messages over `secs`
    fn run_out(co: &mut ClockOut, map: impl Fn(f64) -> TempoMap, line: &Line, from: f64, secs: f64) -> Vec<(f64, Out)> {
        let mut sent = Vec::new();
        let mut now = from;
        while now < from + secs {
            match co.peek(&map(now), line, now, 0.0) {
                Some((due, out)) if due <= now + 1e-4 => {
                    sent.push((now.max(due), out));
                    co.sent(out);
                }
                Some((due, _)) => now = due.min(now + 0.002),
                None => now += 0.002,
            }
        }
        sent
    }

    #[test]
    fn clock_out_ticks_land_on_the_beat() {
        let line = Line { t: 0.0, f: 0.0, spf: 1.0 / RATE };
        let base = TempoMap::new(120.0, 4, RATE);
        // 120 BPM, then 90 from frame 48 000 (beat 2)
        let map = |t: f64| if t < 1.0 { base } else { base.with_bpm(48_000.0, 90.0) };
        let mut co = ClockOut::default();
        let sent = run_out(&mut co, map, &line, 0.0, 2.0);
        assert_eq!(sent[0].1, Out::Start, "the top of the song: Start");
        let ticks: Vec<f64> = sent.iter().filter(|(_, o)| *o == Out::Tick).map(|(t, _)| *t).collect();
        assert!(ticks[0].abs() < 1e-9);
        for (k, t) in ticks.iter().enumerate().take(48) {
            assert!((t - k as f64 / 48.0).abs() < 1e-9, "tick {k} at {t}"); // 120 BPM: 48 ticks a second
        }
        let after: Vec<f64> = ticks.windows(2).skip(49).map(|w| w[1] - w[0]).collect();
        assert!(after.iter().all(|d| (d - 60.0 / 90.0 / PPQN).abs() < 1e-9), "90 BPM spacing");
    }

    #[test]
    fn clock_out_starts_mid_song_with_a_position_and_stops() {
        let line = Line { t: 0.0, f: 0.0, spf: 1.0 / RATE };
        let stopped = TempoMap::new(120.0, 4, RATE).with_beat(0.0, 4.1).with_running(0.0, false);
        let running = stopped.with_running(24_000.0, true); // starts at 0.5 s from beat 4.1
        let map = |t: f64| if t < 0.5 { stopped } else { running };
        let mut co = ClockOut::default();
        let sent = run_out(&mut co, map, &line, 0.0, 1.0);
        // the next 16th after 4.1 is 4.25 = 17 sixteenths
        assert_eq!(sent[0].1, Out::Position(17));
        assert_eq!(sent[1].1, Out::Continue);
        let first = sent.iter().find(|(_, o)| *o == Out::Tick).unwrap().0;
        assert!((first - (0.5 + 0.15 * 0.5)).abs() < 1e-9, "the tick of beat 4.25 at {first}");
        let map = |t: f64| if t < 1.2 { running } else { running.with_running(57_600.0, false) };
        let sent = run_out(&mut co, map, &line, 1.0, 0.5);
        assert_eq!(sent.last().unwrap().1, Out::Stop);
        assert_eq!(Out::Position(17).bytes(), vec![0xF2, 17, 0]);
    }
}
