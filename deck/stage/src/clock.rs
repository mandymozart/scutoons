//! The time base (CYBERDECKS.md §8 "Two clocks", #181): the exact audio position as heard. The audio callback stamps
//! every block it renders – (frames rendered before it, the monotonic moment its first frame reaches the speaker) – into
//! a seqlock; the frame thread feeds those stamps into a delay-locked loop (Fons Adriaensen's DLL, JACK's and
//! PipeWire's own) and asks it for the position at the frame's moment. Whole device periods per video frame or none (the
//! X1's SOF card, 8e1cab16) no longer step the picture: the DLL interpolates between blocks and smooths the callback's
//! jitter, the answer never goes backwards.
use std::sync::atomic::{fence, AtomicU64, Ordering};

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

/// N words handed from ONE writer to any readers without a lock and without allocating (a seqlock): the writer never
/// waits, a reader retries while a write is in flight. The realtime side may be either end.
pub struct SeqLock<const N: usize> {
    seq: AtomicU64,
    words: [AtomicU64; N],
}

impl<const N: usize> Default for SeqLock<N> {
    fn default() -> Self {
        SeqLock { seq: AtomicU64::new(0), words: std::array::from_fn(|_| AtomicU64::new(0)) }
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
                std::hint::spin_loop();
                continue;
            }
            let w = std::array::from_fn(|i| self.words[i].load(Ordering::Relaxed));
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

/// A block stamp: [frames rendered before the block, the moment (now_ns) its first frame is heard]
pub type Stamp = SeqLock<2>;

/// the loop bandwidth (Hz): low enough to smooth a bursty driver, high enough to settle within a second or two
const BANDWIDTH: f64 = 1.0;
/// an error past this re-anchors (a stalled device, a restart) instead of being smoothed
const JUMP_S: f64 = 0.05;
/// past the last block the position runs on for this long, then holds (the device stopped – the picture waits)
const STALL_S: f64 = 0.25;

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

/// The frame thread's half: block stamps in, the heard position (s) per video frame out.
pub struct TimeBase {
    rate: f64,
    line: Option<Line>,
    version: u64,
    out: f64,
}

impl TimeBase {
    pub fn new(rate: f32) -> TimeBase {
        TimeBase { rate: rate as f64, line: None, version: 0, out: 0.0 }
    }

    pub fn line(&self) -> Option<Line> {
        self.line
    }

    /// read the newest stamp (a new block since the last frame → into the loop)
    pub fn poll(&mut self, stamp: &Stamp) {
        if let Some((v, [frames, heard])) = stamp.read() {
            if v != self.version {
                self.version = v;
                self.block(frames, heard);
            }
        }
    }

    /// one block: `frames` rendered before it, its first frame heard at `heard_ns`
    pub fn block(&mut self, frames: u64, heard_ns: u64) {
        let (t, f) = (heard_ns as f64 * 1e-9, frames as f64);
        let nominal = 1.0 / self.rate;
        let Some(m) = self.line.as_mut() else {
            self.line = Some(Line { t, f, spf: nominal });
            return;
        };
        let df = f - m.f;
        if df <= 0.0 {
            if df < 0.0 {
                self.line = Some(Line { t, f, spf: nominal });
            }
            return;
        }
        let predicted = m.t + df * m.spf;
        let e = t - predicted;
        if e.abs() > JUMP_S {
            *m = Line { t, f, spf: m.spf };
            return;
        }
        // the second-order loop, per block of any length: w = 2π B Δt, b = √2 w, c = w²
        let w = (std::f64::consts::TAU * BANDWIDTH * df * m.spf).min(0.5);
        let (b, c) = (std::f64::consts::SQRT_2 * w, w * w);
        m.t = predicted + b * e;
        m.f = f;
        m.spf = (m.spf + c * e / df).clamp(nominal * 0.98, nominal * 1.02);
    }

