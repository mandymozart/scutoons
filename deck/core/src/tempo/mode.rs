//! The clock as a deck shows it (#370): the picker's four choices (`ClockMode`), the strobe's flash on the beat (`Flash`)
//! and the tempo line (`Now::line`) – once, for the native HUD and the browser's panel, so no page keeps its own copy of
//! the choice, the beat edge or the words. The page keeps the buttons and draws what these say.
use super::controller::{Now, Source};

/// The picker's choices (owner 2026-11-03, #321): follow the show clock, the deck's own clock, the own clock leading the
/// show, MIDI clock in (it leads the show – the venue's table-top clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
    Show,
    Own,
    Lead,
    Midi,
}

impl ClockMode {
    /// in the picker's order (the state block carries the index)
    pub const ALL: [ClockMode; 4] = [ClockMode::Show, ClockMode::Own, ClockMode::Lead, ClockMode::Midi];

    pub fn name(self) -> &'static str {
        match self {
            ClockMode::Show => "show",
            ClockMode::Own => "own",
            ClockMode::Lead => "lead",
            ClockMode::Midi => "midi",
        }
    }

    pub fn parse(s: &str) -> Option<ClockMode> {
        ClockMode::ALL.into_iter().find(|m| m.name().eq_ignore_ascii_case(s))
    }

    /// the source it makes the master and whether the deck asks to lead (MIDI clock in leads anyway)
    pub fn source(self) -> (Source, bool) {
        match self {
            ClockMode::Show => (Source::Relay, false),
            ClockMode::Own => (Source::Internal, false),
            ClockMode::Lead => (Source::Internal, true),
            ClockMode::Midi => (Source::Midi, false),
        }
    }

    /// the choice a master and the ask to lead amount to; Link and OSC are none of the four (the native deck's F3, a scene)
    pub fn of(source: Source, lead: bool) -> Option<ClockMode> {
        match source {
            Source::Relay => Some(ClockMode::Show),
            Source::Internal if lead => Some(ClockMode::Lead),
            Source::Internal => Some(ClockMode::Own),
            Source::Midi => Some(ClockMode::Midi),
            Source::Link | Source::Osc => None,
        }
    }
}

/// how long the strobe flashes on a beat (s)
pub const FLASH_S: f64 = 0.06;
/// how far past the heard moment a clock frame already names the next beat's flash (s): longer than a pause between two
/// frames (the browser's 25 ms, a late one), shorter than the shortest beat (0.2 s at the 300 BPM cap) less the flash – so
/// a flash is never missed and the last one has ended before the next is named
pub const FLASH_AHEAD_S: f64 = 0.1;

/// The strobe's flash (#370): the beat edge it flashes on, in s from the heard moment (≤ 0: it began, > 0: it comes), and
/// how long it lasts. A page draws it while its own time is within [edge, edge + dur].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flash {
    pub edge_s: f64,
    pub dur_s: f64,
}

impl Now {
    /// the flash of the beat at most `FLASH_AHEAD_S` past the heard moment; None while the clock stands
    pub fn flash(&self) -> Option<Flash> {
        if !self.running || self.bpm <= 0.0 {
            return None;
        }
        let bps = self.bpm / 60.0;
        let edge = (self.pos.beat + FLASH_AHEAD_S * bps).floor();
        Some(Flash { edge_s: (edge - self.pos.beat) / bps, dur_s: FLASH_S })
    }

    /// the tempo line – BPM, bar and beat (counted from 1), stopped, the click: the native HUD's line (its beat lights sit
    /// at its end) and the browser panel's, word for word; `Controller::status` says the source's state next to it
    pub fn line(&self, click: bool) -> String {
        format!(
            "BPM {:.1}  BAR {} BEAT {}{}{}",
            self.bpm,
            self.pos.bar + 1,
            self.pos.beat_in_bar + 1,
            if self.running { "" } else { " STOP" },
            if click { " CLK" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tempo::{Position, MAX_BPM};

    fn now(bpm: f64, beat: f64, running: bool) -> Now {
        Now { bpm, pos: Position { beat, bar: (beat / 4.0).floor() as i64, beat_in_bar: beat.rem_euclid(4.0) as u32, ..Position::default() }, per_bar: 4, running }
    }

    #[test]
    fn every_choice_is_a_source_and_back() {
        for m in ClockMode::ALL {
            let (s, lead) = m.source();
            assert_eq!(ClockMode::of(s, lead), Some(m));
            assert_eq!(ClockMode::parse(m.name()), Some(m));
        }
        assert_eq!(ClockMode::of(Source::Midi, true), Some(ClockMode::Midi), "MIDI clock in leads anyway");
        assert_eq!(ClockMode::of(Source::Link, false), None);
    }

    #[test]
    fn the_flash_names_the_beat_edge_ahead_and_holds_it_after() {
        // 120 BPM: 2 beats a second, the flash named 0.1 s (0.2 beats) ahead
        let f = now(120.0, 4.9, true).flash().unwrap();
        assert!((f.edge_s - 0.05).abs() < 1e-9 && f.dur_s == FLASH_S, "{f:?}");
        let f = now(120.0, 5.02, true).flash().unwrap();
        assert!((f.edge_s + 0.01).abs() < 1e-9, "the beat began 10 ms ago: {f:?}");
        assert!(now(120.0, 5.5, true).flash().unwrap().edge_s < -FLASH_S, "mid-beat: the last flash is over");
        assert_eq!(now(120.0, 5.0, false).flash(), None, "stopped: no flash");
        // at the cap the last flash is over before the next beat is named
        let beat_s = 60.0 / MAX_BPM;
        assert!(FLASH_AHEAD_S + FLASH_S < beat_s && FLASH_AHEAD_S > 0.025);
    }

    #[test]
    fn the_tempo_line_is_the_huds() {
        assert_eq!(now(120.0, 4.5, true).line(true), "BPM 120.0  BAR 2 BEAT 1 CLK");
        assert_eq!(now(93.0, 3.0, false).line(false), "BPM 93.0  BAR 1 BEAT 4 STOP");
    }
}
