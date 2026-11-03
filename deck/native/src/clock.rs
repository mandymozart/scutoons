//! The time base (CYBERDECKS.md §8 "Two clocks", #181): the exact audio position as heard. The audio callback stamps
//! every block it renders – (frames rendered before it, the monotonic moment its first frame reaches the speaker) – into
//! a seqlock; the frame thread feeds those stamps into a delay-locked loop (Fons Adriaensen's DLL, JACK's and
//! PipeWire's own) and asks it for the position at the frame's moment. Whole device periods per video frame or none (the
//! X1's SOF card, 8e1cab16) no longer step the picture: the DLL interpolates between blocks and smooths the callback's
//! jitter, the answer never goes backwards. The loop (`TimeBase`) is deck-core's since #321 (the browser deck runs it
//! too); here: the device's clock and the stamp.
pub use deck_core::tempo::{Line, Moment, SeqLock, TimeBase};

/// Monotonic nanoseconds. On Linux CLOCK_MONOTONIC itself – PipeWire's `pw_time.now` is on that clock, so its stamps
/// compare directly; elsewhere an `Instant` since the first call (plus 1 s, so early differences never underflow).
pub fn now_ns() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: clock_gettime writes the timespec we own; CLOCK_MONOTONIC always exists on Linux
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
        ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
    }
    #[cfg(not(target_os = "linux"))]
    {
        static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        EPOCH.get_or_init(std::time::Instant::now).elapsed().as_nanos() as u64 + 1_000_000_000
    }
}

/// A block stamp: [frames rendered before the block, the moment (now_ns) its first frame is heard]
pub type Stamp = SeqLock<2>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seqlock_hands_over_whole_stamps() {
        let s = Stamp::default();
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

    #[test]
    fn readers_never_see_a_torn_stamp() {
        use std::sync::atomic::{AtomicBool, Ordering};
        // the writer writes until the reader has read enough (a fixed count let a busy CI runner finish the writes
        // before the reader got a single turn – Windows, run 41128816256)
        let s = std::sync::Arc::new(Stamp::default());
        let done = std::sync::Arc::new(AtomicBool::new(false));
        let (w, stop) = (s.clone(), done.clone());
        let writer = std::thread::spawn(move || {
            let mut i = 0u64;
            while !stop.load(Ordering::Relaxed) {
                i += 1;
                w.write([i, i * 3]);
            }
            i
        });
        let (mut seen, start) = (0, std::time::Instant::now());
        while seen < 20_000 && start.elapsed() < std::time::Duration::from_secs(10) {
            if let Some((_, [a, b])) = s.read() {
                assert_eq!(b, a * 3);
                seen += 1;
            }
        }
        done.store(true, Ordering::Relaxed);
        let writes = writer.join().unwrap();
        assert!(seen > 0 && writes > 0, "{seen} reads while {writes} writes");
    }
}
