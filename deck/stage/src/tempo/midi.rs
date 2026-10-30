//! MIDI clock (24 ticks per beat) in and out (#181). In: every tick is stamped on arrival (clock::now_ns) and a least
//! squares line through the last two beats of ticks gives the tempo and the phase – USB / ALSA jitter averages out, a
//! tempo change is caught after three ticks off the line on the same side. Start / continue / stop / song position
//! move the transport. Out: its own thread sends each tick when its beat is heard (the tempo map → the time base's
//! line → a moment), so gear that follows our clock plays with what the deck plays. The follower and the schedule are
//! always built; the ports (midir) are the `midi` module's (#185).
use super::map::{fit, TempoMap};
use crate::clock::Line;
use std::collections::VecDeque;
#[cfg(feature = "midi")]
pub use io::{connect_in, spawn_out, ClockOutThread};

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

/// The ports: midir's connections and the clock out's thread.
#[cfg(feature = "midi")]
mod io {
    use super::super::map::Shared;
    use super::super::{Event, What};
    use super::ClockOut;
    use crate::clock::now_ns;
    use anyhow::{anyhow, bail, Result};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use std::time::Duration;

    /// the first port whose name contains `want` (any case); "" = the first port
    fn pick<P: Clone>(ports: Vec<P>, name: impl Fn(&P) -> String, want: &str) -> Option<(P, String)> {
        let want = want.to_lowercase();
        ports.into_iter().map(|p| (p.clone(), name(&p))).find(|(_, n)| n.to_lowercase().contains(&want))
    }

    /// MIDI clock in from the port matching `want`: its clock messages go to the tempo as events. Dropping the
    /// connection closes the port.
    pub fn connect_in(want: &str, tx: Sender<Event>) -> Result<(midir::MidiInputConnection<()>, String)> {
        let mut input = midir::MidiInput::new("deck-stage clock in").map_err(|e| anyhow!("MIDI in: {e}"))?;
        input.ignore(midir::Ignore::SysexAndActiveSense);
        let Some((port, name)) = pick(input.ports(), |p| input.port_name(p).unwrap_or_default(), want) else {
            bail!("no MIDI input port matching '{want}'")
        };
        let conn = input
            .connect(
                &port,
                "deck-stage-clock-in",
                move |_, msg, _| {
                    if let Some(&s @ (0xF8 | 0xFA | 0xFB | 0xFC | 0xF2)) = msg.first() {
                        let mut m = [s, 0, 0];
                        for (d, b) in m.iter_mut().zip(msg).skip(1) {
                            *d = *b;
                        }
                        let _ = tx.send(Event { at_ns: now_ns(), what: What::Midi(m) });
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("MIDI in '{name}': {e}"))?;
        Ok((conn, name))
    }

    /// MIDI clock out on its own thread; dropping it stops the thread (and the port).
    pub struct ClockOutThread {
        stop: Arc<AtomicBool>,
        pub port: String,
    }

    impl Drop for ClockOutThread {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    /// MIDI clock out to the port matching `want`, from the shared tempo map and time line; `lead` s early.
    pub fn spawn_out(want: &str, shared: Arc<Shared>, lead: f64) -> Result<ClockOutThread> {
        let output = midir::MidiOutput::new("deck-stage clock out").map_err(|e| anyhow!("MIDI out: {e}"))?;
        let Some((port, name)) = pick(output.ports(), |p| output.port_name(p).unwrap_or_default(), want) else {
            bail!("no MIDI output port matching '{want}'")
        };
        let mut conn = output.connect(&port, "deck-stage-clock-out").map_err(|e| anyhow!("MIDI out '{name}': {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::Builder::new().name("midi-clock-out".into()).spawn(move || {
            let mut co = ClockOut::default();
            let secs = || now_ns() as f64 * 1e-9;
            while !flag.load(Ordering::Relaxed) {
                let (Some((map, _)), Some(line)) = (shared.map(), shared.line()) else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let now = secs();
                let Some((due, out)) = co.peek(&map, &line, now, lead) else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                // far ahead: sleep a little and look again (the map may change meanwhile); close: wait it out exactly
                if due - now > 0.003 {
                    std::thread::sleep(Duration::from_secs_f64((due - now - 0.002).min(0.005)));
                    continue;
                }
                while secs() < due {
                    std::hint::spin_loop();
                }
                if let Err(e) = conn.send(&out.bytes()) {
                    tracing::warn!("MIDI clock out: {e}");
                }
                co.sent(out);
            }
        })?;
        Ok(ClockOutThread { stop, port: name })
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