    /// the heard position (s of audio) at `now_ns` – None before the first block; never less than the last answer
    pub fn seconds(&mut self, now_ns: u64) -> Option<f64> {
        let m = self.line?;
        let t = (now_ns as f64 * 1e-9).min(m.t + STALL_S);
        let s = (m.frames_at(t) / self.rate).max(self.out);
        self.out = s;
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// a small deterministic noise in −1..1 (no rand dependency)
    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// drive the time base: blocks of `period` frames, stamped by `stamp(k)` (seconds), video frames every 1/60 s
    /// (with 0.3 ms of jitter); returns (the true heard seconds, the answer) per video frame
    fn run(period: u64, seconds: f64, stamp: impl Fn(u64) -> f64) -> Vec<(f64, f64)> {
        let mut tb = TimeBase::new(RATE as f32);
        let mut out = Vec::new();
        let mut k = 0;
        let frames = (seconds * 60.0) as u64;
        for i in 1..frames {
            let now = i as f64 / 60.0 + 0.0003 * jitter(i + 7);
            // the blocks whose callback has run by now (rendered ahead of the heard moment by the latency)
            while stamp(k) - 0.02 <= now {
                tb.block(k * period, (stamp(k) * 1e9) as u64);
                k += 1;
            }
            out.push((now, tb.seconds((now * 1e9) as u64).unwrap_or(0.0)));
        }
        out
    }

    fn check_smooth(v: &[(f64, f64)], what: &str, tolerance: f64) {
        let settled = &v[v.len() / 2..];
        for w in settled.windows(2) {
            let (step, real) = (w[1].1 - w[0].1, w[1].0 - w[0].0);
            assert!((step - real).abs() < 0.0015, "{what}: a frame moved {:.2} ms for {:.2} ms", step * 1e3, real * 1e3);
        }
        for &(t, s) in settled {
            // the first block is heard at 0.03 s
            assert!((s - (t - 0.03)).abs() < tolerance, "{what}: {s:.4} vs the true {:.4}", t - 0.03);
        }
    }

    #[test]
    fn exact_stamps_interpolate_between_blocks() {
        let mut tb = TimeBase::new(48_000.0);
        tb.block(0, 1_000_000_000);
        tb.block(1024, 1_000_000_000 + 21_333_333);
        let s = tb.seconds(1_010_000_000).unwrap();
        assert!((s - 0.010).abs() < 1e-5, "{s}");
        let s = tb.seconds(1_030_000_000).unwrap(); // past the last block: runs on
        assert!((s - 0.030).abs() < 1e-5, "{s}");
    }

    #[test]
    fn it_never_goes_backwards_and_holds_on_a_stall() {
        let mut tb = TimeBase::new(48_000.0);
        assert!(tb.seconds(5).is_none());
        tb.block(48_000, 2_000_000_000);
        let a = tb.seconds(2_100_000_000).unwrap();
        let b = tb.seconds(2_050_000_000).unwrap(); // an earlier moment: the same answer, never less
        assert_eq!(a, b);
        let held = tb.seconds(9_000_000_000).unwrap(); // the device stopped: STALL_S on, then it holds
        assert!((held - (1.0 + STALL_S)).abs() < 1e-6, "{held}");
    }

    #[test]
    fn a_jump_re_anchors() {
        let mut tb = TimeBase::new(48_000.0);
        tb.block(0, 1_000_000_000);
        tb.block(48_000, 3_000_000_000); // a second of audio, two seconds later (a stalled device)
        assert!((tb.seconds(3_000_000_000).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn whole_periods_per_frame_or_none_give_smooth_time() {
        // the X1: 1024-frame periods (21.3 ms) against 16.7 ms video frames, the callback 1.5 ms late or early
        let p = 1024.0 / RATE;
        check_smooth(&run(1024, 6.0, |k| 0.03 + k as f64 * p + 0.0015 * jitter(k)), "1024 jittery", 0.002);
    }

    #[test]
    fn a_bursty_driver_gives_smooth_time() {
        // 4 quanta of 128 frames back to back (0.1 ms apart), then nothing for the rest of 10.7 ms: smooth, and off by
        // at most half a burst (the stamps themselves say four blocks are heard at once)
        let p = 512.0 / RATE;
        check_smooth(&run(128, 6.0, |k| 0.03 + (k / 4) as f64 * p + (k % 4) as f64 * 0.0001), "bursts", 256.0 / RATE);
    }

    #[test]
    fn a_device_running_fast_is_followed() {
        // the device's crystal: 48 048 Hz against the nominal 48 000 – the loop learns the rate
        let p = 512.0 / 48_048.0;
        let v = run(512, 12.0, |k| 0.03 + k as f64 * p);
        for &(t, s) in &v[v.len() / 2..] {
            // the heard position in nominal seconds runs 0.1 % fast
            assert!((s - (t - 0.03) * 48_048.0 / RATE).abs() < 0.002, "{s} vs {t}");
        }
    }

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
        let s = std::sync::Arc::new(Stamp::default());
        let w = s.clone();
        let writer = std::thread::spawn(move || {
            for i in 1..200_000u64 {
                w.write([i, i * 3]);
            }
        });
        let mut seen = 0;
        while !writer.is_finished() {
            if let Some((_, [a, b])) = s.read() {
                assert_eq!(b, a * 3);
                seen += 1;
            }
        }
        writer.join().unwrap();
        assert!(seen > 0);
    }
}
