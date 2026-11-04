//! The tempo map across threads (#181, moved here with #308): the frame thread owns the map and hands it to the audio
//! thread and the MIDI clock out through a seqlock (`Shared`); readers evaluate it per sample. Atomics only – no
//! thread, lock or clock.
use super::time::{Line, SeqLock};
use super::transport::JUMP_BEATS;
use super::TempoMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The map and the time base's line, handed from the frame thread (the one writer) to the audio thread and the MIDI
/// clock out without a lock. Flags: bit 0 running, bit 1 the click is on, from bit 2 on the transport epoch (#359,
/// research/TRANSPORT.md §2): bumped by `publish` whenever the transport starts, stops or its beat jumps, so a reader
/// gets a map and the epoch it belongs to in one read.
#[derive(Default)]
pub struct Shared {
    map: SeqLock<6>,
    line: SeqLock<3>,
}

impl Shared {
    pub fn publish(&self, m: &TempoMap, click: bool) {
        let epoch = match self.transport() {
            Some((p, _, e)) if p.running != m.running || (p.beat_at(m.frame) - m.beat).abs() > JUMP_BEATS => e + 1,
            Some((_, _, e)) => e,
            None => 0,
        };
        let flags = m.running as u64 as f64 + if click { 2.0 } else { 0.0 } + (epoch * 4) as f64;
        self.map.write_f64([m.frame, m.beat, m.bpm, m.rate, m.per_bar as f64, flags]);
    }

    /// (the map, the click on) – None before the first publish
    pub fn map(&self) -> Option<(TempoMap, bool)> {
        self.transport().map(|(m, click, _)| (m, click))
    }

    /// (the map, the click on, the transport epoch) – None before the first publish
    pub fn transport(&self) -> Option<(TempoMap, bool, u64)> {
        let (_, [frame, beat, bpm, rate, per_bar, flags]) = self.map.read_f64()?;
        let flags = flags as u64;
        let m = TempoMap { frame, beat, bpm, rate, running: flags & 1 == 1, per_bar: per_bar as u32 };
        Some((m, flags & 2 == 2, flags >> 2))
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

    /// (the map, the transport epoch, the block's first frame) – what a node that follows the transport reads
    pub fn transport(&self) -> Option<(TempoMap, u64, f64)> {
        let (m, _, epoch) = self.shared.transport()?;
        Some((m, epoch, self.frames.load(Ordering::Relaxed) as f64))
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

    #[test]
    fn the_epoch_counts_starts_stops_and_jumps_never_a_tempo_change() {
        let s = Shared::default();
        let m = TempoMap::new(120.0, 4, RATE);
        s.publish(&m, false);
        let epoch = || s.transport().unwrap().2;
        assert_eq!(epoch(), 0);
        s.publish(&m.at(4800.0), false); // re-anchored: the same line
        s.publish(&m.with_bpm(9600.0, 90.0), false); // a tempo change keeps the beat
        assert_eq!(epoch(), 0);
        let stopped = m.with_bpm(9600.0, 90.0).with_running(19_200.0, false);
        s.publish(&stopped, true);
        assert_eq!(epoch(), 1, "a stop");
        s.publish(&stopped.with_beat(30_000.0, 0.0), true);
        assert_eq!(epoch(), 2, "a song position while stopped");
        s.publish(&stopped.with_beat(30_000.0, 0.0).with_running(40_000.0, true), true);
        assert_eq!(epoch(), 3, "a start");
        assert_eq!(s.map().map(|(m, c)| (m.running, c)), Some((true, true)));
    }
}
