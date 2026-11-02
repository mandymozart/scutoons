//! The time types the tempo clock runs on (moved from deck/native's clock.rs, #308): the seqlock that hands words from
//! one writer to readers without a lock, the time base's line (frame ↔ seconds) and one video frame's `Moment`.
//! Platform-free: nothing here reads a clock – the caller passes `now_ns`.
use core::sync::atomic::{fence, AtomicU64, Ordering};

/// N words handed from ONE writer to any readers without a lock and without allocating (a seqlock): the writer never
/// waits, a reader retries while a write is in flight. The realtime side may be either end.
pub struct SeqLock<const N: usize> {
    seq: AtomicU64,
    words: [AtomicU64; N],
}

impl<const N: usize> Default for SeqLock<N> {
    fn default() -> Self {
        SeqLock { seq: AtomicU64::new(0), words: core::array::from_fn(|_| AtomicU64::new(0)) }
    }
}

impl<const N: usize> SeqLock<N> {
    /// one writer only (each SeqLock has a single owner thread that writes)
    pub fn write(&self, w: [u64; N]) {
        let s = self.seq.load(Ordering::Relaxed);
        self.seq.store(s.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        for (a, v) in self.words.iter().zip(w) {
            a.store(v, Ordering::Relaxed);
        }
        self.seq.store(s.wrapping_add(2), Ordering::Release);
    }

    /// (the version, the words) – None before the first write (or when a writer keeps it busy, which it never does)
    pub fn read(&self) -> Option<(u64, [u64; N])> {
        for _ in 0..64 {
            let s1 = self.seq.load(Ordering::Acquire);
            if s1 & 1 == 1 {
                core::hint::spin_loop();
                continue;
            }
            let w = core::array::from_fn(|i| self.words[i].load(Ordering::Relaxed));
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) == s1 {
                return (s1 != 0).then_some((s1, w));
            }
        }
        None
    }

    pub fn write_f64(&self, w: [f64; N]) {
        self.write(w.map(f64::to_bits));
    }

    pub fn read_f64(&self) -> Option<(u64, [f64; N])> {
        self.read().map(|(v, w)| (v, w.map(f64::from_bits)))
    }
}


/// The DLL's line: frame `f` is heard at `t` (s), one frame lasts `spf` (s)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub t: f64,
    pub f: f64,
    pub spf: f64,
}

impl Line {
    /// the frame position heard at `t` (s)
    pub fn frames_at(&self, t: f64) -> f64 {
        self.f + (t - self.t) / self.spf
    }
    /// when frame `f` is heard (s)
    pub fn time_of(&self, f: f64) -> f64 {
        self.t + (f - self.f) * self.spf
    }
}

/// One video frame's view of the time base: what is heard now, what the audio thread has rendered, and the line that
/// maps any moment (an event's timestamp) to a frame position. Without audio a virtual 48 kHz line from the start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    pub now_ns: u64,
    pub rate: f64,
    /// the frame position heard now
    pub heard: f64,
    /// frames the audio thread has rendered (≥ heard by the output latency + a block)
    pub rendered: f64,
    pub line: Line,
}

impl Moment {
    /// no audio: frames on the wall clock since `origin_ns` at `rate` (heard = rendered)
    pub fn wall(now_ns: u64, origin_ns: u64, rate: f64) -> Moment {
        let line = Line { t: origin_ns as f64 * 1e-9, f: 0.0, spf: 1.0 / rate };
        let heard = line.frames_at(now_ns as f64 * 1e-9).max(0.0);
        Moment { now_ns, rate, heard, rendered: heard, line }
    }

    /// the frame position heard at `ns` (an event's moment)
    pub fn frames_at(&self, ns: u64) -> f64 {
        self.line.frames_at(ns as f64 * 1e-9)
    }

    pub fn seconds(&self) -> f64 {
        self.heard / self.rate
    }

    /// samples rendered past the heard moment
    pub fn lead(&self) -> usize {
        (self.rendered - self.heard).max(0.0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seqlock_hands_over_whole_words() {
        let s = SeqLock::<2>::default();
        assert!(s.read().is_none());
        s.write([7, 9]);
        let (v1, w) = s.read().unwrap();
        assert_eq!(w, [7, 9]);
        s.write([8, 10]);
        let (v2, w) = s.read().unwrap();
        assert!(v2 > v1);
        assert_eq!(w, [8, 10]);
        let l = SeqLock::<3>::default();
        l.write_f64([1.5, -2.0, 0.25]);
        assert_eq!(l.read_f64().unwrap().1, [1.5, -2.0, 0.25]);
    }
}
