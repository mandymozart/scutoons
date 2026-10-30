//! The tempo map across the stage's threads (#181): the map itself is deck-core's `tempo` module (#185); the frame
//! thread owns it and hands it to the audio thread and the MIDI clock out through a seqlock (`Shared`); readers
//! evaluate it per sample.
use crate::clock::{Line, SeqLock};
pub use deck_core::tempo::{clamp_bpm, fit, position, Position, Quantize, Tap, TempoMap, MAX_BPM, MIN_BPM};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The map and the time base's line, handed from the frame thread (the one writer) to the audio thread and the MIDI
/// clock out without a lock. Flags: bit 0 running, bit 1 the click is on.
#[derive(Default)]
pub struct Shared {
    map: SeqLock<6>,
    line: SeqLock<3>,
}

impl Shared {
    pub fn publish(&self, m: &TempoMap, click: bool) {
        let flags = m.running as u64 as f64 + if click { 2.0 } else { 0.0 };
        self.map.write_f64([m.frame, m.beat, m.bpm, m.rate, m.per_bar as f64, flags]);
    }

    /// (the map, the click on) – None before the first publish
    pub fn map(&self) -> Option<(TempoMap, bool)> {
        let (_, [frame, beat, bpm, rate, per_bar, flags]) = self.map.read_f64()?;
        let flags = flags as u64;
        let m = TempoMap { frame, beat, bpm, rate, running: flags & 1 == 1, per_bar: per_bar as u32 };
        Some((m, flags & 2 == 2))
    }

    pub fn publish_line(&self, l: &Line) {
        self.line.write_f64([l.t, l.f, l.spf]);
    }

    pub fn line(&self) -> Option<Line> {
        let (_, [t, f, spf]) = self.line.read_f64()?;
        Some(Line { t, f, spf })
    }
}

/// The audio thread's view: the shared map + the engine's frame counter (= the first frame of the block being
/// rendered, the engine counts after the block). Read once per block; never allocates.
#[derive(Clone)]
pub struct BeatReader {
    pub shared: Arc<Shared>,
    pub frames: Arc<AtomicU64>,
}

impl BeatReader {
    /// (the map, the click on, the block's first frame)
    pub fn block(&self) -> Option<(TempoMap, bool, f64)> {
        let (m, click) = self.shared.map()?;
        Some((m, click, self.frames.load(Ordering::Relaxed) as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    #[test]
    fn the_shared_map_round_trips() {
        let s = Shared::default();
        assert!(s.map().is_none());
        let m = TempoMap::new(96.0, 3, RATE).with_beat(1234.0, 7.5).with_running(1234.0, false);
        s.publish(&m, true);
        assert_eq!(s.map(), Some((m, true)));
        s.publish_line(&Line { t: 1.5, f: 48.0, spf: 1.0 / RATE });
        assert_eq!(s.line().unwrap().f, 48.0);
    }
}
