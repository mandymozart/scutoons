//! MIDI clock (24 ticks per beat) in and out (#181): the ports. The follower (`ClockIn`, `WallLine`) and the clock
//! out's schedule (`ClockOut`, `Out`) are deck-core's (#308); here only midir's connections and the clock out's thread
//! (it sleeps until a message is due and sends its bytes). In: every message is stamped on arrival (clock::now_ns).
//! The ports (midir) are the `midi` module's (#185).
#[cfg(feature = "midi")]
pub use io::{connect_in, spawn_out, ClockOutThread};
pub use deck_core::tempo::{ClockIn, ClockOut, Out, WallLine, LOST_S, PPQN};

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
        let mut input = midir::MidiInput::new("deck-native clock in").map_err(|e| anyhow!("MIDI in: {e}"))?;
        input.ignore(midir::Ignore::SysexAndActiveSense);
        let Some((port, name)) = pick(input.ports(), |p| input.port_name(p).unwrap_or_default(), want) else {
            bail!("no MIDI input port matching '{want}'")
        };
        let conn = input
            .connect(
                &port,
                "deck-native-clock-in",
                move |_, msg, _| {
                    if let Some(&s) = msg.first().filter(|s| deck_core::tempo::CLOCK_STATUS.contains(s)) {
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
        let output = midir::MidiOutput::new("deck-native clock out").map_err(|e| anyhow!("MIDI out: {e}"))?;
        let Some((port, name)) = pick(output.ports(), |p| output.port_name(p).unwrap_or_default(), want) else {
            bail!("no MIDI output port matching '{want}'")
        };
        let mut conn = output.connect(&port, "deck-native-clock-out").map_err(|e| anyhow!("MIDI out '{name}': {e}"))?;
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
