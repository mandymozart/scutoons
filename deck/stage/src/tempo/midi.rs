//! MIDI clock (24 ticks per beat) in and out (#181). The follower (`ClockIn`, `WallLine`) is deck-core's (#308). In: every tick is stamped on arrival (clock::now_ns) and a least
//! squares line through the last two beats of ticks gives the tempo and the phase – USB / ALSA jitter averages out, a
//! tempo change is caught after three ticks off the line on the same side. Start / continue / stop / song position
//! move the transport. Out: its own thread sends each tick when its beat is heard (the tempo map → the time base's
//! line → a moment), so gear that follows our clock plays with what the deck plays. The follower and the schedule are
//! always built; the ports (midir) are the `midi` module's (#185).
use super::map::TempoMap;
use crate::clock::Line;
#[cfg(feature = "midi")]
pub use io::{connect_in, spawn_out, ClockOutThread};
pub use deck_core::tempo::{ClockIn, WallLine, LOST_S, PPQN};

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
