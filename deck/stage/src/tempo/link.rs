//! Ableton Link as a tempo source (#181, cargo feature `link`: rusty_link builds Ableton's C++ library – needs cmake,
//! a C++ compiler and libclang at build time). Link's beat at a host moment is what peers hear then; the stage asks it
//! for the beat at this frame's moment (Link's own clock, mapped onto clock::now_ns's by sampling both) and turns it
//! into a wall line → a tempo map on the time base's frames. The quantum is the bar. A tempo set here (+ / −, a tap)
//! goes to the whole session.
use super::midi::WallLine;
use crate::clock::now_ns;
use rusty_link::{AblLink, SessionState};

pub struct Link {
    link: AblLink,
    state: SessionState,
}

impl Link {
    /// joins the network session (UDP multicast on the local network)
    pub fn new(bpm: f64) -> Link {
        let l = Link::local(bpm);
        l.link.enable(true);
        l
    }

    /// a session of one, not on the network (tests)
    pub fn local(bpm: f64) -> Link {
        Link { link: AblLink::new(bpm), state: SessionState::new() }
    }

    /// Link's clock (µs) at our moment `ns`
    fn micros_at(&self, ns: u64) -> i64 {
        let (link, ours) = (self.link.clock_micros(), now_ns());
        link + (ns as i64 - ours as i64) / 1000
    }

    /// the session's line at the moment `ns`
    pub fn line(&mut self, ns: u64, per_bar: u32) -> WallLine {
        self.link.capture_app_session_state(&mut self.state);
        let beat = self.state.beat_at_time(self.micros_at(ns), per_bar.max(1) as f64);
        WallLine { t: ns as f64 * 1e-9, beat, bps: self.state.tempo() / 60.0, running: true }
    }

    pub fn set_tempo(&mut self, bpm: f64) {
        self.link.capture_app_session_state(&mut self.state);
        self.state.set_tempo(bpm, self.micros_at(now_ns()));
        self.link.commit_app_session_state(&self.state);
    }

    pub fn peers(&self) -> u64 {
        self.link.num_peers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_session_counts_beats_at_its_tempo() {
        let mut l = Link::local(120.0);
        let t = now_ns();
        let a = l.line(t, 4);
        let b = l.line(t + 500_000_000, 4);
        assert!((a.bpm() - 120.0).abs() < 1e-6);
        assert!((b.beat - a.beat - 1.0).abs() < 1e-3, "half a second at 120 = one beat: {} → {}", a.beat, b.beat);
        l.set_tempo(90.0);
        assert!((l.line(now_ns(), 4).bpm() - 90.0).abs() < 1e-6);
        assert_eq!(l.peers(), 0);
    }
}
